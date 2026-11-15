use std::process::Command;

use super::{RlimitEntry, RlimitPlan, RlimitResource};

#[test]
fn admission_slot_preserves_infinity_and_unrelated_limits() {
    for (soft, hard, prepared_soft, prepared_hard) in [
        (0, 0, 1, 1),
        (1, 2, 2, 2),
        (16, u64::MAX, 17, u64::MAX),
        (u64::MAX, u64::MAX, u64::MAX, u64::MAX),
    ] {
        let entry = RlimitEntry {
            resource: RlimitResource::Nproc,
            soft,
            hard,
        };
        let prepared = entry.before_credentials();
        assert_eq!(
            (prepared.soft, prepared.hard),
            (prepared_soft, prepared_hard)
        );
        let other = RlimitEntry {
            resource: RlimitResource::Nofile,
            ..entry
        };
        assert_eq!(other.before_credentials(), other);
    }
}

fn probe(plan: &RlimitPlan, root: bool, soft: u64) -> bool {
    if plan.apply_before_credentials().is_err() {
        return false;
    }
    if root {
        // SAFETY: this fork child changes only its own credentials to a fresh UID.
        let uid = 1_000_000 + unsafe { libc::getpid() } as u32;
        if unsafe { libc::setuid(uid) } != 0 {
            return false;
        }
    }
    let admitted = plan.finish_credentials();
    if soft == 0 {
        if !matches!(admitted,Err(error) if error.code == koci_sdk::ErrorCode::ResourceExhausted) {
            return false;
        }
    } else {
        if admitted.is_err() {
            return false;
        }
        let mut actual = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        // SAFETY: `actual` is writable and RLIMIT_NPROC is a valid resource.
        if unsafe { libc::getrlimit(libc::RLIMIT_NPROC, &mut actual) } != 0
            || (actual.rlim_cur, actual.rlim_max) != (soft, soft)
        {
            return false;
        }
        if root {
            // SAFETY: the exact one-slot budget must reject this fork.
            let child = unsafe { libc::fork() };
            if child == 0 {
                // SAFETY: no workload or Rust destructors run on unexpected admission.
                unsafe { libc::_exit(101) }
            }
            if child > 0 {
                let _ = super::super::pid_supervisor::wait_for_child(child);
                return false;
            }
            if std::io::Error::last_os_error().raw_os_error() != Some(libc::EAGAIN) {
                return false;
            }
        }
    }
    let mut status = 0;
    // SAFETY: WNOHANG cannot block; no admission child may remain.
    let waited = unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG) };
    waited == -1 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD)
}

#[test]
fn enforces_nproc_after_uid_transition_in_an_isolated_process() {
    const ENV: &str = "K_OCI_NPROC_ADMISSION_TEST";
    const TEST: &str =
        "executor::rlimit::nproc_tests::enforces_nproc_after_uid_transition_in_an_isolated_process";
    if let Ok(case) = std::env::var(ENV) {
        // SAFETY: getuid has no preconditions.
        let root = unsafe { libc::getuid() } == 0;
        let soft = if case == "zero" {
            0
        } else if root {
            1
        } else {
            u64::MAX
        };
        let plan = RlimitPlan {
            limits: vec![RlimitEntry {
                resource: RlimitResource::Nproc,
                soft,
                hard: soft,
            }],
        };
        let threads = std::fs::read_dir("/proc/self/task")
            .expect("test task inventory")
            .count();
        // SAFETY: the isolated test executable forks a single-threaded probe,
        // using only the prepared limit plan and bounded libc calls before _exit.
        // The libtest coordinator must not consume the one-slot workload budget.
        let child = unsafe { libc::fork() };
        assert!(child >= 0, "fork single-threaded nproc probe");
        if child == 0 {
            let passed = probe(&plan, root, soft);
            // SAFETY: do not return into the inherited libtest coordinator.
            unsafe { libc::_exit(if passed { 0 } else { 101 }) }
        }
        assert_eq!(
            super::super::pid_supervisor::wait_for_child(child).expect("reap exact nproc probe"),
            super::super::pid_supervisor::ChildOutcome::Exited(0)
        );
        println!("nproc-admission-case={case} transitioned={root} soft={soft} launcherThreads={threads} probeThreads=1");
        return;
    }
    for case in ["zero", "one"] {
        let output = Command::new(std::env::current_exe().expect("test executable"))
            .args(["--exact", TEST, "--nocapture"])
            .env(ENV, case)
            .output()
            .expect("run isolated nproc admission child");
        assert!(output.status.success(), "{output:?}");
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
}
