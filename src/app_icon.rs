//! Use the same source image for the Dock during development and bundled builds.

#[cfg(target_os = "macos")]
pub fn init() {
    use objc2::{AnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    let main_thread = MainThreadMarker::new().expect("app icon must be set on the main thread");
    // Embed the image so `cargo run` and copied executables work from any directory.
    let data = NSData::with_bytes(include_bytes!("../popcorn.png"));
    let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else {
        eprintln!("Could not decode the Popcorn application icon");
        return;
    };
    let app = NSApplication::sharedApplication(main_thread);
    // SAFETY: AppKit runs on the main thread; `image` is a valid retained NSImage.
    // NSApplication retains the icon after this function returns.
    unsafe { app.setApplicationIconImage(Some(&image)) };
}

#[cfg(not(target_os = "macos"))]
pub fn init() {}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use objc2::AnyThread;
    use objc2_app_kit::NSImage;
    use objc2_foundation::NSData;

    #[test]
    fn embedded_icon_decodes_as_a_square_appkit_image() {
        let data = NSData::with_bytes(include_bytes!("../popcorn.png"));
        let image = NSImage::initWithData(NSImage::alloc(), &data).expect("valid app icon PNG");
        assert!(image.isValid());
        assert_eq!(image.size().width, 512.);
        assert_eq!(image.size().height, 512.);
    }
}
