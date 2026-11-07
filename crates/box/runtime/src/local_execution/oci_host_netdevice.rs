//! Tear down leftover keep-authority host-netdevice leases.
//!
//! Leases live under the legacy layout dir `boxes/{id}/sandbox/`. MicroVM
//! destroy/reap still calls [`teardown_lease`] so old leases, veths, and DNAT
//! rules clean up.

use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::process::Command;

use kbox_core::{ExecutionManagerError, ExecutionManagerResult};
use serde::{Deserialize, Serialize};

const LEASE_SCHEMA: &str = "kbox.sandbox-host-netdevice.v5";
const GUEST_IFACE_NAME: &str = "eth0";

/// One static TCP/UDP host→container publication persisted on the lease.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PublishedTcpPort {
    pub host_port: u16,
    pub guest_port: u16,
}

/// Durable lease for a staged host netdevice pair on a Box bridge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostNetDeviceLease {
    pub schema: String,
    pub network: String,
    pub subnet: String,
    pub bridge_iface: String,
    pub container_iface: String,
    pub peer_iface: String,
    pub guest_name: String,
    pub container_ip: String,
    #[serde(default)]
    pub published_tcp: Vec<PublishedTcpPort>,
    /// Static UDP publications. Absent on v4 leases.
    #[serde(default)]
    pub published_udp: Vec<PublishedTcpPort>,
}

impl HostNetDeviceLease {
    #[allow(clippy::too_many_arguments)] // lease fields are the persisted wire shape
    #[cfg(test)]
    fn new(
        network: &str,
        subnet: String,
        bridge_iface: String,
        container_iface: String,
        peer_iface: String,
        container_ip: Ipv4Addr,
        published_tcp: Vec<PublishedTcpPort>,
        published_udp: Vec<PublishedTcpPort>,
    ) -> Self {
        Self {
            schema: LEASE_SCHEMA.to_string(),
            network: network.to_string(),
            subnet,
            bridge_iface,
            container_iface,
            peer_iface,
            guest_name: GUEST_IFACE_NAME.to_string(),
            container_ip: container_ip.to_string(),
            published_tcp,
            published_udp,
        }
    }
}

pub(crate) fn lease_path(home_dir: &Path, box_id: &str) -> PathBuf {
    home_dir
        .join("boxes")
        .join(box_id)
        .join("sandbox")
        .join("host-netdevice.json")
}

pub(crate) fn teardown_lease(home_dir: &Path, box_id: &str) -> ExecutionManagerResult<()> {
    let path = lease_path(home_dir, box_id);
    let lease = match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str::<HostNetDeviceLease>(&raw).map_err(|error| {
            ExecutionManagerError::Internal(format!(
                "failed to decode host netdevice lease {}: {error}",
                path.display()
            ))
        })?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(ExecutionManagerError::Internal(format!(
                "failed to read host netdevice lease {}: {error}",
                path.display()
            )));
        }
    };

    remove_published_dnat(&lease)?;
    delete_link_if_present(&lease.container_iface)?;
    delete_link_if_present(&lease.peer_iface)?;
    // Network-scoped bridge is shared across endpoints; delete only when empty.
    try_delete_bridge_if_idle(&lease.bridge_iface, &lease.subnet)?;

    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(ExecutionManagerError::Internal(format!(
            "failed to remove host netdevice lease {}: {error}",
            path.display()
        ))),
    }
}

