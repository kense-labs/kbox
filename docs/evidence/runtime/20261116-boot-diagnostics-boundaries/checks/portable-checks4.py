from pathlib import Path
import json,subprocess
R=Path(__file__).resolve().parent; E=Path('docs/evidence/runtime/20261116-boot-diagnostics-boundaries/checks').resolve(); source=R/'portable-diagnostics.rs'; rows=[]
targets=['x86_64-pc-windows-msvc']
cmd=['cargo','check','--locked','--offline','-p','tracing','--message-format=json']
for target in targets:cmd+=['--target',target]
r=subprocess.run(cmd,capture_output=True,text=True); (E/'portable-tracing-artifacts4.stdout').write_text(r.stdout); (E/'portable-tracing-artifacts4.stderr').write_text(r.stderr); assert r.returncode==0
artifacts=[json.loads(line) for line in r.stdout.splitlines() if line.startswith('{')]
for target in targets:
 deps=Path('target')/target/'debug/deps'; cmd=['rustc','--edition=2021','--crate-name','boot_diag_portable','--crate-type','lib','--target',target,'--emit=metadata',str(source),'-L','dependency='+str(deps),'-L','dependency=target/debug/deps','-o',str(R/('boot-diag-'+target+'.rmeta'))]
 for name in ['cap_std','cap_fs_ext','serde','serde_json','uuid','tracing']:
  files=list(deps.glob('lib'+name+'-*.rmeta'))
  if name=='tracing':files=[Path(p) for a in artifacts if a.get('reason')=='compiler-artifact' and a['target']['name']=='tracing' for p in a['filenames'] if '/'+target+'/' in p and p.endswith('.rmeta')]
  assert len(files)==1,(target,name,files);cmd+=['--extern',name+'='+str(files[0])]
 r=subprocess.run(cmd,capture_output=True,text=True); (E/('portable4-'+target+'.stdout')).write_text(r.stdout); (E/('portable4-'+target+'.stderr')).write_text(r.stderr); rows.append(dict(command=cmd,exit=r.returncode,target=target,nativeRuntimeClaim=False))
(E/'portable-windows4.json').write_text(json.dumps(rows,indent=2)+'\n');print(json.dumps([dict(target=r['target'],exit=r['exit']) for r in rows]));assert all(row['exit']==0 for row in rows)
