//! `kbox export` command — Export a box's filesystem to a tar archive.

use clap::Args;

use crate::resolve;
use crate::state::StateFile;

#[derive(Args)]
pub struct ExportArgs {
    /// Box name or ID to export
    pub name: String,

    /// Output file path (e.g., "mybox.tar")
    #[arg(short, long)]
    pub output: String,
}

pub async fn execute(args: ExportArgs) -> Result<(), Box<dyn std::error::Error>> {
    let initial_state = StateFile::load_default()?;
    let box_id = resolve::resolve(&initial_state, &args.name)?.id.clone();
    let lifecycle_lock = crate::lifecycle::acquire_box_lifecycle_lock(&box_id).await?;
    let state = StateFile::load_default()?;
    let record = state.find_by_id(&box_id).ok_or_else(|| {
        format!(
            "Box '{}' was removed while waiting for its lifecycle lock",
            args.name
        )
    })?;

    record.require_claimed()?;
    if record.status == "running" {
        export_live(record, &args.output).await?;
        drop(lifecycle_lock);
    } else if record.status == "paused" {
        return Err(format!(
            "Cannot export paused MicroVM box '{}'; resume it first",
            record.name
        )
        .into());
    } else if kbox_runtime::rootfs::guest_native_ext4_generation_exists(&record.box_dir)? {
        let mut file = tokio::fs::File::create(&args.output)
            .await
            .map_err(|error| format!("Failed to create {}: {error}", args.output))?;
        super::rootfs_capture::archive_stopped_guest_native_rootfs(record, &mut file).await?;
        file.sync_all().await?;
        drop(lifecycle_lock);
    } else {
        let rootfs_dir = super::resolve_box_rootfs(&record.box_dir)
            .ok_or_else(|| rootfs_not_found_message(&args.name, &record.box_dir))?;
        // Match stopped commit: fail closed without guest rootfs metadata so
        // NTFS host mode/ownership is never archived as guest filesystem truth.
        let rootfs_metadata = super::commit::read_guest_rootfs_metadata(&rootfs_dir)
            .map_err(|error| format!("Cannot export stopped box '{}': {error}", record.name))?;
        super::commit::create_tar_from_guest_metadata(
            &rootfs_dir,
            &rootfs_metadata,
            std::path::Path::new(&args.output),
        )?;
        drop(lifecycle_lock);
    }

    let size = std::fs::metadata(&args.output)
        .map(|m| m.len())
        .unwrap_or(0);

    println!("{}", export_success_line(&args.name, &args.output, size));
    Ok(())
}

/// Live-export transport selected from the record's durable runtime route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiveExportRoute {
    /// Spec050 Sandbox: engine archive API; no host PID, no guest exec socket.
    Sandbox,
    /// MicroVM guest archive over the exec socket, gated on a live host PID.
    Guest,
}

fn live_export_route(record: &crate::state::BoxRecord) -> LiveExportRoute {
    if super::rootfs_capture::is_sandbox_routed(record) {
        LiveExportRoute::Sandbox
    } else {
        LiveExportRoute::Guest
    }
}

async fn export_live(
    record: &crate::state::BoxRecord,
    output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match live_export_route(record) {
        LiveExportRoute::Sandbox => export_live_sandbox(record, output).await,
        LiveExportRoute::Guest => export_live_guest(record, output).await,
    }
}

async fn export_live_sandbox(
    record: &crate::state::BoxRecord,
    output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    export_live_sandbox_in(&kbox_core::dirs_home(), record, output).await
}

async fn export_live_sandbox_in(
    home: &std::path::Path,
    record: &crate::state::BoxRecord,
    output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = tokio::fs::File::create(output)
        .await
        .map_err(|error| format!("Failed to create {output}: {error}"))?;
    if let Err(error) =
        super::rootfs_capture::archive_live_sandbox_rootfs_in(home, record, &mut file).await
    {
        drop(file);
        // Never leave a truncated archive that looks like a finished export.
        let _ = tokio::fs::remove_file(output).await;
        return Err(error);
    }
    file.sync_all().await?;
    Ok(())
}

#[cfg(unix)]
async fn export_live_guest(
    record: &crate::state::BoxRecord,
    output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let live_pid = record.pid.is_some_and(|pid| {
        crate::process::is_process_alive_with_identity(pid, record.pid_start_time)
    });
    if !live_pid {
        return Err(format!(
            "Cannot export running box '{}' because its host process is not live",
            record.name
        )
        .into());
    }
    if !record.exec_socket_path.exists() {
        return Err(format!(
            "Cannot export running box '{}' because its guest archive endpoint is unavailable",
            record.name
        )
        .into());
    }
    let client = kbox_runtime::ExecClient::connect(&record.exec_socket_path).await?;
    let mut file = tokio::fs::File::create(output)
        .await
        .map_err(|error| format!("Failed to create {output}: {error}"))?;
    let written = client.archive_rootfs(&mut file, true).await?;
    if written == 0 {
        return Err("Guest rootfs archive was empty".into());
    }
    file.sync_all().await?;
    Ok(())
}

#[cfg(not(unix))]
async fn export_live_guest(
    record: &crate::state::BoxRecord,
    _output: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    Err(format!(
        "Live filesystem export is unavailable for box '{}' on this platform",
        record.name
    )
    .into())
}

fn rootfs_not_found_message(name: &str, box_dir: &std::path::Path) -> String {
    format!(
        "Rootfs not found for box '{}' under {} (looked for merged/ and rootfs/). \
         For overlay-backed boxes the filesystem is only available while the box exists; \
         export a running box.",
        name,
        box_dir.display()
    )
}

