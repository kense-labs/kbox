use crate::{Error, ErrorCode, Result};

const OPERATION: &str = "admit-windows-whpx-host";
const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;

/// Require the native Windows x64 process/host supported by the current VMM.
/// A successful hypervisor probe alone must not admit x64 emulation on ARM64.
#[cfg(windows)]
pub fn admit_windows_whpx_host() -> Result<()> {
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, IsWow64Process2};
    let mut process_machine = 0;
    let mut native_machine = 0;
    // SAFETY: the current-process pseudo handle requires no close; both outputs
    // point to initialized USHORTs and remain valid for the synchronous query.
    if unsafe {
        IsWow64Process2(
            GetCurrentProcess(),
            &mut process_machine,
            &mut native_machine,
        )
    } == 0
    {
        return Err(Error::new(
            ErrorCode::Unavailable,
            format!(
                "cannot establish native Windows architecture: {}",
                std::io::Error::last_os_error()
            ),
        )
        .for_operation(OPERATION));
    }
    validate(std::env::consts::ARCH, process_machine, native_machine)
}

fn validate(process_arch: &str, process_machine: u16, native_machine: u16) -> Result<()> {
    if process_arch == "x86_64"
        && process_machine == 0
        && native_machine == IMAGE_FILE_MACHINE_AMD64
    {
        Ok(())
    } else {
        Err(Error::new(
            ErrorCode::Unsupported,
            format!("WHPX requires a native Windows x86_64 process and host; current libkrun/libkrunfw dependencies do not support Windows ARM64 or emulated execution (process_arch={process_arch}, process_machine=0x{process_machine:04x}, native_machine=0x{native_machine:04x})"),
        ).for_operation(OPERATION))
    }
}

#[cfg(test)]
#[path = "windows_architecture_tests.rs"]
mod tests;
