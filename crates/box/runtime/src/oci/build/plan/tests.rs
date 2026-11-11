use super::*;
use kcl::SchemaDiagnosticCode;
use kbox_core::platform::Platform;

const PLAN: &str = r#"
build "oci" {
  target = "release"
  schema = "kbox.build-plan.v1"
  platform = "linux/amd64"
  network = "none"
  file = "Dockerfile"
  context = "."
  cache = "content-addressed"
}
"#;

#[test]
fn canonical_kcl_and_digest_are_stable() {
    let reordered = r#"
      # Input order and comments do not define build identity.
      build "oci" {
        cache = "content-addressed"
        context = "."
        file = "Dockerfile"
        network = "none"
        platform = "linux/amd64"
        schema = "kbox.build-plan.v1"
        target = "release"
      }
    "#;
    let expected = concat!(
        "build \"oci\" {\n",
        "  cache = \"content-addressed\"\n",
        "  context = \".\"\n",
        "  file = \"Dockerfile\"\n",
        "  network = \"none\"\n",
        "  platform = \"linux/amd64\"\n",
        "  schema = \"kbox.build-plan.v1\"\n",
        "  target = \"release\"\n",
        "}\n",
    );

    let first = BoxBuildPlan::parse_kcl(PLAN).expect("valid build plan");
    let second = BoxBuildPlan::parse_kcl(reordered).expect("valid reordered build plan");

    assert_eq!(first, second);
    assert_eq!(first.canonical_kcl().unwrap(), expected);
    assert_eq!(
        first.canonical_digest().unwrap(),
        "sha256:2bcbe1541e8b5b728a2f3d088a97e788b0aaf124ba6b0515559ca10f9617d2d4"
    );
}

#[test]
fn parser_applies_a_bounded_document_limit() {
    let oversized = format!("{PLAN}\n# {}", "x".repeat(16 * 1024));

    assert!(matches!(
        BoxBuildPlan::parse_kcl(&oversized),
        Err(BoxBuildPlanError::KclParse { .. })
    ));
}

#[test]
fn schema_is_closed_and_identity_values_are_exact() {
    let unknown = PLAN.replace(
        "  cache = \"content-addressed\"",
        "  cache = \"content-addressed\"\n  extra = \"rejected\"",
    );
    assert!(matches!(
        BoxBuildPlan::parse_kcl(&unknown),
        Err(BoxBuildPlanError::Schema {
            code: SchemaDiagnosticCode::UnknownAttribute,
            ..
        })
    ));

    let wrong_label = PLAN.replace("build \"oci\"", "build \"container\"");
    assert!(matches!(
        BoxBuildPlan::parse_kcl(&wrong_label),
        Err(BoxBuildPlanError::InvalidValue { field: "label", .. })
    ));

    let wrong_schema = PLAN.replace("kbox.build-plan.v1", "kbox.build-plan.experimental");
    assert!(matches!(
        BoxBuildPlan::parse_kcl(&wrong_schema),
        Err(BoxBuildPlanError::InvalidValue {
            field: "schema",
            ..
        })
    ));
}

#[test]
fn cache_and_network_policies_are_closed() {
    let outbound = BoxBuildPlan::parse_kcl(
        &PLAN
            .replace("network = \"none\"", "network = \"outbound\"")
            .replace("content-addressed", "disabled"),
    )
    .unwrap();
    assert_eq!(outbound.network(), BuildNetworkPolicy::Outbound);
    assert_eq!(outbound.cache(), BuildCachePolicy::Disabled);

    let bad_cache = PLAN.replace("content-addressed", "shared-directory");
    assert!(matches!(
        BoxBuildPlan::parse_kcl(&bad_cache),
        Err(BoxBuildPlanError::InvalidValue { field: "cache", .. })
    ));

    let bad_network = PLAN.replace("network = \"none\"", "network = \"host\"");
    assert!(matches!(
        BoxBuildPlan::parse_kcl(&bad_network),
        Err(BoxBuildPlanError::InvalidValue {
            field: "network",
            ..
        })
    ));
}

