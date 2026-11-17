# k-review — Spec050 sandbox-isolation / production-usable (main)

Date: 2026-11-17  
Branch: `main` (P17) · HEAD `f52bbb50c3a723b479cbf65da12eef6a6ec900fe` (+ uncommitted Spec050 tree)  
Scope: uncommitted Spec050 sandbox-isolation work + `docs/evidence/runtime/20261117-sandbox-isolation-backend/`  
≠ product Pass

## Separation (P20)

| Layer | Result | Meaning |
| --- | --- | --- |
| **checks_passed** | **failed** | Declared `review-checks.json` only. `skill-sync` passed; `ai-engineering` L3 **failed** (wave `sandbox-isolation` missing `lineage.json`, `timeline.jsonl`, `roles/{O,B,R,M,I,S,D}.md`). Receipt integrity verified as failed. L3 = control-plane conformance, **not** app acceptance. |
| **source review** | **pending → findings below** | Hermetic focused re-run on this host: Sandbox default + private-disk Secrets + router pin + brand scan **green**. Residual fused-engine Env retention and packaging optionality remain material risks. |
| **product acceptance** | **not claimed** | Darwin arm64 packaged smokes exist (`PROD_DAILY_OK`, `PKG_COMPOSE_SECRETS_OK`, `MICROVM_OK`). Notarize + multi-host **parked**. Overall GOALS unmet. |

Receipts:

- collect (not_run/77): `.kstudio/evidence/k-review/20261117T094625Z-80130cd8/receipt.json`
- run (failed): `.kstudio/evidence/k-review/20261117T094631Z-dec823cd/receipt.json`
- verify: `integrity_verified: failed; review pending; not product Pass`

Extra validation (outside declared checks, this review):

```text
K_HOME=/tmp/kbox-review-PMuSTT/home
cargo test --locked -p kbox-core --lib execution_boundary_defaults_to_sandbox  → 1 ok
cargo test --locked -p kbox-runtime --lib -- secret_manifest private_disk_store map_env_refuses_missing_secret_bind sandbox_execution_boundary_pins
  → 5 ok (map_env expand/refuse, private_disk_store, sandbox route pin, strip manifest)
python3 scripts/check-sandbox-brand.py → clean
```

Usage/prices: **unknown** (not recorded).

---

## Top findings (impact order)

### F1 — Fused-engine create Env retains expanded Secret values (residual, reproduced class)

- **Trigger:** Sandbox compose `secret_environment` → `map_env` / `materialize_secret_environment` expands binds into process Env at container create (`crates/box/runtime/src/local_execution/sandbox_backend.rs` ~158–240). Engine persists `Spec.env` into `containers.json` (`daemon/engine.rs` `save` / `Saved`) and exposes it via inspect `Config.Env` (`daemon/api.rs` ~2868) and `docker commit` image config (`daemon/ops.rs` ~342).
- **Impact:** Host-readable plaintext Secret values for non-`--rm` containers (compose long-lived); inspect/commit-class tooling can copy them. Box durable `boxes.json` keeps manifest only (good). Same class as Docker `-e`; **not** a MicroVM silent fallback.
- **Evidence:** `T5-COMPOSE-SECRETS.md` Residual; `OUTCOME.md` Remaining gap; source above. No hermetic assert yet that engine durable Env omits expanded Secret keys.
- **Prevention (smallest executable):** Prefer API boundary: do not persist expanded Secret Env in `Saved` / redact inspect+commit Env for Secret-projected containers; add a failing hermetic example that create-with-Secret then inspect durable record must not contain the fixture value. Until then, keep OUTCOME residual explicit — do not claim Secret “cleared”.

### F2 — Packaging/install still treat Sandbox companions as optional while product default is Sandbox

