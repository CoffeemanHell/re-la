//! Socket-free relay client logic: what to send and what to tell the front end
//! for each relay message and each user action. `SyncClient` (client.rs) owns the socket and the timers.
use super::{ClockSample, clock_offset};
use rela_proto::{In, VERSION};
use serde_json::{Map, Value, json};

const CLOCK_SAMPLES: usize = 5;
/// After a server reboot the root may reconnect up to ~20 s after us: ask this many times, 2 s apart.
pub const REFOLLOW_TRIES: u32 = 10;

/// What the front end hears about.
#[derive(Debug, Clone, PartialEq)]
pub enum SyncEvent {
    Changed,
    UsersChanged,
    Connected,
    StateReceived(Map<String, Value>),
    CommandReceived { op: String, args: Value },
    Ended(String),
    ListenerJoined(String),
    ListenerLeft(String),
    Failed(String),
}

/// What the client should do.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Send(In),
    Event(SyncEvent),
    /// (Re)arm the 30 s clock timer.
    StartClockTimer,
    StartRefollowTimer,
    StopRefollowTimer,
    /// Retrying would not help (auth, version, replaced): stop reconnecting until the next start.
    GiveUp,
}

#[derive(Debug, Default)]
pub struct SyncMachine {
    me: String,
    following: String,
    users: Vec<Value>,
    listeners: Vec<String>,
    listeners_known: bool, // a users list arrived since the last welcome
    last_state: Map<String, Value>, // received (follower)
    last_sent: Map<String, Value>,
    last_full: Map<String, Value>, // last full published state
    sent_qrev: i64,
    samples: Vec<ClockSample>,
    offset: i64,
    available: bool,
    allow: bool,
    hidden: bool,
    refollow: bool, // follow re-sent after a reconnect, waiting for the answer
    refollow_left: u32,
}

fn qrev_of(m: &Map<String, Value>) -> i64 {
    m.get("qrev").and_then(Value::as_f64).map_or(0, |q| q as i64)
}

impl SyncMachine {
    pub fn new(allow_control: bool, hidden: bool) -> Self {
        SyncMachine { allow: allow_control, hidden, sent_qrev: -1, ..Default::default() }
    }

    pub fn hello(&self, user: &str, token: &str, salt: &str) -> In {
        In::Hello { v: VERSION, u: user.into(), t: token.into(), s: salt.into(), hidden: self.hidden }
    }

    pub fn available(&self) -> bool {
        self.available
    }
    pub fn users(&self) -> &[Value] {
        &self.users
    }
    pub fn listeners(&self) -> &[String] {
        &self.listeners
    }
    pub fn following(&self) -> &str {
        &self.following
    }
    pub fn me(&self) -> &str {
        &self.me
    }
    pub fn can_control(&self) -> bool {
        self.following.is_empty() || self.last_state.get("allowControl").and_then(Value::as_bool).unwrap_or(false)
    }
    pub fn allow_control(&self) -> bool {
        self.allow
    }
    pub fn hidden(&self) -> bool {
        self.hidden
    }
    /// Relay clock - local clock, ms; 0 before the first sample.
    pub fn offset(&self) -> i64 {
        self.offset
    }
    /// What `publish` last handed to the socket.
    pub fn last_sent(&self) -> &Map<String, Value> {
        &self.last_sent
    }
    /// New connection: the next publish carries the queue.
    pub fn reset_sent(&mut self) {
        self.sent_qrev = -1;
    }

    /// Index commands from a follower that saw an older queue would hit the wrong song: drop them.
    pub fn accept_command(&self, op: &str, qrev: i64) -> bool {
        !matches!(op, "jump" | "remove" | "move") || qrev == qrev_of(&self.last_full)
    }

    fn measure(local_now: i64) -> Effect {
        Effect::Send(In::Time { c: local_now as f64 })
    }

