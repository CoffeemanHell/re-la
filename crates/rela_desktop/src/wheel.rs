//! KDE's "scroll by moving the pointer while holding a button": Qt Quick takes those events as wheel notches and
//! flicks with a capped speed, so a fast drag never goes fast. This filter (installed on the app) moves the Flickable
//! under the pointer as far as the pointer moves instead; it stops at the ends without the overshoot bounce (the user's
//! choice over keeping a C++ file). The wheel and two-finger scrolling stay as they are. RELA_WHEEL_LOG=1 prints every
//! wheel event.
use crate::qtgui::{self, ffi as qt};
use core::pin::Pin;
use cxx_qt_lib::{QPointF, QVariant};
use std::ffi::CStr;

/// KDE's button-scrolling reaches Qt as a touchpad-type device with no gesture phase and small steps (angle 18 = 1.5 px
/// of pointer motion). The pixels to scroll for such an event, as far as Qt Widgets goes (angle / 120 * lines * 24 px);
/// None for anything else (a mouse wheel, a two-finger scroll).
pub fn button_scroll(phase: i32, touchpad: bool, pixel: (i32, i32), angle: (i32, i32), lines: i32) -> Option<(f64, f64)> {
    if phase != 0 || !touchpad || pixel == (0, 0) {
        return None;
    }
    let k = f64::from(lines) / 5.0; // * 24 / 120
    Some(((f64::from(angle.0) * k).round(), (f64::from(angle.1) * k).round()))
}

/// One axis of a Flickable: where the content goes when the pointer moves by delta (the content follows it), kept
/// within [origin - margin_lo, origin + content + margin_hi - view]. None when it wouldn't move.
pub fn flick_to(pos: f64, delta: f64, origin: f64, content: f64, view: f64, margin_lo: f64, margin_hi: f64) -> Option<f64> {
    let lo = origin - margin_lo;
    let hi = (origin + content + margin_hi - view).max(lo);
    let to = (pos - delta).clamp(lo, hi);
    (delta != 0.0 && (to - pos).abs() > 0.01).then_some(to)
}

const fn c(s: &'static [u8]) -> &'static CStr {
    match CStr::from_bytes_with_nul(s) {
        Ok(c) => c,
        Err(_) => panic!("not nul-terminated"),
    }
}

fn get(o: &qt::QObject, name: &CStr) -> f64 {
    // SAFETY: name is a nul-terminated C string for the call
    unsafe { o.property(name.as_ptr()) }.value::<f64>().unwrap_or(0.0)
}

/// Moves one axis of the Flickable; true when it moved.
fn move_axis(o: Pin<&mut qt::QObject>, delta: f64, names: [&CStr; 6]) -> bool {
    let [pos, origin, content, view, lo, hi] = names;
    let r = &*o;
    let Some(to) = flick_to(get(r, pos), delta, get(r, origin), get(r, content), get(r, view), get(r, lo), get(r, hi)) else {
        return false;
    };
    // SAFETY: pos is a nul-terminated C string for the call
    unsafe { o.set_property(pos.as_ptr(), &QVariant::from(&to)) };
    true
}

/// The deepest item under the pointer, then up its parents to the first interactive Flickable that can move.
///
/// # Safety
/// The window's items must not be deleted during the call (true inside its own event dispatch).
unsafe fn scroll(window: &qt::QQuickWindow, at: &QPointF, dx: f64, dy: f64) -> bool {
    let mut chain = vec![];
    let mut item = window.content_item();
    while !item.is_null() {
        chain.push(item);
        // SAFETY: not null (the loop checks) and alive for the call (see # Safety)
        let it = unsafe { &*item };
        let p = it.map_from_scene(at);
        item = it.child_at(p.x(), p.y());
    }
    for item in chain.into_iter().rev() {
        // SAFETY: a live item (see # Safety); QQuickItem's first base is QObject, at the same address
        let o = unsafe { &mut *(item as *mut qt::QObject) };
        // SAFETY: a nul-terminated literal
        if !unsafe { o.inherits(c(b"QQuickFlickable\0").as_ptr()) } {
            continue;
        }
        // SAFETY: a nul-terminated literal
        if !unsafe { o.property(c(b"interactive\0").as_ptr()) }.value::<bool>().unwrap_or(false) {
            continue;
        }
        // SAFETY: QObjects never move
        let mut o = unsafe { Pin::new_unchecked(o) };
        let y = move_axis(o.as_mut(), dy, [c(b"contentY\0"), c(b"originY\0"), c(b"contentHeight\0"), c(b"height\0"), c(b"topMargin\0"), c(b"bottomMargin\0")]);
        let x = move_axis(o.as_mut(), dx, [c(b"contentX\0"), c(b"originX\0"), c(b"contentWidth\0"), c(b"width\0"), c(b"leftMargin\0"), c(b"rightMargin\0")]);
        if x || y {
            return true;
        }
    }
    false
}

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!(<QtCore/QEvent>); // moc registers eventFilter's argument types: they must be complete
        type QEvent = crate::qtgui::ffi::QEvent; // QObject comes with every cxx-qt bridge
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element] // Shell.qml makes it; it puts itself on the app
        type WheelBoost = super::WheelBoostRust;
    }

    unsafe extern "RustQt" {
        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "eventFilter"]
        unsafe fn event_filter(self: Pin<&mut WheelBoost>, watched: *mut QObject, event: *mut QEvent) -> bool;
    }

    impl cxx_qt::Initialize for WheelBoost {}
}

