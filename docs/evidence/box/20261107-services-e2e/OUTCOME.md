# kbox service deployment E2E — functional — 2026-11-07

## Outcome

- Overall: **PASS** (4 pass / 0 fail)
- Scope: not just “box running” — each service asserts **real I/O** from the host (or guest SQL) and volume restart where applicable
- Command:

```sh
TIER=all E2E_OUT=docs/evidence/box/20261107-services-e2e-func \
  ./scripts/e2e-services/run.sh
```

## Environment

| Item | Value |
| --- | --- |
| Host | macOS arm64 |
| kbox | 3.4.2 |
| Isolated `K_HOME` | yes (not retained in git) |

## Functional assertions

| ID | Status | What was verified |
| --- | --- | --- |
| S-NGINX | PASS | Bind-mount custom `index.html`; host `GET /` returns that marker (not stock welcome page) |
| S-REDIS | PASS | Host RESP `SET`/`GET`/`DEL`; AOF enabled; same volume restart still `GET`s the value |
| S-PG | PASS | Host TCP; guest `CREATE`/`INSERT`/`SELECT`; same volume restart retains the row |
| S-RUSTFS | PASS | Host `/health`; SigV4 `CreateBucket` + `PutObject` + `GetObject` round-trip |

## Artifacts

- `results.json`
- per-service logs under `nginx/` `redis/` `postgres/` `rustfs/`

## Not covered

- Compose multi-service networking between boxes
- Host-native `psql`/`redis-cli` clients (guest `psql` / raw RESP / pure-Python SigV4 used instead)
- RustFS console UI / multipart / IAM beyond root keys
