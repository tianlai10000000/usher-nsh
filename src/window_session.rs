//! Window-owned page data and operation tickets; executable effects stay in adapters.
mod ticket;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::LazyLock};
pub use ticket::Ticket;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Config {
    sections: Vec<String>,
    initial_section: String,
    initial_clicks: u32,
}
static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../ui/config/session.json"))
        .expect("bundled session configuration")
});
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    #[serde(skip)]
    next_ticket: u64,
    pub section: String,
    pub clicks: u32,
    pub opacity_preview: Option<u32>,
    pub transparency_preview: Option<u32>,
    #[serde(skip)]
    opacity_ticket: Ticket,
    #[serde(skip)]
    transparency_ticket: Ticket,
    pub background_problem: Option<String>,
    pub operations: BTreeMap<OperationKind, Operation>,
}
impl Default for Session {
    fn default() -> Self {
        let config = &*CONFIG;
        Self {
            next_ticket: 0,
            section: config.initial_section.clone(),
            clicks: config.initial_clicks,
            opacity_preview: None,
            transparency_preview: None,
            opacity_ticket: Ticket::default(),
            transparency_ticket: Ticket::default(),
            background_problem: None,
            operations: BTreeMap::new(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    Settings,
    Directory,
    Picker,
    Export,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Stage {
    Idle,
    Loading,
    Saving,
    Scanning,
    Saved,
    LoadError,
    SaveError,
    ScanError,
    Invalid,
    Recovered,
    ReadOnly,
    Running,
    Done,
    Error,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    #[serde(serialize_with = "serialize_operation_ticket")]
    pub ticket: Option<Ticket>,
    pub stage: Stage,
    pub active: bool,
    pub failed: bool,
    pub error: Option<String>,
    pub value: Option<String>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SessionCommand {
    Section {
        value: String,
    },
    Increment,
    Preview {
        opacity: bool,
        value: Option<u32>,
        #[serde(default)]
        expected: Option<Ticket>,
    },
    BackgroundProblem {
        value: Option<String>,
    },
    Begin {
        operation: OperationKind,
        stage: Stage,
    },
    Finish {
        operation: OperationKind,
        ticket: Ticket,
        stage: Stage,
        error: Option<String>,
        value: Option<String>,
        failed: bool,
    },
    ObserveSettings {
        stage: Stage,
    },
}
impl Session {
    /// Applies a candidate session command; the window service publishes it atomically.
    pub(crate) fn apply(&mut self, command: SessionCommand) -> Result<Receipt, Error> {
        match command {
            SessionCommand::Section { value } => {
                if !CONFIG.sections.contains(&value) {
                    return Err(Error::InvalidSection);
                }
                self.section = value;
            }
            SessionCommand::Increment => {
                self.clicks = self.clicks.checked_add(1).ok_or(Error::CapacityExhausted)?
            }
            SessionCommand::Preview {
                opacity,
                value,
                expected,
            } => {
                let current_ticket = if opacity {
                    self.opacity_ticket
                } else {
                    self.transparency_ticket
                };
                if expected.is_some_and(|expected| expected != current_ticket) {
                    return Ok(Receipt::Superseded);
                }
                if let Some(value) = value {
                    let validate = if opacity {
                        crate::background::validate_opacity
                    } else {
                        crate::background::validate_content_transparency
                    };
                    validate(value).map_err(|_| Error::InvalidPreview)?;
                }
                if value.is_some() {
                    self.next_ticket = self
                        .next_ticket
                        .checked_add(1)
                        .ok_or(Error::CapacityExhausted)?;
                    if opacity {
                        self.opacity_ticket = Ticket::new(self.next_ticket);
                    } else {
                        self.transparency_ticket = Ticket::new(self.next_ticket);
                    }
                }
                if opacity {
                    self.opacity_preview = value;
                } else {
                    self.transparency_preview = value;
                }
                return Ok(if value.is_some() {
                    Receipt::Ticket(Ticket::new(self.next_ticket))
                } else {
                    Receipt::Applied
                });
            }
            SessionCommand::BackgroundProblem { value } => self.background_problem = value,
            SessionCommand::Begin { operation, stage } => {
                if operation == OperationKind::Export
                    && self
                        .operations
                        .get(&operation)
                        .is_some_and(|current| current.active)
                {
                    return Err(Error::OperationInProgress);
                }
                self.next_ticket = self
                    .next_ticket
                    .checked_add(1)
                    .ok_or(Error::CapacityExhausted)?;
                let ticket = Ticket::new(self.next_ticket);
                self.operations.insert(
                    operation,
                    Operation {
                        ticket: Some(ticket),
                        stage,
                        active: true,
                        failed: false,
                        error: None,
                        value: None,
                    },
                );
                return Ok(Receipt::Ticket(ticket));
            }
            SessionCommand::Finish {
                operation,
                ticket,
                stage,
                error,
                value,
                failed,
            } => {
                let Some(current) = self
                    .operations
                    .get_mut(&operation)
                    .filter(|current| current.active && current.ticket == Some(ticket))
                else {
                    return Ok(Receipt::Superseded);
                };
                *current = Operation {
                    ticket: Some(ticket),
                    stage,
                    active: false,
                    failed,
                    error,
                    value,
                };
            }
            SessionCommand::ObserveSettings { stage } => {
                if let Some(operation) = self.operations.get_mut(&OperationKind::Settings) {
                    if !operation.active && !operation.failed {
                        operation.stage = stage;
                    }
                } else {
                    self.operations.insert(
                        OperationKind::Settings,
                        Operation {
                            ticket: None,
                            stage,
                            active: false,
                            failed: false,
                            error: None,
                            value: None,
                        },
                    );
                }
            }
        }
        Ok(Receipt::Applied)
    }
}

/// Typed command outcome; wire compatibility is handled only by the bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "ticket", rename_all = "snake_case")]
pub enum Receipt {
    /// The command completed, including an idempotent no-op.
    Applied,
    /// A newer operation owns the state; this command changed nothing.
    Superseded,
    /// A new operation or preview owns this ticket.
    Ticket(Ticket),
}

/// Window session validation and capacity failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidSection,
    InvalidPreview,
    InvalidTicket,
    CapacityExhausted,
    OperationInProgress,
}
impl Error {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidSection => "session_invalid_section",
            Self::InvalidPreview => "session_invalid_preview",
            Self::InvalidTicket => "session_invalid_command",
            Self::CapacityExhausted => "session_capacity_exhausted",
            Self::OperationInProgress => "operation_in_progress",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

// Preserve the legacy empty ticket for observations that never started an operation.
fn serialize_operation_ticket<S: serde::Serializer>(
    ticket: &Option<Ticket>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    ticket
        .map(String::from)
        .unwrap_or_default()
        .serialize(serializer)
}