pub struct WheelBoostRust {
    log: bool,
}

impl Default for WheelBoostRust {
    fn default() -> Self {
        let log = std::env::var("RELA_WHEEL_LOG").is_ok_and(|v| v != "0" && !v.is_empty());
        if log {
            eprintln!("wheel log on");
        }
        WheelBoostRust { log }
    }
}

impl ffi::WheelBoost {
    /// # Safety
    /// Qt's eventFilter: both pointers valid for the call.
    unsafe fn event_filter(self: Pin<&mut Self>, watched: *mut cxx_qt::QObject, event: *mut qt::QEvent) -> bool {
        let watched = watched as *mut qt::QObject; // the same ::QObject, bound in qtgui
        // SAFETY: Qt passes both valid for the call
        let (w, e) = unsafe { (&*watched, &*event) };
        if e.event_type() != qtgui::WHEEL || !w.is_window_type() {
            return false; // the window's copy only
        }
        // SAFETY: the type check above says it is a QWheelEvent
        let e = unsafe { &*(event as *const qt::QWheelEvent) };
        let dev = e.pointing_device();
        // SAFETY: null or a device Qt keeps for the program's life
        let dev = unsafe { dev.as_ref() };
        let (px, an) = (e.pixel_delta(), e.angle_delta());
        if self.log {
            eprintln!(
                "wheel pixel={},{} angle={},{} phase={} inverted={} device='{}' type={}",
                px.x(),
                px.y(),
                an.x(),
                an.y(),
                e.phase().0,
                e.inverted(),
                dev.map(|d| d.name().to_string()).unwrap_or_default(),
                dev.map_or(-1, |d| d.device_type().0)
            );
        }
        // SAFETY: null or the application's style hints, alive while it runs
        let lines = unsafe { qt::QGuiApplication::style_hints().as_ref() }.map_or(3, |h| h.wheel_scroll_lines());
        let touchpad = dev.is_some_and(|d| d.device_type() == qtgui::TOUCHPAD);
        let Some((dx, dy)) = button_scroll(e.phase().0, touchpad, (px.x(), px.y()), (an.x(), an.y()), lines) else {
            return false;
        };
        // SAFETY: a nul-terminated literal
        if !unsafe { w.inherits(c(b"QQuickWindow\0").as_ptr()) } {
            return false;
        }
        // SAFETY: inherits() said QQuickWindow; QWindow's first base is QObject, at the same address
        let window = unsafe { &*(watched as *const qt::QQuickWindow) };
        // SAFETY: we are inside the window's event dispatch: its items stay for the call
        unsafe { scroll(window, &e.position(), dx, dy) } // nothing to move: Qt gets the event as before
    }
}

impl cxx_qt::Initialize for ffi::WheelBoost {
    fn initialize(self: Pin<&mut Self>) {
        // SAFETY: only turned into a pointer, never moved; QObject is its base, at the same address
        let me = unsafe { self.get_unchecked_mut() } as *mut ffi::WheelBoost as *mut qt::QObject;
        // SAFETY: null or the running application
        if let Some(app) = unsafe { qt::QCoreApplication::instance().as_mut() } {
            // SAFETY: QObjects never move; Qt drops the filter when it is deleted
            unsafe { Pin::new_unchecked(app).install_event_filter(me) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_button_scrolling_counts() {
        assert_eq!(button_scroll(0, true, (0, -2), (0, -18), 3), Some((0.0, -11.0))); // measured: angle 18, pixel 1-2, phase 0
        assert_eq!(button_scroll(0, true, (0, 3), (0, 36), 3), Some((0.0, 22.0))); // a fast step goes twice as far
        assert_eq!(button_scroll(2, true, (0, -2), (0, -18), 3), None); // two-finger scroll comes with phases: untouched
        assert_eq!(button_scroll(0, false, (0, 0), (0, 120), 3), None); // a mouse wheel: untouched
        assert_eq!(button_scroll(0, true, (0, 0), (0, 120), 3), None);
    }

    #[test]
    fn flick_stays_in_bounds() {
        // a 1000 px list in a 300 px view: 0..700
        assert_eq!(flick_to(100.0, -11.0, 0.0, 1000.0, 300.0, 0.0, 0.0), Some(111.0)); // pointer up: content up
        assert_eq!(flick_to(5.0, 11.0, 0.0, 1000.0, 300.0, 0.0, 0.0), Some(0.0)); // stops at the top
        assert_eq!(flick_to(0.0, 11.0, 0.0, 1000.0, 300.0, 0.0, 0.0), None); // already there: the parent may move
        assert_eq!(flick_to(695.0, -11.0, 0.0, 1000.0, 300.0, 0.0, 0.0), Some(700.0));
        assert_eq!(flick_to(0.0, -11.0, 0.0, 200.0, 300.0, 0.0, 0.0), None); // shorter than the view
        assert_eq!(flick_to(-20.0, -11.0, -20.0, 1000.0, 300.0, 0.0, 10.0), Some(-9.0)); // ListView origin, margins
        assert_eq!(flick_to(0.0, 30.0, 0.0, 1000.0, 300.0, 20.0, 0.0), Some(-20.0)); // topMargin reachable
        assert_eq!(flick_to(50.0, 0.0, 0.0, 1000.0, 300.0, 0.0, 0.0), None);
    }
}
