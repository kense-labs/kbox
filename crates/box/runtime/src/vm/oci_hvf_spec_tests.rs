use super::*;

#[test]
fn hvf_nonroot_cap_add_retains_only_explicit_undropped_values_across_exec() {
    use oci_spec::runtime::Capability;
    let cases: &[(u32, &[&str], &[Capability])] = &[
        (0, &[], &[]),
        (
            1001,
            &[],
            &[Capability::Mknod, Capability::Setfcap, Capability::Chown],
        ),
        (1001, &["MKNOD"], &[Capability::Setfcap, Capability::Chown]),
        (1001, &["ALL"], &[]),
    ];
    for (uid, drops, expected) in cases {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        std::fs::create_dir(root.join("rootfs")).unwrap();
        std::fs::create_dir(root.join("workspace")).unwrap();
        let config = BoxConfig {
            workspace: root.join("workspace"),
            cap_add: vec![" cap_mknod ".into(), "SETFCAP".into(), "CAP_CHOWN".into()],
            cap_drop: drops.iter().map(|name| (*name).into()).collect(),
            ..Default::default()
        };
        let instance = InstanceSpec {
            rootfs: RootfsSource::directory(root.join("rootfs")),
            fs_mounts: vec![FsMount {
                tag: "workspace".into(),
                host_path: root.join("workspace"),
                read_only: false,
            }],
            ..Default::default()
        };
        let process = MicrovmRuntimeProcess {
            args: vec!["/bin/sh".into()],
            environment: vec![],
            cwd: "/".into(),
            uid: *uid,
            gid: *uid,
            additional_gids: vec![],
            dropped_capabilities: vec![],
        };
        let mut spec = compile_portable_microvm_oci_spec(
            "11111111-1111-4111-8111-111111111111",
            "test",
            &process,
        )
        .unwrap();
        complete(&mut spec, &config, &instance).unwrap();
        let capabilities = spec
            .process()
            .as_ref()
            .unwrap()
            .capabilities()
            .as_ref()
            .unwrap();
        let expected: std::collections::HashSet<_> = expected.iter().copied().collect();
        assert_eq!(capabilities.inheritable().as_ref().unwrap(), &expected);
        assert_eq!(capabilities.ambient().as_ref().unwrap(), &expected);
        assert!(expected.is_subset(capabilities.permitted().as_ref().unwrap()));
        assert_eq!(spec.process().as_ref().unwrap().user().uid(), *uid);
    }
}
use crate::oci_portable_spec::{compile_portable_microvm_oci_spec, MicrovmRuntimeProcess};
use kbox_core::vmm::{FsMount, RootfsSource};

#[test]
fn hvf_shm_size_keeps_default_omission_and_exact_safe_mount() {
    let mut config = BoxConfig::default();
    assert!(tmpfs::mounts(&config).unwrap().is_empty());
    config.tmpfs = vec!["/dev/shm:size=2m".into(), "/scratch:size=4k,ro".into()];
    let mounts = tmpfs::mounts(&config).unwrap();
    assert_eq!(mounts.len(), 2);
    assert_eq!(mounts[0].destination(), std::path::Path::new("/dev/shm"));
    assert_eq!(mounts[0].typ().as_deref(), Some("tmpfs"));
    assert_eq!(
        mounts[0].source().as_deref(),
        Some(std::path::Path::new("shm"))
    );
    assert_eq!(
        mounts[0].options().as_ref().unwrap(),
        &[
            "rw",
            "nosuid",
            "nodev",
            "noexec",
            "mode=1777",
            "size=2097152"
        ]
    );
    assert_eq!(mounts[1].destination(), std::path::Path::new("/scratch"));
    assert_eq!(mounts[1].options().as_ref().unwrap(), &["size=4k", "ro"]);
    config.tmpfs.push("/dev/./shm:size=4k".into());
    assert!(tmpfs::mounts(&config).is_err());
    config.tmpfs.pop();
    config.volumes.push("/tmp:/dev/shm/data:rw".into());
    assert!(tmpfs::mounts(&config).is_err());
}

