#!/usr/bin/env python3
"""Install the local Linux bundle for this user; never remove game data."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import sys
import tempfile


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def desktop_quote(value):
    for old, new in [('\\', '\\\\\\\\'), ('"', '\\"'), ('`', '\\`'), ('$', '\\$'), ('%', '%%')]:
        value = value.replace(old, new)
    return '"' + value + '"'


def atomic_write(path, content, mode):
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix='.luma-', dir=path.parent)
    try:
        with os.fdopen(fd, 'wb') as output:
            output.write(content)
            os.fchmod(output.fileno(), mode)
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main():
    parser = argparse.ArgumentParser(description='Instala Luma para tu usuario, sin sudo.')
    parser.add_argument('--uninstall', action='store_true', help='Retira la aplicación y conserva mundos, cuentas y ajustes.')
    args = parser.parse_args()
    if platform.system() != 'Linux' or platform.machine() not in ('x86_64', 'AMD64'):
        raise RuntimeError('Este paquete requiere Linux x64 con glibc.')
    home = Path.home()
    data = Path(os.environ.get('XDG_DATA_HOME') or home / '.local/share')
    if not data.is_absolute() or any(c in str(data) for c in '\n\r\0'):
        raise RuntimeError('XDG_DATA_HOME debe ser una ruta absoluta sin saltos de línea.')
    root = data / 'luma-launcher'
    executable = root / 'bin/luma-launcher'
    marker = root / 'install.json'
    desktop = data / 'applications/dev.luma.Luma.desktop'
    icon = data / 'icons/hicolor/scalable/apps/luma-launcher.svg'
    uninstaller = root / 'uninstall.py'
    link = home / '.local/bin/luma-launcher'
    license_notice = root / 'LICENSE'
    source_notice = root / 'SOURCE.txt'
    files = [executable, desktop, icon, uninstaller, license_notice, source_notice]
    for path in [root, root / 'bin', marker, *files]:
        if path.is_symlink():
            raise RuntimeError('No se sustituirá un enlace: ' + str(path))
    state = json.loads(marker.read_text()) if marker.exists() else None
    if state is not None and state.get('application') != 'dev.luma.Luma':
        raise RuntimeError('El registro de instalación no pertenece a Luma.')
    if link.exists() or link.is_symlink():
        if not link.is_symlink() or os.readlink(link) != str(executable):
            raise RuntimeError('Ya existe un comando ajeno a esta instalación: ' + str(link))
    if args.uninstall:
        if state is None:
            raise RuntimeError('No hay una instalación por usuario registrada.')
        # Delete only exact owned paths, without following links or traversing trees.
        for path in files:
            if path.is_file():
                if path not in (executable, uninstaller) and digest(path) != state['files'].get(str(path)):
                    print('Se conserva el archivo modificado:', path)
                    continue
                path.unlink()
        if link.is_symlink():
            link.unlink()
        marker.unlink()
        for directory in [root / 'bin', root]:
            if directory.is_dir() and not any(directory.iterdir()):
                directory.rmdir()
        print('Luma desinstalado. Mundos, cuentas, Java y ajustes conservados.')
        return
    bundle = Path(__file__).resolve().parent
    source = bundle / 'luma-launcher'
    metadata = json.loads((bundle / 'package.json').read_text())
    expected = (bundle / 'SHA256SUMS.txt').read_text().splitlines()
    for line in expected:
        checksum, filename = line.split('  ', 1)
        if filename not in ('luma-launcher', 'install.py', 'install.sh', 'luma-launcher.svg', 'package.json', 'LICENSE', 'SOURCE.txt'):
            raise RuntimeError('Archivo inesperado en SHA256SUMS.txt')
        if digest(bundle / filename) != checksum:
            raise RuntimeError('Falló la comprobación SHA-256: ' + filename)
    if not any(line.endswith('  luma-launcher') for line in expected):
        raise RuntimeError('Falta la suma del ejecutable.')
    header = source.read_bytes()[:20]
    if header[:6] != b'\x7fELF\x02\x01' or int.from_bytes(header[18:20], 'little') != 62:
        raise RuntimeError('El archivo no es un ELF Linux x64.')
    for path in files:
        if path.exists() and (state is None or str(path) not in state.get('files', {})):
            raise RuntimeError('No se sobrescribirá un archivo ajeno: ' + str(path))
    desktop_content = ('[Desktop Entry]\nType=Application\nName=Luma\n'
                       'Comment=Launcher de Minecraft Java Edition\nExec=' + desktop_quote(str(executable)) +
                       '\nIcon=luma-launcher\nTerminal=false\nCategories=Game;\nStartupWMClass=luma-launcher\n')
    atomic_write(executable, source.read_bytes(), 0o755)
    atomic_write(desktop, desktop_content.encode(), 0o644)
    atomic_write(icon, (bundle / 'luma-launcher.svg').read_bytes(), 0o644)
    atomic_write(uninstaller, Path(__file__).read_bytes(), 0o755)
    atomic_write(license_notice, (bundle / 'LICENSE').read_bytes(), 0o644)
    atomic_write(source_notice, (bundle / 'SOURCE.txt').read_bytes(), 0o644)
    atomic_write(marker, json.dumps({'application': 'dev.luma.Luma', 'version': metadata['version'],
                                   'files': {str(p): digest(p) for p in files}}, indent=2).encode(), 0o644)
    link.parent.mkdir(parents=True, exist_ok=True)
    if not link.is_symlink():
        link.symlink_to(executable)
    print('Luma instalado. Ábrelo desde el menú de aplicaciones o ejecuta:', link)
    print('Para desinstalar conservando tus datos: python3', uninstaller, '--uninstall')


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, RuntimeError) as error:
        sys.exit('No se pudo completar la operación: ' + str(error))