    pub fn on_text(&mut self, text: &str, local_now: i64) -> Vec<Effect> {
        let Ok(Value::Object(m)) = serde_json::from_str::<Value>(text) else { return vec![] };
        let str_of = |k: &str| m.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let mut fx = vec![];
        match m.get("type").and_then(Value::as_str).unwrap_or("") {
            "welcome" => {
                self.available = true;
                self.me = str_of("you");
                self.listeners_known = false;
                self.reset_sent();
                self.samples.clear();
                fx.push(Self::measure(local_now));
                fx.push(Effect::StartClockTimer);
                if !self.following.is_empty() {
                    // relay restarted or we reconnected
                    self.refollow = true;
                    self.refollow_left = REFOLLOW_TRIES;
                    let user = self.following.clone();
                    fx.extend(self.follow(&user));
                }
                fx.push(Effect::Event(SyncEvent::Connected));
                fx.push(Effect::Event(SyncEvent::Changed));
            }
            "time" => {
                let num = |k: &str| m.get(k).and_then(Value::as_f64).unwrap_or(0.0) as i64;
                self.samples.push(ClockSample { c: num("c"), s: num("s"), r: local_now });
                self.offset = clock_offset(&self.samples);
                if self.samples.len() < CLOCK_SAMPLES {
                    fx.push(Self::measure(local_now));
                }
            }
            "users" => {
                self.users = m.get("users").and_then(Value::as_array).cloned().unwrap_or_default();
                // hidden: the list says nothing about who follows us, so keep the last known
                let mut now = self.listeners.clone();
                if !self.hidden {
                    now = self
                        .users
                        .iter()
                        .filter(|u| {
                            !self.me.is_empty()
                                && u["following"].as_str().is_some_and(|f| f.to_lowercase() == self.me.to_lowercase())
                        })
                        .map(|u| u["user"].as_str().unwrap_or("").to_string())
                        .collect();
                }
                let before = std::mem::replace(&mut self.listeners, now);
                let toast = std::mem::replace(&mut self.listeners_known, true);
                fx.push(Effect::Event(SyncEvent::UsersChanged));
                if toast {
                    for n in self.listeners.iter().filter(|n| !before.contains(n)) {
                        fx.push(Effect::Event(SyncEvent::ListenerJoined(n.clone())));
                    }
                    for n in before.iter().filter(|n| !self.listeners.contains(n)) {
                        fx.push(Effect::Event(SyncEvent::ListenerLeft(n.clone())));
                    }
                }
            }
            "following" => {
                self.refollow = false;
                self.following = str_of("root");
                fx.push(Effect::Event(SyncEvent::Changed));
            }
            "state" => {
                self.last_state = m.clone();
                fx.push(Effect::Event(SyncEvent::StateReceived(m.clone())));
                fx.push(Effect::Event(SyncEvent::Changed));
            }
            "cmd" => {
                let op = str_of("op");
                if self.accept_command(&op, qrev_of(&m)) {
                    let args = m.get("args").cloned().unwrap_or_else(|| json!({}));
                    fx.push(Effect::Event(SyncEvent::CommandReceived { op, args }));
                }
            }
            "ended" => fx.extend(self.end_following(&str_of("reason"))),
            "error" => {
                let code = str_of("code");
                if code == "not-found" && self.refollow {
                    self.refollow_left = self.refollow_left.saturating_sub(1);
                    if self.refollow_left > 0 {
                        return vec![Effect::StartRefollowTimer]; // root not back yet: ask again, no status
                    }
                    fx.extend(self.end_following("host-left")); // root didn't come back
                }
                if matches!(code.as_str(), "auth" | "version" | "replaced") {
                    fx.push(Effect::GiveUp);
                }
                fx.push(Effect::Event(SyncEvent::Failed(code)));
            }
            _ => {}
        }
        fx
    }

    /// The re-follow timer fired.
    pub fn refollow_tick(&mut self) -> Vec<Effect> {
        if !self.refollow {
            return vec![];
        }
        let user = self.following.clone();
        self.follow(&user)
    }

    /// The 30 s clock timer fired: measure again from scratch.
    pub fn clock_tick(&mut self, local_now: i64) -> Vec<Effect> {
        self.samples.clear();
        vec![Self::measure(local_now)]
    }

    /// Socket gone or never came up.
    pub fn disconnected(&mut self) -> Vec<Effect> {
        if !self.available {
            return vec![];
        }
        self.available = false;
        vec![Effect::Event(SyncEvent::Changed)]
    }

    pub fn publish(&mut self, mut state: Map<String, Value>) -> Vec<Effect> {
        state.insert("allowControl".into(), self.allow.into());
        self.last_full = state.clone();
        let qrev = qrev_of(&state);
        if qrev == self.sent_qrev {
            state.remove("queue"); // followers already have these songs
        }
        self.sent_qrev = qrev;
        self.last_sent = state.clone();
        if self.available { vec![Effect::Send(In::State(state))] } else { vec![] }
    }

    pub fn follow(&mut self, user: &str) -> Vec<Effect> {
        if self.available { vec![Effect::Send(In::Follow { user: user.into() })] } else { vec![] }
    }

