//! Shares update services by the host-owned settings scope without creating a persistence format.
use super::{Error, UpdateService};
use crate::app_state::SettingsService;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use usher_state::StoreId;

/// Application-owned registry. Keep one alongside SettingsRegistry for the application lifetime.
#[derive(Default)]
pub struct UpdateRegistry {
    scopes: Mutex<Scopes>,
}
#[derive(Default)]
struct Scopes {
    entries: HashMap<StoreId, Arc<UpdateService>>,
    closed: bool,
}
impl UpdateRegistry {
    /// Borrows the same service across windows sharing settings. The service is created here, not
    /// yet configured: the machine arrives with `Command::Configure`, once the controller knows
    /// the installed shell's version, and a second configuration is refused by the service.
    /// Neither the directory nor the identity can be supplied by an untrusted UI command.
    pub fn for_settings(&self, settings: &SettingsService) -> Result<Arc<UpdateService>, Error> {
        let id = settings
            .state_snapshot()
            .map_err(|_| Error::Closed)?
            .store_id();
        let mut scopes = self.scopes.lock().map_err(|_| Error::Unavailable)?;
        if scopes.closed {
            return Err(Error::Closed);
        }
        if let Some(service) = scopes.entries.get(&id) {
            service.snapshot()?;
            return Ok(service.clone());
        }
        let service = settings.updates().map_err(|_| Error::Closed)?;
        scopes.entries.insert(id, service.clone());
        Ok(service)
    }
    /// Explicit session/process teardown closes even services retained by consumers.
    pub fn close(&self) {
        let mut scopes = self
            .scopes
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        scopes.closed = true;
        for service in scopes.entries.values() {
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
