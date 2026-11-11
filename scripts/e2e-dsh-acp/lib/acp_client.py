"""Bounded stdio ACP client for live task, cancellation and persistence checks."""

import json
import os
import queue
import subprocess
import threading
import time


class RpcError(RuntimeError):
    def __init__(self, error):
        self.code = error["code"]
        super().__init__(error.get("message", "ACP error"))


class AcpClient:
    def __init__(self, command, directory, label, *, bootstrap=None, client_handler=None, secrets=(), env=None):
        self.started = time.monotonic()
        self.stderr = (directory / f"{label}.stderr.log").open("w")
        self.trace = (directory / f"{label}.jsonl").open("w", encoding="utf-8")
        self.proc = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=self.stderr, text=True, encoding="utf-8", env=env)
        self.inbox = queue.Queue(maxsize=256)
        self.responses = {}
        self.updates = []
        self.permissions = []
        self.allow_permissions = False
        self.sequence = 0
        self.client_handler = client_handler
        self.pending_client = {}
        self.secrets = tuple(secrets)
        if bootstrap is not None:
            # Credentials enter the guest over stdin, never argv, Box state or the trace.
            self.proc.stdin.write(json.dumps(bootstrap) + "\n")
            self.proc.stdin.flush()
        threading.Thread(target=self._read, daemon=True).start()

    def _read(self):
        try:
            while True:
                line = self.proc.stdout.readline(4 * 1024 * 1024 + 1)
                if not line:
                    raise EOFError(f"ACP disconnected (exit={self.proc.poll()})")
                if len(line) > 4 * 1024 * 1024 or not line.endswith("\n"):
                    raise ValueError("oversized or truncated ACP line")
                try:
                    message = json.loads(line)
                except json.JSONDecodeError as error:
                    raise ValueError(f"non-protocol stdout: {line[:160]!r}") from error
                if message.get("jsonrpc") != "2.0":
                    raise ValueError("stdout contains a non-JSON-RPC message")
                self.inbox.put(message)
        except Exception as error:
            self.inbox.put(error)

    def _log(self, direction, message):
        # Thoughts are irrelevant to the gate; retain their presence, not their text.
        if message.get("params", {}).get("update", {}).get("sessionUpdate") == "agent_thought_chunk":
            message = {"method": "session/update", "params": {"sessionId": message["params"]["sessionId"],
                       "update": {"sessionUpdate": "agent_thought_chunk"}}}
        line = json.dumps({"elapsed_s": round(time.monotonic() - self.started, 3),
                           "direction": direction, "message": message}, ensure_ascii=False)
        for secret in (*self.secrets, os.environ.get("DEEPSEEK_API_KEY"), os.environ.get("OPENAI_API_KEY")):
            if secret:
                line = line.replace(secret, "<redacted>")
        self.trace.write(line + "\n")
        self.trace.flush()

    def _write(self, message):
        self._log("client", message)
        self.proc.stdin.write(json.dumps(message, ensure_ascii=False) + "\n")
        self.proc.stdin.flush()

    def send(self, method, params, notification=False):
        message = {"jsonrpc": "2.0", "method": method, "params": params}
        if not notification:
            self.sequence += 1
            message["id"] = self.sequence
        self._write(message)
        return message.get("id")

    def pump(self, timeout=0.25):
        for request_id, future in list(self.pending_client.items()):
            if future.done():
                del self.pending_client[request_id]
                try:
                    self._client_result(request_id, future.result())
                except Exception as error:
                    self._client_error(request_id, error)
        try:
            message = self.inbox.get(timeout=timeout)
        except queue.Empty:
            return
        if isinstance(message, Exception):
            raise message
        self._log("agent", message)
        if "method" in message and "id" in message:
            if message["method"] == "session/request_permission":
                kind = "allow_once" if self.allow_permissions else "reject_once"
                option = next((o for o in message["params"]["options"] if o["kind"] == kind), None)
                outcome = {"outcome": "cancelled"} if option is None else {
                    "outcome": "selected", "optionId": option["optionId"]}
                self.permissions.append({"request": message["params"], "outcome": outcome})
                self._write({"jsonrpc": "2.0", "id": message["id"], "result": {"outcome": outcome}})
            elif self.client_handler is not None:
                try:
                    result = self.client_handler(message["method"], message["params"])
                    if hasattr(result, "done"):
                        self.pending_client[message["id"]] = result
                    else:
                        self._client_result(message["id"], result)
                except Exception as error:
                    self._client_error(message["id"], error)
            else:
                self._write({"jsonrpc": "2.0", "id": message["id"],
                             "error": {"code": -32601, "message": "unsupported client method"}})
        elif message.get("method") == "session/update":
            if message["params"]["update"]["sessionUpdate"] != "agent_thought_chunk":
                self.updates.append(message["params"])
        elif "id" in message:
            self.responses[message["id"]] = message

    def _client_result(self, request_id, result):
        self._write({"jsonrpc": "2.0", "id": request_id, "result": result})

    def _client_error(self, request_id, error):
        self._write({"jsonrpc": "2.0", "id": request_id,
                     "error": {"code": getattr(error, "code", -32000), "message": str(error)}})

    def wait(self, request_id, timeout=60, tick=None):
        deadline = time.monotonic() + timeout
        while request_id not in self.responses:
            if time.monotonic() >= deadline:
                raise TimeoutError(f"ACP request {request_id} exceeded {timeout}s")
            if tick is not None:
                tick()
            self.pump(min(0.25, max(0, deadline - time.monotonic())))
        response = self.responses.pop(request_id)
        if "error" in response:
            raise RpcError(response["error"])
        return response["result"]

    def request(self, method, params, timeout=60):
        return self.wait(self.send(method, params), timeout)

    def initialize(self, capabilities=None):
        result = self.request("initialize", {"protocolVersion": 1, "clientCapabilities": capabilities or {},
                              "clientInfo": {"name": "kbox-acceptance", "version": "1"}}, 180)
        assert result["protocolVersion"] == 1
        if "agentInfo" in result:
            assert result["agentInfo"]["name"]
        return result

    def new_session(self, cwd="/workspace"):
        result = self.request("session/new", {"cwd": cwd, "mcpServers": []})
        assert isinstance(result["sessionId"], str) and result["sessionId"]
        return result["sessionId"]

    def configure(self, session):
        model = os.environ.get("DSH_MODEL", "deepseek-flash")
        provider = os.environ.get("DSH_PROVIDER", "deepseek-official")
        if "/" in model:
            provider, model = model.split("/", 1)
        for option, value in (("model", json.dumps([provider, model], separators=(",", ":"))),
                              ("reasoning_effort", os.environ.get("DSH_REASONING_EFFORT", "low"))):
            result = self.request("session/set_config_option", {
                "sessionId": session, "configId": option, "value": value})
            assert any(o["id"] == option and o["currentValue"] == value for o in result["configOptions"])

    def prompt(self, session, text, timeout=480, tick=None):
        start = len(self.updates)
        began = time.monotonic()
        request = self.send("session/prompt", {"sessionId": session, "prompt": [{"type": "text", "text": text}]})
        result = self.wait(request, timeout, tick)
        assert result["stopReason"] == "end_turn", result
        updates = [u["update"] for u in self.updates[start:] if u["sessionId"] == session]
        reply = "".join(u["content"]["text"] for u in updates if u["sessionUpdate"] == "agent_message_chunk"
                        and u["content"]["type"] == "text")
        assert reply.strip(), "empty agent reply"
        calls = {u["toolCallId"] for u in updates if u["sessionUpdate"] == "tool_call"}
        settled = {u["toolCallId"] for u in updates if u["sessionUpdate"] in ("tool_call", "tool_call_update")
                   and u.get("status") in ("completed", "failed")}
        assert calls <= settled, f"prompt settled with unfinished tool calls: {calls - settled}"
        return {"reply": reply, "elapsed_s": round(time.monotonic() - began, 3),
                "tool_calls": len(calls), "updates": len(updates)}

    def close(self, strict=True):
        try:
            if not self.proc.stdin.closed:
                self.proc.stdin.close()
            try:
                code = self.proc.wait(timeout=45)
            except subprocess.TimeoutExpired:
                self.proc.terminate()
                try:
                    self.proc.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    self.proc.kill()
                    self.proc.wait(timeout=10)
                if strict:
                    raise AssertionError("ACP process did not exit on stdin EOF")
                return
            if strict:
                assert code == 0, f"ACP process exited {code}"
        finally:
            self.trace.close()
            self.stderr.close()
            self.proc.stdout.close()