#[test]
fn hvf_nproc_ulimits_bind_actual_nonroot_process_without_rewriting_defaults() {
    let mut config = BoxConfig {
        user: Some("1001:2002".into()),
        cap_drop: vec!["ALL".into()],
        ..Default::default()
    };
    config.resource_limits.ulimits = vec!["nproc=16:16".into()];
    let user = oci_spec::runtime::UserBuilder::default()
        .uid(1001u32)
        .gid(2002u32)
        .build()
        .unwrap();
    let mut process = oci_spec::runtime::ProcessBuilder::default()
        .user(user)
        .capabilities(workload_capabilities(&config).unwrap())
        .no_new_privileges(true)
        .build()
        .unwrap();
    apply_workload_rlimits(&config, &mut process).unwrap();
    assert_eq!(
        serde_json::to_value(&process).unwrap()["rlimits"],
        serde_json::json!([
            {"type":"RLIMIT_NOFILE","soft":1024,"hard":1024},
            {"type":"RLIMIT_NPROC","soft":16,"hard":16},
        ])
    );
    for uid in [0, 1002] {
        let mut invalid = process.clone();
        invalid.user_mut().set_uid(uid);
        let before = serde_json::to_vec(&invalid).unwrap();
        assert!(apply_workload_rlimits(&config, &mut invalid).is_err());
        assert_eq!(serde_json::to_vec(&invalid).unwrap(), before);
    }
    for value in [None, Some(false)] {
        let mut invalid = process.clone();
        invalid.set_no_new_privileges(value);
        let before = serde_json::to_vec(&invalid).unwrap();
        assert!(apply_workload_rlimits(&config, &mut invalid).is_err());
        assert_eq!(serde_json::to_vec(&invalid).unwrap(), before);
    }
    let mut invalid = process.clone();
    invalid.set_capabilities(None);
    let before = serde_json::to_vec(&invalid).unwrap();
    assert!(apply_workload_rlimits(&config, &mut invalid).is_err());
    assert_eq!(serde_json::to_vec(&invalid).unwrap(), before);
}

#[test]
fn hvf_sigpending_ulimits_preserve_exact_values_and_nproc_entry_guards() {
    queued_resource_limits(
        "sigpending",
        oci_spec::runtime::PosixRlimitType::RlimitSigpending,
    );
}

#[test]
fn hvf_msgqueue_ulimits_preserve_exact_values_and_per_uid_entry_guards() {
    queued_resource_limits(
        "msgqueue",
        oci_spec::runtime::PosixRlimitType::RlimitMsgqueue,
    );
}

fn queued_resource_limits(resource: &str, typ: oci_spec::runtime::PosixRlimitType) {
    let mut config = BoxConfig {
        user: Some("1001:2002".into()),
        cap_drop: vec!["ALL".into()],
        ..Default::default()
    };
    let user = oci_spec::runtime::UserBuilder::default()
        .uid(1001u32)
        .gid(2002u32)
        .build()
        .unwrap();
    let mut process = oci_spec::runtime::ProcessBuilder::default()
        .user(user)
        .capabilities(workload_capabilities(&config).unwrap())
        .no_new_privileges(true)
        .build()
        .unwrap();
    for (value, soft, hard) in [
        ("16:32", 16, 32),
        ("0:0", 0, 0),
        ("-1:-1", u64::MAX, u64::MAX),
    ] {
        config.resource_limits.ulimits = vec![format!("{}={value}", resource.to_ascii_uppercase())];
        apply_workload_rlimits(&config, &mut process).unwrap();
        let limits = process.rlimits().as_ref().unwrap();
        assert_eq!(limits.len(), 2);
        assert_eq!(
            limits[0].typ(),
            oci_spec::runtime::PosixRlimitType::RlimitNofile
        );
        assert_eq!((limits[0].soft(), limits[0].hard()), (1024, 1024));
        assert_eq!(limits[1].typ(), typ);
        assert_eq!((limits[1].soft(), limits[1].hard()), (soft, hard));
        for uid in [0, 1002] {
            let mut invalid = process.clone();
            invalid.user_mut().set_uid(uid);
            let before = serde_json::to_vec(&invalid).unwrap();
            assert!(apply_workload_rlimits(&config, &mut invalid).is_err());
            assert_eq!(serde_json::to_vec(&invalid).unwrap(), before);
        }
        for nnp in [None, Some(false)] {
            let mut invalid = process.clone();
            invalid.set_no_new_privileges(nnp);
            assert!(apply_workload_rlimits(&config, &mut invalid).is_err());
        }
        let mut invalid = process.clone();
        invalid.set_capabilities(None);
        assert!(apply_workload_rlimits(&config, &mut invalid).is_err());
        let before = serde_json::to_vec(&process).unwrap();
        apply_workload_rlimits(&config, &mut process).unwrap();
        assert_eq!(serde_json::to_vec(&process).unwrap(), before);
    }
}

