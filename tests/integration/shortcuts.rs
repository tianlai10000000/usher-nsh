//! Application shortcut module boundaries, lazy index contracts and NSH normalization.
use nsh_core::platform::Os;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use usher_nsh::shortcuts::{
    self, Error,
    adapters::nsh::normalize_local,
    catalog::{ModuleConfig, ModuleIndex, assemble},
    modules::ModuleFactory,
};
use usher_shortcuts::{DomainId, ScopeId, ScopeRelations};

fn domain() -> DomainId {
    DomainId::new("window").unwrap()
}
fn scope() -> ScopeId {
    ScopeId::new("application").unwrap()
}
fn production_config() -> ModuleConfig {
    serde_json::from_str(include_str!("../../ui/config/shortcuts/navigation.json")).unwrap()
}
fn mapped_command(id: &str) -> Result<shortcuts::Command, Error> {
    match id {
        "back" => Ok(shortcuts::Command::Navigation(
            shortcuts::modules::navigation::Command::Back,
        )),
        "forward" => Ok(shortcuts::Command::Navigation(
            shortcuts::modules::navigation::Command::Forward,
        )),
        _ => Err(Error::UnknownCommand(id.into())),
    }
}
fn index_with(load: fn() -> Result<ModuleConfig, Error>) -> ModuleIndex {
    ModuleIndex::parse(
        include_str!("fixtures/shortcuts-index.json"),
        vec![ModuleFactory {
            id: "navigation",
            load,
            command: mapped_command,
        }],
    )
    .unwrap()
}
#[test]
fn shortcuts_all_declared_modules_validate_on_desktop_targets() {
    for os in [Os::MacOs, Os::Windows, Os::Linux] {
        let index = ModuleIndex::application().unwrap();
        let modules: Vec<_> = index
            .descriptors()
            .map(|d| index.load(&d.id, os, domain(), scope()).unwrap())
            .collect();
        let catalog = assemble(
            1,
            &modules,
            ScopeRelations::new(BTreeSet::from([scope()]), vec![]).unwrap(),
        )
        .unwrap();
        assert_eq!(catalog.bindings().count(), 4);
        let navigation = modules
            .iter()
            .find(|m| m.descriptor.id == "navigation")
            .unwrap();
        assert_eq!(navigation.commands.len(), 2);
        assert!(
            navigation
                .bindings
                .iter()
                .all(|b| b.policy.editing == usher_shortcuts::EditingPolicy::NotEditing)
        );
    }
}
#[test]
fn shortcuts_nsh_normalizes_primary_aliases_and_exact_modifiers() {
    assert_eq!(
        normalize_local("mod+[", Os::MacOs).unwrap(),
        normalize_local("COMMAND+[", Os::MacOs).unwrap()
    );
    assert_eq!(
        normalize_local("mod+[", Os::Windows).unwrap(),
        normalize_local("control+[", Os::Windows).unwrap()
    );
    assert_ne!(
        normalize_local("mod+[", Os::MacOs).unwrap(),
        normalize_local("mod+shift+[", Os::MacOs).unwrap()
    );
    assert!(matches!(
        normalize_local("cmd+q", Os::MacOs),
        Err(Error::ReservedKeystroke(_))
    ));
    assert!(matches!(
        normalize_local("mod++", Os::Linux),
        Err(Error::InvalidKeystroke(_))
    ));
}
static LOADS: AtomicUsize = AtomicUsize::new(0);
fn counted_load() -> Result<ModuleConfig, Error> {
    LOADS.fetch_add(1, Ordering::SeqCst);
    Ok(production_config())
}
#[test]
fn shortcuts_index_observation_does_not_execute_module_factories() {
    LOADS.store(0, Ordering::SeqCst);
    let index = index_with(counted_load);
    assert_eq!(index.descriptors().count(), 1);
    assert_eq!(LOADS.load(Ordering::SeqCst), 0);
    index
        .load("navigation", Os::MacOs, domain(), scope())
        .unwrap();
    assert_eq!(LOADS.load(Ordering::SeqCst), 1);
}
fn bad_version() -> Result<ModuleConfig, Error> {
    let mut config = production_config();
    config.version = "incompatible".into();
    Ok(config)
}
fn bad_command() -> Result<ModuleConfig, Error> {
    let mut config = production_config();
    config.bindings[0].command = "unknown".into();
    Ok(config)
}
fn conflict() -> Result<ModuleConfig, Error> {
    let mut config = production_config();
    config.bindings[1].keystroke = config.bindings[0].keystroke.clone();
    Ok(config)
}
#[test]
fn shortcuts_load_rejects_version_command_and_binding_conflicts() {
    assert!(matches!(
        index_with(bad_version).load("navigation", Os::MacOs, domain(), scope()),
        Err(Error::VersionMismatch(_))
    ));
    assert!(matches!(
        index_with(bad_command).load("navigation", Os::MacOs, domain(), scope()),
        Err(Error::UnknownCommand(_))
    ));
    assert!(matches!(
        index_with(conflict).load("navigation", Os::MacOs, domain(), scope()),
        Err(Error::Core(usher_shortcuts::Error::Conflict(..)))
    ));
}
#[test]
fn shortcuts_index_rejects_missing_or_duplicate_module_factories() {
    let json = include_str!("fixtures/shortcuts-index.json");
    assert!(matches!(
        ModuleIndex::parse(json, vec![]),
        Err(Error::UnknownModule(_))
    ));
    let factory = ModuleFactory {
        id: "navigation",
        load: counted_load,
        command: mapped_command,
    };
    assert!(matches!(
        ModuleIndex::parse(json, vec![factory, factory]),
        Err(Error::DuplicateModule(_))
    ));
}
#[test]
fn shortcuts_aggregate_rejects_cross_module_binding_collision() {
    let index = ModuleIndex::application().unwrap();
    let a = index
        .load("navigation", Os::MacOs, domain(), scope())
        .unwrap();
    let mut b = index
        .load("navigation", Os::MacOs, domain(), scope())
        .unwrap();
    b.bindings[0].id = usher_shortcuts::BindingId::new("other.back").unwrap();
    b.bindings[1].id = usher_shortcuts::BindingId::new("other.forward").unwrap();
    assert!(matches!(
        assemble(
            1,
            &[a, b],
            ScopeRelations::new(BTreeSet::from([scope()]), vec![]).unwrap()
        ),
        Err(Error::Core(usher_shortcuts::Error::Conflict(..)))
    ));
}

