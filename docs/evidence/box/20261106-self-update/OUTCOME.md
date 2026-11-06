# kbox self-update and 3.3.0 release — 2026-11-06

## Outcome

`kbox update --check` discovers the latest stable `kbox-v*` Release in `kense-labs/kense`, ignoring drafts, prereleases, and unrelated tags. `kbox update` installs that release; an explicit `MAJOR.MINOR.PATCH` argument selects a version, `--prefix` selects the installation prefix, and `--force` permits reinstalling the running version.

The command downloads the versioned installer without piping network content into a shell. The installer displays a compact curl progress bar only when stderr is a terminal, requires the archive's SHA-256 asset, verifies it, atomically replaces binaries and libraries, and runs the installed `kbox version`. The update command independently checks the installed version before reporting success.

## Environment and release

- Host: macOS arm64.
- Source package versions: `kbox-cli` and `kbox-core` 3.3.0.
- Source revision: `959bb70` in `kense-labs/kbox`.
- Release: `kbox-v3.3.0` in `kense-labs/kense`, published at `2026-11-06T10:32:32Z`.
- Package: `kbox-3.3.0-macos-arm64.tar.gz`, 22,930,560 bytes.
- Archive SHA-256: `c75ee78e9be922ed19910ae3d4e1a68bd9012b58b350ba14f4070babf6b967ac`; the GitHub asset API reported the same digest.
- No credentials are used or recorded by the update implementation.

## Implementation checks

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

## Public Release verification

The official Release installer installed the archive into a new isolated prefix and printed `kbox version 3.3.0`. The installed binary then ran `kbox update --force` with no `--prefix`, inferred its own prefix, selected stable 3.3.0 through the real GitHub API, downloaded the public assets, verified SHA-256, atomically replaced its own files, and printed `Successfully installed kbox 3.3.0.` A subsequent `kbox update --check` selected 3.3.0 and reported `kbox is up to date.`

A separate run under a Python `pty.fork()` pseudo-terminal displayed curl's progress bar advancing from 0.0% to 100.0% for the archive, followed by checksum verification and successful installation. Noninteractive runs remained free of progress control output.

With the same officially installed and self-updated binary first on `PATH` and a new isolated `K_HOME`, the complete documented HTTPS command fetched the public benchmark from `raw.githubusercontent.com`, pulled the public dsh-acp image, prewarmed a VM and agent process, and completed a real DeepSeek ACP greeting:

- CLI spawn to initialize response: 15.031 ms.
- Initialize request RTT: 13.402 ms.
- Model prompt RTT: 963.367 ms.
- Selection: `deepseek-official`, `deepseek-flash`, `low`.
- Nonempty Chinese model text, `end_turn`, exit code 0, followed by pool cleanup.

The API key was read from the owner's local configuration into the process environment and was not written to the package, benchmark output, release notes, or this evidence.

## Explicitly not claimed

- Linux, Windows, x86_64, downgrade safety on every filesystem, and GitHub API outage behavior beyond explicit error handling were not tested.
- SHA-256 verifies release asset integrity and accidental corruption; it is not a signature or protection against compromise of both release assets.
- Update checks run only when the user invokes `kbox update`; no background update service is installed.
- The complete 1 ms CLI startup target is not claimed. The final end-to-end CLI-to-initialize measurement was 15.031 ms.
