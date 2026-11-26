//! The server side of the player: login, the saved queue, listening together, calls from themes, demo, downloads.
use super::state::{Job, PlayerCore, queue_key};
use crate::queue::{apply_rating, apply_star, for_themes, login_error, next_index, queue_window, readable, subsonic_query, text, writable};
use crate::i18n::{english, tr};
use crate::subsonic::Login;
use crate::subwave::SubwaveSnapshot;
use crate::sync::{SyncEvent, relay_url};
use serde_json::{Map, Value, json};
use std::path::PathBuf;
use std::sync::Arc;

impl PlayerCore {
    /// At start: the queue from disk (paused, so the song is open before the server answers), then connect when a
    /// login is saved.
    pub fn start(&mut self) {
        if self.login.token.is_empty() {
            return;
        }
        self.load_local_queue();
        self.connect_server();
    }

    /// Token auth: the password itself is never sent or stored.
    pub fn login(&mut self, server: &str, user: &str, password: &str) {
        match Login::from_password(server, user, password) {
            Err(why) => {
                self.login_failed = true;
                self.set_status(why);
            }
            Ok(l) => {
                self.fresh = true;
                self.set_login(l);
                self.connect_server();
            }
        }
    }

    pub fn logout(&mut self) {
        self.stop_station(); // unload the stream: its URL may hold a password
        self.set_playing(false);
        if let Some(j) = self.report_stopped() {
            self.jobs.push(j); // built before the token goes
        }
        self.generation += 1; // cancels in-flight connects and pending retries
        self.persist_seq += 1; // a pending queue save must not go out with the next login's token
        if self.downloading || self.caching {
            self.jobs.push(Job::CancelDownload);
        }
        self.online = false;
        self.offline = false;
        self.playback_report = false;
        self.report = Default::default();
        let mut l = self.login.clone();
        l.token.clear();
        self.set_login(l);
        self.sync.stop(true);
        self.leaving = false;
        let _ = std::fs::remove_file(self.local_queue_file());
        self.local_key.clear();
        self.settings.remove("token");
        self.queue = Arc::default(); // never the next user's, nor saved to their server
        self.before.clear();
        self.extend_seq += 1;
        self.extending = false;
        self.load(-1, 0.0);
        self.connecting = false;
        self.fresh = false;
        self.login_failed = false;
        self.set_status(tr("Çıkış yapıldı", "Logged out"));
    }

    pub(super) fn connect_server(&mut self) {
        if self.online {
            return; // already connected (or demo)
        }
        self.generation += 1;
        let g = self.generation;
        self.connecting = true;
        self.login_failed = false;
        if !self.offline {
            self.set_status(tr("Bağlanıyor…", "Connecting…"));
        }
        self.api("ping", vec![], false, move |c, r| {
            if g != c.generation {
                return;
            }
            if r.is_none() {
                c.connecting = false;
                let why = login_error(c.last_error, &c.last_msg);
                // wrong credentials, or any failure of a login typed just now: back to the form with the reason
                if c.last_error == 40 || c.last_error == 41 || c.fresh {
                    c.logout();
                    c.login_failed = true;
                    return c.set_status(why);
                }
                c.login_failed = true;
                c.set_status(why);
                c.enter_offline();
                return c.retry(g); // network or server down: keep the saved login and try again
            }
            let l = c.login.clone();
            for (k, v) in [("url", &l.server), ("user", &l.user), ("salt", &l.salt), ("token", &l.token)] {
                c.settings.set(k, v.as_str().into()); // the settings file keeps itself 0600 once it holds a token
            }
            c.online = true;
            c.offline = false;
            c.connecting = false;
            c.fresh = false;
            c.set_status(""); // connected: nothing to say
            let relay = relay_url(&l.server, &text(c.options.get("relayUrl").unwrap_or(&Value::Null)));
            c.sync.start(relay, l.user, l.token, l.salt);
            c.restore_queue();
            c.api("getOpenSubsonicExtensions", vec![], true, move |c, r| {
                if g != c.generation {
                    return;
                }
                let ext = r.and_then(|r| r["openSubsonicExtensions"].as_array().cloned()).unwrap_or_default();
                if ext.iter().any(|e| e["name"] == "playbackReport") {
                    c.playback_report = true;
                }
            });
        });
    }

    fn retry(&mut self, g: u64) {
        if g != self.generation {
            return;
        }
        if !self.offline {
            let st = format!("{} · {}", self.status, tr("5 sn sonra yeniden denenecek", "retrying in 5 s"));
            self.set_status(st);
        }
        self.timer(5000, move |c| {
            if g == c.generation {
                c.connect_server();
            }
        });
    }