#[test]
fn shortcuts_page_index_requires_explicit_owner_without_loading() {
    let factory = ModuleFactory {
        id: "navigation",
        load: counted_load,
        command: mapped_command,
    };
    let json = include_str!("fixtures/shortcuts-index.json").replace("application", "page");
    assert!(matches!(
        ModuleIndex::parse(&json, vec![factory]),
        Err(Error::InvalidConfig(_))
    ));
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    value["modules"][0]["page"] = serde_json::Value::String("opaque-page".into());
    let index = ModuleIndex::parse(&value.to_string(), vec![factory]).unwrap();
    assert_eq!(
        index.descriptors().next().unwrap().page.as_deref(),
        Some("opaque-page")
    );
}

// Window services use the real router and publication backend, with isolated page declarations.
fn shortcut_window() -> std::sync::Arc<usher_nsh::state::window::WindowService> {
    usher_nsh::state::window::WindowService::new(
        serde_json::from_str(include_str!("../../ui/pages/routes.json")).unwrap(),
    )
    .unwrap()
}
fn navigate_settings(window: &usher_nsh::state::window::WindowService) {
    window
        .navigate(usher_router::NavigationCommand::Push(
            usher_router::Target::path("/settings"),
        ))
        .unwrap();
}
fn host(
    window: std::sync::Arc<usher_nsh::state::window::WindowService>,
    index: ModuleIndex,
) -> std::sync::Arc<shortcuts::service::ShortcutService> {
    shortcuts::service::ShortcutService::new(
        window,
        std::sync::Arc::new(shortcuts::loader::ModuleLoader::new(index)),
        Os::MacOs,
        1,
    )
    .unwrap()
}
fn acknowledge_all(
    service: &shortcuts::service::ShortcutService,
    effects: Vec<shortcuts::service::WireEffect>,
) {
    let mut queue: std::collections::VecDeque<_> = effects.into();
    while let Some(e) = queue.pop_front() {
        queue.extend(
            service
                .acknowledge(&e.ticket, true, e.kind == "release")
                .unwrap(),
        );
    }
}
fn page_config() -> Result<ModuleConfig, Error> {
    let mut config = production_config();
    config.id = "fixture-page".into();
    config.bindings.truncate(1);
    config.bindings[0].id = "fixture-page.back".into();
    config.bindings[0].keystroke = "mod+k".into();
    Ok(config)
}
fn page_index(load: fn() -> Result<ModuleConfig, Error>) -> ModuleIndex {
    let mut json: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/shortcuts-index.json")).unwrap();
    json["modules"].as_array_mut().unwrap().push(
        serde_json::json!({"id":"fixture-page","version":"1","kind":"page","page":"settings"}),
    );
    let mut factories: Vec<_> = shortcuts::modules::factories()
        .into_iter()
        .filter(|factory| factory.id == "navigation")
        .collect();
    factories.push(ModuleFactory {
        id: "fixture-page",
        load,
        command: mapped_command,
    });
    ModuleIndex::parse(&json.to_string(), factories).unwrap()
}
#[test]
fn shortcuts_window_updates_availability_focus_and_status_without_view_ownership() {
    let window = shortcut_window();
    let service = host(window.clone(), ModuleIndex::application().unwrap());
    assert_eq!(service.snapshot().unwrap().state().startup, "validating");
    assert!(service.initialize().unwrap().is_empty());
    assert_eq!(service.snapshot().unwrap().state().startup, "ready");
    navigate_settings(&window);
    let effects = service.context(true, false).unwrap();
    assert_eq!(effects.len(), 1);
    let ticket = effects[0].ticket.clone();
    assert!(service.trigger(&ticket).unwrap().is_none());
    assert_eq!(service.snapshot().unwrap().state().startup, "installing");
    acknowledge_all(&service, effects);
    assert!(service.trigger(&ticket).unwrap().is_some());
    let release = service.context(false, false).unwrap();
    assert!(service.trigger(&ticket).unwrap().is_none());
    acknowledge_all(&service, release);
    acknowledge_all(&service, service.context(true, false).unwrap());
    assert_eq!(service.snapshot().unwrap().state().startup, "ready");
}
#[test]
fn shortcuts_modal_blocks_immediately_and_guard_cancellation_preserves_current_page() {
    let window = shortcut_window();
    navigate_settings(&window);
    let service = host(window.clone(), page_index(page_config));
    service.initialize().unwrap();
    acknowledge_all(&service, service.context(true, false).unwrap());
    let entry = window.snapshot().unwrap().state().history.current().clone();
    let (page, effects) = service
        .begin_page(&entry.key, &entry.location.page)
        .unwrap();
    acknowledge_all(&service, effects);
    let effects = service.finish_page(&page).unwrap();
    let ticket = effects
        .iter()
        .find(|e| e.binding == "fixture-page.back")
        .unwrap()
        .ticket
        .clone();
    acknowledge_all(&service, effects);
    let guard = window
        .begin(usher_router::NavigationCommand::Go(-1))
        .unwrap();
    assert!(service.trigger(&ticket).unwrap().is_some());
    window
        .complete(&guard, usher_nsh::state::window::Decision::Cancel)
        .unwrap();
    assert!(service.trigger(&ticket).unwrap().is_some());
    let effects = service.context(true, true).unwrap();
    assert!(service.trigger(&ticket).unwrap().is_none());
    acknowledge_all(&service, effects);
    acknowledge_all(&service, service.context(true, false).unwrap());
    assert_eq!(service.snapshot().unwrap().state().page, "ready");
}
static PAGE_LOADS: AtomicUsize = AtomicUsize::new(0);
fn counted_page() -> Result<ModuleConfig, Error> {
    PAGE_LOADS.fetch_add(1, Ordering::SeqCst);
    page_config()
}
#[test]
fn shortcuts_page_load_is_lazy_cached_and_late_requests_do_not_activate() {
    PAGE_LOADS.store(0, Ordering::SeqCst);
    let window = shortcut_window();
    let service = host(window.clone(), page_index(counted_page));
    service.initialize().unwrap();
    assert_eq!(PAGE_LOADS.load(Ordering::SeqCst), 0);
    assert_eq!(
        service
            .snapshot()
            .unwrap()
            .state()
            .modules
            .iter()
            .find(|m| m.id == "fixture-page")
            .unwrap()
            .phase,
        "notLoaded"
    );
    navigate_settings(&window);
    let entry = window.snapshot().unwrap().state().history.current().clone();
    let (first, _) = service
        .begin_page(&entry.key, &entry.location.page)
        .unwrap();
    service.leave_page(&first).unwrap();
    service.finish_page(&first).unwrap();
    assert_eq!(PAGE_LOADS.load(Ordering::SeqCst), 0);
    let (second, _) = service
        .begin_page(&entry.key, &entry.location.page)
        .unwrap();
    service.finish_page(&second).unwrap();
    assert_eq!(PAGE_LOADS.load(Ordering::SeqCst), 1);
    service.leave_page(&second).unwrap();
    let (third, _) = service
        .begin_page(&entry.key, &entry.location.page)
        .unwrap();
    service.finish_page(&third).unwrap();
    assert_eq!(PAGE_LOADS.load(Ordering::SeqCst), 1);
    assert_eq!(service.snapshot().unwrap().state().startup, "ready");
    assert_eq!(service.snapshot().unwrap().state().page, "ready");
}
#[test]
fn shortcuts_committed_navigation_fences_page_trigger_before_ui_refresh() {
    let window = shortcut_window();
    navigate_settings(&window);
    let service = host(window.clone(), page_index(page_config));
    service.initialize().unwrap();
    acknowledge_all(&service, service.context(true, false).unwrap());
    let entry = window.snapshot().unwrap().state().history.current().clone();
    let (page, _) = service
        .begin_page(&entry.key, &entry.location.page)
        .unwrap();
    let effects = service.finish_page(&page).unwrap();
    let ticket = effects
        .iter()
        .find(|e| e.binding == "fixture-page.back")
        .unwrap()
        .ticket
        .clone();
    acknowledge_all(&service, effects);
    window
        .navigate(usher_router::NavigationCommand::Go(-1))
        .unwrap();
    assert!(service.trigger(&ticket).unwrap().is_none());
    acknowledge_all(&service, service.context(true, false).unwrap());
    assert!(service.trigger(&ticket).unwrap().is_none());
}
fn conflicting_page() -> Result<ModuleConfig, Error> {
    let mut config = page_config()?;
    config.bindings[0].keystroke = "mod+[".into();
    Ok(config)
}
#[test]
fn shortcuts_lazy_conflict_fails_only_the_page_and_keeps_window_ready() {
    let window = shortcut_window();
    navigate_settings(&window);
    let service = host(window.clone(), page_index(conflicting_page));
    service.initialize().unwrap();
    acknowledge_all(&service, service.context(true, false).unwrap());
    let entry = window.snapshot().unwrap().state().history.current().clone();
    let (page, _) = service
        .begin_page(&entry.key, &entry.location.page)
        .unwrap();
    service.finish_page(&page).unwrap();
    let health = service.snapshot().unwrap();
    assert_eq!(health.state().page, "failed");
    assert_eq!(health.state().startup, "ready");
    assert!(
        !health
            .state()
            .bindings
            .iter()
            .any(|b| b.module == "fixture-page")
    );
}
#[test]
fn shortcuts_registration_failure_is_visible_and_explicit_retry_replaces_ticket() {
    let window = shortcut_window();
    navigate_settings(&window);
    let service = host(window, ModuleIndex::application().unwrap());
    service.initialize().unwrap();
    let effects = service.context(true, false).unwrap();
    let old = effects[0].ticket.clone();
    service.acknowledge(&old, false, false).unwrap();
    assert_eq!(service.snapshot().unwrap().state().startup, "failed");
    let effects = service.retry().unwrap();
    assert_ne!(effects[0].ticket, old);
    acknowledge_all(&service, effects);
    assert_eq!(service.snapshot().unwrap().state().startup, "ready");
    assert!(service.trigger(&old).unwrap().is_none());
}
#[test]
fn shortcuts_window_services_and_close_are_isolated() {
    let a = shortcut_window();
    let b = shortcut_window();
    navigate_settings(&a);
    let first = host(a, ModuleIndex::application().unwrap());
    let second = host(b, ModuleIndex::application().unwrap());
    first.initialize().unwrap();
    second.initialize().unwrap();
    assert_ne!(
        first.snapshot().unwrap().store_id(),
        second.snapshot().unwrap().store_id()
    );
    let effects = first.context(true, false).unwrap();
    let ticket = effects[0].ticket.clone();
    acknowledge_all(&first, effects);
    assert!(second.context(true, false).unwrap().is_empty());
    first.close();
    assert!(first.trigger(&ticket).unwrap().is_none());
    assert_eq!(second.snapshot().unwrap().state().startup, "ready");
}