#[test]
fn hvf_nice_ulimits_preserve_exact_values_and_legacy_root_defaults() {
    let mut config = BoxConfig::default();
    let mut process = oci_spec::runtime::ProcessBuilder::default()
        .build()
        .unwrap();
    for (value, soft, hard) in [
        ("nice=25:30", 25, 30),
        ("NICE=0:0", 0, 0),
        ("nice=-1:-1", u64::MAX, u64::MAX),
    ] {
        config.resource_limits.ulimits = vec![value.into()];
        apply_workload_rlimits(&config, &mut process).unwrap();
        let limits = process.rlimits().as_ref().unwrap();
        assert_eq!(limits.len(), 2);
        assert_eq!(
            limits[0].typ(),
            oci_spec::runtime::PosixRlimitType::RlimitNofile
        );
        assert_eq!((limits[0].soft(), limits[0].hard()), (1024, 1024));
        assert_eq!(
            limits[1].typ(),
            oci_spec::runtime::PosixRlimitType::RlimitNice
        );
        assert_eq!((limits[1].soft(), limits[1].hard()), (soft, hard));
        assert_eq!(process.user().uid(), 0);
        let before = serde_json::to_vec(&process).unwrap();
        apply_workload_rlimits(&config, &mut process).unwrap();
        assert_eq!(serde_json::to_vec(&process).unwrap(), before);
    }
}

#[test]
fn hvf_realtime_ulimits_preserve_exact_values_and_legacy_root_defaults() {
    let mut config = BoxConfig::default();
    let mut process = oci_spec::runtime::ProcessBuilder::default()
        .build()
        .unwrap();
    for (values, expected) in [
        (["rtprio=5:10", "rttime=20000:80000"], [5, 10, 20000, 80000]),
        (["RTPRIO=0:0", "RTTIME=0:0"], [0; 4]),
        (["rtprio=-1:-1", "rttime=-1:-1"], [u64::MAX; 4]),
    ] {
        config.resource_limits.ulimits = values.iter().map(|value| value.to_string()).collect();
        apply_workload_rlimits(&config, &mut process).unwrap();
        assert_eq!(
            serde_json::to_value(process.rlimits()).unwrap(),
            serde_json::json!([
                {"type":"RLIMIT_NOFILE","soft":1024,"hard":1024},
                {"type":"RLIMIT_RTPRIO","soft":expected[0],"hard":expected[1]},
                {"type":"RLIMIT_RTTIME","soft":expected[2],"hard":expected[3]},
            ])
        );
        assert_eq!(process.user().uid(), 0);
        let before = serde_json::to_vec(&process).unwrap();
        apply_workload_rlimits(&config, &mut process).unwrap();
        assert_eq!(serde_json::to_vec(&process).unwrap(), before);
    }
}

