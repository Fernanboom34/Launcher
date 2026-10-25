#!/usr/bin/env python3
"""Package an already built native ELF; does not download or publish anything."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile

repo = Path(__file__).resolve().parent.parent
binary = Path(sys.argv[1]).resolve()
version = re.search(r'^version = "([0-9.]+)"', (repo / 'Cargo.toml').read_text(), re.M)[1]
artifacts = repo / 'artifacts'
artifacts.mkdir(exist_ok=True)
symbols = subprocess.check_output(['objdump', '-T', str(binary)], text=True)
# Weak imports have runtime fallbacks (e.g. Rust's optional pidfd_spawnp).
strong_imports = '\n'.join(line for line in symbols.splitlines() if '*UND*' in line and 'w' not in line.split())
versions = re.findall(r'GLIBC_([0-9.]+)', strong_imports)
glibc = max(versions, key=lambda s: tuple(map(int, s.split('.'))))
header = binary.read_bytes()[:20]
if header[:6] != b'\x7fELF\x02\x01' or int.from_bytes(header[18:20], 'little') != 62:
    sys.exit('El binario no es Linux x64.')
stem = f'Luma-{version}-linux-x64'
outputs = []
with tempfile.TemporaryDirectory(prefix='luma-package-') as temporary:
    root = Path(temporary)
    bundle = root / stem
    bundle.mkdir()
    shutil.copy2(binary, bundle / 'luma-launcher')
    (bundle / 'luma-launcher').chmod(0o755)
    for name in ['install.sh', 'install.py', 'luma-launcher.svg']:
        shutil.copy2(repo / 'packaging/linux' / name, bundle / name)
    (bundle / 'install.sh').chmod(0o755)
    (bundle / 'package.json').write_text(json.dumps({'version': version, 'architecture': 'x86_64', 'glibc_min': glibc}))
    names = ['luma-launcher', 'install.sh', 'install.py', 'luma-launcher.svg', 'package.json']
    (bundle / 'SHA256SUMS.txt').write_text(''.join(hashlib.sha256((bundle / n).read_bytes()).hexdigest() + '  ' + n + '\n' for n in names))
    shutil.copy2(repo / 'docs/LINUX.md', bundle / 'README.md')
    bundle.chmod(0o755)
    for path in bundle.iterdir():
        path.chmod(0o755 if path.name in ('luma-launcher', 'install.sh', 'install.py') else 0o644)
    tar_path = artifacts / (stem + '.tar.gz')
    with tarfile.open(tar_path, 'w:gz') as archive:
        archive.add(bundle, arcname=stem)
    outputs.append(tar_path)
    deb = root / 'deb'
    for directory in ['DEBIAN', 'usr/lib/luma-launcher', 'usr/bin', 'usr/share/applications', 'usr/share/icons/hicolor/scalable/apps', 'usr/share/doc/luma-launcher']:
        (deb / directory).mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, deb / 'usr/lib/luma-launcher/luma-launcher')
    (deb / 'usr/lib/luma-launcher/luma-launcher').chmod(0o755)
    (deb / 'usr/bin/luma-launcher').symlink_to('../lib/luma-launcher/luma-launcher')
    (deb / 'usr/share/applications/dev.luma.Luma.desktop').write_text('[Desktop Entry]\nType=Application\nName=Luma\nComment=Launcher de Minecraft Java Edition\nExec=/usr/bin/luma-launcher\nIcon=luma-launcher\nTerminal=false\nCategories=Game;\nStartupWMClass=luma-launcher\n')
    shutil.copy2(repo / 'packaging/linux/luma-launcher.svg', deb / 'usr/share/icons/hicolor/scalable/apps/luma-launcher.svg')
    shutil.copy2(repo / 'docs/LINUX.md', deb / 'usr/share/doc/luma-launcher/README.md')
    dependencies = f'libc6 (>= {glibc}), libgcc-s1, libx11-6, libx11-xcb1, libxcb1, libxcursor1, libxi6, libxrandr2, libxkbcommon0, libxkbcommon-x11-0, libgl1, libegl1, libwayland-client0, libwayland-cursor0, libwayland-egl1, xdg-utils'
    (deb / 'DEBIAN/control').write_text(f'Package: luma-launcher\nVersion: {version}\nArchitecture: amd64\nMaintainer: Luma contributors\nSection: games\nPriority: optional\nDepends: {dependencies}\nRecommends: xdg-desktop-portal, xdg-desktop-portal-gtk\nHomepage: https://github.com/Fernanboom34/Launcher\nDescription: Luma Minecraft Java Edition launcher\n Native Linux x64 launcher with isolated instances and Modrinth content.\n')
    deb_path = artifacts / f'luma-launcher_{version}_amd64.deb'
    deb.chmod(0o755)
    for path in deb.rglob('*'):
        if not path.is_symlink():
            path.chmod(0o755 if path.is_dir() or path == deb / 'usr/lib/luma-launcher/luma-launcher' else 0o644)
    subprocess.run(['dpkg-deb', '--root-owner-group', '--build', str(deb), str(deb_path)], check=True)
    outputs.append(deb_path)
raw = artifacts / stem
shutil.copy2(binary, raw)
raw.chmod(0o755)
outputs.append(raw)
manifest = artifacts / 'luma-release-linux-x64.json'
manifest.write_text(json.dumps({'version': version, 'url': f'https://github.com/Fernanboom34/Launcher/releases/download/v{version}/{stem}', 'sha256': hashlib.sha256(raw.read_bytes()).hexdigest()}, indent=2) + '\n')
outputs.append(manifest)
(artifacts / 'SHA256SUMS-linux-x64.txt').write_text(''.join(hashlib.sha256(p.read_bytes()).hexdigest() + '  ' + p.name + '\n' for p in outputs))
for path in [*outputs, artifacts / 'SHA256SUMS-linux-x64.txt']:
    path.chmod(0o755 if path == raw else 0o644)
print(f'Paquetes Linux x64 generados en {artifacts}. glibc mínima del binario: {glibc}.')
