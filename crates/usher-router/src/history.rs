//! Pure history transitions. The application commits one returned state atomically.
use crate::{Error, Location, Router, Target};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub key: String,
    pub location: Location,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    entries: Vec<Entry>,
    cursor: usize,
    #[serde(skip)]
    next_key: u64,
    #[serde(skip)]
    revision: u64,
    catalog_identity: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "target", rename_all = "snake_case")]
pub enum NavigationCommand {
    Push(Target),
    Replace(Target),
    Go(i64),
}
/// Cancellation/supersession/failure are produced by the application guard layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavigationResult {
    Committed,
    Duplicated,
    Cancelled,
    Superseded,
    NotFound,
    InvalidTarget,
    Failed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub state: History,
    pub result: NavigationResult,
}
impl History {
    pub fn current(&self) -> &Entry {
        &self.entries[self.cursor]
    }
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    pub fn cursor(&self) -> usize {
        self.cursor
    }
    pub fn catalog_identity(&self) -> &str {
        &self.catalog_identity
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn can_go_back(&self) -> bool {
        self.cursor > 0
    }
    pub fn can_go_forward(&self) -> bool {
        self.cursor + 1 < self.entries.len()
    }
}
impl Router {
    pub fn initial(&self) -> Result<History, Error> {
        let location = self.resolve(&Target::Named {
            id: self.catalog().initial.clone(),
            params: Default::default(),
            query: Default::default(),
        })?;
        Ok(History {
            entries: vec![Entry {
                key: 0.to_string(),
                location,
            }],
            cursor: 0,
            next_key: 1,
            revision: 0,
            catalog_identity: self.catalog().identity.clone(),
        })
    }
    /// Resolves retained URLs against a replacement catalog without changing entry keys
    /// or the cursor. Removed routes and tightened schemas become NotFound entries;
    /// infrastructure failures abort the whole migration and leave the input untouched.
    pub fn migrate_history(&self, current: &History) -> Result<History, Error> {
        let mut next = current.clone();
        for entry in &mut next.entries {
            entry.location = match self.resolve(&Target::path(entry.location.url())) {
                Ok(location) => location,
                Err(Error::InvalidParameter | Error::InvalidQuery) => Location {
                    id: None,
                    page: self.catalog().not_found_page.clone(),
                    path: entry.location.path.clone(),
                    params: Default::default(),
                    query: entry.location.query.clone(),
                    ancestors: Vec::new(),
                },
                Err(error) => return Err(error),
            };
        }
        next.catalog_identity = self.catalog().identity.clone();
        next.revision = next
            .revision
            .checked_add(1)
            .ok_or(Error::CapacityExhausted)?;
        Ok(next)
    }
    /// A failed transition leaves its input untouched. Callers must commit against
    /// the same source revision after asynchronous guards, never a newer history.
    pub fn transition(
        &self,
        current: &History,
        command: &NavigationCommand,
    ) -> Result<Outcome, Error> {
        if current.catalog_identity != self.catalog().identity {
            return Err(Error::InvalidHistory);
        }
        let mut state = current.clone();
        let location = match command {
            NavigationCommand::Go(delta) => {
                let cursor = i128::try_from(state.cursor).map_err(|_| Error::CapacityExhausted)?
                    + i128::from(*delta);
                if cursor < 0
                    || cursor >= state.entries.len() as i128
                    || cursor == state.cursor as i128
                {
                    return Ok(Outcome {
                        state,
                        result: NavigationResult::Duplicated,
                    });
                }
                state.cursor = cursor as usize;
                None
            }
            NavigationCommand::Push(target) | NavigationCommand::Replace(target) => {
                Some(self.resolve(target)?)
            }
        };
        if let Some(location) = location {
            if location == state.current().location {
                return Ok(Outcome {
                    state,
                    result: NavigationResult::Duplicated,
                });
            }
            let next_key = state
                .next_key
                .checked_add(1)
                .ok_or(Error::CapacityExhausted)?;
            let entry = Entry {
                key: state.next_key.to_string(),
                location,
            };
            state.next_key = next_key;
            match command {
                NavigationCommand::Push(_) => {
                    state.entries.truncate(state.cursor + 1);
                    state.entries.push(entry);
                    state.cursor += 1;
                }
                NavigationCommand::Replace(_) => state.entries[state.cursor] = entry,
                NavigationCommand::Go(_) => unreachable!("go does not construct a new location"),
            }
        }
        state.revision = state
            .revision
            .checked_add(1)
            .ok_or(Error::CapacityExhausted)?;
        let result = if state.current().location.id.is_none() {
            NavigationResult::NotFound
        } else {
            NavigationResult::Committed
        };
        Ok(Outcome { state, result })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matcher::{Match, MatcherFactory, Pattern, RouteMatcher};
    struct Static(Vec<Pattern>);
    impl MatcherFactory for Static {
        fn build(&self, patterns: &[Pattern]) -> Result<Box<dyn RouteMatcher>, Error> {
            Ok(Box::new(Static(patterns.to_vec())))
        }
    }
    impl RouteMatcher for Static {
        fn recognize(&self, path: &str) -> Result<Option<Match>, Error> {
            Ok(self.0.iter().find(|p| p.path == path).map(|p| Match {
                route: p.route,
                params: Default::default(),
            }))
        }
    }
    #[test]
    fn exhausted_counters_reject_without_mutating_history() {
        let catalog = serde_json::from_str(include_str!("../tests/fixtures/catalog.json")).unwrap();
        let router = Router::new(catalog, &Static(Vec::new())).unwrap();
        for exhausted_key in [false, true] {
            let mut history = router.initial().unwrap();
            if exhausted_key {
                history.next_key = u64::MAX;
            } else {
                history.revision = u64::MAX;
                assert_eq!(
                    router.migrate_history(&history),
                    Err(Error::CapacityExhausted)
                );
            }
            let before = history.clone();
            assert_eq!(
                router.transition(&history, &NavigationCommand::Push(Target::path("/about"))),
                Err(Error::CapacityExhausted)
            );
            assert_eq!(history, before);
            assert_eq!(
                router
                    .transition(&history, &NavigationCommand::Go(-1))
                    .unwrap()
                    .state,
                before
            );
        }
    }
}
