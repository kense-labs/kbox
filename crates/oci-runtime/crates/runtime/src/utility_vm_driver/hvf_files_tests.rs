use std::os::unix::fs::{MetadataExt, PermissionsExt};

use koci_agent_protocol::GuestPath;
use koci_sdk::{ContainerId, FileGrant, FileGrants, StorageAccessMode, StorageAttachmentId};
use koci_sdk::{ContainerTarget, Generation};
use serde_json::json;

use super::*;

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    share: PathBuf,
    bundle: OciBundle,
    contract: CreateAttachments,
    target: ContainerTarget,
    guest: GuestPath,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let share = root.join("runtime/g1");
        fs::create_dir_all(share.join("bundle/rootfs")).unwrap();
        fs::create_dir(root.join("caller")).unwrap();
        let mut config =
            serde_json::to_value(koci_sdk::oci_spec::runtime::Spec::default()).unwrap();
        let mut grants = Vec::new();
        let mut mounts = Vec::new();
        for (i, access) in [StorageAccessMode::ReadOnly, StorageAccessMode::ReadWrite]
            .into_iter()
            .enumerate()
        {
            let source = root.join(format!("caller/文件 {i}"));
            fs::write(&source, if i == 0 { b"initial".as_slice() } else { b"" }).unwrap();
            fs::set_permissions(&source, fs::Permissions::from_mode(0o666)).unwrap();
            let m = fs::metadata(&source).unwrap();
            mounts.push(json!({"type":"bind","source":source,"destination":format!("/exact-{i}"),"options":["bind",if i == 0 {"ro"} else {"rw"}]}));
            grants.push(
                FileGrant::new(
                    i,
                    StorageAttachmentId::new(format!("file-{i}")).unwrap(),
                    FileSourceIdentity::new(m.dev(), m.ino()).unwrap(),
                    access,
                )
                .unwrap(),
            );
        }
        fs::write(root.join("caller/canary"), b"private").unwrap();
        config["mounts"] = json!(mounts);
        config["root"] = json!({"path":"rootfs","readonly":true});
        config["annotations"] = json!({koci_sdk::FILE_GRANTS_EXTENSION:FileGrants::new(grants).unwrap().annotation_value().unwrap()});
        let bundle = OciBundle::from_json(share.join("bundle"), config.to_string()).unwrap();
        fs::write(bundle.directory().join("config.json"), bundle.config_json()).unwrap();
        fs::set_permissions(
            bundle.directory().join("config.json"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        let contract =
            CreateAttachments::from_bundle(&bundle, koci_sdk::ProcessIo::default()).unwrap();
        Self {
            _temp: temp,
            root,
            share,
            bundle,
            contract,
            target: ContainerTarget::exact(ContainerId::new("file-tests").unwrap(), Generation(1)),
            guest: GuestPath::new("/run/koci-runtime/bundle").unwrap(),
        }
    }
    fn request(&self) -> UtilityVmLaunchRequest<'_> {
        UtilityVmLaunchRequest {
            target: &self.target,
            runtime_share: &self.share,
            bundle: &self.bundle,
            guest_bundle: &self.guest,
            attachment_contract: &self.contract,
        }
    }
    fn source(&self, index: usize) -> PathBuf {
        self.root.join(format!("caller/文件 {index}"))
    }
}

