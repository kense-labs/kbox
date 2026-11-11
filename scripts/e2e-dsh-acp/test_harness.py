"""No-key regression checks for the runner; these do not replace live E2E."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parent / "lib"))
from acp_client import AcpClient, RpcError


SERVER = r'''
import json, sys, time
def send(value):
    print(json.dumps({"jsonrpc":"2.0", **value}), flush=True)
for line in sys.stdin:
    request=json.loads(line)
    method=request["method"]
    if method == "error":
        send({"id":request["id"],"error":{"code":-32602,"message":"invalid"}})
    elif method == "heartbeat":
        for _ in range(20):
            send({"method":"session/update","params":{"sessionId":"s","update":{
                "sessionUpdate":"usage_update","used":1,"size":100}}})
            time.sleep(0.02)
        send({"id":request["id"],"result":{}})
    elif method == "bad":
        print("diagnostic on protocol stdout", flush=True)
    elif method == "session/prompt":
        def update(value):
            send({"method":"session/update","params":{"sessionId":"s","update":value}})
        update({"sessionUpdate":"tool_call","toolCallId":"tool-1","status":"in_progress"})
        send({"id":request["id"],"method":"session/request_permission","params":{
            "sessionId":"s","toolCall":{"toolCallId":"tool-1"},"options":[
            {"optionId":"yes","kind":"allow_once"},{"optionId":"no","kind":"reject_once"}]}})
        permission=json.loads(sys.stdin.readline())
        update({"sessionUpdate":"tool_call_update","toolCallId":"tool-1","status":"completed"})
        update({"sessionUpdate":"agent_message_chunk","content":{
            "type":"text","text":permission["result"]["outcome"]["optionId"]}})
        send({"id":request["id"],"result":{"stopReason":"end_turn"}})
'''


class HarnessTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.directory = Path(self.temp.name)
        self.clients = []

    def tearDown(self):
        for client in self.clients:
            client.close(strict=False)
        self.temp.cleanup()

    def client(self):
        client = AcpClient([sys.executable, "-u", "-c", SERVER], self.directory, "selfcheck")
        self.clients.append(client)
        return client

    def test_permission_request_is_not_confused_with_correlated_response(self):
        client = self.client()
        result = client.prompt("s", "run a tool", timeout=2)
        self.assertEqual(result["reply"], "no")
        self.assertEqual(result["tool_calls"], 1)
        client.allow_permissions = True
        self.assertEqual(client.prompt("s", "run another tool", timeout=2)["reply"], "yes")

    def test_notifications_cannot_extend_request_deadline(self):
        client = self.client()
        with self.assertRaises(TimeoutError):
            client.wait(client.send("heartbeat", {}), timeout=0.1)

    def test_typed_rpc_error_is_observable(self):
        with self.assertRaises(RpcError) as caught:
            self.client().request("error", {})
        self.assertEqual(caught.exception.code, -32602)

    def test_protocol_stdout_is_strict(self):
        with self.assertRaises(ValueError):
            self.client().request("bad", {})

    def helper(self, function, bad=False):
        binary = self.directory / "kbox"
        binary.write_text(f"#!{sys.executable}\n" + '''
import json, os, sys
if os.environ.get("BAD_REPLY"):
    print("not json")
elif sys.argv[1:] == ["image-inspect", "test:latest"]:
    print(json.dumps([{"Digest":"sha256:"+"a"*64}]))
elif sys.argv[1:] == ["ps", "--all", "--format", "json"]:
    print(json.dumps([{"status":"Up 5 minutes", "raw_status":v} for v in ("running","paused","stopped")]))
else:
    raise SystemExit(2)
''')
        binary.chmod(0o755)
        env = {**os.environ, "KBOX": str(binary)}
        if bad:
            env["BAD_REPLY"] = "1"
        return subprocess.run(["bash", "-c", 'source "$1"; ' + function, "selfcheck",
                               str(Path(__file__).parent / "lib/common.sh")],
                              env=env, capture_output=True, text=True)

    def test_cleanup_counts_real_cli_status_field(self):
        result = self.helper("e2e_running_box_count")
        self.assertEqual((result.returncode, result.stdout.strip()), (0, "2"))

    def test_broken_listing_cannot_be_reported_as_zero_boxes(self):
        self.assertNotEqual(self.helper("e2e_running_box_count", bad=True).returncode, 0)

    def test_digest_uses_actual_cli_and_manifest_field(self):
        result = self.helper("e2e_image_digest_hint test:latest")
        self.assertEqual((result.returncode, result.stdout.strip()), (0, "sha256:" + "a" * 64))

    def test_missing_live_credentials_is_a_failing_gate(self):
        env = {k: v for k, v in os.environ.items() if k != "DEEPSEEK_API_KEY"}
        rows = self.directory / "results.jsonl"
        result = subprocess.run([sys.executable, str(Path(__file__).parent / "cases/p4_tasks.py")],
                                env={**env, "RESULTS_JSONL": str(rows)}, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        statuses = [json.loads(line)["status"] for line in rows.read_text().splitlines()]
        self.assertEqual(statuses, ["BLOCKED"] * 9)

    def test_existing_evidence_cannot_be_overwritten(self):
        rows = self.directory / "results.jsonl"
        rows.write_text("retained evidence\n")
        result = subprocess.run(["bash", str(Path(__file__).parent / "run.sh")],
                                env={**os.environ, "E2E_OUT": str(self.directory)}, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("preserve earlier evidence", result.stderr)
        self.assertEqual(rows.read_text(), "retained evidence\n")

    def test_shell_permission_id_collision_preserves_tool_result(self):
        incoming, outgoing = self.directory / "incoming", self.directory / "outgoing"
        messages = [
            {"method": "session/update", "params": {"update": {"sessionUpdate": "tool_call", "toolCallId": "t"}}},
            {"id": 5, "method": "session/request_permission", "params": {"options": [
                {"kind": "allow_once", "optionId": "yes"}, {"kind": "reject_once", "optionId": "no"}]}},
            {"method": "session/update", "params": {"update": {"sessionUpdate": "tool_call_update", "status": "completed"}}},
            {"method": "session/update", "params": {"update": {"sessionUpdate": "agent_message_chunk", "content": {"type": "text", "text": "91"}}}},
            {"id": 5, "result": {"stopReason": "end_turn"}},
        ]
        incoming.write_text("".join(json.dumps(m) + "\n" for m in messages))
        code = 'source "$1"; exec 3>"$2" 4<"$3"; SESSION_ID=s; ACP_ALLOW_ONCE=1; acp_prompt test 2; [[ "$TOOL_COMPLETE_COUNT" == 1 && "$REPLY" == 91 ]]'
        result = subprocess.run(["bash", "-c", code, "selfcheck",
                                 str(Path(__file__).parent / "lib/acp_session.sh"), str(outgoing), str(incoming)],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(outgoing.read_text().splitlines()[1])["result"]["outcome"]["optionId"], "yes")


if __name__ == "__main__":
    unittest.main()
