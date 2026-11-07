use std::collections::HashMap;

use kbox_core::compose::{normalize_compose, ComposeDiagnosticCode, ComposeSourceFormat};

const YAML_FIXTURE: &str = include_str!("fixtures/compose/canonical.yaml");
const ACL_FIXTURE: &str = include_str!("fixtures/compose/canonical.kcl");
const NORMALIZED_FIXTURE: &str = include_str!("fixtures/compose/canonical.normalized.json");

fn environment() -> HashMap<String, String> {
    HashMap::from([(
        "API_IMAGE".to_string(),
        "ghcr.io/kbox/api:latest".to_string(),
    )])
}

#[test]
fn yaml_and_kcl_normalize_to_the_same_golden_document() {
    let environment = environment();
    let yaml = normalize_compose(YAML_FIXTURE, ComposeSourceFormat::Yaml, &environment)
        .expect("normalize YAML fixture");
    let kcl = normalize_compose(ACL_FIXTURE, ComposeSourceFormat::Kcl, &environment)
        .expect("normalize KCL fixture");

    assert_eq!(yaml, kcl);
    assert_eq!(
        yaml.to_canonical_json().expect("serialize normalized YAML"),
        NORMALIZED_FIXTURE.replace("\r\n", "\n"),
        "golden JSON must compare by content regardless of Git's checkout line endings"
    );
    assert_eq!(
        yaml.service_order().expect("deterministic service order"),
        ["db", "api"]
    );
}

#[test]
fn normalization_is_stable_across_repeated_runs() {
    let environment = environment();
    let expected = normalize_compose(YAML_FIXTURE, ComposeSourceFormat::Yaml, &environment)
        .expect("normalize fixture")
        .to_canonical_json()
        .expect("serialize fixture");

    for _ in 0..32 {
        let actual = normalize_compose(YAML_FIXTURE, ComposeSourceFormat::Yaml, &environment)
            .expect("normalize fixture")
            .to_canonical_json()
            .expect("serialize fixture");
        assert_eq!(actual, expected);
    }
}

#[test]
fn yaml_unknown_fields_return_sorted_structured_diagnostics() {
    let source = r#"
name: silently-ignored-before
services:
  db:
    image: postgres:17
  api:
    image: api:latest
    build: .
    deploy:
      replicas: 2
    depends_on:
      db:
        restart: true
    networks:
      backend:
        priority: 10
    healthcheck:
      test: ["CMD", "true"]
      grace_period: 5s
volumes:
  data:
    external: true
networks:
  backend:
    external: true
"#;

    let error = normalize_compose(source, ComposeSourceFormat::Yaml, &HashMap::new())
        .expect_err("unsupported fields must fail normalization");
    let diagnostics = error.diagnostics();

    assert!(diagnostics
        .iter()
        .all(|diagnostic| diagnostic.code == ComposeDiagnosticCode::UnsupportedField));
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.path.as_str())
            .collect::<Vec<_>>(),
        [
            "/name",
            "/networks/backend/external",
            "/services/api/build",
            "/services/api/depends_on/db/restart",
            "/services/api/deploy",
            "/services/api/healthcheck/grace_period",
            "/services/api/networks/backend/priority",
            "/volumes/data/external",
        ]
    );
}

#[test]
fn kcl_unknown_fields_return_a_structured_diagnostic() {
    let error = normalize_compose(
        r#"service "api" { image = "api:latest" build = "." }"#,
        ComposeSourceFormat::Kcl,
        &HashMap::new(),
    )
    .expect_err("unsupported KCL field must fail normalization");

    assert_eq!(error.diagnostics().len(), 1);
    assert_eq!(
        error.diagnostics()[0].code,
        ComposeDiagnosticCode::UnsupportedField
    );
    assert_eq!(error.diagnostics()[0].path, "/services/api/build");
    assert_eq!(
        serde_json::to_value(&error.diagnostics()[0]).unwrap()["code"],
        "compose.unsupported_field"
    );
}

#[test]
fn unsupported_driver_values_return_structured_diagnostics() {
    let error = normalize_compose(
        r#"
services:
  api:
    image: api:latest
volumes:
  data:
    driver: nfs
networks:
  backend:
    driver: overlay
"#,
        ComposeSourceFormat::Yaml,
        &HashMap::new(),
    )
    .expect_err("unsupported drivers must fail normalization");

    assert_eq!(
        error
            .diagnostics()
            .iter()
            .map(|diagnostic| (&diagnostic.code, diagnostic.path.as_str()))
            .collect::<Vec<_>>(),
        [
            (
                &ComposeDiagnosticCode::UnsupportedValue,
                "/networks/backend/driver",
            ),
            (
                &ComposeDiagnosticCode::UnsupportedValue,
                "/volumes/data/driver",
            ),
        ]
    );
}

#[test]
fn unsupported_multi_network_runtime_shape_fails_before_lifecycle_work() {
    let error = normalize_compose(
        r#"
services:
  api:
    image: api:latest
    networks: [frontend, backend]
"#,
        ComposeSourceFormat::Yaml,
        &HashMap::new(),
    )
    .expect_err("a service cannot be partially attached to multiple networks");

    assert_eq!(error.diagnostics().len(), 1);
    assert_eq!(
        error.diagnostics()[0].code,
        ComposeDiagnosticCode::UnsupportedValue
    );
    assert_eq!(error.diagnostics()[0].path, "/services/api/networks");
}

#[test]
fn kcl_transient_secret_references_never_resolve_into_configuration() {
    let source = r#"
service "api" {
  image = "api:latest"
  secret_environment = {
    DATABASE_URL = "DATABASE_URL_SECRET"
  }
}
"#;
    let plaintext = "postgres://must-not-enter-compose-data";
    let environment = HashMap::from([("DATABASE_URL_SECRET".to_string(), plaintext.to_string())]);

    let normalized = normalize_compose(source, ComposeSourceFormat::Kcl, &environment)
        .expect("normalize transient Secret reference");
    assert_eq!(
        normalized.services["api"].secret_environment["DATABASE_URL"],
        "DATABASE_URL_SECRET"
    );
    assert!(!normalized.to_canonical_json().unwrap().contains(plaintext));
}

#[test]
fn transient_secret_references_reject_value_expansion_and_literal_collisions() {
    for source in [
        r#"service "api" {
  image = "api:latest"
  secret_environment = { TOKEN = env("TOKEN_SECRET") }
}"#,
        r#"service "api" {
  image = "api:latest"
  secret_environment = { TOKEN = "${TOKEN_SECRET}" }
}"#,
        r#"service "api" {
  image = "api:latest"
  environment = { TOKEN = "literal" }
  secret_environment = { TOKEN = "TOKEN_SECRET" }
}"#,
    ] {
        let error = normalize_compose(
            source,
            ComposeSourceFormat::Kcl,
            &HashMap::from([("TOKEN_SECRET".to_string(), "plaintext".to_string())]),
        )
        .expect_err("Secret references must remain non-sensitive names");
        assert!(!error.to_string().contains("plaintext"));
    }
}