fn remove_published_dnat(lease: &HostNetDeviceLease) -> ExecutionManagerResult<()> {
    #[cfg(target_os = "linux")]
    {
        let mut failures = Vec::new();
        for (protocol, mappings) in [("tcp", &lease.published_tcp), ("udp", &lease.published_udp)] {
            for mapping in mappings {
                let host = mapping.host_port.to_string();
                let dest = format!("{}:{}", lease.container_ip, mapping.guest_port);
                let guest = mapping.guest_port.to_string();
                if let Err(error) = iptables_delete_if_present(&[
                    "-t",
                    "nat",
                    "-D",
                    "PREROUTING",
                    "-p",
                    protocol,
                    "--dport",
                    &host,
                    "-j",
                    "DNAT",
                    "--to-destination",
                    &dest,
                ]) {
                    failures.push(error.to_string());
                }
                if let Err(error) = iptables_delete_if_present(&[
                    "-t",
                    "nat",
                    "-D",
                    "OUTPUT",
                    "-d",
                    "127.0.0.1",
                    "-p",
                    protocol,
                    "--dport",
                    &host,
                    "-j",
                    "DNAT",
                    "--to-destination",
                    &dest,
                ]) {
                    failures.push(error.to_string());
                }
                if let Err(error) = iptables_delete_if_present(&[
                    "-D",
                    "FORWARD",
                    "-d",
                    &lease.container_ip,
                    "-p",
                    protocol,
                    "--dport",
                    &guest,
                    "-j",
                    "ACCEPT",
                ]) {
                    failures.push(error.to_string());
                }
            }
        }
        if !failures.is_empty() {
            return Err(ExecutionManagerError::Unavailable(format!(
                "failed to remove keep-authority published DNAT for container {}: {}",
                lease.container_ip,
                failures.join("; ")
            )));
        }
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = lease;
        Ok(())
    }
}

/// Delete an iptables rule when present; treat "missing rule" as success.
#[cfg(target_os = "linux")]
fn iptables_delete_if_present(delete_args: &[&str]) -> ExecutionManagerResult<()> {
    let mut check_args = delete_args.to_vec();
    if let Some(action) = check_args.iter_mut().find(|arg| **arg == "-D") {
        *action = "-C";
    } else {
        return Err(ExecutionManagerError::Internal(
            "iptables delete args missing -D action".into(),
        ));
    }
    if !iptables_check(&check_args)? {
        return Ok(());
    }
    run_iptables(delete_args)
}

#[cfg(target_os = "linux")]
fn iptables_check(args: &[&str]) -> ExecutionManagerResult<bool> {
    let output = Command::new("iptables")
        .args(args)
        .output()
        .map_err(|error| {
            ExecutionManagerError::Unavailable(format!(
                "failed to query `iptables {}`: {error}",
                args.join(" ")
            ))
        })?;
    Ok(output.status.success())
}

#[cfg(target_os = "linux")]
fn remove_bridge_egress_nat(subnet: &str, bridge_iface: &str) -> ExecutionManagerResult<()> {
    if subnet.is_empty() || bridge_iface.is_empty() {
        return Ok(());
    }
    iptables_delete_if_present(&[
        "-t",
        "nat",
        "-D",
        "POSTROUTING",
        "-s",
        subnet,
        "!",
        "-o",
        bridge_iface,
        "-j",
        "MASQUERADE",
    ])
}

#[cfg(target_os = "linux")]
fn run_iptables(args: &[&str]) -> ExecutionManagerResult<()> {
    let output = Command::new("iptables")
        .args(args)
        .output()
        .map_err(|error| {
            ExecutionManagerError::Unavailable(format!(
                "failed to execute `iptables {}`: {error}",
                args.join(" ")
            ))
        })?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(ExecutionManagerError::Unavailable(format!(
        "`iptables {}` failed: {}",
        args.join(" "),
        stderr.trim()
    )))
}

