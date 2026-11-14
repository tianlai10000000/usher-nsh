//! Extends the production registry only in the isolated macOS button test host.

use dispatch2::DispatchQueue;
use nsh_core::prelude::*;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAccessibility, NSApplication, NSControl, NSOpenPanel, NSPopUpButton, NSSegmentedControl,
    NSView,
};

pub use usher_nsh::__nsh_native_components_macos;

/// Keep every production module and component, adding only the test panel driver.
pub fn __nsh_native_module() -> Registry {
    let mut registry = usher_nsh::__nsh_native_module();
    registry.module::<NativePicker>();
    registry.module::<NativeInteraction>();
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

/// Exercise the production AppKit targets without depending on private NSH view identifiers.
#[module]
pub struct NativeInteraction;

fn visit_controls(view: &NSView, activate: bool, states: &mut Vec<String>) {
    if let Some(label) = view.accessibilityLabel() {
        let label = label.to_string();
        if label.starts_with("Contract ") {
            let control = view
                .downcast_ref::<NSControl>()
                .expect("contract fixtures are controls");
            if activate {
                if let Some(segments) = view.downcast_ref::<NSSegmentedControl>() {
                    segments.setSelectedSegment(1);
                }
                if let Some(popup) = view.downcast_ref::<NSPopUpButton>() {
                    popup.selectItemAtIndex(1);
                }
                if let Some(action) = control.action() {
                    // SAFETY: The production control owns its matching action and retained target.
                    unsafe {
                        control.sendAction_to(Some(action), control.target().as_deref());
                    }
                }
            }
            let selected = if let Some(segments) = view.downcast_ref::<NSSegmentedControl>() {
                segments.selectedSegment()
            } else if let Some(popup) = view.downcast_ref::<NSPopUpButton>() {
                popup.indexOfSelectedItem()
            } else {
                -1
            };
            states.push(format!("{label}:{}:{selected}", control.isEnabled()));
        }
    }
    for child in view.subviews() {
        visit_controls(&child, activate, states);
    }
}

#[module]
impl NativeInteraction {
    /// Inspect or activate only labelled controls in the disposable acceptance host.
    pub fn inspect(activate: bool) -> String {
        let mut states = Vec::new();
        let mut inspect = || {
            let mtm = MainThreadMarker::new().expect("native controls run on the main queue");
            for window in NSApplication::sharedApplication(mtm).windows() {
                if let Some(view) = window.contentView() {
                    visit_controls(&view, activate, &mut states);
                }
            }
        };
        if MainThreadMarker::new().is_some() {
            inspect();
        } else {
            DispatchQueue::main().exec_sync(inspect);
        }
        states.sort();
        states.join(",")
    }
}
