//! Backend-independent identities and normalized registration data.

macro_rules! identity {
    ($name:ident) => {
        #[doc = "Opaque caller-owned identity; empty values are rejected at construction."]
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);
        impl $name {
            /// Constructs an identity without interpreting its business meaning.
            pub fn new(value: impl Into<String>) -> Result<Self, Error> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(Error::EmptyIdentity);
                }
                Ok(Self(value))
            }
            /// Returns the caller-provided representation.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}
identity!(BindingId);
identity!(ScopeId);
identity!(OwnerToken);
identity!(DomainId);
identity!(CanonicalStroke);

/// Stable validation and lifecycle failures without backend or business types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    EmptyIdentity,
    DuplicateBinding(BindingId),
    UnknownScope(ScopeId),
    InvalidRelation,
    Conflict(BindingId, BindingId),
    UnknownBinding(BindingId),
    StaleRevision,
    InvalidReceipt,
    RetryExhausted,
    Disposed,
    SequenceExhausted,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

/// Focused-editor policy, interpreted by the backend rather than the catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditingPolicy {
    Auto,
    Always,
    NotEditing,
}

/// Registration behavior; no product defaults are embedded in the library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    pub editing: EditingPolicy,
    pub repeat: bool,
    pub pass_through: bool,
    pub required: bool,
}

/// A resource namespace and normalized key. Callers map backend collision domains here.
/// CanonicalStroke must be produced by the backend parser, including exact modifiers.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResourceKey {
    pub domain: DomainId,
    pub stroke: CanonicalStroke,
}

/// Immutable registration declaration. It deliberately contains no executable callback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingSpec {
    pub id: BindingId,
    pub scope: ScopeId,
    pub resource: ResourceKey,
    pub policy: Policy,
}

/// One enabled registration, tied to a caller-owned lifecycle and catalog revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesiredBinding {
    pub binding: BindingSpec,
    pub owner: OwnerToken,
    pub revision: u64,
}
