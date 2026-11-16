use super::*;

#[test]
fn independent_allocations_preserve_caller_files_and_have_private_mode() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("caller-canary"), b"caller-owned").unwrap();
    let first = TemplateDirectory::create(root.path()).unwrap();
    let second = TemplateDirectory::create(root.path()).unwrap();
    assert_ne!(first.path(), second.path());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(first.path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }
    std::fs::write(first.path().join("template.ram"), b"first-memory").unwrap();
    std::fs::write(second.path().join("template.ram"), b"second-memory").unwrap();
    first.remove().unwrap();
    assert!(!first.path().exists());
    assert_eq!(
        std::fs::read(second.path().join("template.ram")).unwrap(),
        b"second-memory"
    );
    assert_eq!(
        std::fs::read(root.path().join("caller-canary")).unwrap(),
        b"caller-owned"
    );
    second.remove().unwrap();
}

#[test]
fn deletion_requires_explicit_consumer_close_not_manager_drop() {
    let root = tempfile::tempdir().unwrap();
    let directory = TemplateDirectory::create(root.path()).unwrap();
    let consumer = directory.claim().unwrap();
    assert_eq!(
        directory.remove().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    consumer.closed();
    directory.remove().unwrap();
    let directory = TemplateDirectory::create(root.path()).unwrap();
    drop(directory.claim().unwrap());
    assert_eq!(
        directory.remove().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    assert!(directory.path().is_dir());
}

#[cfg(unix)]
#[test]
fn replacing_an_owned_path_does_not_delete_the_replacement() {
    let root = tempfile::tempdir().unwrap();
    let directory = TemplateDirectory::create(root.path()).unwrap();
    let pinned = root.path().join("pinned");
    std::fs::rename(directory.path(), &pinned).unwrap();
    std::fs::create_dir(directory.path()).unwrap();
    std::fs::write(directory.path().join("caller-canary"), b"replacement-data").unwrap();
    assert_eq!(
        directory.remove().unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    assert!(directory.claim().is_err());
    assert_eq!(
        std::fs::read(directory.path().join("caller-canary")).unwrap(),
        b"replacement-data"
    );
    std::fs::remove_dir_all(directory.path()).unwrap();
    std::os::unix::fs::symlink(&pinned, directory.path()).unwrap();
    assert!(directory.remove().is_err());
    assert!(pinned.is_dir());
    std::fs::remove_file(directory.path()).unwrap();
    std::fs::rename(pinned, directory.path()).unwrap();
    directory.remove().unwrap();
}

#[test]
fn workload_grants_cannot_expose_private_template_inputs() {
    let root = tempfile::tempdir().unwrap();
    let directory = TemplateDirectory::create(root.path()).unwrap();
    let input = directory.path().join("template.ram");
    std::fs::write(&input, b"snapshot-data").unwrap();
    let sibling = root.path().join("caller 空格");
    std::fs::create_dir(&sibling).unwrap();
    let consumer = directory.claim().unwrap();
    for source in [root.path(), directory.path(), input.as_path()] {
        assert_eq!(
            consumer.validate_grant(source).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
    }
    #[cfg(unix)]
    {
        let alias = root.path().join("parent-alias");
        std::os::unix::fs::symlink(root.path(), &alias).unwrap();
        assert_eq!(
            consumer.validate_grant(&alias).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
    }
    consumer.validate_grant(&sibling).unwrap();
    assert_eq!(std::fs::read(&input).unwrap(), b"snapshot-data");
    consumer.closed();
    directory.remove().unwrap();
}

#[test]
fn linked_snapshot_inputs_refuse_admission_and_rewrite() {
    let root = tempfile::tempdir().unwrap();
    let directory = TemplateDirectory::create(root.path()).unwrap();
    let input = directory.path().join("template.ram");
    std::fs::write(&input, b"private-input").unwrap();
    let alias = root.path().join("caller-alias");
    std::fs::hard_link(&input, &alias).unwrap();
    assert!(directory.claim().is_err());
    assert!(directory.exclusive_directory().is_err());
    assert_eq!(std::fs::read(&alias).unwrap(), b"private-input");
    std::fs::remove_file(&alias).unwrap();
    #[cfg(unix)]
    {
        std::fs::remove_file(&input).unwrap();
        std::fs::write(&alias, b"caller-owned").unwrap();
        std::os::unix::fs::symlink(&alias, &input).unwrap();
        assert!(directory.claim().is_err());
        assert!(directory.exclusive_directory().is_err());
        assert_eq!(std::fs::read(&alias).unwrap(), b"caller-owned");
        std::fs::remove_file(&input).unwrap();
    }
    directory.remove().unwrap();
}

#[test]
fn removed_directory_can_be_finalized_again_without_touching_its_replacement() {
    let root = tempfile::tempdir().unwrap();
    let directory = TemplateDirectory::create(root.path()).unwrap();
    directory.remove().unwrap();
    directory.remove().unwrap();
    std::fs::create_dir(directory.path()).unwrap();
    std::fs::write(directory.path().join("caller"), b"replacement").unwrap();
    directory.remove().unwrap();
    assert_eq!(
        std::fs::read(directory.path().join("caller")).unwrap(),
        b"replacement"
    );
}
