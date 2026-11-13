//! Replaceable, owned path matching contracts; no backend errors or borrows escape.
use crate::{
    Error,
    location::{decode, encode},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Segment {
    Literal(String),
    Parameter(String),
    CatchAll(String),
}
/// Validated portable grammar: absolute paths, whole-segment parameters and a
/// final catch-all. Suffix parameters, regexes and optional segments are rejected.
#[derive(Clone, Debug)]
pub struct Pattern {
    pub route: usize,
    pub path: String,
    pub segments: Vec<Segment>,
}
impl Pattern {
    pub(crate) fn parse(route: usize, path: &str) -> Result<Self, Error> {
        if !path.starts_with('/') || path.starts_with("//") || path.contains(['?', '#', '\\']) {
            return Err(Error::InvalidPattern);
        }
        let raw: Vec<_> = path[1..].split('/').collect();
        let mut names = BTreeSet::new();
        let mut segments = Vec::new();
        for (index, text) in raw.iter().enumerate() {
            let segment = if text.starts_with('{') && text.ends_with('}') {
                let inner = &text[1..text.len() - 1];
                let catch = inner.starts_with('*');
                let name = if catch { &inner[1..] } else { inner };
                if name.is_empty()
                    || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    || !names.insert(name.to_owned())
                    || (catch && index + 1 != raw.len())
                {
                    return Err(Error::InvalidPattern);
                }
                if catch {
                    Segment::CatchAll(name.into())
                } else {
                    Segment::Parameter(name.into())
                }
            } else {
                if text.contains(['{', '}']) || (text.is_empty() && index + 1 != raw.len()) {
                    return Err(Error::InvalidPattern);
                }
                let decoded = decode(text).map_err(|_| Error::InvalidPattern)?;
                if decoded.contains(['/', '\\'])
                    || matches!(decoded.as_str(), "." | "..")
                    || encode(&decoded) != *text
                {
                    return Err(Error::InvalidPattern);
                }
                Segment::Literal((*text).into())
            };
            segments.push(segment);
        }
        Ok(Self {
            route,
            path: path.into(),
            segments,
        })
    }
    pub(crate) fn shape(&self) -> Vec<(u8, String)> {
        self.segments
            .iter()
            .map(|segment| match segment {
                Segment::Literal(value) => (0, value.clone()),
                Segment::Parameter(_) => (1, String::new()),
                Segment::CatchAll(_) => (2, String::new()),
            })
            .collect()
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub route: usize,
    pub params: BTreeMap<String, String>,
}
/// Match canonical encoded paths. Returned parameters are still encoded; the
/// router validates and decodes them once. None means a legitimate unmatched path.
pub trait RouteMatcher: Send + Sync {
    fn recognize(&self, path: &str) -> Result<Option<Match>, Error>;
}
/// Build a complete matcher or fail without installing a partial catalog.
pub trait MatcherFactory {
    fn build(&self, patterns: &[Pattern]) -> Result<Box<dyn RouteMatcher>, Error>;
}

#[cfg(feature = "matcher-matchit")]
pub struct Matchit;
#[cfg(feature = "matcher-matchit")]
impl MatcherFactory for Matchit {
    fn build(&self, patterns: &[Pattern]) -> Result<Box<dyn RouteMatcher>, Error> {
        let mut router = matchit::Router::new();
        for pattern in patterns {
            router
                .insert(&pattern.path, pattern.route)
                .map_err(|_| Error::Conflict)?;
        }
        struct Adapter(matchit::Router<usize>);
        impl RouteMatcher for Adapter {
            fn recognize(&self, path: &str) -> Result<Option<Match>, Error> {
                Ok(self.0.at(path).ok().map(|found| Match {
                    route: *found.value,
                    params: found
                        .params
                        .iter()
                        .map(|(key, value)| (key.into(), value.into()))
                        .collect(),
                }))
            }
        }
        Ok(Box::new(Adapter(router)))
    }
}
