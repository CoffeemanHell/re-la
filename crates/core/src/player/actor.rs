//! Player: the core on its own thread (a current-thread tokio runtime), with the two decks, the SyncClient and the
//! SubwaveClient. Front ends send it closures (`call`) and get typed snapshots back through a PlayerListener; nothing
//! here blocks the Qt thread or Android's main thread (persist_now on quit is the one wait, by design).
use super::deck::{Deck, DeckEvent, DeckSink};
use super::state::{ApiThen, Changes, Job, PlayerCore, PlayerSnapshot, StationPort, SyncPort};
use crate::covers::CoverCache;
use crate::queue::failure_text;
use crate::settings::Settings;
use crate::subsonic::{Failure, Subsonic};
use crate::subwave::{SubwaveClient, SubwaveEvent, SubwaveListener, SubwaveSnapshot, Timings, local_now};
use crate::sync::{SyncClient, SyncEvent, SyncListener, SyncSnapshot};
use serde_json::{Map, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::time::Instant;

/// Called on the player's (or the relay client's, or SUB/WAVE's) thread.
pub trait PlayerListener: Send + Sync {
    fn on_player(&self, changes: Changes, snapshot: PlayerSnapshot);
    /// A Player.get/set reply: None = null; call false = the login changed meanwhile, drop the callback.
    fn on_reply(&self, id: u64, reply: Option<Value>, call: bool);
    fn on_sync(&self, event: SyncEvent, snapshot: SyncSnapshot);
    fn on_subwave(&self, event: SubwaveEvent, snapshot: SubwaveSnapshot);
}

type Run = Box<dyn FnOnce(&mut PlayerCore) + Send>;
type CoverDone = Box<dyn FnOnce(Option<Vec<u8>>) + Send>;
type FormatsDone = Box<dyn FnOnce(Vec<String>) + Send>;
type WaveformDone = Box<dyn FnOnce(Option<crate::waveform::Waveform>) + Send>;

enum Msg {
    Run(Run),
    Deck(usize, DeckEvent),
    Sync(SyncEvent),
    Subwave(Box<SubwaveSnapshot>),
    Api(ApiThen, Result<Value, Failure>),
    PersistNow(std::sync::mpsc::Sender<()>),
    Cover(String, u32, CoverDone),
    Waveform(String, WaveformDone),
    TranscodeFormats(FormatsDone),
    Quit,
}

/// The relay's view the core reads synchronously: kept up to date by the listener before each event reaches the core.
#[derive(Default)]
struct SyncView {
    following: String,
    listeners: bool,
}

struct SyncPortImpl {
    client: Arc<SyncClient>,
    view: Arc<Mutex<SyncView>>,
}

impl SyncPort for SyncPortImpl {
    fn following(&self) -> String {
        self.view.lock().unwrap().following.clone()
    }
    fn has_listeners(&self) -> bool {
        self.view.lock().unwrap().listeners
    }
    fn now(&self) -> f64 {
        self.client.now()
    }
    fn publish(&self, state: Map<String, Value>) {
        self.client.publish(state);
    }
    fn send(&self, op: &str, args: Value) {
        self.client.send(op, args);
    }
    fn start(&self, url: String, user: String, token: String, salt: String) {
        self.client.start(url, user, token, salt);
    }
    fn unfollow(&self) {
        self.client.unfollow();
    }
    fn stop(&self, bye: bool) {
        self.client.stop(bye);
    }
    fn demo(&self) {
        self.client.demo();
    }
}

struct StationPortImpl(Arc<SubwaveClient>);

impl StationPort for StationPortImpl {
    fn open(&self, home_page: String, stream_url: String) {
        self.0.open(home_page, stream_url);
    }
    fn close(&self) {
        self.0.close();
    }
}

/// Relay and SUB/WAVE events: to the core (through the channel) and to the front end.
struct Fan {
    tx: mpsc::UnboundedSender<Msg>,
    view: Arc<Mutex<SyncView>>,
    front: Arc<dyn PlayerListener>,
}

impl SyncListener for Fan {
    fn on_event(&self, event: SyncEvent, snapshot: SyncSnapshot) {
        {
            let mut v = self.view.lock().unwrap();
            v.following = snapshot.following.clone();
            v.listeners = !snapshot.listeners.is_empty();
        }
        if matches!(event, SyncEvent::StateReceived(_) | SyncEvent::CommandReceived { .. } | SyncEvent::Connected | SyncEvent::Ended(_) | SyncEvent::Failed(_)) {
            let _ = self.tx.send(Msg::Sync(event.clone()));
        }
        self.front.on_sync(event, snapshot);
    }
}

impl SubwaveListener for Fan {
    fn on_event(&self, event: SubwaveEvent, snapshot: SubwaveSnapshot) {
        let _ = self.tx.send(Msg::Subwave(Box::new(snapshot.clone())));
        self.front.on_subwave(event, snapshot);
    }
}

pub struct Player {
    tx: mpsc::UnboundedSender<Msg>,
    sync: Arc<SyncClient>,
    subwave: Arc<SubwaveClient>,
    settings: Arc<dyn Settings>,
    thread: Option<std::thread::JoinHandle<()>>,
}

fn wall_ms() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_millis() as f64)
}

