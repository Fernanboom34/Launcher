#!/usr/bin/env python3
"""Offline Ed25519 signing. Requires OpenSSL with Ed25519 support.
Never run with a private key in the ordinary build workflow.
"""
import argparse
import base64
import hashlib
import json
import shutil
import os
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument('manifest', type=Path)
parser.add_argument('artifact', type=Path)
parser.add_argument('--private-key', type=Path, required=True)
args = parser.parse_args()
openssl = shutil.which('openssl')
if not openssl and os.name == 'nt':
    for root in [os.environ.get('ProgramFiles', ''), os.environ.get('LOCALAPPDATA', '')]:
        for relative in ['Git/usr/bin/openssl.exe', 'Programs/Git/usr/bin/openssl.exe']:
            candidate = Path(root) / relative
            if candidate.is_file():
                openssl = str(candidate)
                break
        if openssl:
            break
if not openssl:
    parser.error('No se encuentra OpenSSL. Instala Git para Windows u OpenSSL.')
# Ensure this private key belongs to the public trust embedded in this checkout.
public = subprocess.run([openssl, 'pkey', '-in', str(args.private_key), '-pubout', '-outform', 'DER'], check=True, stdout=subprocess.PIPE).stdout
prefix = bytes.fromhex('302a300506032b6570032100')
trust = json.loads((Path(__file__).resolve().parent.parent / 'security/update-trust.json').read_text())
if len(public) != 44 or not public.startswith(prefix) or base64.b64encode(public[12:]).decode() not in trust['ed25519_public_keys']:
    parser.error('La clave privada no corresponde a una clave pública Ed25519 confiable de Luma.')
manifest = json.loads(args.manifest.read_text())
assert args.artifact.name == manifest['url'].split('/')[-1], 'El artefacto no corresponde a la URL'
digest = hashlib.sha256()
with args.artifact.open('rb') as artifact:
    for chunk in iter(lambda: artifact.read(65536), b''):
        digest.update(chunk)
assert digest.hexdigest() == manifest['sha256'], 'Hash del artefacto incorrecto'
payload = ('Luma update manifest v1\n' + manifest['version'] + '\n' + manifest['url'] + '\n' + manifest['sha256'] + '\n').encode()
with tempfile.TemporaryDirectory(prefix='luma-sign-') as folder:
    source = Path(folder) / 'payload'
    source.write_bytes(payload)
    signature = subprocess.run([openssl, 'pkeyutl', '-sign', '-rawin', '-inkey', str(args.private_key), '-in', str(source)], check=True, stdout=subprocess.PIPE).stdout
assert len(signature) == 64, 'Se requiere una clave Ed25519'
manifest['signature'] = base64.b64encode(signature).decode()
args.manifest.write_text(json.dumps(manifest, indent=2) + '\n')