    /// The queue from disk gives way only while untouched: same songs, same place, paused.
    fn untouched(&self) -> bool {
        !self.local_key.is_empty()
            && !self.playing
            && queue_key(&self.queue, self.idx) == self.local_key
            && (self.position() - self.local_pos).abs() < 1.0
    }

    /// Where we left off last time (any device of this user), paused.
    pub(super) fn restore_queue(&mut self) {
        if (!self.queue.is_empty() && !self.untouched()) || self.following() {
            return;
        }
        let g = self.generation;
        self.api("getPlayQueue", vec![], false, move |c, r| {
            let Some(r) = r else { return };
            let pq = &r["playQueue"];
            let mut q = pq["entry"].as_array().cloned().unwrap_or_default();
            if g != c.generation || q.is_empty() || (!c.queue.is_empty() && !c.untouched()) || c.following() || !c.station.is_empty() {
                return;
            }
            c.local_key.clear();
            let cur = text(&pq["current"]);
            let mut i = q.iter().rposition(|s| text(&s["id"]) == cur).unwrap_or(0);
            q = queue_window(&q, &mut i); // another client may have saved more than the relay takes
            let xf = c.xfade;
            // the same song and place as from disk: the deck stays as it is
            c.apply(q, i as i64, false, xf, 0.0, pq["position"].as_f64().unwrap_or(0.0) / 1000.0);
            c.publish(); // the relay may already hold our empty state from before the reply
        });
    }

    /// The queue of the last save, paused, before connecting (the server's comes only after ping and the reply).
    fn load_local_queue(&mut self) {
        let Ok(bytes) = std::fs::read(self.local_queue_file()) else { return };
        let Ok(o) = serde_json::from_slice::<Value>(&bytes) else { return };
        let q = o["queue"].as_array().cloned().unwrap_or_default();
        if q.is_empty() || text(&o["server"]) != self.login.server || text(&o["user"]) != self.login.user {
            return;
        }
        self.local_pos = o["position"].as_f64().unwrap_or(0.0);
        let i = o["index"].as_i64().unwrap_or(0).clamp(0, q.len() as i64 - 1);
        let (xf, pos) = (self.xfade, self.local_pos);
        self.apply(q, i, false, xf, 0.0, pos);
        self.local_key = queue_key(&self.queue, self.idx);
    }

    /// On quit: our play queue to the server and the server's now playing cleared. The calls go out at once; the
    /// caller waits on them (at most 1.5 s).
    pub fn persist_now(&mut self) -> Vec<Job> {
        self.persist_seq += 1; // the debounced save is this one now
        let mut out: Vec<Job> = self.report_stopped().into_iter().collect();
        if self.online && !self.login.token.is_empty() && !self.following() && !self.queue.is_empty() {
            if let Some(client) = self.client.clone() {
                out.push(Job::Api { client, method: "savePlayQueue".into(), query: self.play_queue_query(), then: Box::new(|_, _| {}) });
            }
            self.save_local_queue();
        } else if self.offline {
            self.save_local_queue(); // where we got to; the server's is older
        }
        out
    }

    // ------------------------------------------------------------ listening together

    pub fn on_sync(&mut self, e: &SyncEvent) {
        match e {
            SyncEvent::StateReceived(s) => self.on_state(s),
            SyncEvent::CommandReceived { op, args } => self.on_command(op, args),
            SyncEvent::Connected => self.publish(), // the first state after a (re)connect carries the queue
            SyncEvent::Ended(_) => {
                // stopped following: we are our own root now, with our own modes
                if std::mem::take(&mut self.leaving) {
                    self.stop_here();
                }
                self.shuffle = self.settings.get_bool("shuffle", false);
                self.repeat = self.settings.get_i64("repeat", 0);
                self.before.clear();
                self.drop_prefetch(); // the root's repeat may have queued the wrong next song
                self.publish();
                self.state_changed();
            }
            SyncEvent::Failed(code) => {
                self.set_status(if code == "denied" { "Host doesn't allow control".to_string() } else { format!("Sync: {code}") })
            }
            _ => {}
        }
    }

