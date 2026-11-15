from pathlib import Path
import json,os,re,signal,subprocess,time
E=Path("docs/evidence/runtime/20261116-oci-vm-sizing").resolve();c=json.loads((E/"context.json").read_text());checks=E/"checks";prefix=Path(c["ociPrefix"])
rows=[]
for label in ["vm-sizing-attempt-1","vm-sizing-old-shim-attempt-1"]:
 root=Path(c["privateRoot"])/label
 assert root.resolve()==root and (root/".kbox-hvf-native").read_text()=="owned-hvf-native-test\n"
 assert not (root/"compat-entry/entered").exists()
 records=list((root/"service/state/containers").glob("*/record.json"));assert len(records)==1
 record=json.loads(records[0].read_text())["record"]
 assert record["generation"]==1 and record["driver"]=="libk-hvf" and record["isolation"]=="dedicated-vm"
 starts=lambda:sum(json.loads(p.read_text()).get("kind")=="start" for p in (root/"service/state/operations").glob("*.json"))
 before_starts=starts()
 env={k:v for k,v in os.environ.items() if not k.startswith(("DYLD_","K_BOX_")) and k!="K_HOME"}
 env.update(K_HOME=str(root/"home"),DYLD_LIBRARY_PATH="",PATH="/usr/bin:/bin:/usr/sbin:/sbin",K_BOX_TEST_HVF_ROOT=str(root),K_BOX_TEST_HVF_HOST=str(prefix/"koci"),K_BOX_TEST_HVF_SHIM=str(prefix/"koci-kshim"),K_BOX_TEST_HVF_MANIFEST=c["systemImageManifest"])
 hostcmd=[str(prefix/"koci"),"macos-hvf-host-service","--root",str(root/"service"),"--shim",str(prefix/"koci-kshim"),"--system-image-manifest",c["systemImageManifest"]]
 receipt=dict(root=str(root),recordBefore=record,hostCommand=hostcmd,workloadStartsBefore=before_starts)
 out=checks/("reconcile-sdk2-"+label+".stdout");err=checks/("reconcile-sdk2-"+label+".stderr")
 with out.open("w") as stdout,err.open("w") as stderr:
  host=subprocess.Popen(hostcmd,env=env,stdout=stdout,stderr=stderr)
  try:
   deadline=time.monotonic()+20
   while not (root/"service/runtime.sock").exists():
    assert host.poll() is None and time.monotonic()<deadline
    time.sleep(.1)
   cmd=[str(Path(c["root"])/"owned-sdk-delete"),str(root/"service/runtime.sock"),record["state"]["id"],record["config_digest"],str(checks/("reconcile-sdk2-"+label+".json"))]
   r=subprocess.run(cmd,env=env,capture_output=True,text=True)
   receipt["sdkDelete"]=dict(command=cmd,exit=r.returncode,stdout=r.stdout,stderr=r.stderr)
  finally:
   if host.poll() is None:host.send_signal(signal.SIGINT)
   receipt["hostExit"]=host.wait(timeout=30)
   receipt["socketAbsentAfterSdkDeleteShutdown"]=not (root/"service/runtime.sock").exists()
 (checks/("reconcile-launch2-"+label+".json")).write_text(json.dumps(receipt,indent=2)+"\n")
 assert receipt["sdkDelete"]["exit"]==0,receipt
 assert receipt["hostExit"]==0 and receipt["socketAbsentAfterSdkDeleteShutdown"]
 cmd=[c["nativeClient"],"--ignored","--exact","local_execution::oci_hvf::tests::native::native_cleanup_owned_hvf_test_root","--nocapture"]
 assert cmd[3]+": test" in subprocess.check_output([c["nativeClient"],"--list"],text=True).splitlines()
 with (checks/("reconcile-box2-"+label+".stdout")).open("w") as out,(checks/("reconcile-box2-"+label+".stderr")).open("w") as err:r=subprocess.run(cmd,env=env,stdout=out,stderr=err)
 text=(checks/("reconcile-box2-"+label+".stdout")).read_text();counts=re.findall(r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;",text)
 row=dict(root=str(root),command=cmd,exit=r.returncode,actualCounts=counts,workloadStartsBefore=before_starts,workloadStartsAfter=starts(),workloadMarkerAbsent=not (root/"compat-entry/entered").exists(),boxCleanupExists=(root/"cleanup-boxes.json").is_file())
 rows.append(row);(checks/"failed-candidates-reconciled2.json").write_text(json.dumps(rows,indent=2)+"\n")
 assert r.returncode==0 and counts==[("1","0","0")],row
 assert starts()==before_starts and row["workloadMarkerAbsent"] and json.loads((root/"cleanup-boxes.json").read_text())==[]
 print(json.dumps(row),flush=True)
