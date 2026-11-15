use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize)]
struct Envelope {
    #[serde(with = "super")]
    process: oci_spec::runtime::Process,
}

#[test]
fn capability_set_reconstruction_has_stable_wire_order_without_reordering_argv() {
    let input = json!({"process":{"user":{"uid":0,"gid":0},"cwd":"/",
        "args":["/bin/echo","z","a"],"capabilities":{
            "bounding":["CAP_SYS_ADMIN","CAP_CHOWN","CAP_DAC_OVERRIDE"],
            "effective":["CAP_SYS_ADMIN","CAP_CHOWN"],
            "permitted":["CAP_SYS_ADMIN","CAP_CHOWN"],"ambient":[],"inheritable":[]}}});
    let first: Envelope = serde_json::from_value(input.clone()).unwrap();
    let expected = serde_json::to_value(first).unwrap();
    assert_eq!(
        expected["process"]["capabilities"]["bounding"],
        json!(["CAP_CHOWN", "CAP_DAC_OVERRIDE", "CAP_SYS_ADMIN"])
    );
    assert_eq!(expected["process"]["args"], input["process"]["args"]);
    for _ in 0..32 {
        let recreated: Envelope = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(serde_json::to_value(recreated).unwrap(), expected);
    }
}
