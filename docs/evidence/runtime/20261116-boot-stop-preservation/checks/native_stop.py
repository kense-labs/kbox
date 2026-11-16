from pathlib import Path
import hashlib,json,os,re,shutil,signal,subprocess,time

E=Path('docs/evidence/runtime/20261116-boot-stop-preservation').resolve()
c=json.loads((E/'context.json').read_text());checks=E/'checks';prefix=Path(c['boxPrefix'])
root=Path(c['privateRoot'])/'stop-failure-attempt-1';root.mkdir(mode=0o700)
home=root/'home';(home/'bin').mkdir(parents=True,mode=0o700);home.chmod(0o700);(home/'lib').mkdir()
for name in ['kbox-shim','kbox-guest-init']:shutil.copy2(prefix/'bin'/name,home/'bin'/name)
for path in (prefix/'lib').iterdir():(home/'lib'/path.name).symlink_to(path.resolve())
(home/'images').symlink_to(c['imageStore'])
(home/'.kbox-boot-stop-native').write_text('owned-boot-stop-native-test\n')
client=Path(c['root'])/'native-client-stop-failure1';shutil.copy2(c['nativeClient'],client)
env={k:v for k,v in os.environ.items() if not k.startswith(('DYLD_','K_BOX_','KBOX_')) and k not in ['K_HOME','K_LIBKRUN_PATH','K_LIBKRUNFW_PATH','LIBKRUN_NO_FSYNC']}
env.update(K_HOME=str(home),K_BOX_TEST_STREAM_IMAGE=c['image'],DYLD_LIBRARY_PATH='',DYLD_FALLBACK_LIBRARY_PATH='/usr/lib',PATH=str(prefix/'bin')+':/usr/bin:/bin:/usr/sbin:/sbin',RUST_LOG='kbox_runtime=debug')
entry='vm::tests::boot_stop::native_failed_stop_preserves_live_vm_then_retries_real_finalization'
cmd=[str(client),'--ignored','--exact',entry,'--nocapture']
assert entry+': test' in subprocess.check_output([str(client),'--list'],text=True).splitlines()
outpath=checks/'native-fault1.stdout';errpath=checks/'native-fault1.stderr'
loaded={};snapshots={};started=time.monotonic()
with outpath.open('w') as out,errpath.open('w') as err:
 p=subprocess.Popen(cmd,cwd=root,env=env,stdout=out,stderr=err)
 while p.poll() is None:
  if time.monotonic()-started>300:
   p.send_signal(signal.SIGINT)
   raise TimeoutError('owned native fault test exceeded300s; resources need exact audit')
  for path in (home/'boxes').glob('*/logs/*.log'):
   try:data=path.read_bytes()[:1024*1024]
   except FileNotFoundError:continue
   digest=hashlib.sha256(data).hexdigest();key=str(path)+'@'+digest
   if key not in snapshots:
    target=checks/'native-fault-live-logs'/path.relative_to(home);target.parent.mkdir(parents=True,exist_ok=True);target=target.with_name(target.name+'.'+digest[:12]);target.write_bytes(data);snapshots[key]=dict(source=str(path),saved=str(target),bytes=len(data),sha256=digest)
  listing=subprocess.check_output(['ps','-axo','pid=,ppid=,command='],text=True)
  for line in listing.splitlines():
   parts=line.strip().split(None,2)
   if len(parts)!=3 or str(home) not in parts[2] or 'kbox-shim --config' not in parts[2]:continue
   pid,parent,command=parts
   if pid in loaded:continue
   r=subprocess.run(['/usr/sbin/lsof','-a','-p',pid,'-d','txt','-Fn'],capture_output=True,text=True)
   expected={'kbox-shim':prefix/'bin/kbox-shim',**{n:prefix/'lib'/n for n in ['libkrun.1.17.0.dylib','libkrunfw.5.dylib']}}
   actual={Path(v[1:]).name:Path(v[1:]) for v in r.stdout.splitlines() if v.startswith('n/') and Path(v[1:]).name in expected}
   if set(actual)!=set(expected):continue
   for name,path in actual.items():
    assert hashlib.sha256(path.read_bytes()).digest()==hashlib.sha256(expected[name].read_bytes()).digest()
    assert path.resolve().parent==(prefix/'lib').resolve() if name!='kbox-shim' else path.resolve()==(home/'bin/kbox-shim').resolve()
   loaded[pid]=dict(pid=int(pid),parent=int(parent),command=command,paths={k:str(v) for k,v in actual.items()})
   (checks/('native-fault-loader-'+pid+'.log')).write_text(r.stdout+r.stderr)
  time.sleep(.3)
 p.wait()
counts=re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',outpath.read_text())
record=dict(command=cmd,root=str(root),exit=p.returncode,seconds=time.monotonic()-started,actualCounts=counts,loaded=list(loaded.values()),snapshots=list(snapshots.values()),clientSha256=hashlib.sha256(client.read_bytes()).hexdigest(),boxPrefix=str(prefix),sourceInputsSha256=c['sourceInputsSha256'])
(checks/'native-fault1.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps({k:v for k,v in record.items() if k not in ['loaded','snapshots']}),flush=True)
assert p.returncode==0 and counts==[('1','0','0')],errpath.read_text()[-5000:]
assert loaded
proof=json.loads((root/'result.json').read_text());assert proof['status']=='PASS' and proof['retryActuallyStoppedVm'] and proof['noLateWrites'] and proof['exactOwnerAndAllocationRetained']
cleanup=json.loads((root/'native-cleanup.json').read_text());assert cleanup['destroyOk'] and cleanup['boxDirectoryAbsent'] and cleanup['socketDirectoryAbsent']

assert cleanup["remainingVolumes"] == []
