//! State-specific test consumers, scenario fixtures and mock backend wiring.

pub mod memory;

use serde::Deserialize;
use usher_state::Transition;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixtures {
    pub schema_version: u32,
    pub store_ids: Vec<u128>,
    pub initial: i64,
    pub published: Vec<i64>,
    pub scenarios: Vec<Scenario>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub case_id: String,
    pub actions: Vec<Action>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub command: Command,
    pub state: i64,
    pub sequence: u64,
    pub effects: Vec<String>,
    pub rejected: bool,
}

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Command {
    Set(i64),
    Effect(String),
    Reject,
}

pub struct Machine;

impl Machine {
    pub fn transition(
        &self,
        current: &i64,
        command: Command,
    ) -> Result<Transition<i64, String>, &'static str> {
        match command {
            Command::Reject => Err("fixture_rejection"),
            Command::Effect(effect) => Ok(Transition::Unchanged {
                effects: vec![effect],
            }),
            Command::Set(state) if state != *current => Ok(Transition::Changed {
                state,
                effects: vec![],
            }),
            Command::Set(_) => Ok(Transition::Unchanged { effects: vec![] }),
        }
    }
}

pub fn fixtures() -> Fixtures {
    let data: Fixtures = serde_json::from_str(include_str!("../fixtures/contracts.json"))
        .expect("valid state fixtures");
    assert_eq!(data.schema_version, 1, "supported fixture schema");
    assert!(data.store_ids.len() > 1 && data.published.len() > 1);
    assert!(!data.scenarios.is_empty());
    usher_test_support::fixture::assert_unique_ids(
        data.scenarios.iter().map(|case| case.case_id.as_str()),
    );
    for scenario in &data.scenarios {
        assert!(!scenario.actions.is_empty(), "nonempty scenario");
    }
    assert_ne!(data.store_ids[0], data.store_ids[1]);
    data
}