    fn on_state(&mut self, s: &Map<String, Value>) {
        let q = match s.get("queue") {
            Some(q) => q.as_array().cloned().unwrap_or_default(),
            None => (*self.queue).clone(),
        };
        self.shuffle = s.get("shuffle").and_then(Value::as_bool).unwrap_or(false); // the root's modes, shown here; not saved
        self.repeat = s.get("repeat").and_then(Value::as_i64).unwrap_or(0);
        let num = |k: &str| s.get(k).and_then(Value::as_f64).unwrap_or(0.0);
        let index = s.get("index").and_then(Value::as_i64).unwrap_or(0);
        let st = text(s.get("station").unwrap_or(&Value::Null));
        if !st.is_empty() {
            // the root is on a stream: keep its queue, open the same station
            self.queue = Arc::new(q);
            self.idx = index;
            self.resume_pos = num("position");
            if st != text(self.station.get("id").unwrap_or(&Value::Null)) && st != self.want_station {
                self.set_playing(false); // no queue song while the root is on the stream, even if ours fails to open
                self.open_station(&st);
            }
            self.state_changed();
            return;
        }
        self.end_station();
        let playing = s.get("playing").and_then(Value::as_bool).unwrap_or(false);
        let xfade = s.get("xfade").and_then(Value::as_i64).unwrap_or(0);
        self.apply(q, index, playing, xfade, num("anchor"), num("position"));
    }

    /// A follower's action, already allowed by the relay (allowControl) and checked for a stale queue (SyncClient).
    fn on_command(&mut self, op: &str, a: &Value) {
        let i = |k: &str| a[k].as_i64().unwrap_or(0);
        let songs = || a["songs"].as_array().cloned().unwrap_or_default();
        match op {
            "toggle" => self.toggle(),
            "next" => self.next(),
            "prev" => self.prev(),
            "seek" => self.seek(a["s"].as_f64().unwrap_or(0.0)),
            "jump" => self.jump(i("i")),
            "remove" => self.remove(i("i")),
            "move" => self.move_song(i("from"), i("to")),
            "clearUpcoming" => self.clear_upcoming(),
            "play" => self.play(songs(), i("start")),
            "add" => self.add(songs(), i("where") as i32),
            "station" => self.play_station(&text(&a["id"])),
            "shuffle" => self.set_shuffle(a["on"].as_bool().unwrap_or(false)),
            "repeat" => self.set_repeat(i("mode")),
            _ => {}
        }
    }

    pub(super) fn apply(&mut self, q: Vec<Value>, mut i: i64, play: bool, xfade: i64, anchor: f64, position: f64) {
        if xfade > 0 && self.exclusive {
            self.set_exclusive(false); // two decks can't share an exclusive device
            self.set_status("Exclusive off: a peer turned crossfade on");
        }
        let k = self.cur;
        let d = &self.decks[k];
        // gapless next, by the old queue
        let queued = if d.appended && self.valid(d.appended_idx) { self.song_id(d.appended_idx) } else { String::new() };
        self.queue = Arc::new(q);
        self.xfade = xfade;
        let mut pos = if play { (self.server_now() - anchor) / 1000.0 } else { position };
        // joined late: walk past tracks that already finished
        while i >= 0 && i + 1 < self.queue.len() as i64 {
            let dur = self.song_dur(i);
            let step = dur - self.fade_of(dur);
            if step <= 0.0 || pos <= step {
                break; // no duration known: nothing to walk by
            }
            pos -= step;
            i += 1;
        }
        // A follower gets the root's state often (seeks, re-anchors): the gapless next stays queued while it is still next.
        let d = &self.decks[k];
        let still_next = !queued.is_empty()
            && xfade == 0
            && self.valid(i)
            && d.id == self.song_id(i)
            && next_index(i, self.queue.len() as i64, self.repeat as i32) == d.appended_idx
            && self.valid(d.appended_idx)
            && self.song_id(d.appended_idx) == queued;
        if !still_next {
            self.drop_prefetch();
        }
        self.playing = play;
        if !self.valid(i) || self.decks[k].id != self.song_id(i) {
            self.load(i, pos);
        } else {
            self.idx = i;
            if self.decks[k].loading {
                self.decks[k].pending_seek = pos; // FILE_LOADED seeks there; a seek now would run the FLAC search twice
            } else if (self.decks[k].pos - pos).abs() > if self.decks[k].live_transcode { 2.0 } else { 0.5 } {
                self.seek_deck(k, pos);
            }
        }
        self.anchor = self.server_now() - pos * 1000.0;
        self.set_playing(play);
    }

    /// What SUB/WAVE says the station plays: the title goes to the online list, the library track and how far the
    /// listener is into it go to the server as now playing (Discord) and count as a play.
    pub fn on_subwave(&mut self, s: &SubwaveSnapshot, elapsed_ms: i64) {
        let now = if s.active { s.now.clone() } else { Value::Null };
        let radio = json!({"title": text(&now["title"]), "artist": text(&now["artist"])}).as_object().cloned().unwrap();
        self.radio_id = text(&now["id"]);
        self.radio_pos_ms = elapsed_ms as f64;
        self.radio_dur = now["duration"].as_f64().unwrap_or(0.0);
        if radio != self.radio_now {
            self.radio_now = radio;
            if !self.station.is_empty() {
                self.publish(); // skipped while following: the root's state is the truth
            }
        }
    }

