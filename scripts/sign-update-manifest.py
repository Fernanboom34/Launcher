#!/usr/bin/env python3
"""Offline Ed25519 signing. Requires OpenSSL with Ed25519 support.
Never run with a private key in the ordinary build workflow.
"""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument('manifest', type=Path)
parser.add_argument('artifact', type=Path)
parser.add_argument('--private-key', type=Path, required=True)
args = parser.parse_args()
manifest = json.loads(args.manifest.read_text())
assert args.artifact.name == manifest['url'].split('/')[-1], 'El artefacto no corresponde a la URL'
assert hashlib.file_digest(args.artifact.open('rb'), 'sha256').hexdigest() == manifest['sha256'], 'Hash del artefacto incorrecto'
payload = ('Luma update manifest v1\n' + manifest['version'] + '\n' + manifest['url'] + '\n' + manifest['sha256'] + '\n').encode()
with tempfile.TemporaryDirectory(prefix='luma-sign-') as folder:
    source = Path(folder) / 'payload'
    source.write_bytes(payload)
    signature = subprocess.run(['openssl', 'pkeyutl', '-sign', '-rawin', '-inkey', str(args.private_key), '-in', str(source)], check=True, capture_output=True).stdout
assert len(signature) == 64, 'Se requiere una clave Ed25519'
manifest['signature'] = base64.b64encode(signature).decode()
args.manifest.write_text(json.dumps(manifest, indent=2) + '\n')
