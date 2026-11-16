from source_snapshot import snapshot
from pathlib import Path
import hashlib,json,os,shutil,subprocess,time
context=Path('docs/evidence/runtime/20261116-boot-diagnostics-boundaries/context.json');c=json.loads(context.read_text());root=Path(c['root']);checks=Path(c['evidence'])/'checks';frozen=json.loads((root/'source-frozen-1.json').read_text());current=snapshot();assert current['files']==frozen['files'] and current['sourceInputsSha256']==frozen['sourceInputsSha256']
env=os.environ.copy()
for key in list(env):
 if key.startswith(('DYLD_','K_BOX_TEST_','K_BOX_OCI_')) or key in ['K_HOME','K_LIBKRUN_PATH','K_LIBKRUNFW_PATH','LIBKRUN_NO_FSYNC']:env.pop(key)
env['CARGO_NET_OFFLINE']='true';env.update(K_HOME=str(root/'package-home-bootdiag-final1'),K_BUILD_LIBKRUN='1');cmd=['bash','scripts/package-macos-arm64.sh'];start=time.monotonic()
with (checks/'package-bootdiag-final1.log').open('w') as out:r=subprocess.run(cmd,stdout=out,stderr=subprocess.STDOUT,env=env)
(checks/'package-bootdiag-final1.json').write_text(json.dumps(dict(command=cmd,exitCode=r.returncode,seconds=time.monotonic()-start,environment=dict(K_HOME=env['K_HOME'],K_BUILD_LIBKRUN='1'),sourceInputsSha256=frozen['sourceInputsSha256']),indent=2)+'\n');assert r.returncode==0
current=snapshot();assert current['files']==frozen['files'] and current['sourceInputsSha256']==frozen['sourceInputsSha256'],'compiled source drifted during Box build';subprocess.run(['git','merge-base','--is-ancestor',frozen['sourceCommit'],current['sourceCommit']],check=True)
stem='kbox-3.4.3-macos-arm64';dist=root/'final-dist-bootdiag-1';dist.mkdir()
for name in [stem+'.tar.gz',stem+'.tar.gz.sha256']:shutil.copy2(Path('dist')/name,dist/name)
archive=dist/(stem+'.tar.gz');sha=hashlib.sha256(archive.read_bytes()).hexdigest();assert (dist/(stem+'.tar.gz.sha256')).read_text().split()[0]==sha
unpack=root/'unpacked-bootdiag-final-1';unpack.mkdir();extract=['tar','-xzf',str(archive),'-C',str(unpack)];subprocess.run(extract,check=True)
prefix=root/'installed-bootdiag-final-1';assert not prefix.exists();install=['sh',str(unpack/stem/'install.sh')];install_env=dict(env,KBOX_PREFIX=str(prefix));start=time.monotonic()
with (checks/'install-bootdiag-final1.log').open('w') as out:r=subprocess.run(install,stdout=out,stderr=subprocess.STDOUT,env=install_env)
assert r.returncode==0
lines=(prefix/'SHA256SUMS').read_text().splitlines()
for line in lines:
 digest,name=line.split('  ',1);assert hashlib.sha256((prefix/name).read_bytes()).hexdigest()==digest,name
build_info=json.loads((prefix/'build-info.json').read_text());assert build_info['sourceInputsSha256']==frozen['sourceInputsSha256']
rec=dict(command=install,extractCommand=extract,exitCode=r.returncode,seconds=time.monotonic()-start,prefix=str(prefix),archive=str(archive),sha256=sha,compressedBytes=archive.stat().st_size,installedDiskBytes=int(subprocess.check_output(['du','-sk',str(prefix)],text=True).split()[0])*1024,filesVerified=len(lines),buildInfo=build_info)
(checks/'install-bootdiag-final1.json').write_text(json.dumps(rec,indent=2)+'\n');c=json.loads(context.read_text());c.update(boxPrefix=str(prefix),installedPrefix=str(prefix),finalBoxPrefix=str(prefix),boxArchive=str(archive));context.write_text(json.dumps(c,indent=2)+'\n');print(json.dumps(rec),flush=True)
