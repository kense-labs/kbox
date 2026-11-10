use std::path::Path;
use std::time::Instant;

use kbox_contract::contract::{
    ResourceControl, RuntimeCapabilities, RuntimeInspection, RuntimeUnitState,
};
use kbox_contract::RuntimeClient;

use super::cases::ResourceShape;
use super::fixture::BoxRuntimeConformanceFixture;
use super::{require, Result};

const CPU_MILLIS: u64 = 100;
const CPU_PERIOD_US: u64 = 100_000;
const CPU_QUOTA_US: u64 = CPU_MILLIS * (CPU_PERIOD_US / 1_000);
const MEMORY_BYTES: u64 = 128 * 1024 * 1024;
const PIDS: u32 = 32;
const TASK_EXECUTION_TIMEOUT_MS: u64 = 400;
const TASK_TIMEOUT_MAX_RUNTIME_MS: u64 = 5_000;
const EPHEMERAL_STORAGE_BYTES: u64 = 128 * 1024 * 1024;

pub(super) async fn run(
    fixture: &BoxRuntimeConformanceFixture,
    client: &dyn RuntimeClient,
    capabilities: &RuntimeCapabilities,
) -> Result<()> {
    let ephemeral_supported = capabilities
        .resource_controls
        .contains(&ResourceControl::EphemeralStorage);
    let mut service = fixture.cases.apply(
        "resources-service",
        kbox_contract::contract::RuntimeUnitClass::Service,
        "printf 'r17-resources-ready\\n'; exec sleep 3600",
        ResourceShape {
            cpu_millis: CPU_MILLIS,
            memory_bytes: MEMORY_BYTES,
            pids: PIDS,
            execution_timeout_ms: None,
        },
        kbox_contract::contract::RestartPolicy::Never,
    );
    if ephemeral_supported {
        service.spec.resources.ephemeral_storage_bytes = Some(EPHEMERAL_STORAGE_BYTES);
    }
    let running = client.apply(&service).await?;
    require(
        running.state == RuntimeUnitState::Running,
        "resource fixture Service did not reach running",
    )?;
    let record = fixture.record_for(&service.spec).await?;
    let config = &record
        .managed_execution
        .as_ref()
        .ok_or_else(|| super::protocol("resource fixture lost managed metadata"))?
        .request
        .config;
    require(
        config.resource_limits.cpu_quota == Some(CPU_QUOTA_US as i64)
            && config.resource_limits.cpu_period == Some(CPU_PERIOD_US),
        "CPU limits changed before provider launch",
    )?;
    require(
        config.resources.memory_mb == (MEMORY_BYTES / (1024 * 1024)) as u32
            && record.memory_mb == config.resources.memory_mb
            && config.resource_limits.sandbox_memory_limit_bytes.is_none()
            && config.resource_limits.memory_swap == Some(MEMORY_BYTES as i64),
        "MicroVM memory sizing changed before provider launch",
    )?;
    require(
        config.resource_limits.pids_limit == Some(u64::from(PIDS)),
        "PID limit changed before provider launch",
    )?;
    if ephemeral_supported {
        require(
            config.resources.ephemeral_storage_bytes == Some(EPHEMERAL_STORAGE_BYTES),
            "ephemeral storage quota did not reach the MicroVM provider",
        )?;
    }

    if ephemeral_supported {
        let quota = client
            .exec(&fixture.cases.exec(
                "resources-ephemeral-storage-behavior",
                &service.spec,
                vec![
                    "/bin/sh".into(),
                    "-c".into(),
                    "path=/tmp/.kr17-ephemeral-quota; rm -f \"$path\"; if dd if=/dev/zero of=\"$path\" bs=1048576 count=192 2>/dev/null; then printf 'quota-not-enforced\\n'; exit 71; fi; actual=$(wc -c <\"$path\"); test \"$actual\" -gt 0; test \"$actual\" -lt 201326592; printf 'quota-enforced:%s\\n' \"$actual\"; rm -f \"$path\"".into(),
                ],
                20_000,
            ))
            .await?;
        require(
            quota.exit_code == 0
                && quota.stdout.starts_with("quota-enforced:")
                && quota.stderr.is_empty(),
            format!(
                "ephemeral writable-layer quota was not enforced: exit_code={} stdout={:?} stderr={:?}",
                quota.exit_code, quota.stdout, quota.stderr
            ),
        )?;
    }

    let visible = client
        .exec(&fixture.cases.exec(
            "resources-visible-config",
            &service.spec,
            vec![
                "/bin/sh".into(),
                "-c".into(),
                "cgroup_path=$(awk -F: '$1 == \"0\" && $2 == \"\" { print $3 }' /proc/self/cgroup); test -n \"$cgroup_path\"; printf 'cgroup=%s\\n' \"$cgroup_path\"; cat \"/sys/fs/cgroup${cgroup_path}/cpu.max\"; cat \"/sys/fs/cgroup${cgroup_path}/memory.max\"; cat \"/sys/fs/cgroup${cgroup_path}/pids.max\"".into(),
            ],
            5_000,
        ))
        .await?;
    let mut visible_lines = visible.stdout.lines();
    let workload_namespace_path = visible_lines
        .next()
        .and_then(|line| line.strip_prefix("cgroup="))
        .ok_or_else(|| {
            super::protocol(format!(
                "workload did not report its cgroup path: {:?}",
                visible.stdout
            ))
        })?;
    let visible_cpu_max = visible_lines.next();
    let visible_memory_max = visible_lines.next();
    let visible_pids_max = visible_lines.next();
    let expected_cpu_max = format!("{CPU_QUOTA_US} {CPU_PERIOD_US}");
    let expected_memory_max = "max".to_string();
    let expected_pids_max = PIDS.to_string();
    require(
        workload_namespace_path
            .strip_prefix("/box-")
            .is_some_and(|suffix| !suffix.is_empty() && !suffix.contains('/')),
        format!("workload joined an unexpected cgroup path: {workload_namespace_path:?}"),
    )?;
    require(
        visible_cpu_max == Some(expected_cpu_max.as_str())
            && visible_memory_max == Some(expected_memory_max.as_str())
            && visible_pids_max == Some(expected_pids_max.as_str()),
        format!(
            "workload did not observe its exact cgroup limits: {:?}",
            visible.stdout
        ),
    )?;
    let throttled_before = guest_counter(
        fixture,
        client,
        &service.spec,
        "resources-cpu-before",
        "cpu.stat",
        "nr_throttled",
    )
    .await?;
    let cpu = client
        .exec(&fixture.cases.exec(
            "resources-cpu-behavior",
            &service.spec,
            vec![
                "/bin/sh".into(),
                "-c".into(),
                "timeout 1 sh -c 'trap \"exit 124\" TERM; while :; do :; done'".into(),
            ],
            5_000,
        ))
        .await?;
    require(
        cpu.exit_code == 124,
        format!(
            "CPU saturation oracle exited with {}; stdout={:?}; stderr={:?}",
            cpu.exit_code, cpu.stdout, cpu.stderr
        ),
    )?;
    let throttled_after = guest_counter(
        fixture,
        client,
        &service.spec,
        "resources-cpu-after",
        "cpu.stat",
        "nr_throttled",
    )
    .await?;
    require(
        throttled_after > throttled_before,
        "CPU workload was never throttled by the configured quota",
    )?;

    let pid_max_before = guest_counter(
        fixture,
        client,
        &service.spec,
        "resources-pids-before",
        "pids.events",
        "max",
    )
    .await?;
    let _ = client
        .exec(&fixture.cases.exec(
            "resources-pids-behavior",
            &service.spec,
            vec![
                "/bin/sh".into(),
                "-c".into(),
                "i=0; while [ \"$i\" -lt 96 ]; do sleep 1 & i=$((i + 1)); done; wait".into(),
            ],
            15_000,
        ))
        .await?;
    let pid_max_after = guest_counter(
        fixture,
        client,
        &service.spec,
        "resources-pids-after",
        "pids.events",
        "max",
    )
    .await?;
    require(
        pid_max_after > pid_max_before,
        "PID-heavy workload did not hit the configured pids.max",
    )?;

    let memory = client
        .exec(&fixture.cases.exec(
            "resources-memory-behavior",
            &service.spec,
            vec![
                "/bin/sh".into(),
                "-c".into(),
                "awk 'BEGIN { s=\"0123456789abcdef\"; for (i=0; i<25; i++) s=s s; print length(s) }'"
                    .into(),
            ],
            15_000,
        ))
        .await?;
    require(
        memory.exit_code != 0,
        "memory-heavy MicroVM workload escaped the configured VM memory boundary",
    )?;
    let RuntimeInspection::Found { observation, .. } =
        client.inspect(&service.spec.unit_id).await?
    else {
        return Err(super::protocol(
            "resource probes lost the long-running Service",
        ));
    };
    require(
        observation.state == RuntimeUnitState::Running,
        "resource-limit probes killed the provider Service",
    )?;
    let control_alive = client
        .exec(&fixture.cases.exec(
            "resources-control-after-oom",
            &service.spec,
            vec![
                "/bin/sh".into(),
                "-c".into(),
                "printf 'r17-control-alive\\n'".into(),
            ],
            5_000,
        ))
        .await?;
    require(
        control_alive.exit_code == 0
            && control_alive.stdout == "r17-control-alive\n"
            && control_alive.stderr.is_empty(),
        format!(
            "workload OOM damaged the provider exec transport: exit_code={} stdout={:?} stderr={:?}",
            control_alive.exit_code, control_alive.stdout, control_alive.stderr
        ),
    )?;

    let timeout = fixture.cases.task(
        "resources-execution-timeout",
        "exec sleep 3600",
        TASK_EXECUTION_TIMEOUT_MS,
    );
    let applied_at = Instant::now();
    let timed_out = client.apply(&timeout).await?;
    let wall_elapsed_ms = applied_at.elapsed().as_millis();
    let provider_elapsed_ms = timed_out
        .started_at_ms
        .zip(timed_out.finished_at_ms)
        .and_then(|(started, finished)| finished.checked_sub(started));
    require(
        timed_out.state == RuntimeUnitState::Failed
            && timed_out
                .failure
                .as_ref()
                .is_some_and(|failure| failure.code == "execution_timeout" && !failure.retryable)
            && provider_elapsed_ms
                .is_some_and(|elapsed| elapsed < TASK_TIMEOUT_MAX_RUNTIME_MS),
        format!(
            "Task execution timeout was not behaviorally enforced: state={:?} failure={:?} provider_elapsed_ms={provider_elapsed_ms:?} wall_elapsed_ms={wall_elapsed_ms}",
            timed_out.state, timed_out.failure
        ),
    )?;
    fixture
        .remove_unit(client, &timeout.spec, "resources-execution-timeout")
        .await?;
    fixture
        .remove_unit(client, &service.spec, "resources-service")
        .await
}

