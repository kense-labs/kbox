from pathlib import Path
import hashlib,json,os,platform,plistlib,re,runpy,subprocess,time
E=Path("docs/evidence/runtime/20261116-pool-postboot-cleanup");checks=E/"checks";c=json.loads((E/"context.json").read_text());root=Path(c["root"]);private=Path(c["privateRoot"])
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=json.loads((root/"source-frozen-1.json").read_text());current=runpy.run_path(str(root/"source_snapshot.py"))["snapshot"]();assert current["files"]==frozen["files"] and current["sourceInputsSha256"]==frozen["sourceInputsSha256"]
assert json.loads((checks/"native-fault-launch1.json").read_text())["exit"]==0
fault=json.loads((checks/"native-fault1.json").read_text());assert fault["exit"]==0 and fault["actualCounts"]==[["1","0","0"]] and fault["loaded"]
case=Path(fault["root"]);result=json.loads((case/"result.json").read_text());assert result["status"]=="PASS" and result["noLateWrites"] and result["vmStopped"]
cleanup=json.loads((case/"native-cleanup.json").read_text());assert cleanup["destroyOk"] and cleanup["boxDirectoryAbsent"] and cleanup["socketDirectoryAbsent"]
rows=json.loads((checks/"af-final1/native-results.json").read_text());assert len(rows)==5 and all(r["status"]=="PASS" for r in rows)
pools=json.loads((checks/"pool-wave-1/native-results.json").read_text());assert pools["status"]=="PASS" and pools["daemons"]==4 and pools["initialReadyVms"]==16 and len(pools["commands"])==4
assert json.loads((checks/"pool-wave-1/cleanup.json").read_text())["status"]=="PASS"
homes={case/"home",*(root/"pool-wave-1").glob("*/home")}
for p in (root/"af-final1").rglob("boxes.json"):homes.add(p.parent)
assert len(homes)==10,homes
inventories=[]
for home in sorted(homes):
 assert home.is_dir() and (home.is_relative_to(root) or home.is_relative_to(private))
 env={k:v for k,v in os.environ.items() if not k.startswith(("DYLD_","K_BOX_","KBOX_")) and k!="K_HOME"};env.update(K_HOME=str(home),PATH="/usr/bin:/bin:/usr/sbin:/sbin",DYLD_LIBRARY_PATH="")
 commands=[]
 for args in [["ps","-a","--format","json"],["volume","ls","--quiet"]]:
  cmd=[str(Path(c["boxPrefix"])/"bin/kbox"),*args];r=subprocess.run(cmd,env=env,capture_output=True,text=True);commands.append(dict(command=cmd,exit=r.returncode,stdout=r.stdout,stderr=r.stderr));assert r.returncode==0,commands[-1]
 assert json.loads(commands[0]["stdout"])==[] and not commands[1]["stdout"].strip(),home
 disks=[str(p) for name in ["boxes","volumes"] for p in (home/name).rglob("*.ext4")];assert not disks,disks
 inventories.append(dict(home=str(home),commands=commands,workloadDisks=disks))
(checks/"final-home-inventories.json").write_text(json.dumps(inventories,indent=2)+"\n")
saved=checks/"native-fault";saved.mkdir()
for p in case.glob("*.json"):(saved/p.name).write_bytes(p.read_bytes())
processes=subprocess.check_output(["ps","-axo","pid=,ppid=,command="],text=True);owned=[]
for line in processes.splitlines():
 parts=line.strip().split(None,2)
 if len(parts)==3 and Path(parts[2].split(None,1)[0]).name in ["kbox","kbox-shim","koci","koci-kshim","koci-agent"] and any(str(p) in parts[2] for p in [root,private]):owned.append(line)
