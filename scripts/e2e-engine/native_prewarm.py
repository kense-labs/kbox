#!/usr/bin/env python3
"""Native canonical prewarm handoff; owns its home and freezes test artifacts."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import uuid


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--kbox", type=Path, required=True)
    parser.add_argument("--test-binary", type=Path, required=True)
    parser.add_argument("--image-store", type=Path, required=True)
    parser.add_argument("--image", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    binary = args.kbox.resolve()
    root = args.output.resolve() / uuid.uuid4().hex
    home = root / "k-home"
    (home / "bin").mkdir(parents=True, mode=0o700)
    home.chmod(0o700)
    (home / "lib").mkdir(mode=0o700)
    for name in ("kbox-shim", "kbox-guest-init"):
        shutil.copy2(binary.parent / name, home / "bin" / name)
    for file in (binary.parent.parent / "lib").iterdir():
        (home / "lib" / file.name).symlink_to(file.resolve())
    (home / "images").symlink_to(args.image_store.resolve(), target_is_directory=True)
    (home / ".krun-prewarm-native").write_text("owned-prewarm-native-test\n")
    test = root / "native-client"
    shutil.copy2(args.test_binary.resolve(), test)
    environment = dict(os.environ, K_HOME=str(home), K_BOX_TEST_STREAM_IMAGE=args.image)
    command = [str(test), "--ignored", "--exact", "kbox_contract_driver::prewarm_native::native_prewarm_handoff_preserves_vm_identity_and_retires_dirty_resources", "--nocapture"]
    hashes = {"testSha256": hashlib.sha256(test.read_bytes()).hexdigest(), "kboxSha256": hashlib.sha256(binary.read_bytes()).hexdigest()}
    (root / "command.json").write_text(json.dumps(dict(command=command, image=args.image, **hashes), indent=2))
    try:
        with (root / "native.stdout").open("wb") as out, (root / "native.stderr").open("wb") as err:
            completed = subprocess.run(command, env=environment, stdout=out, stderr=err, timeout=600)
        assert completed.returncode == 0, "native test failed; logs retained"
        result = json.loads((root / "result.json").read_text())
        assert result["status"] == "PASS" and result["zeroPrivateExecutions"]
        volumes = subprocess.run([str(binary), "volume", "ls", "-q"], env=environment, capture_output=True, check=True, timeout=60)
        assert not volumes.stdout.strip()
        result.update(image=args.image, zeroPrivateVolumes=True, **hashes)
        (root / "result.json").write_text(json.dumps(result, indent=2) + "\n")
        print(root, flush=True)
    except BaseException as error:
        (root / "failure.json").write_text(json.dumps(dict(type=type(error).__name__, message=str(error))))
        raise
    finally:
        listed = subprocess.run([str(binary), "ps", "-a", "--format", "json"], env=environment, capture_output=True, check=True, timeout=60)
        records = json.loads(listed.stdout)
        (root / "cleanup-before.json").write_text(json.dumps(records, indent=2))
        for record in records:
            subprocess.run([str(binary), "rm", "-f", record["id"]], env=environment, capture_output=True, check=True, timeout=120)
        listed = subprocess.run([str(binary), "ps", "-a", "--format", "json"], env=environment, capture_output=True, check=True, timeout=60)
        assert json.loads(listed.stdout) == []
        (root / "cleanup-after.json").write_text("[]\n")


if __name__ == "__main__":
    main()
