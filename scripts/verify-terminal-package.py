#!/usr/bin/env python3
"""Verify archive integrity and installation in an isolated temporary directory."""
import hashlib
import os
import pathlib
import subprocess
import sys
import tarfile
import tempfile
import zipfile

archives = list(pathlib.Path(sys.argv[1]).glob('*.tar.gz')) + list(pathlib.Path(sys.argv[1]).glob('*.zip'))
if len(archives) != 1:
    raise SystemExit('Expected one native archive')
archive = archives[0]
expected, name = pathlib.Path(str(archive) + '.sha256').read_text().strip().split('  ')
assert name == archive.name and hashlib.sha256(archive.read_bytes()).hexdigest() == expected
with tempfile.TemporaryDirectory(prefix='overseer-install-') as tmp:
    root = pathlib.Path(tmp)
    if archive.suffix == '.zip':
        with zipfile.ZipFile(archive) as z:
            assert all(not pathlib.PurePosixPath(n).is_absolute() and '..' not in pathlib.PurePosixPath(n).parts for n in z.namelist())
            z.extractall(root)
    else:
        with tarfile.open(archive) as t:
            t.extractall(root, filter='data')
    stage = next(f for f in root.iterdir() if f.is_dir())
    install = root / 'installed'
    environment = dict(os.environ, OVERSEER_INSTALL_DIR=str(install), OVERSEER_DATA_DIR=str(root / 'data'))
    if os.name == 'nt':
        command = ['pwsh', '-NoProfile', '-File', str(stage / 'install-terminal.ps1'), '-InstallDir', str(install)]
        binary = install / 'overseer-tui.exe'
    else:
        command = ['sh', str(stage / 'install-terminal.sh')]
        binary = install / 'overseer-tui'
    subprocess.run(command, env=environment, check=True)
    subprocess.run(command, env=environment, check=True)  # upgrade/reinstall
    subprocess.run([str(binary), '--version'], env=environment, check=True)
    subprocess.run([str(binary), '--help'], env=environment, check=True)
    for args in [['--invalid'], ['--remote', '--url'], ['--url', 'http://localhost:8080'], ['--remote', '--url', 'file:///tmp/x']]:
        assert subprocess.run([str(binary), *args], env=environment, capture_output=True).returncode != 0
    assert not (root / 'data').exists(), 'CLI metadata/error commands must not initialize user data'
    # Corrupted downloads must fail before replacing an existing installation.
    source = stage / binary.name
    source.write_bytes(source.read_bytes() + b'corrupt')
    installed_digest = hashlib.sha256(binary.read_bytes()).hexdigest()
    assert subprocess.run(command, env=environment, capture_output=True).returncode != 0
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == installed_digest
    binary.unlink()  # user-level uninstall
    assert not binary.exists()
print('Passed archive checksum, install, reinstall, CLI metadata/errors, corruption rejection, uninstall.')
