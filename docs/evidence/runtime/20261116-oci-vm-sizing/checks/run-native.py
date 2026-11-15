from pathlib import Path
import hashlib, json, os, re, shutil, subprocess, sys, time

context = Path(sys.argv[1]).resolve()
c = json.loads(context.read_text())
checks = context.parent / 'checks'
mode=sys.argv[3] if len(sys.argv)>3 else 'anonymous'
assert mode in ('vm-sizing','vm-sizing-baseline','vm-sizing-old-host','vm-sizing-old-shim','vm-sizing-legacy-old-shim','shm-size','cap-add','anonymous','raw-ro','tmpfs','named','files','rootro','old-rootro','ulimits','cpu-time','host-service','memory-limits','nproc','sigpending','msgqueue','nice','realtime','nproc-old-host','nproc-old-shim','nproc-old-agent')
if mode in ('old-host','old-rootro'):
    c['ociPrefix']=c['prior']['ociPrefix'];c['systemImageManifest']=c['prior']['systemImageManifest']
label = mode+'-attempt-' + str(int(sys.argv[2]))
root = Path(c['privateRoot']) / label
root.mkdir(mode=0o700)
home = root / 'home'
(home / 'bin').mkdir(parents=True, mode=0o700)
home.chmod(0o700)
(home / 'lib').mkdir()
old = Path(c.get('boxPrefix', str(Path(c['root']) / 'previous-dist/kbox-3.4.3-macos-arm64')))
for name in ['kbox-shim', 'kbox-guest-init']:
    shutil.copy2(old / 'bin' / name, home / 'bin' / name)
for path in (old / 'lib').iterdir():
    (home / 'lib' / path.name).symlink_to(path.resolve())
(home / 'images').symlink_to(c['imageStore'])
(root / '.kbox-hvf-native').write_text('owned-hvf-native-test\n')
client = Path(c['root']) / ('native-client-' + label)
shutil.copy2(Path(c['nativeClient']), client)
oci_current=Path(c['ociPrefix']);oci_old=Path(c['oldNprocOciPrefix'])
if mode=='vm-sizing-legacy-old-shim': oci_old=Path(c['prior']['ociPrefix'])
if mode.startswith('vm-sizing-old-'): oci_old=Path(c['prior']['ociPrefix'])
host_prefix=oci_old if mode in ('nproc-old-host','vm-sizing-old-host') else oci_current
shim_prefix=oci_old if mode in ('nproc-old-host','nproc-old-shim','vm-sizing-old-host','vm-sizing-old-shim','vm-sizing-legacy-old-shim') else oci_current
image_prefix=oci_old if mode in ('nproc-old-host','nproc-old-agent') else oci_current
env = os.environ.copy()
for key in list(env):
    if key.startswith(('DYLD_', 'K_BOX_OCI_', 'K_BOX_TEST_')):
        env.pop(key)
env.update(K_HOME=str(home), K_BOX_TEST_HVF_ROOT=str(root), K_BOX_TEST_HVF_HOST=str(host_prefix / 'koci'), K_BOX_TEST_HVF_SHIM=str(shim_prefix / 'koci-kshim'), K_BOX_TEST_HVF_MANIFEST=str(image_prefix/'system-image/system-image.json'), K_BOX_TEST_STREAM_IMAGE=c['image'], K_BOX_TEST_HVF_FOREIGN_UID_IMAGE=c['image'], DYLD_LIBRARY_PATH='', PATH='/usr/bin:/bin:/usr/sbin:/sbin')
if mode.startswith('nproc'):
    nproc_input=root/'nproc-probe-input';shutil.copy2(c['nprocProbe'],nproc_input)
    assert hashlib.sha256(nproc_input.read_bytes()).digest()==hashlib.sha256(Path(c['nprocProbe']).read_bytes()).digest()
