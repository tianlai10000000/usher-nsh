//! Pure domain transitions with explicit publication and effect decisions.

/// A domain rule set. Implementations must not perform I/O or mutate `current`.
pub trait StateMachine {
    /// Immutable domain state.
    type State;
    /// Typed command accepted by the domain.
    type Command;
    /// Work to execute by the application after a successful commit.
    type Effect;
    /// Domain rejection, distinct from store/backend failures.
    type Error;

    /// Computes a candidate without committing it or executing its effects.
    fn transition(
        &self,
        current: &Self::State,
        command: Self::Command,
    ) -> Result<Transition<Self::State, Self::Effect>, Self::Error>;
}

/// Whether a successful domain decision publishes a new snapshot.
#[derive(Debug, PartialEq, Eq)]
pub enum Transition<S, E> {
    /// No observable state change; effects may still be requested.
    Unchanged { effects: Vec<E> },
    /// The domain declares a change; no implicit equality check is performed.
    Changed { state: S, effects: Vec<E> },
}
