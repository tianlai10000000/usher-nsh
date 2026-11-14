//! Catalog validation and pure selection of externally enabled bindings.
use crate::{BindingId, BindingSpec, DesiredBinding, Error, OwnerToken, ScopeId};
use std::collections::{BTreeMap, BTreeSet};

/// Explicitly disjoint scope pairs. Missing pairs conservatively overlap.
#[derive(Clone, Debug)]
pub struct ScopeRelations {
    scopes: BTreeSet<ScopeId>,
    disjoint: BTreeSet<(ScopeId, ScopeId)>,
}
impl ScopeRelations {
    /// Validates all relation endpoints; a scope cannot be disjoint from itself.
    pub fn new(scopes: BTreeSet<ScopeId>, pairs: Vec<(ScopeId, ScopeId)>) -> Result<Self, Error> {
        let mut disjoint = BTreeSet::new();
        for (a, b) in pairs {
            for id in [&a, &b] {
                if !scopes.contains(id) {
                    return Err(Error::UnknownScope(id.clone()));
                }
            }
            if a == b {
                return Err(Error::InvalidRelation);
            }
            disjoint.insert(ordered(a, b));
        }
        Ok(Self { scopes, disjoint })
    }
    /// Reports possible overlap without interpreting scope names.
    pub fn overlaps(&self, a: &ScopeId, b: &ScopeId) -> Result<bool, Error> {
        for id in [a, b] {
            if !self.scopes.contains(id) {
                return Err(Error::UnknownScope(id.clone()));
            }
        }
        Ok(!self.disjoint.contains(&ordered(a.clone(), b.clone())))
    }
}
fn ordered(a: ScopeId, b: ScopeId) -> (ScopeId, ScopeId) {
    if a <= b { (a, b) } else { (b, a) }
}

/// Validated immutable bindings. Business command validation belongs to callers.
#[derive(Clone, Debug)]
pub struct Catalog {
    revision: u64,
    bindings: BTreeMap<BindingId, BindingSpec>,
    relations: ScopeRelations,
}
impl Catalog {
    /// Rejects duplicate identities and keys in potentially overlapping scopes.
    pub fn new(
        revision: u64,
        specs: Vec<BindingSpec>,
        relations: ScopeRelations,
    ) -> Result<Self, Error> {
        let mut bindings = BTreeMap::<BindingId, BindingSpec>::new();
        for spec in specs {
            relations.overlaps(&spec.scope, &spec.scope)?;
            if bindings.contains_key(&spec.id) {
                return Err(Error::DuplicateBinding(spec.id));
            }
            for previous in bindings.values() {
                if previous.resource == spec.resource
                    && relations.overlaps(&previous.scope, &spec.scope)?
                {
                    return Err(Error::Conflict(previous.id.clone(), spec.id));
                }
            }
            bindings.insert(spec.id.clone(), spec);
        }
        Ok(Self {
            revision,
            bindings,
            relations,
        })
    }
    /// Returns declarations without exposing mutable catalog state.
    pub fn bindings(&self) -> impl Iterator<Item = &BindingSpec> {
        self.bindings.values()
    }
    /// Selects active/enabled declarations. Unknown inputs and conflicting active scopes fail.
    /// Callers compute availability and supply the owner; no business predicate is invoked.
    pub fn plan(
        &self,
        active: &BTreeSet<ScopeId>,
        enabled: &BTreeSet<BindingId>,
        owner: OwnerToken,
    ) -> Result<Vec<DesiredBinding>, Error> {
        for scope in active {
            self.relations.overlaps(scope, scope)?;
        }
        for id in enabled {
            if !self.bindings.contains_key(id) {
                return Err(Error::UnknownBinding(id.clone()));
            }
        }
        let mut plan: Vec<DesiredBinding> = Vec::new();
        for spec in self
            .bindings
            .values()
            .filter(|s| active.contains(&s.scope) && enabled.contains(&s.id))
        {
            if let Some(old) = plan
                .iter()
                .find(|old| old.binding.resource == spec.resource)
            {
                return Err(Error::Conflict(old.binding.id.clone(), spec.id.clone()));
            }
            plan.push(DesiredBinding {
                binding: spec.clone(),
                owner: owner.clone(),
                revision: self.revision,
            });
        }
        Ok(plan)
    }
}
