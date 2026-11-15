use super::*;
use std::os::unix::fs::symlink;
use std::process::Command;

#[test]
fn refuses_writable_aliases_hardlinks_and_replaced_sources() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let source = root.join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("input"), b"original").unwrap();
    let alias = root.join("alias");
    symlink(&source, &alias).unwrap();
    assert!(MacosReadOnlyMounts::open(&[&source], &[&alias]).is_err());
    assert!(MacosReadOnlyMounts::open(&[&alias], &[]).is_err());
    assert!(MacosReadOnlyMounts::open(&[&source], &[&root]).is_err());
    let firmlink = PathBuf::from("/System/Volumes/Data").join(source.strip_prefix("/").unwrap());
    if firmlink.exists() {
        assert!(MacosReadOnlyMounts::open(&[&source], &[&firmlink]).is_err());
        assert!(MacosReadOnlyMounts::open(&[&source], &[firmlink.parent().unwrap()]).is_err());
    }
    assert!(MacosReadOnlyMounts::open(&[&source], &[&source.join("future")]).is_err());
    fs::hard_link(source.join("input"), root.join("hardlink")).unwrap();
    let error = MacosReadOnlyMounts::open(&[&source], &[]).unwrap_err();
    assert!(error.to_string().contains("hard-linked"));
    fs::remove_file(root.join("hardlink")).unwrap();

    let mounts = MacosReadOnlyMounts::open(&[&source], &[]).unwrap();
    let moved = root.join("moved");
    fs::rename(&source, &moved).unwrap();
    fs::create_dir(&source).unwrap();
    fs::write(source.join("input"), b"replacement").unwrap();
    assert!(mounts.reverify().is_err());
    let pinned = Dir::from_std_file(File::open(mounts.pinned_path(0).unwrap()).unwrap());
    assert_eq!(pinned.read("input").unwrap(), b"original");
    assert_eq!(fs::read(source.join("input")).unwrap(), b"replacement");
    assert!(mounts.pinned_path(1).is_none());
}

#[test]
fn host_denies_mutations_through_paths_descriptors_threads_and_children() {
    let temporary = tempfile::Builder::new()
        .prefix("kbox-ro-中文-\"-\\-")
        .tempdir()
        .unwrap();
    let root = temporary.path().canonicalize().unwrap();
    fs::create_dir_all(root.join("parent/readonly/subdir")).unwrap();
    fs::create_dir(root.join("writable")).unwrap();
    fs::write(root.join("parent/readonly/input"), b"original").unwrap();
    fs::write(root.join("writable/replacement"), b"replacement").unwrap();
    symlink(
        root.join("parent/readonly/input"),
        root.join("writable/alias"),
    )
    .unwrap();
    symlink("input", root.join("parent/readonly/inside-alias")).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "macos_readonly::tests::confined_child",
            "--nocapture",
        ])
        .env("KBOX_READONLY_TEST_ROOT", &root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(root.join("parent/readonly/input")).unwrap(),
        b"original"
    );
    assert_eq!(fs::read(root.join("writable/output")).unwrap(), b"allowed");
    assert!(!root.join("parent/readonly/new").exists());
    // The owning host process is not confined; its live updates still propagate.
    fs::write(root.join("parent/readonly/input"), b"host update").unwrap();
    assert_eq!(
        fs::read(root.join("writable/alias")).unwrap(),
        b"host update"
    );
}

#[test]
#[ignore = "entered by the parent test in a disposable native process"]
fn confined_child() {
    let root = PathBuf::from(std::env::var_os("KBOX_READONLY_TEST_ROOT").unwrap());
    let source = root.join("parent/readonly");
    let writable = root.join("writable");
    let already_open = OpenOptions::new()
        .write(true)
        .open(source.join("input"))
        .unwrap();
    let mounts = MacosReadOnlyMounts::open(&[&source], &[&writable]).unwrap();
    let pinned = Dir::from_std_file(File::open(mounts.pinned_path(0).unwrap()).unwrap());
    assert!(mounts
        .confine()
        .unwrap_err()
        .to_string()
        .contains("pre-opened writable descriptor"));
    drop(already_open);
    let firmlink = PathBuf::from("/System/Volumes/Data").join(source.strip_prefix("/").unwrap());
    if firmlink.exists() {
        let aliased = OpenOptions::new()
            .write(true)
            .open(firmlink.join("input"))
            .unwrap();
        assert!(mounts
            .confine()
            .unwrap_err()
            .to_string()
            .contains("pre-opened writable descriptor"));
        drop(aliased);
    }
    mounts.confine().unwrap();
    assert_eq!(pinned.read("input").unwrap(), b"original");
    denied(fs::write(source.join("new"), b"no"));
    denied(fs::write(source.join("input"), b"no"));
    denied(
        OpenOptions::new()
            .append(true)
            .open(source.join("input"))
            .map(|_| ()),
    );
    denied(
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(source.join("input"))
            .map(|_| ()),
    );
    denied(fs::remove_file(source.join("input")));
    denied(fs::remove_dir(source.join("subdir")));
    denied(fs::create_dir(source.join("newdir")));
    denied(fs::rename(source.join("input"), writable.join("moved")));
    denied(fs::rename(
        writable.join("replacement"),
        source.join("input"),
    ));
    denied(fs::rename(&source, writable.join("source")));
    denied(fs::rename(root.join("parent"), root.join("moved-parent")));
    denied(pinned.write("input", b"no"));
    denied(fs::write(writable.join("alias"), b"no"));
    denied(fs::write(source.join("inside-alias"), b"no"));
    denied(symlink(writable.join("replacement"), source.join("link")));
    denied(fs::hard_link(
        source.join("input"),
        writable.join("hardlink"),
    ));
    std::thread::spawn(move || {
        let error = pinned.write("input", b"thread write").unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::EPERM));
    })
    .join()
    .unwrap();
    assert!(Command::new("/bin/sh")
        .args(["-c", "exit 0"])
        .status()
        .is_err());
    fs::write(writable.join("output"), b"allowed").unwrap();
    fs::hard_link(writable.join("output"), writable.join("allowed-hardlink")).unwrap();
    println!("native host read-only denial: paths, open descriptors, replacement, links, thread and child; writable control passed");
}

#[track_caller]
fn denied(result: std::io::Result<()>) {
    let error = result.unwrap_err();
    assert!(
        matches!(error.raw_os_error(), Some(libc::EPERM | libc::EACCES)),
        "{error}"
    );
}
