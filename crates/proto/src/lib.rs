//! Relay protocol v1 (docs/development.md): what clients send (`In`) and what the relay sends back (`Out`).
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Protocol version clients send in `hello`.
pub const VERSION: u32 = 1;
/// Longest queue the relay accepts in a state: longer ones would break following.
pub const MAX_QUEUE: usize = 1000;

/// Client → relay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum In {
    Hello {
        v: u32,
        u: String,
        t: String,
        s: String,
        #[serde(default)]
        hidden: bool,
    },
    Time {
        c: f64,
    },
    /// Kept as a raw object: the relay stores and forwards it without interpreting most fields.
    State(Map<String, Value>),
    Follow {
        user: String,
    },
    Unfollow,
    Cmd {
        op: String,
        #[serde(default)]
        args: Value,
        #[serde(default)]
        qrev: i64,
    },
    Presence {
        hidden: bool,
    },
    Bye,
}

/// Relay → client. The relay builds these with `json!`; its e2e tests check every frame parses as one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Out {
    Welcome {
        you: String,
        now: f64,
    },
    Time {
        c: f64,
        s: f64,
    },
    Users {
        users: Vec<User>,
    },
    State(Map<String, Value>),
    Following {
        root: String,
    },
    Cmd {
        from: String,
        op: String,
        #[serde(default)]
        args: Value,
        #[serde(default)]
        qrev: i64,
    },
    Ended {
        reason: String,
    },
    Error {
        code: String,
        #[serde(default)]
        message: String,
    },
}

/// One visible online user in `users`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub user: String,
    pub following: Option<String>,
    pub station: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub playing: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn in_round_trips_with_type_tag() {
        let v = serde_json::to_value(In::Follow { user: "ali".into() }).unwrap();
        assert_eq!(v, json!({"type": "follow", "user": "ali"}));
        let s: In = serde_json::from_value(json!({"type": "state", "qrev": 3, "playing": true})).unwrap();
        assert!(matches!(s, In::State(m) if m["qrev"] == 3 && !m.contains_key("type")));
    }

    #[test]
    fn out_parses_relay_messages() {
        let u: Out = serde_json::from_value(json!({"type": "users", "users": [
            {"user": "c", "following": null, "station": null, "title": null, "artist": null, "playing": false}]}))
        .unwrap();
        assert_eq!(
            u,
            Out::Users {
                users: vec![User { user: "c".into(), following: None, station: None, title: None, artist: None, playing: false }]
            }
        );
        let e: Out = serde_json::from_value(json!({"type": "error", "code": "auth", "message": "auth"})).unwrap();
        assert_eq!(e, Out::Error { code: "auth".into(), message: "auth".into() });
        let w: Out = serde_json::from_value(json!({"type": "welcome", "you": "ali", "now": 1795188866751_i64})).unwrap();
        assert!(matches!(w, Out::Welcome { now, .. } if now == 1795188866751.0));
    }
}
