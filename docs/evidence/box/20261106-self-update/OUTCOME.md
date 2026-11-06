# kbox self-update and 3.3.0 release — 2026-11-06

## Outcome

`kbox update --check` discovers the latest stable `kbox-v*` Release in `kense-labs/kense`, ignoring drafts, prereleases, and unrelated tags. `kbox update` installs that release; an explicit `MAJOR.MINOR.PATCH` argument selects a version, `--prefix` selects the installation prefix, and `--force` permits reinstalling the running version.

The command downloads the versioned installer without piping network content into a shell. The installer displays a compact curl progress bar only when stderr is a terminal, requires the archive's SHA-256 asset, verifies it, atomically replaces binaries and libraries, and runs the installed `kbox version`. The update command independently checks the installed version before reporting success.

## Environment

- Host: macOS arm64.
- Source package versions: `kbox-cli` and `kbox-core` 3.3.0.
- Package: `kbox-3.3.0-macos-arm64.tar.gz`, 22,930,537 bytes before the final installer-only repack; final archive size is checked again during publication.
- Final local archive SHA-256: `c75ee78e9be922ed19910ae3d4e1a68bd9012b58b350ba14f4070babf6b967ac`.
- No credentials are used or recorded by the update implementation.

## Checks completed before publication

```sh
cargo fmt --all -- --check
cargo test --locked -p kbox-cli commands::update
# 4 passed

cargo clippy --locked -p kbox-cli --all-targets --no-deps
# passed; existing unrelated warnings remained

cargo test --locked -p kbox-cli --test command_coverage
# 16 passed

cargo build --locked --profile release-dist -p kbox-cli -p kbox-shim
# passed

sh -n install.sh dist/install.sh
# passed
```

The unit tests cover numeric version ordering, rejection of malformed versions and shell metacharacters, stable kbox tag selection, prefix inference, and installer URL construction. The command-coverage suite discovered and executed help for the new command.

Before 3.3.0 was published, the real GitHub API check reported current 3.3.0 and selected published 3.2.7, correctly declining to downgrade. A final local package installed into an isolated prefix, printed `kbox version 3.3.0`, exposed `update --help`, and completed the same live update check.

An earlier isolated-prefix 3.3.0 runtime package with an empty `K_HOME` pulled the public dsh-acp image, created a process-prewarmed VM, completed a real `deepseek-flash` / `low` ACP greeting, returned `end_turn` and exit code 0, and cleaned up the pool. The measured CLI-to-initialize time was 11.873 ms and prompt RTT was 898.448 ms. The subsequent self-update change did not modify VM, pool, or ACP runtime code; the final published archive is tested again after publication.

## Explicitly not claimed

- Actual public-Release `kbox update` download and replacement is not claimed before publication. It is a required post-publication check.
- The terminal progress branch requires a TTY; noninteractive execution intentionally remains quiet. A pseudo-terminal check is required for the visual progress claim.
- Linux, Windows, x86_64, downgrade safety on every filesystem, and GitHub API outage behavior beyond explicit error handling were not tested.
- SHA-256 verifies release asset integrity and accidental corruption; it is not a signature or protection against compromise of both release assets.
