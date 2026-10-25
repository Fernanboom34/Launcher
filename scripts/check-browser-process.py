"""Smoke test de las webs reales en un proceso separado y con datos desechables.

Uso: python scripts/check-browser-process.py ruta/al/Luma.exe
Requiere WebView2 instalado. No descarga ni instala un runtime, mods ni Minecraft.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import shutil

exe = Path(sys.argv[1]).resolve()
assert exe.is_file(), exe
folder = Path(tempfile.mkdtemp(prefix='Luma-browser-smoke-'))
try:
    env = os.environ.copy()
    env['LUMA_DATA_DIR'] = str(folder / 'data')
    env['LUMA_BROWSER_PROBE'] = '1'
    # La compilación GNU necesita el cargador; MSVC lo enlaza estáticamente.
    for loader in Path('target/debug/build').glob('webview2-com-sys-*/out/x64/WebView2Loader.dll'):
        env['PATH'] = str(loader.resolve().parent) + os.pathsep + env.get('PATH', '')
        break
    profile = dict(id='browser-smoke', name='Prueba desechable', minecraft='1.21.1',
                   version_id='1.21.1', loader='Fabric', loader_version='',
                   storage_dir=str(folder), game_dir=str(folder / 'game'), imported=False)
    for site in ['https://modrinth.com/mods', 'https://www.curseforge.com/minecraft/mc-mods']:
        request = dict(profile=profile, kind='Mod', world=None, url=site)
        result = subprocess.run([str(exe), '--content-browser'], input=json.dumps(request),
                                text=True, encoding='utf-8', capture_output=True, env=env, timeout=50)
        print(site, 'exit=', result.returncode, flush=True)
        print(result.stdout, result.stderr, flush=True)
        assert result.returncode == 0
finally:
    # WebView2 puede tardar brevemente en liberar sus archivos después del cierre.
    for attempt in range(10):
        try:
            shutil.rmtree(folder)
            break
        except OSError:
            time.sleep(0.3)
    else:
        print('Carpeta temporal pendiente de limpieza:', folder)