else: nproc_input=Path(c['nprocProbe'])
env['K_BOX_TEST_NPROC_PROBE']=str(nproc_input)
env['K_BOX_TEST_NPROC_GUEST_TESTS']=c['nprocGuestAgentTests']
env['K_BOX_TEST_NPROC_OLD_AGENT']=str(oci_old/'koci-agent')
env['K_BOX_TEST_NPROC_OLD_AGENT_PROOF']=c['nprocOldAgentProof']
env['K_BOX_TEST_NPROC_OLD_ASSET']=mode.removeprefix('nproc-')
env['K_BOX_TEST_HVF_RECOVERY_SHIM']=str(oci_current/'koci-kshim')
env['K_BOX_TEST_HVF_RECOVERY_MANIFEST']=str(oci_current/'system-image/system-image.json')
env['K_BOX_TEST_NPROC_EXPECTED_REFUSAL']='Host lacks required kernel-counted nproc admission' if mode=='nproc-old-host' else 'execution backend unavailable'
env['K_BOX_TEST_NPROC_CLI_SCRIPT']=c['nprocCliProbe']
sigpending_input=root/'sigpending-probe-input'
if mode=='sigpending':
    shutil.copy2(c['sigpendingProbe'],sigpending_input)
    assert hashlib.sha256(sigpending_input.read_bytes()).digest()==hashlib.sha256(Path(c['sigpendingProbe']).read_bytes()).digest()
    env['K_BOX_TEST_SIGPENDING_PROBE']=str(sigpending_input)
env['K_BOX_TEST_SIGPENDING_CLI_SCRIPT']=c['sigpendingCliProbe']
if mode=='msgqueue':
    mq_input=root/'msgqueue-probe-input';shutil.copy2(c['msgqueueProbe'],mq_input)
    assert hashlib.sha256(mq_input.read_bytes()).digest()==hashlib.sha256(Path(c['msgqueueProbe']).read_bytes()).digest()
    env['K_BOX_TEST_MSGQUEUE_PROBE']=str(mq_input)
env['K_BOX_TEST_MSGQUEUE_CLI_SCRIPT']=c['msgqueueCliProbe']
if mode=='nice':
    nice_input=root/'nice-probe-input';shutil.copy2(c['niceProbe'],nice_input)
    assert hashlib.sha256(nice_input.read_bytes()).digest()==hashlib.sha256(Path(c['niceProbe']).read_bytes()).digest()
    env['K_BOX_TEST_NICE_PROBE']=str(nice_input)
env['K_BOX_TEST_NICE_CLI_SCRIPT']=c['niceCliProbe']
if mode=='realtime':
    rt_input=root/'realtime-probe-input';shutil.copy2(c['realtimeProbe'],rt_input)
    assert hashlib.sha256(rt_input.read_bytes()).digest()==hashlib.sha256(Path(c['realtimeProbe']).read_bytes()).digest()
    env['K_BOX_TEST_REALTIME_PROBE']=str(rt_input)
env['K_BOX_TEST_REALTIME_CLI_SCRIPT']=c['realtimeCliProbe']


env['K_BOX_TEST_MEMORY_LIMIT_PROBE']=c['memoryLimitProbe']
env['K_BOX_TEST_CPU_TIME_PROBE']=c['cpuTimeProbe']
env['K_BOX_TEST_LIMIT_PROBE']=str(Path(c['root'])/'inputs/rlimit-probe')
env['K_BOX_TEST_HVF_PROBE'] = c['readonlyProbe']
env['K_BOX_TEST_HVF_FILE_PROBE'] = c['fileProbe']
env['K_BOX_TEST_VM_HARDWARE_PROBE']=c['hardwareProbe']
env['K_BOX_TEST_VM_SIZING_OLD_ASSET']=mode.removeprefix('vm-sizing-')
env['K_BOX_TEST_VM_SIZING_MODE']='baseline' if mode in ('vm-sizing-baseline','vm-sizing-legacy-old-shim') else 'matrix'
env['K_BOX_TEST_HVF_NAMED_IMAGE'] = c['sameUidImage']
env['K_BOX_TEST_HVF_VOLUME_IMAGE']=c['volumeImage']
env['K_BOX_TEST_HVF_RAW_IMAGE_LAYOUT']=str(Path(c['imageStore'])/'sha256'/c['image'].split('@sha256:')[1])
env.pop('K_BOX_TEST_HVF_OLD_CLI',None)
if c.get('requireTmpfsCli'): env['K_BOX_TEST_HVF_CLI'] = str(Path(c['boxPrefix']) / 'bin/kbox')
else: env.pop('K_BOX_TEST_HVF_CLI',None)
if mode=='shm-size':
    env['K_BOX_TEST_SHM_PROBE']=c['shmProbe'];env['K_BOX_TEST_SHM_CLI_SCRIPT']=c['shmCliProbe']