impl Player {
    /// make_deck(index, sink): the front end's audio output; it reports through sink. The saved login (if any)
    /// connects at once.
    /// `data_dir` keeps what must outlive a cleared cache (the DJ's memory).
    pub fn new(
        settings: Arc<dyn Settings>,
        cache_dir: PathBuf,
        data_dir: PathBuf,
        make_deck: impl Fn(usize, DeckSink) -> Box<dyn Deck>,
        listener: Arc<dyn PlayerListener>,
    ) -> Player {
        let (tx, rx) = mpsc::unbounded_channel();
        let view: Arc<Mutex<SyncView>> = Arc::default();
        let fan = Arc::new(Fan { tx: tx.clone(), view: view.clone(), front: listener.clone() });
        let sync = Arc::new(SyncClient::new(settings.get_bool("sync/allowControl", false), settings.get_bool("sync/hidden", false), fan.clone()));
        let subwave = Arc::new(SubwaveClient::new(fan, Timings::default()));
        let deck_tx = tx.clone();
        let sink: DeckSink = Arc::new(move |k, e| {
            let _ = deck_tx.send(Msg::Deck(k, e));
        });
        let decks = [make_deck(0, sink.clone()), make_deck(1, sink)];
        let core = PlayerCore::new(
            settings.clone(),
            cache_dir.clone(),
            data_dir,
            decks,
            Box::new(SyncPortImpl { client: sync.clone(), view }),
            Box::new(StationPortImpl(subwave.clone())),
        );
        let sw = subwave.clone();
        let waves = cache_dir.join("waveforms");
        let covers = Arc::new(CoverCache::open(cache_dir.join("covers-rs"), 256 * 1024 * 1024));
        let tx2 = tx.clone();
        let thread = std::thread::Builder::new()
            .name("re-la-player".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("player runtime");
                rt.block_on(run(core, rx, tx2, listener, covers, waves, sw));
            })
            .expect("player thread");
        Player { tx, sync, subwave, settings, thread: Some(thread) }
    }

    /// Runs f on the core, on the player's thread.
    pub fn call(&self, f: impl FnOnce(&mut PlayerCore) + Send + 'static) {
        let _ = self.tx.send(Msg::Run(Box::new(f)));
    }

    pub fn sync(&self) -> &SyncClient {
        &self.sync
    }

    pub fn subwave(&self) -> &SubwaveClient {
        &self.subwave
    }

    pub fn settings(&self) -> &Arc<dyn Settings> {
        &self.settings
    }

    /// getCoverArt through the disk cache (offline: never the network), then done(bytes) on the player's runtime.
    /// "sw:<id>": the SUB/WAVE cover of that song (a public URL; None while no SUB/WAVE is open).
    pub fn cover(&self, id: String, size: u32, done: impl FnOnce(Option<Vec<u8>>) + Send + 'static) {
        let _ = self.tx.send(Msg::Cover(id, size, Box::new(done)));
    }

    /// The song's waveform from the disk cache or the relay, then done(it) on the player's runtime. None without asking
    /// the relay when offline, for SUB/WAVE ("sw:") or no song; None too when the relay has none (yet).
    pub fn waveform(&self, id: String, done: impl FnOnce(Option<crate::waveform::Waveform>) + Send + 'static) {
        let _ = self.tx.send(Msg::Waveform(id, Box::new(done)));
    }

    /// What the server can transcode to, then done(it) on the player's runtime: its profiles ("opus-320") when the relay
    /// tells them, else formats alone ("opus").
    pub fn transcode_formats(&self, done: impl FnOnce(Vec<String>) + Send + 'static) {
        let _ = self.tx.send(Msg::TranscodeFormats(Box::new(done)));
    }

    /// On quit: saves the play queue and clears now playing on the server; blocks up to 1.5 s (2 s at most).
    pub fn persist_now(&self) {
        let (done, wait) = std::sync::mpsc::channel();
        if self.tx.send(Msg::PersistNow(done)).is_ok() {
            let _ = wait.recv_timeout(Duration::from_secs(2));
        }
    }

    /// The state now, for a front end that must show it before the first event (blocks up to 2 s).
    pub fn snapshot(&self) -> Option<PlayerSnapshot> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.call(move |c| {
            let _ = tx.send(c.snapshot());
        });
        rx.recv_timeout(Duration::from_secs(2)).ok()
    }

    /// Every snapshot field again, for a front end that just appeared.
    pub fn refresh(&self) {
        self.call(|c| c.changes = Changes::all());
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Quit);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[derive(Default)]
struct Downloads {
    user: Option<Download>,
    cache: Option<Download>,
}

