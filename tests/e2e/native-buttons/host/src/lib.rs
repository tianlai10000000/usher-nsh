//! Extends the production registry only in the isolated macOS button test host.

use dispatch2::DispatchQueue;
use nsh_core::prelude::*;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSOpenPanel};

pub use usher_nsh::__nsh_native_components_macos;

/// Keep every production module and component, adding only the test panel driver.
pub fn __nsh_native_module() -> Registry {
    let mut registry = usher_nsh::__nsh_native_module();
    registry.module::<NativePicker>();
    registry
}

/// Test driver for the real AppKit open panel in this host process.
#[module]
pub struct NativePicker;

/// Operate only on a visible open panel attached to this isolated host's own window.
fn with_panel(cancel: bool) -> bool {
    let inspect = || {
        let mtm = MainThreadMarker::new().expect("panel access runs on the main queue");
        let app = NSApplication::sharedApplication(mtm);
        for window in app.windows() {
            let Some(sheet) = window.attachedSheet() else {
                continue;
            };
            let Ok(panel) = sheet.downcast::<NSOpenPanel>() else {
                continue;
            };
            if panel.isVisible() {
                if cancel {
                    // SAFETY: This is a retained NSOpenPanel on the AppKit main thread.
                    // Its public cancel action delivers the real pick completion callback.
                    unsafe { panel.cancel(None) };
                }
                return true;
            }
        }
        false
    };
    if MainThreadMarker::new().is_some() {
        inspect()
    } else {
        let mut result = false;
        DispatchQueue::main().exec_sync(|| {
            result = inspect();
        });
        result
    }
}

#[module]
impl NativePicker {
    /// Cancel a real, visible NSOpenPanel; false means no panel was ready yet.
    pub fn cancel_visible() -> bool {
        with_panel(true)
    }

    /// Observe panel closure independently of the application's busy projection.
    pub fn visible() -> bool {
        with_panel(false)
    }
}
