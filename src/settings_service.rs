//! Serializes settings transactions, checks application capacity and publishes immutable state.
mod error;
pub use error::Error;

use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use usher_state::{BackendFactory, StoreId, StoreReader, StoreWriter, create_store};

use crate::settings::{Command, SettingsStore, Snapshot};

/// Application command boundary for one settings repository or engine session.
/// The legacy store remains the sole mutable domain model. The state reader is
/// its immutable published projection; it never owns persistence or domain rules.
/// Disk-backed commands must run on a blocking worker, as the NSH bridge does.
pub struct SettingsService {
    updates: std::sync::Arc<crate::update_service::UpdateService>,
    transaction: Mutex<Transaction>,
    reader: StoreReader<Snapshot>,
    closed: AtomicBool,
}

struct Transaction {
    domain: SettingsStore,
    diagnostics: Option<usher_logging::runtime::Runtime>,
    writer: StoreWriter<Snapshot>,
}

// An unexpected fault after domain mutation cannot roll back successful I/O.
// Close and reject subsequent commands, including when unwinding a panic. A new
// application assembly must reload the authoritative repository before resuming.
struct Mutation<'a> {
    transaction: &'a mut Transaction,
    closed: &'a AtomicBool,
    updates: &'a crate::update_service::UpdateService,
    completed: bool,
}

impl Drop for Mutation<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.closed.store(true, Ordering::Release);
            self.transaction.writer.close();
            self.updates.close();
        }
    }
}

impl SettingsService {
    pub(crate) fn new(
        domain: SettingsStore,
        id: StoreId,
        factory: &dyn BackendFactory<Snapshot>,
    ) -> Result<Self, Error> {
        let (writer, reader) =
            create_store(id, domain.snapshot(), factory).map_err(Error::Store)?;
        let updates = std::sync::Arc::new(
            crate::update_service::UpdateService::new(&usher_state::backends::WatchBackend)
                .map_err(Error::Synchronization)?,
        );
        Ok(Self {
            updates,
            transaction: Mutex::new(Transaction {
                domain,
                writer,
                diagnostics: None,
            }),
            reader,
            closed: AtomicBool::new(false),
        })
    }

    /// Binds an output runtime to this settings scope and applies the authoritative preferences.
    pub fn attach_diagnostics(
        &self,
        runtime: usher_logging::runtime::Runtime,
    ) -> Result<Snapshot, Error> {
        let mut transaction = self.transaction.lock().map_err(|_| Error::Poisoned)?;
        self.ensure_open()?;
        let snapshot = transaction.domain.snapshot();
        runtime
            .logger()
            .set_level(snapshot.diagnostic_level.to_level());
        runtime.set_outputs(snapshot.diagnostic_outputs());
        transaction.diagnostics = Some(runtime);
        Ok(snapshot)
    }

    /// Installs the process pipeline while serialized with preference commits.
    pub(crate) fn start_diagnostics(&self, root: std::path::PathBuf) -> Result<Snapshot, Error> {
        let mut transaction = self.transaction.lock().map_err(|_| Error::Poisoned)?;
        self.ensure_open()?;
        let snapshot = transaction.domain.snapshot();
        let runtime = crate::diagnostics::install(
            root,
            snapshot.diagnostic_level.to_level(),
            snapshot.diagnostic_outputs(),
        );
        crate::diagnostics::started(&runtime);
        transaction.diagnostics = Some(runtime);
        Ok(snapshot)
    }

    /// Borrows the service created with this settings scope, before versions are known.
    pub fn updates(&self) -> Result<std::sync::Arc<crate::update_service::UpdateService>, Error> {
        self.ensure_open()?;
        Ok(self.updates.clone())
    }

    /// Serializes one-time version configuration against preference changes.
    pub(crate) fn configure_updates(
        &self,
        config: crate::update_service::transport::Configuration,
    ) -> Result<(), Error> {
        let transaction = self.transaction.lock().map_err(|_| Error::Poisoned)?;
        self.ensure_open()?;
        let preferences = transaction.domain.snapshot();
        let info = config.info().map_err(Error::Update)?;
        let config = config
            .settings(preferences.update_auto_check)
            .map_err(Error::Update)?;
        self.updates
            .configure_runtime(config, info)
            .map_err(Error::Update)
    }

    /// Returns a read-only capability. Reads and subscriptions never acquire the
    /// domain transaction lock, including while a disk write is in progress.
    /// After closure, existing readers retain the last published snapshot.
    pub fn reader(&self) -> StoreReader<Snapshot> {
        self.reader.clone()
    }

    /// Reads a versioned snapshot for transports without exposing a writer.
    pub fn state_snapshot(&self) -> Result<usher_state::Snapshot<Snapshot>, Error> {
        self.ensure_open()?;
        self.reader.snapshot().map_err(Error::Store)
    }

    /// Registers while the domain is live. Unlike a retained reader, a transport
    /// cannot establish a new binding to an already closed service.
    pub fn subscribe(&self) -> Result<usher_state::Subscription<Snapshot>, Error> {
        let _transaction = self.transaction.lock().map_err(|_| Error::Poisoned)?;
        self.ensure_open()?;
        self.reader.subscribe().map_err(Error::Store)
    }

