from pathlib import Path
import hashlib,json,os,plistlib,re,runpy,subprocess
E=Path('docs/evidence/runtime/20261116-boot-diagnostics-boundaries'); checks=E/'checks'; c=json.loads((E/'context.json').read_text()); root=Path(c['root']); private=Path(c['privateRoot'])
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=json.loads((root/'source-frozen-3.json').read_text()); current=runpy.run_path(str(root/'source_snapshot.py'))['snapshot'](); assert current['files']==frozen['files'] and current['sourceInputsSha256']==frozen['sourceInputsSha256']
identities=json.loads((checks/'final-identities-3.json').read_text()); assert identities['sourceInputsSha256']==current['sourceInputsSha256']; assert all(row['exitCode']==0 for row in identities['commands'])
failed=json.loads((checks/'native-fault1.json').read_text()); middle=json.loads((checks/'native-fault2.json').read_text()); fault=json.loads((checks/'native-fault3.json').read_text()); assert failed['actualCounts']==[['0','1','0']]; assert fault['exit']==0 and fault['actualCounts']==[['1','0','0']] and fault['loaded']
result=json.loads((Path(fault['root'])/'result.json').read_text()); assert result['status']=='PASS' and result['guestPlanted'] and result['outsideCanaryRetained'] and result['handlerReleased']
for row in [failed,middle,fault]:
 case=Path(row['root']); cleanup=json.loads((case/'native-cleanup.json').read_text()); assert cleanup['destroyOk'] and cleanup['boxDirectoryAbsent'] and cleanup['socketDirectoryAbsent']
 assert (case/'foreign/canary').read_bytes()==b'ungranted-host-canary'
rows=json.loads((checks/'af-final4/native-results.json').read_text()); assert len(rows)==5 and all(row['status']=='PASS' for row in rows)
homes={Path(row['root'])/'home' for row in [failed,middle,fault]}
for af in ['af-final2','af-final3','af-final4']:
 for p in (root/af).rglob('boxes.json'):homes.add(p.parent)
 homes.add(root/af/'native-directory/home')
inventories=[]
for home in sorted(homes):
 assert home.is_dir() and (home.is_relative_to(root) or home.is_relative_to(private))
 env={k:v for k,v in os.environ.items() if not k.startswith(('DYLD_','K_BOX_','KBOX_')) and k!='K_HOME'}; env.update(K_HOME=str(home),PATH='/usr/bin:/bin:/usr/sbin:/sbin',DYLD_LIBRARY_PATH='')
 commands=[]
 for args in [['ps','-a','--format','json'],['volume','ls','--quiet']]:
  cmd=[str(Path(c['boxPrefix'])/'bin/kbox'),*args]; r=subprocess.run(cmd,env=env,capture_output=True,text=True); commands.append(dict(command=cmd,exit=r.returncode,stdout=r.stdout,stderr=r.stderr)); assert r.returncode==0,commands[-1]
 assert json.loads(commands[0]['stdout'])==[] and not commands[1]['stdout'].strip(),home
 disks=[str(p) for name in ['boxes','volumes'] for p in (home/name).rglob('*.ext4')]; assert not disks,disks
 inventories.append(dict(home=str(home),commands=commands,workloadDisks=disks))
(checks/'final3-home-inventories.json').write_text(json.dumps(inventories,indent=2)+'\n')
saved=checks/'native-cases3';saved.mkdir()
for row in [failed,middle,fault]:
 case=Path(row['root']); directory=saved/case.name;directory.mkdir()
 for p in case.glob('*.json'):(directory/p.name).write_bytes(p.read_bytes())
processes=subprocess.check_output(['ps','-axo','pid=,ppid=,command='],text=True); owned=[]
for line in processes.splitlines():
 parts=line.strip().split(None,2)
 if len(parts)==3 and Path(parts[2].split(None,1)[0]).name in ['kbox','kbox-shim','koci','koci-kshim','koci-agent'] and any(str(p) in parts[2] for p in [root,private]):owned.append(line)
for line in processes.splitlines():
 parts=line.strip().split(None,2)
 if len(parts)==3 and Path(parts[2].split(None,1)[0]).name=='kbox' and ' engine serve ' in parts[2]:
  observed=subprocess.run(['/usr/sbin/lsof','-a','-p',parts[0],'-d','cwd','-Fn'],capture_output=True,text=True)
  if any(value.startswith('n'+str(base)) for value in observed.stdout.splitlines() for base in [root,private]):owned.append(line)
assert not owned,owned
mounts=subprocess.check_output(['/sbin/mount'],text=True); owned_mounts=[line for line in mounts.splitlines() if any(str(p) in line for p in [root,private])]; assert not owned_mounts
images=plistlib.loads(subprocess.check_output(['/usr/bin/hdiutil','info','-plist'])); owned_images=[image for image in images.get('images',[]) if any(str(p) in json.dumps(image) for p in [root,private])]; assert not owned_images
ids={result['execution']}; sockets=set()
for row in [failed,middle,fault]:
 before=json.loads((Path(row['root'])/'before-cleanup.json').read_text()); ids.add(before['id'])
