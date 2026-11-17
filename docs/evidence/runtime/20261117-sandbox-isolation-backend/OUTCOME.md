# OUTCOME — sandbox isolation backend (Spec050) — in progress

Date: 2026-11-17  
Wave: `sandbox-isolation`  
Status: **partial** · overall goal **unmet** · ≠Pass

## Shipped this campaign slice

1. Spec050 planning: `specs/050-sandbox-isolation-backend/`
2. T2 vendor: `crates/box/sandbox/*` as `kbox-sandbox-*`, Apache NOTICE retained
3. T3 routes: `ExecutionBoundary::Sandbox`, `ManagedRuntimeRoute::Sandbox`
4. T4: router `with_sandbox` + production attach + Sandbox route pin
5. T4 daemon client: `docker_http` + adapter start/inspect/kill (hermetic fake
   Unix HTTP + fail-closed without daemon)
6. **T5 hermetic default flip:** `ExecutionBoundary::default() = Sandbox`;
   driver `new()` inherits Sandbox; `new_confidential` pins Microvm;
   `ensure_daemon(home)` spawn-or-attach on start path
7. Brand zero: `scripts/check-sandbox-brand.py` clean
8. **T5 native + CLI:** alpine echo via API and `kbox run` stdout
9. **Package/install:** `package-macos-arm64.sh` ships sandbox engine + guest;
   clean-prefix install → `CLEAN_PREFIX_OK`
10. Volumes (`-v`), publish (`-p`), `ps` (sandbox skips host-PID death),
    pause/unpause/rm
11. `--isolation microvm` native smoke (`MICROVM_OK`)
12. In-tree debranded `guest/kernel`; `scripts/build-sandbox-guest.sh`
13. **Compose Sandbox path:** `compose up` → LocalExecutionManager (sandbox
    route, pid null); managed teardown (no lifecycle deadlock);
    evidence `T5-COMPOSE-SANDBOX.md`
14. **Sandbox `kbox exec`:** `DockerHttpClient::container_exec` +
    `SandboxLocalExecutionBackend::execute`; session uses
    `uses_backend_session` (no MicroVM guest socket); native
    `SANDBOX_EXEC_OK` (`T5-SANDBOX-EXEC.md`)
15. **Package refresh:** `dist/kbox-3.4.3-macos-arm64.tar.gz` (~32 MB) with
    compose+exec; packaged smoke `PKG_EXEC_OK` (`T5-PACKAGE-COMPOSE-EXEC.md`)
16. **Compose Bridge / project networks:** `DockerHttpClient`
    network inspect/ensure/connect/remove + create `HostConfig.NetworkMode`
    with service aliases; Sandbox compose creates fused-engine Docker
    networks (not MicroVM NetworkStore); Bridge admitted; native
    `nslookup web` + `ping web` (`T5-COMPOSE-BRIDGE.md`)
17. **Developer install path:** `xattr -cr` quarantine clear + packaged
    `QUARANTINE_OK` (`T5-INSTALL-QUARANTINE.md`) — not notarize
18. **Sandbox PTY:** `container_exec_pty` hijack +
    `SandboxLocalExecutionBackend::start_pty`; native
    `printf … | kbox exec -it` → `PTY_OK` (`T5-SANDBOX-PTY.md`)
19. **Sandbox non-TTY streaming:** `container_exec_attach` mux hijack +
    `SandboxLocalExecutionBackend::start_process`; managed CLI `-i`
    without `-t`; native `printf … | kbox exec -i` → `STREAM_OK`
    (`T5-SANDBOX-STREAM.md`)
20. **Package refresh (PTY/stream CLI):** rebuild
    `dist/kbox-3.4.3-macos-arm64.tar.gz` (~32 MB); isolated-prefix install;
    packaged `printf … | kbox exec -i` → `PKG_STREAM_OK`
    (`T5-PACKAGE-STREAM.md`) — package refresh ≠ notarize
21. **Packaged daily path smoke:** install → run/exec/stream/PTY/pause/
    volume/rm → `PROD_DAILY_OK` (`T5-PROD-DAILY-PATH.md`) — ≠Pass / ≠notarize
22. **Sandbox captured exec stdin:** `container_exec` AttachStdin hijack +
    `SandboxLocalExecutionBackend::execute` finite stdin (16 MiB bound);
    CLI reads non-TTY pipe into captured path without `-i`; native
    `printf … | kbox exec` → `CAP_STDIN_OK` (`T5-SANDBOX-EXEC-STDIN.md`)
