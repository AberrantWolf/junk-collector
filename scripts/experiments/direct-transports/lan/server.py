"""Disposable LAN player experiment. Never point BACKEND at a real installation."""
import asyncio
import datetime
import ipaddress
import json
import os
from pathlib import Path
import re
import secrets
import signal
import ssl
import time

from aiohttp import ClientSession, ClientTimeout, web
from aioquic.asyncio import QuicConnectionProtocol, serve
from aioquic.h3.connection import H3Connection, H3_ALPN
from aioquic.h3.events import HeadersReceived, WebTransportStreamDataReceived
from aioquic.quic.configuration import QuicConfiguration
from aioquic.quic.events import ConnectionTerminated, ProtocolNegotiated, StreamReset
from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.x509.oid import NameOID

ROOT = Path(__file__).parent
STATE = Path('/fixture/bridge')
ORIGIN = 'https://collector-lan.awkwardexplorer.com:19978'
PUBLIC_ORIGIN = 'https://collector.awkwardexplorer.com'
PUBLIC_PREFIX = '/__collector_lan_test_20261022'
BIND = '192.168.1.106'
BACKEND = 'http://127.0.0.1:19981'
MEDIA = re.compile(r'/api/v1/items/[0-9a-f-]{36}/representations/[0-9a-f-]{36}/[0-9]+\Z')
BLOCK = 262144
GRANTS = {}
PROTOCOLS = set()
HTTP = None
CERT = None
STATS = {'reads': 0, 'bytes': 0, 'http_media_denied': 0, 'statuses': {}, 'last_reads': []}


def cookie(request, public=False):
    value = request.cookies.get('collector_wt_public_session' if public else 'collector_wt_session', '')
    return 'collector_session=' + value if re.fullmatch(r'[A-Za-z0-9._~-]{1,512}', value) else ''


def status(code):
    key = str(code)
    STATS['statuses'][key] = STATS['statuses'].get(key, 0) + 1


async def media_read(value, origin):
    """Fixed authority, exact media capability, bounded byte range, live auth."""
    token = value.get('token', '')
    grant = GRANTS.get(token) if isinstance(token, str) else None
    if not grant or grant[1] <= time.monotonic() or grant[2] != origin:
        return 401, {}, b''
    path, method, byte_range = value.get('path', ''), value.get('method'), value.get('range', '')
    if not isinstance(path, str) or not MEDIA.fullmatch(path) or path != FIXTURE['path']:
        return 403, {}, b''
    if method not in ('HEAD', 'GET'):
        return 405, {}, b''
    if method == 'GET':
        match = re.fullmatch(r'bytes=(\d+)-(\d+)', byte_range) if isinstance(byte_range, str) else None
        if not match or not 0 <= int(match[1]) <= int(match[2]) < int(match[1]) + BLOCK:
            return 416, {}, b''
    headers = {'Cookie': grant[0], 'Origin': grant[2]}
    if method == 'GET':
        headers['Range'] = byte_range
    async with HTTP.request(method, BACKEND + path, headers=headers, allow_redirects=False) as response:
        length = int(response.headers.get('content-length', '-1')) if method != 'HEAD' else 0
        if not 0 <= length <= BLOCK or response.status not in (200, 206, 401, 403, 404, 409, 416):
            return 502, {}, b''
        body = await response.content.readexactly(length) if length else b''
        keep = {key.lower(): value for key, value in response.headers.items()
                if key.lower() in ('content-type', 'content-length', 'content-range', 'accept-ranges', 'etag')}
        STATS['reads'] += 1
        STATS['bytes'] += len(body)
        STATS['last_reads'] = (STATS['last_reads'] + [{'method': method, 'range': byte_range,
                                   'status': response.status, 'bytes': len(body)}])[-24:]
        print(json.dumps({'media': method, 'range': byte_range, 'status': response.status,
                          'bytes': len(body)}), flush=True)
        return response.status, keep, body


