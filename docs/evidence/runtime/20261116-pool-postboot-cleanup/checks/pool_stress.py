from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import hashlib, json, os, shutil, subprocess, sys, time

c=json.loads(Path(sys.argv[1]).read_text())
root=Path(c['root'])/('pool-wave-'+sys.argv[2]);root.mkdir(mode=0o700)
checks=Path(c['evidence'])/'checks'/root.name;checks.mkdir()
prefix=Path(c['boxPrefix']);cli=str(prefix/'bin/kbox')
sockets=Path('/private/tmp')/('kbox-warm-'+Path(c['root']).name[-12:]+'-'+sys.argv[2]);sockets.mkdir(mode=0o700)
(sockets/'.owned-kbox-warm-test').write_text(str(root)+'\n')
jobs=[];snapshots={};events=[]
def call(job,args,timeout=30):
    command=[cli,*args];start=time.monotonic()
    p=subprocess.run(command,env=job['env'],cwd=root,capture_output=True,text=True,timeout=timeout)
    events.append(dict(command=command,exit=p.returncode,seconds=time.monotonic()-start,stdout=p.stdout,stderr=p.stderr))
    assert p.returncode==0,(command,p.stderr)
    return p.stdout
def snapshot():
    for job in jobs:
        for path in (job['home']/'boxes').glob('*/logs/*.log'):
            try:data=path.read_bytes()[:1024*1024]
            except FileNotFoundError:continue
            digest=hashlib.sha256(data).hexdigest();key=str(path)+'@'+digest
            if key not in snapshots:
                target=checks/'owned-box-logs'/path.relative_to(root);target.parent.mkdir(parents=True,exist_ok=True)
                target=target.with_name(target.name+'.'+digest[:12]);target.write_bytes(data)
                snapshots[key]=dict(source=str(path),saved=str(target),sha256=digest,bytes=len(data))
try:
    for n in range(4):
        home=root/str(n)/'home';(home/'bin').mkdir(parents=True,mode=0o700);home.chmod(0o700);(home/'lib').mkdir()
        for name in ('kbox-shim','kbox-guest-init'):shutil.copy2(prefix/'bin'/name,home/'bin'/name)
        for lib in (prefix/'lib').iterdir():(home/'lib'/lib.name).symlink_to(lib.resolve())
        (home/'images').symlink_to(c['imageStore'])
        env={k:v for k,v in os.environ.items() if not k.startswith(('DYLD_','K_BOX_','KBOX_')) and k!='K_HOME'}
        env.update(K_HOME=str(home),PATH=str(prefix/'bin')+':/usr/bin:/bin:/usr/sbin:/sbin',DYLD_LIBRARY_PATH='',DYLD_FALLBACK_LIBRARY_PATH='/usr/lib',RUST_LOG='kbox_runtime=debug,kbox_shim=debug')
        out=(checks/f'{n}-daemon.stdout').open('w');err=(checks/f'{n}-daemon.stderr').open('w');sock=str(sockets/f'{n}.sock')
        command=[cli,'pool','start','--image',c['baseImage'],'--size','4','--max','4','--boot-concurrency','2','--ttl','0','--socket',sock,'--json']
        p=subprocess.Popen(command,cwd=root,env=env,stdout=out,stderr=err)
        jobs.append(dict(home=home,env=env,process=p,stdout=out,stderr=err,socket=sock,command=command))
    deadline=time.monotonic()+180;statuses={}
    while time.monotonic()<deadline:
        snapshot()
        for n,job in enumerate(jobs):
            assert job['process'].poll() is None,('daemon exited',n,job['process'].poll())
            statuses[n]=json.loads(call(job,['pool','status','--socket',job['socket'],'--json']))
        if all(len(statuses[n])==1 and statuses[n][0]['idle']==4 for n in range(4)):break
        time.sleep(0.2)
    else:raise TimeoutError(('four-daemon full warm fill did not complete',statuses))
    (checks/'filled-pools.json').write_text(json.dumps(statuses,indent=2)+'\n')
    def run_one(pair):
        n,job=pair;expected=f'pool-native-{n}'
        value=call(job,['pool','run','--socket',job['socket'],'--','python3','-c',f'print({expected!r})'])
        assert value.strip()==expected,(n,value)
        return dict(case=n,stdout=value)
    with ThreadPoolExecutor(max_workers=4) as executor:results=list(executor.map(run_one,enumerate(jobs)))
    (checks/'native-results.json').write_text(json.dumps(dict(status='PASS',daemons=4,initialReadyVms=16,commands=results),indent=2)+'\n')
    print('PASS:4 daemons,16 authenticated idle VMs,4 real Guest commands',flush=True)
finally:
    snapshot()
    def stop(job):
        try:call(job,['pool','stop','--socket',job['socket'],'--json'],timeout=90)
        finally:
            try:job['process'].wait(timeout=15)
            except subprocess.TimeoutExpired:job['process'].terminate();job['process'].wait(timeout=10)
            job['stdout'].close();job['stderr'].close()
    with ThreadPoolExecutor(max_workers=4) as executor:list(executor.map(stop,jobs))
    (checks/'commands.json').write_text(json.dumps(events,indent=2)+'\n')
    (checks/'owned-log-snapshots.json').write_text(json.dumps(list(snapshots.values()),indent=2)+'\n')
    listing=subprocess.check_output(['ps','-ax','-o','pid=,ppid=,command='],text=True)
    remaining=[line for line in listing.splitlines() if str(root) in line and ('kbox-shim --config' in line or 'kbox pool start' in line)]
    assert not remaining,remaining
    for job in jobs:
        assert json.loads(call(job,['ps','-a','--format','json']))==[]
        assert not Path(job['socket']).exists(),job['socket']
    (checks/'cleanup.json').write_text(json.dumps(dict(status='PASS',homes=len(jobs),ownedProcesses=remaining,existingPoolSockets=[],managedExecutions=[]),indent=2)+'\n')
