//! Validates immutable catalogs and builds ID indexes and cached ancestor chains.
use crate::matcher::{MatcherFactory, Pattern, RouteMatcher, Segment};
use crate::{Catalog, Error, Route, Target};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Immutable routing rules. History is owned by the caller, never hidden here.
pub struct Router {
    pub(crate) catalog: Catalog,
    pub(crate) patterns: Vec<Pattern>,
    pub(crate) matcher: Box<dyn RouteMatcher>,
    pub(crate) indices: HashMap<String, usize>,
    pub(crate) ancestors: Vec<Vec<String>>,
}
impl Router {
    /// Validates the complete catalog before exposing a matcher or initial state.
    pub fn new(catalog: Catalog, factory: &dyn MatcherFactory) -> Result<Self, Error> {
        validate_catalog(&catalog)?;
        let indices = route_indices(&catalog)?;
        let patterns = route_patterns(&catalog)?;
        let ancestors = ancestor_chains(&catalog, &indices)?;
        let matcher = factory.build(&patterns)?;
        let router = Self {
            catalog,
            patterns,
            matcher,
            indices,
            ancestors,
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
}

fn validate_catalog(catalog: &Catalog) -> Result<(), Error> {
    if catalog.schema_version != 1 {
        return Err(Error::UnsupportedSchema);
    }
    if catalog.identity.is_empty() || catalog.redirect_limit == 0 || catalog.routes.is_empty() {
        return Err(Error::InvalidCatalog);
    }
    let pages: BTreeSet<_> = catalog.pages.iter().collect();
    if pages.len() != catalog.pages.len() || pages.iter().any(|page| page.is_empty()) {
        return Err(Error::InvalidCatalog);
    }
    if !pages.contains(&catalog.not_found_page) {
        return Err(Error::InvalidCatalog);
    }
    for route in &catalog.routes {
        if route.id.is_empty() || route.title_key.is_empty() || !pages.contains(&route.page) {
            return Err(Error::InvalidCatalog);
        }
    }
    Ok(())
}

fn route_indices(catalog: &Catalog) -> Result<HashMap<String, usize>, Error> {
    let mut indices = HashMap::new();
    for (index, route) in catalog.routes.iter().enumerate() {
        if indices.insert(route.id.clone(), index).is_some() {
            return Err(Error::InvalidCatalog);
        }
    }
    if !indices.contains_key(&catalog.initial) {
        return Err(Error::InvalidCatalog);
    }
    Ok(indices)
}

fn route_patterns(catalog: &Catalog) -> Result<Vec<Pattern>, Error> {
    let mut shapes = BTreeSet::new();
    catalog
        .routes
        .iter()
        .enumerate()
        .map(|(index, route)| {
            let pattern = Pattern::parse(index, &route.path)?;
            if !shapes.insert(pattern.shape()) {
                return Err(Error::Conflict);
            }
            validate_parameters(route, &pattern)?;
            Ok(pattern)
        })
        .collect()
}

fn validate_parameters(route: &Route, pattern: &Pattern) -> Result<(), Error> {
    let names: BTreeSet<_> = pattern
        .segments
        .iter()
        .filter_map(|segment| match segment {
            Segment::Parameter(name) | Segment::CatchAll(name) => Some(name),
            Segment::Literal(_) => None,
        })
        .collect();
    if names != route.params.keys().collect() {
        return Err(Error::InvalidCatalog);
    }
    if route
        .params
        .values()
        .any(|p| !p.valid_schema() || p.multiple || !p.defaults.is_empty())
    {
        return Err(Error::InvalidCatalog);
    }
    if route
        .query
        .iter()
        .any(|(name, p)| name.is_empty() || !p.valid_schema())
    {
        return Err(Error::InvalidCatalog);
    }
    Ok(())
}

fn ancestor_chains(
    catalog: &Catalog,
    indices: &HashMap<String, usize>,
) -> Result<Vec<Vec<String>>, Error> {
    let mut chains: Vec<Vec<String>> = vec![Vec::new(); catalog.routes.len()];
    for index in 0..catalog.routes.len() {
        let mut trail = Vec::new();
        let mut visited = BTreeSet::new();
        let mut current = Some(index);
        let mut ancestors = Vec::new();
        while let Some(index) = current {
            if !chains[index].is_empty() {
                ancestors = chains[index].clone();
                break;
            }
            if !visited.insert(index) {
                return Err(Error::InvalidCatalog);
            }
            trail.push(index);
            current = catalog.routes[index]
                .parent
                .as_ref()
                .map(|parent| indices.get(parent).copied().ok_or(Error::InvalidCatalog))
                .transpose()?;
        }
        for index in trail.into_iter().rev() {
            ancestors.push(catalog.routes[index].id.clone());
            chains[index] = ancestors.clone();
        }
    }
    Ok(chains)
}
