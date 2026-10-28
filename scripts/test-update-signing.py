#!/usr/bin/env python3
"""Exercise the offline signer with disposable keys; never uses production secrets."""
import base64
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

with tempfile.TemporaryDirectory(prefix='luma-sign-test-') as folder:
    root = Path(folder)
    (root / 'scripts').mkdir()
    (root / 'security').mkdir()
    script = root / 'scripts/sign-update-manifest.py'
    shutil.copyfile(Path(__file__).with_name('sign-update-manifest.py'), script)
    key = root / 'key.pem'
    subprocess.run(['openssl', 'genpkey', '-algorithm', 'Ed25519', '-out', str(key)], check=True)
    public = subprocess.check_output(['openssl', 'pkey', '-in', str(key), '-pubout', '-outform', 'DER'])
    (root / 'security/update-trust.json').write_text(json.dumps({'ed25519_public_keys': [base64.b64encode(public[-32:]).decode()]}))
    artifact = root / 'Luma-0.8.1.exe'
    artifact.write_bytes(b'MZ disposable signing test')
    manifest = root / 'luma-release.json'
    data = {'version': '0.8.1', 'url': 'https://github.com/Fernanboom34/Launcher/releases/download/v0.8.1/' + artifact.name, 'sha256': hashlib.sha256(artifact.read_bytes()).hexdigest()}
    manifest.write_text(json.dumps(data))
    command = ['python3', str(script), str(manifest), str(artifact), '--private-key', str(key)]
    subprocess.run(command, check=True)
    signed = json.loads(manifest.read_text())
    assert len(base64.b64decode(signed['signature'])) == 64
    payload = root / 'payload'
    payload.write_bytes(('Luma update manifest v1\n' + data['version'] + '\n' + data['url'] + '\n' + data['sha256'] + '\n').encode())
    signature = root / 'signature'
    signature.write_bytes(base64.b64decode(signed['signature']))
    pub = root / 'pub.pem'
    pub.write_bytes(subprocess.check_output(['openssl', 'pkey', '-in', str(key), '-pubout']))
    subprocess.run(['openssl', 'pkeyutl', '-verify', '-rawin', '-pubin', '-inkey', str(pub), '-in', str(payload), '-sigfile', str(signature)], check=True)
    artifact.write_bytes(b'tampered')
    assert subprocess.run(command, capture_output=True).returncode != 0
    assert json.loads(manifest.read_text()) == signed
    artifact.write_bytes(b'MZ disposable signing test')
    (root / 'security/update-trust.json').write_text('{"ed25519_public_keys": []}')
    assert subprocess.run(command, capture_output=True).returncode != 0
    assert json.loads(manifest.read_text()) == signed
print('Offline signing: valid signature verified, wrong key and tampered binary rejected.')
