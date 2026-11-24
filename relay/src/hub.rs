//! Pure relay state: who is online, who follows whom, the last state of each root.
//! No I/O and no clock: every call gets `now` (unix ms) and returns the messages to send.
use rela_proto::In;
use serde_json::{Map, Value, json};
use std::collections::HashMap;

pub type ConnId = u64;

#[derive(Debug)]
pub enum Out {
    To(ConnId, Value),
    Close(ConnId),
}

pub const GRACE_MS: u64 = 60_000;
pub use rela_proto::MAX_QUEUE;

struct User {
    name: String,
    conn: Option<ConnId>,
    hidden: bool,
    following: Option<String>, // key of a root; roots never follow, so this is never a chain
    state: Map<String, Value>, // last state without "queue"
    queue: Value,              // last full queue
    grace_until: Option<u64>,
}

#[derive(Default)]
pub struct Hub {
    users: HashMap<String, User>,
    conns: HashMap<ConnId, String>,
    last_users: Value,
}

/// Longest station name and track field shown in the online list, in characters.
const MAX_STATION: usize = 100;
const MAX_TRACK: usize = 200;

/// A string cut to `max` characters; null for anything that is not a string.
fn bounded(v: Option<&Value>, max: usize) -> Value {
    match v.and_then(Value::as_str) {
        Some(s) => Value::String(s.chars().take(max).collect()),
        None => Value::Null,
    }
}

fn msg(kind: &str, mut body: Value) -> Value {
    body["type"] = kind.into();
    body
}
fn error(code: &str) -> Value {
    msg("error", json!({"code": code, "message": code}))
}

impl Hub {
    pub fn new() -> Self {
        Self::default()
    }

    /// (users online, listening groups): counts only, for `re-with status`.
    pub fn summary(&self) -> (usize, usize) {
        let online = self.users.values().filter(|u| u.conn.is_some()).count();
        let roots: std::collections::HashSet<&String> = self.users.values().filter_map(|u| u.following.as_ref()).collect();
        (online, roots.len())
    }

    /// Called once auth succeeded. A second login of the same user takes over the entry.
    pub fn join(&mut self, conn: ConnId, user: &str, hidden: bool, now: u64) -> Vec<Out> {
        let key = user.to_lowercase();
        let mut out = Vec::new();
        match self.users.get_mut(&key) {
            Some(u) => {
                if let Some(old) = u.conn.replace(conn) {
                    out.push(Out::To(old, error("replaced")));
                    out.push(Out::Close(old));
                    self.conns.remove(&old);
                }
                u.name = user.to_string();
                u.hidden = hidden;
                u.grace_until = None;
            }
            None => {
                let u = User {
                    name: user.to_string(),
                    conn: Some(conn),
                    hidden,
                    following: None,
                    state: Map::new(),
                    queue: Value::Array(vec![]),
                    grace_until: None,
                };
                self.users.insert(key.clone(), u);
            }
        }
        self.conns.insert(conn, key.clone());
        out.push(Out::To(conn, msg("welcome", json!({"you": user, "now": now}))));
        if !self.broadcast_users(&mut out) {
            // list unchanged (hidden joiner, same-user reconnect): the new connection still needs it
            out.push(Out::To(conn, self.users_for(&key)));
        }
        out
    }

    pub fn handle(&mut self, conn: ConnId, m: In, now: u64) -> Vec<Out> {
        let Some(key) = self.conns.get(&conn).cloned() else { return vec![] };
        let mut out = Vec::new();
        match m {
            In::Hello { .. } => {}
            In::Time { c } => out.push(Out::To(conn, msg("time", json!({"c": c, "s": now})))),
            In::State(s) => self.on_state(&key, conn, s, &mut out),
            In::Follow { user } => self.on_follow(&key, conn, &user.to_lowercase(), &mut out),
            In::Unfollow => {
                if self.users.get_mut(&key).and_then(|u| u.following.take()).is_some() {
                    out.push(Out::To(conn, msg("ended", json!({"reason": "unfollowed"}))));
                }
            }
            In::Cmd { op, args, qrev } => self.on_cmd(&key, conn, op, args, qrev, &mut out),
            In::Presence { hidden } => {
                if let Some(u) = self.users.get_mut(&key) {
                    u.hidden = hidden;
                }
            }
            In::Bye => self.end_followers(&key, &mut out),
        }
        self.broadcast_users(&mut out);
        out
    }