#[test]
fn repository_paths_and_target_are_bounded_posix_values() {
    for invalid in [
        PLAN.replace("context = \".\"", "context = \"../outside\""),
        PLAN.replace("file = \"Dockerfile\"", "file = \"/Dockerfile\""),
        PLAN.replace("file = \"Dockerfile\"", "file = \"build//Dockerfile\""),
        PLAN.replace("file = \"Dockerfile\"", "file = \"build\\\\Dockerfile\""),
        PLAN.replace("target = \"release\"", "target = \"release/escape\""),
    ] {
        assert!(
            matches!(
                BoxBuildPlan::parse_kcl(&invalid),
                Err(BoxBuildPlanError::InvalidValue { .. })
            ),
            "unsafe plan was admitted:\n{invalid}"
        );
    }
}

#[test]
fn compiles_exactly_into_the_existing_build_engine() {
    let source = tempfile::TempDir::new().unwrap();
    let context = source.path().join("service");
    let recipe = source.path().join(".kbox");
    std::fs::create_dir_all(&context).unwrap();
    std::fs::create_dir_all(&recipe).unwrap();
    std::fs::write(recipe.join("Containerfile"), "FROM scratch\n").unwrap();

    let plan = BoxBuildPlan::parse_kcl(
        r#"
build "oci" {
  schema = "kbox.build-plan.v1"
  context = "service"
  file = ".kbox/Containerfile"
  platform = "linux/arm64"
  target = "release"
  network = "none"
  cache = "disabled"
}
"#,
    )
    .unwrap();
    let options = BoxBuildOptions {
        tag: Some("registry.example/kbox/test:build".to_string()),
        quiet: true,
    };

    let config = plan.compile(source.path(), options).unwrap();

    assert_eq!(config.context_dir, context.canonicalize().unwrap());
    assert_eq!(
        config.dockerfile_path,
        recipe.join("Containerfile").canonicalize().unwrap()
    );
    assert_eq!(
        config.tag.as_deref(),
        Some("registry.example/kbox/test:build")
    );
    assert!(config.build_args.is_empty());
    assert!(config.quiet);
    assert_eq!(config.platforms, vec![Platform::linux_arm64()]);
    assert_eq!(config.target.as_deref(), Some("release"));
    assert!(config.no_cache);
    assert_eq!(config.network, BuildNetworkPolicy::None);
    assert!(config.metrics.is_none());
    assert!(config.run_pool.is_none());
}

#[test]
fn compilation_requires_an_absolute_existing_source_root() {
    let plan = BoxBuildPlan::parse_kcl(PLAN).unwrap();

    assert!(matches!(
        plan.compile(
            std::path::Path::new("relative/source"),
            BoxBuildOptions::default()
        ),
        Err(BoxBuildPlanError::InvalidSourceRoot { .. })
    ));
}

#[cfg(unix)]
#[test]
fn compilation_rejects_context_and_file_symlink_escapes() {
    use std::os::unix::fs::symlink;

    let temporary = tempfile::TempDir::new().unwrap();
    let source = temporary.path().join("source");
    let outside = temporary.path().join("outside");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(source.join("Dockerfile"), "FROM scratch\n").unwrap();
    std::fs::write(outside.join("Containerfile"), "FROM scratch\n").unwrap();

    symlink(&outside, source.join("context-link")).unwrap();
    let context_plan =
        BoxBuildPlan::parse_kcl(&PLAN.replace("context = \".\"", "context = \"context-link\""))
            .unwrap();
    assert!(matches!(
        context_plan.compile(&source, BoxBuildOptions::default()),
        Err(BoxBuildPlanError::UnsafePath {
            field: "context",
            ..
        })
    ));

    symlink(outside.join("Containerfile"), source.join("file-link")).unwrap();
    let file_plan =
        BoxBuildPlan::parse_kcl(&PLAN.replace("file = \"Dockerfile\"", "file = \"file-link\""))
            .unwrap();
    assert!(matches!(
        file_plan.compile(&source, BoxBuildOptions::default()),
        Err(BoxBuildPlanError::UnsafePath { field: "file", .. })
    ));
}
