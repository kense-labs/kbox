//! Windows smoke test for klibksys
//!
//! Verifies that krun.dll can be loaded and basic API calls work.
//!
//! Run with:
//!   cargo run --target x86_64-pc-windows-msvc --example windows_smoke

#[cfg(windows)]
use klibksys::*;

#[cfg(windows)]
fn main() {
    println!("=== klibksys Windows Smoke Test ===\n");

    unsafe {
        // Test 1: Set log level
        println!("Test 1: kset_log_level");
        let ret = kset_log_level(K_LOG_LEVEL_INFO);
        assert_eq!(ret, 0, "kset_log_level failed: {}", ret);
        println!("  ✓ kset_log_level(INFO) = {}\n", ret);

        // Test 2: Create context
        println!("Test 2: kcreate_ctx");
        let ctx = kcreate_ctx();
        assert!(ctx >= 0, "kcreate_ctx failed: {}", ctx);
        let ctx_id = ctx as u32;
        println!("  ✓ kcreate_ctx() = {}\n", ctx_id);

        // Test 3: Set VM config
        println!("Test 3: kset_vm_config");
        let ret = kset_vm_config(ctx_id, 2, 512);
        assert_eq!(ret, 0, "kset_vm_config failed: {}", ret);
        println!("  ✓ kset_vm_config(2 vCPUs, 512 MiB) = {}\n", ret);

        // Test 4: Disable implicit console
        println!("Test 4: kdisable_implicit_console");
        let ret = kdisable_implicit_console(ctx_id);
        assert_eq!(ret, 0, "kdisable_implicit_console failed: {}", ret);
        println!("  ✓ kdisable_implicit_console() = {}\n", ret);

        // Test 5: Disable implicit vsock
        println!("Test 5: kdisable_implicit_vsock");
        let ret = kdisable_implicit_vsock(ctx_id);
        assert_eq!(ret, 0, "kdisable_implicit_vsock failed: {}", ret);
        println!("  ✓ kdisable_implicit_vsock() = {}\n", ret);

        // Test 6: Free context
        println!("Test 6: kfree_ctx");
        let ret = kfree_ctx(ctx_id);
        assert_eq!(ret, 0, "kfree_ctx failed: {}", ret);
        println!("  ✓ kfree_ctx() = {}\n", ret);
    }

    println!("=== All tests passed! ===");
}

#[cfg(not(windows))]
fn main() {}
