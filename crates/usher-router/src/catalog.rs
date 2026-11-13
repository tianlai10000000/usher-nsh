//! Atomic catalog validation, backend assembly and authoritative target resolution.
use crate::{
    Catalog, Error, Location, Target, UnknownQuery,
    location::{decode, encode, parse},
    matcher::{MatcherFactory, Pattern, RouteMatcher, Segment},
};
use std::collections::{BTreeMap, BTreeSet};

/// Immutable routing rules. History is owned by the caller, never hidden here.
pub struct Router {
    catalog: Catalog,
    patterns: Vec<Pattern>,
    matcher: Box<dyn RouteMatcher>,
}
impl Router {
    /// Validates the entire directory before exposing a matcher or initial state.
    pub fn new(catalog: Catalog, factory: &dyn MatcherFactory) -> Result<Self, Error> {
        if catalog.schema_version != 1 {
            return Err(Error::UnsupportedSchema);
        }
        let pages: BTreeSet<_> = catalog.pages.iter().collect();
        let ids: BTreeSet<_> = catalog.routes.iter().map(|route| &route.id).collect();
        if catalog.identity.is_empty()
            || catalog.redirect_limit == 0
            || catalog.routes.is_empty()
            || pages.len() != catalog.pages.len()
            || pages.iter().any(|page| page.is_empty())
            || ids.len() != catalog.routes.len()
            || !pages.contains(&catalog.not_found_page)
            || !ids.contains(&catalog.initial)
        {
            return Err(Error::InvalidCatalog);
        }
        let mut patterns = Vec::new();
        let mut shapes = BTreeSet::new();
        for (index, route) in catalog.routes.iter().enumerate() {
            if route.id.is_empty() || route.title_key.is_empty() || !pages.contains(&route.page) {
                return Err(Error::InvalidCatalog);
            }
            let pattern = Pattern::parse(index, &route.path)?;
            if !shapes.insert(pattern.shape()) {
                return Err(Error::Conflict);
            }
            let names: BTreeSet<_> = pattern
                .segments
                .iter()
                .filter_map(|s| match s {
                    Segment::Parameter(name) | Segment::CatchAll(name) => Some(name),
                    _ => None,
                })
                .collect();
            if names != route.params.keys().collect()
                || route
                    .params
                    .values()
                    .any(|p| !p.valid_schema() || p.multiple || !p.defaults.is_empty())
                || route
                    .query
                    .iter()
                    .any(|(name, p)| name.is_empty() || !p.valid_schema())
            {
                return Err(Error::InvalidCatalog);
            }
            let mut visited = BTreeSet::new();
            let mut current = Some(route.id.as_str());
            while let Some(id) = current {
                if !visited.insert(id) {
                    return Err(Error::InvalidCatalog);
                }
                current = catalog
                    .routes
                    .iter()
                    .find(|r| r.id == id)
                    .ok_or(Error::InvalidCatalog)?
                    .parent
                    .as_deref();
            }
            patterns.push(pattern);
        }
        let matcher = factory.build(&patterns)?;
        let router = Self {
            catalog,
            patterns,
            matcher,
        };
        router.resolve(&Target::Named {
            id: router.catalog.initial.clone(),
            params: BTreeMap::new(),
            query: BTreeMap::new(),
        })?;
        Ok(router)
    }
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }
    /// Resolves without changing state. A valid unmatched path returns NotFound;
    /// unknown names and invalid parameters return errors and leave callers untouched.
    pub fn resolve(&self, target: &Target) -> Result<Location, Error> {
        let (path, query) = match target {
            Target::Path { location } => parse(location)?,
            Target::Named { id, params, query } => {
                let index = self
                    .catalog
                    .routes
                    .iter()
                    .position(|r| r.id == *id)
                    .ok_or(Error::UnknownRoute)?;
                if params.keys().collect::<BTreeSet<_>>()
                    != self.catalog.routes[index].params.keys().collect()
                {
                    return Err(Error::InvalidParameter);
                }
                let mut segments = Vec::new();
                for segment in &self.patterns[index].segments {
                    segments.push(match segment {
                        Segment::Literal(value) => value.clone(),
                        Segment::Parameter(name) | Segment::CatchAll(name) => {
                            let value = &params[name];
                            if value.is_empty()
                                || value.contains('\\')
                                || value.chars().any(char::is_control)
                            {
                                return Err(Error::InvalidParameter);
                            }
                            if matches!(segment, Segment::Parameter(_)) && value.contains('/') {
                                return Err(Error::InvalidParameter);
                            }
                            let parts: Vec<_> = value.split('/').collect();
                            if parts
                                .iter()
                                .any(|part| part.is_empty() || matches!(*part, "." | ".."))
                            {
                                return Err(Error::InvalidParameter);
                            }
                            self.catalog.routes[index].params[name].validate(value)?;
                            parts.into_iter().map(encode).collect::<Vec<_>>().join("/")
                        }
                    });
                }
                (format!("/{}", segments.join("/")), query.clone())
            }
        };
        let matched = self.matcher.recognize(&path)?;
        let Some(matched) = matched else {
            if matches!(target, Target::Named { .. }) {
                return Err(Error::BackendUnavailable);
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
        let route = self
            .catalog
            .routes
            .get(matched.route)
            .ok_or(Error::BackendUnavailable)?;
        // A static route can shadow a named parameter value. Never silently resolve
        // a named request to a different page.
        if matches!(target, Target::Named { id, .. } if *id != route.id) {
            return Err(Error::InvalidTarget);
        }
        if matched.params.keys().collect::<BTreeSet<_>>() != route.params.keys().collect() {
            return Err(Error::BackendUnavailable);
        }
        let mut params = BTreeMap::new();
        for (name, value) in matched.params {
            let value = decode(&value)?;
            route.params[&name].validate(&value)?;
            params.insert(name, value);
        }
        let mut query = query;
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
        let mut ancestors = Vec::new();
        let mut current = Some(route);
        while let Some(route) = current {
            ancestors.push(route.id.clone());
            current = route
                .parent
                .as_ref()
                .and_then(|id| self.catalog.routes.iter().find(|r| r.id == *id));
        }
        ancestors.reverse();
        Ok(Location {
            id: Some(route.id.clone()),
            page: route.page.clone(),
            path,
            params,
            query,
            ancestors,
        })
    }
}