23. **Package refresh (captured exec stdin):** rebuild
    `dist/kbox-3.4.3-macos-arm64.tar.gz` (~33 MB);
    isolated-prefix install; packaged
    `printf … | kbox exec` (no `-i`/`-t`) → `PKG_CAP_STDIN_OK`
    (`T5-PACKAGE-EXEC-STDIN.md`) — package refresh ≠ notarize
24. **Compose Secrets on Sandbox:** private-disk binds + `map_env`; native
    `COMPOSE_SECRETS_OK` (`T5-COMPOSE-SECRETS.md`) — no secret bytes in evidence
25. **Package refresh (Compose Secrets):** rebuild
    `dist/kbox-3.4.3-macos-arm64.tar.gz` (~33 MB);
    isolated-prefix install; packaged
    `kbox compose -p pkgsec` secret_environment → private compare
    `PKG_COMPOSE_SECRETS_OK` (`T5-PACKAGE-COMPOSE-SECRETS.md`) —
    package refresh ≠ notarize; no secret bytes in evidence
26. **Optional MicroVM regression (HG3):** packaged isolated install;
    `kbox run -d --isolation microvm --name microvmreg` → exec
    `MICROVM_OK` → rm; default path still
    `execution_boundary=sandbox` (`T5-MICROVM-REGRESSION.md`)
27. **Developer / daily install note:** quarantine-clear install of current
    `dist/kbox-3.4.3-macos-arm64.tar.gz` (sha256 `e5782d3f…`) + Sandbox
    default check + one smoke; points to `PARK-NOTARIZE.md` for cold
    Gatekeeper (`T5-DAILY-INSTALL.md`) — ≠Pass / ≠notarize
28. **Package/install require Sandbox companions (k-review F2):**
    `package-macos-arm64.sh` refuses missing engine/`guest/out`;
    `install-package.sh` lists engine + guest as required FILES;
    fail-path documented (`T5-PACKAGE-REQUIRE-SANDBOX.md`)
29. **Package embed of required install gate:** rebuild
    `dist/kbox-3.4.3-macos-arm64.tar.gz` sha256 `e975167b…`; staged
    `install.sh` matches source (no `OPTIONAL_FILES`); neg-test missing
    engine/Image → refuse; isolated install + `PKG_REQUIRE_OK`
    (`T5-PACKAGE-REQUIRE-EMBEDDED.md`) — ≠Pass / ≠notarize; Env scrub
    may need a later engine refresh
30. **Fused-engine Secret Env scrub/avoid:** create Env excludes Secret
    values; `POST /kbox/containers/{id}/ephemeral-env` overlay for
    process/exec; durable `containers.json` Env keys `PATH` only;
    native `ENGINE_DURABLE_CLEAN` (`T5-ENGINE-SECRET-ENV-SCRUB.md`)
31. **Packaged F1 Secret Env scrub:** rebuild
    `dist/kbox-3.4.3-macos-arm64.tar.gz` sha256 `a6c7a125…`; isolated
    install + compose secret private compare + durable
    `containers.json` clean → `PKG_SCRUB_OK`
    (`T5-PACKAGE-ENGINE-SECRET-ENV-SCRUB.md`) — ≠Pass / ≠notarize
32. **Foreground Sandbox `run --rm` exit code:** managed-provider inspect +
    `TrustProvider` for Sandbox (not host-PID / `.kexit_code`); native
    `echo RM_EXIT_OK` → `EXIT:0`, fail-closed `exit 7` → `EXIT:7`
    (`T5-RUN-RM-EXIT.md`)
33. **Package refresh (`run --rm` exit):** rebuild
    `dist/kbox-3.4.3-macos-arm64.tar.gz` sha256 `5c1e8eba…`; isolated
    install → `PKG_RM_EXIT_OK` / `EXIT:0`, fail-closed `EXIT:7`
    (`T5-PACKAGE-RUN-RM-EXIT.md`) — ≠Pass / ≠notarize