if mode=='cap-add':
    env['K_BOX_TEST_CAPABILITY_PROBE']=c['capabilityProbe'];env['K_BOX_TEST_CAPABILITY_CLI_SCRIPT']=c['capabilityCliProbe']
entry='native_anonymous_volumes_preserve_guest_metadata_identity_restart_and_owned_cleanup' if mode=='anonymous' else 'native_old_host_refuses_raw_volume_capability_before_allocation_or_workload'
module='anonymous'
if mode=='shm-size': module='shm';entry='native_shm_size_enforces_capacity_posix_effects_all_entries_and_recovery'
if mode=='cap-add': module='capabilities';entry='native_cap_add_enforces_guest_permissions_drop_controls_all_entries_and_recovery'
if mode=='old-rootro': module='root_readonly';entry='native_old_host_refuses_readonly_root_before_preparation_without_fallback'
if mode=='host-service': module='';entry='native_box_hvf_host_service_preserves_routes_and_recovery'
if mode.startswith('nproc-old-'): module='nproc';entry='native_old_nproc_assets_refuse_without_workload_or_route_fallback'
if mode=='nproc': module='nproc';entry='native_nproc_enforces_nonroot_budget_all_process_paths_and_recovery'
if mode=='sigpending': module='sigpending';entry='native_sigpending_limits_enforce_queue_effects_all_entries_and_recovery'
if mode=='msgqueue': module='msgqueue';entry='native_msgqueue_limits_enforce_byte_budget_all_entries_and_recovery'
if mode=='nice': module='nice';entry='native_nice_limits_enforce_priority_floor_all_entries_and_recovery'
if mode=='realtime': module='realtime';entry='native_realtime_limits_enforce_scheduler_budget_all_entries_and_recovery'
if mode=='memory-limits': module='memory_limits';entry='native_memory_limits_enforce_all_process_paths_replay_and_recovery'
if mode=='cpu-time': module='cpu_time';entry='native_cpu_time_limits_signal_all_process_paths_replay_and_recovery'
if mode=='ulimits': module='ulimits';entry='native_ulimits_enforce_kernel_effects_all_process_paths_replay_and_recovery'
if mode=='rootro': module='root_readonly';entry='native_root_readonly_intent_enforces_all_access_paths_and_preserves_inner_mounts'
if mode=='tmpfs': module='tmpfs';entry='native_tmpfs_intent_enforces_flags_quota_identity_restart_and_stopped_recovery'
if mode=='named': module='named';entry='native_named_volumes_preserve_identity_live_data_ro_and_shared_lifetime'
if mode=='files': module='files';entry='native_box_hvf_single_file_host_pep'
if mode.startswith('vm-sizing'): module='vm_sizing';entry='native_vm_hardware_sizes_match_persisted_intent'
if mode.startswith('vm-sizing-old-'): entry='native_old_vm_sizing_assets_refuse_before_workload_without_fallback'
if mode=='raw-ro': module='raw_ro';entry='native_raw_ro_disks_reject_guest_remount_child_and_writes_with_unchanged_host_image'
cmd = [str(client), '--ignored', '--exact', 'local_execution::oci_hvf::tests::native::'+(('' if mode in ('files','host-service') else module+'::'))+entry, '--nocapture']
assert cmd[3]+': test' in subprocess.check_output([str(client),'--list'],text=True).splitlines(), 'exact native test name missing'
start = time.monotonic()
outpath = checks / ('box-native-' + label + '.stdout')
errpath = checks / ('box-native-' + label + '.stderr')
loaded = {}
with outpath.open('w') as out, errpath.open('w') as err:
    p = subprocess.Popen(cmd, cwd=root, env=env, stdout=out, stderr=err)
    while p.poll() is None:
        assert time.monotonic()-start < 600, 'owned native test exceeded its deadline'
        listing = subprocess.check_output(['ps','-ax','-o','pid=,ppid=,command='],text=True)
        for line in listing.splitlines():
            if str(root) not in line: continue
            kind = 'oci' if 'koci-kshim __macos-agent-vm-worker' in line else 'box' if 'kbox-shim --config' in line else None
            if not kind: continue
            pid,parent,command = line.strip().split(None,2); key=kind+'-'+pid
            if key in loaded:
                if mode=='raw-ro' and not loaded[key].get('nativeDiskDescriptorProof'):
                    sample=subprocess.run(['/usr/sbin/lsof','-a','-p',pid,'-Fpcfan'],capture_output=True,text=True)
                    items=[];item={}
                    for field in sample.stdout.splitlines():
                        if field.startswith('f'):
                            if item:items.append(item)
                            item={'fd':field[1:]}
                        elif field.startswith('a'):item['access']=field[1:]
                        elif field.startswith('n'):item['path']=field[1:]
                    if item:items.append(item)
                    sources={name:[entry for entry in items if entry.get('path')==str(root/'raw-inputs'/name/'rootfs.ext4')] for name in ['readonly','writable']}
                    if all(len(entries)>=2 for entries in sources.values()):
                        assert all(entry['access']=='r' for entry in sources['readonly'])
                        assert all(entry['access']=='u' for entry in sources['writable'])
                        snapshot=checks/('descriptors-'+label+'-'+key+'-native.log');snapshot.write_text(sample.stdout+sample.stderr)
                        loaded[key]['nativeDiskDescriptorProof']=dict(log=str(snapshot),sources=sources)
                continue
            result = subprocess.run(['/usr/sbin/lsof','-a','-p',pid,'-d','txt','-Fn'],capture_output=True,text=True)
            prefix = shim_prefix if kind=='oci' else old
            if kind=='oci' and Path(command.split(None,1)[0]).resolve() != (shim_prefix/'koci-kshim').resolve():
                assert mode in ('nproc-old-shim','nproc-old-agent','vm-sizing-old-shim') and (root/'host-2.pid').is_file()
                assert Path(command.split(None,1)[0]).resolve()==(oci_current/'koci-kshim').resolve()
                prefix=oci_current
            library = prefix/'koci-kbox-runtime' if kind=='oci' else prefix/'lib'
            executable = 'koci-kshim' if kind=='oci' else 'kbox-shim'
            expected = {name:library/name for name in ['libkrun.1.17.0.dylib','libkrunfw.5.dylib']}
            expected[executable] = prefix/executable if kind=='oci' else prefix/'bin'/executable
            actual={Path(v[1:]).name:Path(v[1:]) for v in result.stdout.splitlines() if v.startswith('n/') and Path(v[1:]).name in expected}
            if set(actual)!=set(expected): continue
            for name,path in actual.items():
                assert hashlib.sha256(path.read_bytes()).digest()==hashlib.sha256(expected[name].read_bytes()).digest(), path
                if name!=executable: assert path.resolve().parent==library.resolve(),path
                else: assert path.resolve()==expected[executable].resolve() or kind=='box' and path.resolve()==(home/'bin'/executable).resolve(),path
            loaded[key]=dict(kind=kind,pid=int(pid),parent=int(parent),command=command,paths={k:str(v) for k,v in actual.items()})
            (checks/('loader-'+label+'-'+key+'.log')).write_text(result.stdout+result.stderr)
            descriptors=subprocess.run(['/usr/sbin/lsof','-a','-p',pid,'-Fpcfan'],capture_output=True,text=True)
            (checks/('descriptors-'+label+'-'+key+'.log')).write_text(descriptors.stdout+descriptors.stderr)
        time.sleep(.3)
    p.wait()
