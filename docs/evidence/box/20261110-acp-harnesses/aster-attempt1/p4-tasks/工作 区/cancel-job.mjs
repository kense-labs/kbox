import {spawn} from 'node:child_process';
import {writeFileSync} from 'node:fs';
const child=spawn(process.execPath,['-e',"setTimeout(()=>require('fs').writeFileSync('/workspace/out/late-write.txt','late'),45000)"],{stdio:'ignore'});
writeFileSync('/workspace/out/cancel-started.json',JSON.stringify({pid:process.pid,childPid:child.pid,at:Date.now()}));
setTimeout(()=>writeFileSync('/workspace/out/late-parent.txt','late'),180000);
