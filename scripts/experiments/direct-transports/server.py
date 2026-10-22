"""Loopback-only transport primitive experiment; not Collector playback."""
import asyncio
import datetime
import ipaddress
import json
import os
from pathlib import Path
import signal
import ssl
import tempfile

from aioquic.asyncio import QuicConnectionProtocol, serve
from aioquic.h3.connection import H3Connection, H3_ALPN
from aioquic.h3.events import HeadersReceived, WebTransportStreamDataReceived
from aioquic.quic.configuration import QuicConfiguration
from aioquic.quic.events import ProtocolNegotiated
from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.x509.oid import NameOID

ROOT = Path(tempfile.mkdtemp(prefix='collector-direct-transport-'))
os.chmod(ROOT, 0o700)
ORIGIN = 'http://localhost:19978'

class Experiment(QuicConnectionProtocol):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.http = None
        self.sessions = set()
        self.total = 0

    def quic_event_received(self, event):
        if isinstance(event, ProtocolNegotiated):
            self.http = H3Connection(self._quic, enable_webtransport=True)
        if self.http is None:
            return
        for item in self.http.handle_event(event):
            if isinstance(item, HeadersReceived):
                headers = dict(item.headers)
                approved = (headers.get(b':method') == b'CONNECT'
                            and headers.get(b':protocol') == b'webtransport'
                            and headers.get(b':path') == b'/experiment'
                            and headers.get(b'origin') == ORIGIN.encode())
                if approved:
                    self.sessions.add(item.stream_id)
                    self.http.send_headers(item.stream_id, [(b':status', b'200')])
                else:
                    self.http.send_headers(item.stream_id, [(b':status', b'403')], end_stream=True)
            elif isinstance(item, WebTransportStreamDataReceived):
                self.total += len(item.data)
                if item.session_id not in self.sessions or self.total > 1024 * 1024:
                    self.close(error_code=1, reason_phrase='experiment bound')
                    continue
                self._quic.send_stream_data(item.stream_id, item.data, end_stream=item.stream_ended)
            self.transmit()

async def http(reader, writer, tls=False):
    try:
        raw = await asyncio.wait_for(reader.readuntil(b'\r\n\r\n'), 5)
        if len(raw) > 8192:
            return
        lines = raw.decode('ascii').split('\r\n')
        method, path, _ = lines[0].split(' ')
        headers = dict(line.lower().split(': ', 1) for line in lines[1:] if ': ' in line)
        length = int(headers.get('content-length', 0))
        if length > 65536:
            return
        if method == 'POST' and path == '/result' and not tls:
            value = json.loads(await asyncio.wait_for(reader.readexactly(length), 5))
            name = f'result-{len(list(ROOT.glob("result-*.json"))) + 1}.json'
            (ROOT / name).write_text(json.dumps(value, indent=2) + '\n')
            print('RESULT ' + json.dumps(value), flush=True)
            body, content_type = b'OK', 'text/plain'
        elif method == 'GET' and path == '/config' and not tls:
            body = json.dumps({'hash': list(CERT.fingerprint(hashes.SHA256()))}).encode()
            content_type = 'application/json'
        elif method == 'GET' and path == '/' and not tls:
            body = Path(__file__).with_name('browser.html').read_bytes()
            content_type = 'text/html; charset=utf-8'
        else:
            body, content_type = b'bounded experiment', 'text/plain'
        writer.write(('HTTP/1.1 200 OK\r\nContent-Type: ' + content_type
                      + '\r\nContent-Length: ' + str(len(body))
                      + '\r\nAccess-Control-Allow-Origin: ' + ORIGIN
                      + '\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n').encode() + body)
        await writer.drain()
    except (Exception, asyncio.CancelledError):
        pass
    finally:
        writer.close()

async def main():
    global CERT
    key = ec.generate_private_key(ec.SECP256R1())
    now = datetime.datetime.now(datetime.timezone.utc)
    name = x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, 'localhost')])
    CERT = (x509.CertificateBuilder().subject_name(name).issuer_name(name)
            .public_key(key.public_key()).serial_number(x509.random_serial_number())
            .not_valid_before(now - datetime.timedelta(minutes=1))
            .not_valid_after(now + datetime.timedelta(days=7))
            .add_extension(x509.SubjectAlternativeName([x509.DNSName('localhost'),
                x509.IPAddress(ipaddress.ip_address('127.0.0.1'))]), critical=False)
            .sign(key, hashes.SHA256()))
    cert_path, key_path = ROOT / 'cert.pem', ROOT / 'key.pem'
    cert_path.write_bytes(CERT.public_bytes(serialization.Encoding.PEM))
    with key_path.open('xb') as handle:
        os.chmod(key_path, 0o600)
        handle.write(key.private_bytes(serialization.Encoding.PEM,
            serialization.PrivateFormat.PKCS8, serialization.NoEncryption()))
    configuration = QuicConfiguration(is_client=False, alpn_protocols=H3_ALPN,
        max_datagram_frame_size=65536)
    configuration.load_cert_chain(cert_path, key_path)
    quic = await serve('127.0.0.1', 19979, configuration=configuration, create_protocol=Experiment)
    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(cert_path, key_path)
    clear = await asyncio.start_server(http, '127.0.0.1', 19978, limit=8192)
    encrypted = await asyncio.start_server(lambda r, w: http(r, w, True),
                                          '127.0.0.1', 19980, ssl=context, limit=8192)
    print('Loopback experiment: ' + ORIGIN + '/; artifacts: ' + str(ROOT), flush=True)
    stop = asyncio.Event()
    loop = asyncio.get_running_loop()
    for signum in (signal.SIGTERM, signal.SIGINT):
        loop.add_signal_handler(signum, stop.set)
    try:
        async with clear, encrypted:
            try:
                await asyncio.wait_for(stop.wait(), 1800)
            except TimeoutError:
                pass
    finally:
        quic.close()
        key_path.unlink(missing_ok=True)
        cert_path.unlink(missing_ok=True)

if __name__ == '__main__':
    asyncio.run(main())