fn export_success_line(name: &str, output: &str, size: u64) -> String {
    format!(
        "Exported {} to {} ({})",
        name,
        output,
        crate::output::format_bytes(size)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn rootfs_not_found_message_mentions_box_path_and_expected_dirs() {
        let message = rootfs_not_found_message("web", Path::new("/tmp/kbox/boxes/web"));

        assert!(message.contains("Rootfs not found for box 'web'"));
        assert!(message.contains("/tmp/kbox/boxes/web"));
        assert!(message.contains("merged/ and rootfs/"));
        assert!(message.contains("export a running box"));
    }

    fn managed_record(
        home: &Path,
        status: &str,
        route: kbox_runtime::ManagedRuntimeRoute,
    ) -> crate::state::BoxRecord {
        use kbox_core::{BoxConfig, CreateExecutionRequest, ExecutionGeneration, OperationId};

        let id = "22222222-2222-4222-8222-222222222222";
        // A Sandbox record has no host PID and no guest exec socket.
        let mut record = crate::test_helpers::fixtures::make_record(id, "sbx", status, None);
        record.box_dir = home.join("boxes").join(id);
        record.exec_socket_path = home.join("missing-exec.sock");
        let mut metadata = kbox_runtime::ManagedExecutionMetadata::new(
            OperationId::new("export-route-create").unwrap(),
            ExecutionGeneration::INITIAL,
            CreateExecutionRequest {
                external_sandbox_id: "export-route-external".to_string(),
                config: BoxConfig {
                    image: record.image.clone(),
                    ..Default::default()
                },
                labels: std::collections::BTreeMap::new(),
                policy: Default::default(),
                rootfs_snapshot_id: None,
            },
        )
        .unwrap();
        metadata.runtime_route = route;
        record.managed_execution = Some(metadata);
        record
    }

    #[test]
    fn sandbox_record_selects_engine_archive_not_the_host_pid_guest_path() {
        use kbox_runtime::ManagedRuntimeRoute;
        let home = Path::new("/tmp/kbox-export-route");

        let sandbox = managed_record(home, "running", ManagedRuntimeRoute::Sandbox);
        assert!(sandbox.pid.is_none());
        assert_eq!(live_export_route(&sandbox), LiveExportRoute::Sandbox);

        // MicroVM and OCI live export keep the existing guest-archive path.
        for route in [ManagedRuntimeRoute::BoxVm, ManagedRuntimeRoute::OciSdk] {
            let record = managed_record(home, "running", route);
            assert_eq!(live_export_route(&record), LiveExportRoute::Guest);
        }
        let legacy = crate::test_helpers::fixtures::make_record("id", "vm", "running", Some(1));
        assert_eq!(live_export_route(&legacy), LiveExportRoute::Guest);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn microvm_live_export_still_requires_a_live_host_process() {
        let record = crate::test_helpers::fixtures::make_record("id", "vm", "running", None);
        let error = export_live(&record, "never-written.tar")
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("host process is not live"), "{error}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sandbox_live_export_without_pid_reports_engine_failure_and_leaves_no_file() {
        let home = tempfile::tempdir().unwrap();
        let record = managed_record(
            home.path(),
            "running",
            kbox_runtime::ManagedRuntimeRoute::Sandbox,
        );
        std::fs::create_dir_all(&record.box_dir).unwrap();
        std::fs::write(record.box_dir.join("sandbox.container_id"), "cafebabe").unwrap();
        let output = home.path().join("out.tar");

        let error = export_live_sandbox_in(home.path(), &record, output.to_str().unwrap())
            .await
            .unwrap_err()
            .to_string();

        assert!(!error.contains("host process is not live"), "{error}");
        assert!(error.contains("daemon unreachable"), "{error}");
        assert!(!output.exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sandbox_live_export_writes_the_engine_archive_without_a_host_pid() {
        use base64::Engine;
        use std::io::{Read, Write};

        let home = tempfile::tempdir().unwrap();
        let record = managed_record(
            home.path(),
            "running",
            kbox_runtime::ManagedRuntimeRoute::Sandbox,
        );
        std::fs::create_dir_all(&record.box_dir).unwrap();
        std::fs::write(record.box_dir.join("sandbox.container_id"), "cafebabe").unwrap();

        let mut header = tar::Header::new_gnu();
        header.set_size(4);
        header.set_mode(0o644);
        let mut builder = tar::Builder::new(Vec::new());
        builder
            .append_data(&mut header, "etc/hostname", &b"box\n"[..])
            .unwrap();
        let archive = builder.into_inner().unwrap();
        let stat = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(
            r#"{"name":"/","size":4096,"mode":2147484141,"mtime":"2026-01-02T03:04:05Z","linkTarget":""}"#,
        );

        let socket = home.path().join(".kbox/run/docker.sock");
        std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
        let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        let served = archive.clone();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut raw = vec![0_u8; 8192];
            let n = stream.read(&mut raw).unwrap();
            let head = String::from_utf8_lossy(&raw[..n]).into_owned();
            let mut reply = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\nX-Docker-Container-Path-Stat: {stat}\r\n\r\n",
                served.len()
            )
            .into_bytes();
            reply.extend_from_slice(&served);
            stream.write_all(&reply).unwrap();
            head
        });
        let output = home.path().join("out.tar");

        export_live_sandbox_in(home.path(), &record, output.to_str().unwrap())
            .await
            .unwrap();
        let head = server.join().unwrap();

        assert!(head.starts_with("GET /containers/cafebabe/archive?path=%2F "));
        assert_eq!(std::fs::read(&output).unwrap(), archive);
    }

    #[test]
    fn export_success_line_formats_archive_size() {
        assert_eq!(
            export_success_line("web", "web.tar", 1536),
            "Exported web to web.tar (1.5 KB)"
        );
    }
}
