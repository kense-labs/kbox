//! Bounded static process limits, shared by OCI init, exec, streams and PTY.
use super::error;
use kbox_core::{
    guest_exec::{GuestRlimit, GuestRlimitResource},
    BoxConfig, Result,
};
use oci_spec::runtime::{
    LinuxCapabilities, PosixRlimit, PosixRlimitBuilder, PosixRlimitType, Process,
};

pub(super) fn limits(config: &BoxConfig) -> Result<(Vec<PosixRlimit>, Option<u32>)> {
    let mut limits = Vec::new();
    let mut per_uid = None;
    for limit in GuestRlimit::parse_specs(&config.resource_limits.ulimits).map_err(error)? {
        if matches!(
            limit.resource(),
            GuestRlimitResource::Nproc
                | GuestRlimitResource::Sigpending
                | GuestRlimitResource::Msgqueue
        ) {
            let (uid, _) = crate::local_execution::parse_process_user(config.user.as_deref())
                .map_err(error)?;
            let capabilities = super::workload_capabilities(config)?;
            // Per-UID budgets must retain one unprivileged accounting identity.
            if uid == 0 || !capability_free(Some(&capabilities)) {
                return Err(error(
                    "Mac per-UID limits require an explicit nonroot numeric user without capabilities",
                ));
            }
            per_uid = Some(uid);
        }
        let typ = match limit.resource() {
            GuestRlimitResource::Cpu => {
                // Linux interprets zero as one second; require the caller's exact intent.
                if limit.soft() == 0 {
                    return Err(error("Mac cpu limits require positive seconds or -1"));
                }
                PosixRlimitType::RlimitCpu
            }
            GuestRlimitResource::Nofile => {
                if limit.soft() < 16 || limit.hard() > 1_048_576 {
                    return Err(error("Mac nofile limits must be between16 and1048576"));
                }
                PosixRlimitType::RlimitNofile
            }
            GuestRlimitResource::Fsize => PosixRlimitType::RlimitFsize,
            GuestRlimitResource::Core => PosixRlimitType::RlimitCore,
            GuestRlimitResource::Data => PosixRlimitType::RlimitData,
            GuestRlimitResource::Stack => PosixRlimitType::RlimitStack,
            GuestRlimitResource::Memlock => PosixRlimitType::RlimitMemlock,
            GuestRlimitResource::Nice => PosixRlimitType::RlimitNice,
            GuestRlimitResource::Rtprio => PosixRlimitType::RlimitRtprio,
            GuestRlimitResource::Rttime => PosixRlimitType::RlimitRttime,
            GuestRlimitResource::Nproc => PosixRlimitType::RlimitNproc,
            GuestRlimitResource::Sigpending => PosixRlimitType::RlimitSigpending,
            GuestRlimitResource::Msgqueue => PosixRlimitType::RlimitMsgqueue,
            _ => {
                return Err(error(
                    "this Mac profile qualifies only cpu,nofile,fsize,core,data,stack,memlock,nice,rtprio,rttime and nonroot nproc,sigpending,msgqueue ulimits",
                ))
            }
        };
        limits.push(
            PosixRlimitBuilder::default()
                .typ(typ)
                .soft(limit.soft())
                .hard(limit.hard())
                .build()
                .map_err(error)?,
        );
    }
    Ok((limits, per_uid))
}

pub(super) fn apply(config: &BoxConfig, process: &mut Process) -> Result<()> {
    let (requested, per_uid) = limits(config)?;
    if let Some(uid) = per_uid {
        if process.user().uid() != uid
            || process.no_new_privileges() != Some(true)
            || !capability_free(process.capabilities().as_ref())
        {
            return Err(error(
                "Mac per-UID process must retain its configured nonroot UID, empty capabilities and no_new_privileges",
            ));
        }
    }
    if requested.is_empty() {
        return Ok(());
    }
    // Preserve OCI builder defaults and legacy request bytes when no override is declared.
    let mut limits = process.rlimits().clone().unwrap_or_default();
    for limit in requested {
        if let Some(existing) = limits
            .iter_mut()
            .find(|existing| existing.typ() == limit.typ())
        {
            *existing = limit;
        } else {
            limits.push(limit);
        }
    }
    process.set_rlimits(Some(limits));
    Ok(())
}

fn capability_free(capabilities: Option<&LinuxCapabilities>) -> bool {
    let Some(capabilities) = capabilities else {
        return false;
    };
    [
        capabilities.bounding(),
        capabilities.effective(),
        capabilities.permitted(),
        capabilities.inheritable(),
        capabilities.ambient(),
    ]
    .into_iter()
    .all(|values| values.as_ref().is_some_and(|values| values.is_empty()))
}
