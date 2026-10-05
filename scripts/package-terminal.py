#!/usr/bin/env python3
"""Package a native release binary, without publishing or touching user data."""
import argparse
import hashlib
import json
import pathlib
import re
import shutil
import subprocess
import tarfile
import tempfile
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
TARGETS = ('aarch64-apple-darwin', 'x86_64-apple-darwin', 'x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc')

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--target', choices=TARGETS, required=True)
    p.add_argument('--binary', type=pathlib.Path, required=True)
    p.add_argument('--output', type=pathlib.Path, default=ROOT / 'dist/terminal')
    args = p.parse_args()
    version = re.search(r'^version = "([^"]+)"', (ROOT / 'Cargo.toml').read_text(), re.M)[1]
    windows = 'windows' in args.target
    name = 'overseer-tui.exe' if windows else 'overseer-tui'
    # Do not silently produce an archive containing a stale/wrong executable.
    actual = subprocess.check_output([str(args.binary.resolve()), '--version'], text=True).strip()
    if actual != f'overseer-tui {version}':
        raise SystemExit(f'Build/version mismatch: {actual!r}')
    args.output.mkdir(parents=True, exist_ok=True)
    stem = f'overseer-terminal-v{version}-{args.target}'
    with tempfile.TemporaryDirectory() as tmp:
        stage = pathlib.Path(tmp) / stem
        stage.mkdir()
        shutil.copy2(args.binary, stage / name)
        (stage / name).chmod(0o755)
        shutil.copy2(ROOT / 'packaging/terminal/README.txt', stage / 'README.txt')
        shutil.copy2(ROOT / 'crates/overseer-client/LICENSE', stage / 'LICENSE')
        installer = 'install-terminal.ps1' if windows else 'install-terminal.sh'
        shutil.copy2(ROOT / 'scripts' / installer, stage / installer)
        (stage / 'TARGET').write_text(args.target + '\n')
        (stage / 'CHECKSUMS.txt').write_text(''.join(f'{digest(f)}  {f.name}\n' for f in sorted(stage.iterdir())))
        archive = args.output / (stem + ('.zip' if windows else '.tar.gz'))
        if windows:
            with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as z:
                for f in stage.iterdir():
                    z.write(f, f'{stem}/{f.name}')
        else:
            with tarfile.open(archive, 'w:gz') as t:
                t.add(stage, arcname=stem)
    (args.output / (archive.name + '.sha256')).write_text(f'{digest(archive)}  {archive.name}\n')
    print(json.dumps({'archive': str(archive), 'sha256': digest(archive), 'bytes': archive.stat().st_size}))

if __name__ == '__main__':
    main()
