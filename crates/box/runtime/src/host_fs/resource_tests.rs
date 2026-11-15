use super::oci::OciResources;
use kbox_core::config::{BoxConfig, ResourceLimits};

#[test]
fn explicit_limits_and_byte_precision_survive_compilation() {
    let config = BoxConfig {
        resource_limits: ResourceLimits {
            sandbox_memory_limit_bytes: Some(128 * 1024 * 1024 + 7),
            memory_reservation: Some(64 * 1024 * 1024),
            memory_swap: Some(-1),
            cpu_shares: Some(512),
            cpu_period: Some(200_000),
            cpu_quota: Some(75_000),
            cpuset_cpus: Some("0-1".into()),
            pids_limit: Some(32),
            ..Default::default()
        },
        ..Default::default()
    };
    let resources = OciResources::from_box_config(&config).unwrap();
    assert_eq!(resources.memory_limit, 128 * 1024 * 1024 + 7);
    assert_eq!(resources.memory_reservation, Some(64 * 1024 * 1024));
    assert_eq!(resources.memory_swap, Some(-1));
    assert_eq!(resources.cpu_shares, Some(512));
    assert_eq!(
        (resources.cpu_quota, resources.cpu_period),
        (75_000, 200_000)
    );
    assert_eq!(resources.cpuset_cpus.as_deref(), Some("0-1"));
    assert_eq!(resources.pids_limit, 32);
}

#[test]
fn invalid_limits_fail_before_compilation_can_drop_or_wrap_them() {
    for limits in [
        ResourceLimits {
            cpu_period: Some(0),
            ..Default::default()
        },
        ResourceLimits {
            cpu_period: Some(u64::MAX),
            ..Default::default()
        },
        ResourceLimits {
            cpu_shares: Some(1),
            ..Default::default()
        },
        ResourceLimits {
            cpu_quota: Some(0),
            ..Default::default()
        },
        ResourceLimits {
            cpuset_cpus: Some("0; echo unsafe".into()),
            ..Default::default()
        },
        ResourceLimits {
            pids_limit: Some(0),
            ..Default::default()
        },
        ResourceLimits {
            pids_limit: Some(u64::MAX),
            ..Default::default()
        },
        ResourceLimits {
            memory_reservation: Some(u64::MAX),
            ..Default::default()
        },
        ResourceLimits {
            sandbox_memory_limit_bytes: Some(0),
            ..Default::default()
        },
        ResourceLimits {
            sandbox_memory_limit_bytes: Some(u64::MAX),
            ..Default::default()
        },
        ResourceLimits {
            memory_swap: Some(-2),
            ..Default::default()
        },
        ResourceLimits {
            memory_swap: Some(1),
            ..Default::default()
        },
    ] {
        let config = BoxConfig {
            resource_limits: limits,
            ..Default::default()
        };
        assert!(
            OciResources::from_box_config(&config).is_err(),
            "{:?}",
            config.resource_limits
        );
    }
}
