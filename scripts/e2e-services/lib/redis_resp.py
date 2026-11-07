#!/usr/bin/env python3
"""Minimal Redis RESP client for E2E assertions."""
from __future__ import annotations

import socket
import sys
import time


def _encode(cmd: list[str]) -> bytes:
    parts = [f"*{len(cmd)}\r\n".encode()]
    for arg in cmd:
        b = arg.encode()
        parts.append(f"${len(b)}\r\n".encode())
        parts.append(b + b"\r\n")
    return b"".join(parts)


def _read_reply(sock: socket.socket) -> str:
    buf = b""
    while b"\r\n" not in buf:
        chunk = sock.recv(4096)
        if not chunk:
            raise RuntimeError("connection closed")
        buf += chunk
    line, rest = buf.split(b"\r\n", 1)
    kind = line[:1]
    if kind == b"+":
        return line[1:].decode()
    if kind == b"-":
        raise RuntimeError(line[1:].decode())
    if kind == b":":
        return line[1:].decode()
    if kind == b"$":
        n = int(line[1:])
        if n < 0:
            return ""
        while len(rest) < n + 2:
            chunk = sock.recv(4096)
            if not chunk:
                raise RuntimeError("connection closed mid-bulk")
            rest += chunk
        return rest[:n].decode()
    raise RuntimeError(f"unsupported reply: {line!r}")


def call(host: str, port: int, cmd: list[str], attempts: int = 30) -> str:
    last = ""
    for _ in range(attempts):
        sock = socket.socket()
        sock.settimeout(3)
        try:
            sock.connect((host, port))
            sock.sendall(_encode(cmd))
            return _read_reply(sock)
        except Exception as exc:  # noqa: BLE001 — E2E retry surface
            last = str(exc)
            time.sleep(1)
        finally:
            sock.close()
    raise RuntimeError(f"redis {cmd[0]} failed: {last}")


def main() -> int:
    if len(sys.argv) < 4:
        print("usage: redis_resp.py HOST PORT CMD [ARG...]", file=sys.stderr)
        return 2
    host, port = sys.argv[1], int(sys.argv[2])
    print(call(host, port, sys.argv[3:]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
