"""ACP editor services for the acceptance client; every terminal runs inside kbox."""

from concurrent.futures import ThreadPoolExecutor
import os
from pathlib import Path, PurePosixPath
import pty
import signal
import subprocess
import threading


class ClientServices:
    capabilities = {"fs": {"readTextFile": True, "writeTextFile": True}, "terminal": True}

    def __init__(self, kbox, box, mounts):
        self.kbox, self.box, self.mounts = kbox, box, mounts
        self.sessions = set()
        self.terminals = {}
        self.calls = []
        self.workers = ThreadPoolExecutor(max_workers=8)

    def path(self, guest):
        path = PurePosixPath(guest)
        if not path.is_absolute() or ".." in path.parts:
            raise ValueError("ACP file path must be absolute without traversal")
        for root, host in self.mounts.items():
            if path.is_relative_to(root):
                resolved = (host / path.relative_to(root)).resolve()
                if not resolved.is_relative_to(host.resolve()):
                    raise PermissionError("file path escapes the mounted directory")
                return resolved
        raise PermissionError("file path is outside the client's mounted directories")

    def __call__(self, method, params):
        session = params["sessionId"]
        if session not in self.sessions:
            raise ValueError("unknown ACP session")
        self.calls.append({"method": method, "sessionId": session})
        if method == "fs/read_text_file":
            path = self.path(params["path"])
            if path.stat().st_size > 4 * 1024 * 1024:
                raise ValueError("file exceeds the client's 4 MiB read limit")
            text = path.read_text(encoding="utf-8")
            line, limit = params.get("line", 1), params.get("limit")
            if type(line) is not int or line < 1 or (limit is not None and (type(limit) is not int or limit < 1)):
                raise ValueError("line and limit must be positive integers")
            if "line" in params or limit is not None:
                text = "".join(text.splitlines(keepends=True)[line - 1:None if limit is None else line - 1 + limit])
            return {"content": text}
        if method == "fs/write_text_file":
            path = self.path(params["path"])
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(params["content"], encoding="utf-8")
            return {}
        if method == "terminal/create":
            terminal_id = str(len(self.calls))
            self.terminals[terminal_id] = (session, Terminal(self.kbox, self.box, params))
            return {"terminalId": terminal_id}
        if method.startswith("terminal/"):
            owner, terminal = self.terminals[params["terminalId"]]
            if owner != session:
                raise PermissionError("terminal belongs to another ACP session")
            if method == "terminal/output":
                return terminal.output()
            if method == "terminal/wait_for_exit":
                return self.workers.submit(terminal.wait)
            if method == "terminal/kill":
                return self.workers.submit(terminal.kill)
            if method == "terminal/release":
                del self.terminals[params["terminalId"]]
                return self.workers.submit(terminal.kill)
        from acp_client import RpcError
        raise RpcError({"code": -32601, "message": "unsupported client method"})

    def close(self):
        for _, terminal in self.terminals.values():
            terminal.kill()
        self.workers.shutdown(wait=True, cancel_futures=True)


class Terminal:
    def __init__(self, kbox, box, params):
        cwd = params.get("cwd", "/workspace")
        if not PurePosixPath(cwd).is_absolute():
            raise ValueError("terminal cwd must be absolute")
        self.limit = params.get("outputByteLimit", 65536)
        if type(self.limit) is not int or not 0 <= self.limit <= 4 * 1024 * 1024:
            raise ValueError("terminal output limit must be between 0 and 4 MiB")
        self.kbox, self.box = kbox, box
        command = [kbox, "exec", "-t", "--timeout", "1800", "-w", cwd]
        for item in params.get("env", []):
            name, value = item["name"], item["value"]
            if not name or "=" in name or "\0" in name + value:
                raise ValueError("invalid terminal environment")
            command += ["-e", f"{name}={value}"]
        command += [box, "--", "setsid", "--wait", "/bin/sh", "-c",
                    'printf "__KBOX_TERMINAL_PID__=%s\\n" "$$"; exec "$@"',
                    "acp-terminal", params["command"], *params.get("args", [])]
        # kbox's PTY path streams output; its ordinary exec path returns captured output at exit.
        self.pty_master, slave = pty.openpty()
        try:
            self.proc = subprocess.Popen(command, stdin=slave, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        except Exception:
            os.close(self.pty_master)
            raise
        finally:
            os.close(slave)
        self.buffer, self.truncated = b"", False
        self.lock, self.ready = threading.Lock(), threading.Event()
        self.pid = None
        self.reader = threading.Thread(target=self._read, daemon=True)
        self.reader.start()

    def _append(self, chunk):
        with self.lock:
            data = self.buffer + chunk
            self.truncated |= len(data) > self.limit
            self.buffer = data[-self.limit:] if self.limit else b""

    def _read(self):
        first = self.proc.stdout.readline(256)
        if first.startswith(b"__KBOX_TERMINAL_PID__="):
            try:
                self.pid = int(first.split(b"=", 1)[1])
            except ValueError:
                self._append(first)
        else:
            self._append(first)
        self.ready.set()
        while chunk := os.read(self.proc.stdout.fileno(), 8192):
            self._append(chunk)

    def wait(self):
        code = self.proc.wait()
        self.reader.join()
        return {"exitCode": code} if code >= 0 else {"signal": signal.Signals(-code).name}

    def output(self):
        with self.lock:
            result = {"output": self.buffer.decode("utf-8", errors="ignore"), "truncated": self.truncated}
        if self.proc.poll() is not None:
            result["exitStatus"] = self.wait()
        return result

    def kill(self):
        self.ready.wait(timeout=5)
        if self.pid is not None and self.proc.poll() is None:
            subprocess.run([self.kbox, "exec", "--timeout", "5", self.box, "--", "/bin/kill", "-TERM",
                            "--", f"-{self.pid}"], capture_output=True, timeout=10, check=False)
            try:
                self.proc.wait(timeout=3)
            except subprocess.TimeoutExpired:
                subprocess.run([self.kbox, "exec", "--timeout", "5", self.box, "--", "/bin/kill", "-KILL",
                                "--", f"-{self.pid}"], capture_output=True, timeout=10, check=False)
        self.proc.wait(timeout=10)
        self.reader.join(timeout=5)
        self.proc.stdout.close()
        if self.pty_master is not None:
            os.close(self.pty_master)
            self.pty_master = None
        return {}
