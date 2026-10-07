#!/usr/bin/env python3
"""Package an existing ad hoc signed Apple Silicon app as a local preview.

Does not build, publish, notarize, change signatures, or disable Gatekeeper.
"""
import argparse
import hashlib
import json
import pathlib
import plistlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--app', type=pathlib.Path, default=ROOT / 'desktop-ui/src-tauri/target/release/bundle/macos/Overseer.app')
    parser.add_argument('--output', type=pathlib.Path, default=ROOT / 'dist/desktop-preview')
    args = parser.parse_args()
    if sys.platform != 'darwin':
        parser.error('Run on macOS; packaging requires codesign, lipo and ditto.')
    app = args.app.resolve()
    with (app / 'Contents/Info.plist').open('rb') as handle:
        metadata = plistlib.load(handle)
    if metadata['CFBundleIdentifier'] != 'co.tz.skyconnect.overseer':
        raise SystemExit('Unexpected app identifier')
    version = metadata['CFBundleShortVersionString']
    if not version or any(c not in '0123456789.-' for c in version):
        raise SystemExit('Unexpected bundle version')
    binaries = [app / 'Contents/MacOS' / metadata['CFBundleExecutable'], app / 'Contents/Resources/backend/overseer']
    for binary in binaries:
        if not binary.is_file():
            raise SystemExit(f'Missing executable: {binary}')
        arches = subprocess.check_output(['lipo', '-archs', str(binary)], text=True).strip()
        if arches != 'arm64':
            raise SystemExit(f'Expected Apple Silicon binary, found {arches}: {binary}')
        subprocess.run(['codesign', '--verify', '--strict', str(binary)], check=True)
    subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)
    signature = subprocess.run(['codesign', '-dv', str(app)], check=True, capture_output=True, text=True).stderr
    if 'Signature=adhoc' not in signature:
        raise SystemExit('This packager is only for ad hoc signed previews')
    args.output.mkdir(parents=True, exist_ok=True)
    stem = f'overseer-desktop-v{version}-macos-arm64-preview'
    archive = args.output.resolve() / f'{stem}.zip'
    with tempfile.TemporaryDirectory(prefix='overseer-preview-') as tmp:
        stage = pathlib.Path(tmp) / stem
        stage.mkdir()
        subprocess.run(['ditto', str(app), str(stage / 'Overseer.app')], check=True)
        (stage / 'README.txt').write_text((ROOT / 'packaging/desktop/README.txt').read_text())
        subprocess.run(['ditto', '-c', '-k', '--sequesterRsrc', '--keepParent', str(stage), str(archive)], check=True)
        # Verify the archive round trip preserves both executable signatures.
        extracted = pathlib.Path(tmp) / 'extracted'
        subprocess.run(['ditto', '-x', '-k', str(archive), str(extracted)], check=True)
        restored = extracted / stem / 'Overseer.app'
        subprocess.run(['codesign', '--verify', '--deep', '--strict', str(restored)], check=True)
        for binary in binaries:
            subprocess.run(['codesign', '--verify', '--strict', str(restored / binary.relative_to(app))], check=True)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_suffix('.zip.sha256').write_text(f'{digest}  {archive.name}\n')
    report = {
        'archive': archive.name, 'sha256': digest, 'bytes': archive.stat().st_size,
        'desktop_version': version, 'platform': 'macOS Apple Silicon',
        'channel': 'preview', 'signature': 'ad hoc; no Developer ID',
        'notarized': False, 'published': False,
        'checks': ['app and bundled backend are arm64', 'code signatures verify before and after ZIP extraction'],
        'limitations': ['Clean-machine downloaded installation has not been verified', 'Packaging is not functional acceptance testing'],
    }
    (args.output / 'preview-manifest.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
