"""Independent byte/authorization probe against the disposable NAS fixture only."""
import argparse
import asyncio
import hashlib
import http.cookiejar
import json
import ipaddress
from pathlib import Path
import socket
import urllib.error
import urllib.request

from aioquic.asyncio import QuicConnectionProtocol, connect
from aioquic.h3.connection import H3Connection, H3_ALPN
from aioquic.h3.events import HeadersReceived, WebTransportStreamDataReceived
from aioquic.quic.configuration import QuicConfiguration
from aioquic.quic.events import ProtocolNegotiated, StreamDataReceived
from cryptography import x509
from cryptography.hazmat.primitives import hashes

ORIGIN = 'https://collector-lan.awkwardexplorer.com:19978'
PUBLIC_ORIGIN = 'https://collector.awkwardexplorer.com'
PUBLIC_PREFIX = '/__collector_lan_test_20261022'


class Client(QuicConnectionProtocol):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.http = None
        self.responses = {}
        self.header_waiters = {}

    def quic_event_received(self, event):
        # Responses on our own bidirectional WT streams have no second HTTP/3
        # preface. This client already knows their session association.
        if isinstance(event, StreamDataReceived) and event.stream_id in self.responses:
            waiter, body = self.responses[event.stream_id]
            body.extend(event.data)
            if len(body) > 270336:
                raise RuntimeError('response bound')
            if event.end_stream and not waiter.done():
                waiter.set_result(bytes(body))
            return
        if isinstance(event, ProtocolNegotiated):
            self.http = H3Connection(self._quic, enable_webtransport=True)
        if not self.http:
            return
        for value in self.http.handle_event(event):
            if isinstance(value, HeadersReceived):
                waiter = self.header_waiters.get(value.stream_id)
                if waiter and not waiter.done():
                    waiter.set_result(dict(value.headers))
            elif isinstance(value, WebTransportStreamDataReceived):
                waiter, body = self.responses[value.stream_id]
                body.extend(value.data)
                if len(body) > 270336:
                    raise RuntimeError('response bound')
                if value.stream_ended and not waiter.done():
                    waiter.set_result(bytes(body))

    async def session(self, origin=ORIGIN):
        stream = self._quic.get_next_available_stream_id()
        future = asyncio.get_running_loop().create_future()
        self.header_waiters[stream] = future
        self.http.send_headers(stream, [(b':method', b'CONNECT'), (b':scheme', b'https'),
            (b':authority', b'192.168.1.106:19979'), (b':path', b'/media'),
            (b':protocol', b'webtransport'), (b'origin', origin.encode())])
        self.transmit()
        headers = await asyncio.wait_for(future, 10)
        return stream, int(headers[b':status'])

    async def read(self, session, value):
        stream = self.http.create_webtransport_stream(session, is_unidirectional=False)
        future = asyncio.get_running_loop().create_future()
        self.responses[stream] = (future, bytearray())
        self._quic.send_stream_data(stream, json.dumps(value).encode(), end_stream=True)
        self.transmit()
        try:
            result = await asyncio.wait_for(future, 12)
            envelope, body = result.split(b'\n', 1)
            return json.loads(envelope), body
        finally:
            self.responses.pop(stream, None)


