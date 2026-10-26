"""Verify the reported Fresh Animations download through a real release EXE.

Only uses a disposable instance. Requires an installed WebView2 runtime.
Test dependency: websocket-client (not a launcher dependency).
Usage: python scripts/check-web-download.py path/to/Luma.exe
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import zipfile
import urllib.request
import websocket

exe = Path(sys.argv[1]).resolve()
assert exe.is_file(), exe
folder = Path(tempfile.mkdtemp(prefix='Luma-download-regression-'))
try:
    (folder / 'game').mkdir()
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        port = listener.getsockname()[1]
    # Debug only this disposable browser process; no registry or persistent
    # WebView2 settings are changed. See Microsoft's WebView2 debugging docs.
    env = dict(os.environ, LUMA_DATA_DIR=str(folder / 'data'), LUMA_BROWSER_PROBE='1',
               WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=f'--remote-debugging-port={port}')
    profile = dict(id='download-regression', name='Prueba desechable', minecraft='26.3',
                   version_id='26.3', loader='NeoForge', loader_version='',
                   storage_dir=str(folder), game_dir=str(folder / 'game'), imported=False)
    request = dict(profile=profile, kind='ResourcePack', world=None,
                   url='https://modrinth.com/resourcepack/fresh-animations')
    process = subprocess.Popen([str(exe), '--content-browser'], stdin=subprocess.PIPE,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               text=True, encoding='utf-8', env=env)
    try:
        process.stdin.write(json.dumps(request))
        process.stdin.close()
        process.stdin = None
        for attempt in range(100):
            try:
                targets = json.load(urllib.request.urlopen(f'http://127.0.0.1:{port}/json/list', timeout=1))
                page = next(t for t in targets if t.get('type') == 'page' and t.get('url', '').startswith('https://modrinth.com/'))
                break
            except (OSError, StopIteration):
                time.sleep(0.1)
        else:
            raise AssertionError('Disposable WebView2 debugging target did not start')
        debug = websocket.create_connection(page['webSocketDebuggerUrl'], suppress_origin=True, timeout=5)
        try:
            debug.send(json.dumps(dict(id=1, method='Page.navigate', params=dict(url='https://cdn.modrinth.com/data/50dA9Sha/versions/tvGVYMD4/FreshAnimations_v1.11.1_26.1%2B.zip'))))
            debug.recv()
        finally:
            debug.close()
        stdout, stderr = process.communicate(timeout=45)
        print(stdout, stderr, flush=True)
        assert process.returncode == 0, process.returncode
    finally:
        if process.poll() is None:
            process.kill()
            process.communicate()
    file = folder / 'game' / 'resourcepacks' / 'FreshAnimations_v1.11.1_26.1+.zip'
    assert hashlib.sha1(file.read_bytes()).hexdigest() == 'ae9175983c5109162c86daae1acff34ccf623faf'
    with zipfile.ZipFile(file) as archive:
        assert archive.testzip() is None
    print('Release EXE saved the reported ZIP in the selected instance; SHA-1 and archive verified.')
finally:
    for attempt in range(10):
        try:
            shutil.rmtree(folder)
            break
        except OSError:
            time.sleep(0.3)
    else:
        print('Temporary fixture remains:', folder)