#[tokio::test]
async fn live_file_transport_replays_and_cleans_only_private_aliases() {
    let f = Fixture::new();
    let before = fs::metadata(f.source(0)).unwrap();
    let digest = super::super::hvf_directory::prepare(&f.request())
        .await
        .unwrap()
        .unwrap();
    let manifest = super::super::vm_attachment_transport::read_manifest(
        &f.share.join(AGENT_VM_ATTACHMENT_MANIFEST_FILE_NAME),
    )
    .await
    .unwrap();
    assert_eq!(manifest.files().len(), 2);
    assert!(manifest.directories().is_empty());
    let files = prepare(&f.request()).await.unwrap();
    assert_eq!(manifest.files(), files);
    assert_eq!(manifest.digest().unwrap(), digest);
    let rw = files
        .iter()
        .find(|g| g.attachment().access_mode() == StorageAccessMode::ReadWrite)
        .unwrap();
    let ro = files
        .iter()
        .find(|g| g.attachment().access_mode() == StorageAccessMode::ReadOnly)
        .unwrap();
    fs::write(rw.projection_path(&f.share).join("file"), b"writeback").unwrap();
    fs::write(f.source(0), b"host live longer update").unwrap();
    assert_eq!(fs::read(f.source(1)).unwrap(), b"writeback");
    assert_eq!(
        fs::read(ro.projection_path(&f.share).join("file")).unwrap(),
        b"host live longer update"
    );
    assert!(
        prepare(&f.request()).await.is_ok(),
        "live size changes do not change identity"
    );
    cleanup(
        &f.share,
        &ContainerTarget::exact(f.target.id.clone(), Generation(2)),
    )
    .await
    .unwrap_err();
    assert_eq!(fs::metadata(f.source(0)).unwrap().nlink(), 2);
    cleanup(&f.share, &f.target).await.unwrap();
    cleanup(&f.share, &f.target).await.unwrap();
    let after = fs::metadata(f.source(0)).unwrap();
    assert_eq!(
        (
            before.dev(),
            before.ino(),
            before.uid(),
            before.gid(),
            before.mode()
        ),
        (
            after.dev(),
            after.ino(),
            after.uid(),
            after.gid(),
            after.mode()
        )
    );
    assert_eq!(after.nlink(), 1);
    assert_eq!(fs::read(f.root.join("caller/canary")).unwrap(), b"private");
    assert!(!file_projection_root(&f.share).exists());
}

#[tokio::test]
async fn source_replacement_cannot_be_reauthorized_or_deleted_by_cleanup() {
    let f = Fixture::new();
    let files = prepare(&f.request()).await.unwrap();
    let retained = f.root.join("retained");
    fs::rename(f.source(0), &retained).unwrap();
    fs::write(f.source(0), b"replacement").unwrap();
    assert_eq!(
        prepare(&f.request()).await.unwrap_err().code,
        ErrorCode::FailedPrecondition
    );
    cleanup(&f.share, &f.target).await.unwrap();
    assert_eq!(fs::read(f.source(0)).unwrap(), b"replacement");
    assert_eq!(fs::read(&retained).unwrap(), b"initial");
    assert_eq!(fs::metadata(&retained).unwrap().nlink(), 1);
    for file in files {
        assert!(!file.projection_path(&f.share).exists());
    }
}

#[tokio::test]
async fn malformed_aliases_concurrent_grants_and_partial_preparation_fail_closed() {
    let f = Fixture::new();
    let alias = f.root.join("caller/existing-alias");
    fs::hard_link(f.source(0), &alias).unwrap();
    assert!(prepare(&f.request()).await.is_err());
    assert!(!file_projection_root(&f.share).exists());
    fs::remove_file(&alias).unwrap();
    let files = prepare(&f.request()).await.unwrap();
    // A second generation cannot authorize an already projected inode.
    let mut other = f.request();
    let other_share = f.root.join("runtime/g2");
    other.runtime_share = &other_share;
    assert!(prepare(&other).await.is_err());
    assert!(!file_projection_root(&other_share).exists());
    let first = &files[0];
    fs::write(first.projection_path(&f.share).join("canary"), b"unowned").unwrap();
    assert!(prepare(&f.request()).await.is_err());
    assert!(cleanup(&f.share, &f.target).await.is_err());
    fs::remove_file(first.projection_path(&f.share).join("canary")).unwrap();
    // Model cancellation before manifest publication with one unfinished projection.
    fs::remove_file(first.projection_path(&f.share).join("file")).unwrap();
    assert!(prepare(&f.request()).await.is_err());
    cleanup(&f.share, &f.target).await.unwrap();
    for i in 0..2 {
        assert!(f.source(i).is_file());
        assert_eq!(fs::metadata(f.source(i)).unwrap().nlink(), 1);
    }
}

