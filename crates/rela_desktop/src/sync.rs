//! `Sync` (relay client) as the QObject QML knows: the properties, signals and calls that
//! themes use. The client itself is the player's (hub): states and commands go to the core, not through Qt.
use crate::hub::{SYNC_QT, player};
use crate::qt::to_qvariant;
use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QList, QString, QStringList, QVariant};
use rela_core::sync::{SyncClient, SyncEvent, SyncSnapshot};

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qlist.h");
        type QList_QVariant = cxx_qt_lib::QList<cxx_qt_lib::QVariant>;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(bool, available, READ, NOTIFY = changed)]
        #[qproperty(QList_QVariant, users, READ, NOTIFY = users_changed)]
        #[qproperty(QStringList, listeners, READ, NOTIFY = users_changed)]
        #[qproperty(QString, following, READ, NOTIFY = changed)]
        #[qproperty(bool, can_control, cxx_name = "canControl", READ, NOTIFY = changed)]
        #[qproperty(bool, allow_control, cxx_name = "allowControl", READ, WRITE = set_allow_control, NOTIFY = changed)]
        #[qproperty(bool, hidden, READ, WRITE = set_hidden, NOTIFY = changed)]
        #[cxx_name = "Sync"]
        type RustSync = super::SyncRust;
    }

    unsafe extern "RustQt" {
        #[qsignal]
        fn changed(self: Pin<&mut RustSync>);
        #[qsignal]
        #[cxx_name = "usersChanged"]
        fn users_changed(self: Pin<&mut RustSync>);
        #[qsignal]
        fn connected(self: Pin<&mut RustSync>);
        /// A state from the root we follow, as a JSON object.
        /// A follower's command (we are the root); args as a JSON object.
        #[qsignal]
        fn ended(self: Pin<&mut RustSync>, reason: QString);
        #[qsignal]
        #[cxx_name = "listenerJoined"]
        fn listener_joined(self: Pin<&mut RustSync>, name: QString);
        #[qsignal]
        #[cxx_name = "listenerLeft"]
        fn listener_left(self: Pin<&mut RustSync>, name: QString);
        #[qsignal]
        fn failed(self: Pin<&mut RustSync>, code: QString);

        #[cxx_name = "setAllowControl"]
        fn set_allow_control(self: Pin<&mut RustSync>, on: bool);
        #[cxx_name = "setHidden"]
        fn set_hidden(self: Pin<&mut RustSync>, on: bool);

        #[qinvokable]
        fn follow(self: &RustSync, user: &QString);
        #[qinvokable]
        fn unfollow(self: &RustSync);
        /// With `bye`, followers hear at once that we left (waits up to 500 ms for it to go out).
        #[qinvokable]
        fn stop(self: &RustSync, bye: bool);
    }

    impl cxx_qt::Threading for RustSync {}
    impl cxx_qt::Initialize for RustSync {}
}

#[derive(Default)]
pub struct SyncRust {
    available: bool,
    users: QList<QVariant>,
    listeners: QStringList,
    following: QString,
    can_control: bool,
    allow_control: bool,
    hidden: bool,
}

/// A relay event (from hub, already on the Qt thread): the snapshot's values, then the signal it stands for.
pub(crate) fn deliver(mut o: Pin<&mut ffi::RustSync>, event: SyncEvent, snapshot: SyncSnapshot) {
    o.as_mut().apply(&snapshot);
    match event {
        SyncEvent::Changed => o.changed(),
        SyncEvent::UsersChanged => o.users_changed(),
        SyncEvent::Connected => o.connected(),
        SyncEvent::Ended(r) => o.ended(QString::from(r.as_str())),
        SyncEvent::ListenerJoined(n) => o.listener_joined(QString::from(n.as_str())),
        SyncEvent::ListenerLeft(n) => o.listener_left(QString::from(n.as_str())),
        SyncEvent::Failed(c) => o.failed(QString::from(c.as_str())),
        SyncEvent::StateReceived(_) | SyncEvent::CommandReceived { .. } => {} // the player's (core) business
    }
}

impl cxx_qt::Initialize for ffi::RustSync {
    fn initialize(mut self: Pin<&mut Self>) {
        *SYNC_QT.lock().unwrap() = Some(self.qt_thread());
        let s = player().settings();
        let mut r = self.as_mut().rust_mut();
        r.can_control = true; // alone: our own buttons work
        r.allow_control = s.get_bool("sync/allowControl", false);
        r.hidden = s.get_bool("sync/hidden", false);
    }
}

impl ffi::RustSync {
    /// Takes the snapshot's values; the caller sends the signal the event stands for.
    fn apply(mut self: Pin<&mut Self>, s: &SyncSnapshot) {
        let mut r = self.as_mut().rust_mut();
        r.available = s.available;
        r.users = s.users.iter().map(to_qvariant).collect();
        let mut listeners = QList::<QString>::default();
        for n in &s.listeners {
            listeners.append(QString::from(n.as_str()));
        }
        r.listeners = QStringList::from(&listeners);
        r.following = QString::from(s.following.as_str());
        r.can_control = s.can_control;
        r.allow_control = s.allow_control;
        r.hidden = s.hidden;
    }

    fn client(&self) -> &'static SyncClient {
        player().sync()
    }

    fn set_allow_control(self: Pin<&mut Self>, on: bool) {
        player().settings().set("sync/allowControl", on.into());
        self.client().set_allow_control(on);
    }

    fn set_hidden(self: Pin<&mut Self>, on: bool) {
        player().settings().set("sync/hidden", on.into());
        self.client().set_hidden(on);
    }

    fn follow(&self, user: &QString) {
        self.client().follow(&user.to_string());
    }

    fn unfollow(&self) {
        self.client().unfollow();
    }

    fn stop(&self, bye: bool) {
        self.client().stop(bye);
    }
}
