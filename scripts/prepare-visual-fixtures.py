#!/usr/bin/env python3
"""Create disposable local UI fixtures. No game downloads or player data."""
import hashlib
import json
from pathlib import Path
import struct
import sys
import zipfile
import zlib

root = Path(sys.argv[1]).resolve()
if root.exists() and any(root.iterdir()):
    sys.exit('El directorio de fixtures debe estar vacío.')
root.mkdir(parents=True, exist_ok=True)


def png(size, pixel):
    def chunk(kind, data):
        return struct.pack('!I', len(data)) + kind + data + struct.pack('!I', zlib.crc32(kind + data))
    raw = b''.join(b'\0' + bytes(v for x in range(size) for v in pixel(x, y)) for y in range(size))
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('!2I5B', size, size, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(raw)) + chunk(b'IEND', b''))


def skin_pixel(x, y):
    base = (y < 16 and x < 32) or (16 <= y < 32 and x < 56) or (48 <= y and 16 <= x < 48)
    return (80 + x * 2, 60 + y * 2, 70 + (x ^ y) * 2, 255 if base else 0)


skin = png(64, skin_pixel)
key = hashlib.sha256(skin).hexdigest()
(root / 'skins/textures').mkdir(parents=True)
(root / f'skins/textures/{key}.png').write_bytes(skin)
(root / 'skins/library.json').write_text(json.dumps({'skins': [{'id': key, 'name': 'Atlas de prueba', 'slim': False,
                                                            'model_explicit': True, 'builtin': True}], 'selected': {}}))
game = root / 'game'
profile = {'id': 'visual', 'name': 'Fabric 1.21.1', 'minecraft': '1.21.1', 'version_id': 'fabric-visual',
           'loader': 'Fabric', 'loader_version': '0.16.0', 'storage_dir': str(root), 'game_dir': str(game)}
(root / 'profiles.json').write_text(json.dumps([profile]))
for folder, filename, title, author, version, color in [
    ('mods', 'collective.jar', 'Collective', 'Serilum', '8.17', (190, 150, 70)),
    ('mods', 'fabric-api.jar', 'Fabric API', 'modmuss50', '0.116.17+1.21.1', (170, 185, 205)),
    ('shaderpacks', 'ComplementaryReimagined.zip', 'Complementary Shaders - Reimagined', 'EminGTR', 'r5.7.1', (170, 105, 200)),
    ('resourcepacks', 'FreshAnimations.zip', 'Fresh Animations', 'FreshLX', '1.11.1', (85, 160, 100)),
]:
    path = game / folder / filename
    path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(path, 'w') as archive:
        archive.writestr('fabric.mod.json', json.dumps({'schemaVersion': 1, 'id': filename.split('.')[0].lower(),
                                                       'name': title, 'version': version, 'authors': [author], 'icon': 'icon.png'}))
        archive.writestr('pack.mcmeta', json.dumps({'pack': {'pack_format': 34, 'description': title}}))
        icon = png(32, lambda x, y: (*color, 255))
        archive.writestr('icon.png', icon)
        archive.writestr('pack.png', icon)
print(root)
