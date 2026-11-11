#!/usr/bin/env python3
"""Real-VM regression check for the ACP client's terminal services."""

import argparse
import json
from pathlib import Path
import secrets
import subprocess
import sys
import time

from client_services import ClientServices


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--kbox', required=True)
    parser.add_argument('--box', required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    assert not args.out.exists(), 'preserve previous evidence'
    services = ClientServices(args.kbox, args.box, {})
    services.sessions.add('s')

    def call(method, **params):
        return services('terminal/' + method, {'sessionId': 's', **params})

    try:
        terminal = call('create', command='node', args=['-e', "process.stdout.write('前缀🪐tail')"],
                        cwd='/tmp', outputByteLimit=8)['terminalId']
        assert call('wait_for_exit', terminalId=terminal).result(timeout=20) == {'exitCode': 0}
        output = call('output', terminalId=terminal)
        assert output['output'] == '🪐tail' and output['truncated'], output
        call('release', terminalId=terminal).result(timeout=10)
        marker = '/tmp/kbox-client-terminal-' + secrets.token_hex(8)
        child = f"setTimeout(()=>require('fs').writeFileSync({json.dumps(marker)},'late'),5000)"
        script = (f"require('child_process').spawn(process.execPath,['-e',{json.dumps(child)}],"
                  "{stdio:'ignore'});console.log('started');setTimeout(()=>{},30000)")
        terminal = call('create', command='node', args=['-e', script], cwd='/tmp')['terminalId']
        deadline = time.monotonic() + 20
        while 'started' not in call('output', terminalId=terminal)['output']:
            assert time.monotonic() < deadline, 'running terminal did not stream output'
            time.sleep(.1)
        call('kill', terminalId=terminal).result(timeout=20)
        status = call('wait_for_exit', terminalId=terminal).result(timeout=10)
        assert status.get('exitCode') != 0, status
        time.sleep(6)
        subprocess.run([args.kbox, 'exec', '--timeout', '5', args.box, '--', 'test', '!', '-e', marker],
                       check=True, capture_output=True, timeout=15)
        call('release', terminalId=terminal).result(timeout=10)
        report = {'status': 'PASS', 'box': args.box, 'utf8_tail': output, 'kill_exit': status,
                  'late_write_observation_s': 6, 'calls': services.calls}
        args.out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    except Exception as error:
        args.out.write_text(json.dumps({'status': 'FAIL', 'error': str(error)}, indent=2) + '\n')
        raise
    finally:
        services.close()


if __name__ == '__main__':
    main()
