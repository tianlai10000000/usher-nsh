//! Identical public consumers exercise each matcher and immutable history semantics.
mod support;
use std::collections::BTreeMap;
use support::{Reference, cases, catalog};
use usher_router::{
    Error, NavigationCommand as Command, NavigationResult as ResultKind, Router, Target,
    matcher::MatcherFactory,
};

fn resolve_cases(factory: &dyn MatcherFactory) {
    let router = Router::new(catalog(), factory).unwrap();
    for case in cases().cases {
        let found = router.resolve(&Target::path(case.target)).unwrap();
        assert_eq!(found.id, case.route, "{}", case.case_id);
        assert_eq!(found.url(), case.url, "{}", case.case_id);
        assert_eq!(router.resolve(&Target::path(found.url())).unwrap(), found);
    }
}
fn invalid_targets(factory: &dyn MatcherFactory) {
    let router = Router::new(catalog(), factory).unwrap();
    for input in [
        "https://example.com",
        "//host/path",
        "/users/%",
        "/users/%FF",
        "/users/%2F",
        "/users/%5C",
        "/users/..",
        "/users/x#fragment",
        "/users/x?page=abc",
        "/users/x?page=1&page=2",
        "/users/x?extra=1",
        "/users/x?tag=%00",
    ] {
        assert!(router.resolve(&Target::path(input)).is_err(), "{input}");
    }
    for (id, params) in [
        ("missing", BTreeMap::new()),
        ("user", BTreeMap::new()),
        ("user", BTreeMap::from([("id".into(), "a/b".into())])),
        ("user", BTreeMap::from([("id".into(), "new".into())])),
    ] {
        assert!(
            router
                .resolve(&Target::Named {
                    id: id.into(),
                    params,
                    query: BTreeMap::new()
                })
                .is_err()
        );
    }
    assert_eq!(router.resolve(&Target::path("/files/")).unwrap().id, None);
}
fn catalog_rejection(factory: &dyn MatcherFactory) {
    let mut invalid = Vec::new();
    let mut c = catalog();
    c.schema_version += 1;
    invalid.push(c);
    let mut c = catalog();
    c.routes.push(c.routes[0].clone());
    invalid.push(c);
    let mut c = catalog();
    c.routes[1].path = c.routes[0].path.clone();
    invalid.push(c);
    let mut c = catalog();
    c.routes[1].parent = Some("absent".into());
    invalid.push(c);
    let mut c = catalog();
    c.routes[0].parent = Some("about".into());
    invalid.push(c);
    let mut c = catalog();
    c.routes[0].page = "absent".into();
    invalid.push(c);
    let mut c = catalog();
    c.initial = "user".into();
    invalid.push(c);
    let mut c = catalog();
    c.routes[4].params.clear();
    invalid.push(c);
    let mut c = catalog();
    c.routes[4].path = "/users/{id}.json".into();
    invalid.push(c);
    let mut c = catalog();
    c.routes[5].path = "/files/{*rest}/end".into();
    invalid.push(c);
    let mut c = catalog();
    c.routes[4].path = "/{id}/{id}".into();
    invalid.push(c);
    let mut c = catalog();
    c.routes[1].path = "/about/%41".into();
    invalid.push(c);
    let mut c = catalog();
    c.routes[1].path = "/about//x".into();
    invalid.push(c);
    let mut c = catalog();
    c.routes[1].id = "".into();
    invalid.push(c);
    for c in invalid {
        assert!(Router::new(c, factory).is_err());
    }
    // An invalid replacement never damages an already assembled router.
    assert_eq!(
        Router::new(catalog(), factory)
            .unwrap()
            .initial()
            .unwrap()
            .current()
            .location
            .id
            .as_deref(),
        Some("home")
    );
}
fn history_contract(factory: &dyn MatcherFactory) {
    let router = Router::new(catalog(), factory).unwrap();
    let initial = router.initial().unwrap();
    let push = |s, path| {
        router
            .transition(s, &Command::Push(Target::path(path)))
            .unwrap()
            .state
    };
    let one = push(&initial, "/about");
    assert_eq!(initial.entries().len(), 1);
    assert_eq!(one.entries().len(), 2);
    assert_eq!(one.current().location.ancestors, ["home", "about"]);
    let duplicate = router
        .transition(&one, &Command::Push(Target::path("/about")))
        .unwrap();
    assert_eq!(duplicate.result, ResultKind::Duplicated);
    assert_eq!(duplicate.state, one);
    let two = push(&one, "/users/alice");
    let back = router.transition(&two, &Command::Go(-1)).unwrap().state;
    let branch = push(&back, "/files/a/b");
    assert_eq!(branch.entries().len(), 3);
    assert!(!branch.can_go_forward());
    assert_ne!(branch.current().key, two.current().key);
    let replaced = router
        .transition(&branch, &Command::Replace(Target::path("/about/")))
        .unwrap()
        .state;
    assert_eq!(replaced.entries().len(), branch.entries().len());
    assert_ne!(replaced.current().key, branch.current().key);
    assert_eq!(
        router
            .transition(&initial, &Command::Go(i64::MIN))
            .unwrap()
            .state,
        initial
    );
    assert_eq!(
        router
            .transition(&two, &Command::Go(i64::MAX))
            .unwrap()
            .state,
        two
    );
    let forward = router.transition(&back, &Command::Go(1)).unwrap().state;
    assert_eq!(forward.current(), two.current());
    let not_found = router
        .transition(&two, &Command::Push(Target::path("/unknown")))
        .unwrap();
    assert_eq!(not_found.result, ResultKind::NotFound);
    assert_eq!(not_found.state.current().location.page, "not-found");
    assert_eq!(
        router.transition(&two, &Command::Push(Target::path("external"))),
        Err(Error::InvalidTarget)
    );
    assert_eq!(two.entries().len(), 3);
}
fn migration_contract(factory: &dyn MatcherFactory) {
    let router = Router::new(catalog(), factory).unwrap();
    let initial = router.initial().unwrap();
    let first = router
        .transition(&initial, &Command::Push(Target::path("/about")))
        .unwrap()
        .state;
    let last = router
        .transition(&first, &Command::Push(Target::path("/users/alice?page=1")))
        .unwrap()
        .state;
    let old = router.transition(&last, &Command::Go(-1)).unwrap().state;
    let mut replacement = catalog();
    replacement.identity.push_str("-replacement");
    replacement
        .routes
        .iter_mut()
        .find(|r| r.id == "about")
        .unwrap()
        .path = "/moved".into();
    replacement
        .routes
        .iter_mut()
        .find(|r| r.id == "user")
        .unwrap()
        .query
        .clear();
    let router = Router::new(replacement, factory).unwrap();
    let migrated = router.migrate_history(&old).unwrap();
    assert_eq!(migrated.cursor(), old.cursor());
    assert_eq!(migrated.revision(), old.revision() + 1);
    for (before, after) in old.entries().iter().zip(migrated.entries()) {
        assert_eq!(before.key, after.key);
        assert_eq!(before.location.url(), after.location.url());
    }
    assert_eq!(migrated.current().location.id, None);
    assert_eq!(migrated.entries()[2].location.id, None);
    let forward = router.transition(&migrated, &Command::Go(1)).unwrap();
    assert_eq!(forward.result, ResultKind::NotFound);
    let next = router
        .transition(&forward.state, &Command::Push(Target::path("/moved")))
        .unwrap()
        .state;
    assert!(
        old.entries()
            .iter()
            .all(|entry| entry.key != next.current().key)
    );
    assert_eq!(old.current().location.id.as_deref(), Some("about"));
}
macro_rules! contracts {
    ($name:ident, $factory:expr) => {
        mod $name {
            use super::*;
            #[test]
            fn resolution() {
                resolve_cases(&$factory);
            }
            #[test]
            fn rejects_invalid_targets() {
                invalid_targets(&$factory);
            }
            #[test]
            fn rejects_invalid_catalogs() {
                catalog_rejection(&$factory);
            }
            #[test]
            fn catalog_migration() {
                migration_contract(&$factory);
            }
            #[test]
            fn immutable_history() {
                history_contract(&$factory);
            }
        }
    };
}
contracts!(reference, Reference);
#[cfg(feature = "matcher-matchit")]
contracts!(matchit, usher_router::matcher::Matchit);

