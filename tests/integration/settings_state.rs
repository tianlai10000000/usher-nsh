//! Exercises the real settings service, state publication and persistent repository together.

mod support;

use std::{
    fs,
    sync::{Arc, Barrier},
    thread,
};
use support::{FUTURE, LEGACY, Repository, THEME};
use usher_nsh::state::settings::{
    ColorMode, Command, SettingsError, SettingsRegistry, SettingsService,
};
use usher_state::SubscriptionEnd;
use usher_test_support::task::{WakeCounter, poll, ready};

#[test]
fn settings_initialization_noops_and_validation_preserve_the_published_state() {
    let repo = Repository::new();
    let service = repo.service();
    assert!(Arc::ptr_eq(&service, &repo.service()));
    let reader = service.reader();
    let initial = reader.snapshot().unwrap();
    let mut sub = reader.subscribe().unwrap();
    assert_eq!(sub.initial().state(), initial.state());
    assert_eq!(service.snapshot().unwrap(), *initial.state());
    assert_eq!(
        service
            .apply(Command::SetColorMode(initial.state().color_mode))
            .unwrap(),
        *initial.state()
    );
    assert_eq!(
        service.apply(Command::SetLocale("invalid-locale".into())),
        Err(SettingsError::Domain("invalid_locale"))
    );
    assert_eq!(
        service.apply(Command::SetTheme("custom-missing".into())),
        Err(SettingsError::Domain("unknown_theme"))
    );
    assert_eq!(
        service.import_background_image("bad.png".into(), b"invalid image".to_vec()),
        Err(SettingsError::Domain("background_image_invalid"))
    );
    assert_eq!(reader.snapshot().unwrap().sequence(), initial.sequence());
    assert!(poll(std::pin::pin!(sub.next()), &Arc::default()).is_pending());
    assert!(!repo.path.exists());
}

#[test]
fn settings_consumers_observe_commits_and_immutable_previous_snapshots() {
    let repo = Repository::new();
    let service = repo.service();
    let reader = service.reader();
    let initial = reader.snapshot().unwrap();
    let mut fast = reader.subscribe().unwrap();
    let mut slow = reader.subscribe().unwrap();
    let wakes = Arc::new(WakeCounter::default());
    let mut pending = std::pin::pin!(fast.next());
    assert!(poll(pending.as_mut(), &wakes).is_pending());
    let changed = service
        .apply(Command::SetColorMode(ColorMode::Dark))
        .unwrap();
    assert!(wakes.count() > 0);
    let first = match poll(pending.as_mut(), &wakes) {
        std::task::Poll::Ready(Ok(value)) => value,
        _ => panic!("the commit must wake and deliver the published settings"),
    };
    assert_eq!(first.state(), &changed);
    assert_eq!(first.sequence(), initial.sequence() + 1);
    service.apply(Command::SetLocale("zh-Hans".into())).unwrap();
    let last = ready(slow.next()).unwrap();
    assert_eq!(last.sequence(), first.sequence() + 1);
    assert_eq!(last.state().locale_preference, "zh-Hans");
    assert_eq!(initial.state().color_mode, ColorMode::System);
    assert_eq!(
        first.state().locale_preference,
        initial.state().locale_preference
    );
    assert_eq!(first.store_id(), last.store_id());
}

