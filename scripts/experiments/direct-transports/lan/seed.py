"""Seed only the disposable loopback Collector, using its actual HTTP API."""
import http.cookiejar
import json
from pathlib import Path
import secrets
import time
import urllib.error
import urllib.request
import uuid

ORIGIN = 'https://collector-lan.awkwardexplorer.com:19978'
BASE = 'http://127.0.0.1:19982'
ROOT = Path('/fixture')
jar = http.cookiejar.CookieJar()
client = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))


def call(method, path, value=None, binary=None):
    body = binary if binary is not None else json.dumps(value).encode() if value is not None else b'' if method == 'POST' else None
    headers = {'Origin': ORIGIN, 'Content-Type': 'application/octet-stream' if binary is not None else 'application/json',
               'Idempotency-Key': str(uuid.uuid4())}
    # This setup client talks only to the fixed isolated loopback private listener.
    # Browser cookies remain Secure on the external HTTPS test origin.
    headers['Cookie'] = '; '.join(f'{entry.name}={entry.value}' for entry in jar)
    request = urllib.request.Request(BASE + path, data=body, method=method, headers=headers)
    try:
        with client.open(request, timeout=30) as response:
            data = response.read()
            return json.loads(data) if data else None
    except urllib.error.HTTPError as error:
        raise RuntimeError(f'{method} {path}: {error.code} {error.read(4096).decode()}') from None


password = (ROOT / 'password').read_text().strip()
for attempt in range(60):
    try:
        call('POST', '/api/v1/auth/login', {'username': 'listener', 'password': password})
        break
    except (OSError, RuntimeError):
        if attempt == 59:
            raise
        time.sleep(1)
collection = call('POST', '/api/v1/admin/collections', {'name': 'Disposable LAN audio', 'module': 'org.junk.music'})
call('POST', '/api/v1/admin/users', {'username': 'denied', 'password': password, 'administrator': False})
item_id = str(uuid.uuid4())
item = call('PUT', '/api/v1/items/' + item_id, {'expected_revision': 0, 'command': {
    'collection_id': collection['id'], 'title': 'Synthetic LAN seek test', 'subtitle': '',
    'identity_link': {'kind': 'none'}, 'domain': {'module': 'org.junk.music', 'schema_version': 1, 'payload': {}},
    'identity': 'unidentified', 'translations': [], 'identifiers': [], 'wishlist': False}})
audio = ROOT / 'noise.wav'
upload = call('POST', '/api/v1/uploads', {'item_id': item_id, 'expected_revision': item['revision'],
              'files': [{'name': 'noise.wav', 'bytes': audio.stat().st_size, 'sha256': None}]})
with audio.open('rb') as source:
    offset = 0
    while chunk := source.read(4 * 1024 * 1024):
        call('PUT', f"/api/v1/uploads/{upload['id']}/files/0?offset={offset}", binary=chunk)
        offset += len(chunk)
call('POST', f"/api/v1/uploads/{upload['id']}/commit")
for attempt in range(90):
    item = call('GET', '/api/v1/items/' + item_id)
    representations = [r for c in item['captures'] for r in c['representations']]
    if representations:
        break
    time.sleep(1)
else:
    raise RuntimeError('Upload did not publish within 90 seconds')
representation = representations[0]
path = f"/api/v1/items/{item_id}/representations/{representation['id']}/0"
track = {'itemId': item_id, 'collectionId': collection['id'], 'album': 'Disposable LAN audio',
         'title': 'Three-minute quiet synthetic noise', 'position': 1,
         'representationId': representation['id'], 'fileIndex': 0, 'mediaType': 'audio/wav'}
(ROOT / 'bridge').mkdir(exist_ok=True)
(ROOT / 'bridge' / 'fixture.json').write_text(json.dumps({'path': path, 'track': track}))
print(json.dumps({'seeded': True, 'audio_bytes': audio.stat().st_size, 'path': path}))
