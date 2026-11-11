use std::sync::{Arc, Mutex};

use kbox_contract::contract::{
    MountKind, RuntimeMount, RuntimeMountSource, RuntimeUnitClass, RuntimeUnitSpec,
};
use kbox_contract::{RuntimeDriver, RuntimeError};

use super::*;
use crate::kbox_contract_driver::test_support::{
    accepted, action, apply_request, fake_driver, fixture_tempdir, runtime_spec, unit,
};

struct Grant {
    unit: String,
    path: PathBuf,
    read_only: bool,
    cleanups: Mutex<Vec<String>>,
}

#[async_trait]
impl BoxDirectoryPort for Grant {
    async fn mount_path(
        &self,
        spec: &RuntimeUnitSpec,
        mount: &RuntimeMount,
    ) -> Result<PathBuf, BoxDirectoryPortError> {
        if spec.unit_id != self.unit
            || spec.generation != 1
            || !matches!(&mount.source,RuntimeMountSource::Directory{reference} if reference=="grant://workspace")
            || (self.read_only && !mount.read_only)
        {
            return Err(BoxDirectoryPortError::Rejected("not granted".into()));
        }
        Ok(self.path.clone())
    }
    async fn cleanup_spec(&self, digest: &str) -> Result<(), BoxDirectoryPortError> {
        self.cleanups.lock().unwrap().push(digest.into());
        Ok(())
    }
}

fn mount(read_only: bool) -> RuntimeMount {
    RuntimeMount {
        name: "workspace".into(),
        source: RuntimeMountSource::Directory {
            reference: "grant://workspace".into(),
        },
        target: "/workspace".into(),
        read_only,
    }
}

#[tokio::test]
async fn directory_requires_an_explicit_port_before_execution_reservation() {
    let root = fixture_tempdir();
    let (driver, backend) = fake_driver(&root);
    let mut spec = runtime_spec("directory-no-grant", 1, RuntimeUnitClass::Service);
    spec.mounts = vec![mount(false)];
    assert!(!driver
        .capabilities()
        .await
        .unwrap()
        .mount_kinds
        .contains(&MountKind::Directory));
    assert!(
        matches!(driver.apply(&apply_request(&spec),&accepted(&spec)).await,Err(RuntimeError::UnsupportedCapabilities(missing)) if missing==vec!["mount_kind:Directory"])
    );
    assert_eq!(backend.starts(), 0);
    assert!(driver.manager.managed_records().await.unwrap().is_empty());
}

#[tokio::test]
async fn directory_grant_is_live_and_teardown_does_not_require_the_source_to_remain() {
    let root = fixture_tempdir();
    let (driver, backend) = fake_driver(&root);
    let source = root.path().join("外部目录 with spaces");
    std::fs::create_dir(&source).unwrap();
    let source = source.canonicalize().unwrap();
    std::fs::write(source.join("keep"), b"caller-data").unwrap();
    let mut spec = runtime_spec("directory-grant", 1, RuntimeUnitClass::Service);
    spec.mounts = vec![mount(false)];
    let grant = Arc::new(Grant {
        unit: spec.unit_id.clone(),
        path: source.clone(),
        read_only: false,
        cleanups: Mutex::new(Vec::new()),
    });
    let driver = driver.with_directory_port(grant.clone());
    assert!(driver
        .capabilities()
        .await
        .unwrap()
        .mount_kinds
        .contains(&MountKind::Directory));
    let plan = driver.artifact_storage.prepare_plan(&spec).await.unwrap();
    assert_eq!(
        plan.volumes(),
        [format!("{}:/workspace:rw", source.display())]
    );
    assert!(plan.volume_names().is_empty());
    let running = driver
        .apply(&apply_request(&spec), &accepted(&spec))
        .await
        .unwrap();
    let moved = source.with_file_name("renamed user data");
    std::fs::rename(&source, &moved).unwrap();
    let retained = driver.artifact_storage.require_plan(&spec).await.unwrap();
    assert_eq!(retained.volumes(), plan.volumes());
    let running = unit(spec.clone(), running);
    let stopped = driver
        .stop(&running, &action("directory-stop", &spec))
        .await
        .unwrap();
    driver
        .remove(
            &unit(spec.clone(), stopped),
            &action("directory-remove", &spec),
        )
        .await
        .unwrap();
    assert_eq!(backend.kills(), 1);
    assert!(driver.manager.managed_records().await.unwrap().is_empty());
    assert_eq!(std::fs::read(moved.join("keep")).unwrap(), b"caller-data");
    assert!(grant
        .cleanups
        .lock()
        .unwrap()
        .contains(&spec.digest().unwrap()));
}

#[tokio::test]
async fn directory_grants_refuse_wrong_units_and_write_escalation() {
    let root = fixture_tempdir();
    let (driver, backend) = fake_driver(&root);
    let source = root.path().join("source");
    std::fs::create_dir(&source).unwrap();
    let mut spec = runtime_spec("directory-read-only", 1, RuntimeUnitClass::Service);
    spec.mounts = vec![mount(false)];
    let driver = driver.with_directory_port(Arc::new(Grant {
        unit: spec.unit_id.clone(),
        path: source.canonicalize().unwrap(),
        read_only: true,
        cleanups: Mutex::new(Vec::new()),
    }));
    assert!(matches!(
        driver.apply(&apply_request(&spec), &accepted(&spec)).await,
        Err(RuntimeError::InvalidRequest(_))
    ));
    spec.mounts[0].read_only = true;
    assert!(driver
        .artifact_storage
        .prepare_plan(&spec)
        .await
        .unwrap()
        .volumes()[0]
        .ends_with(":/workspace:ro"));
    spec.unit_id = "different-unit".into();
    assert!(matches!(
        driver.apply(&apply_request(&spec), &accepted(&spec)).await,
        Err(RuntimeError::InvalidRequest(_))
    ));
    assert_eq!(backend.starts(), 0);
    assert!(driver.manager.managed_records().await.unwrap().is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn directory_source_must_be_canonical_and_plain_when_acquired() {
    let root = fixture_tempdir();
    let (driver, _) = fake_driver(&root);
    let source = root.path().join("source");
    std::fs::create_dir(&source).unwrap();
    let link = root.path().join("link");
    std::os::unix::fs::symlink(&source, &link).unwrap();
    let mut spec = runtime_spec("directory-symlink", 1, RuntimeUnitClass::Service);
    spec.mounts = vec![mount(false)];
    let driver = driver.with_directory_port(Arc::new(Grant {
        unit: spec.unit_id.clone(),
        path: link,
        read_only: false,
        cleanups: Mutex::new(Vec::new()),
    }));
    assert!(matches!(
        driver.artifact_storage.prepare_plan(&spec).await,
        Err(RuntimeError::InvalidRequest(_))
    ));
}
