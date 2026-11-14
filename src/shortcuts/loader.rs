//! Caches immutable normalized declarations on demand, never page handlers or native handles.
use super::{
    Error,
    catalog::{LoadedModule, ModuleIndex},
};
use nsh_core::platform::Os;
use std::{collections::BTreeMap, sync::Mutex};
use usher_shortcuts::{DomainId, ScopeId};

/// Shared across window services in one VM; module factories run only on cache misses.
pub struct ModuleLoader {
    index: ModuleIndex,
    cache: Mutex<BTreeMap<String, LoadedModule>>,
}
impl ModuleLoader {
    pub fn new(index: ModuleIndex) -> Self {
        Self {
            index,
            cache: Mutex::new(BTreeMap::new()),
        }
    }
    pub fn index(&self) -> &ModuleIndex {
        &self.index
    }
    /// Serial factory evaluation coalesces concurrent loads. Failures are never cached.
    /// The immutable index pins versions; replacing the index creates a fresh loader/cache.
    pub fn load(&self, id: &str, os: Os) -> Result<LoadedModule, Error> {
        let key = format!("{os:?}:{id}");
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| Error::InvalidConfig("loader_closed".into()))?;
        if let Some(module) = cache.get(&key) {
            return Ok(module.clone());
        }
        let module = self
            .index
            .load(id, os, DomainId::new("local")?, ScopeId::new(id)?)?;
        cache.insert(key, module.clone());
        Ok(module)
    }
}
