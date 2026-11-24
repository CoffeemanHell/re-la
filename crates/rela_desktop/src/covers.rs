//! `image://cover/<coverArt id>`: album art from the player's cover cache, so the Subsonic token never reaches QML
//! (themes are user code). An asynchronous provider: each request gets its own response object, answered when the
//! bytes arrive, so a page of covers loads in parallel. Qt wants that object made per request; Rust has no `new` for
//! a C++ class, so it comes from Qt's metatype of `CoverResponse` (registered with QML as an anonymous type).
use crate::hub::player;
use core::pin::Pin;
use cxx_qt::casting::Upcast;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QImage, QSize, QString, TransformationMode, AspectRatioMode};
use std::sync::OnceLock;

/// QByteArrayView as Qt 6 lays it out (qbytearrayview.h; Qt 7 swaps the two fields).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ByteArrayView {
    size: isize,
    data: *const u8,
}
// SAFETY: the layout above is QByteArrayView's in Qt 6, a trivially copyable view
unsafe impl cxx::ExternType for ByteArrayView {
    type Id = cxx::type_id!("QByteArrayView");
    type Kind = cxx::kind::Trivial;
}

/// QMetaType: one pointer to its interface.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct MetaType(*const u8);
// SAFETY: QMetaType is one interface pointer, trivially copyable
unsafe impl cxx::ExternType for MetaType {
    type Id = cxx::type_id!("QMetaType");
    type Kind = cxx::kind::Trivial;
}

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qsize.h");
        type QSize = cxx_qt_lib::QSize;
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;
    }

    extern "C++Qt" {
        include!(<QtQuick/QQuickImageProvider>);
        #[qobject]
        type QQuickImageResponse;
    }

    unsafe extern "C++" {
        type QQuickAsyncImageProvider; // no Q_OBJECT of its own: not a #[qobject]
        type QQuickTextureFactory;
        #[Self = "QQuickTextureFactory"]
        #[rust_name = "texture_factory_for_image"]
        fn textureFactoryForImage(image: &QImage) -> *mut QQuickTextureFactory;

        include!(<QtCore/QMetaType>);
        type QMetaType = super::MetaType;
        type QByteArrayView = super::ByteArrayView;
        type QMetaObject;
        // cxx has no `void`; QtGui's OpenGL header (desktop Qt on Linux) typedefs it as GLvoid
        include!(<QtGui/qopengl.h>);
        #[cxx_name = "GLvoid"]
        type Void;
        #[Self = "QMetaType"]
        #[rust_name = "meta_type_from_name"]
        fn fromName(name: QByteArrayView) -> QMetaType;
        #[rust_name = "meta_object"]
        fn metaObject(self: &QMetaType) -> *const QMetaObject;
        #[rust_name = "meta_type"]
        fn metaType(self: &QMetaObject) -> QMetaType;
        // copy: null or a valid object of this type; the caller owns the result
        #[allow(clippy::missing_safety_doc)]
        unsafe fn create(self: &QMetaType, copy: *const Void) -> *mut Void;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element] // registered (not creatable) so main finds its metatype by name, as for CoverResponse
        #[qml_uncreatable]
        #[base = QQuickAsyncImageProvider]
        type CoverProvider = super::CoverProviderRust;

        #[qobject]
        #[qml_element]
        #[qml_uncreatable]
        #[base = QQuickImageResponse]
        type CoverResponse = super::CoverResponseRust;
    }

    unsafe extern "RustQt" {
        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "requestImageResponse"]
        fn request_image_response(self: Pin<&mut CoverProvider>, id: &QString, requested_size: &QSize) -> *mut QQuickImageResponse;

        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "textureFactory"]
        fn texture_factory(self: &CoverResponse) -> *mut QQuickTextureFactory;

        #[qsignal]
        #[inherit]
        fn finished(self: Pin<&mut CoverResponse>);
    }

    impl cxx_qt::Threading for CoverResponse {}
    // the bases take no parent; CoverResponse's default constructor is what its metatype calls
    impl cxx_qt::Constructor<()> for CoverProvider {}
    impl cxx_qt::Constructor<()> for CoverResponse {}
}

