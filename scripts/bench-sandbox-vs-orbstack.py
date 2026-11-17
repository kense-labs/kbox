#!/usr/bin/env python3
"""Per-command wall-clock bench: kbox Sandbox vs Docker (OrbStack on darwin).

Times every top-level `kbox` command from `kbox --help` with a concrete
scenario. Docker/OrbStack gets a mapped scenario when one exists; otherwise
`orb_ms` is null with an explicit skip reason.

Protocol (fair daily-path):
  1) Install/resolve CLIs; kill stale kbox daemon only when BENCH_COLD=1.
  2) Pre-pull alpine:3.20 on both engines (timed separately as cold_pull).
  3) Warm: one throwaway `run --rm echo` on each engine (not scored).
  4) Time each of the 61 commands (median of BENCH_ITERS, default 1 for matrix
     breadth; raise for hotspots).

Outputs JSON (+ optional markdown) under --out. ≠Pass / ≠notarize.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Callable, Optional

ALPINE = "alpine:3.20"


@dataclass
class Row:
    cmd: str
    kbox_ms: Optional[float]
    orb_ms: Optional[float]
    ratio: Optional[float]  # kbox/orb; <1 means kbox faster
    kbox_rc: Optional[int]
    orb_rc: Optional[int]
    orb_skip: Optional[str]
    note: str = ""


def run(
    argv: list[str],
    *,
    env: Optional[dict] = None,
    timeout: float = 180.0,
    cwd: Optional[Path] = None,
) -> tuple[int, str, str, float]:
    e = os.environ.copy()
    if env:
        e.update(env)
    t0 = time.perf_counter()
    try:
        p = subprocess.run(
            argv,
            capture_output=True,
            text=True,
            env=e,
            timeout=timeout,
            cwd=str(cwd) if cwd else None,
        )
        ms = (time.perf_counter() - t0) * 1000.0
        return p.returncode, p.stdout, p.stderr, ms
    except subprocess.TimeoutExpired as exc:
        ms = (time.perf_counter() - t0) * 1000.0
        out = (exc.stdout or b"").decode() if isinstance(exc.stdout, bytes) else (exc.stdout or "")
        err = (exc.stderr or b"").decode() if isinstance(exc.stderr, bytes) else (exc.stderr or "")
        return 124, out, err + "\nTIMEOUT", ms


def timed_median(fn: Callable[[], tuple[int, float]], iters: int) -> tuple[int, float]:
    samples: list[float] = []
    last_rc = 1
    for _ in range(max(1, iters)):
        rc, ms = fn()
        last_rc = rc
        samples.append(ms)
    return last_rc, float(statistics.median(samples))


def kbox_help_cmds(kbox: str) -> list[str]:
    rc, out, err, _ = run([kbox, "--help"])
    text = out + err
    cmds: list[str] = []
    in_cmds = False
    for line in text.splitlines():
        if line.startswith("Commands:"):
            in_cmds = True
            continue
        if in_cmds and line.startswith("Options:"):
            break
        if in_cmds:
            m = re.match(r"^  ([a-z][a-z0-9-]*)\b", line)
            if m:
                cmds.append(m.group(1))
    return cmds


def which_docker() -> Optional[str]:
    for candidate in (
        shutil.which("docker"),
        str(Path.home() / ".orbstack/bin/docker"),
        "/usr/local/bin/docker",
        "/opt/homebrew/bin/docker",
    ):
        if candidate and Path(candidate).is_file():
            return candidate
    return None


def docker_backend_label(docker: str) -> str:
    rc, out, _, _ = run(
        [docker, "info", "--format", "{{.Name}}|{{.ServerVersion}}|{{.OperatingSystem}}"]
    )
    if rc != 0:
        return "docker-unknown"
    return out.strip() or "docker"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--kbox", default=shutil.which("kbox") or "kbox")
    ap.add_argument("--docker", default="")
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--iters", type=int, default=1)
    ap.add_argument("--markdown", type=Path, default=None)
    ap.add_argument("--label", default="")
    args = ap.parse_args()

    kbox = args.kbox
    docker = args.docker or which_docker()
    if not Path(kbox).is_file() and shutil.which(kbox) is None:
        print(f"missing kbox: {kbox}", file=sys.stderr)
        return 2
    if not docker:
        print("missing docker/OrbStack CLI", file=sys.stderr)
        return 2

    work = Path(tempfile.mkdtemp(prefix="kbox-bench-work-"))
    rows: list[Row] = []
    host = f"{os.uname().sysname} {os.uname().machine}"
    backend = docker_backend_label(docker)
    label = args.label or host

    # --- setup / cold metrics ---
    cold: dict[str, float] = {}
    if os.environ.get("BENCH_COLD", "0") == "1":
        subprocess.run(
            ["pkill", "-f", "kbox-sandbox-engine daemon"],
            check=False,
            capture_output=True,
        )
        time.sleep(0.5)

    rc, _, _, ms = run([kbox, "pull", ALPINE])
    cold["kbox_pull_ms"] = ms
    rc, _, _, ms = run([docker, "pull", ALPINE])
    cold["orb_pull_ms"] = ms

    # warm throwaway
    run([kbox, "run", "--rm", ALPINE, "--", "echo", "WARM"])
    run([docker, "run", "--rm", ALPINE, "echo", "WARM"])

    cmds = kbox_help_cmds(kbox)
    if len(cmds) < 50:
        print(f"unexpected command count: {len(cmds)}", file=sys.stderr)
        return 3

    # Shared long-lived box for exec-family timings
    run([kbox, "rm", "-f", "benchlife"], timeout=60)
    run([docker, "rm", "-f", "benchlife"], timeout=60)
    rc, out, err, _ = run(
        [kbox, "run", "-d", "--name", "benchlife", ALPINE, "--", "sleep", "3600"]
    )
    if rc != 0:
        print(f"FATAL: kbox benchlife create failed: {out}\n{err}", file=sys.stderr)
        return 4
    rc, out, err, _ = run([docker, "run", "-d", "--name", "benchlife", ALPINE, "sleep", "3600"])
    if rc != 0:
        print(f"FATAL: docker benchlife create failed: {out}\n{err}", file=sys.stderr)
        return 4

    # Pre-create assets used by several commands
    save_tar = work / "save.tar"
    export_tar = work / "export.tar"
    host_txt = work / "host.txt"
    host_txt.write_text("hostfile\n", encoding="utf-8")
    compose_dir = work / "bridge"
    compose_dir.mkdir()
    (compose_dir / "compose.yaml").write_text(
        """
