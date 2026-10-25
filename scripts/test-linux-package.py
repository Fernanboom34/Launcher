#!/usr/bin/env python3
"""Exercise real packages in an isolated HOME; no root or system installation."""
import json
import os
import re
from pathlib import Path
import subprocess
import tarfile
import tempfile

repo = Path(__file__).resolve().parent.parent
version = re.search(r'^version = "([0-9.]+)"', (repo / 'Cargo.toml').read_text(), re.M)[1]
archive = repo / 'artifacts' / f'Luma-{version}-linux-x64.tar.gz'
deb = repo / 'artifacts' / f'luma-launcher_{version}_amd64.deb'


def run(*command, env, expected=0):
    result = subprocess.run(command, env=env, text=True, capture_output=True)
    if result.returncode != expected:
        raise AssertionError(f'{command}: exit {result.returncode}\n{result.stdout}\n{result.stderr}')
    return result


with tempfile.TemporaryDirectory(prefix='luma-package-test-') as temp:
    root = Path(temp)
    home = root / 'Home with spaces % and $dollar'
    home.mkdir()
    data = home / '.local/share'
    environment = dict(os.environ, HOME=str(home), XDG_DATA_HOME=str(data),
                       XDG_CONFIG_HOME=str(home / '.config'), XDG_CACHE_HOME=str(home / '.cache'),
                       LUMA_DATA_DIR=str(home / 'game-data'))
    with tarfile.open(archive) as package:
        # These are our own generated artifacts, not arbitrary third-party archives.
        if hasattr(tarfile, 'data_filter'):
            package.extractall(root / 'bundle', filter='data')
        else:
            package.extractall(root / 'bundle')
    bundle = next((root / 'bundle').iterdir())
    installer = bundle / 'install.sh'
    # Corrupted payload must fail before any installation.
    source = bundle / 'luma-launcher'
    original = source.read_bytes()
    source.write_bytes(b'corrupt')
    run('sh', str(installer), env=environment, expected=1)
    assert not (data / 'luma-launcher').exists()
    source.write_bytes(original)
    run('sh', str(installer), env=environment)
    installed = data / 'luma-launcher/bin/luma-launcher'
    assert installed.read_bytes() == original
    assert os.access(installed, os.X_OK)
    link = home / '.local/bin/luma-launcher'
    assert link.resolve() == installed
    desktop = data / 'applications/dev.luma.Luma.desktop'
    assert '%%' in desktop.read_text() and '\\$' in desktop.read_text()
    # Import real profile metadata through the installed CLI, then repeat it.
    game = Path(environment['LUMA_DATA_DIR'])
    versions = game / 'versions/smoke-version'
    versions.mkdir(parents=True)
    (versions / 'smoke-version.json').write_text('{"id":"smoke-version","libraries":[]}')
    (game / 'launcher_profiles.json').write_text(json.dumps({'profiles': {'test': {'name': 'Linux smoke', 'lastVersionId': 'smoke-version'}}}))
    first = run(str(link), '--list-profiles', env=environment).stdout
    assert 'Linux smoke' in first and first.count('smoke-version') == 1
    assert run(str(link), '--list-profiles', env=environment).stdout == first
    # Reinstallation replaces the ELF atomically and preserves user data.
    (game / 'keep-world.dat').write_text('keep')
    run('sh', str(installer), env=environment)
    assert (game / 'keep-world.dat').read_text() == 'keep'
    uninstaller = data / 'luma-launcher/uninstall.py'
    run('python3', str(uninstaller), '--uninstall', env=environment)
    assert not installed.exists() and not link.exists() and not desktop.exists()
    assert (game / 'keep-world.dat').read_text() == 'keep'
    # An unrelated existing command must never be overwritten.
    link.write_text('unrelated command')
    run('sh', str(installer), env=environment, expected=1)
    assert link.read_text() == 'unrelated command'
    # Extract the .deb and execute its actual payload without system privileges.
    control = subprocess.check_output(['dpkg-deb', '-f', str(deb)], text=True)
    assert 'Architecture: amd64' in control and 'libc6 (>=' in control
    run('dpkg-deb', '-x', str(deb), str(root / 'deb'), env=environment)
    deb_binary = root / 'deb/usr/bin/luma-launcher'
    assert run(str(deb_binary), '--list-profiles', env=environment).stdout == first
    assert deb_binary.read_bytes() == original
    print('PASS: checksums, installation, executable mode, desktop quoting, CLI/profile discovery, repeatability, data preservation, uninstall, collision protection and .deb payload.')