impl cxx_qt::Constructor<()> for ffi::CoverProvider {
    type NewArguments = ();
    type BaseArguments = ();
    type InitializeArguments = ();
    fn route_arguments(_: ()) -> ((), (), ()) {
        ((), (), ())
    }
    fn new(_: ()) -> CoverProviderRust {
        CoverProviderRust
    }
}

impl cxx_qt::Constructor<()> for ffi::CoverResponse {
    type NewArguments = ();
    type BaseArguments = ();
    type InitializeArguments = ();
    fn route_arguments(_: ()) -> ((), (), ()) {
        ((), (), ())
    }
    fn new(_: ()) -> CoverResponseRust {
        CoverResponseRust::default()
    }
}

#[derive(Default)]
pub struct CoverProviderRust;

#[derive(Default)]
pub struct CoverResponseRust {
    image: QImage,
}

/// A cxx-qt class's metatype, by the pointer type name QML registered ("CoverResponse*").
fn metatype(pointer_name: &[u8]) -> Option<MetaType> {
    let ptr = MetaType::meta_type_from_name(ByteArrayView { size: pointer_name.len() as isize, data: pointer_name.as_ptr() });
    let mo = ptr.meta_object();
    // SAFETY: a registered type's QMetaObject is static: it lives as long as the program
    (!mo.is_null()).then(|| unsafe { &*mo }.meta_type())
}

fn response_type() -> Option<ffi::QMetaType> {
    static T: OnceLock<usize> = OnceLock::new(); // asked once per cover: found once
    let p = *T.get_or_init(|| metatype(b"CoverResponse*").map_or(0, |t| t.0 as usize));
    (p != 0).then_some(MetaType(p as *const u8))
}

/// A new provider for QQmlEngine::addImageProvider (the engine takes it); None when the QML module isn't registered.
pub fn new_provider() -> Option<*mut ffi::CoverProvider> {
    let t = metatype(b"CoverProvider*")?;
    // SAFETY: t is CoverProvider's metatype; null means default-construct; the caller hands the object to the engine
    Some(unsafe { t.create(std::ptr::null()) } as *mut ffi::CoverProvider)
}

impl ffi::CoverProvider {
    fn request_image_response(self: Pin<&mut Self>, id: &QString, requested_size: &QSize) -> *mut ffi::QQuickImageResponse {
        let Some(t) = response_type() else {
            eprintln!("covers: CoverResponse is not registered");
            return std::ptr::null_mut();
        };
        // SAFETY: t is CoverResponse's metatype; null means default-construct; Qt takes the object below
        let r = unsafe { t.create(std::ptr::null()) } as *mut ffi::CoverResponse;
        // SAFETY: r was just made, is not null and never moves (a QObject)
        let qt = unsafe { Pin::new_unchecked(&mut *r) }.qt_thread();
        let (w, h) = (requested_size.width(), requested_size.height());
        player().cover(id.to_string(), w.max(h).max(0) as u32, move |bytes| {
            let _ = qt.queue(move |mut resp| {
                if let Some(img) = bytes.and_then(|b| QImage::from_data(&b, None)) {
                    let fits = w <= 0 || h <= 0 || (img.width() <= w && img.height() <= h);
                    let img = if fits {
                        img
                    } else {
                        img.scaled(w, h, AspectRatioMode::KeepAspectRatio, TransformationMode::SmoothTransformation)
                    };
                    resp.as_mut().rust_mut().image = img;
                }
                resp.finished();
            });
        });
        // SAFETY: r is a CoverResponse, whose base is QQuickImageResponse; Qt deletes it when the image is taken
        unsafe { <ffi::CoverResponse as Upcast<ffi::QQuickImageResponse>>::upcast_ptr(r) as *mut _ }
    }
}

impl ffi::CoverResponse {
    fn texture_factory(&self) -> *mut ffi::QQuickTextureFactory {
        ffi::QQuickTextureFactory::texture_factory_for_image(&self.image)
    }
}
