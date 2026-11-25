use super::*;
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn snapshot_maps_to_record() {
    let mut s = PlayerSnapshot { status: "x".into(), index: 2, playing: true, repeat: 1, bitrate: 900, ..Default::default() };
    s.queue = Arc::new(vec![Value::Null, Value::Null, serde_json::json!({"id": "a", "title": "Şımarık"})]);
    s.current = serde_json::json!({"id": "a", "title": "Şımarık"});
    s.options.insert("dj".into(), true.into());
    let r = player_state(&s);
    assert_eq!((r.status.as_str(), r.index, r.playing, r.repeat, r.bitrate, r.queue_length), ("x", 2, true, 1, 900, 3));
    let cur: Value = serde_json::from_str(&r.current_json).unwrap();
    assert_eq!(cur["title"], "Şımarık");
    assert_eq!(r.options_json, r#"{"dj":true}"#);
    assert_eq!(r.station_json, "{}");
}

#[test]
fn sync_events_keep_their_payload() {
    assert_eq!(sync_note(SyncEvent::ListenerJoined("ali".into())), SyncNote::ListenerJoined { name: "ali".into() });
    assert_eq!(sync_note(SyncEvent::Failed("auth".into())), SyncNote::Failed { code: "auth".into() });
    assert_eq!(sync_note(SyncEvent::StateReceived(Map::new())), SyncNote::Changed);
}

#[derive(Default)]
struct Rec {
    calls: Mutex<Vec<String>>,
}

impl DeckOut for Rec {
    fn load(&self, url: String, paused: bool) {
        self.calls.lock().unwrap().push(format!("load {url} {paused}"));
    }
    fn append(&self, url: String) {
        self.calls.lock().unwrap().push(format!("append {url}"));
    }
    fn clear_appended(&self) {}
    fn drop_first(&self) {}
    fn set_paused(&self, paused: bool) {
        self.calls.lock().unwrap().push(format!("paused {paused}"));
    }
    fn seek(&self, _: f64) {}
    fn set_volume(&self, _: f64) {}
    fn set_speed(&self, _: f64) {}
    fn stop(&self) {}
}

struct Decks(Vec<Arc<Rec>>);

impl DeckFactory for Decks {
    fn make_deck(&self, index: u32) -> Arc<dyn DeckOut> {
        self.0[index as usize].clone()
    }
}

struct Ui(mpsc::Sender<(PlayerChanges, PlayerState, Option<String>)>);

impl CoreListener for Ui {
    fn on_player(&self, changes: PlayerChanges, state: PlayerState, queue_json: Option<String>) {
        let _ = self.0.send((changes, state, queue_json));
    }
    fn on_reply(&self, _: u64, _: String, _: bool) {}
    fn on_sync(&self, _: SyncNote, _: SyncState) {}
    fn on_subwave(&self, _: SubwaveState) {}
    fn on_subwave_tick(&self, _: i64) {}
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rela-android-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Waits for an event the predicate accepts.
fn until(rx: &mpsc::Receiver<(PlayerChanges, PlayerState, Option<String>)>, ok: impl Fn(&PlayerChanges, &PlayerState, &Option<String>) -> bool) -> PlayerState {
    loop {
        let (c, s, q) = rx.recv_timeout(Duration::from_secs(3)).expect("event");
        assert_eq!(c.queue, q.is_some(), "the queue comes with its change only");
        if ok(&c, &s, &q) {
            return s;
        }
    }
}

#[test]
fn deck_events_reach_the_core_and_queue_comes_once() {
    let dir = scratch("decks");
    let decks = vec![Arc::new(Rec::default()), Arc::new(Rec::default())];
    let (tx, rx) = mpsc::channel();
    let core = Core::new(dir.join("files").display().to_string(), dir.join("cache").display().to_string(), Arc::new(Decks(decks.clone())), Arc::new(Ui(tx)));
    core.demo();
    let s = until(&rx, |c, _, q| c.queue && q.as_deref().is_some_and(|q| q.contains("Gidiyorum")));
    assert_eq!(s.queue_length, 7);
    core.toggle();
    until(&rx, |_, s, _| s.playing);
    let loaded = || decks.iter().flat_map(|d| d.calls.lock().unwrap().clone()).filter(|c| c.starts_with("load")).count();
    assert_eq!(loaded(), 1, "{:?}", decks[0].calls.lock().unwrap());
    // the deck that loaded says the file is there, then that it ended: the core moves to song 2
    let k = if decks[0].calls.lock().unwrap().iter().any(|c| c.starts_with("load")) { 0 } else { 1 };
    core.deck_event(k, DeckNote::StartFile);
    core.deck_event(k, DeckNote::FileLoaded);
    core.deck_event(k, DeckNote::Duration { seconds: 230.0 });
    core.deck_event(k, DeckNote::EndFile { reason: DeckEnd::Eof, error: String::new() });
    let s = until(&rx, |_, s, _| s.index == 1);
    assert!(s.current_json.contains("Gidiyorum"));
    assert_eq!(core.setting("token".into()), "");
    drop(core);
    let _ = std::fs::remove_dir_all(&dir);
}
