//! Registers the shared window material and optional AppKit controls.

use std::sync::atomic::{AtomicBool, Ordering};

use nsh_platform_macos::Components;

#[cfg(usher_macos_appkit)]
mod controls;
mod system_symbol;
mod window_geometry;
mod window_material;

static MATERIAL_REGISTERED: AtomicBool = AtomicBool::new(false);
#[cfg(usher_macos_appkit)]
static REGISTERED: AtomicBool = AtomicBool::new(false);

pub(crate) fn register(components: &Components) {
    window_material::register(components);
    system_symbol::register(components);
    MATERIAL_REGISTERED.store(true, Ordering::Release);

    // Keep the optional controls gated by the runtime support of the original theme.
    #[cfg(usher_macos_appkit)]
    if nsh_platform_macos::objc2::runtime::AnyClass::get(c"NSGlassEffectView").is_some() {
        controls::register(components);
        REGISTERED.store(true, Ordering::Release);
    }
}

pub(crate) fn window_material_available() -> bool {
    MATERIAL_REGISTERED.load(Ordering::Acquire)
}

#[cfg(usher_macos_appkit)]
pub(crate) fn theme_available() -> bool {
    REGISTERED.load(Ordering::Acquire)
}