class Bridge(QuicConnectionProtocol):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.http = None
        self.sessions = {}
        self.pending = {}
        self.tasks = {}
        PROTOCOLS.add(self)

    def quic_event_received(self, event):
        if isinstance(event, (ConnectionTerminated, StreamReset)):
            for stream, task in list(self.tasks.items()):
                if isinstance(event, ConnectionTerminated) or stream == event.stream_id:
                    task.cancel()
            if isinstance(event, ConnectionTerminated):
                PROTOCOLS.discard(self)
                return
        if isinstance(event, ProtocolNegotiated):
            self.http = H3Connection(self._quic, enable_webtransport=True)
        if self.http is None:
            return
        for item in self.http.handle_event(event):
            if isinstance(item, HeadersReceived):
                headers = dict(item.headers)
                allowed = (headers.get(b':method') == b'CONNECT'
                           and headers.get(b':protocol') == b'webtransport'
                           and headers.get(b':path') == b'/media'
                           and headers.get(b'origin') in (ORIGIN.encode(), PUBLIC_ORIGIN.encode())
                           and len(PROTOCOLS) <= 8)
                if allowed:
                    origin = headers[b'origin'].decode()
                    self.sessions[item.stream_id] = origin
                    self.http.send_headers(item.stream_id, [(b':status', b'200')])
                    peer = self._quic._network_paths[0].addr[0]
                    print(json.dumps({'connected_peer': peer, 'origin': origin}), flush=True)
                else:
                    self.http.send_headers(item.stream_id, [(b':status', b'403')], end_stream=True)
            elif isinstance(item, WebTransportStreamDataReceived):
                if item.session_id not in self.sessions or item.stream_id in self.tasks:
                    self.close(error_code=1, reason_phrase='invalid stream')
                    continue
                data = self.pending.setdefault(item.stream_id, bytearray())
                data.extend(item.data)
                if len(data) > 4096 or len(self.pending) + len(self.tasks) > 8:
                    self.close(error_code=1, reason_phrase='request bound')
                    continue
                if item.stream_ended:
                    raw = bytes(self.pending.pop(item.stream_id))
                    task = asyncio.create_task(self.respond(item.stream_id, raw, self.sessions[item.session_id]))
                    self.tasks[item.stream_id] = task
            self.transmit()

    async def respond(self, stream, raw, origin):
        try:
            async with asyncio.timeout(12):
                value = json.loads(raw)
                if not isinstance(value, dict):
                    raise ValueError('request must be an object')
                code, headers, body = await media_read(value, origin)
        except asyncio.CancelledError:
            return
        except Exception as error:
            print('read failed: ' + type(error).__name__, flush=True)
            code, headers, body = 502, {}, b''
        finally:
            self.tasks.pop(stream, None)
        status(code)
        envelope = json.dumps({'status': code, 'headers': headers}, separators=(',', ':')).encode() + b'\n'
        self._quic.send_stream_data(stream, envelope + body, end_stream=True)
        self.transmit()


async def handler(request, origin=ORIGIN, prefix=''):
    path = request.path
    public = bool(prefix)
    if prefix:
        if path == prefix:
            return web.Response(status=307, headers={'Location': prefix + '/', 'Cache-Control': 'no-store'})
        if not path.startswith(prefix + '/'):
            return web.Response(status=404)
        path = path[len(prefix):]
    if MEDIA.fullmatch(path):
        STATS['http_media_denied'] += 1
        return web.Response(status=410, text='Experiment media is WebTransport only')
    if path in ('/', '/app.js', '/sw.js') and request.method == 'GET':
        filename = {'/': 'index.html', '/app.js': 'app.js', '/sw.js': 'sw.js'}[path]
        response = web.Response(body=(ROOT / filename).read_bytes(), content_type={
            '/': 'text/html', '/app.js': 'application/javascript', '/sw.js': 'application/javascript'}[path])
        response.headers['Cache-Control'] = 'no-store'
        endpoint_policy = '' if request.query.get('block-wt') == '1' else ' https://192.168.1.106:19979'
        response.headers['Content-Security-Policy'] = (
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; "
            "connect-src 'self'" + endpoint_policy + "; media-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'")
        response.headers['X-Content-Type-Options'] = 'nosniff'
        response.headers['Referrer-Policy'] = 'no-referrer'
        return response
    if path == '/fixture' and request.method == 'GET':
        return web.json_response({**FIXTURE, 'hash': list(CERT.fingerprint(hashes.SHA256())),
                                  'endpoint': 'https://192.168.1.106:19979/media', 'block': BLOCK,
                                  'origin': origin, 'control_prefix': prefix,
                                  'via_cloudflare': bool(request.headers.get('CF-Ray'))})
    if path == '/stats' and request.method == 'GET':
        return web.json_response(STATS)
    if path == '/grant' and request.method == 'POST' and request.headers.get('Origin') == origin:
        current = cookie(request, public)
        async with HTTP.get(BACKEND + '/api/v1/me', headers={'Cookie': current}) as response:
            if response.status != 200:
                return web.Response(status=401)
            who = await response.json()
        # Bounded ephemeral test leases; the real Collector session is still checked on every read.
        for key, grant in list(GRANTS.items()):
            if grant[1] <= time.monotonic():
                GRANTS.pop(key)
        if len(GRANTS) >= 32:
            return web.Response(status=429)
        token = secrets.token_urlsafe(32)
        GRANTS[token] = (current, time.monotonic() + 120, origin)
        descriptor = {'server_id': FIXTURE['server_id'], 'origin': origin,
                      'endpoint': 'https://192.168.1.106:19979/media',
                      'hash': list(CERT.fingerprint(hashes.SHA256())), 'path': FIXTURE['path']}
        return web.json_response({'token': token, 'expires_seconds': 120, 'principal': who, 'descriptor': descriptor})
    if path == '/result' and request.method == 'POST' and request.headers.get('Origin') == origin:
        value = await request.json()
        # Results contain only fixture diagnostics, never credentials or lease tokens.
        print('BROWSER ' + json.dumps(value), flush=True)
        with (STATE / 'results.jsonl').open('a') as output:
            output.write(json.dumps(value) + '\n')
        return web.Response(text='Recorded')
    if path in ('/api/v1/auth/login', '/api/v1/auth/logout', '/api/v1/me'):
        if request.method not in ('GET', 'POST') or (request.method == 'POST' and request.headers.get('Origin') != origin):
            return web.Response(status=403)
        body = await request.read()
        async with HTTP.request(request.method, BACKEND + path, data=body,
                                headers={'Origin': origin, 'Cookie': cookie(request, public),
                                         'Content-Type': 'application/json'}, allow_redirects=False) as upstream:
            response = web.Response(status=upstream.status, body=await upstream.read(),
                                    content_type='application/json')
            for value in upstream.headers.getall('Set-Cookie', []):
                renamed = value.replace('collector_session=',
                                        'collector_wt_public_session=' if public else 'collector_wt_session=', 1)
                if public:
                    renamed = renamed.replace('Path=/', 'Path=' + prefix + '/')
                response.headers.add('Set-Cookie', renamed)
            return response
    return web.Response(status=404)


