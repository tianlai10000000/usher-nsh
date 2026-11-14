//! Lightweight module indexing and on-demand business catalog assembly.
use super::{
    Command, Error,
    adapters::nsh,
    modules::{self, ModuleFactory},
};
use nsh_core::platform::Os;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use usher_shortcuts::{
    BindingId, BindingSpec, Catalog, DomainId, EditingPolicy, Policy, ResourceKey, ScopeId,
    ScopeRelations,
};

/// Application loading category; it never enters usher-shortcuts.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ModuleKind {
    Global,
    Application,
    Page,
}
/// Lightweight identity and lifecycle declaration, without bindings or handlers.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModuleDescriptor {
    pub id: String,
    pub version: String,
    pub kind: ModuleKind,
    /// Required only for page modules; opaque to the generic shortcut library.
    pub page: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Index {
    schema_version: u32,
    modules: Vec<ModuleDescriptor>,
}
/// Module payload, loaded only on explicit request.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleConfig {
    pub id: String,
    pub version: String,
    pub bindings: Vec<BindingConfig>,
}
/// Product binding data. All defaults are supplied by the module configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BindingConfig {
    pub id: String,
    pub command: String,
    pub keystroke: String,
    pub editing: Editing,
    pub repeat: bool,
    pub pass_through: bool,
    pub required: bool,
}
/// Serializable application editor policy, mapped explicitly at the core boundary.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Editing {
    Auto,
    Always,
    NotEditing,
}
/// Loaded business mapping and independent core declarations.
#[derive(Clone)]
pub struct LoadedModule {
    pub descriptor: ModuleDescriptor,
    pub bindings: Vec<BindingSpec>,
    pub commands: BTreeMap<BindingId, Command>,
}
/// Explicit module index. Factories are not evaluated during index construction.
#[derive(Clone)]
pub struct ModuleIndex {
    descriptors: BTreeMap<String, ModuleDescriptor>,
    factories: BTreeMap<String, ModuleFactory>,
}
impl ModuleIndex {
    /// Reads the production lightweight index without loading page declarations.
    pub fn application() -> Result<Self, Error> {
        Self::parse(
            include_str!("../../ui/config/shortcuts/index.json"),
            modules::factories(),
        )
    }
    /// Validates descriptor/factory correspondence but never invokes a factory.
    pub fn parse(json: &str, factories: Vec<ModuleFactory>) -> Result<Self, Error> {
        let index: Index =
            serde_json::from_str(json).map_err(|e| Error::InvalidConfig(e.to_string()))?;
        if index.schema_version != 1 {
            return Err(Error::InvalidConfig("unsupported schema".into()));
        }
        let mut factory_map = BTreeMap::new();
        for factory in factories {
            if factory_map.insert(factory.id.to_owned(), factory).is_some() {
                return Err(Error::DuplicateModule(factory.id.into()));
            }
        }
        let mut descriptors = BTreeMap::new();
        for descriptor in index.modules {
            if descriptor.id.trim().is_empty() || descriptor.version.trim().is_empty() {
                return Err(Error::InvalidConfig("empty module identity".into()));
            }
            let has_page = descriptor
                .page
                .as_ref()
                .is_some_and(|page| !page.trim().is_empty());
            if (descriptor.kind == ModuleKind::Page && !has_page)
                || (descriptor.kind != ModuleKind::Page && descriptor.page.is_some())
            {
                return Err(Error::InvalidConfig("invalid page module ownership".into()));
            }
            if !factory_map.contains_key(&descriptor.id) {
                return Err(Error::UnknownModule(descriptor.id));
            }
            let id = descriptor.id.clone();
            if descriptors.insert(id.clone(), descriptor).is_some() {
                return Err(Error::DuplicateModule(id));
            }
        }
        if let Some(id) = factory_map.keys().find(|id| !descriptors.contains_key(*id)) {
            return Err(Error::UnknownModule(id.clone()));
        }
        Ok(Self {
            descriptors,
            factories: factory_map,
        })
    }
    /// Returns lightweight descriptors only; observing status does not load a module.
    pub fn descriptors(&self) -> impl Iterator<Item = &ModuleDescriptor> {
        self.descriptors.values()
    }
    /// Loads and validates one module for a supplied owner scope/resource domain.
    /// Cross-module validation is performed by assemble before registration is allowed.
    pub fn load(
        &self,
        id: &str,
        os: Os,
        domain: DomainId,
        scope: ScopeId,
    ) -> Result<LoadedModule, Error> {
        let descriptor = self
            .descriptors
            .get(id)
            .ok_or_else(|| Error::UnknownModule(id.into()))?;

        let factory = self
            .factories
            .get(id)
            .ok_or_else(|| Error::UnknownModule(id.into()))?;
        let config = (factory.load)()?;
        if config.id != descriptor.id || config.version != descriptor.version {
            return Err(Error::VersionMismatch(id.into()));
        }
        let mut bindings = Vec::new();
        let mut commands = BTreeMap::new();
        for entry in config.bindings {
            let binding_id = BindingId::new(entry.id)?;
            let command = (factory.command)(&entry.command)?;
            if commands.insert(binding_id.clone(), command).is_some() {
                return Err(usher_shortcuts::Error::DuplicateBinding(binding_id).into());
            }
            if descriptor.kind == ModuleKind::Global
                && (!matches!(entry.editing, Editing::Always)
                    || !entry.repeat
                    || entry.pass_through)
            {
                return Err(Error::InvalidConfig("unsupported global policy".into()));
            }
            let stroke = if descriptor.kind == ModuleKind::Global {
                nsh::normalize_global(&entry.keystroke, os)?
            } else {
                nsh::normalize_local(&entry.keystroke, os)?
            };
            let editing = match entry.editing {
                Editing::Auto => EditingPolicy::Auto,
                Editing::Always => EditingPolicy::Always,
                Editing::NotEditing => EditingPolicy::NotEditing,
            };
            bindings.push(BindingSpec {
                id: binding_id,
                scope: scope.clone(),
                resource: ResourceKey {
                    domain: domain.clone(),
                    stroke,
                },
                policy: Policy {
                    editing,
                    repeat: entry.repeat,
                    pass_through: entry.pass_through,
                    required: entry.required,
                },
            });
        }
        // Validate the module before returning any executable declaration.
        Catalog::new(
            0,
            bindings.clone(),
            ScopeRelations::new(BTreeSet::from([scope]), vec![])?,
        )?;
        Ok(LoadedModule {
            descriptor: descriptor.clone(),
            bindings,
            commands,
        })
    }
}
/// Validates a complete set of loaded modules against caller-declared coexistence rules.
/// This must succeed before installing a new module alongside already active bindings.
pub fn assemble(
    revision: u64,
    modules: &[LoadedModule],
    relations: ScopeRelations,
) -> Result<Catalog, Error> {
    Catalog::new(
        revision,
        modules.iter().flat_map(|m| m.bindings.clone()).collect(),
        relations,
    )
    .map_err(Error::from)
}
