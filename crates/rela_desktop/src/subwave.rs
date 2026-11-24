//! `Subwave` (SUB/WAVE listener client) as the QObject QML and MPRIS know. The client is the player's (hub): the core opens and closes the station itself, so the
//! stream URL (it may hold a password) never leaves Rust.
use crate::hub::{SUBWAVE_QT, player};
use crate::qt::to_qvariant_map;
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QMap, QMapPair_QString_QVariant, QString};
use rela_core::subwave::{SubwaveClient, SubwaveEvent, SubwaveSnapshot, local_now};
use serde_json::Value;

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qmap.h");
        type QMap_QString_QVariant = cxx_qt_lib::QMap<cxx_qt_lib::QMapPair_QString_QVariant>;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(bool, active, READ, NOTIFY = changed)]
        #[qproperty(QMap_QString_QVariant, now, READ, NOTIFY = changed)]
        #[qproperty(QString, cover_url, cxx_name = "coverUrl", READ, NOTIFY = changed)]
        #[qproperty(bool, online, READ, NOTIFY = changed)]
        #[qproperty(QMap_QString_QVariant, peeked, READ, NOTIFY = changed)]
        #[qproperty(QMap_QString_QVariant, lyrics, READ, NOTIFY = changed)]
        #[qproperty(QString, lyrics_state, cxx_name = "lyricsState", READ, NOTIFY = changed)]
        #[qproperty(bool, like_enabled, cxx_name = "likeEnabled", READ, NOTIFY = changed)]
        #[qproperty(bool, liked, READ, NOTIFY = changed)]
        #[qproperty(i32, like_blocked_ms, cxx_name = "likeBlockedMs", READ, NOTIFY = changed)]
        #[qproperty(QMap_QString_QVariant, request, READ, NOTIFY = changed)]
        #[qproperty(i64, elapsed_ms, cxx_name = "elapsedMs", READ = elapsed_ms, NOTIFY = elapsed_changed)]
        #[cxx_name = "Subwave"]
        type RustSubwave = super::SubwaveRust;
    }

    unsafe extern "RustQt" {
        #[qsignal]
        fn changed(self: Pin<&mut RustSubwave>);
        #[qsignal]
        #[cxx_name = "elapsedChanged"]
        fn elapsed_changed(self: Pin<&mut RustSubwave>);

        /// ms since the track started, on the server's clock, as the listener hears it.
        #[cxx_name = "getElapsedMs"]
        fn elapsed_ms(self: &RustSubwave) -> i64;

        /// One /now-playing per home page and session, for the Radio page.
        #[qinvokable]
        fn probe(self: &RustSubwave, home_page: &QString);
        #[qinvokable]
        fn like(self: &RustSubwave);
        #[qinvokable]
        #[cxx_name = "sendRequest"]
        fn send_request(self: &RustSubwave, text: &QString, name: &QString);
        /// Cover of a track on the active station ("" when none is active), for MPRIS.
        #[qinvokable]
        #[cxx_name = "coverUrlFor"]
        fn cover_url_for(self: &RustSubwave, subsonic_id: &QString) -> QString;
    }

    impl cxx_qt::Threading for RustSubwave {}
    impl cxx_qt::Initialize for RustSubwave {}
}

#[derive(Default)]
pub struct SubwaveRust {
    active: bool,
    now: QMap<QMapPair_QString_QVariant>,
    cover_url: QString,
    online: bool,
    peeked: QMap<QMapPair_QString_QVariant>,
    lyrics: QMap<QMapPair_QString_QVariant>,
    lyrics_state: QString,
    like_enabled: bool,
    liked: bool,
    like_blocked_ms: i32,
    request: QMap<QMapPair_QString_QVariant>,
    last: Option<SubwaveSnapshot>,
}

/// A SUB/WAVE event from hub, already on the Qt thread.
pub(crate) fn deliver(mut o: Pin<&mut ffi::RustSubwave>, event: SubwaveEvent, snapshot: SubwaveSnapshot) {
    match event {
        SubwaveEvent::Tick => {
            o.as_mut().rust_mut().last = Some(snapshot);
            o.elapsed_changed();
        }
        SubwaveEvent::Changed => {
            o.as_mut().apply(snapshot);
            o.changed();
        }
    }
}

impl cxx_qt::Initialize for ffi::RustSubwave {
    fn initialize(mut self: Pin<&mut Self>) {
        *SUBWAVE_QT.lock().unwrap() = Some(self.qt_thread());
        self.as_mut().rust_mut().online = true;
    }
}

impl ffi::RustSubwave {
    fn apply(mut self: Pin<&mut Self>, s: SubwaveSnapshot) {
        let mut r = self.as_mut().rust_mut();
        r.active = s.active;
        r.now = to_qvariant_map(&s.now);
        r.cover_url = QString::from(s.cover_url.as_str());
        r.online = s.online;
        r.peeked = to_qvariant_map(&Value::Object(s.peeked.clone()));
        r.lyrics = to_qvariant_map(&s.lyrics);
        r.lyrics_state = QString::from(s.lyrics_state.as_str());
        r.like_enabled = s.like_enabled;
        r.liked = s.liked;
        r.like_blocked_ms = s.like_blocked_ms as i32;
        r.request = to_qvariant_map(&s.request);
        r.last = Some(s);
    }

    fn client(&self) -> &'static SubwaveClient {
        player().subwave()
    }

    fn elapsed_ms(&self) -> i64 {
        self.rust().last.as_ref().map_or(0, |s| s.elapsed_ms(local_now()))
    }

    fn probe(&self, home_page: &QString) {
        self.client().probe(home_page.to_string());
    }

    fn like(&self) {
        self.client().like();
    }

    fn send_request(&self, text: &QString, name: &QString) {
        self.client().send_request(text.to_string(), name.to_string());
    }

    fn cover_url_for(&self, subsonic_id: &QString) -> QString {
        QString::from(self.client().cover_url(&subsonic_id.to_string()).as_str())
    }
}
