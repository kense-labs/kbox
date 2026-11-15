#!/usr/bin/env python3
"""Hermetic macOS archive-installer checks; fixture binaries never launch a VM."""
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import stat
import struct
import subprocess
import tempfile
import unittest

REPO = Path(__file__).resolve().parent.parent


def checksums(package):
    entries = sorted(p for p in package.rglob('*') if p.is_file() and p.name != 'SHA256SUMS')
    (package / 'SHA256SUMS').write_text(''.join(
        hashlib.sha256(p.read_bytes()).hexdigest() + '  ' + p.relative_to(package).as_posix() + '\n'
        for p in entries))


def snapshot(root):
    return {str(p.relative_to(root)): (stat.S_IMODE(p.lstat().st_mode),
            ('link', os.readlink(p)) if p.is_symlink() else ('file', p.read_bytes()))
            for p in root.rglob('*') if p.is_file() or p.is_symlink()}


@unittest.skipUnless(platform.system() == 'Darwin' and platform.machine() == 'arm64',
                     'macOS arm64 archive requires native Mach-O tools')
class InstallerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.scratch = tempfile.TemporaryDirectory(prefix='kbox-install-fixture-')
        cls.fixture = Path(cls.scratch.name) / 'package'
        cls.fixture.mkdir()
        (cls.fixture / 'bin').mkdir()
        (cls.fixture / 'lib').mkdir()
        cls.compile_cli(cls.fixture / 'bin/kbox', 0)
        shutil.copy2(cls.fixture / 'bin/kbox', cls.fixture / 'bin/kbox-shim')
        subprocess.run(['codesign', '--force', '--sign', '-', str(cls.fixture / 'bin/kbox-shim')],
                       check=True, capture_output=True)
        # A minimal static aarch64 ELF header tests admission, not guest execution.
        header = b'\x7fELF\x02\x01\x01' + bytes(9)
        header += struct.pack('<HHIQQQIHHHHHH', 2, 183, 1, 0x400000, 64, 0, 0, 64, 56, 1, 64, 0, 0)
        header += struct.pack('<IIQQQQQQ', 1, 5, 0, 0x400000, 0x400000, 120, 120, 4096)
        (cls.fixture / 'bin/kbox-guest-init').write_bytes(header)
        for name in ['libkrun.1.17.0.dylib', 'libkrunfw.5.dylib']:
            subprocess.run(['cc', '-dynamiclib', '-x', 'c', '-', '-o', str(cls.fixture / 'lib' / name)],
                           input='int fixture(void) { return 0; }', text=True, check=True, capture_output=True)
        for name, target in [('libkrun.1.dylib', 'libkrun.1.17.0.dylib'),
                             ('libkrun.dylib', 'libkrun.1.17.0.dylib'),
                             ('libkrunfw.dylib', 'libkrunfw.5.dylib')]:
            (cls.fixture / 'lib' / name).symlink_to(target)
        shutil.copy2(REPO / 'scripts/install-package.sh', cls.fixture / 'install.sh')
        (cls.fixture / 'VERSION').write_text('0.0.1\n')
        (cls.fixture / 'build-info.json').write_text(json.dumps({'fixture': True}))
        checksums(cls.fixture)

    @staticmethod
    def compile_cli(path, code):
        subprocess.run(['cc', '-x', 'c', '-', '-o', str(path)], input=
                       '#include <stdio.h>\nint main(void) { puts("kbox version 0.0.1"); return ' + str(code) + '; }',
                       text=True, check=True, capture_output=True)

    @classmethod
    def tearDownClass(cls):
        cls.scratch.cleanup()

    def setUp(self):
        self.scratch_case = tempfile.TemporaryDirectory(prefix='kbox-install-case-')
        self.addCleanup(self.scratch_case.cleanup)
        self.root = Path(self.scratch_case.name)
        self.package = self.root / 'package'
        shutil.copytree(self.fixture, self.package, symlinks=True)
        self.prefix = self.root / '中文 prefix with spaces'
        (self.prefix / 'bin').mkdir(parents=True)
        (self.prefix / 'lib').mkdir()
        (self.prefix / 'images').mkdir()
        (self.prefix / 'bin/kbox').write_bytes(b'previous runtime')
        (self.prefix / 'bin/helper').write_bytes(b'caller-owned helper')
        (self.prefix / 'lib/old.dylib').write_bytes(b'previous library')
        (self.prefix / 'images/marker').write_bytes(b'workload data')
        self.before = snapshot(self.prefix)

    def install(self, env=None):
        environment = dict(os.environ, KBOX_PREFIX=str(self.prefix))
        environment.update(env or {})
        return subprocess.run(['sh', str(self.package / 'install.sh')],
                              env=environment, capture_output=True, text=True)

    def assert_preserved(self, result, reason):
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn(reason, result.stderr)
        self.assertEqual(snapshot(self.prefix), self.before, result.stderr)
        self.assertFalse((self.prefix / '.install-lock').exists())

    def test_success_preserves_workload_data_and_unrelated_runtime_files(self):
        result = self.install()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.prefix / 'images/marker').read_bytes(), b'workload data')
        self.assertEqual((self.prefix / 'bin/helper').stat().st_mode & 0o777,
                         self.before['bin/helper'][0])
        self.assertEqual((self.prefix / 'lib/old.dylib').read_bytes(), b'previous library')
        self.assertTrue((self.prefix / 'lib/libkrun.dylib').is_symlink())
        self.assertFalse((self.prefix / '.install-lock').exists())

    def test_checksum_corruption_and_missing_companion_preserve_previous_install(self):
        guest = self.package / 'bin/kbox-guest-init'
        original = guest.read_bytes()
        guest.write_bytes(original + b'corrupt')
        self.assert_preserved(self.install(), "checksum mismatch")
        guest.unlink()
        self.assert_preserved(self.install(), "missing companion")

    def test_versioned_state_directory_survives_install_and_rollback(self):
        state = self.prefix / 'boxes.json'
        state.mkdir(mode=0o700)
        records = state / 'records.v1.json'
        records.write_bytes(b'[{"caller_files":{"/fixture":{"device":1,"inode":2}}}]')
        identity = (state.stat().st_ino, records.stat().st_ino)
        data = records.read_bytes()
        result = self.install()
        self.assertEqual(result.returncode, 0, result.stderr)
        installed = snapshot(self.prefix)
        self.compile_cli(self.package / 'bin/kbox', 9)
        checksums(self.package)
        result = self.install()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('installed executable failed', result.stderr)
        self.assertEqual(snapshot(self.prefix), installed)
        self.assertEqual((state.stat().st_ino, records.stat().st_ino), identity)
        self.assertEqual(records.read_bytes(), data)

    def test_root_alias_prefix_is_refused_before_installation(self):
        for prefix in ['/', '//', '/tmp/..', '/tmp/../']:
            self.assert_preserved(self.install({'KBOX_PREFIX': prefix}), 'unsafe installation prefix')

    def test_wrong_guest_architecture_is_refused_even_with_matching_checksum(self):
        guest = self.package / 'bin/kbox-guest-init'
        data = bytearray(guest.read_bytes())
        data[18:20] = struct.pack('<H', 62)
        guest.write_bytes(data)
        checksums(self.package)
        self.assert_preserved(self.install(), "guest-init must be a static")

    def test_unexpected_companion_symlink_is_refused(self):
        source = self.package / 'bin/kbox'
        outside = self.root / 'outside'
        source.rename(outside)
        source.symlink_to(outside)
        checksums(self.package)
        self.assert_preserved(self.install(), "unexpected symlink")

    def test_installed_executable_failure_rolls_back(self):
        self.compile_cli(self.package / 'bin/kbox', 9)
        checksums(self.package)
        self.assert_preserved(self.install(), "installed executable failed")

    def test_failed_rename_never_deletes_previous_runtime(self):
        tools = self.root / 'tools'
        tools.mkdir()
        shim = tools / 'mv'
        for argument, pattern in [('$2', '*/backup/bin'), ('$1', '*/payload/lib')]:
            shim.write_text('#!/bin/sh\ncase "' + argument + '" in ' + pattern + ') echo "fixture rename failed" >&2; exit 9;; esac\nexec /bin/mv "$@"\n')
            shim.chmod(0o755)
            self.assert_preserved(self.install({'PATH': str(tools) + os.pathsep + os.environ['PATH']}), 'fixture rename failed')


if __name__ == '__main__':
    unittest.main(verbosity=2)