#[test]
fn settings_concurrent_sources_share_initialization_and_commit_without_lost_fields() {
    let repo = Repository::new();
    let commands = vec![
        Command::SetColorMode(ColorMode::Dark),
        Command::SetLocale("zh-Hans".into()),
        Command::SetShowStatusBar(false),
        Command::SetSidebarDocked(false),
    ];
    let count = commands.len();
    let gate = Barrier::new(count);
    let mut results = thread::scope(|scope| {
        let handles: Vec<_> = commands
            .into_iter()
            .map(|command| {
                let repo = &repo;
                let gate = &gate;
                scope.spawn(move || {
                    gate.wait();
                    let service = repo.service();
                    let id = service.reader().snapshot().unwrap().store_id();
                    (id, service.apply(command).unwrap().revision)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    let final_value = repo.service().reader().snapshot().unwrap();
    assert!(results.iter().all(|(id, _)| *id == final_value.store_id()));
    results.sort_by_key(|(_, revision)| *revision);
    assert_eq!(
        results
            .iter()
            .map(|(_, revision)| *revision)
            .collect::<Vec<_>>(),
        (1..=count as u32).collect::<Vec<_>>()
    );
    assert_eq!(final_value.sequence(), count as u64);
    let state = final_value.state();
    assert_eq!(state.color_mode, ColorMode::Dark);
    assert_eq!(state.locale_preference, "zh-Hans");
    assert!(!state.show_status_bar && !state.sidebar_docked);
    assert!(state.saved);
    repo.service().close();
    let reopened = SettingsRegistry::default()
        .for_path(repo.path.clone())
        .unwrap();
    assert_eq!(
        reopened.snapshot().unwrap().locale_preference,
        state.locale_preference
    );
    assert_eq!(reopened.snapshot().unwrap().color_mode, state.color_mode);
}

#[test]
fn settings_failed_write_and_same_revision_retry_both_notify_consumers() {
    let repo = Repository::new();
    let service = repo.service();
    let reader = service.reader();
    let mut sub = reader.subscribe().unwrap();
    let parent = repo.path.parent().unwrap();
    fs::write(parent, "block the settings directory").unwrap();
    let failed = service
        .apply(Command::SetColorMode(ColorMode::Dark))
        .unwrap();
    assert!(!failed.saved);
    assert_eq!(failed.problem, Some("settings_write_failed"));
    let unsaved = ready(sub.next()).unwrap();
    assert_eq!(unsaved.state(), &failed);
    fs::remove_file(parent).unwrap();
    let retried = service
        .apply(Command::SetColorMode(ColorMode::Dark))
        .unwrap();
    let saved = ready(sub.next()).unwrap();
    assert_eq!(saved.state(), &retried);
    assert!(retried.saved);
    assert_eq!(retried.problem, None);
    assert_eq!(retried.revision, failed.revision);
    assert_eq!(saved.sequence(), unsaved.sequence() + 1);
    service
        .apply(Command::SetColorMode(ColorMode::Dark))
        .unwrap();
    assert!(poll(std::pin::pin!(sub.next()), &Arc::default()).is_pending());
    assert_eq!(fs::read_to_string(&repo.path).unwrap(), LEGACY);
}

#[test]
fn settings_scopes_and_engine_sessions_are_isolated_and_ids_are_not_reused() {
    let repo = Repository::new();
    let disk = repo.service();
    let other = repo
        .registry
        .for_path(repo.root.path().join("other/settings.toml"))
        .unwrap();
    let session = SettingsService::session().unwrap();
    let other_session = SettingsService::session().unwrap();
    let services = [&disk, &other, &session, &other_session];
    let ids: std::collections::HashSet<_> = services
        .iter()
        .map(|service| service.reader().snapshot().unwrap().store_id())
        .collect();
    assert_eq!(ids.len(), services.len());
    disk.apply(Command::SetColorMode(ColorMode::Dark)).unwrap();
    session.apply(Command::SetLocale("zh-Hans".into())).unwrap();
    assert_eq!(other.snapshot().unwrap().color_mode, ColorMode::System);
    assert_eq!(
        other_session.snapshot().unwrap().locale_preference,
        "system"
    );
    assert_eq!(
        session.snapshot().unwrap().problem,
        Some("settings_directory_unavailable")
    );
    let mut sub = session.reader().subscribe().unwrap();
    let id = sub.initial().store_id();
    drop(session);
    assert_eq!(ready(sub.next()).unwrap_err(), SubscriptionEnd::Closed);
    assert_ne!(
        SettingsService::session()
            .unwrap()
            .reader()
            .snapshot()
            .unwrap()
            .store_id(),
        id
    );
    assert!(
        repo.registry
            .for_path("relative/settings.toml".into())
            .is_err()
    );
}

#[test]
fn settings_legacy_file_roundtrips_without_persisting_publication_metadata() {
    let repo = Repository::new();
    repo.seed(LEGACY);
    let service = repo.service();
    assert_eq!(service.snapshot().unwrap().color_mode, ColorMode::Dark);
    assert_eq!(service.reader().snapshot().unwrap().sequence(), 0);
    assert_eq!(fs::read_to_string(&repo.path).unwrap(), LEGACY);
    service
        .apply(Command::SetColorMode(ColorMode::System))
        .unwrap();
    service
        .apply(Command::SetColorMode(ColorMode::Dark))
        .unwrap();
    assert_eq!(fs::read_to_string(&repo.path).unwrap(), LEGACY);
    let id = service.reader().snapshot().unwrap().store_id();
    service.close();
    let reopened = SettingsRegistry::default()
        .for_path(repo.path.clone())
        .unwrap();
    assert_ne!(reopened.reader().snapshot().unwrap().store_id(), id);
    assert_eq!(reopened.reader().snapshot().unwrap().sequence(), 0);
    assert_eq!(reopened.snapshot().unwrap().revision, 0);
    assert_eq!(reopened.snapshot().unwrap().color_mode, ColorMode::Dark);
}

#[test]
fn settings_future_schema_stays_protected_while_session_changes_are_published() {
    let repo = Repository::new();
    repo.seed(FUTURE);
    let service = repo.service();
    let mut sub = service.reader().subscribe().unwrap();
    let changed = service
        .apply(Command::SetColorMode(ColorMode::Light))
        .unwrap();
    assert!(!changed.saved);
    assert_eq!(changed.problem, Some("settings_unsupported_version"));
    assert_eq!(ready(sub.next()).unwrap().state(), &changed);
    assert_eq!(fs::read_to_string(&repo.path).unwrap(), FUTURE);
}

#[test]
fn settings_corrupt_source_is_preserved_before_publishing_recovered_preferences() {
    let repo = Repository::new();
    let corrupt = "this is not TOML";
    repo.seed(corrupt);
    let service = repo.service();
    assert_eq!(
        service.snapshot().unwrap().problem,
        Some("settings_invalid_recovered")
    );
    let mut sub = service.reader().subscribe().unwrap();
    let saved = service
        .apply(Command::SetColorMode(ColorMode::Dark))
        .unwrap();
    assert!(saved.saved);
    assert_eq!(ready(sub.next()).unwrap().state(), &saved);
    let backups: Vec<_> = fs::read_dir(repo.path.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("settings.toml.corrupt-")
        })
        .collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(fs::read_to_string(&backups[0]).unwrap(), corrupt);
    assert_eq!(fs::read_to_string(&repo.path).unwrap(), LEGACY);
}

#[test]
fn settings_theme_rescan_and_clone_retry_publish_the_complete_catalog() {
    let repo = Repository::new();
    let service = repo.service();
    let themes = repo.path.parent().unwrap().join("themes");
    fs::create_dir_all(&themes).unwrap();
    fs::write(themes.join("local.json"), THEME).unwrap();
    let mut sub = service.reader().subscribe().unwrap();
    let scanned = service.rescan_themes().unwrap();
    assert!(
        scanned
            .style_themes
            .iter()
            .any(|theme| theme.id == "user.integration")
    );
    assert_eq!(ready(sub.next()).unwrap().state(), &scanned);
    service.rescan_themes().unwrap();
    assert!(poll(std::pin::pin!(sub.next()), &Arc::default()).is_pending());
    fs::create_dir(&repo.path).unwrap();
    let failed = service
        .clone_style_theme("user.integration".into(), "Integration copy".into())
        .unwrap();
    assert!(!failed.saved);
    let pending = ready(sub.next()).unwrap();
    let file_count = fs::read_dir(&themes).unwrap().count();
    fs::remove_dir(&repo.path).unwrap();
    let saved = service
        .clone_style_theme("user.integration".into(), "Integration copy".into())
        .unwrap();
    let committed = ready(sub.next()).unwrap();
    assert!(saved.saved);
    assert_eq!(saved.revision, failed.revision);
    assert_eq!(saved.theme_id, failed.theme_id);
    assert_eq!(committed.sequence(), pending.sequence() + 1);
    assert_eq!(fs::read_dir(themes).unwrap().count(), file_count);
}

#[test]
fn settings_cancellation_is_local_and_close_prevents_further_file_writes() {
    let repo = Repository::new();
    let service = repo.service();
    let reader = service.reader();
    let mut cancelled = reader.subscribe().unwrap();
    let mut live = reader.subscribe().unwrap();
    cancelled.cancel();
    let changed = service
        .apply(Command::SetColorMode(ColorMode::Dark))
        .unwrap();
    service.close();
    service.close();
    assert_eq!(
        ready(cancelled.next()).unwrap_err(),
        SubscriptionEnd::Cancelled
    );
    assert_eq!(ready(live.next()).unwrap().state(), &changed);
    assert_eq!(ready(live.next()).unwrap_err(), SubscriptionEnd::Closed);
    assert_eq!(
        service.apply(Command::SetColorMode(ColorMode::Light)),
        Err(SettingsError::Closed)
    );
    assert_eq!(service.rescan_themes(), Err(SettingsError::Closed));
    assert_eq!(service.snapshot(), Err(SettingsError::Closed));
    assert_eq!(reader.snapshot().unwrap().state(), &changed);
    assert_eq!(fs::read_to_string(&repo.path).unwrap(), LEGACY);
}

#[test]
fn settings_background_assets_publish_after_persistence_and_remain_readable() {
    let repo = Repository::new();
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/background.json")).unwrap();
    let bytes = nsh_core::image::decode_data_uri(fixture["dataUrl"].as_str().unwrap())
        .unwrap()
        .1;
    let service = repo.service();
    let mut sub = service.reader().subscribe().unwrap();
    let imported = service
        .import_background_image("fixture.png".into(), bytes.clone())
        .unwrap();
    assert!(imported.saved);
    let image = imported.background_image.as_ref().unwrap();
    assert_eq!(service.load_background_image(&image.id).unwrap(), bytes);
    assert_eq!(ready(sub.next()).unwrap().state(), &imported);
    service
        .import_background_image("fixture.png".into(), bytes.clone())
        .unwrap();
    assert!(poll(std::pin::pin!(sub.next()), &Arc::default()).is_pending());
    service.close();
    let reopened = SettingsRegistry::default()
        .for_path(repo.path.clone())
        .unwrap();
    assert_eq!(reopened.load_background_image(&image.id).unwrap(), bytes);
    let mut sub = reopened.reader().subscribe().unwrap();
    let cleared = reopened.apply(Command::ClearBackgroundImage).unwrap();
    assert!(cleared.saved && cleared.background_image.is_none());
    assert_eq!(ready(sub.next()).unwrap().state(), &cleared);
    assert_eq!(
        reopened.load_background_image(&image.id),
        Err(SettingsError::Domain("background_image_unavailable"))
    );
}

#[test]
fn settings_concurrent_partitions_merge_and_survive_reopening() {
    use usher_nsh::state::settings::{DiagnosticLevel, DiagnosticOutput};
    let repo = Repository::new();
    let service = repo.service();
    let barrier = Arc::new(Barrier::new(4));
    let commands = [
        vec![
            Command::SetColorMode(ColorMode::Dark),
            Command::SetAccent(Some("#b42318".into())),
        ],
        vec![
            Command::SetLocale("zh-Hans".into()),
            Command::SetShowStatusBar(false),
        ],
        vec![
            Command::SetDiagnosticLevel(DiagnosticLevel::Debug),
            Command::SetDiagnosticOutput(DiagnosticOutput::Console, false),
        ],
        vec![
            Command::SetUpdateAutoCheck(false),
            Command::SetSidebarDocked(false),
        ],
    ];
    let workers: Vec<_> = commands
        .into_iter()
        .map(|commands| {
            let service = service.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                for command in commands {
                    assert!(service.apply(command).unwrap().saved);
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let current = service.snapshot().unwrap();
    assert_eq!(current.color_mode, ColorMode::Dark);
    assert_eq!(current.locale_preference, "zh-Hans");
    assert_eq!(current.diagnostic_level, DiagnosticLevel::Debug);
    assert!(
        !current.update_auto_check
            && !current.show_status_bar
            && !current.sidebar_docked
            && !current.diagnostic_console
    );
    // A distinct registry models a new process after all prior transactions finish.
    let restored = SettingsRegistry::default()
        .for_path(repo.path.clone())
        .unwrap()
        .snapshot()
        .unwrap();
    assert_eq!(restored.color_mode, current.color_mode);
    assert_eq!(restored.accent, current.accent);
    assert_eq!(restored.locale_preference, current.locale_preference);
    assert_eq!(restored.diagnostic_level, current.diagnostic_level);
    assert_eq!(restored.diagnostic_console, current.diagnostic_console);
    assert_eq!(restored.update_auto_check, current.update_auto_check);
    assert_eq!(restored.show_status_bar, current.show_status_bar);
    assert_eq!(restored.sidebar_docked, current.sidebar_docked);
}

#[cfg(unix)]
#[test]
fn settings_read_only_directory_preserves_pending_partitions_until_retry() {
    use std::os::unix::fs::PermissionsExt;
    struct Restore(std::path::PathBuf, fs::Permissions);
    impl Drop for Restore {
        fn drop(&mut self) {
            fs::set_permissions(&self.0, self.1.clone()).unwrap();
        }
    }
    let repo = Repository::new();
    let service = repo.service();
    assert!(
        service
            .apply(Command::SetColorMode(ColorMode::Dark))
            .unwrap()
            .saved
    );
    let original = fs::read_to_string(&repo.path).unwrap();
    let directory = repo.path.parent().unwrap();
    let restore = Restore(
        directory.to_path_buf(),
        fs::metadata(directory).unwrap().permissions(),
    );
    fs::set_permissions(directory, fs::Permissions::from_mode(0o500)).unwrap();
    let language = service.apply(Command::SetLocale("zh-Hans".into())).unwrap();
    let updates = service.apply(Command::SetUpdateAutoCheck(false)).unwrap();
    assert!(!language.saved && !updates.saved);
    assert_eq!(updates.problem, Some("settings_write_failed"));
    assert_eq!(fs::read_to_string(&repo.path).unwrap(), original);
    assert_eq!(updates.locale_preference, "zh-Hans");
    drop(restore);
    let retried = service.apply(Command::SetUpdateAutoCheck(false)).unwrap();
    assert!(retried.saved);
    assert_eq!(retried.revision, updates.revision);
    let restored = SettingsRegistry::default()
        .for_path(repo.path.clone())
        .unwrap()
        .snapshot()
        .unwrap();
    assert_eq!(restored.color_mode, ColorMode::Dark);
    assert_eq!(restored.locale_preference, "zh-Hans");
    assert!(!restored.update_auto_check);
}