#[test]
fn hvf_memory_ulimits_preserve_cpu_defaults_and_exact_byte_values() {
    let mut process = oci_spec::runtime::ProcessBuilder::default()
        .build()
        .unwrap();
    let mut config = BoxConfig::default();
    config.resource_limits.ulimits = vec!["cpu=1:2".into()];
    apply_workload_rlimits(&config, &mut process).unwrap();
    for (values, expected) in [
        (
            [
                "data=4194304:4194304",
                "stack=524288:524288",
                "memlock=65536:65536",
            ],
            [4194304, 524288, 65536],
        ),
        (["data=0:0", "stack=0:0", "memlock=0:0"], [0, 0, 0]),
        (
            ["data=-1:-1", "stack=-1:-1", "memlock=-1:-1"],
            [u64::MAX; 3],
        ),
    ] {
        config.resource_limits.ulimits = values.iter().map(|value| value.to_string()).collect();
        apply_workload_rlimits(&config, &mut process).unwrap();
        let value = serde_json::to_value(&process).unwrap();
        assert_eq!(
            value["rlimits"],
            serde_json::json!([
                {"type":"RLIMIT_NOFILE","soft":1024,"hard":1024},
                {"type":"RLIMIT_CPU","soft":1,"hard":2},
                {"type":"RLIMIT_DATA","soft":expected[0],"hard":expected[0]},
                {"type":"RLIMIT_STACK","soft":expected[1],"hard":expected[1]},
                {"type":"RLIMIT_MEMLOCK","soft":expected[2],"hard":expected[2]},
            ])
        );
        let before = serde_json::to_vec(&process).unwrap();
        apply_workload_rlimits(&config, &mut process).unwrap();
        assert_eq!(serde_json::to_vec(&process).unwrap(), before);
    }
}

#[test]
fn hvf_cpu_time_ulimits_complete_and_replace_only_declared_resource() {
    let mut process = oci_spec::runtime::ProcessBuilder::default()
        .build()
        .unwrap();
    let mut config = BoxConfig::default();
    for (value, soft, hard) in [
        ("cpu=1:2", 1, 2),
        ("CPU=1:1", 1, 1),
        ("cpu=1:-1", 1, u64::MAX),
        ("cpu=-1:-1", u64::MAX, u64::MAX),
    ] {
        config.resource_limits.ulimits = vec![value.into()];
        apply_workload_rlimits(&config, &mut process).unwrap();
        let limits = process.rlimits().as_ref().unwrap();
        assert_eq!(limits.len(), 2);
        assert_eq!(
            limits[0].typ(),
            oci_spec::runtime::PosixRlimitType::RlimitNofile
        );
        assert_eq!((limits[0].soft(), limits[0].hard()), (1024, 1024));
        assert_eq!(
            limits[1].typ(),
            oci_spec::runtime::PosixRlimitType::RlimitCpu
        );
        assert_eq!((limits[1].soft(), limits[1].hard()), (soft, hard));
        let before = serde_json::to_vec(&process).unwrap();
        apply_workload_rlimits(&config, &mut process).unwrap();
        assert_eq!(serde_json::to_vec(&process).unwrap(), before);
    }
}