async def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--certificate', required=True, type=Path)
    parser.add_argument('--password-file', required=True, type=Path)
    parser.add_argument('--audio', required=True, type=Path)
    parser.add_argument('--public-ip', help='Use the public origin through this publicly resolved address, only in this probe process')
    args = parser.parse_args()
    origin = PUBLIC_ORIGIN if args.public_ip else ORIGIN
    base = origin + (PUBLIC_PREFIX if args.public_ip else '')
    if args.public_ip:
        if not ipaddress.ip_address(args.public_ip).is_global:
            raise ValueError('public-origin probe requires a public IP')
        original_lookup = socket.getaddrinfo
        def public_lookup(host, port, *rest, **options):
            return original_lookup(args.public_ip if host == 'collector.awkwardexplorer.com' else host,
                                   port, *rest, **options)
        socket.getaddrinfo = public_lookup
    jar = http.cookiejar.CookieJar()
    client = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar), urllib.request.ProxyHandler({}))

    def request(path, value=None):
        req = urllib.request.Request(base + path, data=json.dumps(value).encode() if value is not None else None,
                                     headers={'Origin': origin, 'Content-Type': 'application/json'})
        try:
            with client.open(req, timeout=10) as response:
                data = response.read()
                return response.status, json.loads(data) if data else None
        except urllib.error.HTTPError as error:
            return error.code, None

    code, fixture = request('/fixture')
    if code != 200:
        raise RuntimeError(f'control descriptor HTTP {code}; no media probe was attempted')
    certificate = x509.load_pem_x509_certificate(args.certificate.read_bytes())
    assert list(certificate.fingerprint(hashes.SHA256())) == fixture['hash'], 'authenticated descriptor identity mismatch'
    # Standard chain/hostname validation with exactly the authenticated disposable certificate.
    config = QuicConfiguration(is_client=True, alpn_protocols=H3_ALPN, max_datagram_frame_size=65536)
    config.load_verify_locations(cafile=str(args.certificate))
    password = args.password_file.read_text().strip()
    assert request('/api/v1/auth/login', {'username': 'listener', 'password': password})[0] == 200
    _, grant = request('/grant', {})
    assert grant['descriptor']['origin'] == origin
    assert grant['descriptor']['server_id'] == fixture['server_id']
    assert grant['descriptor']['hash'] == fixture['hash']
    results = {}
    async with connect('192.168.1.106', 19979, configuration=config, create_protocol=Client) as protocol:
        session, code = await protocol.session(origin)
        assert code == 200
        async def read(method='HEAD', path=fixture['path'], token=grant['token'], byte_range=''):
            return await protocol.read(session, {'method': method, 'path': path, 'token': token, 'range': byte_range})
        assert (await read(token=''))[0]['status'] == 401
        results['missing_grant'] = 401
        assert (await read(path='/api/v1/admin/users'))[0]['status'] == 403
        results['path_confinement'] = 403
        assert (await read(method='GET', byte_range='bytes=0-262144'))[0]['status'] == 416
        results['oversized_range'] = 416
        samples = [(0, 65536), (21168078, 262144), (args.audio.stat().st_size - 65536, 65536)]
        results['exact_bytes'] = []
        with args.audio.open('rb') as source:
            for offset, length in samples:
                response, body = await read(method='GET', byte_range=f'bytes={offset}-{offset + length - 1}')
                source.seek(offset)
                expected = source.read(length)
                assert response['status'] == 206 and body == expected
                results['exact_bytes'].append({'offset': offset, 'length': length, 'sha256': hashlib.sha256(body).hexdigest()})
        assert request(fixture['path'])[0] == 410
        results['ordinary_https_media'] = 410
        assert request('/api/v1/auth/logout', {})[0] == 204
        assert (await read())[0]['status'] == 401
        results['revoked_real_session'] = 401
        assert request('/api/v1/auth/login', {'username': 'denied', 'password': password})[0] == 200
        _, denied = request('/grant', {})
        assert (await read(token=denied['token']))[0]['status'] == 403
        results['cross_collection_denial'] = 403
        other_origin = ORIGIN if args.public_ip else PUBLIC_ORIGIN
        other_session, code = await protocol.session(other_origin)
        assert code == 200
        other_response, _ = await protocol.read(other_session, {'method': 'HEAD', 'path': fixture['path'],
                                                               'token': denied['token']})
        assert other_response['status'] == 401
        results['cross_origin_grant_denial'] = 401
        await asyncio.sleep(0)
    async with connect('192.168.1.106', 19979, configuration=config, create_protocol=Client) as protocol:
        _, code = await protocol.session('https://unrelated.invalid')
        assert code == 403
        results['wrong_origin'] = 403
    print(json.dumps(results, indent=2))


if __name__ == '__main__':
    asyncio.run(main())