    pub fn disconnect(&mut self, conn: ConnId, now: u64) -> Vec<Out> {
        let Some(key) = self.conns.remove(&conn) else { return vec![] };
        let mut out = Vec::new();
        if self.followers(&key).is_empty() {
            self.users.remove(&key);
        } else if let Some(u) = self.users.get_mut(&key) {
            u.conn = None;
            u.grace_until = Some(now + GRACE_MS);
        }
        self.broadcast_users(&mut out);
        out
    }

    /// Ends sessions of roots that stayed away longer than the grace period.
    pub fn tick(&mut self, now: u64) -> Vec<Out> {
        let expired: Vec<String> =
            self.users.iter().filter(|(_, u)| u.grace_until.is_some_and(|g| now > g)).map(|(k, _)| k.clone()).collect();
        let mut out = Vec::new();
        for key in expired {
            self.end_followers(&key, &mut out);
            self.users.remove(&key);
        }
        self.broadcast_users(&mut out);
        out
    }

    fn followers(&self, root: &str) -> Vec<String> {
        self.users.iter().filter(|(_, u)| u.following.as_deref() == Some(root)).map(|(k, _)| k.clone()).collect()
    }

    fn conn_of(&self, key: &str) -> Option<ConnId> {
        self.users.get(key).and_then(|u| u.conn)
    }

    /// Last state of a root with its full queue, as a follower should receive it.
    fn full_state(&self, root: &str) -> Option<Value> {
        let u = self.users.get(root)?;
        if u.state.is_empty() {
            return None;
        }
        let mut v = Value::Object(u.state.clone());
        v["queue"] = u.queue.clone();
        Some(msg("state", v))
    }

    fn on_state(&mut self, key: &str, conn: ConnId, mut s: Map<String, Value>, out: &mut Vec<Out>) {
        let Some(u) = self.users.get_mut(key) else { return };
        if u.following.is_some() {
            return; // followers play the root's state; theirs is local
        }
        let queue = s.remove("queue");
        if queue.as_ref().and_then(Value::as_array).is_some_and(|q| q.len() > MAX_QUEUE) {
            out.push(Out::To(conn, error("too-large")));
            return;
        }
        let mut forward = Value::Object(s.clone());
        if let Some(q) = queue {
            forward["queue"] = q.clone();
            u.queue = q;
        }
        u.state = s;
        let forward = msg("state", forward);
        for f in self.followers(key) {
            if let Some(c) = self.conn_of(&f) {
                out.push(Out::To(c, forward.clone()));
            }
        }
    }

    fn on_follow(&mut self, key: &str, conn: ConnId, target: &str, out: &mut Vec<Out>) {
        let visible = |k: &str| self.users.get(k).is_some_and(|u| !u.hidden);
        let online = self.conn_of(target).is_some();
        let root = self.users.get(target).and_then(|u| u.following.clone()).unwrap_or_else(|| target.to_string());
        if !online || !visible(target) || !visible(&root) {
            return out.push(Out::To(conn, error("not-found")));
        }
        if root == key {
            return out.push(Out::To(conn, error("self")));
        }
        let root_name = self.users[&root].name.clone();
        let state = self.full_state(&root);
        let mut movers = self.followers(key);
        movers.push(key.to_string());
        for m in movers {
            if let Some(u) = self.users.get_mut(&m) {
                u.following = Some(root.clone());
            }
            if let Some(c) = self.conn_of(&m) {
                out.push(Out::To(c, msg("following", json!({"root": root_name}))));
                if let Some(s) = &state {
                    out.push(Out::To(c, s.clone()));
                }
            }
        }
    }