#[test]
fn backend_failures_remain_stable_and_cannot_forge_a_route() {
    use usher_router::matcher::{Match, Pattern, RouteMatcher};
    struct Switchable {
        failed: std::sync::Arc<std::sync::atomic::AtomicBool>,
        matcher: Box<dyn RouteMatcher>,
    }
    impl MatcherFactory for Switchable {
        fn build(&self, patterns: &[Pattern]) -> Result<Box<dyn RouteMatcher>, Error> {
            Ok(Box::new(Self {
                failed: self.failed.clone(),
                matcher: Reference.build(patterns)?,
            }))
        }
    }
    impl RouteMatcher for Switchable {
        fn recognize(&self, path: &str) -> Result<Option<Match>, Error> {
            if self.failed.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(Error::BackendUnavailable);
            }
            self.matcher.recognize(path)
        }
    }
    let failed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let router = Router::new(
        catalog(),
        &Switchable {
            failed: failed.clone(),
            matcher: Reference.build(&[]).unwrap(),
        },
    )
    .unwrap();
    let history = router.initial().unwrap();
    let before = history.clone();
    failed.store(true, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(
        router.migrate_history(&history),
        Err(Error::BackendUnavailable)
    );
    assert_eq!(history, before);
    struct Broken;
    impl MatcherFactory for Broken {
        fn build(&self, _: &[Pattern]) -> Result<Box<dyn RouteMatcher>, Error> {
            Err(Error::BackendUnavailable)
        }
    }
    assert!(matches!(
        Router::new(catalog(), &Broken),
        Err(Error::BackendUnavailable)
    ));
    struct Forged;
    impl RouteMatcher for Forged {
        fn recognize(&self, _: &str) -> Result<Option<Match>, Error> {
            Ok(Some(Match {
                route: usize::MAX,
                params: Default::default(),
            }))
        }
    }
    impl MatcherFactory for Forged {
        fn build(&self, _: &[Pattern]) -> Result<Box<dyn RouteMatcher>, Error> {
            Ok(Box::new(Forged))
        }
    }
    assert!(matches!(
        Router::new(catalog(), &Forged),
        Err(Error::BackendUnavailable)
    ));
}

