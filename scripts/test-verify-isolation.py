#!/usr/bin/env python3
"""Check the verification script's home confinement and tool failure boundary."""
import os
from pathlib import Path
import stat
import shutil
import socket
import subprocess
import tempfile
import unittest

REPO = Path(__file__).resolve().parent.parent
SCRIPT = REPO / "scripts/verify-commit.sh"


class VerifyIsolation(unittest.TestCase):
    def run_verifier(self, inherited, failure=None, long_base=False):
        with tempfile.TemporaryDirectory(prefix="kbox-verify-test-") as directory:
            root = Path(directory)
            tools = root / "tools"
            tools.mkdir()
            temporary = root / "temporary 空格"
            temporary.mkdir(mode=0o700)
            if long_base:
                temporary = temporary / ("long-" + "x" * 110)
                temporary.mkdir(mode=0o700)
            previous = root / "caller-home"
            previous.mkdir()
            (previous / "canary").write_bytes(b"caller-data")
            observed = root / "observed.tsv"
            for name in ("git", "cargo", "python3"):
                tool = tools / name
                tool.write_text("#!/bin/sh\n"
                    "name=${0##*/}\n"
                    "printf '%s\\t%s\\t%s\\n' \"$name\" \"${K_HOME-}\" \"${TMPDIR-}\" >> \"$VERIFY_ISOLATION_OBSERVED\"\n"
                    "[ -n \"${K_HOME-}\" ] || exit 96\n"
                    "mkdir -p \"$K_HOME\"\n"
                    "printf '%s\\n' \"$name\" > \"$K_HOME/probe-$name\"\n"
                    "if [ \"$name\" = cargo ] && [ \"${1-}\" = \"${VERIFY_ISOLATION_FAIL-}\" ]; then exit 42; fi\n"
                    "exit 0\n")
                tool.chmod(0o700)
            env = dict(os.environ, PATH=str(tools) + ":/usr/bin:/bin:/usr/sbin:/sbin",
                       TMPDIR=str(temporary), VERIFY_ISOLATION_OBSERVED=str(observed),
                       VERIFY_ISOLATION_FAIL=failure or "")
            env.pop("K_HOME", None)
            if inherited:
                env["K_HOME"] = str(previous)
            result = subprocess.run(["/bin/bash", str(SCRIPT)], cwd=REPO, env=env,
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            self.assertEqual(result.returncode, 42 if failure else 0, result.stdout + result.stderr)
            rows = [line.split("\t") for line in observed.read_text().splitlines()]
            self.assertTrue(rows)
            homes = {Path(home) for _, home, _ in rows}
            self.assertEqual(len(homes), 1)
            home = homes.pop()
            self.assertNotEqual(home, previous, "inherited caller home must stay outside verification")
            self.assertTrue(home.is_relative_to(temporary), str(home))
            self.assertTrue(home.is_dir(), "the owned home must remain available for audit")
            self.assertEqual(stat.S_IMODE(home.stat().st_mode), 0o700)
            self.assertEqual(stat.S_IMODE(home.parent.stat().st_mode), 0o700)
            self.assertEqual((home.parent / ".kbox-verify-owner").read_text(), "kbox-verify-private-home\n")
            self.assertIn(str(home), result.stdout, "the script must report its exact owned home")
            self.assertEqual([p.name for p in previous.iterdir()], ["canary"])
            self.assertEqual((previous / "canary").read_bytes(), b"caller-data")
            temporary_roots = {Path(value) for _, _, value in rows}
            self.assertEqual(len(temporary_roots), 1)
            child_temporary = temporary_roots.pop()
            self.assertNotEqual(child_temporary, temporary, "long caller TMPDIR must not control IPC path lengths")
            self.assertFalse(child_temporary.is_symlink())
            self.assertEqual(stat.S_IMODE(child_temporary.stat().st_mode), 0o700)
            self.assertEqual(Path((child_temporary / ".kbox-verify-owner").read_text().strip()), home.parent)
            try:
                with tempfile.TemporaryDirectory(dir=child_temporary) as ipc:
                    endpoint = Path(ipc) / "filesystem-mkdir-keyed-retry.sock"
                    with socket.socket(socket.AF_UNIX) as listener:
                        listener.bind(str(endpoint))
            finally:
                # The child-created root is bound to this exact retained verification home.
                shutil.rmtree(child_temporary)
            if not failure:
                self.assertIn("verify-commit: OK", result.stdout)
                self.assertEqual({name for name, _, _ in rows}, {"git", "cargo", "python3"})
            else:
                self.assertNotIn("verify-commit: OK", result.stdout)

    def test_inherited_home_is_replaced_only_in_the_verification_child(self):
        self.run_verifier(inherited=True)

    def test_unset_home_gets_a_private_owned_directory(self):
        self.run_verifier(inherited=False)

    def test_tool_failure_is_preserved_with_its_owned_home(self):
        self.run_verifier(inherited=True, failure="check")

    def test_long_temporary_base_still_provides_a_usable_native_socket_directory(self):
        self.run_verifier(inherited=True, long_base=True)


if __name__ == "__main__":
    unittest.main()