(checks/('loader-'+label+'.json')).write_text(json.dumps(list(loaded.values()),indent=2)+'\n')
counts = re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', outpath.read_text())
record = dict(command=cmd, root=str(root), clientSha256=hashlib.sha256(client.read_bytes()).hexdigest(), boxAssetsPrefix=str(old), ociHostPrefix=str(host_prefix),ociShimPrefix=str(shim_prefix),systemImagePrefix=str(image_prefix), image=c['volumeImage'] if mode in ('tmpfs','anonymous','rootro','old-rootro') else c['sameUidImage'] if mode=='named' else c['image'], foreignUidImage=None, exitCode=p.returncode, seconds=time.monotonic()-start, actualCounts=counts, resultExists=(root/'result.json').exists(), cleanupExists=(root/'cleanup-boxes.json').exists())
(checks / ('box-native-' + label + '.json')).write_text(json.dumps(record, indent=2) + '\n')
print(json.dumps(record), flush=True)
print('\n'.join(errpath.read_text().splitlines()[-20:]), flush=True)
if p.returncode == 0:
    assert counts == [('1', '0', '0')]
    if mode=='vm-sizing-old-host': assert not loaded, 'old Host entered a VM despite mandatory sizing'
    if mode=='vm-sizing-old-shim':
        assert all(row['kind']=='oci' and Path(row['command'].split(None,1)[0]).resolve()==(oci_current/'koci-kshim').resolve() for row in loaded.values()), 'old Shim entered a VM before mandatory argument refusal'
        assert (root/'host-2.pid').is_file()
        assert json.loads((root/'pending-create-cleanup.json').read_text())['workloadStartsAdded']==0
    if not mode.startswith(('nproc-old-','vm-sizing-old-')): assert {v['kind'] for v in loaded.values()} == ({'oci','box'} if mode in ('host-service','vm-sizing') else {'oci'} if mode not in ('old-host','old-rootro','nproc-old-host','nproc-old-shim') else set())
    if mode in ('nproc-old-host','nproc-old-shim'): assert {v['kind'] for v in loaded.values()}.issubset({'oci'})
    if mode in ('nproc-old-shim','nproc-old-agent'):
        logs=[]
        for path in [root/'nproc-compatibility-refusal.txt',*(root/'service').rglob('*.log')]:
            if path.is_file() and path.stat().st_size<8*1024*1024:
                for line in path.read_text(errors='replace').splitlines():
                    if 'nprocTransportSchema' in line or 'koci.agent-vm-attachments.v10' in line:logs.append(dict(path=str(path),line=line))
        if mode=='nproc-old-agent':
            proof_path=Path(c['nprocOldAgentProof']);proof=json.loads(proof_path.read_text())
            assert proof['status']=='PASS' and proof['schema']=='koci.agent-vm-attachments.v10' and proof['guestAgentExitCode']==1
            assert proof['agentSha256']==hashlib.sha256((oci_old/'koci-agent').read_bytes()).hexdigest()
            assert proof['guestDecoderRefusedBeforeToken'] and proof['socketDirectoryRemoved'] and proof['workerReaped']
            assert 'unknown field `nprocTransportSchema`' in proof['console']
            logs.append(dict(path=str(proof_path),proof=proof))
        assert logs, 'missing actual old asset mandatory-v10 rejection log'
        (checks/('nproc-compatibility-'+label+'.json')).write_text(json.dumps(dict(status='PASS',oldPrefix=str(oci_old),oldAgentSha256=hashlib.sha256((oci_old/'koci-agent').read_bytes()).hexdigest(),logs=logs),indent=2)+'\n')
    if c.get('requireTmpfsCli') and mode in ('vm-sizing','shm-size','cap-add','anonymous','named','tmpfs','rootro','ulimits','cpu-time','host-service','memory-limits','nproc','sigpending','msgqueue','nice','realtime'): assert json.loads((root / 'result.json').read_text())['cliChecked'] is True
    if mode=='raw-ro': assert any(row.get('nativeDiskDescriptorProof') for row in loaded.values()), 'no retained/native disk FD proof'
    if mode=='vm-sizing-old-shim':
        logs=[dict(path=str(p),line=line) for p in [root/'vm-sizing-compatibility-refusal.txt',*(root/'service').rglob('*.log')] if p.is_file() and p.stat().st_size<8*1024*1024 for line in p.read_text(errors='replace').splitlines() if '--vm-vcpus' in line and ('unexpected argument' in line or 'unrecognized' in line)]
        assert logs, 'missing actual old Shim rejection of explicit sizing arguments'
        (checks/('compatibility-'+label+'.json')).write_text(json.dumps(dict(status='PASS',logs=logs),indent=2)+'\n')
    assert json.loads((root / 'result.json').read_text())['status'] == 'PASS'
    assert json.loads((root / 'cleanup-boxes.json').read_text()) == []
raise SystemExit(p.returncode)
