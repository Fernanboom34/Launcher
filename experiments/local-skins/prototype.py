#!/usr/bin/env python3
"""Experimental, read-only loopback texture service. No Microsoft credentials.
Python 3.10+ and OpenSSL; optional Luma CLI launch in an isolated directory.
"""
import argparse
import base64
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import re
import secrets
import signal
import shutil
import struct
import subprocess
import tempfile
import threading
import time
from urllib.parse import urlsplit, parse_qs

AGENT_URL = 'https://authlib-injector.yushi.moe/artifact/56/authlib-injector-1.2.8.jar'
AGENT_SHA256 = '9c7f4343e6c82034958ffb48c14a2cb0c85928be7283103ce17da00c6d5a7b10'


def offline_uuid(name):
    if not re.fullmatch(r'[A-Za-z0-9_]{1,16}', name):
        raise ValueError('El nombre debe tener 1–16 letras, números o guiones bajos.')
    value = bytearray(hashlib.md5(('OfflinePlayer:' + name).encode()).digest())
    value[6] = (value[6] & 15) | 48
    value[8] = (value[8] & 63) | 128
    return value.hex()


def openssl_path():
    binary = shutil.which('openssl')
    if binary:
        return binary
    for root in [os.environ.get('ProgramFiles', ''), os.environ.get('LOCALAPPDATA', '')]:
        for suffix in ['Git/usr/bin/openssl.exe', 'Programs/Git/usr/bin/openssl.exe']:
            path = Path(root) / suffix
            if path.is_file():
                return str(path)
    raise RuntimeError('Instala OpenSSL o Git para Windows.')


