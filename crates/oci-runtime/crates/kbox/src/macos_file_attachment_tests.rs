use super::*;
use koci_agent_protocol::{AgentVmDirectorySourceIdentity, AgentVmFileAttachment};
use koci_sdk::{
    CreateAttachments, FileGrant, FileGrants, FileSourceIdentity, OciBundle, ProcessIo,
    StorageAccessMode, StorageAttachmentId,
};
use serde_json::json;
use std::os::unix::fs::PermissionsExt;

#[test]
fn pinned_file_reverifies_identity_without_freezing_live_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let share = root.join("share");
    fs::create_dir(&share).unwrap();
    let source = root.join("中文 input");
    fs::write(&source, b"").unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o666)).unwrap();
    let m = fs::metadata(&source).unwrap();
    let id = FileSourceIdentity::new(m.dev(), m.ino()).unwrap();
    let grants = FileGrants::new(vec![FileGrant::new(
        0,
        StorageAttachmentId::new("file").unwrap(),
        id,
        StorageAccessMode::ReadOnly,
    )
    .unwrap()])
    .unwrap();
    let mut config = serde_json::to_value(koci_sdk::oci_spec::runtime::Spec::default()).unwrap();
    config["root"] = json!({"path":"rootfs","readonly":true});
    config["mounts"] =
        json!([{"type":"bind","source":source,"destination":"/exact","options":["bind","ro"]}]);
    config["annotations"] =
        json!({koci_sdk::FILE_GRANTS_EXTENSION:grants.annotation_value().unwrap()});
    let bundle = OciBundle::from_json(root.join("bundle"), config.to_string()).unwrap();
    let a = CreateAttachments::from_bundle(&bundle, ProcessIo::default()).unwrap();
    let tag = koci_agent_protocol::file_tag(bundle.config_digest(), "file").unwrap();
    let projection = koci_agent_protocol::file_projection_root(&share).join(tag);
    fs::create_dir_all(&projection).unwrap();
    fs::set_permissions(&projection, fs::Permissions::from_mode(0o700)).unwrap();
    fs::hard_link(&source, projection.join("file")).unwrap();
    let d = fs::metadata(&projection).unwrap();
    let grant = AgentVmFileAttachment::new(
        a.storage()[0].clone(),
        source.to_str().unwrap(),
        id,
        AgentVmDirectorySourceIdentity::new(d.dev(), d.ino()).unwrap(),
        bundle.config_digest(),
    )
    .unwrap();
    let pinned = MacosFileAttachment::open(grant.clone(), &share).unwrap();
    fs::write(&source, b"live bytes and size changed").unwrap();
    pinned.reverify().unwrap();
    assert_eq!(
        fs::read(pinned.alias_path()).unwrap(),
        b"live bytes and size changed"
    );
    fs::write(projection.join("sibling"), b"must never export").unwrap();
    assert!(pinned.reverify().is_err());
    fs::remove_file(projection.join("sibling")).unwrap();
    fs::hard_link(&source, root.join("extra-link")).unwrap();
    assert!(pinned.reverify().is_err());
    fs::remove_file(root.join("extra-link")).unwrap();
    let retained = root.join("retained");
    fs::rename(&source, &retained).unwrap();
    fs::write(&source, b"replacement").unwrap();
    assert!(pinned.reverify().is_err());
    assert!(MacosFileAttachment::open(grant, &share).is_err());
    assert_eq!(
        fs::read(pinned.alias_path()).unwrap(),
        b"live bytes and size changed"
    );
    assert_eq!(fs::read(&source).unwrap(), b"replacement");
}
