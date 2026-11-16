from pathlib import Path
import hashlib,subprocess

def snapshot():
    paths=sorted(set(subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard'],text=True).splitlines()))
    files={}; digest=hashlib.sha256()
    for name in paths:
        path=Path(name)
        if path.is_file() and (name in ['Cargo.toml','Cargo.lock','rust-toolchain.toml'] or name.startswith('.cargo/') or name.startswith('crates/') and path.suffix in ['.rs','.toml','.proto','.plist'] or name.startswith('scripts/package-') or name=='scripts/install-package.sh'):
            data=path.read_bytes(); files[name]=hashlib.sha256(data).hexdigest(); digest.update(name.encode()+b'\0'+data+b'\0')
    return dict(sourceInputsSha256=digest.hexdigest(),files=files,sourceCommit=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip())