#[test]
fn hvf_ulimits_intent_reaches_existing_oci_process_executor() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    for name in ["rootfs", "workspace"] {
        std::fs::create_dir(root.join(name)).unwrap();
    }
    let instance = InstanceSpec {
        rootfs: RootfsSource::directory(root.join("rootfs")),
        fs_mounts: vec![FsMount {
            tag: "workspace".into(),
            host_path: root.join("workspace"),
            read_only: false,
        }],
        ..Default::default()
    };
    let process = MicrovmRuntimeProcess {
        args: vec!["/bin/sh".into()],
        environment: vec![],
        cwd: "/".into(),
        uid: 0,
        gid: 0,
        additional_gids: vec![],
        dropped_capabilities: vec!["ALL".into()],
    };
    let mut config = BoxConfig::default();
    config.resource_limits.ulimits = vec![
        "NOFILE=64:128".into(),
        "fsize=4096:4096".into(),
        "core=0:0".into(),
    ];
    let mut spec =
        compile_portable_microvm_oci_spec("11111111-1111-4111-8111-111111111111", "test", &process)
            .unwrap();
    complete(&mut spec, &config, &instance).unwrap();
    let value = serde_json::to_value(spec).unwrap();
    assert_eq!(
        value["process"]["rlimits"],
        serde_json::json!([{"type":"RLIMIT_NOFILE","soft":64,"hard":128},{"type":"RLIMIT_FSIZE","soft":4096,"hard":4096},{"type":"RLIMIT_CORE","soft":0,"hard":0}])
    );
}

#[test]
fn hvf_ulimits_keep_empty_request_bytes_and_unmentioned_oci_defaults() {
    let mut process = oci_spec::runtime::ProcessBuilder::default()
        .build()
        .unwrap();
    for absent in [false, true] {
        if absent {
            process.set_rlimits(None);
        }
        let before = serde_json::to_vec(&process).unwrap();
        apply_workload_rlimits(&BoxConfig::default(), &mut process).unwrap();
        assert_eq!(serde_json::to_vec(&process).unwrap(), before);
    }
    let mut process = oci_spec::runtime::ProcessBuilder::default()
        .build()
        .unwrap();
    let mut config = BoxConfig::default();
    config.resource_limits.ulimits = vec!["core=0:0".into()];
    apply_workload_rlimits(&config, &mut process).unwrap();
    let first = serde_json::to_vec(&process).unwrap();
    apply_workload_rlimits(&config, &mut process).unwrap();
    assert_eq!(serde_json::to_vec(&process).unwrap(), first);
    let limits = process.rlimits().as_ref().unwrap();
    assert_eq!(limits.len(), 2);
    assert_eq!(
        limits[0].typ(),
        oci_spec::runtime::PosixRlimitType::RlimitNofile
    );
    assert_eq!((limits[0].soft(), limits[0].hard()), (1024, 1024));
}

#[test]
fn hvf_root_readonly_intent_reaches_existing_guest_executor() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    std::fs::create_dir(root.join("rootfs")).unwrap();
    std::fs::create_dir(root.join("workspace")).unwrap();
    let instance = InstanceSpec {
        rootfs: RootfsSource::directory(root.join("rootfs")),
        fs_mounts: vec![FsMount {
            tag: "workspace".into(),
            host_path: root.join("workspace"),
            read_only: false,
        }],
        ..Default::default()
    };
    let process = MicrovmRuntimeProcess {
        args: vec!["/bin/sh".into()],
        environment: vec![],
        cwd: "/".into(),
        uid: 0,
        gid: 0,
        additional_gids: vec![],
        dropped_capabilities: vec!["ALL".into()],
    };
    for read_only in [false, true] {
        let config = BoxConfig {
            read_only,
            tmpfs: vec!["/scratch:size=1m,mode=1777,rw".into()],
            cap_drop: vec!["ALL".into()],
            ..Default::default()
        };
        let mut spec = compile_portable_microvm_oci_spec(
            "11111111-1111-4111-8111-111111111111",
            "test",
            &process,
        )
        .unwrap();
        complete(&mut spec, &config, &instance).unwrap();
        assert_eq!(spec.root().as_ref().unwrap().readonly(), Some(read_only));
        assert!(spec
            .mounts()
            .as_ref()
            .unwrap()
            .iter()
            .any(|mount| mount.destination() == &PathBuf::from("/scratch")
                && mount
                    .options()
                    .as_ref()
                    .unwrap()
                    .iter()
                    .any(|option| option == "rw")));
        assert!(spec
            .process()
            .as_ref()
            .unwrap()
            .capabilities()
            .as_ref()
            .unwrap()
            .effective()
            .as_ref()
            .unwrap()
            .is_empty());
    }
}

