//! Shared stopped guest-native rootfs capture through the maintenance VM.

pub(crate) async fn archive_stopped_guest_native_rootfs<W>(
    record: &crate::state::BoxRecord,
    output: &mut W,
) -> Result<u64, Box<dyn std::error::Error>>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    ensure_stopped_rootfs_is_unowned(record)?;
    let config = crate::boot::config_from_record(record)
        .map_err(|error| -> Box<dyn std::error::Error> { error.into() })?;
    let written =
        kbox_runtime::archive_stopped_guest_native_rootfs(config, record.id.clone(), output)
            .await?;
    if written == 0 {
        return Err("Guest rootfs maintenance archive was empty".into());
    }
    Ok(written)
}

/// Whether the durable runtime route is the Spec050 Sandbox engine.
///
/// Sandbox records have no host PID and no guest exec socket, so host-PID and
/// guest-archive gates must not apply to them.
pub(crate) fn is_sandbox_routed(record: &crate::state::BoxRecord) -> bool {
    record.managed_execution.as_ref().is_some_and(|metadata| {
        metadata.runtime_route == kbox_runtime::ManagedRuntimeRoute::Sandbox
    })
}

/// Archive a running Sandbox box's rootfs through the engine archive API.
pub(crate) async fn archive_live_sandbox_rootfs<W>(
    record: &crate::state::BoxRecord,
    output: &mut W,
) -> Result<u64, Box<dyn std::error::Error>>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    archive_live_sandbox_rootfs_in(&kbox_core::dirs_home(), record, output).await
}

pub(crate) async fn archive_live_sandbox_rootfs_in<W>(
    home: &std::path::Path,
    record: &crate::state::BoxRecord,
    output: &mut W,
) -> Result<u64, Box<dyn std::error::Error>>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    use tokio::io::AsyncWriteExt;
    let archive = kbox_runtime::SandboxLocalExecutionBackend::new(home)
        .export_rootfs_archive(record)
        .await
        .map_err(|error| format!("Cannot archive running box '{}': {error}", record.name))?;
    output.write_all(&archive).await?;
    output.flush().await?;
    Ok(archive.len() as u64)
}

/// Verify that an offline reader cannot race a live or transitional VM owner.
///
/// The lifecycle lock serializes well-behaved CLI operations; this explicit
/// state and PID fence also fails closed for stale records and direct callers.
pub(crate) fn ensure_stopped_rootfs_is_unowned(
    record: &crate::state::BoxRecord,
) -> Result<(), Box<dyn std::error::Error>> {
    if !matches!(
        record.status.as_str(),
        "created" | "stopped" | "dead" | "failed"
    ) {
        return Err(format!(
            "Cannot inspect box '{}' offline while its lifecycle state is {}",
            record.name, record.status
        )
        .into());
    }
    if record.pid.is_some_and(|pid| {
        crate::process::is_process_alive_with_identity(pid, record.pid_start_time)
    }) {
        return Err(format!(
            "Cannot inspect box '{}' offline because its host process is still live",
            record.name
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::fixtures::make_record;

    #[test]
    fn offline_rootfs_ownership_rejects_transitional_state_and_live_pid() {
        let paused = make_record("id", "box", "paused", None);
        assert!(ensure_stopped_rootfs_is_unowned(&paused).is_err());

        let stopped_but_live = make_record("id", "box", "stopped", Some(std::process::id()));
        assert!(ensure_stopped_rootfs_is_unowned(&stopped_but_live).is_err());

        let stopped = make_record("id", "box", "stopped", None);
        ensure_stopped_rootfs_is_unowned(&stopped).unwrap();
    }
}