#[cfg(target_os = "linux")]
fn read_trimmed(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .map(|value| value.trim().to_string())
        .map_err(|error| super::external(&format!("read {}", path.display()), error))
}


async fn guest_counter(
    fixture: &BoxRuntimeConformanceFixture,
    client: &dyn RuntimeClient,
    spec: &kbox_contract::contract::RuntimeUnitSpec,
    case_id: &str,
    file: &str,
    key: &str,
) -> Result<u64> {
    let command = format!(
        "cgroup_path=$(awk -F: '$1 == \"0\" && $2 == \"\" {{ print $3 }}' /proc/self/cgroup); test -n \"$cgroup_path\"; awk '$1 == \"{key}\" {{ print $2; found=1 }} END {{ exit found ? 0 : 1 }}' \"/sys/fs/cgroup${{cgroup_path}}/{file}\""
    );
    let output = client
        .exec(&fixture.cases.exec(
            case_id,
            spec,
            vec!["/bin/sh".into(), "-c".into(), command],
            5_000,
        ))
        .await?;
    require(
        output.exit_code == 0 && output.stderr.is_empty() && !output.truncated,
        format!(
            "guest cgroup counter {file}:{key} failed: exit_code={} stdout={:?} stderr={:?}",
            output.exit_code, output.stdout, output.stderr
        ),
    )?;
    output.stdout.trim().parse::<u64>().map_err(|error| {
        super::external(&format!("decode guest cgroup counter {file}:{key}"), error)
    })
}