for p in [* (root/'af-final2').rglob('*'),* (root/'af-final3').rglob('*'),* (root/'af-final4').rglob('*')]:
 if not p.is_file() or p.suffix not in ['.json','.stdout','.stderr','.log'] or p.stat().st_size>8*1024*1024:continue
 data=p.read_text(errors='replace');ids.update(re.findall(r'/(?:private/)?tmp/kbox-sockets/([a-z0-9-]{36})/',data)); ids.update(re.findall(r'/(?:home|k-home)/boxes/([a-z0-9-]{36})/',data)); sockets.update(re.findall(r'/private/tmp/(?:ko-pool-[a-zA-Z0-9_-]+|kbox-warm-[a-zA-Z0-9_-]+)/[^\s"\\,]+\.sock',data))
loaders = [*failed['loaded'], *middle['loaded'], *fault['loaded']]
for af in ['af-final2', 'af-final3', 'af-final4']:
 path = checks / af / 'native-loaded-libraries.json'
 if path.exists():loaders.extend(json.loads(path.read_text()))
for row in loaders:
 configuration = json.loads(row['command'].split(' --config ', 1)[1])
 assert any(Path(configuration['console_output']).is_relative_to(base) for base in [root, private]), row
 ids.add(configuration['box_id'])
 for key in ['exec_socket_path', 'pty_socket_path', 'attest_socket_path', 'port_forward_socket_path']:
  if configuration.get(key):sockets.add(configuration[key])
for identity in ids:
 directory=Path('/private/tmp/kbox-sockets')/identity; assert not directory.exists(),directory
 sockets.update(str(directory/name) for name in ['exec.sock','pty.sock','attest.sock','portfwd.sock'])
engine_rows=json.loads((checks/'observed-engines.json').read_text()) + json.loads((checks/'af-final4/owned-engines.json').read_text())
for row in engine_rows:
 assert row['command'] not in processes,row
 if row.get('socket'):sockets.add(row['socket'])
for p in sockets:
 assert not Path(p).exists(),p
for label,base in [('artifacts',root),('private',private)]:
 assert not [p for p in base.rglob('*') if p.is_socket()]
 cmd=['/usr/sbin/lsof','+D',str(base)]; r=subprocess.run(cmd,capture_output=True,text=True); (checks/('final3-fd-'+label+'.json')).write_text(json.dumps(dict(command=cmd,exit=r.returncode,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');assert r.returncode==1 and not r.stdout,r.stdout
before=json.loads((checks/'image-cache-before.json').read_text()); compared=[]; catalog=[]
for name,v in before.items():
 p=root/'image-cache'/name; assert p.is_file(),p; st=p.stat(); assert (st.st_uid,st.st_gid,st.st_mode&0o777)==(v['uid'],v['gid'],v['mode']),p; h=sha(p)
 if name=='index.json':catalog.append(dict(path=name,before=v['sha256'],after=h))
 else:assert h==v['sha256'],p;compared.append(name)
for value in c['priorArchives'].values():assert sha(Path(value['path']))==value['sha256']
for name in ['final-dist-bootdiag-1/kbox-3.4.3-macos-arm64.tar.gz','final-dist-bootdiag-2/kbox-3.4.3-macos-arm64.tar.gz']:assert (root/name).is_file()
summary=dict(status='PASS',sourceInputsSha256=current['sourceInputsSha256'],compiledInputCount=len(current['files']),homes=len(homes),managedExecutions=0,managedVolumes=0,ownedProcesses=[],ownedMounts=owned_mounts,ownedDiskImages=owned_images,ownedFdHolders=[],observedSocketDirectories=len(ids),checkedSocketPaths=len(sockets),remainingSockets=[],immutableCacheFilesCompared=len(compared),mutableCatalog=catalog,qualifiedRustNativeTests=3,qualifiedPythonNativeScenarios=3,retainedFailedDiagnosticRustNativeTests=1,retainedFailedInvalidFixtureRustNativeTests=2,retainedInvalidFixtureAfRows=5,retainedTailBytes=result['retainedTailBytes'],retentionReport=result['report'],callerSourcesRetained=True,previousArchivesRetained=True,runtimeMemoryMeasured=False,resourcesDeletedByAudit=[],priorExactCleanupReceipt="invalid-fixture-volume-cleanup.json",scope='Current033 only; six disk-service default failures, historical RAM loss, remote Git and full GA gates stay open')
(checks/'final3-cleanup.json').write_text(json.dumps(summary,indent=2)+'\n'); print(json.dumps(summary,indent=2))
