use super::*;
use koci_sdk::{
    CreateAttachments, ProcessIo, StorageAttachmentId, StorageCleanup, StorageOwnership,
};
use serde_json::json;
use std::os::unix::fs::PermissionsExt;

fn fixture(access: StorageAccessMode) -> (tempfile::TempDir, PathBuf, AgentVmStorageAttachment) {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary
        .path()
        .canonicalize()
        .unwrap()
        .join("卷 image.ext4");
    File::create(&path)
        .unwrap()
        .set_len(16 * 1024 * 1024)
        .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let mut config = serde_json::to_value(koci_sdk::oci_spec::runtime::Spec::default()).unwrap();
    config["mounts"] = json!([{"type":"ext4","source":path,"destination":"/data","options":[if access==StorageAccessMode::ReadOnly {"ro"}else{"rw"}]}]);
    let bundle = OciBundle::from_json(temporary.path().join("bundle"), config.to_string()).unwrap();
    let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default())
        .unwrap()
        .attach_storage_mount(
            &bundle,
            0,
            StorageAttachmentId::new("raw").unwrap(),
            access,
            StorageOwnership::Caller,
            StorageCleanup::DetachOnly,
        )
        .unwrap();
    let metadata = fs::metadata(&path).unwrap();
    let a = &attachments.storage()[0];
    let grant = AgentVmStorageAttachment::new(
        a.identity().clone(),
        a.mount().clone(),
        access,
        a.ownership(),
        a.cleanup(),
        path.to_str().unwrap(),
        AgentVmStorageSourceIdentity::new(
            metadata.dev(),
            metadata.rdev(),
            metadata.ino(),
            metadata.len(),
        )
        .unwrap(),
        &attachments.digest().unwrap(),
    )
    .unwrap();
    (temporary, path, grant)
}

#[test]
fn raw_pin_keeps_exact_descriptor_access_identity_and_private_unaliased_source() {
    for mode in [StorageAccessMode::ReadOnly, StorageAccessMode::ReadWrite] {
        let (_temporary, path, grant) = fixture(mode);
        let pin = MacosStorageAttachment::open(grant.clone()).unwrap();
        let descriptor: i32 = pin
            .disk()
            .loader_path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        // SAFETY: F_GETFL only reads flags of this live owned descriptor.
        let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
        assert_eq!(
            flags & libc::O_ACCMODE,
            if mode == StorageAccessMode::ReadOnly {
                libc::O_RDONLY
            } else {
                libc::O_RDWR
            }
        );
        fs::hard_link(&path, path.with_file_name("alias")).unwrap();
        assert!(pin.reverify().is_err());
        fs::remove_file(path.with_file_name("alias")).unwrap();
        let original = path.with_file_name("original");
        fs::rename(&path, &original).unwrap();
        File::create(&path)
            .unwrap()
            .set_len(16 * 1024 * 1024)
            .unwrap();
        assert!(pin.reverify().is_err());
        assert!(MacosStorageAttachment::open(grant).is_err());
        assert!(original.exists());
    }
}

#[test]
fn raw_ro_pin_refuses_inherited_writers_and_changed_capacity() {
    let (_temporary, path, grant) = fixture(StorageAccessMode::ReadOnly);
    let writer = OpenOptions::new().write(true).open(&path).unwrap();
    assert!(MacosStorageAttachment::open(grant.clone()).is_err());
    drop(writer);
    let pin = MacosStorageAttachment::open(grant.clone()).unwrap();
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(8 * 1024 * 1024)
        .unwrap();
    assert!(pin.reverify().is_err());
    assert!(MacosStorageAttachment::open(grant).is_err());
}

#[test]
fn raw_disk_pins_serialize_writers_and_allow_only_shared_readers() {
    let (_temporary, _path, writer_grant) = fixture(StorageAccessMode::ReadWrite);
    let writer = MacosStorageAttachment::open(writer_grant.clone()).unwrap();
    assert!(MacosStorageAttachment::open(writer_grant.clone()).is_err());
    drop(writer);
    MacosStorageAttachment::open(writer_grant).unwrap();
    let (_temporary, _path, reader_grant) = fixture(StorageAccessMode::ReadOnly);
    let first = MacosStorageAttachment::open(reader_grant.clone()).unwrap();
    let second = MacosStorageAttachment::open(reader_grant).unwrap();
    drop((first, second));
}

