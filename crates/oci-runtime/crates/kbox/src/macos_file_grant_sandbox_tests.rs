use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn exact_file_projection_preserves_live_data_without_parent_or_metadata_authority() {
    let temporary = tempfile::Builder::new()
        .prefix("kfile-中文 ")
        .tempdir_in("/private/tmp")
        .unwrap();
    let root = temporary.path().canonicalize().unwrap();
    for directory in ["caller", "runtime", "project-ro", "project-rw"] {
        fs::create_dir(root.join(directory)).unwrap();
    }
    let source_ro = root.join("caller/输入 ro");
    let source_rw = root.join("caller/empty rw");
    fs::write(&source_ro, b"initial").unwrap();
    fs::write(&source_rw, b"").unwrap();
    fs::write(root.join("caller/canary"), b"must remain private").unwrap();
    fs::set_permissions(&source_ro, fs::Permissions::from_mode(0o666)).unwrap();
    fs::set_permissions(&source_rw, fs::Permissions::from_mode(0o666)).unwrap();
    let original = [
        fs::metadata(&source_ro).unwrap(),
        fs::metadata(&source_rw).unwrap(),
    ];
    fs::hard_link(&source_ro, root.join("project-ro/file")).unwrap();
    fs::hard_link(&source_rw, root.join("project-rw/file")).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "macos_worker_sandbox::file_tests::file_projection_child",
            "--nocapture",
        ])
        .env("K_OCI_TEST_FILE_PEP_ROOT", &root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !root.join("runtime/ready").exists() && Instant::now() < deadline {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let ready = root.join("runtime/ready").exists();
    if ready {
        fs::write(&source_ro, b"host-live-update").unwrap();
        fs::write(root.join("runtime/updated"), b"ready").unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        ready && output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(&source_ro).unwrap(), b"host-live-update");
    assert_eq!(fs::read(&source_rw).unwrap(), b"guest-write");
    assert_eq!(
        fs::read(root.join("caller/canary")).unwrap(),
        b"must remain private"
    );
    for (index, source) in [source_ro, source_rw].iter().enumerate() {
        let now = fs::metadata(source).unwrap();
        assert_eq!(
            (now.uid(), now.gid(), now.mode()),
            (
                original[index].uid(),
                original[index].gid(),
                original[index].mode()
            )
        );
        assert_eq!(now.ino(), original[index].ino());
    }
}

#[test]
#[ignore = "invoked by parent in a disposable native sandbox process"]
fn file_projection_child() {
    let root = PathBuf::from(std::env::var_os("K_OCI_TEST_FILE_PEP_ROOT").unwrap());
    let ro = root.join("project-ro/file");
    let rw = root.join("project-rw/file");
    let source_ro = root.join("caller/输入 ro");
    let source_rw = root.join("caller/empty rw");
    let preopened = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&source_ro)
        .unwrap();
    let console = crate::unix_process::prepare_console_output(&root.join("console"), None).unwrap();
    let rejected = super::confine(
        &[&ro],
        &[&rw],
        &[&source_ro, &source_rw],
        &root.join("runtime"),
        &console,
        None,
        &[],
        &[],
        &[],
    )
    .unwrap_err();
    assert!(rejected.message.contains("pre-opened writable descriptor"));
    drop(preopened);
    super::confine(
        &[&ro],
        &[&rw],
        &[&source_ro, &source_rw],
        &root.join("runtime"),
        &console,
        None,
        &[],
        &[],
        &[],
    )
    .unwrap();
    assert_eq!(fs::read(&ro).unwrap(), b"initial");
    assert!(fs::write(&ro, b"illegal").is_err());
    assert!(fs::read(&source_ro).is_err());
    assert!(fs::read(root.join("caller/canary")).is_err());
    assert!(fs::read_dir(root.join("caller")).is_err());
    assert!(fs::write(root.join("caller/new"), b"illegal").is_err());
    assert!(fs::remove_file(&source_rw).is_err());
    assert!(fs::rename(&source_rw, root.join("caller/moved")).is_err());
    assert!(fs::rename(&rw, root.join("runtime/moved")).is_err());
    assert!(fs::remove_file(&rw).is_err());
    assert!(fs::hard_link(&ro, root.join("runtime/alias")).is_err());
    assert!(fs::set_permissions(&rw, fs::Permissions::from_mode(0o777)).is_err());
    assert!(fs::metadata(&source_ro).is_ok());
    fs::write(&rw, b"guest-write").unwrap();
    fs::write(root.join("runtime/ready"), b"ready").unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !root.join("runtime/updated").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(fs::read(&ro).unwrap(), b"host-live-update");
}
