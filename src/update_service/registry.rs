//! Shares update services by the host-owned settings scope without creating a persistence format.
use super::{Error, UpdateService};
use crate::app_state::SettingsService;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use usher_state::{StoreId, backends::WatchBackend};
use usher_update::state::Machine;

type Entry = (Machine, Arc<UpdateService>);
/// Application-owned registry. Keep one alongside SettingsRegistry for the application lifetime.
#[derive(Default)]
pub struct UpdateRegistry {
    scopes: Mutex<Scopes>,
}
#[derive(Default)]
struct Scopes {
    entries: HashMap<StoreId, Entry>,
    closed: bool,
}
impl UpdateRegistry {
    /// Borrows the same domain across windows sharing settings; contradictory initialization fails.
    /// Neither the directory nor the identity can be supplied by an untrusted UI command.
    pub fn for_settings(
        &self,
        settings: &SettingsService,
        initial: Machine,
    ) -> Result<Arc<UpdateService>, Error> {
        let id = settings
            .state_snapshot()
            .map_err(|_| Error::Closed)?
            .store_id();
        let mut scopes = self.scopes.lock().map_err(|_| Error::Unavailable)?;
        if scopes.closed {
            return Err(Error::Closed);
        }
        if let Some((original, service)) = scopes.entries.get(&id) {
            if original != &initial {
                return Err(Error::Configuration);
            }
            service.snapshot()?;
            return Ok(service.clone());
        }
        let service = Arc::new(UpdateService::new(initial.clone(), &WatchBackend)?);
        scopes.entries.insert(id, (initial, service.clone()));
        Ok(service)
    }
    /// Explicit session/process teardown closes even services retained by consumers.
    pub fn close(&self) {
        let mut scopes = self
            .scopes
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        scopes.closed = true;
        for (_, service) in scopes.entries.values() {
            service.close();
        }
        scopes.entries.clear();
    }
}
impl Drop for UpdateRegistry {
    fn drop(&mut self) {
        self.close();
    }
}
