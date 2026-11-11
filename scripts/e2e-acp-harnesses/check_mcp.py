#!/usr/bin/env python3
"""Validate ACP session/new MCP stdio handoff with a real model and guest MCP process."""

import json
import os
from pathlib import Path
import secrets
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'e2e-dsh-acp/lib'))
from profiles import Profile


def main():
    evidence = Path(os.environ['E2E_OUT']).resolve()
    evidence.mkdir(parents=True, exist_ok=False)
    workspace, external, data = [evidence / name for name in ('workspace', 'external', 'data')]
    for path in (workspace, external, data):
        path.mkdir()
    profile = Profile()
    client = None
    report = {'harness': profile.name, 'status': 'FAIL'}
    try:
        client = profile.client(['-v', f'{workspace}:/workspace', '-v', f'{external}:/external',
                                 '-v', f'{data}:/data'], evidence, 'mcp')
        client.initialize()
        client.allow_permissions = True
        challenge = 'mcp-' + secrets.token_hex(16)
        result = client.request('session/new', {'cwd': '/workspace', 'mcpServers': [{
            'name': 'acceptance', 'command': '/usr/local/bin/node', 'args': ['/acceptance/mcp.mjs'],
            'env': [{'name': 'ACCEPTANCE_CHALLENGE', 'value': challenge}]}]}, 180)
        session = result['sessionId']
        client.services.sessions.add(session)
        client.configure(session)
        turn = client.prompt(session, '调用本会话提供的 MCP acceptance_challenge 工具，'
                             '回复工具返回的完整校验值。不要读文件或环境变量，也不要用命令模拟该工具。', timeout=180)
        assert challenge in turn['reply'], 'model did not return the MCP tool result'
        assert json.loads((external / 'mcp-called.json').read_text()) == {'challenge': challenge}
        assert turn['tool_calls'] > 0
        profile.close_session(client, session)
        report.update(status='PASS', tool_calls=turn['tool_calls'], elapsed_s=turn['elapsed_s'])
    except Exception as error:
        report['error'] = str(error)
        raise
    finally:
        if client:
            client.close(strict=False)
        boxes = subprocess.run([os.environ['KBOX'], 'ps', '--all', '--format', 'json'],
                               capture_output=True, text=True, check=True)
        remaining = [b for b in json.loads(boxes.stdout) if b['raw_status'] in ('running', 'paused')]
        report['remaining_boxes'] = remaining
        if remaining:
            report['status'] = 'FAIL'
        (evidence / 'results.json').write_text(json.dumps(report, indent=2) + '\n')
        for path in evidence.glob('*.log'):
            text = path.read_text(errors='replace')
            for secret in profile.redactions:
                text = text.replace(secret, '<redacted>')
            path.write_text(text)
    assert report['status'] == 'PASS', report
    print(json.dumps(report))


if __name__ == '__main__':
    main()
