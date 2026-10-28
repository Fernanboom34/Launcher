import base64
import hashlib
import importlib.util
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest
import urllib.error
import urllib.request
import zlib

spec = importlib.util.spec_from_file_location('prototype', Path(__file__).with_name('prototype.py'))
prototype = importlib.util.module_from_spec(spec)
spec.loader.exec_module(prototype)


def png():
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', 64, 64, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress((b'\0' + b'\xff\x00\x00\xff' * 64) * 64)) + chunk(b'IEND', b'')


class PrototypeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.folder = tempfile.TemporaryDirectory()
        cls.skin = Path(cls.folder.name) / 'skin.png'
        cls.skin.write_bytes(png())
        cls.service = prototype.SkinService('LumaTest', cls.skin, True)

    @classmethod
    def tearDownClass(cls):
        cls.service.close()
        cls.folder.cleanup()

    def get(self, path, headers=None):
        return urllib.request.urlopen(urllib.request.Request(self.service.url + path, headers=headers or {}), timeout=5)

    def test_signed_profile_and_texture(self):
        metadata = json.load(self.get(''))
        self.assertEqual(metadata['skinDomains'], ['127.0.0.1'])
        profile = json.load(self.get('sessionserver/session/minecraft/profile/' + self.service.uuid + '?unsigned=false'))
        prop = profile['properties'][0]
        payload = json.loads(base64.b64decode(prop['value']))
        self.assertEqual(payload['profileId'], self.service.uuid)
        self.assertEqual(payload['textures']['SKIN']['metadata']['model'], 'slim')
        self.assertEqual(urllib.request.urlopen(payload['textures']['SKIN']['url']).read(), self.skin.read_bytes())
        public = Path(self.folder.name) / 'public.pem'
        public.write_text(metadata['signaturePublickey'])
        signature = Path(self.folder.name) / 'signature'
        signature.write_bytes(base64.b64decode(prop['signature']))
        command = [self.service.openssl, 'dgst', '-sha1', '-verify', str(public), '-signature', str(signature)]
        self.assertEqual(subprocess.run(command, input=prop['value'].encode(), capture_output=True).returncode, 0)
        self.assertNotEqual(subprocess.run(command, input=(prop['value'] + 'x').encode(), capture_output=True).returncode, 0)

    def test_uuid_matches_minecraft_offline_algorithm(self):
        self.assertEqual(prototype.offline_uuid('Notch'), 'b50ad385829d3141a2167e7d7539ba7f')
        with self.assertRaises(ValueError):
            prototype.offline_uuid('../escape')

    def test_unknown_profile(self):
        self.assertEqual(self.get('sessionserver/session/minecraft/profile/' + '0' * 32).status, 204)

    def test_origin_and_host_blocked(self):
        for headers in [{'Origin': 'https://attacker.invalid'}, {'Host': 'attacker.invalid'}]:
            with self.assertRaises(urllib.error.HTTPError) as caught:
                self.get('', headers)
            self.assertEqual(caught.exception.code, 403)

    def test_routes_cannot_read_disk_or_authenticate(self):
        for path in ['../session.pem', 'textures/../../../etc/passwd', 'sessionserver/session/minecraft/hasJoined?username=LumaTest&serverId=x']:
            with self.assertRaises(urllib.error.HTTPError):
                self.get(path)
        for path in ['authserver/authenticate', 'sessionserver/session/minecraft/join']:
            request = urllib.request.Request(self.service.url + path, data=b'{}', method='POST')
            with self.assertRaises(urllib.error.HTTPError) as caught:
                urllib.request.urlopen(request)
            self.assertEqual(caught.exception.code, 403)

    def test_missing_capability_and_large_bodies_rejected(self):
        base = self.service.url.split('/' + self.service.secret)[0]
        with self.assertRaises(urllib.error.HTTPError) as caught:
            urllib.request.urlopen(base + '/')
        self.assertEqual(caught.exception.code, 404)
        request = urllib.request.Request(self.service.url + 'api/profiles/minecraft', data=b' ' * 4097, method='POST')
        with self.assertRaises(urllib.error.HTTPError) as caught:
            urllib.request.urlopen(request)
        self.assertEqual(caught.exception.code, 413)

    def test_readonly_name_lookup(self):
        request = urllib.request.Request(self.service.url + 'api/profiles/minecraft', data=b'["LumaTest","Nobody"]', method='POST')
        self.assertEqual(json.load(urllib.request.urlopen(request)), [{'id': self.service.uuid, 'name': 'LumaTest'}])


if __name__ == '__main__':
    unittest.main()