#[test]
fn properties_round_trip_and_history_invariants() {
    use proptest::{
        prelude::*,
        test_runner::{Config, RngSeed, TestRunner},
    };
    let data = cases();
    let config = Config {
        cases: data.property_cases,
        rng_seed: RngSeed::Fixed(data.property_seed),
        source_file: Some(file!()),
        ..Config::default()
    };
    let mut runner = TestRunner::new(config);
    let router = Router::new(catalog(), &Reference).unwrap();
    runner
        .run(
            &(
                "[a-zA-Z0-9_你好%]{1,30}"
                    .prop_filter("static route shadows this parameter", |name| name != "new"),
                proptest::collection::vec(0u8..5, 0..80),
            ),
            |(name, actions)| {
                let target = Target::Named {
                    id: "user".into(),
                    params: BTreeMap::from([("id".into(), name)]),
                    query: Default::default(),
                };
                let resolved = router.resolve(&target).unwrap();
                prop_assert_eq!(
                    router.resolve(&Target::path(resolved.url())).unwrap(),
                    resolved
                );
                let mut state = router.initial().unwrap();
                for action in actions {
                    let command = match action {
                        0 => Command::Push(Target::path("/about")),
                        1 => Command::Push(target.clone()),
                        2 => Command::Replace(Target::path("/")),
                        3 => Command::Go(-1),
                        _ => Command::Go(1),
                    };
                    let before = state.clone();
                    state = router.transition(&state, &command).unwrap().state;
                    prop_assert!(state.cursor() < state.entries().len());
                    prop_assert_eq!(
                        state
                            .entries()
                            .iter()
                            .map(|entry| &entry.key)
                            .collect::<std::collections::BTreeSet<_>>()
                            .len(),
                        state.entries().len()
                    );
                    if matches!(command, Command::Replace(_)) {
                        prop_assert_eq!(state.entries().len(), before.entries().len());
                    }
                }
                Ok(())
            },
        )
        .unwrap();
}
