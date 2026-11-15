from pathlib import Path
import hashlib, json, os, plistlib, re, runpy, subprocess

evidence = Path('docs/evidence/runtime/20261116-native-tool-diagnostics')
c = json.loads((evidence / 'context.json').read_text())
root, private = Path(c['root']), Path(c['privateRoot'])
checks = evidence / 'checks'
frozen = json.loads((root / 'source-frozen-1.json').read_text())
current = runpy.run_path(str(root / 'source_snapshot.py'))['snapshot']()
assert current['files'] == frozen['files']
assert current['sourceInputsSha256'] == frozen['sourceInputsSha256']
subprocess.run(['git', 'merge-base', '--is-ancestor', frozen['sourceCommit'], current['sourceCommit']], check=True)

# Derive all five completed homes from their actual result locations, including prewarm k-home.
rows = json.loads((checks / 'af-final1/native-results.json').read_text())
assert len(rows) == 5 and all(row['status'] == 'PASS' for row in rows)
homes = [Path(row['result']).parent / ('k-home' if row['case'] == 'prewarm' else 'home') for row in rows]
failed = json.loads((checks / 'box-native-files-attempt-1.json').read_text())
assert failed['exitCode'] == -6 and not failed['actualCounts'] and not failed['resultExists']
homes.append(Path(failed['root']) / 'home')
assert len(set(homes)) == 6
inventories = []
for home in homes:
    assert home.is_dir() and (home.is_relative_to(root) or home.is_relative_to(private))
    env = {k: v for k, v in os.environ.items() if not k.startswith(('DYLD_', 'K_BOX_', 'KBOX_')) and k != 'K_HOME'}
    env.update(K_HOME=str(home), PATH='/usr/bin:/bin:/usr/sbin:/sbin', DYLD_LIBRARY_PATH='')
    commands = []
    for args in [['ps', '-a', '--format', 'json'], ['volume', 'ls', '--quiet']]:
        command = [str(Path(c['boxPrefix']) / 'bin/kbox'), *args]
        result = subprocess.run(command, env=env, capture_output=True, text=True, check=True)
        commands.append(dict(command=command, exit=result.returncode, stdout=result.stdout, stderr=result.stderr))
    assert json.loads(commands[0]['stdout']) == [] and not commands[1]['stdout'].strip(), home
    disks = [str(p) for name in ['boxes', 'volumes'] for p in (home / name).rglob('*.ext4')]
    assert not disks, disks
    inventories.append(dict(home=str(home), commands=commands, workloadDisks=disks))
(checks / 'final-home-inventories.json').write_text(json.dumps(inventories, indent=2) + '\n')

processes = subprocess.check_output(['ps', '-axo', 'pid=,ppid=,command='], text=True)
owned = []
for line in processes.splitlines():
    parts = line.strip().split(None, 2)
    if len(parts) == 3 and Path(parts[2].split(None, 1)[0]).name in ['kbox', 'kbox-shim', 'koci', 'koci-kshim', 'koci-agent'] and any(str(p) in parts[2] for p in [root, private]):
        owned.append(line)
assert not owned, owned
mounts = subprocess.check_output(['/sbin/mount'], text=True)
assert all(str(p) not in mounts for p in [root, private])
images = plistlib.loads(subprocess.check_output(['/usr/bin/hdiutil', 'info', '-plist']))
assert all(not any(str(p) in json.dumps(image) for p in [root, private]) for image in images.get('images', []))

ids, socket_paths = set(), set()
for base in [checks, root / 'af-final1', private]:
    for path in base.rglob('*'):
        if not path.is_file() or path.suffix not in ['.json', '.stdout', '.stderr', '.log']:
            continue
        if path.stat().st_size > 8 * 1024 * 1024:
            continue
        data = path.read_text(errors='replace')
        ids.update(re.findall(r'/(?:private/)?tmp/kbox-sockets/([a-z0-9-]{36})/', data))
        ids.update(re.findall(r'/(?:home|k-home)/boxes/([a-z0-9-]{36})/', data))
        socket_paths.update(re.findall(r'/private/tmp/(?:ko-pool-[a-zA-Z0-9_-]+|kbox-warm-[a-zA-Z0-9_-]+)/[^\s"\\,]+\.sock', data))
for identity in ids:
    directory = Path('/private/tmp/kbox-sockets') / identity
    assert not directory.exists(), directory
    socket_paths.update(str(directory / name) for name in ['exec.sock', 'pty.sock', 'attest.sock', 'port-forward.sock'])
assert not [p for p in socket_paths if Path(p).exists()]
for base in [root, private]:
    assert not [str(p) for p in base.rglob('*') if p.is_socket()]
    command = ['/usr/sbin/lsof', '+D', str(base)]
    result = subprocess.run(command, capture_output=True, text=True)
    (checks / ('final-fd-' + ('artifacts' if base == root else 'private') + '.json')).write_text(json.dumps(dict(command=command, exit=result.returncode, stdout=result.stdout, stderr=result.stderr), indent=2) + '\n')
    assert result.returncode == 1 and not result.stdout, result.stdout

cache_results = []
for label in ['native', 'af']:
    before = json.loads((checks / (label + '-image-cache-before.json')).read_text())
    store = root / (label + '-image-cache')
    compared, metadata = [], []
    for name, identity in before.items():
        path = store / name
        assert path.is_file(), path
        stat = path.stat()
        assert (stat.st_uid, stat.st_gid, stat.st_mode & 0o777) == (identity['uid'], identity['gid'], identity['mode']), path
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if name == 'index.json':
            metadata.append(dict(path=name, before=identity['sha256'], after=actual))
        else:
            assert actual == identity['sha256'], path
            compared.append(name)
    cache_results.append(dict(store=str(store), immutableFilesCompared=len(compared), mutableCatalog=metadata))

summary = dict(status='PASS', scope='Current phase owned resources only; historical RAM preservation failure remains open', sourceInputsSha256=current['sourceInputsSha256'], sourceCommit=current['sourceCommit'], compiledInputCount=len(current['files']), homes=len(homes), managedExecutions=0, managedVolumes=0, ownedProcesses=[], ownedMounts=[], ownedFdHolders=[], observedSocketDirectories=len(ids), observedSocketPaths=len(socket_paths), remainingSockets=[], caches=cache_results, completedPythonNativeScenarios=3, completedRustNativeTests=2, abortedRustNativeTests=1, resourcesDeletedByAudit=[])
(checks / 'final-cleanup.json').write_text(json.dumps(summary, indent=2) + '\n')
print(json.dumps(summary, indent=2))
