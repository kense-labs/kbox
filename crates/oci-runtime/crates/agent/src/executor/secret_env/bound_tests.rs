use super::*;
use serde_json::json;
use std::os::fd::AsRawFd;
use std::os::unix::fs::symlink;

fn fixture() -> (
    tempfile::TempDir,
    Vec<String>,
    Vec<MemorySecretEnvironmentSource>,
    PathBuf,
) {
    let root = tempfile::tempdir().unwrap();
    let guest = format!("/.kbox-secrets/{}/000.secret", "a".repeat(64));
    let source = root.path().join(guest.trim_start_matches('/'));
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(&source, b"synthetic-memory-env-alpha").unwrap();
    std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o400)).unwrap();
    std::fs::hard_link(&source, root.path().join("exact-runtime-alias")).unwrap();
    let binding = json!({"variable":"SYNTHETIC_ENV","path":guest});
    let env = vec![format!("K_BOX_SECRET_ENV_V1={}", json!([binding]))];
    let sources =
        serde_json::from_value(json!([{"binding":binding,"protectedSource":source}])).unwrap();
    (root, env, sources, source)
}

#[test]
fn bound_environment_keeps_pinned_identity_cloexec_and_legacy_alias_refusal() {
    let (root, mut env, sources, source) = fixture();
    assert!(super::super::materialize_from(&mut env.clone(), root.path()).is_err());
    let prepared = PreparedMemorySecretEnvironment::prepare(&env, &sources).unwrap();
    if let PreparedMemorySecretEnvironment::Bound { files, .. } = &prepared {
        assert_eq!(files.len(), 1);
        // SAFETY: the live owned source descriptor is inspected without modification.
        assert_ne!(
            unsafe { libc::fcntl(files[0].1.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
            0
        );
    } else {
        panic!("missing exact binding")
    }
    let metadata = serde_json::to_string(&sources).unwrap();
    assert!(!metadata.contains("synthetic-memory-env-alpha"));
    assert!(!format!("{prepared:?}").contains("synthetic-memory-env-alpha"));
    let retained = root.path().join("retained");
    std::fs::rename(&source, &retained).unwrap();
    let other = root.path().join("ungranted");
    std::fs::write(&other, b"ungranted-replacement").unwrap();
    symlink(&other, &source).unwrap();
    prepared.materialize(&mut env).unwrap();
    assert!(env == ["SYNTHETIC_ENV=synthetic-memory-env-alpha"]);
    assert!(!env
        .iter()
        .any(|entry| entry.starts_with("K_BOX_SECRET_ENV_V1")));
    assert!(std::fs::read(other).unwrap() == b"ungranted-replacement");
}

#[test]
fn bound_environment_refuses_retarget_conflicts_ambiguity_and_extra_aliases() {
    let (root, env, sources, source) = fixture();
    let retarget = vec![env[0].replace(&"a".repeat(64), &"b".repeat(64))];
    assert_eq!(
        PreparedMemorySecretEnvironment::prepare(&retarget, &sources)
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    );
    let mut conflict = env.clone();
    conflict.push("SYNTHETIC_ENV=untrusted".into());
    assert!(PreparedMemorySecretEnvironment::prepare(&conflict, &sources).is_err());
    assert!(requested_sources(&retarget, &sources).is_err());
    assert!(requested_sources(&conflict, &sources).is_err());
    let mut missing = serde_json::to_value(&sources).unwrap();
    missing[0]["protectedSource"] = json!(root.path().join("not-opened-yet"));
    let missing: Vec<MemorySecretEnvironmentSource> = serde_json::from_value(missing).unwrap();
    assert!(requested_sources(&env, &missing).unwrap().is_some());
    assert!(PreparedMemorySecretEnvironment::prepare(&env, &missing).is_err());
    let mut duplicate = sources.clone();
    duplicate.extend(sources.clone());
    assert!(PreparedMemorySecretEnvironment::prepare(&env, &duplicate).is_err());
    let prepared = PreparedMemorySecretEnvironment::prepare(&env, &sources).unwrap();
    std::fs::hard_link(&source, root.path().join("ungranted-alias")).unwrap();
    let mut unchanged = env.clone();
    assert!(prepared.materialize(&mut unchanged).is_err());
    assert_eq!(unchanged, env);
}

#[test]
fn bound_environment_preserves_omission_and_rechecks_mode_and_value_bytes() {
    let (_root, env, sources, source) = fixture();
    let prepared = PreparedMemorySecretEnvironment::prepare(&[], &sources).unwrap();
    let mut empty = Vec::new();
    prepared.materialize(&mut empty).unwrap();
    assert!(empty.is_empty());
    for bytes in [b"nul\0byte".as_slice(), b"\xff", b""] {
        std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::write(&source, bytes).unwrap();
        std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o400)).unwrap();
        match PreparedMemorySecretEnvironment::prepare(&env, &sources) {
            Ok(prepared) => assert!(prepared.materialize(&mut env.clone()).is_err()),
            Err(error) => assert_eq!(error.code, ErrorCode::FailedPrecondition),
        }
    }
    std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::write(&source, b"bounded").unwrap();
    assert!(PreparedMemorySecretEnvironment::prepare(&env, &sources).is_err());
}
