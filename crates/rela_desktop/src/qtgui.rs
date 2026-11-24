//! The few Qt calls the wheel filter and App make, bound with cxx. Qt's nested enums come in as their int values.
use cxx::{ExternType, type_id};
// named in the library, not only in main.rs: the build's Qt initializers call this crate, so the test binaries must link it
pub use cxx_qt_lib_extras::QApplication;

macro_rules! int_enum {
    ($name:ident, $cpp:literal) => {
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub struct $name(pub i32);
        // SAFETY: a repr(transparent) i32, as the C++ enum is
        unsafe impl ExternType for $name {
            type Id = type_id!($cpp);
            type Kind = cxx::kind::Trivial;
        }
    };
}
int_enum!(EventType, "QEvent::Type");
int_enum!(DeviceType, "QInputDevice::DeviceType");
int_enum!(ScrollPhase, "Qt::ScrollPhase");

pub const WHEEL: EventType = EventType(31); // QEvent::Wheel
pub const TOUCHPAD: DeviceType = DeviceType(0x4); // QInputDevice::DeviceType::TouchPad

#[cxx::bridge]
#[allow(clippy::missing_safety_doc)] // the unsafe ones take a nul-terminated C string
pub mod ffi {
    unsafe extern "C++" {
        include!(<QtGui/QWheelEvent>);
        include!(<QtGui/QPointingDevice>);
        include!(<QtGui/QGuiApplication>);
        include!(<QtGui/QStyleHints>);
        include!(<QtGui/QFontDatabase>);
        type QCoreApplication;
        #[Self = "QCoreApplication"]
        fn quit();
        #[Self = "QCoreApplication"]
        fn instance() -> *mut QCoreApplication;
        #[Self = "QCoreApplication"]
        #[cxx_name = "setOrganizationName"]
        fn set_organization_name(name: &QString);
        #[Self = "QCoreApplication"]
        #[cxx_name = "setApplicationName"]
        fn set_application_name(name: &QString);
        #[Self = "QGuiApplication"]
        #[cxx_name = "setQuitOnLastWindowClosed"]
        fn set_quit_on_last_window_closed(quit: bool);
        #[cxx_name = "installEventFilter"]
        unsafe fn install_event_filter(self: Pin<&mut QCoreApplication>, filter: *mut QObject);
        include!(<QtQuick/QQuickWindow>);
        include!(<QtQuick/QQuickItem>);
        include!("cxx-qt-lib/qpoint.h");
        type QPoint = cxx_qt_lib::QPoint;
        include!("cxx-qt-lib/qpointf.h");
        type QPointF = cxx_qt_lib::QPointF;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qqmlengine.h");
        type QQmlEngine = cxx_qt_lib::QQmlEngine;
        include!(<QtQml/QQmlEngine>);
        /// The engine that made this object (null for one made outside QML).
        #[cxx_name = "qmlEngine"]
        unsafe fn qml_engine(object: *const QObject) -> *mut QQmlEngine;
        type QObject; // ::QObject (cxx_qt::QObject is another crate's: no methods can be added to it)

        type QEvent;
        type QWheelEvent;
        type QPointingDevice;
        type QStyleHints;
        type QGuiApplication;
        type QFontDatabase;
        type QQuickWindow;
        type QQuickItem;
        #[namespace = "QEvent"]
        #[cxx_name = "Type"]
        type EventType = super::EventType;
        #[namespace = "QInputDevice"]
        #[cxx_name = "DeviceType"]
        type DeviceType = super::DeviceType;
        #[namespace = "Qt"]
        type ScrollPhase = super::ScrollPhase;

        #[cxx_name = "type"]
        fn event_type(self: &QEvent) -> EventType;
        #[cxx_name = "pixelDelta"]
        fn pixel_delta(self: &QWheelEvent) -> QPoint;
        #[cxx_name = "angleDelta"]
        fn angle_delta(self: &QWheelEvent) -> QPoint;
        fn phase(self: &QWheelEvent) -> ScrollPhase;
        fn inverted(self: &QWheelEvent) -> bool;
        fn position(self: &QWheelEvent) -> QPointF;
        #[cxx_name = "pointingDevice"]
        fn pointing_device(self: &QWheelEvent) -> *const QPointingDevice;
        #[cxx_name = "type"]
        fn device_type(self: &QPointingDevice) -> DeviceType;
        fn name(self: &QPointingDevice) -> QString;

        #[Self = "QGuiApplication"]
        #[cxx_name = "styleHints"]
        fn style_hints() -> *mut QStyleHints;
        #[cxx_name = "wheelScrollLines"]
        fn wheel_scroll_lines(self: &QStyleHints) -> i32;

        #[Self = "QFontDatabase"]
        #[cxx_name = "addApplicationFont"]
        fn add_application_font(path: &QString) -> i32;

        #[cxx_name = "isWindowType"]
        fn is_window_type(self: &QObject) -> bool;
        unsafe fn inherits(self: &QObject, class_name: *const c_char) -> bool;
        unsafe fn property(self: &QObject, name: *const c_char) -> QVariant;
        #[cxx_name = "setProperty"]
        unsafe fn set_property(self: Pin<&mut QObject>, name: *const c_char, value: &QVariant) -> bool;

        #[cxx_name = "contentItem"]
        fn content_item(self: &QQuickWindow) -> *mut QQuickItem;
        #[cxx_name = "childAt"]
        fn child_at(self: &QQuickItem, x: f64, y: f64) -> *mut QQuickItem;
        #[cxx_name = "mapFromScene"]
        fn map_from_scene(self: &QQuickItem, point: &QPointF) -> QPointF;
    }
}
