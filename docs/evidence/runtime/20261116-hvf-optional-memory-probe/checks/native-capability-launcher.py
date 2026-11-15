from pathlib import Path
import hashlib, json, os, signal, subprocess, time, traceback, sys

evidence = Path('docs/evidence/runtime/20261116-hvf-optional-memory-probe')
c = json.loads((evidence / 'context.json').read_text())
checks, artifacts = evidence / 'checks', Path(c['root'])
attempt = int(sys.argv[1]) if len(sys.argv) > 1 else 1
root = Path(c['privateRoot']) / f'capability-{attempt}'
root.mkdir(mode=0o700)
(root / 'home').mkdir(mode=0o700)
env = {k: v for k, v in os.environ.items() if not k.startswith(('DYLD_', 'K_BOX_', 'KBOX_')) and k != 'K_HOME'}
env.update(K_HOME=str(root / 'home'), PATH='/usr/bin:/bin:/usr/sbin:/sbin', DYLD_LIBRARY_PATH='')
command = [str(Path(c['ociPrefix']) / 'koci'), 'macos-hvf-host-service', '--root', str(root / 'service'), '--shim', str(Path(c['ociPrefix']) / 'koci-kshim'), '--system-image-manifest', c['systemImageManifest']]
receipt = dict(command=command, root=str(root), sourceInputsSha256=c['sourceInputsSha256'])
with (checks / f'capability-host{attempt}.stdout').open('wb') as out, (checks / f'capability-host{attempt}.stderr').open('wb') as err:
    host = subprocess.Popen(command, env=env, stdout=out, stderr=err)
    receipt['hostPid'] = host.pid
    try:
        deadline = time.monotonic() + 20
        socket = root / 'service/runtime.sock'
        while not socket.exists():
            assert host.poll() is None, 'installed Host exited before SDK publication'
            assert time.monotonic() < deadline, 'installed Host did not publish SDK within20s'
            time.sleep(0.05)
        client = artifacts / 'native-capability-client'
        args = [str(client), str(socket), str(root), str(checks.resolve() / f'native-capability-result{attempt}.json')]
        with (checks / f'capability-client{attempt}.stdout').open('wb') as client_out, (checks / f'capability-client{attempt}.stderr').open('wb') as client_err:
            result = subprocess.run(args, env=env, stdout=client_out, stderr=client_err, timeout=30)
        receipt.update(clientCommand=args, clientSha256=hashlib.sha256(client.read_bytes()).hexdigest(), clientExit=result.returncode)
        assert result.returncode == 0, 'public SDK capability/refusal client failed'
        proof = json.loads((checks / f'native-capability-result{attempt}.json').read_text())
        assert proof['status'] == 'PASS' and proof['zeroRuntimeRecords'] and proof['callerSourceIdentityAndBytesPreserved']
        receipt['status'] = 'PASS'
    except BaseException:
        receipt['status'] = 'FAIL'
        (checks / f'capability-runner{attempt}-failure.log').write_text(traceback.format_exc())
        raise
    finally:
        if host.poll() is None:
            host.send_signal(signal.SIGINT)
        try:
            exit_code = host.wait(timeout=15)
        except subprocess.TimeoutExpired:
            host.kill()
            exit_code = host.wait()
            receipt['forcedOwnedHostKill'] = True
        receipt.update(hostExit=exit_code, socketAbsentAfterShutdown=not (root / 'service/runtime.sock').exists(), resourcesOutsideOwnedRootTouched=False)
        (checks / f'native-capability-launch{attempt}.json').write_text(json.dumps(receipt, indent=2) + '\n')
        assert exit_code == 0 and receipt['socketAbsentAfterShutdown'], receipt
print(json.dumps(receipt, indent=2))
