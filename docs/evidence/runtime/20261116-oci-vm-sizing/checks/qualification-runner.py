"""Owned diagnostic runner: installed archive only, checked counts and loader maps."""
from pathlib import Path
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
import traceback

context = Path(sys.argv[1]).resolve()
c = json.loads(context.read_text())
repo = context.parents[4]
root = Path(c['root'])
prefix = Path(c['installedPrefix'])
checks = Path(c.get('checks',str(context.parent / 'checks')))
assert root.is_absolute() and prefix.is_absolute() and checks.is_absolute() and Path(c['imageStore']).is_absolute() and Path(c['nativeClient']).is_absolute(), 'qualification paths must be absolute'
assert root.is_relative_to(repo / 'artifacts') and root.parent.is_dir()
root.mkdir()
work = root / 'work'
work.mkdir(exist_ok=True)
env = os.environ.copy()
for key in list(env):
    if key.startswith(('DYLD_', 'K_BOX_TEST_', 'K_BOX_OCI_')) or key in ('K_HOME', 'KBOX_PREFIX'):
        env.pop(key)
env.update(PATH=str(prefix / 'bin') + ':/usr/bin:/bin:/usr/sbin:/sbin',
           DYLD_LIBRARY_PATH='', DYLD_FALLBACK_LIBRARY_PATH='/usr/lib',
           PYTHONDONTWRITEBYTECODE='1')
expected = {name: hashlib.sha256((prefix / path).read_bytes()).hexdigest()
            for name, path in [('kbox-shim', 'bin/kbox-shim'),
                               ('libkrun.1.17.0.dylib', 'lib/libkrun.1.17.0.dylib'),
                               ('libkrunfw.5.dylib', 'lib/libkrunfw.5.dylib')]}
maps = {}


def capture_loaded(case):
    for home in list(root.glob('native-*/*/home')) + [root/'native-directory/home']:
        for path in home.rglob('*.log'):
            if path.is_file() and path.stat().st_size <= 1024*1024:
                data=path.read_bytes();digest=hashlib.sha256(data).hexdigest()
                directory=checks/'owned-box-log-snapshots';directory.mkdir(exist_ok=True)
                saved=directory/(path.parent.name+'-'+path.name+'-'+digest[:12])
                if not saved.exists():saved.write_bytes(data)
    listing = subprocess.check_output(['ps', '-ax', '-o', 'pid=,ppid=,command='], text=True)
    for line in listing.splitlines():
        if str(root) not in line or 'kbox-shim --config' not in line:
            continue
        pid, parent, command = line.strip().split(None, 2)
        key = case + '-' + pid
        if key in maps:
            continue
        observed = subprocess.run(['/usr/sbin/lsof', '-a', '-p', pid, '-d', 'txt', '-Fn'],
                                  capture_output=True, text=True)
        paths = [Path(line[1:]) for line in observed.stdout.splitlines() if line.startswith('n/')]
        native = {p.name: p for p in paths if p.name in expected}
        if not native:
            continue
        for name, path in native.items():
            assert hashlib.sha256(path.read_bytes()).hexdigest() == expected[name], path
            if name != 'kbox-shim':
                assert path.resolve().parent == (prefix / 'lib').resolve(), path
            else:
                assert path.resolve().is_relative_to(root) or path.resolve() == (prefix / 'bin/kbox-shim').resolve(), path
        if set(native) != set(expected):
            continue
        raw = checks / ('loader-' + key + '.log')
        raw.write_text(observed.stdout + observed.stderr)
        maps[key] = dict(case=case, pid=int(pid), parent=int(parent), command=command,
                         loaded={name: str(p) for name, p in native.items()}, log=raw.name)
        (checks / 'native-loaded-libraries.json').write_text(json.dumps(list(maps.values()), indent=2) + '\n')


def wait_process(case, command, stdout, stderr, environment):
    with stdout.open('wb') as out, stderr.open('wb') as err:
        process = subprocess.Popen(command, cwd=work, env=environment, stdout=out, stderr=err)
        while process.poll() is None:
            capture_loaded(case)
            time.sleep(0.3)
        return process.wait()


def check_one_test(path):
    found = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', path.read_text())
    assert found == [('1', '0', '0')], (path, found)
    return dict(passed=1, failed=0, ignored=0)