assert not owned,owned
mounts=subprocess.check_output(["/sbin/mount"],text=True);assert all(str(p) not in mounts for p in [root,private])
images=plistlib.loads(subprocess.check_output(["/usr/bin/hdiutil","info","-plist"]));assert all(not any(str(p) in json.dumps(image) for p in [root,private]) for image in images.get("images",[]))
(checks/"final-platform-resources2.json").write_text(json.dumps(dict(globalProcessRows=len(processes.splitlines()),ownedProcesses=owned,ownedMounts=[line for line in mounts.splitlines() if any(str(p) in line for p in [root,private])],ownedImages=[image for image in images.get("images",[]) if any(str(p) in json.dumps(image) for p in [root,private])]),indent=2)+"\n")
ids=set();sockets=set()
for base in [private,root/"af-final1",root/"pool-wave-1"]:
 for p in base.rglob("*"):
  if not p.is_file() or p.suffix not in [".json",".stdout",".stderr",".log"] or p.stat().st_size>8*1024*1024:continue
  data=p.read_text(errors="replace");ids.update(re.findall(r"/(?:private/)?tmp/kbox-sockets/([a-z0-9-]{36})/",data));ids.update(re.findall(r"/(?:home|k-home)/boxes/([a-z0-9-]{36})/",data));sockets.update(re.findall(r"/private/tmp/(?:ko-pool-[a-zA-Z0-9_-]+|kbox-warm-[a-zA-Z0-9_-]+)/[^\s\"\\,]+\.sock",data))
for identity in ids:
 directory=Path("/private/tmp/kbox-sockets")/identity;assert not directory.exists(),directory
 sockets.update(str(directory/name) for name in ["exec.sock","pty.sock","attest.sock","portfwd.sock"])
assert not [p for p in sockets if Path(p).exists()]
for label,base in [("artifacts",root),("private",private)]:
 assert not [p for p in base.rglob("*") if p.is_socket()]
 cmd=["/usr/sbin/lsof","+D",str(base)];r=subprocess.run(cmd,capture_output=True,text=True)
 (checks/("final-fd-"+label+".json")).write_text(json.dumps(dict(command=cmd,exit=r.returncode,stdout=r.stdout,stderr=r.stderr),indent=2)+"\n")
 assert r.returncode==1 and not r.stdout,r.stdout
before=json.loads((checks/"image-cache-before.json").read_text());store=root/"image-cache";compared=[];catalog=[]
for name,v in before.items():
 p=store/name;assert p.is_file(),p;st=p.stat();assert (st.st_uid,st.st_gid,st.st_mode&0o777)==(v["uid"],v["gid"],v["mode"]),p;h=sha(p)
 if name=="index.json":catalog.append(dict(path=name,before=v["sha256"],after=h))
 else:assert h==v["sha256"],p;compared.append(name)
assert (case/"caller/canary").read_bytes()==b"caller-owned-retained"
assert (case/"caller/writes").is_file() and (case/"caller/writes").stat().st_size==result["callerBytesRetained"]
for key,expected in [("boxArchive","aa924efec61b7cce329061d8fe356d83ddc05cadb520d6ff18c8c1d4b9001da6"),("ociArchive","d5f599951c86696d19d0e775993c50573bf74fd597f17e325f477be407874a79")]:assert sha(Path(c["prior"][key]))==expected
summary=dict(status="PASS",sourceInputsSha256=current["sourceInputsSha256"],compiledInputCount=len(current["files"]),homes=len(homes),managedExecutions=0,managedVolumes=0,ownedProcesses=[],ownedMounts=[],ownedFdHolders=[],observedSocketDirectories=len(ids),checkedSocketPaths=len(sockets),remainingSockets=[],immutableCacheFilesCompared=len(compared),mutableCatalog=catalog,qualifiedRustNativeTests=3,qualifiedPythonNativeScenarios=3,concurrentPoolDaemons=4,concurrentIdleVms=16,realPoolCommands=4,postBootFailureSeconds=result["postBootCheckSeconds"],callerSourcesRetained=True,priorArchivesRetainedUnchanged=True,runtimeMemoryMeasured=False,resourcesDeletedByAudit=[],scope="This phase only; historical RAM and global external gates stay open")
(checks/"final-cleanup.json").write_text(json.dumps(summary,indent=2)+"\n");print(json.dumps(summary,indent=2))