#[test]
fn shortcuts_release_failure_retry_keeps_ownership_until_acknowledged() {
    let window = shortcut_window();
    navigate_settings(&window);
    let service = host(window, ModuleIndex::application().unwrap());
    service.initialize().unwrap();
    acknowledge_all(&service, service.context(true, false).unwrap());
    let release = service.context(false, false).unwrap();
    let old = release[0].ticket.clone();
    service.acknowledge(&old, false, true).unwrap();
    assert!(service.trigger(&old).unwrap().is_none());
    let retry = service.retry().unwrap();
    assert!(
        retry
            .iter()
            .any(|effect| effect.kind == "release" && effect.ticket == old)
    );
    acknowledge_all(&service, retry);
    acknowledge_all(&service, service.context(true, false).unwrap());
    assert_eq!(service.snapshot().unwrap().state().startup, "ready");
    assert!(service.trigger(&old).unwrap().is_none());
}

#[test]
fn shortcuts_loader_coalesces_concurrent_requests_and_replacement_invalidates_cache() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    fn load() -> Result<ModuleConfig, Error> {
        CALLS.fetch_add(1, Ordering::SeqCst);
        page_config()
    }
    let loader = std::sync::Arc::new(shortcuts::loader::ModuleLoader::new(page_index(load)));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let loader = loader.clone();
            std::thread::spawn(move || {
                loader
                    .load("fixture-page", Os::MacOs)
                    .unwrap()
                    .bindings
                    .len()
            })
        })
        .collect();
    for worker in workers {
        assert_eq!(worker.join().unwrap(), 1);
    }
    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
    let replacement = shortcuts::loader::ModuleLoader::new(page_index(load));
    replacement.load("fixture-page", Os::MacOs).unwrap();
    assert_eq!(CALLS.load(Ordering::SeqCst), 2);
}