#[test]
fn hvf_tmpfs_preserves_flags_data_defaults_and_refuses_image_subtree_collisions() {
    let config = BoxConfig {
        tmpfs: vec![
            "/临时 scratch/./:size=1M,mode=1770,uid=123,gid=456,ro,nosuid,nodev,noexec".into(),
            "/ephemeral".into(),
        ],
        ..Default::default()
    };
    let mounts = tmpfs::mounts(&config).unwrap();
    assert_eq!(mounts[0].destination(), &PathBuf::from("/临时 scratch"));
    assert_eq!(
        mounts[0].options().as_ref().unwrap(),
        &[
            "size=1M",
            "mode=1770",
            "uid=123",
            "gid=456",
            "ro",
            "nosuid",
            "nodev",
            "noexec"
        ]
    );
    assert_eq!(mounts[1].options().as_ref().unwrap(), &["rw"]);
    validate_tmpfs_image_volumes(&config, &["/临时 scratch/".into()]).unwrap();
    for value in ["/临时 scratch/sub", "/ephemeral/sub"] {
        assert!(validate_tmpfs_image_volumes(&config, &[value.into()]).is_err());
    }
}

#[test]
fn hvf_cap_drop_all_spellings_override_explicit_guest_admin() {
    for name in ["ALL", " all ", "CAP_ALL", " cap_all "] {
        let config = BoxConfig {
            cap_add: vec!["CAP_SYS_ADMIN".into()],
            cap_drop: vec![name.into()],
            ..Default::default()
        };
        let capabilities = workload_capabilities(&config).unwrap();
        assert!(
            capabilities.bounding().as_ref().unwrap().is_empty(),
            "{name}"
        );
        assert!(
            capabilities.effective().as_ref().unwrap().is_empty(),
            "{name}"
        );
        assert!(
            capabilities.permitted().as_ref().unwrap().is_empty(),
            "{name}"
        );
    }
}

