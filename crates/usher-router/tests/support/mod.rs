//! Router-owned fixtures and a deterministic linear matcher for the portable grammar.
#![allow(dead_code)]
use std::collections::BTreeMap;
use usher_router::{
    Catalog, Error,
    matcher::{Match, MatcherFactory, Pattern, RouteMatcher, Segment},
};

pub fn catalog() -> Catalog {
    serde_json::from_str(include_str!("../fixtures/catalog.json")).unwrap()
}
#[derive(serde::Deserialize)]
pub struct Cases {
    pub schema_version: u32,
    pub property_cases: u32,
    pub property_seed: u64,
    pub cases: Vec<Case>,
}
#[derive(serde::Deserialize)]
pub struct Case {
    pub case_id: String,
    pub target: String,
    pub route: Option<String>,
    pub url: String,
}
pub fn cases() -> Cases {
    let data: Cases = serde_json::from_str(include_str!("../fixtures/cases.json")).unwrap();
    assert_eq!(data.schema_version, 1);
    usher_test_support::fixture::assert_unique_ids(
        data.cases.iter().map(|case| case.case_id.as_str()),
    );
    data
}
pub struct Reference;
impl MatcherFactory for Reference {
    fn build(&self, patterns: &[Pattern]) -> Result<Box<dyn RouteMatcher>, Error> {
        struct Linear(Vec<Pattern>);
        impl RouteMatcher for Linear {
            fn recognize(&self, path: &str) -> Result<Option<Match>, Error> {
                let parts: Vec<_> = path[1..].split('/').collect();
                let mut candidates = Vec::new();
                for pattern in &self.0 {
                    let mut params = BTreeMap::new();
                    let mut priority = Vec::new();
                    let mut consumed = 0;
                    let mut valid = true;
                    for segment in &pattern.segments {
                        let Some(part) = parts.get(consumed) else {
                            valid = false;
                            break;
                        };
                        match segment {
                            Segment::Literal(value) if value == part => priority.push(3),
                            Segment::Parameter(name) if !part.is_empty() => {
                                priority.push(2);
                                params.insert(name.clone(), (*part).into());
                            }
                            Segment::CatchAll(name) if !parts[consumed..].join("/").is_empty() => {
                                priority.push(1);
                                params.insert(name.clone(), parts[consumed..].join("/"));
                                consumed = parts.len();
                                break;
                            }
                            _ => {
                                valid = false;
                                break;
                            }
                        }
                        consumed += 1;
                    }
                    if valid && consumed == parts.len() {
                        candidates.push((
                            priority,
                            Match {
                                route: pattern.route,
                                params,
                            },
                        ));
                    }
                }
                candidates.sort_by(|a, b| b.0.cmp(&a.0));
                Ok(candidates.into_iter().next().map(|(_, found)| found))
            }
        }
        Ok(Box::new(Linear(patterns.to_vec())))
    }
}
