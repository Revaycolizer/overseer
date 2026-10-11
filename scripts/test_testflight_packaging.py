"""Exercise TestFlight profile checks without Apple credentials or private keys."""
import copy
from datetime import datetime, timedelta, timezone
import hashlib
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('packager', Path(__file__).with_name('package-testflight-macos.py'))
packager = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packager)


class ProfileTests(unittest.TestCase):
    def setUp(self):
        certificate = b'public-certificate-fixture'
        self.fingerprint = hashlib.sha1(certificate).hexdigest().upper()
        self.profile = {
            'TeamIdentifier': ['TEAM123456'],
            'ApplicationIdentifierPrefix': ['PREFIX1234'],
            'Platform': ['OSX'],
            'ExpirationDate': datetime.now(timezone.utc) + timedelta(days=1),
            'DeveloperCertificates': [certificate],
            'Entitlements': {
                'com.apple.application-identifier': 'PREFIX1234.co.tz.skyconnect.overseer',
                'com.apple.developer.team-identifier': 'TEAM123456',
                'beta-reports-active': True,
            },
        }

    def test_valid_profile_with_distinct_app_prefix(self):
        self.assertEqual(packager.validate_profile(self.profile, self.fingerprint),
                         ('TEAM123456', 'PREFIX1234.co.tz.skyconnect.overseer'))

    def test_invalid_profiles(self):
        changes = [
            ('Platform', ['iOS']),
            ('ExpirationDate', datetime.now(timezone.utc) - timedelta(days=1)),
            ('ProvisionedDevices', ['device-fixture']),
            ('ProvisionsAllDevices', True),
            ('DeveloperCertificates', [b'wrong-certificate']),
            ('TeamIdentifier', []),
        ]
        for key, value in changes:
            with self.subTest(key=key):
                profile = copy.deepcopy(self.profile)
                profile[key] = value
                with self.assertRaises(ValueError):
                    packager.validate_profile(profile, self.fingerprint)
        for key, value in [
            ('com.apple.application-identifier', 'PREFIX1234.other.app'),
            ('com.apple.application-identifier', 'PREFIX1234.*'),
            ('com.apple.developer.team-identifier', 'OTHER'),
            ('beta-reports-active', False),
            ('get-task-allow', True),
            ('com.apple.security.get-task-allow', True),
        ]:
            with self.subTest(entitlement=key):
                profile = copy.deepcopy(self.profile)
                profile['Entitlements'][key] = value
                with self.assertRaises(ValueError):
                    packager.validate_profile(profile, self.fingerprint)


if __name__ == '__main__':
    unittest.main()
