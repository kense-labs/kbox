use super::{decode_guest_mount_spec, encode_guest_mount_spec};

#[test]
fn mount_spec_preserves_unicode_quotes_and_legacy_values() {
    for value in [
        "vol0:/data:copy",
        "vol1:/原生 workspace:ro:file",
        "vol2:/quotes\"and\\slash",
        "/临时 tmp:size=16m",
    ] {
        let encoded = encode_guest_mount_spec(value);
        assert!(encoded.is_ascii() && !encoded.contains(['"', '\\']));
        assert_eq!(decode_guest_mount_spec(&encoded).unwrap(), value);
    }
    assert_eq!(
        encode_guest_mount_spec("vol0:/data:copy"),
        "vol0:/data:copy"
    );
}

#[test]
fn encoded_mount_spec_refuses_malformed_utf8_and_nul() {
    for value in [
        "kbox-b64-v1:!",
        "kbox-b64-v1:/w==",
        "kbox-b64-v1:AA==",
        "vol0:/bad\0",
    ] {
        assert!(decode_guest_mount_spec(value).is_err());
    }
}