class SkinService:
    def __init__(self, name, skin, slim=False):
        self.requests = {"metadata": 0, "profile": 0, "texture": 0}
        self.name, self.uuid = name, offline_uuid(name)
        path = Path(skin)
        if path.is_symlink() or path.stat().st_size > 1024 * 1024:
            raise ValueError('Usa un PNG local de hasta 1 MiB, sin enlace simbólico.')
        self.png = path.read_bytes()
        if (len(self.png) < 33 or self.png[:8] != b'\x89PNG\r\n\x1a\n'
                or self.png[12:16] != b'IHDR'
                or struct.unpack('>II', self.png[16:24]) not in [(64, 64), (64, 32)]):
            raise ValueError('Se requiere una skin PNG de 64×64 o 64×32.')
        self.slim = slim
        self.texture_hash = hashlib.sha256(self.png).hexdigest()
        self.secret = secrets.token_urlsafe(24)
        self.openssl = openssl_path()
        self.temporary = tempfile.TemporaryDirectory(prefix='luma-skin-key-')
        self.key = Path(self.temporary.name) / 'session.pem'
        subprocess.run([self.openssl, 'genpkey', '-algorithm', 'RSA', '-pkeyopt', 'rsa_keygen_bits:2048', '-out', str(self.key)], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.key.chmod(0o600)
        self.public = subprocess.check_output([self.openssl, 'pkey', '-in', str(self.key), '-pubout']).decode()
        owner = self

        class Handler(BaseHTTPRequestHandler):
            protocol_version = 'HTTP/1.0'

            def setup(self):
                super().setup()
                self.connection.settimeout(3)

            def log_message(self, *_):
                pass  # Never log request paths containing the local capability.

            def reply(self, code, data=None, content_type='application/json'):
                raw = b'' if data is None else (data if isinstance(data, bytes) else json.dumps(data, separators=(',', ':')).encode())
                self.send_response(code)
                self.send_header('Content-Type', content_type)
                self.send_header('Content-Length', str(len(raw)))
                self.send_header('Cache-Control', 'no-store')
                self.send_header('X-Content-Type-Options', 'nosniff')
                self.end_headers()
                if raw:
                    self.wfile.write(raw)

            def route(self):
                if self.headers.get('Host') != f'127.0.0.1:{owner.server.server_port}' or self.headers.get('Origin'):
                    self.reply(403)
                    return None
                if len(self.path) > 2048:
                    self.reply(414)
                    return None
                path = urlsplit(self.path).path
                prefix = '/' + owner.secret + '/'
                if not path.startswith(prefix):
                    self.reply(404)
                    return None
                return path[len(prefix):]

            def do_GET(self):
                route = self.route()
                if route is None:
                    return
                if not route:
                    owner.requests["metadata"] += 1
                    self.reply(200, {'meta': {'serverName': 'Luma local skins (experimental)', 'implementationName': 'Luma prototype', 'implementationVersion': '0.9.0-alpha.1', 'feature.no_mojang_namespace': True, 'feature.enable_mojang_anti_features': True}, 'skinDomains': ['127.0.0.1'], 'signaturePublickey': owner.public})
                elif route == 'textures/' + owner.texture_hash:
                    owner.requests['texture'] += 1
                    self.reply(200, owner.png, 'image/png')
                elif route == 'sessionserver/session/minecraft/profile/' + owner.uuid:
                    owner.requests['profile'] += 1
                    unsigned = parse_qs(urlsplit(self.path).query).get('unsigned', ['true'])[0] != 'false'
                    self.reply(200, owner.profile(unsigned))
                elif route == 'api/users/profiles/minecraft/' + owner.name:
                    self.reply(200, {'id': owner.uuid, 'name': owner.name})
                elif route.startswith('sessionserver/session/minecraft/profile/'):
                    self.reply(204)
                else:
                    self.reply(404)

            def do_POST(self):
                route = self.route()
                if route is None:
                    return
                # No authenticate, refresh, validate, join or hasJoined implementation.
                if route != 'api/profiles/minecraft':
                    self.reply(403)
                    return
                try:
                    length = int(self.headers.get('Content-Length', '-1'))
                    if not 0 <= length <= 4096:
                        self.reply(413)
                        return
                    names = json.loads(self.rfile.read(length))
                    if not isinstance(names, list) or len(names) > 100 or not all(isinstance(n, str) and len(n) <= 16 for n in names):
                        raise ValueError()
                    self.reply(200, [{'id': owner.uuid, 'name': owner.name}] if any(n.lower() == owner.name.lower() for n in names) else [])
                except (ValueError, UnicodeError):
                    self.reply(400)

        class Server(ThreadingHTTPServer):
            daemon_threads = True
            def __init__(self, *args):
                self.slots = threading.BoundedSemaphore(8)
                super().__init__(*args)
            def process_request(self, request, address):
                if not self.slots.acquire(blocking=False):
                    self.shutdown_request(request)
                    return
                try:
                    super().process_request(request, address)
                except BaseException:
                    self.slots.release()
                    raise
            def process_request_thread(self, *args):
                try:
                    super().process_request_thread(*args)
                finally:
                    self.slots.release()

        self.server = Server(('127.0.0.1', 0), Handler)
        self.url = f'http://127.0.0.1:{self.server.server_port}/{self.secret}/'
        value = base64.b64encode(json.dumps({'timestamp': int(time.time() * 1000), 'profileId': self.uuid, 'profileName': name, 'textures': {'SKIN': {'url': self.url + 'textures/' + self.texture_hash, 'metadata': {'model': 'slim' if slim else 'default'}}}}, separators=(',', ':')).encode()).decode()
        # SHA1withRSA is mandated by the Yggdrasil texture protocol. It is unrelated to update signing.
        signature = subprocess.run([self.openssl, 'dgst', '-sha1', '-sign', str(self.key)], input=value.encode(), stdout=subprocess.PIPE, check=True).stdout
        self.property = {'name': 'textures', 'value': value, 'signature': base64.b64encode(signature).decode()}
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()

    def profile(self, unsigned=False):
        prop = dict(self.property)
        if unsigned:
            prop.pop('signature', None)
        return {'id': self.uuid, 'name': self.name, 'properties': [prop]}

    def close(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=4)
        self.temporary.cleanup()


def run_owned(command, env):
    """Keep the launcher and Java in a process group owned by this experiment."""
    child = subprocess.Popen(command, env=env, start_new_session=os.name != 'nt',
                             creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if os.name == 'nt' else 0)
    try:
        code = child.wait()
        if code:
            raise subprocess.CalledProcessError(code, command)
    except BaseException:
        if child.poll() is None:
            if os.name == 'nt':
                subprocess.run(['taskkill', '/PID', str(child.pid), '/T', '/F'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            else:
                os.killpg(child.pid, signal.SIGTERM)
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                if os.name != 'nt':
                    os.killpg(child.pid, signal.SIGKILL)
                child.wait(timeout=5)
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--name', required=True)
    parser.add_argument('--skin', type=Path, required=True)
    parser.add_argument('--slim', action='store_true')
    parser.add_argument('--agent', type=Path, required=True)
    parser.add_argument('--launcher', type=Path)
    parser.add_argument('--data-dir', type=Path)
    parser.add_argument('--version', default='1.21.1')
    parser.add_argument('--prepare', action='store_true', help='Descarga Minecraft en la carpeta experimental antes de iniciarlo.')
    args = parser.parse_args()
    if hashlib.sha256(args.agent.read_bytes()).hexdigest() != AGENT_SHA256:
        parser.error('El agente no coincide con authlib-injector 1.2.8 fijado en este prototipo.')
    if args.launcher and not args.data_dir:
        parser.error('--launcher requiere una carpeta --data-dir experimental.')
    service = SkinService(args.name, args.skin, args.slim)
    try:
        agent_arg = '-javaagent:' + str(args.agent.resolve()) + '=' + service.url
        if not args.launcher:
            print('Servicio local activo. No se proporciona autenticación multijugador.')
            print(agent_arg)
            print('Ctrl+C para cerrar.')
            threading.Event().wait()
        else:
            root = args.data_dir.resolve()
            if root.is_symlink():
                parser.error("No se permite una carpeta experimental enlazada.")
            root.mkdir(parents=True, exist_ok=True)
            marker = root / 'luma-local-skin-experiment.json'
            if any(root.iterdir()) and not marker.is_file():
                parser.error('La carpeta debe estar vacía o pertenecer a este prototipo. No uses tus datos habituales.')
            if marker.exists() and (marker.is_symlink() or json.loads(marker.read_text()).get("experiment") != "local-skin-agent-v1"):
                parser.error("El registro no pertenece a este prototipo.")
            marker.write_text('{"experiment":"local-skin-agent-v1"}')
            # This directory belongs exclusively to the prototype: no real sessions, account tokens or JVM hooks.
            if (root / 'settings.json').is_symlink():
                parser.error('El archivo de opciones no puede ser un enlace.')
            (root / 'settings.json').write_text(json.dumps({'local_name': args.name, 'memory_mb': 1024, 'runtime': {'jvm_args': '"' + agent_arg.replace('\\', '\\\\').replace('"', '\\"') + '" -Dauthlibinjector.noLogFile -Dauthlibinjector.noShowServerName', 'width': 960, 'height': 640}}))
            env = dict(os.environ, LUMA_DATA_DIR=str(root))
            launcher = str(args.launcher.resolve())
            if args.prepare:
                run_owned([launcher, '--install', args.version], env)
            run_owned([launcher, '--play-local', args.version, args.name], env)
    except KeyboardInterrupt:
        pass
    finally:
        service.close()
        print("Peticiones locales:", json.dumps(service.requests))


if __name__ == '__main__':
    main()
