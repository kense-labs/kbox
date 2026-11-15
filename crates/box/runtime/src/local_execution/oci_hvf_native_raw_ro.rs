//! Native public-SDK raw RO transport; Host syscall evidence is a separate native test.
use super::*;
use koci_sdk::{
    ContainerId, ContainerTarget, CreateAttachments, CreateRequest, DeleteMode, DeleteRequest,
    IoMode, IsolationRequest, ListRequest, OciBundle, OperationContext, ProcessId, ProcessIo,
    ProcessTarget, ReadOutputRequest, RuntimeClient, RuntimeTransportClient, StartRequest,
    StorageAccessMode, StorageAttachmentId, StorageCleanup, StorageOwnership, WaitRequest,
};
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;

fn operation(value: &str) -> OperationContext {
    OperationContext::new(koci_sdk::OperationId::new(value).unwrap())
}

async fn cleanup_sdk(root: &Path, client: &RuntimeClient) {
    for (index, record) in client
        .list(ListRequest::default())
        .await
        .unwrap()
        .into_iter()
        .enumerate()
    {
        assert_eq!(record.state.id(), "raw-ro-native");
        if *record.state.status() == koci_sdk::oci_spec::runtime::ContainerState::Creating {
            let directory = root.join("service/state/containers/raw-ro-native");
            let stored: serde_json::Value =
                serde_json::from_slice(&fs::read(directory.join("record.json")).unwrap()).unwrap();
            let bundle = OciBundle::from_json(
                record.state.bundle(),
                fs::read_to_string(directory.join("config.json")).unwrap(),
            )
            .unwrap();
            let request = CreateRequest {
                context: operation("raw-ro-create"),
                id: ContainerId::new("raw-ro-native").unwrap(),
                bundle,
                isolation: IsolationRequest::DedicatedVm,
                attachments: serde_json::from_value(stored["attachments"].clone()).unwrap(),
            };
            fs::write(
                root.join("raw-ro-exact-create-cleanup-request.json"),
                serde_json::to_vec_pretty(&request).unwrap(),
            )
            .unwrap();
            let resumed = client.create(request).await.unwrap();
            assert_eq!(
                *resumed.state.status(),
                koci_sdk::oci_spec::runtime::ContainerState::Created
            );
            assert_eq!(resumed.generation, record.generation);
        }
        client
            .delete(DeleteRequest {
                context: operation(&format!("raw-ro-cleanup-{index}")),
                target: ContainerTarget::exact(
                    ContainerId::new(record.state.id().clone()).unwrap(),
                    record.generation,
                ),
                mode: DeleteMode::Force,
            })
            .await
            .unwrap();
    }
    assert!(client
        .list(ListRequest::default())
        .await
        .unwrap()
        .is_empty());
    fs::write(root.join("raw-ro-sdk-cleanup.json"), b"[]\n").unwrap();
}

