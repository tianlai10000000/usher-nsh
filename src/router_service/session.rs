//! Window-owned page data and operation tickets; executable effects stay in adapters.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Config {
    sections: Vec<String>,
    initial_section: String,
    initial_clicks: u32,
}
fn config() -> Config {
    serde_json::from_str(include_str!("../../ui/config/session.json"))
        .expect("bundled session configuration")
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub section: String,
    pub clicks: u32,
    pub opacity_preview: Option<u32>,
    pub transparency_preview: Option<u32>,
    #[serde(skip)]
    opacity_ticket: u64,
    #[serde(skip)]
    transparency_ticket: u64,
    pub background_problem: Option<String>,
    pub operations: BTreeMap<OperationKind, Operation>,
}
impl Default for Session {
    fn default() -> Self {
        let config = config();
        Self {
            section: config.initial_section,
            clicks: config.initial_clicks,
            opacity_preview: None,
            transparency_preview: None,
            opacity_ticket: 0,
            transparency_ticket: 0,
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
    pub ticket: String,
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
        expected: Option<String>,
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
        ticket: String,
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
    /// Returns an operation ticket for begin, or an empty receipt for ordinary changes.
    pub(super) fn apply(
        &mut self,
        command: SessionCommand,
        next_ticket: &mut u64,
    ) -> Result<String, &'static str> {
        match command {
            SessionCommand::Section { value } => {
                if !config().sections.contains(&value) {
                    return Err("session_invalid_section");
                }
                self.section = value;
            }
            SessionCommand::Increment => {
                self.clicks = self
                    .clicks
                    .checked_add(1)
                    .ok_or("session_capacity_exhausted")?
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
                if expected.is_some_and(|expected| expected != current_ticket.to_string()) {
                    return Ok("superseded".into());
                }
                let range = if opacity {
                    (
                        crate::background::CONFIG.min_opacity,
                        crate::background::CONFIG.max_opacity,
                    )
                } else {
                    (
                        crate::background::CONFIG.content_transparency.min,
                        crate::background::CONFIG.content_transparency.max,
                    )
                };
                if value.is_some_and(|v| v < range.0 || v > range.1) {
                    return Err("session_invalid_preview");
                }
                if value.is_some() {
                    *next_ticket = next_ticket
                        .checked_add(1)
                        .ok_or("session_capacity_exhausted")?;
                    if opacity {
                        self.opacity_ticket = *next_ticket;
                    } else {
                        self.transparency_ticket = *next_ticket;
                    }
                }
                if opacity {
                    self.opacity_preview = value;
                } else {
                    self.transparency_preview = value;
                }
                return Ok(if value.is_some() {
                    next_ticket.to_string()
                } else {
                    String::new()
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
                    return Err("operation_in_progress");
                }
                *next_ticket = next_ticket
                    .checked_add(1)
                    .ok_or("session_capacity_exhausted")?;
                let ticket = next_ticket.to_string();
                self.operations.insert(
                    operation,
                    Operation {
                        ticket: ticket.clone(),
                        stage,
                        active: true,
                        failed: false,
                        error: None,
                        value: None,
                    },
                );
                return Ok(ticket);
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
                    .filter(|current| current.active && current.ticket == ticket)
                else {
                    return Ok("superseded".into());
                };
                *current = Operation {
                    ticket,
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
                            ticket: String::new(),
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
        Ok(String::new())
    }
}
