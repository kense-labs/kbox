use super::*;

#[test]
fn guest_ulimits_keep_linux_ids_values_and_infinity() {
    for (name, id) in [
        ("cpu", 0),
        ("fsize", 1),
        ("data", 2),
        ("stack", 3),
        ("core", 4),
        ("rss", 5),
        ("nproc", 6),
        ("nofile", 7),
        ("memlock", 8),
        ("locks", 10),
        ("sigpending", 11),
        ("msgqueue", 12),
        ("nice", 13),
        ("rtprio", 14),
        ("rttime", 15),
    ] {
        let parsed = GuestRlimit::parse(&format!("{}=10:20", name.to_ascii_uppercase())).unwrap();
        assert_eq!(parsed.resource().as_str(), name);
        assert_eq!(parsed.resource().linux_id(), id);
        assert_eq!(parsed.guest_spec(), format!("{id}=10:20"));
    }
    let parsed = GuestRlimit::parse("core=0:-1").unwrap();
    assert_eq!(parsed.soft(), 0);
    assert_eq!(parsed.hard(), u64::MAX);
}

#[test]
fn guest_ulimits_refuse_unknown_malformed_escaping_duplicate_or_unbounded_input() {
    for input in [
        "nofile",
        "nofile=1",
        "x=1:2",
        "nofile=+1:2",
        "nofile= 1:2",
        "nofile=2:1",
        "nofile=1:2:3",
        "nofile=-2:-1",
        "nofile=1:2\0",
        "nofile=18446744073709551616:-1",
    ] {
        assert!(GuestRlimit::parse(input).is_err(), "{input:?}");
    }
    assert!(GuestRlimit::parse(&format!("core={}:0", "0".repeat(260))).is_err());
    assert!(GuestRlimit::parse_specs(&["nofile=1:2".into(), "NOFILE=2:2".into()]).is_err());
    assert!(GuestRlimit::parse_specs(&vec!["core=0:0".into(); 16]).is_err());
}