    /// Leaving never waits for the relay: offline or after a relay restart it may never answer.
    pub fn unfollow(&mut self) -> Vec<Effect> {
        let mut fx = if self.available { vec![Effect::Send(In::Unfollow)] } else { vec![] };
        fx.extend(self.end_following("unfollowed"));
        fx
    }

    fn end_following(&mut self, reason: &str) -> Vec<Effect> {
        self.refollow = false;
        let mut fx = vec![Effect::StopRefollowTimer];
        if self.following.is_empty() {
            return fx;
        }
        self.following.clear();
        self.last_state.clear();
        fx.push(Effect::Event(SyncEvent::Ended(reason.into())));
        fx.push(Effect::Event(SyncEvent::Changed));
        fx
    }

    /// A command to the root while following.
    pub fn send(&self, op: &str, args: Value) -> Vec<Effect> {
        if !self.available || self.following.is_empty() {
            return vec![];
        }
        vec![Effect::Send(In::Cmd { op: op.into(), args, qrev: qrev_of(&self.last_state) })]
    }

    pub fn set_allow_control(&mut self, on: bool) -> Vec<Effect> {
        if on == self.allow {
            return vec![];
        }
        self.allow = on;
        let mut fx = if self.last_full.is_empty() { vec![] } else { self.publish(self.last_full.clone()) };
        fx.push(Effect::Event(SyncEvent::Changed));
        fx
    }

    pub fn set_hidden(&mut self, on: bool) -> Vec<Effect> {
        if on == self.hidden {
            return vec![];
        }
        self.hidden = on;
        self.listeners_known = false; // hidden: the relay hides whom anyone follows; shown again: a fresh list, no toasts
        let mut fx = if self.available { vec![Effect::Send(In::Presence { hidden: on })] } else { vec![] };
        fx.push(Effect::Event(SyncEvent::Changed));
        fx
    }

    /// Stopped (logout, quit): back to alone, nobody listed.
    pub fn stop(&mut self) -> Vec<Effect> {
        self.available = false;
        self.following.clear();
        self.last_state.clear();
        self.users.clear();
        vec![Effect::StopRefollowTimer, Effect::Event(SyncEvent::UsersChanged), Effect::Event(SyncEvent::Changed)]
    }

