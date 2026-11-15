use super::*;

#[test]
fn new_exec_fingerprint_preserves_v1_to_v6_encodings_and_ordered_values() {
    let original = serde_json::json!({"caps":["z","a"],"argv":["z","a"]});
    let normalized = serde_json::json!({"caps":["a","z"],"argv":["z","a"]});
    let old = request_digest(&original, "test").unwrap();
    let new = request_digest(&original, "test")
        .unwrap()
        .with_current(&normalized, "test")
        .unwrap();
    for version in [
        OPERATION_SCHEMA_VERSION_V1,
        OPERATION_SCHEMA_VERSION_V2,
        OPERATION_SCHEMA_VERSION_V3,
        OPERATION_SCHEMA_VERSION_V4,
        OPERATION_SCHEMA_VERSION_V5,
        OPERATION_SCHEMA_VERSION_V6,
    ] {
        assert_eq!(new.for_schema(version), old.for_schema(version));
    }
    assert_ne!(new.current(), old.current());
    assert_eq!(
        new.current(),
        request_digest(&normalized, "test").unwrap().current()
    );
    let reordered_argv = serde_json::json!({"caps":["a","z"],"argv":["a","z"]});
    assert_ne!(
        new.current(),
        request_digest(&reordered_argv, "test").unwrap().current()
    );
}
