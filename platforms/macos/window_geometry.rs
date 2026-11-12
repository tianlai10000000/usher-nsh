//! Applies the initial frame and native resize limits once per window.

use std::cell::RefCell;
use std::ptr;

use nsh_platform_macos::objc2::rc::Weak;
use nsh_platform_macos::objc2_app_kit::NSWindow;
use nsh_platform_macos::objc2_foundation::NSSize;

/// The smallest window frame the layout is designed for, in points.
const MIN_WIDTH: f64 = 800.0;
const MIN_HEIGHT: f64 = 600.0;

thread_local! {
    // The native window outlives TSX remounts, so HMR must not reset its frame.
    static CONFIGURED_WINDOW: RefCell<Weak<NSWindow>> = RefCell::new(Weak::default());
}

/// Configures the app window on its owning AppKit thread, after mounting.
pub(super) fn configure(window: &NSWindow) {
    CONFIGURED_WINDOW.with(|configured| {
        if configured
            .borrow()
            .load()
            .is_some_and(|previous| ptr::eq(&*previous, window))
        {
            return;
        }

        let minimum = NSSize::new(MIN_WIDTH, MIN_HEIGHT);
        window.setMinSize(minimum);
        let mut frame = window.frame();
        // Only grow a window that opens smaller than the minimum. A window the host or the
        // user has already made larger keeps its size: the minimum is a floor, not a preset.
        let grown = NSSize::new(
            frame.size.width.max(minimum.width),
            frame.size.height.max(minimum.height),
        );
        if grown.width != frame.size.width || grown.height != frame.size.height {
            // Keep the top edge fixed, so growing extends the window downwards.
            frame.origin.y += frame.size.height - grown.height;
            frame.size = grown;
            window.setFrame_display(frame, true);
        }
        *configured.borrow_mut() = Weak::new(window);
    });
}
