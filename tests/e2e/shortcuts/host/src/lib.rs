//! Adds a test page factory to the production shortcut bridge in an isolated host only.
use nsh_core::{
    element::{Local, System},
    prelude::*,
};
use std::{
    cell::RefCell,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
#[cfg(target_os = "macos")]
pub use usher_nsh::__nsh_native_components_macos;
use usher_nsh::shortcuts::{
    self,
    bridge::{ShortcutEnvelope, attach_with_loader},
    catalog::{ModuleConfig, ModuleIndex},
    loader::ModuleLoader,
    modules::ModuleFactory,
};
pub fn __nsh_native_module() -> Registry {
    let mut r = usher_nsh::__nsh_native_module();
    r.module::<ShortcutFixture>();
    r
}
static LOADS: AtomicUsize = AtomicUsize::new(0);
fn config() -> serde_json::Value {
    serde_json::from_str(include_str!("../../config.json")).expect("fixture configuration")
}
fn load() -> Result<ModuleConfig, shortcuts::Error> {
    LOADS.fetch_add(1, Ordering::SeqCst);
    serde_json::from_value(config()["payload"].clone())
        .map_err(|e| shortcuts::Error::InvalidConfig(e.to_string()))
}
fn command(id: &str) -> Result<shortcuts::Command, shortcuts::Error> {
    match id {
        "back" => Ok(shortcuts::Command::Navigation(
            shortcuts::modules::navigation::Command::Back,
        )),
        _ => Err(shortcuts::Error::UnknownCommand(id.into())),
    }
}
#[derive(Default)]
struct FixtureLoader(RefCell<Option<Arc<ModuleLoader>>>);
impl Local for FixtureLoader {}
/// Fixture-owned page module loading and observation.
#[module]
pub struct ShortcutFixture;
#[module]
impl ShortcutFixture {
    /// Starts an empty global catalog so local acceptance never acquires product system keys.
    pub fn global(
        sys: System,
    ) -> Result<shortcuts::global_bridge::GlobalShortcutAttachment, String> {
        let index = ModuleIndex::parse(r#"{"schemaVersion":1,"modules":[]}"#, vec![])
            .map_err(|_| "fixture_catalog")?;
        shortcuts::global_bridge::attach_with_loader(sys, Arc::new(ModuleLoader::new(index)))
    }

    /// Attaches the real service with production navigation and the isolated page declaration.
    pub fn attach(window: String, os: String, sys: System) -> Result<ShortcutEnvelope, String> {
        let local = sys.locals.get::<FixtureLoader>();
        let mut slot = local.0.borrow_mut();
        if slot.is_none() {
            let production = ModuleIndex::application().map_err(|_| "fixture_index")?;
            let mut descriptors: Vec<_> = production
                .descriptors()
                .filter(|d| d.kind != shortcuts::catalog::ModuleKind::Global)
                .map(|d| serde_json::to_value(d).expect("descriptor"))
                .collect();
            descriptors.push(config()["module"].clone());
            let mut factories: Vec<_> = shortcuts::modules::factories()
                .into_iter()
                .filter(|factory| factory.id == "navigation")
                .collect();
            factories.push(ModuleFactory {
                id: "fixture-page",
                load,
                command,
            });
            let index = ModuleIndex::parse(
                &serde_json::json!({"schemaVersion":1,"modules":descriptors}).to_string(),
                factories,
            )
            .map_err(|_| "fixture_catalog")?;
            *slot = Some(Arc::new(ModuleLoader::new(index)));
        }
        attach_with_loader(
            window,
            os,
            sys.clone(),
            slot.as_ref().expect("initialized").clone(),
        )
    }
    /// Reports actual factory evaluations without forcing a module load.
    pub fn loads() -> u32 {
        LOADS.load(Ordering::SeqCst) as u32
    }
}
