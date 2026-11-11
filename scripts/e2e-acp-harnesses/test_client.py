import json
import os
from concurrent.futures import Future
from pathlib import Path
import sys
import subprocess
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'e2e-dsh-acp/lib'))
from acp_client import AcpClient
from client_services import ClientServices


SERVER = r'''
import json, os, sys
secret = json.loads(sys.stdin.readline())["secret"]
request = json.loads(sys.stdin.readline())
print(json.dumps({"jsonrpc":"2.0", "id":request["id"], "result":{
    "protocolVersion":1,"agentCapabilities":{},"echo":request["params"]["clientCapabilities"],
    "environment":os.environ.get("ACP_ENV_TEST")}}), flush=True)
request = json.loads(sys.stdin.readline())
print(json.dumps({"jsonrpc":"2.0", "id":request["id"], "method":"fs/read_text_file",
    "params":{"sessionId":"s","path":"/workspace/test"}}), flush=True)
response = json.loads(sys.stdin.readline())
print(json.dumps({"jsonrpc":"2.0", "id":request["id"], "result":response["result"]}), flush=True)
for line in sys.stdin: pass
'''


class ClientTests(unittest.TestCase):
    def test_unknown_session_accepts_standard_and_legacy_typed_errors(self):
        sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'e2e-dsh-acp/cases'))
        from p4_tasks import invalid
        from acp_client import RpcError
        class Agent:
            def request(self, *_):
                raise RpcError({'code': self.code, 'message': 'unknown session'})
        agent = Agent()
        for code in (-32002, -32602):
            agent.code = code
            invalid(agent, 'session/prompt', {}, code=(-32002, -32602))
        agent.code = -32603
        with self.assertRaises(AssertionError):
            invalid(agent, 'session/prompt', {}, code=(-32002, -32602))

    def test_failure_before_cases_is_nine_blocked_cases(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            binary = root / 'kbox'
            binary.write_text(f'#!{sys.executable}\nimport json\nprint(json.dumps([{{"Digest":"sha256:' +
                              'a' * 64 + '"}]))\n')
            binary.chmod(0o755)
            config = root / 'config.kcl'
            config.write_text('')
            result = subprocess.run([sys.executable, str(Path(__file__).parent / 'run.py'), '--harness',
                                     'deepseek', '--image', 'test', '--out', str(root / 'out'),
                                     '--image-store', str(root), '--provider-config', str(config)],
                                    env={**os.environ, 'KBOX': str(binary)}, capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            report = json.loads((root / 'out/results.json').read_text())
            self.assertFalse(report['accepted'])
            self.assertEqual(report['summary']['blocked'], 9)

    def test_bootstrap_is_not_traced_and_optional_agent_info_is_accepted(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            pending = Future()
            client = AcpClient([sys.executable, '-u', '-c', SERVER], root, 'wire',
                               bootstrap={'secret': 'synthetic-secret'}, client_handler=lambda *_: pending,
                               env={**os.environ, 'ACP_ENV_TEST':'synthetic-source'}, secrets=['synthetic-source'])
            try:
                init = client.initialize(ClientServices.capabilities)
                self.assertEqual(init['echo'], ClientServices.capabilities)
                self.assertEqual(init['environment'], 'synthetic-source')
                request = client.send('request-file', {})
                client.pump(1)
                self.assertNotIn(request, client.responses)
                self.assertEqual(len(client.pending_client), 1)
                pending.set_result({'content': 'callback-result'})
                self.assertEqual(client.wait(request, 2), {'content': 'callback-result'})
            finally:
                client.close()
            self.assertNotIn('synthetic-secret', (root / 'wire.jsonl').read_text())
            self.assertNotIn('synthetic-source', (root / 'wire.jsonl').read_text())

    def test_files_are_scoped_by_session_and_mount_and_preserve_lines(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            services = ClientServices('unused', 'unused', {'/workspace': root / 'workspace'})
            (root / 'workspace').mkdir()
            services.sessions.add('s')
            params = {'sessionId': 's', 'path': '/workspace/中文 路径/a.txt', 'content': 'one\n二\nthree\n'}
            try:
                self.assertEqual(services('fs/write_text_file', params), {})
                self.assertEqual(services('fs/read_text_file', {**params, 'line': 2, 'limit': 1}), {'content': '二\n'})
                (root / 'private').write_text('private')
                (root / 'workspace/escape').symlink_to(root / 'private')
                for path in ['/workspace/escape', '/workspace/../private', '/other', 'relative']:
                    with self.assertRaises((ValueError, PermissionError)):
                        services('fs/read_text_file', {**params, 'path': path})
                with self.assertRaises(ValueError):
                    services('fs/read_text_file', {**params, 'sessionId': 'unknown'})
            finally:
                services.close()


if __name__ == '__main__':
    unittest.main()
