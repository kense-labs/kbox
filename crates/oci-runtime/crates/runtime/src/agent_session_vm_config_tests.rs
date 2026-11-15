use super::*;

#[test]
fn legacy_hvf_shim_command_retains_its_exact_argument_prefix() {
    let mut command = Command::new("fixed-shim");
    configure_hvf_agent_vm_command(&mut command, None);
    let args = command.as_std().get_args().collect::<Vec<_>>();
    assert_eq!(args, ["agent-vm-smoke", "--runtime-owner"]);
}

#[test]
fn explicit_hvf_resources_follow_the_subcommand_and_are_never_omitted() {
    for (cpus, memory) in [(1, 512), (2, 768), (8, 8192)] {
        let mut command = Command::new("fixed-shim");
        configure_hvf_agent_vm_command(
            &mut command,
            Some(koci_sdk::VmConfig::new(cpus, memory).unwrap()),
        );
        let args = command
            .as_std()
            .get_args()
            .map(|s| s.to_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            args,
            [
                "agent-vm-smoke",
                "--runtime-owner",
                "--vm-vcpus",
                &cpus.to_string(),
                "--vm-memory-mib",
                &memory.to_string()
            ]
        );
    }
}