#[test]
fn hvf_bundle_preserves_host_grants_exact_resources_and_explicit_guest_capability() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    for name in ["rootfs", "workspace", "readonly", "writable"] {
        std::fs::create_dir(root.join(name)).unwrap();
    }
    let mut config = BoxConfig {
        workspace: root.join("workspace"),
        ..Default::default()
    };
    config.resources.vcpus = 1;
    config.resources.memory_mb = 512;
    config.volumes = vec![
        format!("{}:/readonly:ro", root.join("readonly").display()),
        format!("{}:/writable:rw", root.join("writable").display()),
    ];
    config.cap_add = vec!["SYS_ADMIN".into()];
    config.resource_limits.cpu_quota = Some(50_000);
    config.resource_limits.cpu_period = Some(100_000);
    config.resource_limits.cpu_shares = Some(512);
    config.resource_limits.sandbox_memory_limit_bytes = Some(128 * 1024 * 1024);
    config.resource_limits.memory_reservation = Some(16 * 1024 * 1024);
    config.resource_limits.memory_swap = Some(-1);
    config.resource_limits.pids_limit = Some(16);
    let instance = InstanceSpec {
        rootfs: RootfsSource::directory(root.join("rootfs")),
        fs_mounts: vec![
            FsMount {
                tag: "workspace".into(),
                host_path: root.join("workspace"),
                read_only: false,
            },
            FsMount {
                tag: "vol0".into(),
                host_path: root.join("readonly"),
                read_only: true,
            },
            FsMount {
                tag: "vol1".into(),
                host_path: root.join("writable"),
                read_only: false,
            },
        ],
        ..Default::default()
    };
    let process = MicrovmRuntimeProcess {
        args: vec!["/bin/sh".into()],
        environment: vec![],
        cwd: "/".into(),
        uid: 0,
        gid: 0,
        additional_gids: vec![],
        dropped_capabilities: vec![],
    };
    let mut spec =
        compile_portable_microvm_oci_spec("11111111-1111-4111-8111-111111111111", "test", &process)
            .unwrap();
    complete(&mut spec, &config, &instance).unwrap();
    let value = serde_json::to_value(&spec).unwrap();
    assert_eq!(
        value["annotations"][koci_sdk::RUNTIME_BUNDLE_HANDOFF_EXTENSION],
        koci_sdk::RUNTIME_BUNDLE_HANDOFF_MOVE_V2
    );
    assert_eq!(value["linux"]["resources"]["cpu"]["quota"], 50_000);
    assert_eq!(value["linux"]["resources"]["cpu"]["shares"], 512);
    assert_eq!(
        value["linux"]["resources"]["memory"]["limit"],
        128 * 1024 * 1024
    );
    assert_eq!(
        value["linux"]["resources"]["memory"]["reservation"],
        16 * 1024 * 1024
    );
    assert_eq!(value["linux"]["resources"]["memory"]["swap"], -1);
    assert_eq!(value["linux"]["resources"]["pids"]["limit"], 16);
    assert_eq!(value["mounts"][3]["destination"], "/readonly");
    assert!(value["mounts"][3]["options"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("ro")));
    assert!(spec
        .process()
        .as_ref()
        .unwrap()
        .capabilities()
        .as_ref()
        .unwrap()
        .effective()
        .as_ref()
        .unwrap()
        .contains(&Capability::SysAdmin));
    config.cap_drop = vec!["ALL".into()];
    spec =
        compile_portable_microvm_oci_spec("11111111-1111-4111-8111-111111111111", "test", &process)
            .unwrap();
    complete(&mut spec, &config, &instance).unwrap();
    assert!(spec
        .process()
        .as_ref()
        .unwrap()
        .capabilities()
        .as_ref()
        .unwrap()
        .effective()
        .as_ref()
        .unwrap()
        .is_empty());
}

#[test]
fn hvf_file_sources_admit_plain_data_and_refuse_aliases_and_invalid_controls() {
    let temporary = tempfile::tempdir().unwrap();
    let file = temporary.path().canonicalize().unwrap().join("file");
    std::fs::write(&file, b"caller-owned").unwrap();
    let mut config = BoxConfig {
        volumes: vec![format!("{}:/file:ro", file.display())],
        ..Default::default()
    };
    assert!(validate(&config).is_ok());
    std::fs::hard_link(&file, temporary.path().join("alias")).unwrap();
    assert!(crate::local_execution::oci_file_grants::observe(&file, true).is_err());
    std::fs::remove_file(temporary.path().join("alias")).unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), b"caller-owned");
    config.volumes.clear();
    config.cap_add = vec!["TYPO".into()];
    assert!(validate(&config).is_err());
    config.cap_add.clear();
    config.resource_limits.cpu_period = Some(0);
    assert!(validate(&config).is_err());
}

