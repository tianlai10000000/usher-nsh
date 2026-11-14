//! Independent Carbon allocation oracle for testing actual NSH global registrations on macOS.
use dispatch2::DispatchQueue;
use objc2::MainThreadMarker;
use std::{cell::Cell, ffi::c_void, ptr};
#[repr(C)]
struct HotKeyId {
    signature: u32,
    id: u32,
}
#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn GetApplicationEventTarget() -> *mut c_void;
    fn RegisterEventHotKey(
        key: u32,
        modifiers: u32,
        id: HotKeyId,
        target: *mut c_void,
        options: u32,
        output: *mut *mut c_void,
    ) -> i32;
    fn UnregisterEventHotKey(handle: *mut c_void) -> i32;
}
thread_local! { static BLOCKER: Cell<*mut c_void> = const { Cell::new(ptr::null_mut()) }; }
fn main<T: Send>(run: impl FnOnce() -> T + Send) -> T {
    if MainThreadMarker::new().is_some() {
        run()
    } else {
        let mut result = None;
        DispatchQueue::main().exec_sync(|| result = Some(run()));
        result.expect("main queue completed")
    }
}
pub fn reserve(probe: bool) -> bool {
    main(move || {
        let config = super::config();
        let native = &config["native"];
        let mut handle = ptr::null_mut();
        // SAFETY: Carbon runs on the main queue, output points to live storage and the
        // target is this isolated test application's own event target.
        let accepted = unsafe {
            RegisterEventHotKey(
                native["keyCode"].as_u64().unwrap() as u32,
                native["modifiers"].as_u64().unwrap() as u32,
                HotKeyId {
                    signature: native["signature"].as_u64().unwrap() as u32,
                    id: native["id"].as_u64().unwrap() as u32,
                },
                GetApplicationEventTarget(),
                native["exclusive"].as_u64().unwrap() as u32,
                &mut handle,
            )
        } == 0;
        if accepted {
            if probe {
                // SAFETY: This oracle just acquired this handle and releases it exactly once.
                unsafe {
                    UnregisterEventHotKey(handle);
                }
            } else {
                BLOCKER.with(|slot| {
                    let old = slot.replace(handle);
                    if !old.is_null() {
                        // SAFETY: Only this oracle owns the previous handle on this main thread.
                        unsafe {
                            UnregisterEventHotKey(old);
                        }
                    }
                });
            }
        }
        accepted
    })
}
pub fn release() {
    main(|| {
        BLOCKER.with(|slot| {
            let old = slot.replace(ptr::null_mut());
            if !old.is_null() {
                // SAFETY: Taking the oracle's handle prevents duplicate release.
                unsafe {
                    UnregisterEventHotKey(old);
                }
            }
        })
    });
}

/// A background precondition is verified in the separate helper immediately before input is posted.
pub fn background_press(key: &str) -> Result<String, String> {
    let config = super::config();
    if config["input"]["keys"].get(key).is_none() {
        return Err("unknown_test_key".into());
    }
    let helper = config["inputHelper"]
        .as_str()
        .ok_or("system_input_not_configured")?;
    let input_config = config["inputConfig"]
        .as_str()
        .ok_or("system_input_not_configured")?;
    let receipt = config["inputReceipt"]
        .as_str()
        .ok_or("system_input_not_configured")?;
    match std::fs::remove_file(receipt) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    main(|| {
        let mtm = MainThreadMarker::new().expect("main queue");
        objc2_app_kit::NSApplication::sharedApplication(mtm).hide(None);
    });
    let status = std::process::Command::new("/usr/bin/open")
        .args([
            "-g",
            "-n",
            helper,
            "--args",
            "--send",
            input_config,
            key,
            &std::process::id().to_string(),
            receipt,
        ])
        .status()
        .map_err(|error| error.to_string())?;
    if !status.success() {
        return Err("input_helper_launch_failed".into());
    }
    let timeout = std::time::Duration::from_millis(config["input"]["timeoutMs"].as_u64().unwrap());
    let poll = std::time::Duration::from_millis(config["input"]["pollMs"].as_u64().unwrap());
    let deadline = std::time::Instant::now() + timeout;
    while !std::path::Path::new(receipt).exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(poll);
    }
    std::fs::read_to_string(receipt).map_err(|error| error.to_string())
}
pub fn active() -> bool {
    main(|| {
        objc2_app_kit::NSApplication::sharedApplication(
            MainThreadMarker::new().expect("main queue"),
        )
        .isActive()
    })
}