#[cfg(target_os = "linux")]
fn link_exists(name: &str) -> ExecutionManagerResult<bool> {
    let output = Command::new("ip")
        .args(["link", "show", "dev", name])
        .output()
        .map_err(|error| {
            ExecutionManagerError::Unavailable(format!(
                "failed to query `ip link show dev {name}`: {error}"
            ))
        })?;
    if output.status.success() {
        return Ok(true);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("does not exist")
        || stderr.contains("Cannot find device")
        || stderr.contains("No such device")
    {
        return Ok(false);
    }
    Err(ExecutionManagerError::Unavailable(format!(
        "`ip link show dev {name}` failed: {}",
        stderr.trim()
    )))
}

#[cfg(target_os = "linux")]
fn try_delete_bridge_if_idle(bridge_iface: &str, subnet: &str) -> ExecutionManagerResult<()> {
    if !link_exists(bridge_iface)? {
        return remove_bridge_egress_nat(subnet, bridge_iface);
    }
    let output = Command::new("ip")
        .args(["-o", "link", "show", "master", bridge_iface])
        .output()
        .map_err(|error| {
            ExecutionManagerError::Unavailable(format!(
                "failed to inspect slaves for bridge {bridge_iface}: {error}"
            ))
        })?;
    if !output.status.success() {
        return Err(ExecutionManagerError::Unavailable(format!(
            "failed to list slaves for bridge {bridge_iface}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    if !String::from_utf8_lossy(&output.stdout).trim().is_empty() {
        return Ok(());
    }
    remove_bridge_egress_nat(subnet, bridge_iface)?;
    run_ip(&["link", "del", bridge_iface])
}

#[cfg(not(target_os = "linux"))]
fn try_delete_bridge_if_idle(_bridge_iface: &str, _subnet: &str) -> ExecutionManagerResult<()> {
    Ok(())
}

#[cfg(target_os = "linux")]
fn run_ip(args: &[&str]) -> ExecutionManagerResult<()> {
    let output = Command::new("ip").args(args).output().map_err(|error| {
        ExecutionManagerError::Unavailable(format!(
            "failed to execute `ip {}`: {error}",
            args.join(" ")
        ))
    })?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(ExecutionManagerError::Unavailable(format!(
        "`ip {}` failed: {}",
        args.join(" "),
        stderr.trim()
    )))
}

fn delete_link_if_present(name: &str) -> ExecutionManagerResult<()> {
    #[cfg(target_os = "linux")]
    {
        if !link_exists(name)? {
            return Ok(());
        }
        run_ip(&["link", "del", name])
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = name;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lease_round_trip() {
        let home = tempfile::tempdir().unwrap();
        let id = "22222222-2222-4222-8222-222222222222";
        let lease = HostNetDeviceLease::new(
            "dev",
            "10.88.0.0/24".into(),
            "asterbdev".into(),
            "bv22222222c".into(),
            "bv22222222p".into(),
            "10.88.0.2".parse().unwrap(),
            vec![PublishedTcpPort {
                host_port: 18080,
                guest_port: 80,
            }],
            vec![PublishedTcpPort {
                host_port: 5300,
                guest_port: 53,
            }],
        );
        let path = lease_path(home.path(), id);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_string_pretty(&lease).unwrap()).unwrap();
        let loaded: HostNetDeviceLease =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded, lease);
        teardown_lease(home.path(), id).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn remove_published_tcp_dnat_is_ok_when_no_rules_present() {
        let lease = HostNetDeviceLease::new(
            "dev",
            "10.88.0.0/24".into(),
            "asterbdev".into(),
            "bv33333333c".into(),
            "bv33333333p".into(),
            "10.88.0.3".parse().unwrap(),
            vec![PublishedTcpPort {
                host_port: 18081,
                guest_port: 8080,
            }],
            vec![PublishedTcpPort {
                host_port: 5300,
                guest_port: 53,
            }],
        );
        remove_published_dnat(&lease).unwrap();
    }

    #[test]
    fn v4_lease_without_udp_field_still_loads() {
        let raw = r#"{"schema":"kbox.sandbox-host-netdevice.v4","network":"dev","subnet":"10.88.0.0/24","bridge_iface":"asterb","container_iface":"bv22222222c","peer_iface":"bv22222222p","guest_name":"eth0","container_ip":"10.88.0.2","published_tcp":[]}"#;
        let loaded: HostNetDeviceLease = serde_json::from_str(raw).unwrap();
        assert!(loaded.published_udp.is_empty());
    }
}