#[test]
fn hvf_anonymous_completion_classifies_exact_planned_disk_separately_from_caller_files() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let allocation = root.join("卷 allocation");
    let disk = crate::volume::anonymous_ext4_disk_path(&allocation);
    std::fs::create_dir_all(disk.parent().unwrap()).unwrap();
    std::fs::write(&disk, b"owned raw disk").unwrap();
    let workspace = root.join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let config = BoxConfig {
        workspace: workspace.clone(),
        ..Default::default()
    };
    let instance = InstanceSpec {
        rootfs: RootfsSource::directory(root.clone()),
        fs_mounts: vec![
            FsMount {
                tag: "workspace".into(),
                host_path: workspace,
                read_only: false,
            },
            FsMount {
                tag: "volume".into(),
                host_path: allocation.clone(),
                read_only: false,
            },
        ],
        ..Default::default()
    };
    let spec: Spec = serde_json::from_value(serde_json::json!({
        "ociVersion":"1.3.0", "root":{"path":"rootfs"}, "linux":{},
        "process":{"cwd":"/","args":["/bin/true"],"user":{"uid":0,"gid":0}}
    }))
    .unwrap();
    let plan = vec![(allocation.clone(), "/空 卷".to_string())];
    let mut completed = spec.clone();
    complete_with_anonymous(
        &mut completed,
        &config,
        &instance,
        &Default::default(),
        &plan,
    )
    .unwrap();
    let value = serde_json::to_value(&completed).unwrap();
    let mount = value["mounts"].as_array().unwrap().last().unwrap();
    assert_eq!(mount["type"], "ext4");
    assert_eq!(mount["source"], disk.to_str().unwrap());
    assert_eq!(mount["destination"], "/空 卷");
    assert_eq!(
        value["annotations"][koci_sdk::RAW_STORAGE_IMAGES_EXTENSION],
        koci_sdk::RAW_STORAGE_IMAGES_VALUE
    );
    assert!(value["annotations"]
        .get(koci_sdk::FILE_GRANTS_EXTENSION)
        .is_none());
    for case in ["source", "access", "count", "target"] {
        let mut instance = instance.clone();
        let mut plan = plan.clone();
        match case {
            "source" => plan[0].0 = root.clone(),
            "access" => instance.fs_mounts[1].read_only = true,
            "count" => plan.clear(),
            "target" => plan[0].1 = "/run/kbox/alias".into(),
            _ => unreachable!(),
        }
        assert!(
            complete_with_anonymous(
                &mut spec.clone(),
                &config,
                &instance,
                &Default::default(),
                &plan
            )
            .is_err(),
            "{case}"
        );
    }
    assert_eq!(std::fs::read(&disk).unwrap(), b"owned raw disk");
}

#[test]
fn hvf_tmpfs_intent_reaches_the_existing_guest_mount_executor() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    for name in ["rootfs", "workspace"] {
        std::fs::create_dir(root.join(name)).unwrap();
    }
    let config = BoxConfig {
        workspace: root.join("workspace"),
        tmpfs: vec![
            "/scratch:size=1048576,mode=1777,rw".into(),
            "/sealed:size=4096,ro".into(),
        ],
        ..Default::default()
    };
    let instance = InstanceSpec {
        rootfs: RootfsSource::directory(root.join("rootfs")),
        fs_mounts: vec![FsMount {
            tag: "workspace".into(),
            host_path: root.join("workspace"),
            read_only: false,
        }],
        ..Default::default()
    };
    let process = MicrovmRuntimeProcess {
        args: vec!["/bin/sh".into()],
        environment: vec![],
        cwd: "/".into(),
        uid: 0,
        gid: 0,
        additional_gids: vec![],
        dropped_capabilities: vec![],
    };
    let mut spec = compile_portable_microvm_oci_spec(
        "11111111-1111-4111-8111-111111111111",
        "tmpfs-test",
        &process,
    )
    .unwrap();
    complete(&mut spec, &config, &instance).unwrap();
    let mounts = spec.mounts().as_ref().unwrap();
    for (target, access, data) in [
        ("/scratch", "rw", vec!["size=1048576", "mode=1777"]),
        ("/sealed", "ro", vec!["size=4096"]),
    ] {
        let selected: Vec<_> = mounts
            .iter()
            .filter(|m| m.destination() == std::path::Path::new(target))
            .collect();
        assert_eq!(
            selected.len(),
            1,
            "tmpfs intent disappeared from OCI mounts: {target}"
        );
        let mount = selected[0];
        assert_eq!(mount.typ().as_deref(), Some("tmpfs"));
        assert_eq!(
            mount.source().as_deref(),
            Some(std::path::Path::new("tmpfs"))
        );
        let options = mount.options().as_ref().unwrap();
        assert!(options.iter().any(|v| v == access));
        for value in data {
            assert!(options.iter().any(|v| v == value));
        }
    }
}
