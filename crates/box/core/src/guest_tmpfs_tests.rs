use super::*;

#[test]
fn tmpfs_flags_do_not_become_kernel_data() {
    let mount = GuestTmpfsMount::parse(
        "/临时 scratch:size=1m,mode=1777,uid=123,gid=456,ro,nosuid,nodev,noexec,huge=never",
    )
    .unwrap();
    assert_eq!(mount.path(), "/临时 scratch");
    assert!(mount.read_only());
    assert_eq!(
        mount.data_options().as_deref(),
        Some("size=1m,mode=1777,uid=123,gid=456,huge=never")
    );
    assert_eq!(
        mount
            .flags()
            .map(TmpfsMountFlag::as_str)
            .collect::<Vec<_>>(),
        ["ro", "nosuid", "nodev", "noexec"]
    );
    let default = GuestTmpfsMount::parse("/scratch").unwrap();
    assert!(!default.read_only());
    assert!(default.data_options().is_none());
}

#[test]
fn tmpfs_parser_refuses_escaping_reserved_conflicting_and_unbounded_input() {
    for value in [
        "/",
        "scratch",
        "/run/./kbox/boot",
        "/run//",
        "/x/../y",
        "/x\0",
        "/x:ro,rw",
        "/x:ro,ro",
        "/x:exec,noexec",
        "/x:dev,nodev",
        "/x:suid,nosuid",
    ] {
        assert!(GuestTmpfsMount::parse(value).is_err(), "{value:?}");
    }
    assert!(GuestTmpfsMount::parse(&format!("/{}", "x".repeat(8192))).is_err());
    assert!(GuestTmpfsMount::parse(&format!("/x:{}", vec!["size=1m"; 33].join(","))).is_err());
}
