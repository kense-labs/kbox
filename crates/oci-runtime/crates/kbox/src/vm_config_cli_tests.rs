use super::*;

fn arguments(command: &str) -> Vec<String> {
    let mut args = vec!["koci-kshim".into(), command.into()];
    args.extend(
        [
            "--system-image-manifest",
            "/fixture/system-image.json",
            "--console",
            "/fixture/console",
            "--runtime-share",
            "/fixture/share",
            "--socket-path",
            "/fixture/agent.sock",
        ]
        .into_iter()
        .map(String::from),
    );
    match command {
        "agent-vm-smoke" => args.extend(
            [
                "--rootfs",
                "/fixture/rootfs",
                "--pipe-name",
                "vm-sizing",
                "--owner-pid",
                "1",
            ]
            .into_iter()
            .map(String::from),
        ),
        "__macos-agent-vm-worker" => args.extend(
            ["--guest-token-file", "/run/token"]
                .into_iter()
                .map(String::from),
        ),
        _ => unreachable!(),
    }
    args
}

fn resources(command: Command) -> Option<koci_kbox::VmConfig> {
    let (vcpus, memory) = match command {
        Command::AgentVmSmoke {
            vm_vcpus,
            vm_memory_mib,
            ..
        }
        | Command::MacosAgentVmWorker {
            vm_vcpus,
            vm_memory_mib,
            ..
        } => (vm_vcpus, vm_memory_mib),
        _ => unreachable!(),
    };
    parse_vm_config(vcpus, memory).unwrap()
}

#[test]
fn both_handoffs_preserve_legacy_absence_and_exact_explicit_resources() {
    for command in ["agent-vm-smoke", "__macos-agent-vm-worker"] {
        assert_eq!(
            resources(Cli::try_parse_from(arguments(command)).unwrap().command),
            None
        );
        let mut args = arguments(command);
        args.extend(
            ["--vm-vcpus", "2", "--vm-memory-mib", "768"]
                .into_iter()
                .map(String::from),
        );
        let config = resources(Cli::try_parse_from(args).unwrap().command).unwrap();
        assert_eq!((config.vcpus(), config.memory_mib()), (2, 768));
    }
}

#[test]
fn both_handoffs_reject_partial_resource_arguments_before_dispatch() {
    for command in ["agent-vm-smoke", "__macos-agent-vm-worker"] {
        assert!(Cli::try_parse_from(arguments(command)).is_ok());
        for (flag, value) in [("--vm-vcpus", "2"), ("--vm-memory-mib", "768")] {
            let mut args = arguments(command);
            args.extend([flag, value].into_iter().map(String::from));
            let error = Cli::try_parse_from(args).unwrap_err();
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::MissingRequiredArgument
            );
            let required = if flag == "--vm-vcpus" {
                "--vm-memory-mib"
            } else {
                "--vm-vcpus"
            };
            assert!(error.to_string().contains(required), "{error}");
        }
    }
}

#[test]
fn resource_parser_rejects_invalid_values_and_never_defaults_explicit_intent() {
    for (cpus, memory) in [
        (Some(2), None),
        (None, Some(768)),
        (Some(0), Some(512)),
        (Some(3), Some(512)),
        (Some(1), Some(0)),
        (Some(1), Some(129)),
        (Some(1), Some(8193)),
    ] {
        assert_eq!(
            parse_vm_config(cpus, memory).unwrap_err().code,
            koci_sdk::ErrorCode::InvalidArgument
        );
    }
    assert_eq!(parse_vm_config(None, None).unwrap(), None);
    assert_eq!(
        parse_vm_config(Some(1), Some(512)).unwrap(),
        Some(koci_kbox::VmConfig::default())
    );
}
