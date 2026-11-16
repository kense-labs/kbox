from pathlib import Path
import json,subprocess,sys,time
repo=Path(__file__).resolve().parents[2]
evidence=repo/'docs/evidence/runtime/20261116-verify-home-isolation/checks'
name=sys.argv[1];command=sys.argv[2:]
assert name and command and not (evidence/(name+'.json')).exists()
start=time.monotonic()
with (evidence/(name+'.log')).open('wb') as out,(evidence/(name+'.stderr')).open('wb') as err:
    result=subprocess.run(command,cwd=repo,stdout=out,stderr=err)
row=dict(command=command,exit=result.returncode,seconds=time.monotonic()-start)
receipt=evidence/(name+'.json')
if receipt.exists(): receipt=evidence/(name+'-command.json')
assert not receipt.exists(), 'refusing to overwrite a child result or command receipt'
receipt.write_text(json.dumps(row,indent=2)+'\n')
print(json.dumps(row),flush=True)
print((evidence/(name+'.log')).read_text()[-700:])
if result.returncode: print((evidence/(name+'.stderr')).read_text()[-9000:])
raise SystemExit(result.returncode)