struct Download {
    task: tokio::task::JoinHandle<()>,
    part: PathBuf,
}

async fn run(
    mut core: PlayerCore,
    mut rx: mpsc::UnboundedReceiver<Msg>,
    tx: mpsc::UnboundedSender<Msg>,
    listener: Arc<dyn PlayerListener>,
    covers: Arc<CoverCache>,
    waves: PathBuf,
    subwave: Arc<SubwaveClient>,
) {
    // covers and downloads: no 30 s cap on the whole transfer, only on silence (as Qt's transfer timeout did)
    let http = crate::http_client().connect_timeout(Duration::from_secs(15)).read_timeout(Duration::from_secs(15)).build().expect("http");
    let mut download = Downloads::default();
    core.clock_ms = wall_ms();
    core.start();
    flush(&mut core, &tx, &*listener, &http, &mut download);
    let mut next_tick = Instant::now() + Duration::from_millis(core.tick_ms());
    loop {
        tokio::select! {
            m = rx.recv() => {
                let Some(m) = m else { break };
                core.clock_ms = wall_ms();
                match m {
                    Msg::Run(f) => f(&mut core),
                    Msg::Deck(k, e) => core.deck_event(k, e),
                    Msg::Sync(e) => core.on_sync(&e),
                    Msg::Subwave(s) => core.on_subwave(&s, s.elapsed_ms(local_now())),
                    Msg::Api(then, r) => then(&mut core, r),
                    Msg::PersistNow(done) => {
                        let calls = core.persist_now().into_iter().filter_map(|j| match j {
                            Job::Api { client, method, query, .. } => Some(async move { let _ = client.call(&method, &query).await; }),
                            _ => None,
                        });
                        let _ = tokio::time::timeout(Duration::from_millis(1500), futures_util::future::join_all(calls)).await;
                        let _ = done.send(());
                    }
                    Msg::Cover(id, size, done) => {
                        let url = match id.strip_prefix("sw:") {
                            Some(sw) => url::Url::parse(&subwave.cover_url(sw)).ok(),
                            None => core.cover_url(&id, size),
                        };
                        let offline = !core.is_online();
                        let (covers, http) = (covers.clone(), http.clone());
                        tokio::spawn(async move {
                            let bytes = match url {
                                Some(u) => covers.get(&http, &u, offline).await,
                                None => None,
                            };
                            done(bytes);
                        });
                    }
                    Msg::Waveform(id, done) => {
                        let relay = crate::queue::text(core.options.get("relayUrl").unwrap_or(&Value::Null));
                        let url = (core.is_online() && !id.is_empty() && !id.starts_with("sw:"))
                            .then(|| crate::relay::url(&core.login.server, &relay, &["waveform", &id]))
                            .flatten();
                        let (login, http, waves) = (core.login.clone(), http.clone(), waves.clone());
                        tokio::spawn(async move {
                            let held = crate::waveform::cached(&waves, &login.server, &id);
                            done(match (held, url) {
                                (None, Some(u)) => crate::waveform::fetch(&http, u, &login, &waves, &id).await,
                                (held, _) => held,
                            });
                        });
                    }
                    Msg::TranscodeFormats(done) => {
                        let relay = crate::queue::text(core.options.get("relayUrl").unwrap_or(&Value::Null));
                        let url = crate::relay::url(&core.login.server, &relay, &["transcodings"]);
                        let (client, http) = (core.client.clone().filter(|_| core.is_online()), http.clone());
                        tokio::spawn(async move {
                            let Some(c) = client else { return done(vec![]) };
                            let body = match url {
                                Some(u) => crate::relay::get(&http, u, c.login()).await,
                                None => None,
                            };
                            done(match body.and_then(|b| crate::relay::transcodings(&b)) {
                                Some(profiles) => profiles,
                                None => c.transcode_formats().await,
                            });
                        });
                    }
                    Msg::Quit => break,
                }
            }
            _ = tokio::time::sleep_until(next_tick) => {
                core.clock_ms = wall_ms();
                core.tick();
                next_tick = Instant::now() + Duration::from_millis(core.tick_ms());
            }
        }
        flush(&mut core, &tx, &*listener, &http, &mut download);
        // playing from now on: the position moves from now, not from the next slow tick
        next_tick = next_tick.min(Instant::now() + Duration::from_millis(core.tick_ms()));
    }
}

