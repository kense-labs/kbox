from pathlib import Path
import hashlib,json,os,platform,plistlib,re,runpy,subprocess,time
E=Path("docs/evidence/runtime/20261116-oci-vm-sizing");checks=E/"checks";c=json.loads((E/"context.json").read_text());root=Path(c["root"]);private=Path(c["privateRoot"])
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
frozen=json.loads((root/"source-frozen-2.json").read_text());current=runpy.run_path(str(root/"source_snapshot.py"))["snapshot"]()
assert current["files"]==frozen["files"] and current["sourceInputsSha256"]==frozen["sourceInputsSha256"]
accepted=[]
for label in ["vm-sizing-attempt-2","vm-sizing-old-host-attempt-2","vm-sizing-old-shim-attempt-4","vm-sizing-legacy-old-shim-attempt-1"]:
 receipt=json.loads((checks/("box-native-"+label+".json")).read_text());assert receipt["exitCode"]==0 and receipt["actualCounts"]==[["1","0","0"]]
 case=Path(receipt["root"]);assert json.loads((case/"result.json").read_text())["status"]=="PASS" and json.loads((case/"cleanup-boxes.json").read_text())==[]
 accepted.append(receipt)
rows=json.loads((checks/"af-final1/native-results.json").read_text());assert [r["case"] for r in rows]==["oci","readonly","primary","prewarm","directory"] and all(r["status"]=="PASS" for r in rows)
for item in json.loads((checks/"failed-candidates-reconciled2.json").read_text()):
 assert item["exit"]==0 and item["actualCounts"]==[["1","0","0"]] and item["workloadStartsAfter"]==item["workloadStartsBefore"] and item["workloadMarkerAbsent"]
homes=set(private.glob("*/home"))
for path in root.rglob("boxes.json"):homes.add(path.parent)
assert len(homes)==15
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
for case in private.iterdir():
 if not case.is_dir():continue
 assert not list((case/"service/state/containers").glob("*/record.json")),case
 assert not list((case/"service/runtime/shares").rglob("*.ext4")),case
 saved=checks/("native-"+case.name);saved.mkdir(exist_ok=True)
 for p in [*case.glob("*.json"),*case.glob("*.txt"),*case.glob("host-*.stdout"),*case.glob("host-*.stderr")]:
  dest=saved/p.name
  if dest.exists():assert dest.read_bytes()==p.read_bytes(),dest
  else:dest.write_bytes(p.read_bytes())
processes=subprocess.check_output(["ps","-axo","pid=,ppid=,command="],text=True);owned=[]
for line in processes.splitlines():
 parts=line.strip().split(None,2)
 if len(parts)==3 and Path(parts[2].split(None,1)[0]).name in ["kbox","kbox-shim","koci","koci-kshim","koci-agent"] and any(str(p) in parts[2] for p in [root,private]):owned.append(line)
assert not owned,owned
mounts=subprocess.check_output(["/sbin/mount"],text=True);assert all(str(p) not in mounts for p in [root,private])
images=plistlib.loads(subprocess.check_output(["/usr/bin/hdiutil","info","-plist"]));assert all(not any(str(p) in json.dumps(image) for p in [root,private]) for image in images.get("images",[]))
(checks/"final-platform-resources2.json").write_text(json.dumps(dict(globalProcessRows=len(processes.splitlines()),ownedProcesses=owned,ownedMounts=[line for line in mounts.splitlines() if any(str(p) in line for p in [root,private])],ownedImages=[image for image in images.get("images",[]) if any(str(p) in json.dumps(image) for p in [root,private])]),indent=2)+"\n")
ids=set();sockets=set()
for base in [private,root/"af-final1"]:
 for p in base.rglob("*"):
  if not p.is_file() or p.suffix not in [".json",".stdout",".stderr",".log"] or p.stat().st_size>8*1024*1024:continue
  data=p.read_text(errors="replace");ids.update(re.findall(r"/(?:private/)?tmp/kbox-sockets/([a-z0-9-]{36})/",data));ids.update(re.findall(r"/(?:home|k-home)/boxes/([a-z0-9-]{36})/",data));sockets.update(re.findall(r"/private/tmp/(?:ko-pool-[a-zA-Z0-9_-]+|kbox-warm-[a-zA-Z0-9_-]+)/[^\s\"\\,]+\.sock",data))
