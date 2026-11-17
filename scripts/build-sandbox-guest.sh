#!/usr/bin/env bash
# Build Spec050 sandbox guest artifacts into guest/out/.
# Requires: Docker engine, rustup target aarch64-unknown-linux-musl (or
# x86_64-unknown-linux-musl), rust-lld on PATH (rustc sysroot bin).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/guest/out"
ARCH="${1:-$(uname -m)}"
case "$ARCH" in
  arm64|aarch64) KARCH=arm64; MUSL=aarch64-unknown-linux-musl; KERNEL_NAME=Image ;;
  x86_64|amd64) KARCH=x86_64; MUSL=x86_64-unknown-linux-musl; KERNEL_NAME=bzImage ;;
  *) echo "unsupported arch: $ARCH" >&2; exit 2 ;;
esac
mkdir -p "$OUT"
export PATH="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | awk '/host:/{print $2}')/bin:${PATH:-}"

cmd="${2:-all}"
disk() {
  docker run --rm -v "$OUT:/out" debian:trixie sh -c \
    "set -e; apt-get update -qq >/dev/null && apt-get install -y -qq e2fsprogs >/dev/null; \
     rm -f /out/data.img; truncate -s 64G /out/data.img; \
     mkfs.ext4 -q -F -m 0 -L kbox-data -E lazy_itable_init=1,lazy_journal_init=1 /out/data.img"
  echo "wrote $OUT/data.img"
}

kernel() {
  local cache=kbox-kbuild
  [[ "$KARCH" == x86_64 ]] && cache=kbox-kbuild-x86_64
  docker run --rm \
    -e DEBIAN_FRONTEND=noninteractive \
    -e "KARCH=$KARCH" \
    -v "$cache:/build" \
    -v "$ROOT/guest/kernel:/src:ro" \
    -v "$OUT:/out" \
    debian:trixie sh /src/build.sh
  echo "wrote $OUT/$KERNEL_NAME"
}

initramfs() {
  (cd "$ROOT" && cargo build --locked -p kbox-sandbox-init --release --target "$MUSL")
  local init="$ROOT/target/$MUSL/release/kbox-sandbox-init"
  local stage
  stage="$(mktemp -d)"
  mkdir -p "$stage"/{dev,proc,sys,run,tmp,etc}
  cp "$init" "$stage/init"
  chmod 755 "$stage/init"
  python3 - "$stage" "$OUT/initramfs.cpio" <<'PY'
import os, sys
root, out = sys.argv[1], sys.argv[2]
entries = [(d, "dir", None) for d in ("dev", "proc", "sys", "run", "tmp", "etc")]
entries += [("dev/console", "char", (5, 1)), ("init", "file", os.path.join(root, "init"))]

def pad(n):
    return (4 - (n % 4)) % 4

with open(out, "wb") as f:
    ino = 1
    for name, kind, payload in entries:
        mode = {"dir": 0o040755, "file": 0o100755, "char": 0o020600}[kind]
        data = b""
        maj = minor = 0
        if kind == "file":
            data = open(payload, "rb").read()
        elif kind == "char":
            maj, minor = payload
        names = name.encode() + b"\0"
        hdr = (
            f"070701{ino:08X}{mode:08X}{0:08X}{0:08X}{1:08X}{0:08X}"
            f"{len(data):08X}{0:08X}{0:08X}{maj:08X}{minor:08X}"
            f"{len(names):08X}{0:08X}"
        ).encode()
        f.write(hdr + names + b"\0" * pad(len(hdr) + len(names)))
        f.write(data + b"\0" * pad(len(data)))
        ino += 1
    name = b"TRAILER!!!\0"
    hdr = f"070701{0:08X}{0:08X}{0:08X}{0:08X}{1:08X}{0:08X}{0:08X}{0:08X}{0:08X}{0:08X}{0:08X}{len(name):08X}{0:08X}".encode()
    f.write(hdr + name + b"\0" * pad(len(hdr) + len(name)))
print(out, os.path.getsize(out))
PY
  rm -rf "$stage"
}

case "$cmd" in
  disk) disk ;;
  kernel) kernel ;;
  initramfs|init) initramfs ;;
  all) disk; kernel; initramfs ;;
  *) echo "usage: $0 [arm64|x86_64] [all|disk|kernel|initramfs]" >&2; exit 2 ;;
esac