/// Runs what the core queued, then tells the front end what changed.
fn flush(core: &mut PlayerCore, tx: &mpsc::UnboundedSender<Msg>, listener: &dyn PlayerListener, http: &reqwest::Client, download: &mut Downloads) {
    for job in core.take_jobs() {
        let tx = tx.clone();
        match job {
            Job::Api { client, method, query, then } => {
                tokio::spawn(async move {
                    let r = client.call(&method, &query).await;
                    let _ = tx.send(Msg::Api(then, r));
                });
            }
            Job::Timer { ms, then } => {
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(ms)).await;
                    let _ = tx.send(Msg::Run(then));
                });
            }
            Job::Download { client, id, path } => {
                let part = part_of(&path);
                let (http, p) = (http.clone(), part.clone());
                let task = tokio::spawn(async move {
                    let url = client.url("download", &[("id".into(), id)]);
                    let r = fetch_to(&http, &client, url, &path, &p, Some(&tx)).await;
                    let _ = tx.send(Msg::Run(Box::new(move |c| c.download_done(r))));
                });
                download.user = Some(Download { task, part });
            }
            Job::CacheSong { client, query, path } => {
                let part = part_of(&path);
                let (http, p) = (http.clone(), part.clone());
                let task = tokio::spawn(async move {
                    let ok = fetch_to(&http, &client, client.url("stream", &query), &path, &p, None).await.is_ok();
                    let _ = tx.send(Msg::Run(Box::new(move |c| c.cache_done(ok))));
                });
                download.cache = Some(Download { task, part });
            }
            Job::Work { run } => {
                tokio::task::spawn_blocking(move || {
                    let then = run();
                    let _ = tx.send(Msg::Run(then));
                });
            }
            Job::CancelDownload => {
                if let Some(d) = download.cache.take().filter(|_| core.caching) {
                    d.task.abort();
                    let _ = std::fs::remove_file(d.part);
                    core.cache_done(false);
                }
                if let Some(d) = download.user.take().filter(|_| core.downloading) {
                    d.task.abort();
                    let _ = std::fs::remove_file(d.part);
                    core.download_done(Err("iptal edildi".into()));
                }
            }
        }
    }
    for (id, reply, call) in core.take_replies() {
        listener.on_reply(id, reply, call);
    }
    let ch = core.take_changes();
    if ch.any() {
        listener.on_player(ch, core.snapshot());
    }
}

