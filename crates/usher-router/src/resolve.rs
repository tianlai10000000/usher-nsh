//! Target formatting, matcher validation and location construction for an immutable router.
use crate::location::{Query, decode, encode, parse};
use crate::matcher::{Match, Segment};
use crate::{Error, Location, Route, Router, Target, UnknownQuery};
use std::collections::{BTreeMap, BTreeSet};

impl Router {
    /// Resolves without changing state. Valid unmatched paths use the NotFound page.
    /// Named targets are rematched to reject static routes shadowing parameter values.
    pub fn resolve(&self, target: &Target) -> Result<Location, Error> {
        let (path, query) = self.target_path(target)?;
        let Some(matched) = self.matcher.recognize(&path)? else {
            if matches!(target, Target::Named { .. }) {
                return Err(Error::InvalidMatch);
            }
            return Ok(Location {
                id: None,
                page: self.catalog.not_found_page.clone(),
                path,
                params: BTreeMap::new(),
                query,
                ancestors: Vec::new(),
            });
        };
        let route = self.matched_route(target, &matched)?;
        let params = matched
            .params
            .iter()
            .map(|(name, value)| {
                let value = decode(value)?;
                route.params[name].validate(&value)?;
                Ok((name.clone(), value))
            })
            .collect::<Result<_, Error>>()?;
        Ok(Location {
            id: Some(route.id.clone()),
            page: route.page.clone(),
            path,
            params,
            query: validate_query(route, query)?,
            ancestors: self.ancestors[matched.route].clone(),
        })
    }

    fn matched_route(&self, target: &Target, matched: &Match) -> Result<&Route, Error> {
        let route = self
            .catalog
            .routes
            .get(matched.route)
            .ok_or(Error::InvalidMatch)?;
        if matches!(target, Target::Named { id, .. } if *id != route.id) {
            return Err(Error::InvalidTarget);
        }
        if matched.params.keys().collect::<BTreeSet<_>>() != route.params.keys().collect() {
            return Err(Error::InvalidMatch);
        }
        Ok(route)
    }

    fn target_path(&self, target: &Target) -> Result<(String, Query), Error> {
        match target {
            Target::Path { location } => parse(location),
            Target::Named { id, params, query } => {
                let path = self.named_path(id, params)?;
                Ok((path, query.clone()))
            }
        }
    }
    fn named_path(&self, id: &str, params: &BTreeMap<String, String>) -> Result<String, Error> {
        let index = *self.indices.get(id).ok_or(Error::UnknownRoute)?;
        let route = &self.catalog.routes[index];
        if params.keys().collect::<BTreeSet<_>>() != route.params.keys().collect() {
            return Err(Error::InvalidParameter);
        }
        let segments = self.patterns[index]
            .segments
            .iter()
            .map(|segment| match segment {
                Segment::Literal(value) => Ok(value.clone()),
                Segment::Parameter(name) | Segment::CatchAll(name) => {
                    let value = &params[name];
                    let encoded = encode_parameter(value, matches!(segment, Segment::CatchAll(_)))?;
                    route.params[name].validate(value)?;
                    Ok(encoded)
                }
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(format!("/{}", segments.join("/")))
    }
}

fn validate_query(route: &Route, mut query: Query) -> Result<Query, Error> {
    if query.iter().any(|(key, values)| {
        key.is_empty()
            || values.is_empty()
            || key.chars().any(char::is_control)
            || values.iter().any(|v| v.chars().any(char::is_control))
    }) {
        return Err(Error::InvalidQuery);
    }
    if route.unknown_query == UnknownQuery::Reject
        && query.keys().any(|key| !route.query.contains_key(key))
    {
        return Err(Error::InvalidQuery);
    }
    for (name, rule) in &route.query {
        if !query.contains_key(name) && !rule.defaults.is_empty() {
            query.insert(name.clone(), rule.defaults.clone());
        }
        let values = query.get(name).map(Vec::as_slice).unwrap_or_default();
        if (rule.required && values.is_empty())
            || (!rule.multiple && values.len() > 1)
            || values.iter().any(|v| rule.validate(v).is_err())
        {
            return Err(Error::InvalidQuery);
        }
    }
    Ok(query)
}

fn encode_parameter(value: &str, catch_all: bool) -> Result<String, Error> {
    if value.is_empty() || value.contains('\\') || value.chars().any(char::is_control) {
        return Err(Error::InvalidParameter);
    }
    if !catch_all && value.contains('/') {
        return Err(Error::InvalidParameter);
    }
    let parts: Vec<_> = value.split('/').collect();
    if parts
        .iter()
        .any(|part| part.is_empty() || matches!(*part, "." | ".."))
    {
        return Err(Error::InvalidParameter);
    }
    Ok(parts.into_iter().map(encode).collect::<Vec<_>>().join("/"))
}
