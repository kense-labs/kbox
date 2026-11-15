use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::PathBuf;

use koci_sdk::{
    ContainerId, CreateAttachments, CreateRequest, ErrorCode, FileGrant, FileGrants,
    FileSourceIdentity, IsolationRequest, ListRequest, LocalIpcEndpoint, OciBundle,
    OperationContext, OperationId, ProcessIo, RuntimeClient, StorageAccessMode,
    StorageAttachmentId,
};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    assert_eq!(args.len(), 3);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let client = RuntimeClient::connect(&LocalIpcEndpoint::unix_socket(&args[0])?).await?;
            let features = client.features().await?;
            let hvf = features.drivers.drivers.iter().find(|d| d.driver == koci_sdk::DriverKind::LibkHvf)
                .ok_or("HVF discovery entry missing")?;
            assert!(hvf.can_launch());
            assert_eq!(hvf.evidence["memory_secret_files"], "unavailable-memory-backing-not-verified");
            assert!(hvf.evidence["memory_secret_files_probe_error"].contains("DiskManagement"));
            for version in [1, 2, 3] {
                assert!(!features.attachments.supports_extension(koci_sdk::MEMORY_SECRET_FILES_EXTENSION, version));
            }
            assert!(client.list(ListRequest::default()).await?.is_empty());

            let caller = args[1].join("caller");
            fs::create_dir(&caller)?;
            fs::set_permissions(&caller, fs::Permissions::from_mode(0o700))?;
            let source = caller.join("synthetic private input");
            fs::write(&source, b"synthetic-only-no-real-secret")?;
            fs::set_permissions(&source, fs::Permissions::from_mode(0o600))?;
            let before = fs::metadata(&source)?;
            let id = ContainerId::new("memory-probe-native")?;
            let context = OperationContext::new(OperationId::new("memory-probe-native-create")?);
            let runtime = args[0].parent().ok_or("service socket has no parent")?.join("runtime");
            let directory = koci_sdk::runtime_bundle_handoff_directory(&runtime, &id, &context.operation_id)?;
            fs::create_dir_all(directory.join("rootfs"))?;
            for path in directory.ancestors().take_while(|p| *p != runtime) {
                fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
            }
            let grants = FileGrants::new(vec![FileGrant::new(
                0,
                StorageAttachmentId::new("memory-probe-native-file")?,
                FileSourceIdentity::new(before.dev(), before.ino())?,
                StorageAccessMode::ReadOnly,
            )?])?;
            let mut config = serde_json::to_value(koci_sdk::oci_spec::runtime::Spec::default())?;
            config["root"] = json!({"path":"rootfs","readonly":false});
            config["process"] = json!({"terminal":false,"user":{"uid":0,"gid":0},"args":["/bin/true"],"env":[],"cwd":"/"});
            config["mounts"] = json!([{"type":"bind","source":source,"destination":"/run/secrets/file","options":["bind","ro"]}]);
            config["annotations"] = json!({
                koci_sdk::FILE_GRANTS_EXTENSION:grants.annotation_value()?,
                koci_sdk::MEMORY_SECRET_FILES_EXTENSION:json!({"schemaVersion":1,"mountIndices":[0]}).to_string(),
                koci_sdk::RUNTIME_BUNDLE_HANDOFF_EXTENSION:koci_sdk::RUNTIME_BUNDLE_HANDOFF_MOVE_V2
            });
            fs::write(directory.join("config.json"), serde_json::to_vec(&config)?)?;
            let bundle = OciBundle::from_json(&directory, config.to_string())?;
            let attachments = CreateAttachments::from_bundle(&bundle, ProcessIo::default())?
                .with_runtime_bundle_storage_handoff(&bundle)?;
            let refusal = client.create(CreateRequest {
                context, id, bundle, isolation: IsolationRequest::DedicatedVm, attachments,
            }).await.expect_err("unverified memory Secret must be refused");
            assert_eq!(refusal.code, ErrorCode::Unsupported);
            assert!(refusal.message.contains(koci_sdk::MEMORY_SECRET_FILES_EXTENSION), "{refusal}");
            assert!(client.list(ListRequest::default()).await?.is_empty());
            let after = fs::metadata(&source)?;
            assert_eq!((before.dev(),before.ino(),before.mode(),before.uid(),before.gid()),
                (after.dev(),after.ino(),after.mode(),after.uid(),after.gid()));
            assert_eq!(fs::read(&source)?,b"synthetic-only-no-real-secret");
            fs::write(&args[2], serde_json::to_vec_pretty(&json!({
                "status":"PASS","features":features,"refusal":refusal,
                "zeroRuntimeRecords":true,"callerSourceIdentityAndBytesPreserved":true,
                "retainedCallerBundle":directory,"callerSource":source
            }))?)?;
            Ok::<(), Box<dyn std::error::Error>>(())
        })
}