34. **Local release 3.4.4:** bump cli/core `3.4.3→3.4.4`;
    `dist/kbox-3.4.4-macos-arm64.tar.gz` sha256 `11ef7ae8…`; isolated
    install → `kbox version 3.4.4`, `run --rm … REL_OK` → `EXIT:0`,
    `execution_boundary=sandbox` (`T5-RELEASE-3.4.4.md`); daily pointer
    updated (`T5-DAILY-INSTALL.md`) — ≠Pass / ≠notarize; no git tag/gh
    in this seat

## Evidence

- `T2-VENDOR.md` · `T3-ROUTES.md` · `T4-ROUTER.md` · `T4-ADAPTER.md` ·
  `T4-PRODUCTION-WIRE.md` · `T4-DAEMON-CLIENT.md` · `BRAND-ZERO.md` ·
  `T5-DEFAULT-SANDBOX.md` · `T5-FLIP-CHECKLIST.md` · `T5-NATIVE-SMOKE.md` ·
  `T5-CLI-RUN.md` · `T5-INSTALL-LAYOUT.md` · `T5-PACKAGE-INSTALL.md` ·
  `T5-PUBLISH.md` · `T5-PS-RUNNING.md` · `T5-VOLUME.md` ·
  `T5-ISOLATION-FLAG.md` · `T5-PAUSE-RM.md` · `T5-MICROVM-FLAG.md` ·
  `T5-MICROVM-REGRESSION.md` · `T5-COMPOSE-SANDBOX.md` ·
  `T5-COMPOSE-CURRENT.md` · `T5-SANDBOX-EXEC.md` ·
  `T5-PACKAGE-COMPOSE-EXEC.md` · `T5-COMPOSE-BRIDGE.md` ·
  `T5-INSTALL-QUARANTINE.md` · `T5-SANDBOX-PTY.md` ·
  `T5-SANDBOX-STREAM.md` · `T5-PACKAGE-STREAM.md` ·
  `T5-PROD-DAILY-PATH.md` · `T5-SANDBOX-EXEC-STDIN.md` ·
  `T5-PACKAGE-EXEC-STDIN.md` · `T5-COMPOSE-SECRETS.md` ·
  `T5-PACKAGE-COMPOSE-SECRETS.md` · `T5-DAILY-INSTALL.md` ·
  `T5-PACKAGE-REQUIRE-SANDBOX.md` · `T5-PACKAGE-REQUIRE-EMBEDDED.md` ·
  `T5-ENGINE-SECRET-ENV-SCRUB.md` · `T5-PACKAGE-ENGINE-SECRET-ENV-SCRUB.md` ·
  `T5-RUN-RM-EXIT.md` · `T5-PACKAGE-RUN-RM-EXIT.md` ·
  `T5-RELEASE-3.4.4.md` ·
  `PARK-COMPOSE-SECRETS.md` · `PARK-NOTARIZE.md` · `PARK-MULTIHOST.md`

## Remaining overall-goal gap

- Notarized / Gatekeeper-stapled install (Apple credentials — `PARK-NOTARIZE.md`)
- Multi-host matrix beyond this darwin arm64 host (`PARK-MULTIHOST.md`)
- Packaged F1 Secret Env scrub: **verified** (prior `a6c7a125…` /
  `PKG_SCRUB_OK`); 3.4.4 archive also embeds scrub + rm-exit; still ≠
  product Pass
- Engine ephemeral Secret overlay does not survive daemon process restart
  without Box recreate (in-memory only)
- Optional MicroVM on this host: **verified** this pulse
  (`T5-MICROVM-REGRESSION.md`); still ≠ product Pass
- Packaging F2 companion gate: **embedded**; current **3.4.4** also embeds
  F1 scrub + rm-exit (`11ef7ae8…`); still ≠ product Pass
- Foreground `run --rm` exit parity: **verified** in-tree
  (`T5-RUN-RM-EXIT.md`), packaged 3.4.3 (`T5-PACKAGE-RUN-RM-EXIT.md`),
  and packaged **3.4.4** (`T5-RELEASE-3.4.4.md` `EXIT:0`); still ≠
  product Pass
- Local versioned archive **3.4.4** ready for parent commit/tag/gh;
  notarize still parked

## Explicitly not claimed

- Product Pass / notarized daily install
- Windows/WSL native support
- Completion of unrelated `049-pool-oneshot-discard` WIP
- HTTP app-level discovery (Bridge smoke used DNS + ICMP only)
- This seat did not git commit / tag / GitHub Release
