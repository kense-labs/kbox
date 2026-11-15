//! Installed Guest Init protocol entry points, not the Linux unit-test binary.
use super::*;
use kbox_core::pty::{parse_frame, PtyFrame, PtyRequest};

#[test]
#[ignore = "requires sentinel-owned root and fixed installed Box/HVF companions"]
fn native_installed_guest_path_guards_refuse_and_enforce_exec_and_pty() {
    std::thread::Builder::new().stack_size(32 * 1024 * 1024).spawn(|| {
        let root=PathBuf::from(std::env::var_os("K_BOX_TEST_HVF_ROOT").unwrap());
        assert_eq!(root.canonicalize().unwrap(),root); assert_eq!(fs::read_to_string(root.join(".kbox-hvf-native")).unwrap(),"owned-hvf-native-test\n");
        let host=Host::start(&root,1);
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).thread_stack_size(8*1024*1024).enable_all().build().unwrap().block_on(async {
            let result=std::panic::AssertUnwindSafe(async {
                let manager=connect(&root,OciMigrationPolicy::LegacyOnly).await;
                let image=std::env::var("K_BOX_TEST_HVF_NAMED_IMAGE").unwrap();
                let req=request(&image,"installed-guest-path-guards");
                let mut lease=manager.create_and_start(req,&OperationId::new("installed-guest-path-guards").unwrap()).await.unwrap();
                let store=ManagedExecutionStore::new(root.join("home/boxes.json"));
                let first=lease.generation;
                for wave in 1..=2 {
                    let record=store.get(&lease.execution_id).unwrap().unwrap(); assert_eq!(record.managed_execution.as_ref().unwrap().runtime_route,ManagedRuntimeRoute::BoxVm);
                    let hash=manager.execute(&lease.execution_id,lease.generation,exec(&["/bin/sh","-c","read pid comm state parent rest < /proc/self/stat; /bin/busybox readlink /proc/1/exe; /bin/busybox sha256sum /proc/1/exe; /bin/busybox readlink /proc/$parent/exe; /bin/busybox sha256sum /proc/$parent/exe"])).await.unwrap(); fs::write(root.join(format!("binary-g{wave}-init-sha.stdout")),&hash.stdout).unwrap(); fs::write(root.join(format!("binary-g{wave}-init-sha.stderr")),&hash.stderr).unwrap(); assert_eq!(hash.exit_code,0);
                    let expected=std::env::var("K_BOX_TEST_EXPECTED_GUEST_INIT_SHA").unwrap(); assert_eq!(String::from_utf8_lossy(&hash.stdout).lines().last().unwrap().split_whitespace().next(),Some(expected.as_str()));
                    let prepare="set -eu; mkdir -p /nested-pathguard; cp -a /bin /lib /nested-pathguard/; mkdir -p /nested-pathguard/hidden-dir /nested-pathguard/readonly-dir; printf private-file > /nested-pathguard/hidden-file; printf private-directory > /nested-pathguard/hidden-dir/canary; printf readonly-seed > /nested-pathguard/readonly-file; printf readonly-dir-seed > /nested-pathguard/readonly-dir/seed";
                    let output=manager.execute(&lease.execution_id,lease.generation,exec(&["/bin/sh","-c",prepare])).await.unwrap(); assert_eq!(output.exit_code,0,"{}",String::from_utf8_lossy(&output.stderr));
                    let guest=crate::ExecClient::connect(&record.exec_socket_path).await.unwrap();
                    let mut negative=exec(&["/bin/sh","-c","printf forbidden > /exec-marker"]); negative.rootfs=Some("/nested-pathguard".into()); negative.env=vec!["K_SEC_MASKED_PATHS=relative".into()];
                    let output=guest.exec_command(&negative).await.unwrap(); fs::write(root.join(format!("binary-g{wave}-exec-refusal.json")),serde_json::to_vec_pretty(&json!({"exit":output.exit_code,"stdout":String::from_utf8_lossy(&output.stdout),"stderr":String::from_utf8_lossy(&output.stderr)})).unwrap()).unwrap();
                    let exec_refused = output.exit_code != 0 && String::from_utf8_lossy(&output.stderr).contains("path restriction");
                    let socket=record.exec_socket_path.with_file_name("pty.sock");
                    let mut client=crate::PtyClient::connect(&socket).await.unwrap();
                    client.send_request(&PtyRequest {cmd:vec!["/bin/sh".into(),"-c".into(),"printf forbidden > /pty-marker".into()],env:vec!["K_SEC_READONLY_PATHS=relative".into()],working_dir:None,rootfs:Some("/nested-pathguard".into()),user:None,cols:80,rows:24}).await.unwrap();
                    let (kind,payload)=tokio::time::timeout(Duration::from_secs(10),client.read_frame()).await.unwrap().unwrap().unwrap(); let frame=parse_frame(kind,payload).unwrap(); let pty_refused=matches!(frame,PtyFrame::Error(ref message) if message.contains("path restriction")); drop(client);
                    let check=manager.execute(&lease.execution_id,lease.generation,exec(&["/bin/sh","-c","for name in exec-marker pty-marker; do if test -e /nested-pathguard/$name; then printf '%s=' $name; cat /nested-pathguard/$name; printf '\n'; else printf '%s=absent\n' $name; fi; done; test ! -e /nested-pathguard/exec-marker && test ! -e /nested-pathguard/pty-marker && test \"$(cat /nested-pathguard/hidden-file)\" = private-file"])).await.unwrap();
                    fs::write(root.join(format!("binary-g{wave}-negative.json")),serde_json::to_vec_pretty(&json!({"execRefused":exec_refused,"ptyRefused":pty_refused,"ptyFrame":format!("{frame:?}"),"markerProbeExit":check.exit_code,"markerProbeStdout":String::from_utf8_lossy(&check.stdout),"markerProbeStderr":String::from_utf8_lossy(&check.stderr)})).unwrap()).unwrap();
                    assert!(exec_refused,"installed Guest allowed unsafe masked path"); assert!(pty_refused,"{frame:?}"); assert_eq!(check.exit_code,0);
                    let probe="set -eu; test ! -s /hidden-file; test -z \"$(/bin/busybox ls -A /hidden-dir)\"; test \"$(/bin/busybox cat /readonly-file)\" = readonly-seed; if (printf forbidden > /readonly-file) 2>/denial; then exit 21; fi; /bin/busybox grep -q 'Read-only file system' /denial; if (printf forbidden > /readonly-dir/new) 2>/denial; then exit 22; fi; /bin/busybox grep -q 'Read-only file system' /denial; printf allowed > /sibling; printf PATH_GUARD_EFFECT_OK";
                    let environment=vec!["K_SEC_MASKED_PATHS=/hidden-file:/hidden-dir".into(),"K_SEC_READONLY_PATHS=/readonly-file:/readonly-dir".into()];
                    let mut command=exec(&["/bin/sh","-c",probe]); command.rootfs=Some("/nested-pathguard".into()); command.env=environment.clone();
                    let output=guest.exec_command(&command).await.unwrap(); fs::write(root.join(format!("binary-g{wave}-exec.stdout")),&output.stdout).unwrap(); fs::write(root.join(format!("binary-g{wave}-exec.stderr")),&output.stderr).unwrap(); assert_eq!(output.exit_code,0,"{}",String::from_utf8_lossy(&output.stderr)); assert_eq!(output.stdout,b"PATH_GUARD_EFFECT_OK");
                    let mut client=crate::PtyClient::connect(&socket).await.unwrap(); client.send_request(&PtyRequest{cmd:vec!["/bin/sh".into(),"-c".into(),probe.into()],env:environment,working_dir:None,rootfs:Some("/nested-pathguard".into()),user:None,cols:80,rows:24}).await.unwrap(); let mut bytes=Vec::new();
                    loop { let (kind,payload)=tokio::time::timeout(Duration::from_secs(20),client.read_frame()).await.unwrap().unwrap().unwrap(); match parse_frame(kind,payload).unwrap() {PtyFrame::Data(data)=>{bytes.extend(data);assert!(bytes.len()<65536);},PtyFrame::Exit(exit)=>{assert_eq!(exit.exit_code,0);break;},frame=>panic!("unexpected PTY frame {frame:?}")} }
                    fs::write(root.join(format!("binary-g{wave}-pty.stdout")),&bytes).unwrap(); assert!(String::from_utf8_lossy(&bytes).contains("PATH_GUARD_EFFECT_OK")); drop(client);
                    if wave==1 { lease=manager.restart(&lease.execution_id,lease.generation,&OperationId::new("installed-path-guard-restart").unwrap()).await.unwrap(); assert!(lease.generation>first); let error=manager.execute(&lease.execution_id,first,exec(&["/bin/true"])).await.unwrap_err(); assert!(matches!(error,kbox_core::ExecutionManagerError::Conflict{..})); }
                }
                manager.kill(&lease.execution_id,lease.generation).await.unwrap();manager.remove(&lease.execution_id,lease.generation).await.unwrap();
                fs::write(root.join("result.json"),serde_json::to_vec_pretty(&json!({"status":"PASS","installedGuestInit":true,"execPtyRefusedBeforeLaunch":true,"maskAndReadonlyEffects":true,"restartSameIdentity":true,"firstGeneration":first,"restartGeneration":lease.generation,"staleGenerationRefused":true})).unwrap()).unwrap();
            }).catch_unwind().await;
            cleanup(&root).await;drop(host);assert!(!root.join("service/runtime.sock").exists());if let Err(error)=result {std::panic::resume_unwind(error);}
        });
    }).unwrap().join().unwrap();
}