    fn on_cmd(&mut self, key: &str, conn: ConnId, op: String, args: Value, qrev: i64, out: &mut Vec<Out>) {
        let Some(root) = self.users.get(key).and_then(|u| u.following.clone()) else { return };
        let Some(r) = self.users.get(&root) else { return };
        let Some(rc) = r.conn else { return }; // root in grace: nobody to apply it
        if r.state.get("allowControl") != Some(&Value::Bool(true)) {
            return out.push(Out::To(conn, error("denied")));
        }
        let from = self.users[key].name.clone();
        out.push(Out::To(rc, msg("cmd", json!({"from": from, "op": op, "args": args, "qrev": qrev}))));
    }

    fn end_followers(&mut self, root: &str, out: &mut Vec<Out>) {
        for f in self.followers(root) {
            if let Some(u) = self.users.get_mut(&f) {
                u.following = None;
                if let Some(c) = u.conn {
                    out.push(Out::To(c, msg("ended", json!({"reason": "host-left"}))));
                }
            }
        }
    }

    /// Online, visible users. A follower of a hidden root shows as idle, so "Gizli" hides what the root plays.
    fn users_list(&self) -> Value {
        let mut keys: Vec<&String> = self.users.iter().filter(|(_, u)| u.conn.is_some() && !u.hidden).map(|(k, _)| k).collect();
        keys.sort();
        let list = keys.iter().map(|k| {
            let u = &self.users[*k];
            let root = u.following.as_ref().and_then(|r| self.users.get(r)).filter(|r| !r.hidden);
            let src = root.unwrap_or(u);
            let song = src.state.get("index").and_then(Value::as_u64).and_then(|i| src.queue.get(i as usize));
            // On a radio station the queue only waits, so the label comes from `radio`. Clients send both and the
            // relay must not trust them: strings only, bounded, an empty station name is no label.
            let on_station = src.state.get("station").and_then(Value::as_str).is_some_and(|s| !s.is_empty());
            let radio = src.state.get("radio").filter(|_| on_station);
            let station = bounded(radio.and_then(|r| r.get("name")), MAX_STATION);
            let station = if station.as_str() == Some("") { Value::Null } else { station };
            let track = if station.is_null() { song } else { radio };
            let field = |k| bounded(track.and_then(|t| t.get(k)), MAX_TRACK);
            json!({
                "user": u.name,
                "following": root.map(|r| r.name.clone()),
                "station": station,
                "title": field("title"),
                "artist": field("artist"),
                "playing": src.state.get("playing").and_then(Value::as_bool).unwrap_or(false),
            })
        });
        Value::Array(list.collect())
    }

    /// The `users` message for one user: the list without itself.
    fn users_for(&self, key: &str) -> Value {
        let list = if self.last_users.is_array() { self.last_users.clone() } else { self.users_list() };
        let mine: Vec<Value> =
            list.as_array().unwrap().iter().filter(|e| e["user"].as_str().map(str::to_lowercase).as_deref() != Some(key)).cloned().collect();
        msg("users", json!({"users": mine}))
    }