#[tokio::test]
async fn publication_rejects_replaced_projection_identity_and_symlink_sources() {
    let f = Fixture::new();
    super::super::hvf_directory::prepare(&f.request())
        .await
        .unwrap();
    let files = prepare(&f.request()).await.unwrap();
    let projection = files[0].projection_path(&f.share);
    let kept = f.root.join("retained-projection");
    fs::rename(&projection, &kept).unwrap();
    fs::create_dir(&projection).unwrap();
    fs::set_permissions(&projection, fs::Permissions::from_mode(0o700)).unwrap();
    fs::rename(kept.join("file"), projection.join("file")).unwrap();
    assert!(prepare(&f.request()).await.is_err());
    assert!(cleanup(&f.share, &f.target).await.is_err());
    fs::rename(projection.join("file"), kept.join("file")).unwrap();
    fs::remove_dir(&projection).unwrap();
    fs::rename(&kept, &projection).unwrap();
    cleanup(&f.share, &f.target).await.unwrap();
    let original = f.root.join("original");
    fs::rename(f.source(0), &original).unwrap();
    std::os::unix::fs::symlink(&original, f.source(0)).unwrap();
    assert!(prepare(&f.request()).await.is_err());
    assert_eq!(fs::read(&original).unwrap(), b"initial");
}

#[tokio::test]
async fn cancelled_blocking_preparation_cannot_leave_late_file_aliases() {
    let f = Fixture::new();
    let share = f.share.clone();
    let bundle = f.bundle.clone();
    let contract = f.contract.clone();
    let grants = contract.file_grants(&bundle).unwrap().unwrap();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    let job = tokio::task::spawn_blocking(move || {
        let result = prepare_blocking(&share, &bundle, &contract, &grants, None).unwrap();
        ready_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        // A detached blocking job's output is dropped when its awaiting owner was cancelled.
        drop(result);
        done_tx.send(()).unwrap();
    });
    ready_rx.await.unwrap();
    assert_eq!(
        prepare(&f.request()).await.unwrap_err().code,
        ErrorCode::Conflict
    );
    let cleanup_error = cleanup(&f.share, &f.target).await.unwrap_err();
    assert_eq!(cleanup_error.code, ErrorCode::Conflict);
    assert!(cleanup_error.retryable);
    drop(job);
    release_tx.send(()).unwrap();
    done_rx.await.unwrap();
    assert!(
        !file_projection_root(&f.share).exists(),
        "cancelled preparation published late aliases"
    );
    for i in 0..2 {
        assert_eq!(fs::metadata(f.source(i)).unwrap().nlink(), 1);
    }
}

#[tokio::test]
#[ignore = "requires a sentinel-owned native RAM filesystem, executed by the evidence harness"]
async fn native_cross_filesystem_file_grant_refuses_without_copying() {
    let ram = PathBuf::from(std::env::var_os("K_OCI_TEST_FILE_CROSSFS_ROOT").unwrap());
    assert_eq!(
        fs::read_to_string(ram.join(".koci-files-crossfs")).unwrap(),
        "owned-file-crossfs-test\n"
    );
    let caller = tempfile::tempdir_in(&ram).unwrap();
    let source = caller.path().join("文件 input");
    fs::write(&source, b"live source must not be copied").unwrap();
    let before = fs::metadata(&source).unwrap();
    let mut f = Fixture::new();
    assert_ne!(before.dev(), fs::metadata(&f.share).unwrap().dev());
    let old = f.contract.file_grants(&f.bundle).unwrap().unwrap();
    let grants = FileGrants::new(
        old.entries()
            .iter()
            .map(|g| {
                FileGrant::new(
                    g.mount_index(),
                    g.identity().clone(),
                    if g.mount_index() == 0 {
                        FileSourceIdentity::new(before.dev(), before.ino()).unwrap()
                    } else {
                        g.source_identity()
                    },
                    g.access_mode(),
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap();
    let mut config: serde_json::Value = serde_json::from_str(f.bundle.config_json()).unwrap();
    config["mounts"][0]["source"] = json!(source);
    config["annotations"][koci_sdk::FILE_GRANTS_EXTENSION] =
        json!(grants.annotation_value().unwrap());
    f.bundle = OciBundle::from_json(f.bundle.directory(), config.to_string()).unwrap();
    f.contract = CreateAttachments::from_bundle(&f.bundle, koci_sdk::ProcessIo::default()).unwrap();
    let error = prepare(&f.request()).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::Unsupported);
    assert!(!file_projection_root(&f.share).exists());
    assert_eq!(
        fs::read(&source).unwrap(),
        b"live source must not be copied"
    );
    let now = fs::metadata(&source).unwrap();
    assert_eq!(
        (
            now.dev(),
            now.ino(),
            now.uid(),
            now.gid(),
            now.mode(),
            now.nlink()
        ),
        (
            before.dev(),
            before.ino(),
            before.uid(),
            before.gid(),
            before.mode(),
            1
        )
    );
}