fn part_of(path: &std::path::Path) -> PathBuf {
    let mut part = path.to_path_buf().into_os_string();
    part.push(".part");
    part.into()
}

fn cant_write() -> String {
    crate::i18n::tr("dosya yazılamıyor", "can't write the file").into()
}

/// Streams url to part, then renames it to path: the file appears only when complete; a failure leaves nothing behind.
/// Ok(file name).
async fn fetch_to(
    http: &reqwest::Client,
    client: &Subsonic,
    url: url::Url,
    path: &std::path::Path,
    part: &std::path::Path,
    progress: Option<&mpsc::UnboundedSender<Msg>>,
) -> Result<String, String> {
    let r = fetch_part(http, client, url, path, part, progress).await;
    if r.is_err() {
        let _ = tokio::fs::remove_file(part).await;
    }
    r
}

async fn fetch_part(
    http: &reqwest::Client,
    client: &Subsonic,
    url: url::Url,
    path: &std::path::Path,
    part: &std::path::Path,
    progress: Option<&mpsc::UnboundedSender<Msg>>,
) -> Result<String, String> {
    let l = client.login();
    let secrets = [l.token.as_str(), l.salt.as_str()];
    let mut resp = http.get(url).send().await.map_err(|e| failure_text("", 0, &e.to_string(), &secrets))?;
    let status = resp.status().as_u16();
    // a Subsonic error comes back as a small JSON/XML body instead of the file: keep it for its message
    let ctype = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    if status != 200 || ctype.contains("json") || ctype.contains("xml") {
        let body = resp.bytes().await.unwrap_or_default();
        let msg = serde_json::from_slice::<Value>(&body).ok().and_then(|v| v["subsonic-response"]["error"]["message"].as_str().map(String::from));
        return Err(failure_text(&msg.unwrap_or_default(), if status == 200 { 0 } else { status }, "", &secrets));
    }
    let total = resp.content_length();
    let mut f = tokio::fs::File::create(part).await.map_err(|_| cant_write())?;
    let (mut got, mut shown) = (0u64, std::time::Instant::now());
    while let Some(chunk) = resp.chunk().await.map_err(|e| failure_text("", 0, &e.to_string(), &secrets))? {
        f.write_all(&chunk).await.map_err(|_| cant_write())?;
        got += chunk.len() as u64;
        if let Some(tx) = progress
            && shown.elapsed() > Duration::from_millis(250)
        {
            shown = std::time::Instant::now();
            let _ = tx.send(Msg::Run(Box::new(move |c| c.download_progress(got, total))));
        }
    }
    f.flush().await.map_err(|_| cant_write())?;
    drop(f);
    tokio::fs::rename(part, path).await.map_err(|_| cant_write())?;
    Ok(path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
}
