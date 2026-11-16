from pathlib import Path
import hashlib,json,os,plistlib,re,runpy,subprocess
E=Path('docs/evidence/runtime/20261116-boot-stop-preservation');C=E/'checks';c=json.loads((E/'context.json').read_text());R=Path(c['root']);P=Path(c['privateRoot']);sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=json.loads((R/'source-frozen-1.json').read_text());now=runpy.run_path(str(R/'source_snapshot.py'))['snapshot']();assert now['files']==frozen['files']
identity=json.loads((C/'final-identities-1.json').read_text());assert all(row['exitCode']==0 for row in identity['commands'])
fault=json.loads((C/'native-fault1.json').read_text());assert fault['exit']==0 and fault['actualCounts']==[['1','0','0']] and fault['loaded'];case=Path(fault['root']);result=json.loads((case/'result.json').read_text());assert result['status']=='PASS' and result['syntheticOneShotStopError'] and not result['actualOsStopFailureClaimed'] and result['noLateWrites'] and result['exactOwnerAndAllocationRetained']
cleanup=json.loads((case/'native-cleanup.json').read_text());assert cleanup['destroyOk'] and cleanup['boxDirectoryAbsent'] and cleanup['socketDirectoryAbsent'] and cleanup['remainingVolumes']==[]
rows=json.loads((C/'af-final1/native-results.json').read_text());assert len(rows)==5 and all(row['status']=='PASS' for row in rows)
homes={case/'home',Path(c['fixtureHome'])}
for p in (R/'af-final1').rglob('boxes.json'):homes.add(p.parent)
homes.add(R/'af-final1/native-directory/home');inventories=[]
for home in sorted(homes):
 assert home.is_dir() and (home.is_relative_to(R) or home.is_relative_to(P));env={k:v for k,v in os.environ.items() if not k.startswith(('DYLD_','K_BOX_','KBOX_')) and k!='K_HOME'};env.update(K_HOME=str(home),PATH='/usr/bin:/bin:/usr/sbin:/sbin',DYLD_LIBRARY_PATH='');commands=[]
 for args in [['ps','-a','--format','json'],['volume','ls','--quiet']]:
  cmd=[str(Path(c['boxPrefix'])/'bin/kbox'),*args];r=subprocess.run(cmd,env=env,capture_output=True,text=True);commands.append(dict(command=cmd,exit=r.returncode,stdout=r.stdout,stderr=r.stderr));assert r.returncode==0,commands[-1]
 assert json.loads(commands[0]['stdout'])==[] and not commands[1]['stdout'].strip(),home
 disks=[str(p) for name in ['boxes','volumes'] for p in (home/name).rglob('*.ext4')];assert not disks
 inventories.append(dict(home=str(home),commands=commands,workloadDisks=disks))
(C/'final-home-inventories.json').write_text(json.dumps(inventories,indent=2)+'\n')
saved=C/'native-case';saved.mkdir()
for p in case.glob('*.json'):(saved/p.name).write_bytes(p.read_bytes())
processes=subprocess.check_output(['ps','-axo','pid=,ppid=,command='],text=True);owned=[]
for line in processes.splitlines():
 parts=line.strip().split(None,2)
 if len(parts)!=3:continue
 role=Path(parts[2].split(None,1)[0]).name
 if role in ['kbox','kbox-shim','koci','koci-kshim','koci-agent'] and any(str(base) in parts[2] for base in [R,P]):owned.append(line)
 if role=='kbox' and ' engine serve ' in parts[2]:
  r=subprocess.run(['/usr/sbin/lsof','-a','-p',parts[0],'-d','cwd','-Fn'],capture_output=True,text=True)
  if any(value.startswith('n'+str(base)) for value in r.stdout.splitlines() for base in [R,P]):owned.append(line)