def directory_case(case, output):
    home = output / 'home'
    (home / 'bin').mkdir(parents=True, mode=0o700)
    home.chmod(0o700)
    (home / 'lib').mkdir()
    for name in ('kbox-shim', 'kbox-guest-init'):
        shutil.copy2(prefix / 'bin' / name, home / 'bin' / name)
    for path in (prefix / 'lib').iterdir():
        (home / 'lib' / path.name).symlink_to(path.resolve())
    (home / 'images').symlink_to(Path(c['imageStore']))
    (home / '.kbox-directory-native').write_text('owned-directory-native-test\n')
    environment = dict(env, K_HOME=str(home), K_BOX_TEST_STREAM_IMAGE=c['image'])
    command = [c['nativeClient'], '--ignored', '--exact',
               'kbox_contract_driver::directory::native::native_directory_grants_preserve_live_files_fences_and_cleanup', '--nocapture']
    try:
        status = wait_process(case, command, output / 'native.stdout', output / 'native.stderr', environment)
        assert status == 0, status
        count = check_one_test(output / 'native.stdout')
        result = json.loads((output / 'result.json').read_text())
        assert result['status'] == 'PASS' and result['zeroPrivateExecutions']
        assert result['staleGenerationRefused'] and result['movedSourceCleanup']
        return command, result, count
    finally:
        inventory = subprocess.run([str(prefix / 'bin/kbox'), 'ps', '-a', '--format', 'json'],
                                   env=environment, cwd=work, capture_output=True, check=True)
        records = json.loads(inventory.stdout)
        (output / 'cleanup-before.json').write_text(json.dumps(records, indent=2) + '\n')
        for record in records:
            subprocess.run([str(prefix / 'bin/kbox'), 'rm', '-f', record['id']],
                           env=environment, cwd=work, capture_output=True, check=True)
        after = subprocess.check_output([str(prefix / 'bin/kbox'), 'ps', '-a', '--format', 'json'],
                                        env=environment, cwd=work)
        assert json.loads(after) == []
        (output / 'cleanup-after.json').write_text('[]\n')


rows = []
for case, script, image_option in [('oci', 'native_smoke.py', '--base'),
                                  ('readonly', 'native_readonly.py', '--image'),
                                  ('primary', 'native_primary.py', '--image'),
                                  ('prewarm', 'native_prewarm.py', '--image'),
                                  ('directory', None, None)]:
    output = root / ('native-' + case)
    output.mkdir()
    started = time.monotonic()
    command = None
    print('START ' + case, flush=True)
    try:
        if script:
            command = [sys.executable, str(repo / 'scripts/e2e-engine' / script),
                       '--kbox', str(prefix / 'bin/kbox'), '--image-store', c['imageStore'],
                       image_option, c['image'], '--output', str(output)]
            if case == 'prewarm':
                command += ['--test-binary', c['nativeClient']]
            status = wait_process(case, command, checks / (case + '.stdout'),
                                  checks / (case + '.stderr'), env)
            assert status == 0, ('native driver failed', status)
            files = list(output.glob('*/result.json'))
            assert len(files) == 1, files
            result_file = files[0]
            result = json.loads(result_file.read_text())
            assert result['status'] == 'PASS'
            count = check_one_test(result_file.parent / 'native.stdout') if case == 'prewarm' else dict(completedNativeScenarios=1)
            if case == 'readonly':
                for field in ('firstProbe', 'restartProbe'):
                    probe = result[field]
                    assert probe['remountWritable'] and probe['freshWritableMount']
                    assert len(probe['hostDenials']) == 14
                    assert all(value in (1, 13) for value in probe['hostDenials'].values())
                    assert probe['childDenial'] in (1, 13) and probe['writableControl']
        else:
            command, result, count = directory_case(case, output)
            result_file = output / 'result.json'
        assert any(row['case'] == case for row in maps.values()), 'no complete native loader proof for ' + case
        row = dict(case=case, status='PASS', command=command, seconds=time.monotonic() - started,
                   result=str(result_file), actualCounts=count)
        shutil.copy2(result_file, checks / (case + '-result.json'))
    except BaseException as error:
        (checks / (case + '-runner-failure.log')).write_text(traceback.format_exc())
        row = dict(case=case, status='FAIL', command=command, seconds=time.monotonic() - started,
                   error=type(error).__name__ + ': ' + str(error))
    rows.append(row)
    (checks / 'native-results.json').write_text(json.dumps(rows, indent=2) + '\n')
    print(json.dumps(row), flush=True)
assert all(row['status'] == 'PASS' for row in rows), rows
print('All five native scenarios passed with installed companions and loader proof.', flush=True)
