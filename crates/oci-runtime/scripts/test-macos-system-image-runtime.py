#!/usr/bin/env python3
"""Exercise the real macOS provenance parser before the heavyweight image builder."""
import copy
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parent
ASSETS = json.loads((SCRIPTS / '../crates/kbox/runtime/runtime-assets.json').read_text())

class RuntimeMetadata(unittest.TestCase):
    def invoke(self, data):
        with tempfile.TemporaryDirectory() as work:
            root = Path(work)
            scripts = root / 'scripts'
            (scripts / 'lib').mkdir(parents=True)
            shutil.copy2(SCRIPTS / 'build-macos-system-image.sh', scripts)
            (scripts / 'lib/build-system-image.sh').write_text('kbuild_system_image() { printf "%s\\n" "$K_SYSTEM_IMAGE_RUNTIME_JSON"; }\n')
            manifest = root / 'crates/kbox/runtime/runtime-assets.json'
            manifest.parent.mkdir(parents=True)
            manifest.write_text(json.dumps(data))
            return subprocess.run(['bash', str(scripts / 'build-macos-system-image.sh')], capture_output=True, text=True)

    def test_uses_the_actual_checked_in_runtime_identity(self):
        result = self.invoke(ASSETS)
        self.assertEqual(result.returncode, 0, result.stderr)
        runtime = json.loads(result.stdout)
        bundle = next(b for b in ASSETS['bundles'] if b['target_os'] == 'macos' and b['target_arch'] == 'aarch64')
        self.assertEqual(runtime['archive_sha256'], bundle['archive_sha256'])
        for key, role in [('libkrun_sha256', 'library'), ('firmware_sha256', 'firmware')]:
            self.assertEqual(runtime[key], next(f['sha256'] for f in bundle['files'] if f['role'] == role))
        self.assertEqual(runtime['kernel_bundle_sha256'], bundle['kernel']['sha256'])
        self.assertEqual(runtime['kernel_bundle_size'], bundle['kernel']['size'])
        self.assertEqual(int(runtime['kernel_guest_load_address'], 16), bundle['kernel']['guest_load_address'])

    def test_schema_target_role_and_address_drift_refuse_before_build(self):
        for change in ['schema', 'missing', 'duplicate', 'role', 'address']:
            with self.subTest(change=change):
                data = copy.deepcopy(ASSETS)
                mac = next(b for b in data['bundles'] if b['platform'] == 'macos-aarch64')
                if change == 'schema': data['schema_version'] = 'unknown'
                elif change == 'missing': data['bundles'].remove(mac)
                elif change == 'duplicate': data['bundles'].append(mac)
                elif change == 'role': mac['files'].append(mac['files'][0])
                else: mac['kernel']['guest_load_address'] += 1
                result = self.invoke(data)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(result.stdout, '')

if __name__ == '__main__':
    unittest.main()
