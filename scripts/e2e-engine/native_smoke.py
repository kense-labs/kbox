#!/usr/bin/env python3
"""Native OCI and engine regression. Uses kbox exclusively, in a fresh owned home."""
import argparse
import hashlib
import http.server
import json
import os
from pathlib import Path
import signal
import socket
import struct
import subprocess
import tarfile
import tempfile
import threading
import time
import uuid


def recv_exact(stream, size):
    value = bytearray()
    while len(value) < size:
        chunk = stream.recv(size - len(value))
        if not chunk:
            raise EOFError("engine closed before the response")
        value.extend(chunk)
    return bytes(value)


def rpc(path, command, expect=None):
    with socket.socket(socket.AF_UNIX) as stream:
        stream.settimeout(90)
        stream.connect(str(path))
        payload = json.dumps({"schema": "kbox.engine.request.v1", "command": command}).encode()
        stream.sendall(struct.pack("<I", len(payload)) + payload)
        size = struct.unpack("<I", recv_exact(stream, 4))[0]
        assert size <= 16 * 1024 * 1024
        reply = json.loads(recv_exact(stream, size))
    assert reply["schema"] == "kbox.engine.response.v1", reply
    if expect:
        assert reply["status"] == "error" and reply["error"]["code"] == expect, reply
        return reply
    assert reply["status"] == "ok", reply
    return reply["result"]["value"]