#[test]
fn raw_storage_host_sandbox_denies_ro_mutations_and_preserves_exact_rw_authority() {
    let (temporary, path, grant) = fixture(StorageAccessMode::ReadOnly);
    let root = temporary.path().canonicalize().unwrap();
    fs::create_dir(root.join("runtime")).unwrap();
    fs::write(
        root.join("runtime/grant.json"),
        serde_json::to_vec(&grant).unwrap(),
    )
    .unwrap();
    fs::write(root.join("rw.ext4"), b"rw-initial").unwrap();
    fs::write(root.join("canary"), b"private").unwrap();
    let before = fs::metadata(&path).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "macos_vm_attachment::storage::tests::raw_storage_sandbox_child",
            "--nocapture",
        ])
        .env("K_OCI_TEST_RAW_SANDBOX_ROOT", &root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
    assert!(fs::read(&path).unwrap().iter().all(|byte| *byte == 0));
    assert_eq!(fs::read(root.join("rw.ext4")).unwrap(), b"rw-updated");
    assert_eq!(fs::read(root.join("canary")).unwrap(), b"private");
    let after = fs::metadata(&path).unwrap();
    assert_eq!(
        (
            before.dev(),
            before.ino(),
            before.len(),
            before.mode(),
            before.uid(),
            before.gid()
        ),
        (
            after.dev(),
            after.ino(),
            after.len(),
            after.mode(),
            after.uid(),
            after.gid()
        )
    );
}

#[test]
#[ignore = "invoked by parent in an owned native sandbox process"]
fn raw_storage_sandbox_child() {
    use std::os::unix::fs::FileExt;
    let root = PathBuf::from(std::env::var_os("K_OCI_TEST_RAW_SANDBOX_ROOT").unwrap());
    let grant =
        serde_json::from_slice(&fs::read(root.join("runtime/grant.json")).unwrap()).unwrap();
    let pin = MacosStorageAttachment::open(grant).unwrap();
    let ro = pin.disk().path();
    let ro_fd = pin.disk().loader_path();
    let rw = root.join("rw.ext4");
    let writer = PinnedFile::open_storage_disk(&rw, 10, true).unwrap();
    let rw_fd = writer.loader_path();
    let console = crate::unix_process::prepare_console_output(&root.join("console"), None).unwrap();
    crate::macos_worker_sandbox::confine(
        &[ro, &ro_fd],
        &[&rw, &rw_fd],
        &[],
        &root.join("runtime"),
        &console,
        None,
        &[],
        &[],
        &[],
    )
    .unwrap();
    pin.reverify().unwrap();
    let mut denials = std::collections::BTreeMap::new();
    for (name, result) in [
        ("write", fs::write(ro, b"illegal")),
        (
            "truncate",
            OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(ro)
                .map(|_| ()),
        ),
        (
            "append",
            OpenOptions::new().append(true).open(ro).map(|_| ()),
        ),
        (
            "descriptor-reopen",
            OpenOptions::new().write(true).open(&ro_fd).map(|_| ()),
        ),
        ("delete", fs::remove_file(ro)),
        ("rename", fs::rename(ro, root.join("runtime/moved"))),
        ("replace", fs::rename(&rw, ro)),
        ("hardlink", fs::hard_link(ro, root.join("runtime/alias"))),
        (
            "symlink",
            std::os::unix::fs::symlink(&rw, root.join("alias")),
        ),
        ("rw-delete", fs::remove_file(&rw)),
        ("rw-rename", fs::rename(&rw, root.join("runtime/rw-moved"))),
        ("parent-create", fs::write(root.join("new"), b"illegal")),
    ] {
        let error = result.unwrap_err();
        assert!(
            matches!(error.raw_os_error(), Some(libc::EPERM | libc::EACCES)),
            "{name}: {error}"
        );
        denials.insert(name, error.raw_os_error().unwrap());
    }
    let raw_fd: i32 = ro_fd
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    // SAFETY: the retained pin owns raw_fd and the input byte is valid for this call.
    assert_eq!(
        // SAFETY: raw_fd stays owned by the live projection; the buffer contains one readable byte.
        unsafe { libc::pwrite(raw_fd, b"x".as_ptr().cast(), 1, 0) },
        -1
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::EBADF)
    );
    let thread_ro = ro.to_path_buf();
    let thread_denial = std::thread::spawn(move || {
        fs::write(thread_ro, b"illegal")
            .unwrap_err()
            .raw_os_error()
            .unwrap()
    })
    .join()
    .unwrap();
    assert!(matches!(thread_denial, libc::EPERM | libc::EACCES));
    assert!(fs::read(root.join("canary")).is_err());
    assert!(fs::read_dir(&root).is_err());
    let child = std::process::Command::new("/bin/sh")
        .args(["-c", "exit 0"])
        .status()
        .unwrap_err();
    assert!(matches!(
        child.raw_os_error(),
        Some(libc::EPERM | libc::EACCES)
    ));
    let writable = OpenOptions::new().write(true).open(&rw_fd).unwrap();
    writable.write_all_at(b"rw-updated", 0).unwrap();
    assert_eq!(fs::read(&rw).unwrap(), b"rw-updated");
    println!(
        "{}",
        json!({"hostDenials":denials,"threadDenial":thread_denial,"childExecDenial":child.raw_os_error(),"retainedRoFdWrite":libc::EBADF,"writableControl":true})
    );
}
