# PARK — Compose Secret projection on Sandbox (historical)

Date: 2026-11-17  
Status: **closed on this host** — see `T5-COMPOSE-SECRETS.md`

## Was blocked

Sandbox compose refused Secrets (`requires MicroVM`) / Linux tmpfs-only
preflight on darwin.

## Resolution

Private-disk Secret store under `K_HOME` + bind mounts + `map_env` expansion
on the fused engine. MicroVM memory-backed path unchanged for
`--isolation microvm`.

## Resume

Only if a regression appears or a new Secret shape is required.