#[test]
fn shortcuts_failed_page_factory_is_retryable_without_restarting_window() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    fn load() -> Result<ModuleConfig, Error> {
        if CALLS.fetch_add(1, Ordering::SeqCst) == 0 {
            Err(Error::InvalidConfig("temporary".into()))
        } else {
            page_config()
        }
    }
    let window = shortcut_window();
    navigate_settings(&window);
    let service = host(window.clone(), page_index(load));
    service.initialize().unwrap();
    acknowledge_all(&service, service.context(true, false).unwrap());
    let entry = window.snapshot().unwrap().state().history.current().clone();
    let (first, _) = service
        .begin_page(&entry.key, &entry.location.page)
        .unwrap();
    service.finish_page(&first).unwrap();
    assert_eq!(service.snapshot().unwrap().state().page, "failed");
    assert_eq!(service.snapshot().unwrap().state().startup, "ready");
    let (retry, effects) = service
        .begin_page(&entry.key, &entry.location.page)
        .unwrap();
    acknowledge_all(&service, effects);
    acknowledge_all(&service, service.finish_page(&retry).unwrap());
    assert_eq!(service.snapshot().unwrap().state().page, "ready");
    assert_eq!(CALLS.load(Ordering::SeqCst), 2);
}

// Process-global fixtures intentionally live outside the product catalog.
fn global_config() -> Result<ModuleConfig, Error> {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../e2e/global-shortcuts/config.json")).unwrap();
    let mut module: ModuleConfig = serde_json::from_value(config["payload"].clone())
        .map_err(|e| Error::InvalidConfig(e.to_string()))?;
    module.bindings.truncate(1);
    Ok(module)
}
fn global_command(id: &str) -> Result<shortcuts::Command, Error> {
    match id {
        "focusMain" => Ok(shortcuts::Command::Window(
            shortcuts::modules::window::Command::FocusMain,
        )),
        _ => Err(Error::UnknownCommand(id.into())),
    }
}
fn global_index(load: fn() -> Result<ModuleConfig, Error>) -> ModuleIndex {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../e2e/global-shortcuts/config.json")).unwrap();
    let mut index: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/shortcuts-index.json")).unwrap();
    index["modules"]
        .as_array_mut()
        .unwrap()
        .push(config["module"].clone());
    let mut factories: Vec<_> = shortcuts::modules::factories()
        .into_iter()
        .filter(|factory| factory.id == "navigation")
        .collect();
    factories.push(ModuleFactory {
        id: "fixture-window",
        load,
        command: global_command,
    });
    ModuleIndex::parse(&index.to_string(), factories).unwrap()
}
fn global_host(os: Os) -> std::sync::Arc<shortcuts::global::GlobalService> {
    shortcuts::global::GlobalService::new(
        std::sync::Arc::new(shortcuts::loader::ModuleLoader::new(global_index(
            global_config,
        ))),
        os,
        1,
    )
    .unwrap()
}
#[test]
fn shortcuts_global_initializes_once_and_requires_native_confirmation() {
    let service = global_host(Os::MacOs);
    assert!(service.claim("vm-a", false).unwrap());
    assert!(!service.claim("vm-b", false).unwrap());
    let effects = service.initialize("vm-a").unwrap();
    assert_eq!(effects.len(), 1);
    assert!(service.initialize("vm-a").unwrap().is_empty());
    let ticket = &effects[0].ticket;
    assert!(service.trigger("vm-a", ticket).unwrap().is_none());
    assert_eq!(service.snapshot().unwrap().state().global, "installing");
    assert!(service.acknowledge("vm-b", ticket, true, false).is_err());
    service.acknowledge("vm-a", ticket, true, false).unwrap();
    assert_eq!(service.snapshot().unwrap().state().global, "ready");
    assert!(matches!(
        service.trigger("vm-a", ticket).unwrap(),
        Some(shortcuts::Command::Window(_))
    ));
}
#[test]
fn shortcuts_global_rejection_is_visible_and_retry_replaces_only_settled_attempts() {
    let service = global_host(Os::MacOs);
    service.claim("vm", false).unwrap();
    let first = service.initialize("vm").unwrap().remove(0);
    assert!(service.retry("vm").unwrap().is_empty());
    service
        .acknowledge("vm", &first.ticket, false, false)
        .unwrap();
    assert_eq!(service.snapshot().unwrap().state().global, "failed");
    let second = service.retry("vm").unwrap().remove(0);
    assert_ne!(first.ticket, second.ticket);
    service
        .acknowledge("vm", &second.ticket, true, false)
        .unwrap();
    assert_eq!(service.snapshot().unwrap().state().global, "ready");
    assert!(service.trigger("vm", &first.ticket).unwrap().is_none());
}
#[test]
fn shortcuts_global_pending_owner_requires_verified_host_reset_before_takeover() {
    let service = global_host(Os::MacOs);
    service.claim("old", false).unwrap();
    let first = service.initialize("old").unwrap().remove(0);
    service.abandon("old");
    assert_eq!(service.snapshot().unwrap().state().global, "recovering");
    assert!(!service.claim("new", false).unwrap());
    assert!(service.claim("new", true).unwrap());
    let second = service.initialize("new").unwrap().remove(0);
    assert_ne!(first.ticket, second.ticket);
    assert!(
        service
            .acknowledge("old", &first.ticket, true, false)
            .is_err()
    );
    assert!(service.trigger("old", &first.ticket).unwrap().is_none());
    service.abandon("old");
    service
        .acknowledge("new", &second.ticket, true, false)
        .unwrap();
    assert_eq!(service.snapshot().unwrap().state().global, "ready");
}
#[test]
fn shortcuts_global_unsupported_is_distinct_from_registration_rejection() {
    let service = global_host(Os::Ios);
    service.claim("vm", false).unwrap();
    assert!(service.initialize("vm").unwrap().is_empty());
    assert_eq!(service.snapshot().unwrap().state().global, "unsupported");
    assert!(service.snapshot().unwrap().state().bindings.is_empty());
}
#[test]
fn shortcuts_global_local_conflicts_are_rejected_before_registration() {
    fn conflicting() -> Result<ModuleConfig, Error> {
        let mut config = global_config()?;
        config.bindings[0].keystroke = "mod+[".into();
        Ok(config)
    }
    let index = global_index(conflicting);
    let loader = std::sync::Arc::new(shortcuts::loader::ModuleLoader::new(index.clone()));
    let global = shortcuts::global::GlobalService::new(loader, Os::MacOs, 1).unwrap();
    global.claim("vm", false).unwrap();
    assert!(global.initialize("vm").unwrap().is_empty());
    assert_eq!(global.snapshot().unwrap().state().global, "failed");
    let local = host(shortcut_window(), index);
    assert!(local.initialize().unwrap().is_empty());
    assert_eq!(local.snapshot().unwrap().state().startup, "failed");
}
#[test]
fn shortcuts_global_rejects_policies_the_system_backend_cannot_honor() {
    fn invalid() -> Result<ModuleConfig, Error> {
        let mut config = global_config()?;
        config.bindings[0].pass_through = true;
        Ok(config)
    }
    let index = global_index(invalid);
    assert!(
        index
            .load("fixture-window", Os::MacOs, domain(), scope())
            .is_err()
    );
}