services:
  web:
    image: alpine:3.20
    command: ["sleep", "3600"]
  client:
    image: alpine:3.20
    command: ["sleep", "3600"]
    depends_on: [web]
""".lstrip(),
        encoding="utf-8",
    )

    def add(
        cmd: str,
        *,
        kbox_argv: Optional[list[str]] = None,
        orb_argv: Optional[list[str]] = None,
        orb_skip: Optional[str] = None,
        expect_kbox_fail: bool = False,
        expect_orb_fail: bool = False,
        note: str = "",
        cwd: Optional[Path] = None,
    ) -> None:
        k_ms = k_rc = None
        o_ms = o_rc = None
        if kbox_argv is not None:

            def kfn() -> tuple[int, float]:
                rc, _, _, ms = run(kbox_argv, cwd=cwd)
                return rc, ms

            k_rc, k_ms = timed_median(kfn, args.iters)
            if expect_kbox_fail and k_rc == 0:
                note = (note + "; expected kbox fail").strip("; ")
            if not expect_kbox_fail and k_rc not in (0,):
                # Still record timing; annotate
                note = (note + f"; kbox_rc={k_rc}").strip("; ")
        if orb_skip:
            pass
        elif orb_argv is not None:

            def ofn() -> tuple[int, float]:
                rc, _, _, ms = run(orb_argv, cwd=cwd)
                return rc, ms

            o_rc, o_ms = timed_median(ofn, args.iters)
            if expect_orb_fail and o_rc == 0:
                note = (note + "; expected orb fail").strip("; ")
        ratio = None
        if k_ms is not None and o_ms is not None and o_ms > 0:
            ratio = k_ms / o_ms
        rows.append(
            Row(
                cmd=cmd,
                kbox_ms=round(k_ms, 2) if k_ms is not None else None,
                orb_ms=round(o_ms, 2) if o_ms is not None else None,
                ratio=round(ratio, 3) if ratio is not None else None,
                kbox_rc=k_rc,
                orb_rc=o_rc,
                orb_skip=orb_skip,
                note=note,
            )
        )

    # --- scenarios (order roughly follows e2e-sandbox-full) ---
    add("version", kbox_argv=[kbox, "version"], orb_argv=[docker, "version"])
    add("help", kbox_argv=[kbox, "help"], orb_argv=[docker, "help"])
    add("info", kbox_argv=[kbox, "info"], orb_argv=[docker, "info"])
    add("df", kbox_argv=[kbox, "df"], orb_argv=[docker, "system", "df"])
    add(
        "audit",
        kbox_argv=[kbox, "audit"],
        orb_skip="no docker equivalent (kbox audit ledger)",
    )
    add("pull", kbox_argv=[kbox, "pull", ALPINE], orb_argv=[docker, "pull", ALPINE], note="warm cache")
    add("images", kbox_argv=[kbox, "images"], orb_argv=[docker, "images"])
    add(
        "image-inspect",
        kbox_argv=[kbox, "image-inspect", ALPINE],
        orb_argv=[docker, "image", "inspect", ALPINE],
    )
    add("history", kbox_argv=[kbox, "history", ALPINE], orb_argv=[docker, "history", ALPINE])
    add(
        "tag",
        kbox_argv=[kbox, "tag", ALPINE, "bench-alpine:tag1"],
        orb_argv=[docker, "tag", ALPINE, "bench-alpine:tag1"],
    )
    add(
        "save",
        kbox_argv=[kbox, "save", "--output", str(save_tar), "bench-alpine:tag1"],
        orb_argv=[docker, "save", "-o", str(save_tar.with_name("orb-save.tar")), "bench-alpine:tag1"],
    )
    add(
        "load",
        kbox_argv=[kbox, "load", "--input", str(save_tar), "-t", "bench-alpine:loaded"],
        orb_argv=[docker, "load", "-i", str(save_tar.with_name("orb-save.tar"))],
    )
    add(
        "rmi",
        kbox_argv=[kbox, "rmi", "bench-alpine:loaded"],
        orb_argv=[docker, "rmi", "bench-alpine:tag1"],
        note="best-effort remove",
    )
    # import: kbox-specific rootfs; docker import needs tar
    rootfs = work / "rootfs.tar"
    # minimal empty ustar
    subprocess.run(
        ["tar", "-cf", str(rootfs), "-C", str(work), "host.txt"],
        check=False,
        capture_output=True,
    )
    add(
        "import",
        kbox_argv=[kbox, "import", str(rootfs), "bench-import:latest"],
        orb_argv=[docker, "import", str(rootfs), "bench-import:latest"],
    )
    dockerfile = work / "Dockerfile"
    dockerfile.write_text("FROM alpine:3.20\nCMD [\"echo\",\"BUILD_OK\"]\n", encoding="utf-8")
    add(
        "build",
        kbox_argv=[kbox, "build", "-t", "bench-build:latest", str(work)],
        orb_argv=[docker, "build", "-t", "bench-build:latest", str(work)],
        note="metadata-only Dockerfile",
    )
    def login_k() -> tuple[int, float]:
        t0 = time.perf_counter()
        p = subprocess.run(
            [kbox, "login", "--username", "x", "--password-stdin"],
            input="\n",
            capture_output=True,
            text=True,
            timeout=30,
        )
        return p.returncode, (time.perf_counter() - t0) * 1000

    k_rc, k_ms = timed_median(login_k, args.iters)
    rows.append(
        Row(
            "login",
            round(k_ms, 2),
            None,
            None,
            k_rc,
            None,
            "orb skipped: would mutate docker creds store",
            "fail-closed / local credential path",
        )
    )
    add(
        "logout",
        kbox_argv=[kbox, "logout"],
        orb_skip="orb skipped: would mutate docker creds store",
    )
    add(
        "push",
        kbox_argv=[kbox, "push", "bench-alpine:tag1"],
        orb_skip="fail-closed without registry; docker push would hang/auth",
        expect_kbox_fail=True,
    )

    # lifecycle create/start/... use dedicated names
    run([kbox, "rm", "-f", "benchcreate"], timeout=60)
    add(
        "create",
        kbox_argv=[kbox, "create", "--name", "benchcreate", ALPINE, "--", "sleep", "3600"],
        orb_argv=[docker, "create", "--name", "benchcreate", ALPINE, "sleep", "3600"],
    )
    add(
        "start",
        kbox_argv=[kbox, "start", "benchcreate"],
        orb_argv=[docker, "start", "benchcreate"],
    )
    add(
        "rename",
        kbox_argv=[kbox, "rename", "benchcreate", "benchlife2"],
        orb_argv=[docker, "rename", "benchcreate", "benchlife2"],
    )
    add(
        "restart",
        kbox_argv=[kbox, "restart", "benchlife2"],
        orb_argv=[docker, "restart", "benchlife2"],
    )
    add(
        "stop",
        kbox_argv=[kbox, "stop", "benchlife2"],
        orb_argv=[docker, "stop", "benchlife2"],
    )
    add(
        "wait",
        kbox_argv=[kbox, "wait", "benchlife2"],
        orb_argv=[docker, "wait", "benchlife2"],
        note="already stopped",
    )
    run([kbox, "start", "benchlife2"], timeout=60)
    run([docker, "start", "benchlife2"], timeout=60)
    add(
        "kill",
        kbox_argv=[kbox, "kill", "benchlife2"],
        orb_argv=[docker, "kill", "benchlife2"],
    )
    add(
        "rm",
        kbox_argv=[kbox, "rm", "-f", "benchlife2"],
        orb_argv=[docker, "rm", "-f", "benchlife2"],
    )

    add(
        "run",
        kbox_argv=[kbox, "run", "--rm", ALPINE, "--", "echo", "RUN"],
        orb_argv=[docker, "run", "--rm", ALPINE, "echo", "RUN"],
        note="warm daemon + cached image",
    )
    # Always recreate long-lived boxes before exec-family. Local inspect can
    # succeed from boxes.json after a daemon respawn while the engine container
    # is gone — force a fresh running container for fair exec/pause/top timings.
    run([kbox, "rm", "-f", "benchlife"], timeout=60)
    run([docker, "rm", "-f", "benchlife"], timeout=60)
    rc, out, err, _ = run(
        [kbox, "run", "-d", "--name", "benchlife", ALPINE, "--", "sleep", "3600"],
        timeout=120,
    )
    if rc != 0:
        print(f"WARN: kbox benchlife recreate failed: {out}\n{err}", file=sys.stderr)
    rc, out, err, _ = run(
        [docker, "run", "-d", "--name", "benchlife", ALPINE, "sleep", "3600"],
        timeout=120,
    )
    if rc != 0:
        print(f"WARN: docker benchlife recreate failed: {out}\n{err}", file=sys.stderr)
    add(
        "inspect",
        kbox_argv=[kbox, "inspect", "benchlife"],
        orb_argv=[docker, "inspect", "benchlife"],
    )
    add("ps", kbox_argv=[kbox, "ps"], orb_argv=[docker, "ps"])
    add(
        "exec",
        kbox_argv=[kbox, "exec", "benchlife", "--", "echo", "EXEC"],
        orb_argv=[docker, "exec", "benchlife", "echo", "EXEC"],
    )
    add(
        "logs",
        kbox_argv=[kbox, "logs", "benchlife"],
        orb_argv=[docker, "logs", "benchlife"],
    )
    add(
        "pause",
        kbox_argv=[kbox, "pause", "benchlife"],
        orb_argv=[docker, "pause", "benchlife"],
    )
    add(
        "unpause",
        kbox_argv=[kbox, "unpause", "benchlife"],
        orb_argv=[docker, "unpause", "benchlife"],
    )
    add(
        "top",
        kbox_argv=[kbox, "top", "benchlife"],
        orb_argv=[docker, "top", "benchlife"],
    )
    add(
        "stats",
        kbox_argv=[kbox, "stats", "--no-stream", "benchlife"],
        orb_argv=[docker, "stats", "--no-stream", "benchlife"],
    )
    # attach/shell: bounded — attach with timeout via exec substitute note
    def attach_k() -> tuple[int, float]:
        rc, _, _, ms = run([kbox, "attach", "benchlife"], timeout=2.0)
        return rc, ms

    k_rc, k_ms = timed_median(attach_k, 1)
    rows.append(
        Row(
            "attach",
            round(k_ms, 2),
            None,
            None,
            k_rc,
            None,
            "orb skipped: attach hangs without TTY protocol in bench",
            "bounded 2s timeout",
        )
    )
    add(
        "shell",
        kbox_argv=[kbox, "shell", "benchlife", "--", "echo", "SHELL"],
        orb_argv=[docker, "exec", "benchlife", "echo", "SHELL"],
        note="orb: docker exec proxy (no TTY)",
    )
    add(
        "port",
        kbox_argv=[kbox, "port", "benchlife"],
        orb_argv=[docker, "port", "benchlife"],
        note="may be empty without -p",
    )
    add(
        "port-forward",
        kbox_argv=[kbox, "port-forward", "--host-port", "18766", "--guest-port", "81", "benchlife"],
        orb_skip="docker uses -p at create; no 1:1 CLI",
        expect_kbox_fail=True,
        note="fail-closed / platform-bounded timed",
    )
    # volume/network/cp
    run([kbox, "volume", "rm", "benchvol"], timeout=30)
    run([docker, "volume", "rm", "benchvol"], timeout=30)
    add(
        "volume",
        kbox_argv=[kbox, "volume", "create", "benchvol"],
        orb_argv=[docker, "volume", "create", "benchvol"],
    )
    run([kbox, "network", "rm", "benchnet"], timeout=30)
    run([docker, "network", "rm", "benchnet"], timeout=30)
    add(
        "network",
        kbox_argv=[kbox, "network", "create", "benchnet"],
        orb_argv=[docker, "network", "create", "benchnet"],
    )
    run([kbox, "rm", "-f", "benchcp"], timeout=60)
    run([docker, "rm", "-f", "benchcp"], timeout=60)
    run([kbox, "run", "-d", "--name", "benchcp", "-v", "benchvol:/data", ALPINE, "--", "sleep", "3600"])
    run([docker, "run", "-d", "--name", "benchcp", "-v", "benchvol:/data", ALPINE, "sleep", "3600"])
    add(
        "cp",
        kbox_argv=[kbox, "cp", str(host_txt), "benchcp:/data/host.txt"],
        orb_argv=[docker, "cp", str(host_txt), "benchcp:/data/host.txt"],
    )
    add(
        "compose",
        kbox_argv=[kbox, "compose", "-f", "compose.yaml", "-p", "benchcomp", "up", "-d"],
        orb_argv=[docker, "compose", "-f", "compose.yaml", "-p", "benchcomp", "up", "-d"],
        cwd=compose_dir,
    )
    run([kbox, "compose", "-p", "benchcomp", "down"], cwd=compose_dir, timeout=120)
    run([docker, "compose", "-p", "benchcomp", "down"], cwd=compose_dir, timeout=120)

    run([kbox, "rm", "-f", "benchdiff"], timeout=60)
    run([kbox, "run", "-d", "--name", "benchdiff", ALPINE, "--", "sleep", "3600"])
    run([kbox, "exec", "benchdiff", "--", "sh", "-c", "echo x >/tmp/x"])
    add(
        "diff",
        kbox_argv=[kbox, "diff", "benchdiff"],
        orb_argv=[docker, "diff", "benchlife"],
    )
    add(
        "export",
        kbox_argv=[kbox, "export", "-o", str(export_tar), "benchdiff"],
        orb_argv=[docker, "export", "-o", str(export_tar.with_name("orb-export.tar")), "benchlife"],
    )
    add(
        "commit",
        kbox_argv=[kbox, "commit", "benchdiff", "bench-commit:latest"],
        orb_argv=[docker, "commit", "benchlife", "bench-commit:latest"],
    )
    add(
        "container-update",
        kbox_argv=[kbox, "container-update", "--restart", "no", "benchdiff"],
        orb_argv=[docker, "update", "--restart", "no", "benchlife"],
    )
    def evk() -> tuple[int, float]:
        rc, _, _, ms = run([kbox, "events", "--since", "1s"], timeout=2.0)
        return rc, ms

    def evo() -> tuple[int, float]:
        rc, _, _, ms = run(
            [docker, "events", "--since", "1s", "--until", "1s"], timeout=3.0
        )
        return rc, ms

    k_rc, k_ms = timed_median(evk, 1)
    o_rc, o_ms = timed_median(evo, 1)
    rows.append(
        Row(
            "events",
            round(k_ms, 2),
            round(o_ms, 2),
            round(k_ms / o_ms, 3) if o_ms else None,
            k_rc,
            o_rc,
            None,
            "bounded ~2s",
        )
    )
    add(
        "snapshot",
        kbox_argv=[kbox, "snapshot", "benchdiff"],
        orb_skip="no docker CLI snapshot peer (checkpoint experimental)",
        expect_kbox_fail=True,
    )
    add(
        "engine",
        kbox_argv=[kbox, "engine", "status"],
        orb_argv=[docker, "info"],
        note="orb: docker info as engine status proxy",
    )
    add(
        "pool",
        kbox_argv=[kbox, "pool", "status"],
        orb_skip="kbox warm-pool; no orb peer",
    )
    add(
        "monitor",
        kbox_argv=[kbox, "monitor", "--once"],
        orb_skip="kbox monitor; no orb peer",
    )
    add(
        "scale-api",
        kbox_argv=[kbox, "scale-api", "--help"],
        orb_skip="kbox scale-api; help-path timing only",
    )
    add(
        "update",
        kbox_argv=[kbox, "update", "--check"],
        orb_skip="kbox update check; no orb peer",
    )
    for tee in ("attest", "seal", "unseal", "inject-secret"):
        add(
            tee,
            kbox_argv=[kbox, tee, "benchlife"],
            orb_skip="TEE fail-closed; no docker peer",
            expect_kbox_fail=True,
        )

    # Destructive prune family LAST so earlier scenarios keep a live daemon/box.
    add(
        "prune",
        kbox_argv=[kbox, "prune", "-f"],
        orb_argv=[docker, "container", "prune", "-f"],
    )
    add(
        "image-prune",
        kbox_argv=[kbox, "image-prune", "-f"],
        orb_argv=[docker, "image", "prune", "-f"],
    )
    add(
        "system-prune",
        kbox_argv=[kbox, "system-prune", "-f"],
        orb_argv=[docker, "system", "prune", "-f", "--volumes=false"],
        note="destructive; timed last",
    )

    # coverage of help list
    covered = {r.cmd for r in rows}
    missing = [c for c in cmds if c not in covered]
    for m in missing:
        add(m, kbox_argv=[kbox, m, "--help"], orb_skip="auto help-path fallback", note="fallback")

    # cleanup
    run([kbox, "rm", "-f", "benchlife", "benchcp", "benchdiff"], timeout=120)
    run([docker, "rm", "-f", "benchlife", "benchcp"], timeout=120)
    run([kbox, "volume", "rm", "benchvol"], timeout=30)
    run([docker, "volume", "rm", "benchvol"], timeout=30)
    run([kbox, "network", "rm", "benchnet"], timeout=30)
    run([docker, "network", "rm", "benchnet"], timeout=30)

    comparable = [r for r in rows if r.kbox_ms is not None and r.orb_ms is not None]
    slower = sorted(
        [r for r in comparable if r.ratio and r.ratio > 1.2],
        key=lambda r: r.ratio or 0,
        reverse=True,
    )
    faster = sorted(
        [r for r in comparable if r.ratio and r.ratio < 0.8],
        key=lambda r: r.ratio or 0,
    )

    payload = {
        "label": label,
        "host": host,
        "docker_backend": backend,
        "kbox": kbox,
        "docker": docker,
        "iters": args.iters,
        "cold": cold,
        "commands_expected": cmds,
        "commands_timed": len(rows),
        "comparable_n": len(comparable),
        "rows": [asdict(r) for r in rows],
        "top_kbox_slower": [asdict(r) for r in slower[:15]],
        "top_kbox_faster": [asdict(r) for r in faster[:15]],
        "explicitly_not_claimed": [
            "product Pass",
            "notarize",
            "identical semantics for every mapped docker command",
        ],
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")

    md_path = args.markdown or args.out.with_suffix(".md")
    lines = [
        f"# Bench — kbox vs {backend}",
        "",
        f"Label: `{label}`  ",
        f"Host: `{host}`  ",
        f"Timed commands: **{len(rows)}** / expected **{len(cmds)}**  ",
        f"Comparable pairs: **{len(comparable)}**  ",
        f"Cold pull: kbox `{cold.get('kbox_pull_ms', 0):.0f}` ms · orb `{cold.get('orb_pull_ms', 0):.0f}` ms  ",
        "",
        "| cmd | kbox_ms | orb_ms | ratio (k/o) | skip |",
        "| --- | ---: | ---: | ---: | --- |",
    ]
    for r in rows:
        lines.append(
            f"| `{r.cmd}` | {r.kbox_ms if r.kbox_ms is not None else '—'} | "
            f"{r.orb_ms if r.orb_ms is not None else '—'} | "
            f"{r.ratio if r.ratio is not None else '—'} | {r.orb_skip or r.note or ''} |"
        )
    lines += [
        "",
        "## Top kbox slower (ratio>1.2)",
        "",
    ]
    for r in slower[:15]:
        lines.append(f"- `{r.cmd}`: {r.kbox_ms} / {r.orb_ms} ms (×{r.ratio})")
    lines += ["", "## Top kbox faster (ratio<0.8)", ""]
    for r in faster[:15]:
        lines.append(f"- `{r.cmd}`: {r.kbox_ms} / {r.orb_ms} ms (×{r.ratio})")
    lines += ["", "## Explicitly not claimed", "", "- Product Pass / notarize", ""]
    md_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {args.out}")
    print(f"wrote {md_path}")
    print(f"comparable={len(comparable)} slower={len(slower)} faster={len(faster)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
