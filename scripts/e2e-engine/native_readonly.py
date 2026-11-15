#!/usr/bin/env python3
"""Qualify macOS host write denial through the real Box VM and retained route."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import uuid


GUEST_PROBE = r'''
import ctypes, errno, json, os, pathlib, subprocess, sys
ro = pathlib.Path('/readonly')
rw = pathlib.Path('/writable')
assert (ro/'input').read_text() == sys.argv[1]
libc = ctypes.CDLL(None, use_errno=True)
libc.mount.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p, ctypes.c_ulong, ctypes.c_void_p]
assert libc.mount(None, b'/readonly', None, 32, None) == 0, ctypes.get_errno()
pathlib.Path('/alternate').mkdir(exist_ok=True)
assert libc.mount(b'vol0', b'/alternate', b'virtiofs', 0, None) == 0, ctypes.get_errno()
denials = {}
def deny(name, operation):
    try:
        operation()
    except OSError as error:
        assert error.errno in (errno.EPERM, errno.EACCES), (name, error)
        denials[name] = error.errno
    else:
        raise AssertionError(name + ' unexpectedly wrote the host source')
deny('create', lambda: (ro/'new').write_text('denied'))
deny('overwrite', lambda: (ro/'input').write_text('denied'))
deny('append', lambda: open(ro/'input', 'ab'))
deny('truncate', lambda: os.truncate(ro/'input', 0))
deny('unlink', lambda: (ro/'input').unlink())
deny('mkdir', lambda: (ro/'newdir').mkdir())
deny('rmdir', lambda: (ro/'empty').rmdir())
deny('rename', lambda: (ro/'input').rename(ro/'renamed'))
deny('replace', lambda: (ro/'replacement').replace(ro/'input'))
deny('symlink-create', lambda: (ro/'link').symlink_to('/writable/replacement'))
deny('symlink-follow', lambda: (rw/'inward-link').write_text('denied'))
deny('directory-symlink-follow', lambda: (rw/'directory-link/input').write_text('denied'))
deny('hardlink', lambda: os.link(ro/'input', ro/'hardlink'))
deny('fresh-writable-mount', lambda: pathlib.Path('/alternate/input').write_text('denied'))
child = subprocess.run([sys.executable, '-c', "import errno,pathlib;\ntry: pathlib.Path('/readonly/input').write_text('child')\nexcept OSError as e: assert e.errno in (errno.EPERM,errno.EACCES); print(e.errno)\nelse: raise AssertionError('child bypassed host readonly')"], capture_output=True, text=True)
assert child.returncode == 0, (child.returncode, child.stdout, child.stderr)
(rw/'output').write_text('writable-control')
if (rw/'output-hardlink').exists():
    (rw/'output-hardlink').unlink()
os.link(rw/'output', rw/'output-hardlink')
assert (rw/'output-hardlink').read_text() == 'writable-control'
subprocess.run([sys.executable, '-c', "from pathlib import Path; Path('/writable/child').write_text('guest-child-control')"], check=True)
assert (ro/'input').read_text() == sys.argv[1]
print(json.dumps(dict(remountWritable=True, freshWritableMount=True, hostDenials=denials, childDenial=int(child.stdout), writableControl=True)))
'''


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--kbox', type=Path, required=True)
    parser.add_argument('--image-store', type=Path, required=True)
    parser.add_argument('--image', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    assert platform.system() == 'Darwin' and platform.machine() == 'arm64'
    assert '@sha256:' in args.image
    binary = args.kbox.resolve()
    root = args.output.resolve() / uuid.uuid4().hex
    home = root / 'home'
    home.mkdir(parents=True, mode=0o700)
    (home/'images').symlink_to(args.image_store.resolve(), target_is_directory=True)
    ro = root/'只读 source with spaces'
    rw = root/'可写 source with spaces'
    (ro/'empty').mkdir(parents=True)
    rw.mkdir()
    nonce = uuid.uuid4().hex
    (ro/'input').write_text(nonce)
    (ro/'replacement').write_text('readonly-replacement')
    (rw/'replacement').write_text('replacement')
    (rw/'inward-link').symlink_to('/readonly/input')
    (rw/'directory-link').symlink_to('/readonly', target_is_directory=True)
    identity = (ro.stat().st_dev, ro.stat().st_ino, ro.stat().st_mode)
    environment = dict(os.environ, K_HOME=str(home))
    commands = []

    def cli(*arguments, check=True):
        command = [str(binary)] + list(map(str, arguments))
        result = subprocess.run(command, env=environment, capture_output=True, timeout=180)
        index = len(commands) + 1
        (root/('%03d.stdout' % index)).write_bytes(result.stdout)
        (root/('%03d.stderr' % index)).write_bytes(result.stderr)
        commands.append(dict(command=command, exitCode=result.returncode))
        (root/'commands.json').write_text(json.dumps(commands, indent=2)+'\n')
        if check:
            assert result.returncode == 0, ('command failed; logs retained', index, result.stderr.decode(errors='replace'))
        return result

    try:
        cli('run', '-d', '--name', 'readonly', '--memory', '512', '--privileged',
            '--security-opt', 'seccomp=unconfined', '-v', str(ro)+':/readonly:ro',
            '-v', str(rw)+':/writable:rw', args.image, '--', '/bin/sleep', '2147483647')
        before = json.loads(cli('inspect', 'readonly').stdout)[0]
        assert before['managed_execution']['runtime_route'] == 'box_vm', before['managed_execution']
        first = json.loads(cli('exec', 'readonly', '--', 'python3', '-c', GUEST_PROBE, nonce).stdout)
        assert (ro/'input').read_text() == nonce
        assert (rw/'output').read_text() == 'writable-control'
        assert (rw/'child').read_text() == 'guest-child-control'
        assert (rw/'output-hardlink').stat().st_ino == (rw/'output').stat().st_ino
        updated = nonce+'-host-update'
        (ro/'input').write_text(updated)
        assert cli('exec', 'readonly', '--', 'cat', '/readonly/input').stdout.decode().strip() == updated
        cli('restart', 'readonly')
        after = json.loads(cli('inspect', 'readonly').stdout)[0]
        assert after['id'] == before['id']
        assert after['managed_execution']['runtime_route'] == before['managed_execution']['runtime_route']
        second = json.loads(cli('exec', 'readonly', '--', 'python3', '-c', GUEST_PROBE, updated).stdout)
        assert (ro.stat().st_dev, ro.stat().st_ino, ro.stat().st_mode) == identity
        assert (ro/'input').read_text() == updated
        assert sorted(path.name for path in ro.iterdir()) == ['empty', 'input', 'replacement']
        overlap = cli('run', '-d', '--name', 'overlap', '-v', str(ro)+':/readonly:ro',
                      '-v', str(root)+':/parent:rw', args.image, '--', '/bin/sleep', '60', check=False)
        assert overlap.returncode != 0
        assert b'read-only' in overlap.stderr
        result = dict(status='PASS', platform=platform.platform(), image=args.image,
                      kboxSha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                      scriptSha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                      firstProbe=first, restartProbe=second,
                      firstRecord=before, restartedRecord=after, liveHostUpdates=True,
                      sourceIdentityAndPermissionsPreserved=True, overlapRefused=True)
        (root/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    except BaseException as error:
        (root/'failure.json').write_text(json.dumps(dict(type=type(error).__name__, message=str(error)), indent=2)+'\n')
        raise
    finally:
        records = json.loads(cli('ps', '-a', '--format', 'json').stdout)
        (root/'cleanup-before.json').write_text(json.dumps(records, indent=2)+'\n')
        for record in records:
            box = home/'boxes'/record['id']
            retained = root/'retained-box-logs'/record['id']
            retained.mkdir(parents=True)
            for pattern in ('*.log', 'logs/*.log'):
                for path in box.glob(pattern):
                    shutil.copy2(path, retained/path.name)
            cli('inspect', record['id'])
            cli('rm', '-f', record['id'])
        final = json.loads(cli('ps', '-a', '--format', 'json').stdout)
        assert final == [], final
        (root/'cleanup-after.json').write_text('[]\n')
        assert (ro.stat().st_dev, ro.stat().st_ino, ro.stat().st_mode) == identity
        print(root, flush=True)


if __name__ == '__main__':
    main()