- **Trigger:** `ExecutionBoundary::default() = Sandbox` (`crates/box/core/src/config.rs`). `scripts/package-macos-arm64.sh` copies engine/guest only `if` present; `scripts/install-package.sh` lists them as `OPTIONAL_FILES`. Archive can install successfully without engine/guest.
- **Impact:** Clean install + broken default `kbox run` (fail-closed at ensure_daemon/guest lookup) — recurring “install OK ≠ usable Sandbox default” class. Current evidence archives *do* ship companions; the gate is not mechanical.
- **Evidence:** packaging/install scripts; guest discovery `$PREFIX/share/kbox/sandbox-guest` (`guest.rs` ~585–590); contrast Spec050 R2 / user story “after install … default sandbox”.
- **Prevention:** Static/package gate: refuse `package-macos-arm64.sh` / product archive publish when `bin/kbox-sandbox-engine` or `share/kbox/sandbox-guest/{Image,initramfs.cpio,data.img.tgz}` missing; optionally fail install of Sandbox-default archives if companions absent (required files, not optional).

### F3 — Notarize / Gatekeeper cold install parked (do not invent Pass)

- **Trigger:** Quarantine developer path (`T5-INSTALL-QUARANTINE.md` `xattr -cr`) ≠ notarized staple. `PARK-NOTARIZE.md` blocks Apple credentials.
- **Impact:** End-user Gatekeeper first-run unverified. Packaged Secret smoke also re-`codesign`s engine after install (`T5-PACKAGE-COMPOSE-SECRETS.md`) — entitlement/quarantine fragility for consumers.
- **Evidence:** `PARK-NOTARIZE.md`, quarantine + package Secret receipts. Status: **parked**, ≠Pass.
- **Prevention:** Human/park gate only until credentials exist; CI must not treat `QUARANTINE_OK` / `PROD_DAILY_OK` as notarize Pass.

### F4 — Multi-host matrix parked (do not invent Pass)

- **Trigger:** All product smokes on this darwin arm64 host only (`PARK-MULTIHOST.md`).
- **Impact:** No Linux KVM / other macOS arch / Windows evidence for Sandbox default.
- **Prevention:** Per-host resume with fresh `K_HOME` + host-matched archive; refuse cross-host Pass language in OUTCOME.

### F5 — Sandbox default routing / Secrets private-disk (source OK on reviewed paths; acceptance incomplete)

- **What holds:** Router pins `ManagedRuntimeRoute::Sandbox` with no migration-policy MicroVM downgrade (`router.rs` ~107–111; hermetic pin test ok). Private-disk store under `K_HOME/runtime-secrets/compose-sandbox` with ownership mode checks; Sandbox lease refuses `configure_vm` (`compose/secrets.rs`). Packaged Secret compare `PKG_COMPOSE_SECRETS_OK`.
- **Limits:** Compose Secret preflight always primes Sandbox private-disk (`secrets.rs` ~59–64) — correct while compose has no MicroVM isolation selector; if a selector is added later without updating preflight, MicroVM memory root will be skipped (recurring “preflight ≠ boundary” class). Private-disk is weaker than MicroVM memory-backed by design.
- **Prevention:** Keep router fail-closed tests in CI; if compose gains `--isolation microvm`, couple preflight to `box_config.execution_boundary` with a refusing test for mismatched store.

### F6 — Control-plane L3 incomplete for wave `sandbox-isolation`

- **Trigger:** Missing lineage/timeline/role files under `.kstudio/workflow/waves/sandbox-isolation/`.
- **Impact:** Declared review check fails; does **not** invalidate product source findings above.
- **Prevention:** Add wave lineage artifacts when continuing orchestration; do not weaken `verify-ai-eng L3` to greenwash.

---

## Parks (status only)

| Park | Status |
| --- | --- |
| `PARK-NOTARIZE.md` | parked · ≠Pass |
| `PARK-MULTIHOST.md` | parked · ≠Pass |
| `PARK-COMPOSE-SECRETS.md` | **closed on this host** (historical) |

---

## Explicitly not claimed

- Product Pass / GOALS overall met  
- Notarized daily install  
- Multi-host matrix  
- Engine Env Secret clearance  
- Unrelated Spec049 pool-oneshot-discard WIP  
- That L3 failure implies product rejection (or that hermetic greens imply Pass)

## Semantic conclusion

Spec050 on this branch has a coherent Sandbox-default fail-closed path and private-disk Compose Secrets with darwin arm64 packaged evidence. Material remaining correctness exposure is fused-engine durable/inspect Env retention of expanded Secrets; packaging still allows companion-less archives while default is Sandbox. Notarize and multi-host remain parks — **not** Pass.
