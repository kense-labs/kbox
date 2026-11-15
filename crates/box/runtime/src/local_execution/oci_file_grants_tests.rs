use super::*;
use kbox_core::{BoxConfig, CreateExecutionRequest, ExecutionId, OperationId};

fn record(root: &Path) -> BoxRecord {
    std::fs::write(root.join("文件 input"), b"initial").unwrap();
    super::super::record::build_managed_record(
        root,
        &ExecutionId::new("00000000-0000-4000-8000-000000000009").unwrap(),
        OperationId::new("file-reservation").unwrap(),
        CreateExecutionRequest {
            external_sandbox_id: "file-reservation".into(),
            config: BoxConfig {
                image: "alpine:latest".into(),
                volumes: vec![format!(
                    "{}:/exact/file:ro",
                    root.join("文件 input").display()
                )],
                ..Default::default()
            },
            labels: Default::default(),
            policy: Default::default(),
            rootfs_snapshot_id: None,
        },
        chrono::Utc::now(),
    )
    .unwrap()
}

#[test]
fn file_resource_plan_persists_incarnation_and_refuses_replacement_or_replanning() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let mut record = record(&root);
    assert!(validate_bound(&record).is_err());
    let files = capture(&record).unwrap();
    assert_eq!(files.len(), 1);
    super::super::LocalExecutionResourcePlan {
        caller_files: files.clone(),
        ..Default::default()
    }
    .apply_to(&mut record)
    .unwrap();
    validate_bound(&record).unwrap();
    let mut restored: BoxRecord =
        serde_json::from_slice(&serde_json::to_vec(&record).unwrap()).unwrap();
    assert_eq!(
        restored.managed_execution.as_ref().unwrap().caller_files,
        files
    );
    std::fs::write(root.join("文件 input"), b"live longer content").unwrap();
    validate_bound(&restored).unwrap();
    assert!(super::super::LocalExecutionResourcePlan {
        caller_files: files,
        ..Default::default()
    }
    .apply_to(&mut restored)
    .is_err());
    std::fs::rename(root.join("文件 input"), root.join("retained")).unwrap();
    std::fs::write(root.join("文件 input"), b"replacement").unwrap();
    assert!(validate_bound(&restored).is_err());
    assert_eq!(
        std::fs::read(root.join("retained")).unwrap(),
        b"live longer content"
    );
    assert_eq!(
        std::fs::read(root.join("文件 input")).unwrap(),
        b"replacement"
    );
}

#[test]
fn file_resource_planning_rejects_aliases_without_changing_caller_data() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let record = record(&root);
    let source = root.join("文件 input");
    std::fs::hard_link(&source, root.join("hardlink")).unwrap();
    assert!(capture(&record).is_err());
    std::fs::remove_file(root.join("hardlink")).unwrap();
    std::fs::rename(&source, root.join("retained")).unwrap();
    std::os::unix::fs::symlink(root.join("retained"), &source).unwrap();
    assert!(observe(&source, true).is_err());
    assert_eq!(std::fs::read(root.join("retained")).unwrap(), b"initial");
}