def wait_for(probe, timeout=120):
    end = time.monotonic() + timeout
    while True:
        try:
            result = probe()
            if result:
                return result
        except (OSError, EOFError):
            pass
        if time.monotonic() >= end:
            raise TimeoutError("native condition did not become true")
        time.sleep(0.1)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--kbox", required=True, type=Path)
    parser.add_argument("--image-store", required=True, type=Path)
    parser.add_argument("--base", required=True, help="OCI image containing python3 and bwrap")
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    assert args.kbox.is_file() and args.image_store.is_dir()
    binary_digest = hashlib.sha256(args.kbox.read_bytes()).hexdigest()
    nonce = uuid.uuid4().hex
    root = args.output.resolve() / nonce
    root.mkdir(parents=True, mode=0o700)
    home = root / "home"
    home.mkdir(mode=0o700)
    (home / "images").symlink_to(args.image_store.resolve(), target_is_directory=True)
    workspace = root / "外部目录 with spaces"
    workspace.mkdir()
    environment = dict(os.environ, K_HOME=str(home))
    events = []
    engines = []
    counter = 0
    committed = f"kbox-oci-commit:{nonce}"
    pool_root = tempfile.TemporaryDirectory(prefix="ko-pool-", dir="/private/tmp" if Path("/private/tmp").is_dir() else "/tmp")
    Path(pool_root.name).chmod(0o700)
    build_socket = str(Path(pool_root.name) / "pool.sock")

    def cli(*arguments, timeout=240, check=True):
        nonlocal counter
        counter += 1
        result = subprocess.run([str(args.kbox.resolve()), *map(str, arguments)], env=environment,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout)
        (root / f"{counter:03d}.stdout").write_bytes(result.stdout)
        (root / f"{counter:03d}.stderr").write_bytes(result.stderr)
        events.append({"command": list(map(str, arguments)), "exitCode": result.returncode})
        if check and result.returncode:
            raise RuntimeError(f"kbox command {counter} failed; see retained logs")
        return result.stdout

    try:
        # Registry resolution, image build (including RUN), ordinary OCI process and mounts.
        cli("pull", "busybox:1.37.0")
        context = root / "context"
        context.mkdir()
        (context / "marker").write_text(nonce)
        (context / "Containerfile").write_text(
            f"FROM {args.base}\nUSER root\nCOPY marker /oci-marker\n"
            "RUN python3 -c 'from pathlib import Path; Path(\"/image-build-proof\").write_text(Path(\"/oci-marker\").read_text())'\n"
            'ENTRYPOINT []\nCMD ["sleep", "2147483647"]\n')
        tag = f"kbox-oci-regression:{nonce}"
        cli("build", "--run-pool-socket", build_socket, "--run-pool-autostart",
            "--run-pool-image", args.base, "-f", context / "Containerfile", "-t", tag, context)
        cli("pool", "stop", "--socket", build_socket)
        image = json.loads(cli("image-inspect", tag))[0]
        digest = image["Digest"]
        reference = "docker.io/library/kbox-oci-regression"
        cli("volume", "create", "ordinary-data")
        cli("run", "-d", "--name", "ordinary", "-v",
            f"{workspace}:/workspace", "-v", "ordinary-data:/data", tag)
        body = nonce.encode()

        class Handler(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                self.send_response(200)
                self.end_headers()
                self.wfile.write(body)

            def log_message(self, *_):
                pass

        httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=httpd.serve_forever, daemon=True).start()
        try:
            program = (
                "import hashlib,json,pathlib,subprocess,urllib.request; "
                f"proof=pathlib.Path('/image-build-proof').read_text(); assert proof=={nonce!r},('image proof',proof,{nonce!r}); "
                f"remote=urllib.request.urlopen('http://127.0.0.1:{httpd.server_port}',timeout=10).read(); assert remote=={body!r},('loopback nonce',remote,{body!r}); "
                "value=bytes(range(256))*4096; p=pathlib.Path('/workspace/中文 result.bin'); "
                "p.write_bytes(value); p.rename('/workspace/renamed.bin'); "
                f"pathlib.Path('/data/persistent').write_text({nonce!r}); "
                f"pathlib.Path('/native-rootfs-check').write_text({nonce!r}); "
                "assert subprocess.run(['python3','-c','print(7)'],capture_output=True,check=True).stdout.strip()==b'7'; "
                "print(json.dumps({'bytes':len(value),'sha256':hashlib.sha256(value).hexdigest()}))"
            )
            probe = json.loads(cli("exec", "ordinary", "--", "python3", "-c", program))
            assert hashlib.sha256((workspace / "renamed.bin").read_bytes()).hexdigest() == probe["sha256"]
        finally:
            httpd.shutdown()
            httpd.server_close()
        with tempfile.TemporaryDirectory(dir=root) as export_dir:
            archive = Path(export_dir) / "rootfs.tar"
            cli("export", "ordinary", "-o", archive)
            with tarfile.open(archive) as rootfs:
                entries = {member.name.removeprefix("./").lstrip("/"): member for member in rootfs}
                with rootfs.extractfile(entries["native-rootfs-check"]) as marker:
                    assert marker.read() == nonce.encode()
                assert "workspace/renamed.bin" not in entries
                assert "data/persistent" not in entries
        cli("commit", "ordinary", committed)
        assert cli("run", "--rm", committed, "--", "/bin/cat", "/native-rootfs-check").strip() == nonce.encode()
        cli("rmi", committed)
        cli("stop", "ordinary")
        cli("snapshot", "create", "ordinary", "--name", "ordinary-snapshot")
        cli("snapshot", "restore", "ordinary-snapshot", "--name", "restored")
        cli("start", "restored")
        cli("exec", "restored", "--", "python3", "-c",
            f"from pathlib import Path; assert Path('/native-rootfs-check').read_text()=={nonce!r}; assert Path('/data/persistent').read_text()=={nonce!r}")
        cli("rm", "-f", "restored", "ordinary")
        cli("snapshot", "rm", "ordinary-snapshot")
        cli("volume", "rm", "ordinary-data")
        assert json.loads(cli("ps", "-a", "--format", "json")) == []
        assert not cli("volume", "ls", "-q").strip()

        # Engine service has no ACP executable or manifest. Restart must not create another VM.
        with tempfile.TemporaryDirectory(prefix="ke-", dir="/private/tmp" if Path("/private/tmp").is_dir() else "/tmp") as socket_root:
            Path(socket_root).chmod(0o700)
            endpoint = Path(socket_root) / "control.sock"

            def start_engine():
                stdout = open(root / f"engine-{len(engines)}.stdout", "wb")
                stderr = open(root / f"engine-{len(engines)}.stderr", "wb")
                process = subprocess.Popen([str(args.kbox.resolve()), "engine", "serve", "--socket", str(endpoint)],
                                           env=environment, stdout=stdout, stderr=stderr)
                stdout.close()
                stderr.close()
                engines.append(process)
                return wait_for(lambda: rpc(endpoint, {"op": "discover"}))

            discovery = start_engine()
            service = {
                "artifact": {"uri": f"oci://{reference}@{digest}", "digest": digest,
                             "media_type": "application/vnd.oci.image.manifest.v1+json"},
                "process": {"command": ["/bin/sleep"], "args": ["2147483647"],
                            "working_directory": "/", "environment": {}},
                "mounts": [], "secrets": [],
                "network": {"host_services": [], "mode": "none", "ports": [], "egress_allowlist": None},
                "resources": {"cpu_millis": 1000, "memory_bytes": 536870912, "pids": 256,
                              "ephemeral_storage_bytes": None, "execution_timeout_ms": None},
                "isolation": "sandbox", "health": None, "service_lifecycle": None, "restart": {"kind": "never"},
            }
            intent = {"schema": "kbox.engine.intent.v2", "engineId": discovery["engineId"],
                      "instanceId": nonce, "requestId": "up-1", "revision": 1,
                      "desiredState": "up",
                      "planDigest": "sha256:" + hashlib.sha256(nonce.encode()).hexdigest(), "service": service}
            accepted = rpc(endpoint, {"op": "apply_intent", "intent": intent})

            def running():
                execution = rpc(endpoint, {"op": "inspect_execution", "instance_id": nonce})
                runtime = execution["runtime"]
                if (runtime and runtime.get("observation", {}).get("state") == "running"
                        and runtime["observation"]["generation"] == execution["targetGeneration"]):
                    return runtime["observation"]

            before = wait_for(running)
            engines[-1].kill()
            engines[-1].wait(timeout=20)
            cli("exec", before["provider_resource_id"], "--", "python3", "-c", "print('survived-engine-exit')")
            assert start_engine()["engineId"] == discovery["engineId"]
            assert rpc(endpoint, {"op": "apply_intent", "intent": intent}) == accepted
            after = wait_for(running)
            assert before["provider_resource_id"] == after["provider_resource_id"]
            assert before["generation"] == after["generation"]
            down = dict(intent, requestId="down-2", revision=2, desiredState="down", service=None)
            rpc(endpoint, {"op": "apply_intent", "intent": down})
            wait_for(lambda: (rpc(endpoint, {"op": "inspect_execution", "instance_id": nonce}).get("runtime") or {}).get("status") == "not_found")
            assert rpc(endpoint, {"op": "apply_intent", "intent": intent}) == accepted
            rpc(endpoint, {"op": "apply_intent", "intent": dict(intent, requestId="late-up")}, expect="STALE_INTENT")
            assert json.loads(cli("ps", "-a", "--format", "json")) == []
            # The same guest and resources, with full guest privileges explicitly selected.
            security_probe = (
                "import json,pathlib,subprocess; "
                "status=dict(line.split(':',1) for line in pathlib.Path('/proc/self/status').read_text().splitlines() if ':' in line); "
                "sandbox=subprocess.run(['bwrap','--unshare-user','--uid','0','--gid','0','--ro-bind','/','/','--','/bin/true'],capture_output=True); "
                "print(json.dumps(dict(security={key:status[key].strip() for key in ['CapEff','CapBnd','NoNewPrivs','Seccomp']},"
                "sandboxExit=sandbox.returncode,sandboxStderr=sandbox.stderr.decode())))"
            )
            cli("run", "-d", "--privileged", "--name", "native-baseline", "--network", "none",
                "--cpus", "1", "--memory", "512m", tag)
            baseline = json.loads(cli("exec", "native-baseline", "--", "python3", "-c", security_probe))
            cli("rm", "-f", "native-baseline")
            native_service = dict(service, semantics_profile_digest="sha256:daf902bf6663b433b3d9a97dffefc5044661decf3b872916209348a7e439baeb", resources=dict(service["resources"], pids=4294967295))
            native_up = dict(intent, requestId="native-up-3", revision=3, service=native_service)
            rpc(endpoint, {"op": "apply_intent", "intent": native_up})
            native_execution = wait_for(running)
            native = json.loads(cli("exec", native_execution["provider_resource_id"], "--", "python3", "-c", security_probe))
            assert native == baseline, (native, baseline)
            assert native["security"]["NoNewPrivs"] == "0" and native["security"]["Seccomp"] == "0", native
            assert int(native["security"]["CapEff"], 16) > 0 and native["sandboxExit"] == 0, native
            rpc(endpoint, {"op": "apply_intent", "intent": dict(native_up, requestId="native-down-4", revision=4,
                                                                desiredState="down", service=None)})
            wait_for(lambda: (rpc(endpoint, {"op": "inspect_execution", "instance_id": nonce}).get("runtime") or {}).get("status") == "not_found")
            assert json.loads(cli("ps", "-a", "--format", "json")) == []
        assert hashlib.sha256(args.kbox.read_bytes()).hexdigest() == binary_digest, "binary changed during native acceptance"
        (root / "result.json").write_text(json.dumps({"status": "PASS", "platform": os.uname().sysname,
            "kboxSha256": binary_digest, "imageDigest": digest, "fileProbe": probe, "engineId": discovery["engineId"],
            "operationId": accepted["operationId"], "sameExecutionAfterRestart": True,
            "ordinaryOciWithoutAcp": True, "exportedRootfsExcludesMounts": True,
            "committedImageRuns": True, "nativeGuestProbe": native,
            "nativeGuestMatchesDirectBaseline": True, "zeroPrivateExecutions": True}, indent=2) + "\n")
        print(root)
    finally:
        if Path(build_socket).exists():
            cli("pool", "stop", "--socket", build_socket, check=False)
        pool_root.cleanup()
        for process in engines:
            if process.poll() is None:
                process.send_signal(signal.SIGINT)
                try:
                    process.wait(timeout=20)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=20)
        # This fresh home belongs entirely to this test. Preserve logs and data on failure.
        try:
            for record in json.loads(cli("ps", "-a", "--format", "json")):
                cli("rm", "-f", record["id"], check=False)
            for name in cli("volume", "ls", "-q").decode().splitlines():
                cli("volume", "rm", name, check=False)
            cli("rmi", "-f", committed, check=False)
        finally:
            (root / "commands.json").write_text(json.dumps(events, indent=2) + "\n")


if __name__ == "__main__":
    main()