for identity in ids:
 directory=Path("/private/tmp/kbox-sockets")/identity;assert not directory.exists(),directory
 sockets.update(str(directory/name) for name in ["exec.sock","pty.sock","attest.sock","port-forward.sock"])
assert not [p for p in sockets if Path(p).exists()]
for label,base in [("artifacts",root),("private",private)]:
 assert not [p for p in base.rglob("*") if p.is_socket()]
 cmd=["/usr/sbin/lsof","+D",str(base)];r=subprocess.run(cmd,capture_output=True,text=True)
 (checks/("final-fd-"+label+".json")).write_text(json.dumps(dict(command=cmd,exit=r.returncode,stdout=r.stdout,stderr=r.stderr),indent=2)+"\n")
 assert r.returncode==1 and not r.stdout,r.stdout
caches=[]
for label in ["native","af"]:
 before=json.loads((checks/(label+"-image-cache-before.json")).read_text());store=root/(label+"-image-cache");compared=[];catalog=[]
 for name,v in before.items():
  p=store/name;assert p.is_file(),p;st=p.stat();assert (st.st_uid,st.st_gid,st.st_mode&0o777)==(v["uid"],v["gid"],v["mode"]),p
  h=sha(p)
  if name=="index.json":catalog.append(dict(path=name,before=v["sha256"],after=h))
  else:assert h==v["sha256"],p;compared.append(name)
 caches.append(dict(store=str(store),immutableFilesCompared=len(compared),mutableCatalog=catalog))
main=private/"vm-sizing-attempt-2";source=main/"caller-files-main"
for name,expected in [("只读 input",b"live host main generation 3 changed length"),("empty rw",b"native-writeback"),("canary",b"private-sibling")]:
 p=source/name;assert p.read_bytes()==expected,p
 if name!="canary":assert p.stat().st_nlink==1 and p.stat().st_mode&0o777==0o666,p
assert (main/"readonly/input").read_bytes()==b"caller-owned"
assert (main/"readonly/replacement").read_bytes()==b"replacement"
assert (main/"ungranted").read_bytes()==b"outside-caller-grants"
for p in private.glob("vm-sizing-old-shim-attempt-*/compat-entry"):assert p.is_dir() and not list(p.iterdir()),p
for key,expected in [("boxArchive","619cf47eeb958eb918f4b21ab7ea19a68af515a80b8f17b3ddbcd3ef8ca19496"),("ociArchive","f014049494a8f9f3a16f7f40ceed49f92d73160e6093d6087f8e046a25205423")]:assert sha(Path(c["prior"][key]))==expected
summary=dict(status="PASS",utc=time.strftime("%Y-%m-%dT%H:%M:%SZ",time.gmtime()),platform=platform.platform(),sourceInputsSha256=current["sourceInputsSha256"],compiledInputCount=len(current["files"]),homes=len(homes),managedExecutions=0,managedVolumes=0,ownedProcesses=[],ownedMounts=[],ownedFdHolders=[],observedSocketDirectories=len(ids),observedSocketPaths=len(sockets),remainingSockets=[],caches=caches,qualifiedRustNativeTests=6,qualifiedPythonNativeScenarios=3,hardwareReadbacks=12,separateMaintenanceRustTests=2,separateMaintenanceSdkScenarios=2,priorArchivesRetainedUnchanged=True,callerSourcesRetained=True,runtimeMemoryMeasured=False,resourcesDeletedByAudit=[],scope="Current phase only; prior unknown-actor seven-RAM preservation failure remains open")
(checks/"final-cleanup.json").write_text(json.dumps(summary,indent=2)+"\n");print(json.dumps(summary,indent=2))