#[test]
fn shortcuts_global_concurrent_vm_claims_have_one_process_executor() {
    let service = global_host(Os::MacOs);
    let workers: Vec<_> = (0..8)
        .map(|index| {
            let service = service.clone();
            std::thread::spawn(move || service.claim(&index.to_string(), false).unwrap())
        })
        .collect();
    assert_eq!(
        workers
            .into_iter()
            .map(|worker| usize::from(worker.join().unwrap()))
            .sum::<usize>(),
        1
    );
    // The service mutex establishes the same exclusive lease boundary used by all VM bridges.
    assert!(!service.claim("late", false).unwrap());
}

#[test]
fn shortcuts_global_partial_rejection_preserves_other_registered_commands_and_local_input() {
    let index = ModuleIndex::application().unwrap();
    let global = shortcuts::global::GlobalService::new(
        std::sync::Arc::new(shortcuts::loader::ModuleLoader::new(index.clone())),
        Os::MacOs,
        1,
    )
    .unwrap();
    global.claim("vm", false).unwrap();
    let effects = global.initialize("vm").unwrap();
    assert_eq!(effects.len(), 2);
    global
        .acknowledge("vm", &effects[0].ticket, false, false)
        .unwrap();
    global
        .acknowledge("vm", &effects[1].ticket, true, false)
        .unwrap();
    assert_eq!(global.snapshot().unwrap().state().global, "failed");
    assert!(global.trigger("vm", &effects[1].ticket).unwrap().is_some());
    let window = shortcut_window();
    navigate_settings(&window);
    let local = host(window, index);
    local.initialize().unwrap();
    acknowledge_all(&local, local.context(true, false).unwrap());
    assert_eq!(local.snapshot().unwrap().state().startup, "ready");
}
