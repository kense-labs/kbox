# T5 — Sandbox full-feature packaged E2E (`SANDBOX_FULL_E2E_OK`)

Date: 2026-11-17  
Host: darwin arm64 (Darwin)  
Branch: `main` (P17)  
≠Pass · ≠notarize

## Intent

YOLO scope **确保 kbox 的 sandbox 模式全功能端到端测试通过**: one
re-runnable harness installs the shipped **3.4.4** archive into an isolated
prefix and exercises the Docker-shaped Sandbox daily path end-to-end.

## Harness

```bash
bash scripts/e2e-sandbox-full.sh
# ARCHIVE=dist/kbox-3.4.4-macos-arm64.tar.gz (default)
```

## Archive

```text
archive=dist/kbox-3.4.4-macos-arm64.tar.gz
sha256=11ef7ae81016bffd7d3eba19303ab3af7adad1048881efe5b7ecd9df2b51f466
```

## Observed (this run)

```text
PREFIX=/tmp/kbox-sandbox-e2e-prefix-IZSiIG
K_HOME=/tmp/kbox-sandbox-e2e-home-jBVhpR
LOG=/tmp/kbox-sandbox-e2e-20261117203547.log
INSTALL_OK
BOUNDARY_OK
EXEC_OK
PS_OK
LOGS_OK
STOP_OK
RM_OK
RM_EXIT_OK EXIT:0 EXIT:7
CAP_STDIN_OK
STREAM_OK
PTY_OK
PAUSE_OK
UNPAUSE_OK
VOLUME_OK
VOLUME_RM_OK
PUBLISH_OK
NETWORK_OK
NETWORK_RM_OK
COMPOSE_UP_OK
COMPOSE_DNS_OK
COMPOSE_PING_OK
COMPOSE_BRIDGE_OK
COMPOSE_SECRETS_OK
BRAND_OK
SANDBOX_FULL_E2E_OK
```

Wall time ≈ 85s after alpine pull cache warm.

## Coverage map

| Surface | Marker |
| --- | --- |
| Packaged install + version 3.4.4 | `INSTALL_OK` |
| Default `execution_boundary` / `runtime_route` sandbox | `BOUNDARY_OK` |
| `exec` / `ps` / `logs` / `stop` / `rm` | `EXEC_OK` … `RM_OK` |
| Foreground `run --rm` exit 0 and 7 | `RM_EXIT_OK` |
| Captured stdin / `exec -i` / `exec -it` | `CAP_STDIN_OK` `STREAM_OK` `PTY_OK` |
| `pause` / `unpause` | `PAUSE_OK` `UNPAUSE_OK` |
| Named volume bind | `VOLUME_OK` |
| `-p` publish + HTTP GET | `PUBLISH_OK` |
| `network create` + `--network` | `NETWORK_OK` |
| Compose Bridge DNS + ICMP | `COMPOSE_BRIDGE_OK` |
| Compose `secret_environment` (hash compare; no plaintext in evidence) | `COMPOSE_SECRETS_OK` |
| Brand scan | `BRAND_OK` |

## Explicitly not claimed

- Product Pass / notarized Gatekeeper staple
- Multi-host matrix (see `PARK-MULTIHOST.md`)
- Snapshot / build / registry push / TEE / warm-pool surfaces
- Every CLI subcommand outside the table above