    /// Returns the published preferences in the legacy response shape.
    pub fn snapshot(&self) -> Result<Snapshot, Error> {
        self.ensure_open()?;
        self.reader
            .snapshot()
            .map(|snapshot| snapshot.state().clone())
            .map_err(Error::Store)
    }

    /// Applies one validated preference command and persists it before publication.
    /// Write failures remain successful domain responses with saved=false; validation
    /// failures preserve their existing stable error code and do not publish.
    pub fn apply(&self, command: Command) -> Result<Snapshot, Error> {
        self.mutate(move |store| store.apply(command))
    }

    /// Retries current preferences without overwriting a concurrent preference with a stale read.
    pub(crate) fn retry_save(&self) -> Result<Snapshot, Error> {
        self.mutate(|store| {
            store.apply(Command::SetUpdateAutoCheck(
                store.snapshot().update_auto_check,
            ))
        })
    }

    /// Imports an image through the existing asset validation and persistence order.
    pub fn import_background_image(&self, name: String, bytes: Vec<u8>) -> Result<Snapshot, Error> {
        self.mutate(move |store| store.import_background_image(name, bytes))
    }

    /// Reads only the selected application-owned image through the original repository.
    pub fn load_background_image(&self, id: &str) -> Result<Vec<u8>, Error> {
        let transaction = self.transaction.lock().map_err(|_| Error::Poisoned)?;
        self.ensure_open()?;
        transaction
            .domain
            .load_background_image(id)
            .map_err(Error::Domain)
    }

    /// Refreshes and publishes the complete catalog only when its contents change.
    pub fn rescan_themes(&self) -> Result<Snapshot, Error> {
        self.mutate(|store| Ok(store.rescan_themes()))
    }

    /// Clones a style using the existing retry-safe multi-file operation.
    pub fn clone_style_theme(&self, id: String, name: String) -> Result<Snapshot, Error> {
        self.mutate(move |store| store.clone_style_theme(id, name))
    }

    /// Rejects future commands and ends subscriptions after any in-flight command.
    /// Final published values remain readable through previously borrowed readers.
    pub fn close(&self) {
        let mut transaction = self
            .transaction
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.closed.store(true, Ordering::Release);
        self.updates.close();
        transaction.writer.close();
    }

    fn ensure_open(&self) -> Result<(), Error> {
        if self.closed.load(Ordering::Acquire) {
            Err(Error::Closed)
        } else {
            Ok(())
        }
    }

    fn mutate(
        &self,
        operation: impl FnOnce(&mut SettingsStore) -> Result<Snapshot, &'static str>,
    ) -> Result<Snapshot, Error> {
        let mut transaction = self.transaction.lock().map_err(|_| Error::Poisoned)?;
        self.ensure_open()?;
        let mut mutation = Mutation {
            transaction: &mut transaction,
            closed: &self.closed,
            updates: &self.updates,
            completed: false,
        };
        // This service owns writer lifetime under the transaction lock. Check the
        // published sequence before legacy I/O; backend failures after I/O still close
        // the service. This is an application compatibility policy, not a store reservation.
        let before = mutation
            .transaction
            .writer
            .snapshot()
            .map_err(Error::Store)?;
        before
            .sequence()
            .checked_add(1)
            .ok_or(Error::Store(usher_state::StoreError::SequenceExhausted))?;
        before
            .state()
            .revision
            .checked_add(1)
            .ok_or(Error::RevisionExhausted)?;
        let result = operation(&mut mutation.transaction.domain);
        let candidate = match &result {
            Ok(snapshot) => snapshot.clone(),
            Err(_) => mutation.transaction.domain.snapshot(),
        };
        let committed = if &candidate == before.state() {
            before
        } else {
            mutation
                .transaction
                .writer
                .publish(candidate)
                .map_err(Error::Store)?
        };
        // The settings transaction orders preference writes and one-time update configuration.
        // Persisted settings remain authoritative, including unsaved in-memory preferences.
        if self
            .updates
            .snapshot()
            .map_err(Error::Synchronization)?
            .state()
            .domain
            .is_some()
        {
            self.updates
                .request(crate::update_service::Command::AutoCheck(
                    committed.state().update_auto_check,
                ))
                .map_err(Error::Synchronization)?;
        }
        if let Some(runtime) = &mutation.transaction.diagnostics {
            runtime.change_level(committed.state().diagnostic_level.to_level());
            let outputs = committed.state().diagnostic_outputs();
            if runtime.outputs() != outputs {
                runtime.set_outputs(outputs);
                crate::diagnostics::configured(runtime);
            }
        }
        mutation.completed = true;
        result
            .map(|_| committed.state().clone())
            .map_err(Error::Domain)
    }
}

impl Drop for SettingsService {
    fn drop(&mut self) {
        // A retained window handle must not keep an orphaned update scope writable.
        self.updates.close();
    }
}

#[cfg(test)]
mod tests;
