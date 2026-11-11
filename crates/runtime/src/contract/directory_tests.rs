use super::*;

#[test]
fn live_directory_references_have_an_independent_closed_mount_kind() {
    let source = RuntimeMountSource::Directory {
        reference: "directory://workspace/one".into(),
    };
    source.validate().unwrap();
    assert_eq!(source.kind(), MountKind::Directory);
    let value = serde_json::to_value(&source).unwrap();
    assert_eq!(value["kind"], "directory");
    assert_eq!(
        serde_json::from_value::<RuntimeMountSource>(value).unwrap(),
        source
    );
    for reference in ["", "not a reference", "bad\0reference"] {
        assert!(RuntimeMountSource::Directory {
            reference: reference.into()
        }
        .validate()
        .is_err());
    }
    assert!(
        serde_json::from_str::<RuntimeMountSource>(r#"{"kind":"unknown","reference":"one"}"#)
            .is_err()
    );
}
