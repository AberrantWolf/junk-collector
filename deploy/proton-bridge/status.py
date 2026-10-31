#!/usr/bin/env python3
"""Run the pinned official CLI; publish only bounded, non-identifying sync events.

No raw Bridge output is logged or retained here. The official CLI remains available
through the separate operator login command while this daemon is stopped.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import selectors
import signal
import ssl
import subprocess
import tempfile
import time

ANSI = re.compile(r'\x1b\[[0-?]*[ -/]*[@-~]')
PROGRESS = re.compile(r'^Sync \((.{1,320})\): ([0-9]+\.[0-9]+)% \(Elapsed: ([0-9]+\.[0-9]+)s, ETA: ([0-9]+\.[0-9]+)s\)$')
EVENT = re.compile(r'^A sync has (begun|finished) for (.{1,320})\.$')


class Status:
    def __init__(self):
        self.accounts = {}
        self.phase = 'unknown'
        self.event_at = None

    def consume(self, line, now):
        line = ANSI.sub('', line).replace('\x08', '').strip()
        # ishell's prompt may precede the first asynchronous event on a line.
        if line.startswith('>>> '):
            line = line[4:].lstrip()
        event = EVENT.fullmatch(line)
        progress = PROGRESS.fullmatch(line)
        if event:
            kind, account = event.groups()
            if account not in self.accounts and len(self.accounts) >= 16:
                self.phase = 'unknown'
                return
            self.accounts[account] = ('syncing' if kind == 'begun' else 'synced', None, None, None)
        elif progress:
            account, percent, elapsed, remaining = progress.groups()
            percent, elapsed, remaining = map(float, (percent, elapsed, remaining))
            if not 0 <= percent <= 100 or max(elapsed, remaining) > 315360000:
                return
            if account not in self.accounts and len(self.accounts) >= 16:
                self.phase = 'unknown'
                return
            self.accounts[account] = ('syncing', percent, elapsed, remaining)
        elif re.search(r'\bSync aborted\s+error=', line):
            # The log's opaque user ID cannot safely be equated with CLI names.
            # Invalidate every observed account until new events disambiguate it.
            self.accounts = {name: ('unknown', None, None, None) for name in self.accounts}
            self.phase = 'interrupted'
            self.event_at = now
            return
        else:
            return
        self.event_at = now
        self.phase = ('unknown' if any(v[0] == 'unknown' for v in self.accounts.values()) else
                      'syncing' if any(v[0] == 'syncing' for v in self.accounts.values()) else 'synced')

    def snapshot(self, now, certificate=None):
        active = [v for v in self.accounts.values() if v[0] == 'syncing']
        # No invented aggregate percentage across multiple accounts.
        values = active[0][1:] if self.phase == 'syncing' and len(self.accounts) == 1 else (None, None, None)
        return dict(schema=1, observed_at=now, event_at=self.event_at, phase=self.phase,
                    imap_port=int(os.environ.get('BRIDGE_IMAP_PORT', '1143')),
                    percent=values[0], elapsed_seconds=values[1], remaining_seconds=values[2],
                    certificate_sha256=certificate)


def publish(directory, snapshot):
    fd, pending = tempfile.mkstemp(prefix='.status-', dir=directory)
    try:
        with os.fdopen(fd, 'w') as output:
            json.dump(snapshot, output, allow_nan=False)
            output.write('\n')
            output.flush()
            os.fsync(output.fileno())
        os.replace(pending, directory / 'status.json')
    finally:
        if os.path.exists(pending):
            os.unlink(pending)


def certificate_digest(path):
    try:
        if path.stat().st_size > 16384:
            return None
        with path.open('rb') as source:
            data = source.read(16385)
        if len(data) > 16384:
            return None
        return hashlib.sha256(ssl.PEM_cert_to_DER_cert(data.decode('ascii'))).hexdigest()
    except (OSError, ValueError):
        return None


def reap(child, selector, requested, timeout, kill_timeout):
    """Drain shutdown output so a full pipe cannot prevent Bridge flushing state."""
    def send(number):
        try:
            os.killpg(child.pid, number)
        except ProcessLookupError:
            pass  # The direct child is still reaped below.
    if child.poll() is None:
        if requested:
            try:
                child.stdin.write(b'exit\n')
                child.stdin.flush()
            except (BrokenPipeError, OSError):
                send(signal.SIGTERM)
        else:
            send(signal.SIGTERM)
    forced = False
    deadline = time.monotonic() + timeout
    while child.poll() is None:
        if time.monotonic() >= deadline:
            if forced:
                raise subprocess.TimeoutExpired(child.args, kill_timeout)
            send(signal.SIGKILL)
            forced = True
            deadline = time.monotonic() + kill_timeout
        for key, _events in selector.select(timeout=0.05):
            if not os.read(key.fd, 4096):
                selector.unregister(key.fileobj)
    child.wait()
    return not forced and child.returncode == 0


def main(command=('/usr/bin/protonmail-bridge', '--cli'), shutdown_timeout=20, reap_timeout=5):
    os.umask(0o077)
    directory = Path(os.environ.get('BRIDGE_STATUS_DIRECTORY', '/run/bridge-status'))
    directory.mkdir(mode=0o700, parents=True, exist_ok=True)
    export = Path.home() / 'export'
    export.mkdir(mode=0o700, exist_ok=True)
    status = Status()
    status.phase = 'starting'
    publish(directory, status.snapshot(int(time.time())))
    stopping = False

    def stop(_signal, _frame):
        nonlocal stopping
        stopping = True

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    selector = selectors.DefaultSelector()
    try:
        child = subprocess.Popen(command, stdin=subprocess.PIPE,
                                 stdout=subprocess.PIPE, stderr=subprocess.STDOUT, start_new_session=True)
    except BaseException:
        selector.close()
        raise
    buffer = bytearray()
    discard = False
    next_publish = 0
    try:
        selector.register(child.stdout, selectors.EVENT_READ)
        # Export only inside Bridge's protected home. Collector receives only
        # the public fingerprint. Initialization failures also reap the child.
        child.stdin.write(('cert export\n' + str(export) + '\n').encode())
        child.stdin.flush()
        while not stopping and child.poll() is None:
            for _key, _events in selector.select(timeout=0.5):
                chunk = os.read(child.stdout.fileno(), 4096)
                if not chunk:
                    raise RuntimeError('Bridge status stream ended')
                for byte in chunk:
                    if byte in (10, 13):
                        if not discard:
                            status.consume(buffer.decode('utf-8', errors='replace'), int(time.time()))
                        buffer.clear()
                        discard = False
                    elif not discard:
                        if len(buffer) >= 4096:
                            buffer.clear()
                            discard = True
                        else:
                            buffer.append(byte)
            if time.monotonic() >= next_publish:
                if status.phase == 'starting':
                    status.phase = 'unknown'
                publish(directory, status.snapshot(int(time.time()), certificate_digest(export / 'cert.pem')))
                next_publish = time.monotonic() + 5
    finally:
        try:
            clean = reap(child, selector, stopping, shutdown_timeout, reap_timeout)
        finally:
            selector.close()
        status.accounts.clear()
        status.phase = 'stopped'
        publish(directory, status.snapshot(int(time.time()), certificate_digest(export / 'cert.pem')))
    return 0 if stopping and clean else 1


if __name__ == '__main__':
    raise SystemExit(main())
