import base64
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch
from urllib.request import urlopen

sys.path.insert(0,str(Path(__file__).parent))
import prototype
import web_registry

FIXTURES = Path(__file__).parent / 'web-fixture'
URL = 'https://raw.githubusercontent.com/Fernanboom34/Launcher/' + 'a'*40 + '/experiments/local-skins/web-fixture/registry.json'


class WebTests(unittest.TestCase):
    def fetch(self, url, limit):
        data = (FIXTURES / url.rsplit('/',1)[-1]).read_bytes()
        self.assertLessEqual(len(data),limit)
        return data

    def test_two_clients_resolve_identical_remote_texture(self):
        with patch.object(web_registry,'fetch',side_effect=self.fetch):
            peers=web_registry.load_registry(URL)
        a=prototype.SkinService('LumaWebA',FIXTURES/'red.png',peers=peers)
        b=prototype.SkinService('LumaWebB',FIXTURES/'blue.png',peers=peers)
        try:
            self.assertNotEqual(a.public,b.public)
            for service in (a,b):
                for name,png,_ in peers:
                    profile=json.load(urlopen(service.url+'sessionserver/session/minecraft/profile/'+prototype.offline_uuid(name)+'?unsigned=false'))
                    value=json.loads(base64.b64decode(profile['properties'][0]['value']))
                    texture=urlopen(value['textures']['SKIN']['url']).read()
                    self.assertEqual(texture,png)
                    self.assertIn('signature',profile['properties'][0])
        finally:
            a.close();b.close()

    def test_unsafe_origins_paths_and_mutable_branches_rejected(self):
        for url in ['http://127.0.0.1/registry.json',URL.replace('a'*40,'main'),URL+'?other=1',URL.replace('raw.githubusercontent.com','evil.invalid')]:
            with self.assertRaises(ValueError):web_registry.load_registry(url)

    def test_tampered_texture_rejected(self):
        def fetch(url,limit):return self.fetch(url,limit) if url.endswith('json') else b'tampered'
        with patch.object(web_registry,'fetch',side_effect=fetch):
            with self.assertRaises(ValueError):web_registry.load_registry(URL)

    def test_duplicate_names_and_traversal_rejected(self):
        for change in ('name','file'):
            data=json.loads((FIXTURES/'registry.json').read_text())
            data['players'][1][change]=data['players'][0]['name'] if change=='name' else '../outside.png'
            with patch.object(web_registry,'fetch',side_effect=lambda u,l: json.dumps(data).encode() if u.endswith('json') else self.fetch(u,l)):
                with self.assertRaises(ValueError):web_registry.load_registry(URL)

    def test_local_skin_must_match_published_record(self):
        with patch.object(web_registry,'fetch',side_effect=self.fetch):peers=web_registry.load_registry(URL)
        with self.assertRaises(ValueError):prototype.SkinService('LumaWebA',FIXTURES/'blue.png',peers=peers)


if __name__=='__main__':unittest.main()
