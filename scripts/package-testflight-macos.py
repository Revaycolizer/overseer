#!/usr/bin/env python3
"""Stage and sign an existing Apple Silicon bundle for Mac TestFlight.

Requires a matching Mac App Store Connect provisioning profile and two installed
signing identities. Never uploads. The resulting package still requires sandbox
acceptance testing and App Store Connect validation.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
IDENTIFIER = 'co.tz.skyconnect.overseer'


def validate_profile(profile, identity_hash):
    entitlements = profile.get('Entitlements', {})
    teams = profile.get('TeamIdentifier', [])
    if len(teams) != 1:
        raise ValueError('Profile must identify one Apple team')
    team = teams[0]
    app_id = entitlements.get('com.apple.application-identifier')
    prefixes = profile.get('ApplicationIdentifierPrefix', [])
    if app_id not in [f'{prefix}.{IDENTIFIER}' for prefix in prefixes]:
        raise ValueError(f'Profile must match {IDENTIFIER}, found {app_id!r}')
    if 'OSX' not in profile.get('Platform', []):
        raise ValueError('A macOS profile is required; an iOS profile cannot be used')
    expiry = profile.get('ExpirationDate')
    if not expiry or expiry.replace(tzinfo=timezone.utc) <= datetime.now(timezone.utc):
        raise ValueError('Provisioning profile is expired or has no expiration')
    if profile.get('ProvisionedDevices') or profile.get('ProvisionsAllDevices'):
        raise ValueError('Use a Mac App Store Connect distribution profile')
    if entitlements.get('com.apple.security.get-task-allow') or entitlements.get('get-task-allow'):
        raise ValueError('Development profiles are not accepted')
    if entitlements.get('beta-reports-active') is not True:
        raise ValueError('Profile must authorize TestFlight (beta-reports-active)')
    if entitlements.get('com.apple.developer.team-identifier') != team:
        raise ValueError('Profile team entitlement does not match its team')
    hashes = [hashlib.sha1(cert).hexdigest().upper() for cert in profile.get('DeveloperCertificates', [])]
    if identity_hash.upper() not in hashes:
        raise ValueError('Selected signing certificate is not included in the profile')
    return team, app_id


def installed_identity(name, installer=False):
    allowed = ('3rd Party Mac Developer Installer:', 'Mac Installer Distribution:') if installer else ('Apple Distribution:', '3rd Party Mac Developer Application:', 'Mac App Distribution:')
    if not name.startswith(allowed):
        raise ValueError('Choose a Mac Installer Distribution identity' if installer else 'Choose an Apple Distribution or Mac App Distribution identity')
    output = subprocess.check_output(['security', 'find-identity', '-v', '-p', 'basic' if installer else 'codesigning'], text=True)
    for fingerprint, label in re.findall(r'([A-Fa-f0-9]{40}) "([^"]+)"', output):
        if label == name:
            return fingerprint.upper()
    raise ValueError(f'Identity with private key is not installed: {name}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--app', type=Path, default=ROOT / 'desktop-ui/src-tauri/target/release/bundle/macos/Overseer.app')
    parser.add_argument('--profile', type=Path, required=True)
    parser.add_argument('--identity', required=True)
    parser.add_argument('--installer-identity', required=True)
    parser.add_argument('--build-number', required=True, help='Positive integer greater than previous App Store Connect builds')
    parser.add_argument('--non-exempt-encryption', choices=['yes', 'no'], required=True, help='Your reviewed export-compliance declaration; not inferred by this script')
    parser.add_argument('--check-only', action='store_true')
    parser.add_argument('--output', type=Path, default=ROOT / 'dist/testflight-macos')
    args = parser.parse_args()
    if sys.platform != 'darwin':
        parser.error('Run on macOS')
    if not re.fullmatch(r'[1-9][0-9]{0,3}', args.build_number):
        parser.error('--build-number must be 1..9999')
    identity_hash = installed_identity(args.identity)
    installer_hash = installed_identity(args.installer_identity, installer=True)
    decoded = subprocess.check_output(['security', 'cms', '-D', '-i', str(args.profile.resolve())])
    profile = plistlib.loads(decoded)
    team, app_id = validate_profile(profile, identity_hash)
    if f'({team})' not in args.identity or f'({team})' not in args.installer_identity:
        raise ValueError('Both signing identities must belong to the profile team')
    app = args.app.resolve()
    metadata = plistlib.loads((app / 'Contents/Info.plist').read_bytes())
    if metadata.get('CFBundleIdentifier') != IDENTIFIER:
        raise ValueError('App bundle identifier does not match Overseer')
    main_binary = Path('Contents/MacOS') / metadata['CFBundleExecutable']
    backend = Path('Contents/Resources/backend/overseer')
    for binary in [app / main_binary, app / backend]:
        if subprocess.check_output(['lipo', '-archs', str(binary)], text=True).strip() != 'arm64':
            raise ValueError(f'Expected Apple Silicon executable: {binary}')
    # Additional nested code needs its own inside-out signing rules.
    if list(app.rglob('*.dylib')) or list(app.rglob('*.framework')) or list(app.rglob('*.xpc')):
        raise ValueError('Unexpected nested code; extend signing rules before packaging')
    print('Profile, team, certificates and Apple Silicon executables passed preflight.')
    if args.check_only:
        return
    args.output.mkdir(parents=True, exist_ok=True)
    package = args.output.resolve() / f'Overseer-{metadata["CFBundleShortVersionString"]}-{args.build_number}-macos-arm64.pkg'
    if package.exists():
        raise ValueError(f'Package exists; use a new build number or output directory: {package}')
    with tempfile.TemporaryDirectory(prefix='overseer-testflight-') as tmp:
        tmp = Path(tmp)
        staged = tmp / 'Overseer.app'
        subprocess.run(['ditto', str(app), str(staged)], check=True)
        shutil.copy2(args.profile, staged / 'Contents/embedded.provisionprofile')
        metadata.update({
            'CFBundleVersion': args.build_number,
            'LSMinimumSystemVersion': '12.0',
            'LSApplicationCategoryType': 'public.app-category.productivity',
            'NSMicrophoneUsageDescription': 'Overseer uses the microphone when you enable voice input.',
            'ITSAppUsesNonExemptEncryption': args.non_exempt_encryption == 'yes',
        })
        (staged / 'Contents/Info.plist').write_bytes(plistlib.dumps(metadata))
        entitlements = {
            'com.apple.security.app-sandbox': True,
            'com.apple.security.network.client': True,
            'com.apple.security.network.server': True,
            'com.apple.security.device.audio-input': True,
            'com.apple.application-identifier': app_id,
            'com.apple.developer.team-identifier': team,
            'beta-reports-active': True,
        }
        parent_entitlements = tmp / 'app.plist'
        parent_entitlements.write_bytes(plistlib.dumps(entitlements))
        child_entitlements = tmp / 'backend.plist'
        child_entitlements.write_bytes(plistlib.dumps({
            'com.apple.security.app-sandbox': True,
            'com.apple.security.inherit': True,
        }))
        for target, rights in [(staged / backend, child_entitlements), (staged, parent_entitlements)]:
            subprocess.run(['codesign', '--force', '--sign', identity_hash, '--entitlements', str(rights), str(target)], check=True)
        subprocess.run(['codesign', '--verify', '--strict', str(staged / backend)], check=True)
        subprocess.run(['codesign', '--verify', '--deep', '--strict', str(staged)], check=True)
        temporary_package = tmp / 'Overseer.pkg'
        subprocess.run(['xcrun', 'productbuild', '--sign', installer_hash, '--component', str(staged), '/Applications', str(temporary_package)], check=True)
        subprocess.run(['pkgutil', '--check-signature', str(temporary_package)], check=True)
        shutil.copy2(temporary_package, package)
    digest = hashlib.sha256(package.read_bytes()).hexdigest()
    package.with_suffix('.pkg.sha256').write_text(f'{digest}  {package.name}\n')
    report = {'package': package.name, 'sha256': digest, 'team': team, 'build_number': args.build_number,
              'uploaded': False, 'sandbox_runtime_acceptance': 'unverified', 'apple_validation': 'not submitted'}
    package.with_suffix('.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        raise SystemExit(f'TestFlight packaging stopped: {error}')