    // ------------------------------------------------------------ calls from themes

    /// Player.get (write false) / Player.set (write true). false: not allowed, the theme gets null at once. Otherwise
    /// the reply comes through take_replies: (id, reply or None for null, false when the login changed meanwhile).
    pub fn request(&mut self, id: u64, method: &str, params: Map<String, Value>, write: bool) -> bool {
        if !(if write { writable(method) } else { readable(method) }) {
            return false;
        }
        if !self.online {
            self.replies.push((id, None, true));
            return true;
        }
        let (g, m) = (self.generation, method.to_string());
        let query = subsonic_query(&params);
        self.api(method, query, false, move |c, r| {
            if g != c.generation {
                return c.replies.push((id, None, false));
            }
            let Some(r) = r else { return c.replies.push((id, None, true)) };
            // the queue keeps song copies: let them (and followers) know the new rating or favorite
            let changed = match m.as_str() {
                "setRating" => {
                    let rating = params["rating"].as_i64().or_else(|| text(&params["rating"]).parse().ok()).unwrap_or(0);
                    apply_rating(Arc::make_mut(&mut c.queue).as_mut_slice(), &text(&params["id"]), rating)
                }
                // albums and artists come as albumId and artistId
                "star" | "unstar" => {
                    let ids = subsonic_query(&params).into_iter().filter(|(k, _)| k == "id");
                    ids.map(|(_, id)| apply_star(Arc::make_mut(&mut c.queue).as_mut_slice(), &id, m == "star")).sum()
                }
                _ => 0,
            };
            if changed > 0 {
                c.publish();
                c.state_changed();
            }
            c.replies.push((id, Some(for_themes(&m, r)), true));
        });
        true
    }

    /// --demo: fake online state with sample songs, for designing themes without a server.
    pub fn demo(&mut self) {
        self.load(-1, 0.0); // drop the queue the disk may have opened: demo songs only
        let rows = [
            "Şımarık|Tarkan|Ölürüm Sana|flac|24|96000|230",
            "Gidiyorum|Sezen Aksu|Deniz Yıldızı|flac|16|44100|251",
            "Bir Derdim Var|mor ve ötesi|Dünya Yalan Söylüyor|flac|24|48000|268",
            "Teardrop|Massive Attack|Mezzanine|flac|16|44100|330",
            "Ben Böyleyim|Athena|Holigan|mp3|0|44100|212",
            "Firuze|Sezen Aksu|Firuze|flac|24|192000|244",
            "Islak Islak|Barış Manço|Sözüm Meclisten Dışarı|flac|16|44100|261",
        ];
        let q = rows
            .iter()
            .map(|r| {
                let f: Vec<&str> = r.split('|').collect();
                let n = |s: &str| s.parse::<i64>().unwrap_or(0);
                json!({"id": f[0], "title": f[0], "artist": f[1], "album": f[2], "suffix": f[3], "bitDepth": n(f[4]),
                       "samplingRate": n(f[5]), "duration": n(f[6]), "genre": "Pop"})
            })
            .collect();
        self.queue = Arc::new(q);
        self.idx = 0;
        self.online = true;
        let mut l = self.login.clone();
        l.token.clear(); // in memory only: demo songs must never reach a real server's play queue
        self.set_login(l);
        self.sync.demo();
        self.set_status("Demo · sunucu yok");
        self.state_changed();
    }

    /// Song: the original file; album / playlist: a zip. One at a time.
    pub fn download(&mut self, id: &str, path: PathBuf) {
        if self.downloading {
            return self.set_status(tr("Bir indirme zaten sürüyor.", "A download is already running."));
        }
        let Some(client) = self.client.clone().filter(|_| !id.is_empty()) else {
            return self.set_status(tr("İndirme başarısız: dosya yazılamıyor", "Download failed: can't write the file"));
        };
        self.downloading = true;
        self.jobs.push(Job::Download { client, id: id.into(), path });
    }

    pub fn download_progress(&mut self, got: u64, total: Option<u64>) {
        self.set_status(match total {
            Some(t) if t > 0 && english() => format!("Downloading… {}%", got * 100 / t),
            Some(t) if t > 0 => format!("İndiriliyor… %{}", got * 100 / t),
            _ => format!("{} {} MB", tr("İndiriliyor…", "Downloading…"), got >> 20), // a zip comes without a length
        });
    }

    /// Ok(file name) or Err(why).
    pub fn download_done(&mut self, r: Result<String, String>) {
        self.downloading = false;
        match r {
            Ok(name) => self.set_status(format!("{} {name}", tr("İndirildi:", "Downloaded:"))),
            Err(why) => self.set_status(format!("{} {why}", tr("İndirme başarısız:", "Download failed:"))),
        }
    }
}
