"""Read a controlled, immutable HTTPS fixture. No upload credentials or passwords."""
import hashlib
import json
import re
from urllib.parse import urlsplit
from urllib.request import build_opener, HTTPRedirectHandler


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def fetch(url, limit):
    with build_opener(NoRedirect()).open(url, timeout=15) as response:
        data = response.read(limit + 1)
    if len(data) > limit:
        raise ValueError('La respuesta web supera el límite.')
    return data


def load_registry(url):
    parsed = urlsplit(url)
    pattern = r'/Fernanboom34/Launcher/[a-f0-9]{40}/experiments/local-skins/web-fixture/registry\.json'
    if (parsed.scheme != 'https' or parsed.netloc != 'raw.githubusercontent.com'
            or not re.fullmatch(pattern, parsed.path) or parsed.query or parsed.fragment):
        raise ValueError('Este experimento solo admite un commit inmutable del directorio de prueba de Luma.')
    registry = json.loads(fetch(url, 65536))
    if registry.get('schema') != 1 or not isinstance(registry.get('players'), list) or len(registry['players']) > 16:
        raise ValueError('Directorio experimental inválido.')
    players, names = [], set()
    for entry in registry['players']:
        name, filename, digest = entry.get('name'), entry.get('file'), entry.get('sha256')
        if not isinstance(name, str) or not re.fullmatch(r'[A-Za-z0-9_]{1,16}', name) or name.lower() in names:
            raise ValueError('Nombre duplicado o inválido.')
        if not isinstance(filename, str) or not re.fullmatch(r'[A-Za-z0-9_-]+\.png', filename):
            raise ValueError('Ruta de textura inválida.')
        if not isinstance(digest, str) or not re.fullmatch(r'[a-f0-9]{64}', digest) or type(entry.get('slim')) is not bool:
            raise ValueError('Hash o modelo inválido.')
        png = fetch(url.rsplit('/', 1)[0] + '/' + filename, 1024 * 1024)
        if hashlib.sha256(png).hexdigest() != digest:
            raise ValueError('La textura web no coincide con su SHA-256.')
        names.add(name.lower())
        players.append((name, png, entry['slim']))
    return players
