#!/usr/bin/env python3
"""Run the shared real-model task gate against one ACP harness in isolated kbox state."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--harness', required=True, choices=['aster', 'deepseek', 'codex'])
    parser.add_argument('--image', required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--image-store', type=Path, required=True)
    parser.add_argument('--provider-config', type=Path)
    parser.add_argument('--model')
    args = parser.parse_args()
    if args.harness == 'codex' and not args.model:
        parser.error('--model is required for Codex')
    root = Path(__file__).resolve().parents[2]
    evidence = args.out.resolve()
    evidence.mkdir(parents=True, exist_ok=False)
    home = evidence / 'k-home'
    home.mkdir()
    (home / 'images').symlink_to(args.image_store.resolve(), target_is_directory=True)
    binary = Path(os.environ.get('KBOX', str(root / 'target/release/kbox'))).resolve()
    env = {**os.environ, 'K_HOME': str(home), 'KBOX': str(binary), 'IMAGE': args.image,
           'E2E_OUT': str(evidence), 'RESULTS_JSONL': str(evidence / 'results.jsonl'),
           'HARNESS_PROFILE': args.harness}
    if args.provider_config:
        env['ACP_PROVIDER_CONFIG'] = str(args.provider_config.resolve())
    if args.model:
        env['ACP_CODEX_MODEL' if args.harness == 'codex' else 'DSH_MODEL'] = args.model
    inspected = subprocess.run([str(binary), 'image-inspect', args.image], env=env,
                               capture_output=True, text=True, check=True)
    manifest = json.loads(inspected.stdout)[0]
    (evidence / 'image.json').write_text(json.dumps(manifest, indent=2) + '\n')
    began = time.monotonic()
    with (evidence / 'run.log').open('w') as log:
        result = subprocess.run([sys.executable, str(root / 'scripts/e2e-dsh-acp/cases/p4_tasks.py')],
                                env=env, stdout=log, stderr=subprocess.STDOUT)
    rows_path = evidence / 'results.jsonl'
    rows = [json.loads(line) for line in rows_path.read_text().splitlines()] if rows_path.exists() else []
    if not rows:
        rows = [{'id': f'P4-{i}', 'status': 'BLOCKED', 'message': 'runner exited before executing cases',
                 'detail': {'exit_code': result.returncode}} for i in range(1, 10)]
        rows_path.write_text(''.join(json.dumps(row) + '\n' for row in rows))
    summary = {s.lower(): sum(row['status'] == s for row in rows) for s in ('PASS', 'FAIL', 'BLOCKED', 'SKIP')}
    expected = {f'P4-{i}' for i in range(1, 10)}
    after = subprocess.run([str(binary), 'image-inspect', args.image], env=env,
                           capture_output=True, text=True, check=True)
    unchanged = json.loads(after.stdout)[0]['Digest'] == manifest['Digest']
    accepted = (result.returncode == 0 and len(rows) == 9 and {r['id'] for r in rows} == expected
                and summary['pass'] == 9 and unchanged)
    payload = {'harness': args.harness, 'image': args.image, 'digest': manifest['Digest'],
               'image_unchanged': unchanged, 'accepted': accepted, 'summary': summary,
               'elapsed_s': round(time.monotonic() - began, 3), 'cases': rows}
    (evidence / 'results.json').write_text(json.dumps(payload, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({k: payload[k] for k in ('harness', 'accepted', 'summary', 'elapsed_s')}))
    return 0 if accepted else 1


if __name__ == '__main__':
    raise SystemExit(main())
