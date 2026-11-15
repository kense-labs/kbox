from pathlib import Path
import hashlib,json,os,pty,select,subprocess,sys,time
binary=Path(sys.argv[1]).resolve();owned=Path(sys.argv[2]).resolve();identity=sys.argv[3];root=Path(sys.argv[4]).resolve()
assert (owned/'.kbox-hvf-native').read_text()=='owned-hvf-native-test\n'
assert root.is_relative_to(owned);root.mkdir();rows=[]
environment=os.environ.copy();environment.update(K_HOME=str(owned/'home'),K_BOX_OCI_HVF_ENDPOINT=str(owned/'service/runtime.sock'),K_BOX_OCI_MIGRATION='off')
for name,args,input_data in [('exec',['exec','-t',identity,'--','/shm-probe','1048576','0'],None),('attach',['attach','-it',identity],b'/shm-probe 1048576 0; exit\n')]:
 command=[str(binary),*args];master,slave=pty.openpty();chunks=[];process=None;deadline=time.monotonic()+90
 try:
  with (root/(name+'.stderr')).open('wb') as errors:process=subprocess.Popen(command,stdin=slave,stdout=slave,stderr=errors,env=environment)
  os.close(slave);slave=-1
  if input_data is not None:os.write(master,input_data)
  while process.poll() is None:
   assert time.monotonic()<deadline,'owned PTY deadline'
   if select.select([master],[],[],.1)[0]:
    try:chunks.append(os.read(master,65536))
    except OSError:break
   assert sum(map(len,chunks))<=1024*1024
  status=process.wait(timeout=10)
  while select.select([master],[],[],0)[0]:
   try:
    data=os.read(master,65536)
    if not data:break
    chunks.append(data)
   except OSError:break
 finally:
  if process is not None and process.poll() is None:process.kill();process.wait(timeout=10)
  if slave>=0:os.close(slave)
  os.close(master)
 output=b''.join(chunks);assert len(output)<=1024*1024;(root/(name+'.stdout')).write_bytes(output)
 assert status==0,(command,status,output,(root/(name+'.stderr')).read_text())
 probes=[]
 for line in output.decode().splitlines():
  if line.startswith('{'):probes.append(json.loads(line))
 assert len(probes)==1,(command,output)
 probe=probes[0];assert probe['status']=='PASS' and probe['uid']==0 and probe['tty']
 assert probe['bytes']==1048576 and probe['blockBytes']==4096 and probe['mode']==1023
 assert probe['emptyCapabilities'] and probe['noNewPrivileges'] and probe['nosuidNodevNoexec']
 assert probe['posixSharedMappingAndUnlinkedFdLifetime'] and probe['noexecErrno']==13
 assert probe['enospcErrno']==28 and 524288<probe['writtenBytes']<=1048576 and probe['reuseSucceeded']
 assert probe['childReaped'] and probe['threadJoined'] and probe['ownedObjectsRemoved']
 rows.append(dict(case=name,command=command,exitCode=status,probe=probe))
(root/'result.json').write_text(json.dumps(dict(status='PASS',binary=str(binary),sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),cases=rows),indent=2)+'\n')
print('Real CLI exec PTY and attach passed with exact shared-memory capacity and actual POSIX/ENOSPC effects and policy off.')
