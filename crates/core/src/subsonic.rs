//! The Navidrome (Subsonic API) client: token login, calls, and the guards for what QML may call.
use crate::queue::{NO_SERVER, NOT_SUBSONIC, encode_query, failure_text, for_themes, login_error, normalize_server, readable, subsonic_query, writable};
use md5::{Digest, Md5};
use serde_json::{Map, Value, json};
use std::time::Duration;

/// A login: the password itself is never kept, only md5(password + salt) with a fresh random salt (token auth).
#[derive(Debug, Clone, PartialEq)]
pub struct Login {
    pub server: String,
    pub user: String,
    pub salt: String,
    pub token: String,
}

impl Login {
    /// Err: the status text for an unusable address.
    pub fn from_password(server: &str, user: &str, password: &str) -> Result<Login, String> {
        let server = normalize_server(server);
        if server.is_empty() {
            return Err(crate::i18n::tr("Sunucu adresi geçersiz.", "Invalid server address.").into());
        }
        let salt = format!("{:x}", fastrand::u64(..));
        let token = Md5::digest(format!("{password}{salt}").as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
        Ok(Login { server, user: user.trim().into(), salt, token })
    }
}

/// Why a call failed. `code`: the Subsonic error code, else NO_SERVER (no answer, HTTP 5xx) or NOT_SUBSONIC.
#[derive(Debug, Clone, PartialEq)]
pub struct Failure {
    pub code: i64,
    pub message: String,
    pub http: u16,
    pub net_error: String,
}

impl Failure {
    /// "<method>: <why>" for the status line; never the token or salt (pass them in `secrets`).
    pub fn status_text(&self, method: &str, secrets: &[&str]) -> String {
        format!("{method}: {}", failure_text(&self.message, self.http, &self.net_error, secrets))
    }
    /// What the login form says.
    pub fn login_text(&self) -> String {
        login_error(self.code, &self.message)
    }
}

/// A call's URL for this login (also without a token: the cover cache keys on the URL without the login); None when
/// the server address is unusable.
pub fn subsonic_url(l: &Login, method: &str, params: &[(String, String)]) -> Option<url::Url> {
    let mut q = params.to_vec();
    for (k, v) in [("u", &l.user), ("t", &l.token), ("s", &l.salt)] {
        q.push((k.into(), v.clone()));
    }
    for (k, v) in [("v", "1.16.1"), ("c", "re-la"), ("f", "json")] {
        q.push((k.into(), v.into()));
    }
    url::Url::parse(&format!("{}/rest/{method}?{}", l.server, encode_query(&q))).ok()
}

/// The transcodes a phone plays and a bitrate makes smaller.
pub const TRANSCODES: [&str; 4] = ["opus", "mp3", "aac", "ogg"];

pub struct Subsonic {
    login: Login,
    http: reqwest::Client,
}

impl Subsonic {
    pub fn new(login: Login) -> Subsonic {
        let http = crate::http_client().timeout(Duration::from_secs(30)).build().expect("http client");
        Subsonic { login, http }
    }

    pub fn login(&self) -> &Login {
        &self.login
    }

    /// <server>/rest/<method>?<params>&u=&t=&s=&v=1.16.1&c=re-la&f=json
    pub fn url(&self, method: &str, params: &[(String, String)]) -> url::Url {
        subsonic_url(&self.login, method, params).expect("server checked at login")
    }

    /// The HTTP client the calls go through (30 s per call).
    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// The `subsonic-response` object when its status is "ok".
    pub async fn call(&self, method: &str, params: &[(String, String)]) -> Result<Value, Failure> {
        self.answer(self.http.get(self.url(method, params))).await
    }

    /// The formats this server transcodes to. The Subsonic API has no list, so each is asked for (OpenSubsonic
    /// getTranscodeDecision, for any one song); none on a server without it. Navidrome also says yes to its built-in
    /// formats that have no profile: the relay knows better (relay::transcodings).
    pub async fn transcode_formats(&self) -> Vec<String> {
        let one = self.call("getRandomSongs", &[("size".into(), "1".into())]).await.unwrap_or_default();
        let Some(id) = one["randomSongs"]["song"][0]["id"].as_str() else { return vec![] };
        let url = self.url("getTranscodeDecision", &[("mediaId".into(), id.into()), ("mediaType".into(), "song".into())]);
        let mut out = vec![];
        for f in TRANSCODES {
            let body = json!({"name": "re-la", "transcodingProfiles": [{"container": f, "audioCodec": f, "protocol": "http"}]});
            match self.answer(self.http.post(url.clone()).json(&body)).await {
                Ok(r) if r["transcodeDecision"]["canTranscode"] == true => out.push(f.into()),
                Ok(_) => {}
                Err(_) => break, // no such call here
            }
        }
        out
    }

    async fn answer(&self, req: reqwest::RequestBuilder) -> Result<Value, Failure> {
        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => return Err(Failure { code: NO_SERVER, message: String::new(), http: 0, net_error: e.to_string() }),
        };
        let http = resp.status().as_u16();
        let reason = resp.status().canonical_reason().unwrap_or("").to_string();
        let body = resp.bytes().await.unwrap_or_default();
        let r = serde_json::from_slice::<Value>(&body)
            .ok()
            .and_then(|v| v.get("subsonic-response").cloned())
            .filter(Value::is_object)
            .unwrap_or_else(|| Value::Object(Map::new()));
        if r["status"] == "ok" {
            return Ok(r);
        }
        let empty = r.as_object().is_none_or(Map::is_empty);
        let code = if !empty {
            r["error"]["code"].as_i64().unwrap_or(0)
        } else if http == 0 || http >= 500 {
            NO_SERVER
        } else {
            NOT_SUBSONIC
        };
        let message = r["error"]["message"].as_str().unwrap_or("").to_string();
        Err(Failure { code, message, http, net_error: if http >= 400 { reason } else { String::new() } })
    }

    /// Read-only call for QML pages (Player.get): only `readable` methods; the reply as themes may see it.
    pub async fn get(&self, method: &str, params: &Map<String, Value>) -> Option<Value> {
        if !readable(method) {
            return None;
        }
        self.call(method, &subsonic_query(params)).await.ok().map(|r| for_themes(method, r))
    }

    /// Writes for QML (Player.set): favorites and playlist edits only.
    pub async fn set(&self, method: &str, params: &Map<String, Value>) -> Option<Value> {
        if !writable(method) {
            return None;
        }
        self.call(method, &subsonic_query(params)).await.ok()
    }
}