assert not owned,owned
mounts=subprocess.check_output(['/sbin/mount'],text=True);assert not [line for line in mounts.splitlines() if any(str(base) in line for base in [R,P])]
images=plistlib.loads(subprocess.check_output(['/usr/bin/hdiutil','info','-plist']));assert not [image for image in images.get('images',[]) if any(str(base) in json.dumps(image) for base in [R,P])]
loaders=fault['loaded']+json.loads((C/'af-final1/native-loaded-libraries.json').read_text());ids=set();sockets=set()
for row in loaders:
 config=json.loads(row['command'].split(' --config ',1)[1]);assert any(Path(config['console_output']).is_relative_to(base) for base in [R,P]);ids.add(config['box_id'])
 for key in ['exec_socket_path','pty_socket_path','attest_socket_path','port_forward_socket_path']:
  if config.get(key):sockets.add(config[key])
for p in (R/'af-final1').rglob('*'):
 if not p.is_file() or p.suffix not in ['.json','.stdout','.stderr','.log'] or p.stat().st_size>8*1024*1024:continue
 data=p.read_text(errors='replace');ids.update(re.findall(r'/(?:private/)?tmp/kbox-sockets/([a-z0-9-]{36})/',data));sockets.update(re.findall(r'/private/tmp/(?:ko-pool-[a-zA-Z0-9_-]+|kbox-warm-[a-zA-Z0-9_-]+)/[^\s"\\,]+\.sock',data))
for id in ids:
 directory=Path('/private/tmp/kbox-sockets')/id;assert not directory.exists(),directory
 sockets.update(str(directory/name) for name in ['exec.sock','pty.sock','attest.sock','portfwd.sock'])
engines=json.loads((C/'af-final1/owned-engines.json').read_text());assert engines
for row in engines:
 assert row['command'] not in processes;sockets.add(row['socket'])
assert not [p for p in sockets if Path(p).exists()]
for label,base in [('artifacts',R),('private',P)]:
 assert not [p for p in base.rglob('*') if p.is_socket()]
 cmd=['/usr/sbin/lsof','+D',str(base)];r=subprocess.run(cmd,capture_output=True,text=True);(C/('final-fd-'+label+'.json')).write_text(json.dumps(dict(command=cmd,exit=r.returncode,stdout=r.stdout,stderr=r.stderr),indent=2)+'\n');assert r.returncode==1 and not r.stdout
before=json.loads((C/'image-cache-before.json').read_text());immutable=[];catalog=[]
for name,value in before.items():
 p=R/'image-cache'/name;assert p.is_file();st=p.stat();assert (st.st_uid,st.st_gid,st.st_mode&0o777)==(value['uid'],value['gid'],value['mode']);h=sha(p)
 if name=='index.json':catalog.append(dict(path=name,before=value['sha256'],after=h))
 else:assert h==value['sha256'],p;immutable.append(name)
assert (case/'caller/canary').read_bytes()==b'caller-owned-retained' and (case/'caller/writes').stat().st_size==result['callerBytesRetained']
for value in c['priorArchives'].values():assert sha(Path(value['path']))==value['sha256']
install=json.loads((C/'install-bootstop-final1.json').read_text());assert sha(Path(install['archive']))==install['sha256']
summary=dict(status='PASS',sourceInputsSha256=now['sourceInputsSha256'],compiledInputCount=len(now['files']),homes=len(homes),managedExecutions=0,managedVolumes=0,ownedProcesses=[],ownedMounts=[],ownedDiskImages=[],ownedFdHolders=[],observedVmSocketDirectories=len(ids),checkedSocketPaths=len(sockets),remainingSockets=[],observedEngines=len(engines),immutableCacheFilesCompared=len(immutable),mutableCatalog=catalog,qualifiedRustNativeTests=3,qualifiedPythonScenarios=3,stopFault='Synthetic one-shot; real VM/live writes/allocation identity/actual retry-stop observed',actualOsStopFailureClaimed=False,callerSourcesRetained=True,previousArchivesRetained=True,resourcesDeletedByAudit=[],runtimeMemoryMeasured=False,outerPoolOwnerLifetimeSolved=False,gaComplete=False)
(C/'final-cleanup.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))
