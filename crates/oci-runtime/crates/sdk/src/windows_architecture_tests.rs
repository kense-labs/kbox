use super::*;

#[cfg(all(windows, target_arch = "aarch64"))]
#[test]
fn native_arm64_process_refuses_current_whpx_dependencies() {
    assert_eq!(
        admit_windows_whpx_host().unwrap_err().code,
        ErrorCode::Unsupported
    );
}

#[test]
fn whpx_admission_requires_native_x64_and_refuses_emulation_unknown_and_arm64ec() {
    assert_eq!(validate("x86_64", 0, 0x8664), Ok(()));
    for (arch, process, native) in [
        ("aarch64", 0, 0xaa64),
        ("x86_64", 0x8664, 0xaa64),
        ("x86_64", 0, 0xaa64),
        ("x86", 0x014c, 0x8664),
        ("x86_64", 0x014c, 0x8664),
        ("arm64ec", 0xa641, 0xaa64),
        ("x86_64", 0, 0),
        ("unknown", 0, 0x8664),
        ("x86_64", 0x1234, 0x8664),
    ] {
        let error = validate(arch, process, native).unwrap_err();
        assert_eq!(
            error.code,
            ErrorCode::Unsupported,
            "{arch}/{process:#x}/{native:#x}"
        );
        assert!(!error.retryable);
        assert_eq!(error.operation.as_deref(), Some(OPERATION));
    }
}