    /// --demo: two fake users, for designing themes without a relay.
    pub fn demo(&mut self) -> Vec<Effect> {
        self.users = vec![
            json!({"user": "arkadas", "following": null, "title": "Teardrop", "artist": "Massive Attack", "playing": true}),
            json!({"user": "misafir", "following": "arkadas", "title": "Teardrop", "artist": "Massive Attack", "playing": true}),
        ];
        self.available = true;
        vec![Effect::Event(SyncEvent::UsersChanged), Effect::Event(SyncEvent::Changed)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Map, Value, json};

    fn state(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    fn count(fx: &[Effect], pred: impl Fn(&SyncEvent) -> bool) -> usize {
        fx.iter().filter(|e| matches!(e, Effect::Event(ev) if pred(ev))).count()
    }

    fn sent(fx: &[Effect]) -> Vec<&In> {
        fx.iter().filter_map(|e| if let Effect::Send(m) = e { Some(m) } else { None }).collect()
    }

    // not started: publish only records what would be sent
    #[test]
    fn publish_records_and_drops_queue_on_same_qrev() {
        let mut s = SyncMachine::new(false, false);
        assert!(sent(&s.publish(state(json!({"qrev": 3, "queue": []}))))
            .is_empty()); // not connected: nothing goes out
        assert!(s.last_sent().contains_key("queue"));
        assert!(s.last_sent().contains_key("allowControl"));

        s.publish(state(json!({"qrev": 3, "queue": []})));
        assert!(!s.last_sent().contains_key("queue")); // same qrev on the same connection: queue not resent
        s.reset_sent(); // what a new connection (welcome) does
        s.publish(state(json!({"qrev": 3, "queue": []})));
        assert!(s.last_sent().contains_key("queue")); // first state after a reconnect always carries the queue
        s.publish(state(json!({"qrev": 4, "queue": [], "station": "1"})));
        assert_eq!(s.last_sent()["station"], "1"); // the station id reaches followers with the state
        s.publish(state(json!({"qrev": 5, "queue": [], "shuffle": true, "repeat": 2})));
        assert_eq!(s.last_sent()["shuffle"], true);
        assert_eq!(s.last_sent()["repeat"], 2);
    }

    #[test]
    fn accept_command_drops_stale_index_ops() {
        let mut s = SyncMachine::new(false, false);
        s.publish(state(json!({"qrev": 3, "queue": []})));
        assert!(s.accept_command("next", 1)); // non-index op: always ok
        assert!(s.accept_command("remove", 3));
        assert!(!s.accept_command("remove", 2)); // stale index command dropped
        assert!(!s.accept_command("jump", 2));
        assert!(!s.accept_command("move", 2));
        assert!(s.accept_command("station", 999)); // not index ops: never dropped as stale
        assert!(s.accept_command("shuffle", 999));
        assert!(s.accept_command("repeat", 999));
    }

    #[test]
    fn fresh_machine_is_alone() {
        let s = SyncMachine::new(false, false);
        assert!(s.following().is_empty());
        assert!(s.can_control());
        assert!(!s.available());
    }

    // Leaving must work even when the relay can't confirm it (offline, or relay restarted).
    #[test]
    fn unfollow_offline_still_ends() {
        let mut s = SyncMachine::new(false, false);
        s.on_text(r#"{"type":"following","root":"Ali"}"#, 0);
        assert_eq!(s.following(), "Ali");
        let fx = s.unfollow(); // not connected: nobody will answer
        assert!(s.following().is_empty());
        assert_eq!(count(&fx, |e| matches!(e, SyncEvent::Ended(_))), 1);
        let fx = s.on_text(r#"{"type":"ended","reason":"unfollowed"}"#, 0); // late confirmation: nothing left to end
        assert_eq!(count(&fx, |e| matches!(e, SyncEvent::Ended(_))), 0);
    }

    // Clicking "Dinle" on someone who just left: the current follow stays.
    #[test]
    fn not_found_keeps_current_follow() {
        let mut s = SyncMachine::new(false, false);
        s.on_text(r#"{"type":"following","root":"Ali"}"#, 0);
        s.on_text(r#"{"type":"error","code":"not-found"}"#, 0);
        assert_eq!(s.following(), "Ali");
    }

    // Re-follow after a reconnect: after a server reboot the root may reconnect after us, so keep trying
    // (10 tries, 2 s apart) before giving up; then back to playing on our own, not stuck.
    #[test]
    fn refollow_gives_up_after_ten() {
        let mut s = SyncMachine::new(false, false);
        s.on_text(r#"{"type":"following","root":"Ali"}"#, 0);
        let fx = s.on_text(r#"{"type":"welcome","you":"me","now":0}"#, 0);
        assert!(sent(&fx).contains(&&In::Follow { user: "Ali".into() }));
        for _ in 1..10 {
            let fx = s.on_text(r#"{"type":"error","code":"not-found"}"#, 0);
            assert_eq!(s.following(), "Ali");
            assert_eq!(count(&fx, |e| matches!(e, SyncEvent::Failed(_))), 0); // no status while still waiting
            assert!(fx.contains(&Effect::StartRefollowTimer));
            assert!(sent(&s.refollow_tick()).contains(&&In::Follow { user: "Ali".into() }));
        }
        let fx = s.on_text(r#"{"type":"error","code":"not-found"}"#, 0);
        assert!(s.following().is_empty());
        assert_eq!(count(&fx, |e| *e == SyncEvent::Ended("host-left".into())), 1);
    }

    // Who listens to me: from the users list (following == me). The first list after welcome only sets it (no toasts).
    #[test]
    fn listeners_toasts() {
        let joined = |fx: &[Effect]| -> Vec<String> {
            fx.iter().filter_map(|e| if let Effect::Event(SyncEvent::ListenerJoined(n)) = e { Some(n.clone()) } else { None }).collect()
        };
        let left = |fx: &[Effect]| -> Vec<String> {
            fx.iter().filter_map(|e| if let Effect::Event(SyncEvent::ListenerLeft(n)) = e { Some(n.clone()) } else { None }).collect()
        };
        let mut l = SyncMachine::new(false, false);
        l.on_text(r#"{"type":"welcome","you":"Me","now":0}"#, 0);
        let fx = l.on_text(r#"{"type":"users","users":[{"user":"A","following":"Me","playing":false}]}"#, 0);
        assert_eq!(l.listeners(), ["A"]);
        assert!(joined(&fx).is_empty());
        let fx = l.on_text(
            r#"{"type":"users","users":[{"user":"A","following":"Me","playing":false},{"user":"B","following":"me","playing":false},{"user":"C","following":"X","playing":false}]}"#,
            0,
        );
        assert_eq!(joined(&fx), ["B"]);
        assert!(left(&fx).is_empty());
        let fx = l.on_text(r#"{"type":"users","users":[{"user":"B","following":"me","playing":false}]}"#, 0);
        assert_eq!(left(&fx), ["A"]);
        assert_eq!(l.listeners(), ["B"]);
        l.on_text(r#"{"type":"welcome","you":"Me","now":0}"#, 0); // reconnect: the next list is a first list again
        let fx = l.on_text(r#"{"type":"users","users":[]}"#, 0);
        assert!(left(&fx).is_empty());
        assert!(l.listeners().is_empty());
        // Hiding: the relay stops showing whom anyone follows (following: null), which is not everyone leaving
        l.on_text(r#"{"type":"users","users":[{"user":"A","following":"Me","playing":false}]}"#, 0);
        l.set_hidden(true);
        let fx = l.on_text(r#"{"type":"users","users":[{"user":"A","following":null,"playing":false}]}"#, 0);
        assert!(left(&fx).is_empty());
        assert_eq!(l.listeners(), ["A"]); // last known, kept while hidden
        l.set_hidden(false);
        let fx = l.on_text(r#"{"type":"users","users":[{"user":"A","following":"Me","playing":false}]}"#, 0);
        assert!(joined(&fx).is_empty() && left(&fx).is_empty());
        assert_eq!(l.listeners(), ["A"]);
    }

    #[test]
    fn auth_error_gives_up() {
        let mut m = SyncMachine::new(false, false);
        for code in ["auth", "version", "replaced"] {
            let fx = m.on_text(&format!(r#"{{"type":"error","code":"{code}"}}"#), 0);
            assert!(fx.contains(&Effect::GiveUp));
            assert!(fx.contains(&Effect::Event(SyncEvent::Failed(code.into()))));
        }
        assert!(!m.on_text(r#"{"type":"error","code":"denied"}"#, 0).contains(&Effect::GiveUp));
    }

    #[test]
    fn now_without_samples_is_local() {
        let m = SyncMachine::new(false, false);
        assert_eq!(m.offset(), 0);
    }

    // What the client does with the relay's clock replies: five samples, min round trip wins.
    #[test]
    fn welcome_measures_the_clock() {
        let mut m = SyncMachine::new(false, false);
        let fx = m.on_text(r#"{"type":"welcome","you":"me","now":0}"#, 1000);
        assert!(sent(&fx).contains(&&In::Time { c: 1000.0 }));
        assert!(fx.contains(&Effect::StartClockTimer));
        assert!(m.available());
        for i in 0..4 {
            let fx = m.on_text(r#"{"type":"time","c":1000,"s":6000}"#, 1010);
            assert_eq!(sent(&fx).len(), 1, "sample {i} asks for the next");
        }
        let fx = m.on_text(r#"{"type":"time","c":1000,"s":6000}"#, 1010);
        assert!(sent(&fx).is_empty()); // five samples: done until the next clock tick
        assert_eq!(m.offset(), 6000 - 1005);
    }

    #[test]
    fn command_while_following_goes_to_the_root() {
        let mut m = SyncMachine::new(false, false);
        m.on_text(r#"{"type":"welcome","you":"me","now":0}"#, 0);
        m.on_text(r#"{"type":"following","root":"Ali"}"#, 0);
        m.on_text(r#"{"type":"state","qrev":7,"allowControl":true}"#, 0);
        assert!(m.can_control());
        let fx = m.send("next", json!({}));
        assert_eq!(sent(&fx), [&In::Cmd { op: "next".into(), args: json!({}), qrev: 7 }]);
    }

    #[test]
    fn hidden_and_allow_control_go_out_when_connected() {
        let mut m = SyncMachine::new(false, false);
        assert_eq!(m.hello("u", "t", "s"), In::Hello { v: 1, u: "u".into(), t: "t".into(), s: "s".into(), hidden: false });
        m.on_text(r#"{"type":"welcome","you":"me","now":0}"#, 0);
        assert_eq!(sent(&m.set_hidden(true)), [&In::Presence { hidden: true }]);
        m.publish(state(json!({"qrev": 1, "queue": []})));
        let fx = m.set_allow_control(true); // republishes the last state with the new flag; same qrev: no queue
        assert!(matches!(sent(&fx)[..], [In::State(s)] if s["allowControl"] == true && !s.contains_key("queue")));
    }
}