    /// Sends the online list to everyone (each without itself), only when it changed. True if sent.
    fn broadcast_users(&mut self, out: &mut Vec<Out>) -> bool {
        let list = self.users_list();
        if list == self.last_users {
            return false;
        }
        self.last_users = list;
        for (key, u) in &self.users {
            if let Some(c) = u.conn {
                out.push(Out::To(c, self.users_for(key)));
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rela_proto::In;
    use serde_json::{Value, json};

    fn msgs(out: &[Out], conn: ConnId) -> Vec<Value> {
        out.iter()
            .filter_map(|o| match o {
                Out::To(c, v) if *c == conn => Some(v.clone()),
                _ => None,
            })
            .collect()
    }
    fn types(out: &[Out], conn: ConnId) -> Vec<String> {
        msgs(out, conn).iter().map(|v| v["type"].as_str().unwrap_or("").to_string()).collect()
    }
    fn of_type(out: &[Out], conn: ConnId, t: &str) -> Option<Value> {
        msgs(out, conn).into_iter().find(|v| v["type"] == t)
    }
    fn parse(v: Value) -> In {
        serde_json::from_value(v).unwrap()
    }
    fn state(qrev: i64, queue: Option<Value>, allow: bool) -> In {
        let mut v = json!({"type": "state", "qrev": qrev, "index": 0, "playing": true,
                           "anchor": 1000.0, "position": 0.0, "xfade": 0, "allowControl": allow});
        if let Some(q) = queue {
            v["queue"] = q;
        }
        parse(v)
    }
    fn follow(user: &str) -> In {
        parse(json!({"type": "follow", "user": user}))
    }
    fn cmd(op: &str) -> In {
        parse(json!({"type": "cmd", "op": op, "args": {}, "qrev": 1}))
    }

    #[test]
    fn join_sends_welcome() {
        let mut h = Hub::new();
        let out = h.join(1, "Ali", false, 5);
        let w = of_type(&out, 1, "welcome").unwrap();
        assert_eq!(w["you"], "Ali");
        assert_eq!(w["now"], 5);
    }

    #[test]
    fn follow_is_case_insensitive() {
        let mut h = Hub::new();
        h.join(1, "Listener", false, 0);
        h.join(2, "b", false, 0);
        let out = h.handle(2, follow("listener"), 0);
        assert_eq!(of_type(&out, 2, "following").unwrap()["root"], "Listener");
    }

    #[test]
    fn follow_chain_resolves_root() {
        let mut h = Hub::new();
        h.join(1, "A", false, 0);
        h.join(2, "B", false, 0);
        h.join(3, "C", false, 0);
        h.handle(2, follow("A"), 0);
        let out = h.handle(3, follow("B"), 0);
        assert_eq!(of_type(&out, 3, "following").unwrap()["root"], "A");
    }

    #[test]
    fn root_following_moves_its_followers() {
        let mut h = Hub::new();
        h.join(1, "A", false, 0);
        h.join(2, "B", false, 0);
        h.join(3, "C", false, 0);
        h.handle(2, follow("A"), 0);
        let out = h.handle(1, follow("C"), 0);
        assert_eq!(of_type(&out, 2, "following").unwrap()["root"], "C");
        assert_eq!(of_type(&out, 1, "following").unwrap()["root"], "C");
    }

    #[test]
    fn follow_self_via_chain_is_error() {
        let mut h = Hub::new();
        h.join(1, "A", false, 0);
        h.join(2, "B", false, 0);
        h.handle(2, follow("A"), 0);
        let out = h.handle(1, follow("B"), 0);
        assert_eq!(of_type(&out, 1, "error").unwrap()["code"], "self");
        assert!(of_type(&out, 1, "following").is_none());
    }

    #[test]
    fn follow_hidden_or_unknown_not_found() {
        let mut h = Hub::new();
        h.join(1, "a", true, 0);
        h.join(2, "b", false, 0);
        let out = h.handle(2, follow("a"), 0);
        assert_eq!(of_type(&out, 2, "error").unwrap()["code"], "not-found");
        let out = h.handle(2, follow("zzz"), 0);
        assert_eq!(of_type(&out, 2, "error").unwrap()["code"], "not-found");
    }

    #[test]
    fn late_follower_gets_state_with_queue() {
        let mut h = Hub::new();
        h.join(1, "A", false, 0);
        h.join(2, "B", false, 0);
        h.handle(1, state(1, Some(json!([{"id": "x"}])), false), 0);
        h.handle(1, state(1, None, false), 0);
        let out = h.handle(2, follow("A"), 0);
        let s = of_type(&out, 2, "state").unwrap();
        assert_eq!(s["queue"][0]["id"], "x");
    }

    #[test]
    fn follower_state_is_ignored() {
        let mut h = Hub::new();
        h.join(1, "A", false, 0);
        h.join(2, "B", false, 0);
        h.handle(2, follow("A"), 0);
        let out = h.handle(2, state(1, Some(json!([])), false), 0);
        assert!(of_type(&out, 1, "state").is_none());
    }

    #[test]
    fn too_long_queue_rejected() {
        let mut h = Hub::new();
        h.join(1, "A", false, 0);
        h.join(2, "B", false, 0);
        h.handle(2, follow("A"), 0);
        let q: Vec<Value> = (0..1001).map(|i| json!({"id": i.to_string()})).collect();
        let out = h.handle(1, state(1, Some(Value::Array(q)), false), 0);
        assert_eq!(of_type(&out, 1, "error").unwrap()["code"], "too-large");
        assert!(of_type(&out, 2, "state").is_none());
    }

    #[test]
    fn cmd_denied_when_control_off() {
        let mut h = Hub::new();
        h.join(1, "A", false, 0);
        h.join(2, "B", false, 0);
        h.handle(1, state(1, Some(json!([])), false), 0);
        h.handle(2, follow("A"), 0);
        let out = h.handle(2, cmd("next"), 0);
        assert_eq!(of_type(&out, 2, "error").unwrap()["code"], "denied");
        assert!(of_type(&out, 1, "cmd").is_none());
    }

    #[test]
    fn cmd_forwarded_when_control_on() {
        let mut h = Hub::new();
        h.join(1, "A", false, 0);
        h.join(2, "B", false, 0);
        h.handle(1, state(1, Some(json!([])), true), 0);
        h.handle(2, follow("A"), 0);
        let out = h.handle(2, cmd("next"), 0);
        let c = of_type(&out, 1, "cmd").unwrap();
        assert_eq!(c["from"], "B");
        assert_eq!(c["op"], "next");
        assert_eq!(c["qrev"], 1);
    }

    #[test]
    fn bye_ends_followers_now() {
        let mut h = Hub::new();
        h.join(1, "A", false, 0);
        h.join(2, "B", false, 0);
        h.handle(2, follow("A"), 0);
        let out = h.handle(1, In::Bye, 0);
        assert_eq!(of_type(&out, 2, "ended").unwrap()["reason"], "host-left");
    }

    #[test]
    fn unfollow_ends_with_reason() {
        let mut h = Hub::new();
        h.join(1, "A", false, 0);
        h.join(2, "B", false, 0);
        h.handle(2, follow("A"), 0);
        let out = h.handle(2, In::Unfollow, 0);
        assert_eq!(of_type(&out, 2, "ended").unwrap()["reason"], "unfollowed");
        let out = h.handle(1, state(1, Some(json!([])), false), 0);
        assert!(of_type(&out, 2, "state").is_none());
    }

    #[test]
    fn root_grace_resume() {
        let mut h = Hub::new();
        h.join(1, "A", false, 0);
        h.join(2, "B", false, 0);
        h.handle(2, follow("A"), 0);
        let t0 = 1_000;
        let out = h.disconnect(1, t0);
        assert!(of_type(&out, 2, "ended").is_none());
        h.join(3, "A", false, t0 + 59_000);
        let out = h.tick(t0 + 61_000);
        assert!(of_type(&out, 2, "ended").is_none());
        let out = h.handle(3, state(2, Some(json!([])), false), t0 + 61_000);
        assert!(of_type(&out, 2, "state").is_some());
    }

    #[test]
    fn root_grace_expires() {
        let mut h = Hub::new();
        h.join(1, "A", false, 0);
        h.join(2, "B", false, 0);
        h.handle(2, follow("A"), 0);
        let t0 = 1_000;
        h.disconnect(1, t0);
        assert!(of_type(&h.tick(t0 + 60_000), 2, "ended").is_none());
        let out = h.tick(t0 + 60_001);
        assert_eq!(of_type(&out, 2, "ended").unwrap()["reason"], "host-left");
    }

    #[test]
    fn replaced_connection() {
        let mut h = Hub::new();
        h.join(1, "a", false, 0);
        h.join(2, "b", false, 0);
        h.handle(2, follow("a"), 0);
        let out = h.join(3, "A", false, 0);
        assert_eq!(of_type(&out, 1, "error").unwrap()["code"], "replaced");
        assert!(out.iter().any(|o| matches!(o, Out::Close(1))));
        let out = h.handle(3, state(1, Some(json!([])), false), 0);
        assert!(of_type(&out, 2, "state").is_some());
    }

    #[test]
    fn users_list_hides_hidden_and_self() {
        let mut h = Hub::new();
        h.join(1, "a", false, 0);
        h.join(2, "b", true, 0);
        let out = h.join(3, "c", false, 0);
        let users = of_type(&out, 1, "users").unwrap();
        assert_eq!(users["users"], json!([{"user": "c", "following": null, "station": null, "title": null, "artist": null, "playing": false}]));
        let users3 = of_type(&out, 3, "users").unwrap();
        assert_eq!(users3["users"][0]["user"], "a");
        assert_eq!(users3["users"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn follower_shows_roots_song() {
        let mut h = Hub::new();
        h.join(1, "a", false, 0);
        h.join(2, "b", false, 0);
        h.join(3, "c", false, 0);
        h.handle(1, state(1, Some(json!([{"id": "x", "title": "T", "artist": "R"}])), false), 0);
        let out = h.handle(2, follow("a"), 0);
        let list = of_type(&out, 3, "users").unwrap()["users"].clone();
        let b = list.as_array().unwrap().iter().find(|u| u["user"] == "b").unwrap().clone();
        assert_eq!(b["following"], "a");
        assert_eq!(b["title"], "T");
        assert_eq!(b["playing"], true);
    }

    #[test]
    fn presence_hides_from_list() {
        let mut h = Hub::new();
        h.join(1, "a", false, 0);
        h.join(2, "b", false, 0);
        let out = h.handle(2, parse(json!({"type": "presence", "hidden": true})), 0);
        assert_eq!(of_type(&out, 1, "users").unwrap()["users"], json!([]));
    }

    #[test]
    fn hidden_joiner_still_gets_the_list() {
        let mut h = Hub::new();
        h.join(1, "a", false, 0);
        let out = h.join(2, "b", true, 0);
        assert_eq!(of_type(&out, 2, "users").unwrap()["users"][0]["user"], "a");
    }

    #[test]
    fn replacing_connection_gets_the_list() {
        let mut h = Hub::new();
        h.join(1, "a", false, 0);
        h.join(2, "b", false, 0);
        let out = h.join(3, "b", false, 0); // same name: the list itself doesn't change
        assert_eq!(of_type(&out, 3, "users").unwrap()["users"][0]["user"], "a");
    }

    #[test]
    fn hidden_root_not_revealed_by_followers() {
        let mut h = Hub::new();
        h.join(1, "a", false, 0);
        h.join(2, "b", false, 0);
        h.join(3, "c", false, 0);
        h.handle(1, state(1, Some(json!([{"id": "x", "title": "T", "artist": "R"}])), false), 0);
        h.handle(2, follow("a"), 0);
        let out = h.handle(1, parse(json!({"type": "presence", "hidden": true})), 0);
        let list = of_type(&out, 3, "users").unwrap()["users"].clone();
        assert_eq!(list, json!([{"user": "b", "following": null, "station": null, "title": null, "artist": null, "playing": false}]));
    }

    fn state_radio(station: &str, radio: Option<Value>, queue: Option<Value>) -> In {
        let mut v = json!({"type": "state", "qrev": 1, "index": 0, "playing": true, "anchor": 1000.0,
                           "position": 0.0, "xfade": 0, "allowControl": false, "station": station});
        if let Some(r) = radio {
            v["radio"] = r;
        }
        if let Some(q) = queue {
            v["queue"] = q;
        }
        parse(v)
    }
    fn entry(out: &[Out], to: ConnId, user: &str) -> Value {
        let list = of_type(out, to, "users").unwrap()["users"].clone();
        list.as_array().unwrap().iter().find(|u| u["user"] == user).unwrap().clone()
    }

    #[test]
    fn users_list_shows_station_and_radio() {
        let mut h = Hub::new();
        h.join(1, "a", false, 0);
        h.join(3, "c", false, 0);
        let out = h.handle(1, state_radio("7", Some(json!({"name": "SUB/WAVE", "title": "T", "artist": "R"})), None), 0);
        let a = entry(&out, 3, "a");
        assert_eq!(a["station"], "SUB/WAVE");
        assert_eq!(a["title"], "T");
        assert_eq!(a["artist"], "R");
        assert_eq!(a["playing"], true);
    }

    #[test]
    fn radio_fields_are_bounded() {
        let mut h = Hub::new();
        h.join(1, "a", false, 0);
        h.join(3, "c", false, 0);
        let radio = json!({"name": "ş".repeat(300), "title": "x".repeat(500), "artist": "😀".repeat(250)});
        let a = entry(&h.handle(1, state_radio("7", Some(radio), None), 0), 3, "a");
        assert_eq!(a["station"].as_str().unwrap().chars().count(), 100);
        assert_eq!(a["title"].as_str().unwrap().chars().count(), 200);
        assert_eq!(a["artist"].as_str().unwrap().chars().count(), 200);
    }

    #[test]
    fn song_fields_are_bounded() {
        let mut h = Hub::new();
        h.join(1, "a", false, 0);
        h.join(3, "c", false, 0);
        let a = entry(&h.handle(1, state(1, Some(json!([{"title": "x".repeat(500), "artist": {"n": 1}}])), false), 0), 3, "a");
        assert_eq!(a["title"].as_str().unwrap().chars().count(), 200);
        assert_eq!(a["artist"], Value::Null); // clients read a string or null: anything else would break their whole list
    }

    #[test]
    fn radio_with_wrong_types() {
        let mut h = Hub::new();
        h.join(1, "a", false, 0);
        h.join(3, "c", false, 0);
        let queue = json!([{"id": "x", "title": "Q", "artist": "W"}]);
        let a = entry(&h.handle(1, state_radio("7", Some(json!({"name": 5, "title": "T"})), Some(queue)), 0), 3, "a");
        assert_eq!(a["station"], Value::Null); // no usable name: the queue song, as before
        assert_eq!(a["title"], "Q");
        let b = entry(&h.handle(1, state_radio("7", Some(json!({"name": "S", "title": {"x": 1}, "artist": ["y"]})), None), 0), 3, "a");
        assert_eq!(b["station"], "S");
        assert_eq!(b["title"], Value::Null);
        assert_eq!(b["artist"], Value::Null);
        let c = entry(&h.handle(1, state_radio("7", Some(json!({"name": ""})), None), 0), 3, "a");
        assert_eq!(c["station"], Value::Null); // an empty name is not a station label
    }

    #[test]
    fn station_without_radio_falls_back() {
        let mut h = Hub::new();
        h.join(1, "a", false, 0);
        h.join(3, "c", false, 0);
        let queue = json!([{"id": "x", "title": "T", "artist": "R"}]);
        let a = entry(&h.handle(1, state_radio("7", None, Some(queue)), 0), 3, "a");
        assert!(a.get("station").is_some_and(Value::is_null)); // the key is there, and null
        assert_eq!(a["title"], "T");
    }

    #[test]
    fn follower_of_radio_root_shows_root_station() {
        let mut h = Hub::new();
        h.join(1, "a", false, 0);
        h.join(2, "b", false, 0);
        h.join(3, "c", false, 0);
        h.handle(1, state_radio("7", Some(json!({"name": "SUB/WAVE", "title": "T", "artist": "R"})), None), 0);
        let out = h.handle(2, follow("a"), 0);
        let b = entry(&out, 3, "b");
        assert_eq!(b["following"], "a");
        assert_eq!(b["station"], "SUB/WAVE");
        assert_eq!(b["title"], "T");
    }

    #[test]
    fn hidden_radio_root_hides_station() {
        let mut h = Hub::new();
        h.join(1, "a", false, 0);
        h.join(2, "b", false, 0);
        h.join(3, "c", false, 0);
        h.handle(1, state_radio("7", Some(json!({"name": "SUB/WAVE", "title": "T", "artist": "R"})), None), 0);
        h.handle(2, follow("a"), 0);
        let out = h.handle(1, parse(json!({"type": "presence", "hidden": true})), 0);
        let b = entry(&out, 3, "b");
        assert!(b.get("station").is_some_and(Value::is_null));
        assert_eq!(b["title"], Value::Null);
    }

    #[test]
    fn time_echo() {
        let mut h = Hub::new();
        h.join(1, "a", false, 0);
        let out = h.handle(1, parse(json!({"type": "time", "c": 5.0})), 100);
        let t = of_type(&out, 1, "time").unwrap();
        assert_eq!(t["c"], 5.0);
        assert_eq!(t["s"], 100);
    }

    #[test]
    fn users_not_resent_when_unchanged() {
        let mut h = Hub::new();
        h.join(1, "a", false, 0);
        h.join(2, "b", false, 0);
        let out = h.handle(1, parse(json!({"type": "time", "c": 1.0})), 0);
        assert!(types(&out, 2).is_empty());
    }
}