#[test]
#[ignore = "requires sentinel-owned root and fixed installed independent HVF assets"]
fn native_raw_ro_disks_reject_guest_remount_child_and_writes_with_unchanged_host_image() {
    std::thread::Builder::new().stack_size(32*1024*1024).spawn(|| {
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());assert_eq!(root.canonicalize().unwrap(),root);assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");
        let host=Host::start(&root,1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let _manager=connect(&root,OciMigrationPolicy::MicrovmViaOci).await;
            let stream=tokio::net::UnixStream::connect(root.join("service/runtime.sock")).await.unwrap();let client=RuntimeClient::new(RuntimeTransportClient::from_io(stream).await.unwrap());
            let result=std::panic::AssertUnwindSafe(async {
                let staging=root.join("raw-inputs");fs::create_dir(&staging).unwrap();
                let image=crate::oci::OciImage::from_path(&PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_RAW_IMAGE_LAYOUT").unwrap())).unwrap();
                let mut tree=crate::rootfs::OciWorkloadRootfs::load(&image,&staging).unwrap();
                let readonly=tree.publish_volume("/ownership",&staging.join("readonly"),crate::rootfs::Ext4ArtifactOptions::from_disk_mib(16,[1;16]).unwrap()).unwrap();
                let writable=tree.publish_volume("/ownership",&staging.join("writable"),crate::rootfs::Ext4ArtifactOptions::from_disk_mib(16,[2;16]).unwrap()).unwrap();
                let workload=tree.publish(&staging.join("workload"),crate::rootfs::Ext4ArtifactOptions::from_disk_mib(64,[3;16]).unwrap()).unwrap();
                for disk in [&readonly.disk,&writable.disk] { fs::set_permissions(disk,fs::Permissions::from_mode(0o600)).unwrap(); }
                fs::write(staging.join("canary"),b"never-granted").unwrap();
                let before=Sha256::digest(fs::read(&readonly.disk).unwrap());
                let id=ContainerId::new("raw-ro-native").unwrap();let create_context=operation("raw-ro-create");
                let handoff=koci_sdk::runtime_bundle_handoff_directory(&root.join("service/runtime"),&id,&create_context.operation_id).unwrap();
                fs::create_dir_all(handoff.join("rootfs")).unwrap();let mut current=Some(handoff.as_path());while let Some(path)=current { if path==root.join("service/runtime") {break;}fs::set_permissions(path,fs::Permissions::from_mode(0o700)).unwrap();current=path.parent(); }
                fs::copy(&workload.disk,handoff.join(koci_sdk::ROOTFS_IMAGE_FILE_NAME)).unwrap();fs::set_permissions(handoff.join(koci_sdk::ROOTFS_IMAGE_FILE_NAME),fs::Permissions::from_mode(0o600)).unwrap();
                let descriptor=koci_sdk::RootfsImage::new(format!("sha256:{:x}",Sha256::digest(fs::read(&workload.disk).unwrap())),64*1024*1024,image.manifest_digest()).unwrap();
                let script=r#"set -eu
test "$(cat /probe-ro/secret)" = seed
cat /proc/partitions
cat /sys/class/block/vdc/dev
test "$(cat /sys/class/block/vdc/dev)" = 254:32
/sbin/blockdev --setrw /dev/qual-raw-ro
/sbin/blockdev --getro /dev/qual-raw-ro
# BLKROSET clears the software flag; the hardware RO feature must survive.
test "$(/sbin/blockdev --getro /dev/qual-raw-ro)" = 1
if printf x | dd of=/dev/qual-raw-ro bs=1 seek=8192 count=1 conv=notrunc 2>/dd-error; then exit 35; fi
cat /dd-error
grep 'Operation not permitted' /dd-error
if printf illegal >> /probe-ro/secret; then exit 31; fi
if mount -o remount,rw /probe-ro; then remount=success; else remount=refused; fi
if printf illegal >> /probe-ro/secret; then exit 32; fi
if /bin/sh -c 'printf illegal > /probe-ro/child'; then exit 33; fi
mkdir /fresh
device=/dev/qual-raw-ro
if mount -t ext4 -o rw "$device" /fresh; then
    if printf illegal > /fresh/fresh; then exit 34; fi
    fresh=mounted
else fresh=refused; fi
printf allowed > /probe-rw/control
test "$(cat /probe-rw/control)" = allowed
test "$(cat /probe-ro/secret)" = seed
printf 'raw-ro:passed remount=%s fresh=%s\n' "$remount" "$fresh"
sleep 2
"#;
                let spec=json!({"ociVersion":"1.3.0","root":{"path":"rootfs","readonly":false},
                    "process":{"cwd":"/","args":["/bin/sh","-c",script],"env":["PATH=/bin:/usr/bin"],"user":{"uid":0,"gid":0},"capabilities":{"bounding":["CAP_SYS_ADMIN","CAP_DAC_OVERRIDE"],"effective":["CAP_SYS_ADMIN","CAP_DAC_OVERRIDE"],"permitted":["CAP_SYS_ADMIN","CAP_DAC_OVERRIDE"]}},
                    "linux":{"devices":[{"path":"/dev/qual-raw-ro","type":"b","major":254,"minor":32,"fileMode":384,"uid":0,"gid":0}],"namespaces":[{"type":"mount"},{"type":"pid"},{"type":"ipc"},{"type":"uts"},{"type":"network"}],"resources":{"devices":[{"allow":true,"type":"b","major":254,"minor":32,"access":"rwm"}],"cpu":{"quota":100000,"period":100000},"memory":{"limit":268435456},"pids":{"limit":32}}},
                    "mounts":[{"type":"proc","source":"proc","destination":"/proc"},{"type":"sysfs","source":"sysfs","destination":"/sys","options":["ro","nosuid","nodev","noexec"]},{"type":"ext4","source":readonly.disk,"destination":"/probe-ro","options":["ro","nosuid","nodev"]},{"type":"ext4","source":writable.disk,"destination":"/probe-rw","options":["rw","nosuid","nodev"]}],
                    "annotations":{koci_sdk::ROOTFS_IMAGE_EXTENSION:descriptor.annotation_value().unwrap(),koci_sdk::RAW_STORAGE_IMAGES_EXTENSION:koci_sdk::RAW_STORAGE_IMAGES_VALUE,koci_sdk::RUNTIME_BUNDLE_HANDOFF_EXTENSION:koci_sdk::RUNTIME_BUNDLE_HANDOFF_MOVE_V2}});
                fs::write(handoff.join("config.json"),spec.to_string()).unwrap();fs::set_permissions(handoff.join("config.json"),fs::Permissions::from_mode(0o600)).unwrap();
                let bundle=OciBundle::from_json(&handoff,spec.to_string()).unwrap();
                let mut attachments=CreateAttachments::from_bundle(&bundle,ProcessIo { stdin:IoMode::Null,stdout:IoMode::Capture,stderr:IoMode::Capture,terminal_size:None }).unwrap();
                for (index,identity,access) in [(2,"ro",StorageAccessMode::ReadOnly),(3,"rw",StorageAccessMode::ReadWrite)] { attachments=attachments.attach_storage_mount(&bundle,index,StorageAttachmentId::new(identity).unwrap(),access,StorageOwnership::Caller,StorageCleanup::DetachOnly).unwrap(); }
                attachments=attachments.with_runtime_bundle_storage_handoff(&bundle).unwrap();
                let created=client.create(CreateRequest {context:create_context,id:id.clone(),bundle,isolation:IsolationRequest::DedicatedVm,attachments}).await.unwrap();
                let target=ContainerTarget::exact(id,created.generation);
                let manifest_path=root.join("service/runtime/shares").join(target.id.as_str()).join(created.generation.0.to_string()).join(".koci-agent-vm-attachments.json");
                let manifest:serde_json::Value=serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();assert_eq!(manifest["schemaVersion"],"koci.agent-vm-attachments.v7");fs::write(root.join("raw-ro-manifest.json"),serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
                client.start(StartRequest {context:operation("raw-ro-start"),target:target.clone()}).await.unwrap();
                let exit=client.wait(WaitRequest {target:target.clone(),timeout_ms:Some(20000)}).await.unwrap();
                let output=client.read_output(ReadOutputRequest {process:ProcessTarget {container:target.clone(),process_id:ProcessId::init()},after_sequence:0,max_bytes:65536,wait_timeout_ms:Some(100)}).await.unwrap();
                fs::write(root.join("raw-ro-output.json"),serde_json::to_vec_pretty(&output).unwrap()).unwrap();
                assert_eq!(exit,koci_sdk::ExitStatus::exited(0).unwrap(),"raw disk probe output: {output:?}");
                assert!(output.iter().any(|chunk| chunk.data.windows(b"raw-ro:passed".len()).any(|part| part==b"raw-ro:passed")));
                client.delete(DeleteRequest {context:operation("raw-ro-delete"),target,mode:DeleteMode::Force}).await.unwrap();
                assert_eq!(Sha256::digest(fs::read(&readonly.disk).unwrap()),before);
                assert!(writable.disk.exists());assert_eq!(fs::read(staging.join("canary")).unwrap(),b"never-granted");
                fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","hostRoImageUnchanged":true,"roSha256":hex::encode(before),"callerDisksRetained":true,"guestRemountChildFreshMountAttempts":true,"guestHardwareReadonlyRetained":true,"rawDeviceWriteRefused":true,"hostSyscallProof":"separate macos_vm_attachment::storage native sandbox test"})).unwrap()).unwrap();
            }).catch_unwind().await;
            cleanup_sdk(&root,&client).await;cleanup(&root).await;drop(host);assert!(!root.join("service/runtime.sock").exists());
            if let Err(error)=result { std::panic::resume_unwind(error); }
        });
    }).unwrap().join().unwrap();
}

#[test]
#[ignore = "maintenance only: exact Create replay for this sentinel-owned failed SDK fixture"]
fn native_cleanup_owned_raw_ro_fixture() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let root = PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());
            assert_eq!(root.canonicalize().unwrap(), root);
            assert_eq!(
                fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),
                "owned-hvf-native-test\n"
            );
            let host = Host::start(&root, 90);
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_stack_size(8 * 1024 * 1024)
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let _manager = connect(&root, OciMigrationPolicy::MicrovmViaOci).await;
                    let stream = tokio::net::UnixStream::connect(root.join("service/runtime.sock"))
                        .await
                        .unwrap();
                    let client =
                        RuntimeClient::new(RuntimeTransportClient::from_io(stream).await.unwrap());
                    cleanup_sdk(&root, &client).await;
                    cleanup(&root).await;
                    drop(host);
                    assert!(!root.join("service/runtime.sock").exists());
                });
        })
        .unwrap()
        .join()
        .unwrap();
}
