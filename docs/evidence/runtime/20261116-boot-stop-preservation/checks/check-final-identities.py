from source_snapshot import snapshot
from pathlib import Path
import hashlib,json,os,subprocess
context=Path('docs/evidence/runtime/20261116-boot-stop-preservation/context.json');c=json.loads(context.read_text());root=Path(c['root']);checks=Path(c['evidence'])/'checks';frozen=json.loads((root/'source-frozen-1.json').read_text());current=snapshot();assert current['files']==frozen['files'] and current['sourceInputsSha256']==frozen['sourceInputsSha256'];box=Path(c['boxPrefix']);oci=Path(c['ociPrefix']);rows=[]
def run(cmd,required=True):
 p=subprocess.run(cmd,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True);rows.append(dict(command=cmd,exitCode=p.returncode,output=p.stdout));
 if required:assert p.returncode==0,rows[-1]
 return p.stdout
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for b in iter(lambda:f.read(1024*1024),b''):h.update(b)
 return h.hexdigest()
for prefix in [box,oci]:
 for line in (prefix/'SHA256SUMS').read_text().splitlines():
  h,name=line.split('  ',1);assert sha(prefix/name)==h,name
 assert json.loads((prefix/'build-info.json').read_text())['sourceInputsSha256']==(frozen['sourceInputsSha256'] if prefix==box else c['ociOriginalSourceInputsSha256'])
for p in [box/'bin/kbox',box/'bin/kbox-shim',oci/'koci',oci/'koci-kshim']:
 assert 'arm64' in run(['file',str(p)]);run(['codesign','--verify','--strict',str(p)]);run(['codesign','-dv','--verbose=4',str(p)]);run(['otool','-L',str(p)])
for p in [box/'bin/kbox-guest-init',oci/'koci-agent']:
 assert 'aarch64' in run(['file',str(p)])
for prefix,subdir in [(box,'lib'),(oci,'koci-kbox-runtime')]:
 for p in sorted((prefix/subdir).iterdir()):
  if p.is_file() and p.suffix=='.dylib':
   assert 'arm64' in run(['file',str(p)]);run(['codesign','--verify','--strict',str(p)],False);run(['otool','-L',str(p)])
image=oci/'system-image/koci-system.ext4';agent=root/'final-system-agent-1';run(['/opt/homebrew/opt/e2fsprogs/sbin/debugfs','-R','dump /usr/bin/koci-agent '+str(agent),str(image)]);assert sha(agent)==sha(oci/'koci-agent')
info=dict(sourceInputsSha256=frozen['sourceInputsSha256'],ociOriginalSourceInputsSha256=c['ociOriginalSourceInputsSha256'],boxPrefix=str(box),ociPrefix=str(oci),files={str(p):dict(bytes=p.stat().st_size,sha256=sha(p)) for p in [box/'bin/kbox',box/'bin/kbox-shim',box/'bin/kbox-guest-init',box/'lib/libkrun.1.17.0.dylib',box/'lib/libkrunfw.5.dylib',oci/'koci',oci/'koci-kshim',oci/'koci-agent',oci/'koci-kbox-runtime/libkrun.1.17.0.dylib',oci/'koci-kbox-runtime/libkrunfw.5.dylib',image,oci/'system-image/system-image.json']},embeddedAgentMatchesStandalone=True,commands=rows,runtimeMemory='not measured')
assert not (checks/'final-identities-1.json').exists()
(checks/'final-identities-1.json').write_text(json.dumps(info,indent=2)+'\n');print('Verified final archive files, source provenance, native signatures/architectures and embedded Agent')
