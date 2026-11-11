#!/usr/bin/env python3
"""Exercise primary process streams through the real KBox daemon and MicroVM."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import struct
import subprocess
import tempfile
import threading
import time
import uuid

from native_smoke import recv_exact, wait_for


def send_frame(stream, payload):
    stream.sendall(struct.pack("<I", len(payload)) + payload)


def read_frame(stream, limit=16 * 1024 * 1024):
    size = struct.unpack("<I", recv_exact(stream, 4))[0]
    assert 0 < size <= limit, "invalid wire frame size"
    return recv_exact(stream, size)


def connect(endpoint, command):
    stream = socket.socket(socket.AF_UNIX)
    stream.settimeout(90)
    stream.connect(str(endpoint))
    send_frame(stream, json.dumps({"schema": "kbox.engine.request.v1", "command": command}).encode())
    return stream


def response(stream, expect=None):
    reply = json.loads(read_frame(stream))
    assert reply["schema"] == "kbox.engine.response.v1"
    if expect:
        assert reply["status"] == "error" and reply["error"]["code"] == expect, reply
        return reply
    assert reply["status"] == "ok", reply
    return reply["result"]["value"]


def rpc(endpoint, command, expect=None):
    with connect(endpoint, command) as stream:
        return response(stream, expect)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--kbox", required=True, type=Path)
    parser.add_argument("--image-store", required=True, type=Path)
    parser.add_argument("--image", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    binary = args.kbox.resolve()
    assert binary.is_file() and args.image_store.is_dir()
    assert "@sha256:" in args.image
    root = args.output.resolve() / uuid.uuid4().hex
    home = root / "home"
    home.mkdir(parents=True, mode=0o700)
    (home / "bin").mkdir(mode=0o700)
    (home / "lib").mkdir(mode=0o700)
    for name in ("kbox-shim", "kbox-guest-init"):
        source = binary.parent / name
        assert source.is_file(), f"incomplete runtime prefix: {name}"
        shutil.copy2(source, home / "bin" / name)
    for source in (binary.parent.parent / "lib").iterdir():
        if source.name.endswith(".dylib") or ".so" in source.name:
            (home / "lib" / source.name).symlink_to(source.resolve())
    (home / "images").symlink_to(args.image_store.resolve(), target_is_directory=True)
    environment = dict(os.environ, K_HOME=str(home))
    binary_digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    events, engines, sockets, writers = [], [], [], []
    counter = 0
    endpoint = None
    intent = None
    failure = None

    def cli(*arguments, check=True):
        nonlocal counter
        counter += 1
        result = subprocess.run([str(binary), *map(str, arguments)], env=environment,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=120)
        (root / f"{counter:03d}.stdout").write_bytes(result.stdout)
        (root / f"{counter:03d}.stderr").write_bytes(result.stderr)
        events.append({"command": list(map(str, arguments)), "exitCode": result.returncode})
        if check:
            assert result.returncode == 0, f"native CLI command {counter} failed; logs retained"
        return result.stdout

    try:
        with tempfile.TemporaryDirectory(prefix="kp-", dir="/private/tmp" if Path("/private/tmp").is_dir() else "/tmp") as short:
            Path(short).chmod(0o700)
            endpoint = Path(short) / "s"

            def start_engine():
                with (root / f"engine-{len(engines)}.stdout").open("wb") as out, (root / f"engine-{len(engines)}.stderr").open("wb") as err:
                    process = subprocess.Popen([str(binary), "engine", "serve", "--socket", str(endpoint)],
                                               env=environment, stdout=out, stderr=err)
                engines.append(process)
                discovery = wait_for(lambda: rpc(endpoint, {"op": "discover"}))
                assert "process-stream.primary.v1" in discovery["protocols"]
                return discovery

            discovery = start_engine()
            instance = uuid.uuid4().hex
            service = {
                "artifact": {"uri": "oci://" + args.image, "digest": args.image.rsplit("@", 1)[1],
                             "media_type": "application/vnd.oci.image.manifest.v1+json"},
                "process": {"command": ["/bin/sleep"], "args": ["2147483647"],
                            "working_directory": "/", "environment": {}},
                "mounts": [{"name": "data", "source": {"kind": "volume", "volume_id": instance},
                            "target": "/data", "read_only": False}],
                "secrets": [],
                "network": {"host_services": [], "mode": "none", "ports": [], "egress_allowlist": None},
                "resources": {"cpu_millis": 1000, "memory_bytes": 536870912, "pids": 4294967295,
                              "ephemeral_storage_bytes": None, "execution_timeout_ms": None},
                "isolation": "sandbox", "health": None, "service_lifecycle": None, "restart": {"kind": "never"},
            }
            intent = {"schema": "kbox.engine.intent.v1", "engineId": discovery["engineId"],
                      "instanceId": instance, "requestId": "up-1", "revision": 1,
                      "desiredState": "up", "workload": "acp-harness",
                      "planDigest": "sha256:" + hashlib.sha256(instance.encode()).hexdigest(), "service": service}
            accepted = rpc(endpoint, {"op": "apply_intent", "intent": intent})

            def running(generation):
                with connect(endpoint, {"op": "inspect_execution", "instance_id": instance}) as query:
                    reply = json.loads(read_frame(query))
                if reply["status"] == "error":
                    assert reply["error"]["code"] in {"UNKNOWN", "CLEANUP_PENDING"}, reply
                    return False
                result = reply["result"]["value"]
                runtime = result.get("runtime") or {}
                observed = runtime.get("observation") or {}
                if result["targetGeneration"] == generation and observed.get("generation") == generation and observed.get("state") == "running":
                    return observed

            current = wait_for(lambda: running(1))
            volumes = cli("volume", "ls", "-q").decode().splitlines()
            assert len(volumes) == 1
            volume = Path(json.loads(cli("volume", "inspect", volumes[0]))[0]["Mountpoint"])

            def primary_request(name, generation, program):
                return {"schema": "kbox.engine.primary-open.v1", "engineId": discovery["engineId"],
                        "instanceId": instance, "requestId": name, "generation": generation,
                        "command": ["python3", "-u", "-c", program], "environment": {},
                        "workingDirectory": "/data", "timeoutMs": None}

            def open_primary(request):
                control = connect(endpoint, {"op": "open_primary_process", "process": request})
                sockets.append(control)
                ticket = response(control)
                data = connect(endpoint, {"op": "attach_primary_process", "token": ticket["attachToken"]})
                sockets.append(data)
                info = response(data)
                assert info["state"] == "running" and info["runtimeGeneration"] == request["generation"]
                return control, data, ticket, info

            program = (
                "import pathlib,sys,time; sys.stdout.buffer.write(b'A'*(8*1024*1024)); sys.stdout.buffer.flush(); "
                "pathlib.Path('/tmp/stream-produced').write_text('ready'); data=sys.stdin.buffer.read(); time.sleep(6); "
                "sys.stdout.buffer.write(b'\\xff\\x00tail:'+str(len(data)).encode()); sys.stdout.buffer.flush(); "
                "sys.stderr.buffer.write('尾部stderr\\n'.encode()); sys.stderr.buffer.flush(); sys.exit(37)"
            )
            first = primary_request("primary-1", 1, program)
            started = time.monotonic()
            control, data, ticket, info = open_primary(first)
            assert info["executionId"] == current["provider_resource_id"]
            rpc(endpoint, {"op": "open_primary_process", "process": first}, expect="ALREADY_CONNECTED")
            rpc(endpoint, {"op": "attach_primary_process", "token": ticket["attachToken"]}, expect="ALREADY_CONNECTED")
            time.sleep(0.5)
            assert cli("exec", info["executionId"], "--", "python3", "-c",
                       "from pathlib import Path; print(int(Path('/tmp/stream-produced').exists()))").strip() == b"0"
            errors = []

            def write_input():
                try:
                    for _ in range(32):
                        send_frame(data, b"\x00" + bytes([123]) * 65536)
                    send_frame(data, b"\x01")
                except OSError as error:
                    errors.append(type(error).__name__)

            writer = threading.Thread(target=write_input, daemon=True)
            writers.append(writer)
            writer.start()
            stdout, stderr, exits = bytearray(), bytearray(), []
            while not exits:
                frame = read_frame(data, 65537)
                if frame[0] == 0:
                    stdout.extend(frame[1:])
                elif frame[0] == 1:
                    stderr.extend(frame[1:])
                else:
                    assert frame[0] == 2 and len(frame) == 5
                    exits.append(struct.unpack("<i", frame[1:])[0])
            assert data.recv(1) == b""
            writer.join(timeout=10)
            assert not writer.is_alive() and not errors
            assert stdout == b"A" * (8 * 1024 * 1024) + b"\xff\x00tail:2097152"
            assert stderr == "尾部stderr\n".encode() and exits == [37]
            assert time.monotonic() - started >= 6
            control.close()
            current = wait_for(lambda: running(2))
            assert current["provider_resource_id"] != info["executionId"]
            assert rpc(endpoint, {"op": "inspect_primary_process", "request_id": "primary-1"})["exit"]["exit_code"] == 37
            rpc(endpoint, {"op": "open_primary_process", "process": first}, expect="STREAM_LOST")
            assert rpc(endpoint, {"op": "apply_intent", "intent": intent}) == accepted

            child = "from pathlib import Path; import time\np=Path({path!r})\nwhile True:\n with p.open('ab') as f: f.write(b'x'); f.flush()\n time.sleep(0.05)\n"
            retained = {}
            for generation, name, crash in [(2, "断开 late writes.txt", False), (3, "崩溃 late writes.txt", True)]:
                code = ("import subprocess,sys,time; subprocess.Popen([sys.executable,'-c'," +
                        repr(child.format(path="/data/" + name)) + "]); "
                        "sys.stdout.buffer.write(b'B'*(16*1024*1024)); sys.stdout.buffer.flush(); time.sleep(2147483647)")
                request = primary_request(f"primary-{generation}", generation, code)
                control, data, _, info = open_primary(request)
                target = volume / name
                wait_for(lambda: target.exists() and target.stat().st_size > 0)
                outcome = []

                def saturate_input(connection=data, outcome=outcome):
                    try:
                        for _ in range(256):
                            send_frame(connection, b"\x00" + b"z" * 65536)
                        outcome.append("unexpected-completion")
                    except OSError:
                        outcome.append("disconnected")

                writer = threading.Thread(target=saturate_input, daemon=True)
                writers.append(writer)
                writer.start()
                time.sleep(0.2)
                assert writer.is_alive(), "native input did not backpressure"
                if crash:
                    engines[-1].kill()
                    engines[-1].wait(timeout=20)
                    assert start_engine()["engineId"] == discovery["engineId"]
                else:
                    control.close()
                current = wait_for(lambda: running(generation + 1))
                assert current["provider_resource_id"] != info["executionId"]
                writer.join(timeout=10)
                assert not writer.is_alive() and outcome == ["disconnected"]
                process = rpc(endpoint, {"op": "inspect_primary_process", "request_id": request["requestId"]})
                assert process["state"] == "lost" and process["exit"] is None
                retained[name] = {"bytes": target.stat().st_size, "sha256": hashlib.sha256(target.read_bytes()).hexdigest(),
                                  "confirmedAt": time.monotonic()}
                control.close()
                data.close()

            lost = primary_request("lost-open", 4, "raise RuntimeError('must never be launched')")
            lost_control = connect(endpoint, {"op": "open_primary_process", "process": lost})
            sockets.append(lost_control)

            def reserved():
                with connect(endpoint, {"op": "inspect_primary_process", "request_id": "lost-open"}) as query:
                    reply = json.loads(read_frame(query))
                if reply["status"] == "error":
                    assert reply["error"]["code"] == "NOT_FOUND"
                    return False
                return reply["result"]["value"]["state"] == "reserved"

            wait_for(reserved)
            lost_control.close()
            wait_for(lambda: running(5))
            rpc(endpoint, {"op": "open_primary_process", "process": lost}, expect="STREAM_LOST")
            time.sleep(50)
            for name, receipt in retained.items():
                target = volume / name
                assert target.stat().st_size == receipt["bytes"]
                assert hashlib.sha256(target.read_bytes()).hexdigest() == receipt["sha256"]
                receipt["quietSeconds"] = time.monotonic() - receipt.pop("confirmedAt")
                assert receipt["quietSeconds"] >= 50
            shutil.copytree(volume, root / "retained-volume-data")
            down = dict(intent, requestId="down-2", revision=2, desiredState="down", service=None)
            rpc(endpoint, {"op": "apply_intent", "intent": down})
            wait_for(lambda: (rpc(endpoint, {"op": "inspect_execution", "instance_id": instance}).get("runtime") or {}).get("status") == "not_found")
            assert json.loads(cli("ps", "-a", "--format", "json")) == []
            for name in volumes:
                cli("volume", "rm", name)
            assert cli("volume", "ls", "-q").strip() == b""
            assert hashlib.sha256(binary.read_bytes()).hexdigest() == binary_digest
            (root / "result.json").write_text(json.dumps({"status": "PASS", "platform": os.uname().sysname,
                "kboxSha256": binary_digest, "image": args.image, "engineId": discovery["engineId"],
                "stdinBytes": 2097152, "stdoutBytes": len(stdout), "exitCode": 37,
                "backpressureAndConcurrentExec": True, "halfCloseAndTails": True,
                "noReplayAfterExitDisconnectAndEngineCrash": True, "lostOpenNotLaunched": True,
                "lateWriteChecks": retained, "zeroPrivateExecutionsAndVolumes": True}, indent=2) + "\n")
            print(root)
    except BaseException as error:
        failure = error
        (root / "failure.json").write_text(json.dumps({"type": type(error).__name__, "message": str(error)}, indent=2) + "\n")
        raise
    finally:
        for stream in sockets:
            stream.close()
        for process in engines:
            if process.poll() is None:
                process.send_signal(signal.SIGINT)
                try:
                    process.wait(timeout=30)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=20)
        try:
            inventory = cli("ps", "-a", "--format", "json", check=False)
            for record in json.loads(inventory):
                cli("rm", "-f", record["id"], check=False)
        except (ValueError, OSError, subprocess.SubprocessError) as error:
            (root / "cleanup-error.json").write_text(json.dumps({"type": type(error).__name__, "message": str(error)}, indent=2) + "\n")
            if failure is None:
                raise
        finally:
            (root / "commands.json").write_text(json.dumps(events, indent=2) + "\n")


if __name__ == "__main__":
    main()
