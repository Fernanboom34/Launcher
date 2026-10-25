"""Verify the reported Fresh Animations download through a real release EXE.

Only uses a disposable instance. Requires an installed WebView2 runtime.
Usage: python scripts/check-web-download.py path/to/Luma.exe
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import zipfile

exe = Path(sys.argv[1]).resolve()
assert exe.is_file(), exe
folder = Path(tempfile.mkdtemp(prefix='Luma-download-regression-'))
try:
    (folder / 'game').mkdir()
    env = dict(os.environ, LUMA_DATA_DIR=str(folder / 'data'), LUMA_BROWSER_PROBE='1')
    profile = dict(id='download-regression', name='Prueba desechable', minecraft='26.3',
                   version_id='26.3', loader='NeoForge', loader_version='',
                   storage_dir=str(folder), game_dir=str(folder / 'game'), imported=False)
    request = dict(profile=profile, kind='ResourcePack', world=None,
                   url='https://cdn.modrinth.com/data/50dA9Sha/versions/tvGVYMD4/FreshAnimations_v1.11.1_26.1%2B.zip')
    result = subprocess.run([str(exe), '--content-browser'], input=json.dumps(request),
                            text=True, encoding='utf-8', capture_output=True, env=env, timeout=50)
    print(result.stdout, result.stderr, flush=True)
    assert result.returncode == 0, result.returncode
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