async def main():
    global HTTP, CERT, FIXTURE
    lifetime = int(os.environ.get('EXPERIMENT_LIFETIME_SECONDS', '7200'))
    if not 60 <= lifetime <= 86400:
        raise ValueError('experiment lifetime must be between one minute and one day')
    STATE.mkdir(parents=True, exist_ok=True)
    FIXTURE = json.loads((STATE / 'fixture.json').read_text())
    key = ec.generate_private_key(ec.SECP256R1())
    now = datetime.datetime.now(datetime.timezone.utc)
    name = x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, 'collector-lan-experiment')])
    CERT = (x509.CertificateBuilder().subject_name(name).issuer_name(name).public_key(key.public_key())
            .serial_number(x509.random_serial_number()).not_valid_before(now - datetime.timedelta(minutes=1))
            .not_valid_after(now + datetime.timedelta(days=7))
            .add_extension(x509.SubjectAlternativeName([x509.IPAddress(ipaddress.ip_address(BIND))]), critical=False)
            .sign(key, hashes.SHA256()))
    certpath, keypath = STATE / 'wt-cert.pem', STATE / 'wt-key.pem'
    certpath.write_bytes(CERT.public_bytes(serialization.Encoding.PEM))
    keypath.write_bytes(key.private_bytes(serialization.Encoding.PEM, serialization.PrivateFormat.PKCS8,
                                         serialization.NoEncryption()))
    os.chmod(keypath, 0o600)
    config = QuicConfiguration(is_client=False, alpn_protocols=H3_ALPN,
                              max_datagram_frame_size=65536, idle_timeout=30,
                              max_data=4 * 1024 * 1024, max_stream_data=512 * 1024)
    config.load_cert_chain(certpath, keypath)
    quic = await serve(BIND, 19979, configuration=config, create_protocol=Bridge)
    HTTP = ClientSession(timeout=ClientTimeout(total=10))
    async with HTTP.get(BACKEND + '/api/v1/identity') as response:
        if response.status != 200:
            raise RuntimeError('isolated Collector identity unavailable')
        identity = await response.json()
        FIXTURE['server_id'] = identity['server_id']
    app = web.Application(client_max_size=16384)
    app.router.add_route('*', '/{path:.*}', handler)
    runner = web.AppRunner(app, access_log=None)
    await runner.setup()
    tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    tls.load_cert_chain('/nas-cert/fullchain.pem', '/nas-cert/privkey.pem')
    await web.TCPSite(runner, BIND, 19978, ssl_context=tls).start()
    public_app = web.Application(client_max_size=16384)
    async def public_handler(request):
        return await handler(request, PUBLIC_ORIGIN, PUBLIC_PREFIX)
    public_app.router.add_route('*', '/{path:.*}', public_handler)
    public_runner = web.AppRunner(public_app, access_log=None)
    await public_runner.setup()
    await web.TCPSite(public_runner, '127.0.0.1', 19980).start()
    print('READY ' + ORIGIN + ' lifetime_seconds=' + str(lifetime), flush=True)
    stop = asyncio.Event()
    loop = asyncio.get_running_loop()
    for signum in (signal.SIGTERM, signal.SIGINT):
        loop.add_signal_handler(signum, stop.set)
    try:
        await asyncio.wait_for(stop.wait(), lifetime)
    except TimeoutError:
        pass
    finally:
        for protocol in list(PROTOCOLS):
            protocol.close()
            for task in protocol.tasks.values():
                task.cancel()
        quic.close()
        await runner.cleanup()
        await public_runner.cleanup()
        await HTTP.close()
        GRANTS.clear()
        keypath.unlink(missing_ok=True)
        certpath.unlink(missing_ok=True)


if __name__ == '__main__':
    asyncio.run(main())
