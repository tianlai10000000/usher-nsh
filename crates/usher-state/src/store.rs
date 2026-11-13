//! Unique publication ownership and backend-independent commit semantics.

use crate::{
    BackendFactory, BackendReader, BackendWriter, Snapshot, StoreError, StoreId, Subscription,
    Transition,
};
use std::sync::Arc;

/// Creates a store using an application identity and an injected backend.
/// No runtime, global store table or background thread is created.
pub fn create_store<S: Send + Sync + 'static>(
    id: StoreId,
    initial: S,
    factory: &dyn BackendFactory<S>,
) -> Result<(StoreWriter<S>, StoreReader<S>), StoreError> {
    let parts = factory.create(Snapshot::new(id, 0, initial))?;
    let reader = StoreReader {
        backend: parts.reader,
    };
    let writer = StoreWriter {
        id,
        sequence: 0,
        backend: Some(parts.writer),
        reader: reader.clone(),
    };
    Ok((writer, reader))
}

/// Read-only consumer capability; cloning never grants publication access.
///
/// ```compile_fail
/// use usher_state::StoreReader;
/// fn cannot_publish(reader: StoreReader<u8>) { reader.publish(1); }
/// ```
pub struct StoreReader<S> {
    backend: Arc<dyn BackendReader<S>>,
}

impl<S> Clone for StoreReader<S> {
    fn clone(&self) -> Self {
        Self {
            backend: Arc::clone(&self.backend),
        }
    }
}

impl<S: Send + Sync + 'static> StoreReader<S> {
    /// Reads the current value; this is not a subscription registration.
    pub fn snapshot(&self) -> Result<Snapshot<S>, StoreError> {
        self.backend.snapshot()
    }

    /// Registers atomically. Read `initial()` before awaiting subsequent changes.
    pub fn subscribe(&self) -> Result<Subscription<S>, StoreError> {
        let (initial, backend) = self.backend.subscribe()?;
        Ok(Subscription::new(initial, backend))
    }
}

/// The sole commit capability. `&mut self` serializes commits; applications with
/// multiple command sources must serialize access at their domain service.
/// Dropping the writer closes the store even while readers remain alive.
///
/// ```compile_fail
/// use usher_state::StoreWriter;
/// fn cannot_clone(writer: StoreWriter<u8>) { let second = writer.clone(); }
/// ```
pub struct StoreWriter<S> {
    id: StoreId,
    sequence: u64,
    backend: Option<Box<dyn BackendWriter<S>>>,
    reader: StoreReader<S>,
}

/// Successful publication result. Effects are returned, never executed here.
#[derive(Debug)]
pub struct Commit<S, E> {
    /// The authoritative snapshot after the decision, possibly unchanged.
    pub snapshot: Snapshot<S>,
    /// Domain work released only after the decision successfully commits.
    pub effects: Vec<E>,
}

impl<S: Send + Sync + 'static> StoreWriter<S> {
    /// Reads the authoritative committed state for a pure domain transition.
    pub fn snapshot(&self) -> Result<Snapshot<S>, StoreError> {
        self.reader.snapshot()
    }

    /// Publishes a state declared changed by the domain. Success is linearized
    /// at backend replacement; a failure does not advance the sequence.
    pub fn publish(&mut self, state: S) -> Result<Snapshot<S>, StoreError> {
        let backend = self.backend.as_mut().ok_or(StoreError::Closed)?;
        let sequence = self
            .sequence
            .checked_add(1)
            .ok_or(StoreError::SequenceExhausted)?;
        let snapshot = Snapshot::new(self.id, sequence, state);
        backend.publish(snapshot.clone())?;
        self.sequence = sequence;
        Ok(snapshot)
    }

    /// Commits a domain decision. Unchanged decisions do not publish or advance
    /// the sequence. Effects are released only on success, including unchanged
    /// decisions; callers must not execute candidate effects ahead of this call.
    pub fn apply<E>(&mut self, transition: Transition<S, E>) -> Result<Commit<S, E>, StoreError> {
        if self.backend.is_none() {
            return Err(StoreError::Closed);
        }
        let (snapshot, effects) = match transition {
            Transition::Unchanged { effects } => (self.snapshot()?, effects),
            Transition::Changed { state, effects } => (self.publish(state)?, effects),
        };
        Ok(Commit { snapshot, effects })
    }
}

impl<S> StoreWriter<S> {
    /// Closes once. Existing readers retain access to the last committed value.
    pub fn close(&mut self) {
        if let Some(mut backend) = self.backend.take() {
            backend.close();
        }
    }
}

impl<S> Drop for StoreWriter<S> {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BackendRegistration;

    struct FinalReader(Snapshot<()>);
    impl BackendReader<()> for FinalReader {
        fn snapshot(&self) -> Result<Snapshot<()>, StoreError> {
            Ok(self.0.clone())
        }
        fn subscribe(&self) -> Result<BackendRegistration<()>, StoreError> {
            Err(StoreError::BackendUnavailable)
        }
    }
    struct MustNotPublish;
    impl BackendWriter<()> for MustNotPublish {
        fn publish(&mut self, _: Snapshot<()>) -> Result<(), StoreError> {
            panic!("overflow must be rejected before reaching the backend")
        }
        fn close(&mut self) {}
    }

    #[test]
    fn sequence_exhaustion_is_atomic_but_unchanged_effects_remain_valid() {
        let id = StoreId::new(u128::MAX);
        let reader = StoreReader {
            backend: Arc::new(FinalReader(Snapshot::new(id, u64::MAX, ()))),
        };
        let mut writer = StoreWriter {
            id,
            sequence: u64::MAX,
            backend: Some(Box::new(MustNotPublish)),
            reader,
        };
        assert_eq!(
            writer.publish(()).unwrap_err(),
            StoreError::SequenceExhausted
        );
        assert_eq!(writer.snapshot().unwrap().sequence(), u64::MAX);
        let result = writer
            .apply(Transition::Unchanged { effects: vec![()] })
            .unwrap();
        assert_eq!(result.snapshot.sequence(), u64::MAX);
        assert_eq!(result.effects, vec![()]);
        writer.close();
        assert_eq!(writer.publish(()).unwrap_err(), StoreError::Closed);
    }
}
