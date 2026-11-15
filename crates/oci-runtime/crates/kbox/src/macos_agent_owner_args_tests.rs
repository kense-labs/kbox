use super::*;

#[test]
fn product_owner_lifetime_is_explicit_and_diagnostics_remain_bounded() {
    let args = [
        "koci-kshim",
        "agent-vm-smoke",
        "--rootfs",
        "/rootfs",
        "--system-image-manifest",
        "/manifest",
        "--console",
        "/console",
        "--pipe-name",
        "agent",
        "--socket-path",
        "/agent/socket",
        "--owner-pid",
        "1",
        "--runtime-share",
        "/share",
    ];
    for owner in [false, true] {
        let mut argv = args.to_vec();
        if owner {
            argv.push("--runtime-owner");
        }
        let parsed = Cli::try_parse_from(argv).unwrap();
        let Command::AgentVmSmoke { runtime_owner, .. } = parsed.command else {
            panic!("expected agent command")
        };
        assert_eq!(runtime_owner, owner);
    }
    assert!(Cli::try_parse_from(args.into_iter().chain(["--runtime-owner=true"])).is_err());
}
