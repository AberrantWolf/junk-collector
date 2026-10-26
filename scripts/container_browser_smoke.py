#!/usr/bin/env python3
"""Run short browser acceptance against the exact release image, without debug builds."""
import argparse
import os
from pathlib import Path
import secrets
import socket
import ssl
import subprocess
import tempfile
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('image')
    args = parser.parse_args()
    name = 'collector-browser-' + secrets.token_hex(6)
    password = secrets.token_hex(24)
    with tempfile.TemporaryDirectory(prefix='collector-image-browser-') as temporary:
        root = Path(temporary)
        root.chmod(0o777)
        # Reserve an ordinary host port; fixtures are run serially.
        with socket.socket() as sock:
            sock.bind(('127.0.0.1', 0))
            port = sock.getsockname()[1]
        origin = f'https://localhost:{port}'
        config = root / 'collector.toml'
        config.write_text(f'archive = "/fixture/archive"\nstate = "/fixture/state"\npublic_bind = "127.0.0.1:19841"\nprivate_bind = "127.0.0.1:19842"\nlan_bind = "0.0.0.0:19843"\nlocal_playback_enabled = false\nmulti_user = true\nauthentication_origin = "{origin}"\nallowed_origins = ["{origin}"]\ntls_names = ["localhost"]\n[exposure]\npublic_curation = true\npublic_administration = true\n')
        config.chmod(0o666)  # setup updates this disposable fixture config as UID 10001.
        run = ['docker', 'run', '--rm', '-v', f'{root}:/fixture']
        try:
            subprocess.run([*run, '-i', args.image, '--config', '/fixture/collector.toml', 'setup', '--username', 'owner'],
                           input=password, text=True, check=True, capture_output=True, timeout=30)
            certificate = subprocess.check_output([*run, '--entrypoint', 'cat', args.image, '/fixture/state/tls/cert.pem'], text=True, timeout=30)
            trust = ssl.create_default_context(cadata=certificate)
            subprocess.run([*run, '-d', '--name', name, '-p', f'127.0.0.1:{port}:19843', args.image,
                            '--config', '/fixture/collector.toml', 'serve'], check=True, capture_output=True, timeout=30)
            until = time.monotonic() + 30
            while True:
                try:
                    with urllib.request.urlopen(origin + '/healthz', timeout=1, context=trust) as response:
                        if response.status == 200:
                            break
                except OSError:
                    pass
                if time.monotonic() > until:
                    raise RuntimeError('Release-image browser fixture failed readiness')
                time.sleep(.1)
            subprocess.run(['node', 'web/tests/quick-smoke.mjs'], cwd=ROOT,
                           env=dict(os.environ, COLLECTOR_TEST_ORIGIN=origin, COLLECTOR_TEST_PASSWORD=password, COLLECTOR_TEST_SELF_SIGNED='1'),
                           check=True, timeout=90)
        except BaseException:
            subprocess.run(['docker', 'logs', name], capture_output=False, timeout=10)
            raise
        finally:
            subprocess.run(['docker', 'rm', '-f', name], capture_output=True, timeout=30)
            # Fixture-owned files may be mode 0700 under the image's ordinary UID.
            subprocess.run(['docker', 'run', '--rm', '--user', '0', '-v', f'{root}:/fixture',
                            '--entrypoint', 'sh', args.image, '-c', 'rm -rf /fixture/archive /fixture/state'],
                           capture_output=True, check=True, timeout=30)


if __name__ == '__main__':
    try:
        main()
    except subprocess.CalledProcessError as error:
        if error.stderr:
            print(error.stderr)
        raise
