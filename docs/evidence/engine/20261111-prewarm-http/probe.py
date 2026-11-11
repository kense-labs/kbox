#!/usr/bin/env python3
import argparse
import hashlib
import http.server
import json
import os
from pathlib import Path
import subprocess
import threading
import uuid


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--kbox', type=Path, action='append', required=True)
    parser.add_argument('--image-store', type=Path, required=True)
    parser.add_argument('--image', required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--count', type=int, default=64)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)

    for index, cli in enumerate(args.kbox):
        root = args.output / f'case-{index}'
        home = root / 'home'
        home.mkdir(parents=True, mode=0o700)
        (home / 'images').symlink_to(args.image_store.resolve(), target_is_directory=True)
        env = dict(os.environ, K_HOME=str(home))
        nonce = uuid.uuid4().hex.encode()
        events = []

        def call(*argv, check=True):
            result = subprocess.run([cli, *argv], env=env, capture_output=True, timeout=300)
            number = len(events)
            (root / f'{number}.stdout').write_bytes(result.stdout)
            (root / f'{number}.stderr').write_bytes(result.stderr)
            events.append(dict(command=[str(cli), *argv], exitCode=result.returncode))
            if check:
                result.check_returncode()
            return result.stdout

        class Handler(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                self.send_response(200)
                self.end_headers()
                self.wfile.write(nonce)

            def log_message(self, *_):
                pass

        try:
            call('run', '-d', '--name', 'http-probe', '--memory', '512m',
                 '--entrypoint', '/bin/sleep', args.image, '--', 'infinity')
            with http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler) as server:
                threading.Thread(target=server.serve_forever, daemon=True).start()
                try:
                    program = f'''import hashlib, json, urllib.request
expected = {nonce!r}
url = 'http://127.0.0.1:{server.server_port}'
successes, failures = 0, []
for index in range({args.count}):
    try:
        with urllib.request.urlopen(url, timeout=2) as response:
            body = response.read()
            if response.status == 200 and body == expected:
                successes += 1
            else:
                failures.append(dict(index=index, status=response.status, length=len(body), sha256=hashlib.sha256(body).hexdigest()))
    except Exception as error:
        failures.append(dict(index=index, error=type(error).__name__))
print(json.dumps(dict(requests={args.count}, exactBodies=successes, failures=failures)))'''
                    result = json.loads(call('exec', '--timeout', '220', 'http-probe', '--', 'python3', '-c', program))
                finally:
                    server.shutdown()
            lib = cli.parent.parent / 'lib/libkrun.dylib'
            result.update(kboxSha256=hashlib.sha256(cli.read_bytes()).hexdigest(),
                          libkrunSha256=hashlib.sha256(lib.read_bytes()).hexdigest())
            (root / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
            print(json.dumps(dict(case=index, exactBodies=result['exactBodies'], failures=len(result['failures']))), flush=True)
        finally:
            for record in json.loads(call('ps', '-a', '--format', 'json')):
                call('rm', '-f', record['id'])
            assert json.loads(call('ps', '-a', '--format', 'json')) == []
            (root / 'commands.json').write_text(json.dumps(events, indent=2) + '\n')


if __name__ == '__main__':
    main()
