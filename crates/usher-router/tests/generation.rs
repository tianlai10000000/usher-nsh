//! Catalog-driven generated contracts must remain deterministic, complete and fresh.
mod support;
use usher_router::Router;
#[test]
fn generation_matches_reviewed_contract_and_covers_every_page_and_route() {
    let router = Router::new(support::catalog(), &support::Reference).unwrap();
    assert_eq!(
        router.typescript(),
        include_str!("fixtures/routes.generated.ts")
    );
    for route in &router.catalog().routes {
        assert!(
            router
                .typescript()
                .contains(&serde_json::to_string(&route.id).unwrap())
        );
    }
    for page in &router.catalog().pages {
        assert!(
            router
                .typescript()
                .contains(&serde_json::to_string(page).unwrap())
        );
    }
}
#[test]
fn generation_rejects_unknown_schema_fields_and_invalid_page_bindings() {
    let mut input: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/catalog.json")).unwrap();
    input["undeclared"] = true.into();
    assert!(serde_json::from_value::<usher_router::Catalog>(input).is_err());
    let mut catalog = support::catalog();
    catalog.pages.retain(|key| key != "about");
    assert!(Router::new(catalog, &support::Reference).is_err());
}
#[cfg(feature = "matcher-matchit")]
#[test]
fn generation_cli_check_rejects_stale_or_missing_output_without_writing_it() {
    use std::{fs, process::Command};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let directory =
        std::env::temp_dir().join(format!("usher-route-generation-{}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(directory.clone());
    let output = directory.join("generated.ts");
    let check = || {
        Command::new(env!("CARGO_BIN_EXE_usher-routes"))
            .arg("check")
            .arg(root.join("tests/fixtures/catalog.json"))
            .arg(&output)
            .output()
            .unwrap()
    };
    assert!(!check().status.success());
    assert!(!output.exists());
    fs::write(&output, "stale").unwrap();
    assert!(!check().status.success());
    assert_eq!(fs::read_to_string(&output).unwrap(), "stale");
    fs::write(&output, include_str!("fixtures/routes.generated.ts")).unwrap();
    assert!(check().status.success());
}
