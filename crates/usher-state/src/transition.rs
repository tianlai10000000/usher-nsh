//! Domain decisions with explicit publication and effect semantics.

/// Whether a successful domain decision publishes a new snapshot.
#[derive(Debug, PartialEq, Eq)]
pub enum Transition<S, E> {
    /// No observable state change; effects may still be requested.
    Unchanged { effects: Vec<E> },
    /// The domain declares a change; no implicit equality check is performed.
    Changed { state: S, effects: Vec<E> },
}
