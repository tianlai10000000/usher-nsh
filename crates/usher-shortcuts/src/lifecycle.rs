//! Serial registration ownership with explicit receipts, retry budgets and stale-event rejection.
use crate::{DesiredBinding, Error, ResourceKey};
use std::collections::BTreeMap;

/// Opaque runtime-local operation identity. Backends must return the exact issued ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ticket {
    sequence: u64,
    desired: DesiredBinding,
}
impl Ticket {
    /// Identifies the resource and declaration associated with this operation.
    pub fn desired(&self) -> &DesiredBinding {
        &self.desired
    }
}
/// Side effects are executed by an injected host, never by this library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Install(Ticket),
    Release(Ticket),
}
/// Backend-independent failure category; native diagnostics remain in the adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Rejected,
    Unsupported,
    Unavailable,
    Unknown,
}
/// An install timeout is not a failure receipt: it remains pending until settled.
/// Failed release means the backend may still own the key and blocks replacement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Receipt {
    Installed,
    InstallFailed(Failure),
    Released,
    ReleaseFailed(Failure),
}
/// Observable registration status without UI or business metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Installing,
    Ready,
    Releasing,
    Failed(Failure),
    ReleaseFailed(Failure),
}
#[derive(Clone, Debug)]
struct Slot {
    ticket: Ticket,
    status: Status,
    attempts: usize,
}
/// Read-only resource health. Desired may differ while an older operation settles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub ticket: Ticket,
    pub status: Status,
    pub desired: Option<DesiredBinding>,
    pub attempts: usize,
}
/// Single-owner coordinator. Use one instance for each shared backend ownership domain.
/// The caller serializes access and schedules explicit retries according to its clock policy.
#[derive(Clone, Debug)]
pub struct Runtime {
    desired: BTreeMap<ResourceKey, DesiredBinding>,
    slots: BTreeMap<ResourceKey, Slot>,
    sequence: u64,
    revision: Option<u64>,
    max_attempts: usize,
    disposed: bool,
}
impl Runtime {
    /// Supplies the total automatic install-attempt budget (including the first attempt).
    /// Zero disables installation and is rejected; retry delays are caller-owned.
    pub fn new(max_attempts: usize) -> Result<Self, Error> {
        if max_attempts == 0 {
            return Err(Error::RetryExhausted);
        }
        Ok(Self {
            desired: BTreeMap::new(),
            slots: BTreeMap::new(),
            sequence: 0,
            revision: None,
            max_attempts,
            disposed: false,
        })
    }
    /// Atomically validates/replaces a complete desired plan. Pending operations remain owned.
    /// Revision is monotonic; bindings must carry that same revision.
    pub fn reconcile(
        &mut self,
        revision: u64,
        plan: Vec<DesiredBinding>,
    ) -> Result<Vec<Effect>, Error> {
        if self.disposed {
            return Err(Error::Disposed);
        }
        if self.revision.is_some_and(|old| revision < old)
            || plan.iter().any(|d| d.revision != revision)
        {
            return Err(Error::StaleRevision);
        }
        let mut desired = BTreeMap::<ResourceKey, DesiredBinding>::new();
        let mut ids = std::collections::BTreeSet::new();
        for item in plan {
            if !ids.insert(item.binding.id.clone()) {
                return Err(Error::DuplicateBinding(item.binding.id));
            }
            if let Some(old) = desired.insert(item.binding.resource.clone(), item.clone()) {
                return Err(Error::Conflict(old.binding.id, item.binding.id));
            }
        }
        // Reserve enough sequence space before mutating the desired plan.
        self.sequence
            .checked_add(desired.len() as u64)
            .ok_or(Error::SequenceExhausted)?;
        self.desired = desired;
        self.revision = Some(revision);
        self.drive()
    }
    fn install(&mut self, desired: DesiredBinding, attempts: usize) -> Result<Effect, Error> {
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or(Error::SequenceExhausted)?;
        let ticket = Ticket {
            sequence: self.sequence,
            desired,
        };
        self.slots.insert(
            ticket.desired.binding.resource.clone(),
            Slot {
                ticket: ticket.clone(),
                status: Status::Installing,
                attempts,
            },
        );
        Ok(Effect::Install(ticket))
    }
    fn drive(&mut self) -> Result<Vec<Effect>, Error> {
        let mut effects = Vec::new();
        let keys: Vec<_> = self.slots.keys().cloned().collect();
        for key in keys {
            let slot = self.slots.get_mut(&key).expect("key came from this map");
            if self.desired.get(&key) != Some(&slot.ticket.desired) {
                match slot.status {
                    Status::Ready => {
                        slot.status = Status::Releasing;
                        effects.push(Effect::Release(slot.ticket.clone()));
                    }
                    Status::Failed(_) => {
                        self.slots.remove(&key);
                    }
                    _ => {}
                }
            }
        }
        let missing: Vec<_> = self
            .desired
            .iter()
            .filter(|(key, _)| !self.slots.contains_key(*key))
            .map(|(_, d)| d.clone())
            .collect();
        for item in missing {
            effects.push(self.install(item, 1)?);
        }
        Ok(effects)
    }
    /// Applies a matching receipt. Duplicate/obsolete receipts are harmless no-ops.
    /// A stale successful install is released before any replacement can be installed.
    pub fn acknowledge(&mut self, ticket: &Ticket, receipt: Receipt) -> Result<Vec<Effect>, Error> {
        let key = &ticket.desired.binding.resource;
        let Some(slot) = self.slots.get_mut(key) else {
            return Ok(Vec::new());
        };
        if slot.ticket != *ticket {
            return Ok(Vec::new());
        }
        match (&slot.status, receipt) {
            (Status::Installing, Receipt::Installed) => slot.status = Status::Ready,
            (Status::Installing, Receipt::InstallFailed(reason)) => {
                slot.status = Status::Failed(reason)
            }
            (Status::Releasing | Status::ReleaseFailed(_), Receipt::Released) => {
                self.slots.remove(key);
            }
            (Status::Releasing, Receipt::ReleaseFailed(reason)) => {
                slot.status = Status::ReleaseFailed(reason)
            }
            (Status::ReleaseFailed(_), Receipt::ReleaseFailed(_)) => return Ok(Vec::new()),
            // Repeated receipts cannot change an already advanced operation.
            (_, Receipt::Installed | Receipt::InstallFailed(_)) => return Ok(Vec::new()),
            (_, Receipt::Released | Receipt::ReleaseFailed(_)) => {
                return Err(Error::InvalidReceipt);
            }
        }
        self.drive()
    }
    /// Retries a failed operation within its install budget. Release retries never relinquish ownership.
    /// Callers must not retry an unresolved install, even after their local timeout.
    pub fn retry(&mut self, ticket: &Ticket) -> Result<Vec<Effect>, Error> {
        let key = &ticket.desired.binding.resource;
        let Some(slot) = self.slots.get(key) else {
            return Ok(Vec::new());
        };
        if slot.ticket != *ticket {
            return Ok(Vec::new());
        }
        match slot.status {
            Status::Failed(Failure::Unavailable | Failure::Unknown) if !self.disposed => {
                if slot.attempts >= self.max_attempts {
                    return Err(Error::RetryExhausted);
                }
                let desired = slot.ticket.desired.clone();
                let attempts = slot.attempts + 1;
                Ok(vec![self.install(desired, attempts)?])
            }
            Status::ReleaseFailed(_) => {
                let slot = self.slots.get_mut(key).expect("slot was checked above");
                slot.status = Status::Releasing;
                Ok(vec![Effect::Release(slot.ticket.clone())])
            }
            _ => Err(Error::InvalidReceipt),
        }
    }
    /// Revokes every trigger immediately and releases resources as pending operations settle.
    /// Idempotent; receipt handling must continue until cleanup is complete.
    pub fn dispose(&mut self) -> Result<Vec<Effect>, Error> {
        self.disposed = true;
        self.desired.clear();
        self.drive()
    }
    /// Validates an input ticket before the caller dispatches its own business action.
    pub fn accepts(&self, ticket: &Ticket) -> bool {
        !self.disposed
            && self.desired.get(&ticket.desired.binding.resource) == Some(&ticket.desired)
            && self
                .slots
                .get(&ticket.desired.binding.resource)
                .is_some_and(|s| s.ticket == *ticket && s.status == Status::Ready)
    }
    /// Returns current health, including pending obsolete resources awaiting cleanup.
    pub fn snapshot(&self) -> Vec<Snapshot> {
        self.slots
            .iter()
            .map(|(key, slot)| Snapshot {
                ticket: slot.ticket.clone(),
                status: slot.status.clone(),
                desired: self.desired.get(key).cloned(),
                attempts: slot.attempts,
            })
            .collect()
    }
    /// Confirms all resources have settled after disposal.
    pub fn is_disposed(&self) -> bool {
        self.disposed && self.slots.is_empty()
    }
}
