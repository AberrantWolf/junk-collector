#!/usr/bin/env python3
"""Build-once OCI evidence and authenticated GitHub artifact consumption."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import stat
import subprocess
import tarfile
import tempfile
import zipfile

import container_release as release

WORKFLOW = '.github/workflows/container-release.yml'
VALIDATION_TITLE = 'Validate container candidate'
MAX_ARCHIVE = 4 * 1024**3
MAX_JSON = 4 * 1024**2
CHUNK = 1024 * 1024


def sha_file(path):
    digest = hashlib.sha256()
    with path.open('rb') as source:
        while chunk := source.read(CHUNK):
            digest.update(chunk)
    return 'sha256:' + digest.hexdigest()


def json_file(path):
    if path.stat().st_size > MAX_JSON:
        raise ValueError('Oversized evidence JSON')
    return json.loads(path.read_text())


def blob_json(archive, digest):
    if not release.DIGEST.fullmatch(digest):
        raise ValueError('Invalid OCI digest')
    member = archive.getmember('blobs/sha256/' + digest[7:])
    if not member.isfile() or member.size > MAX_JSON:
        raise ValueError('Invalid OCI JSON blob')
    with archive.extractfile(member) as source:
        data = source.read(MAX_JSON + 1)
    if 'sha256:' + hashlib.sha256(data).hexdigest() != digest:
        raise ValueError('OCI JSON digest mismatch')
    return json.loads(data)


def native_descriptors(archive, digest, architecture):
    root = blob_json(archive, digest)
    if root.get('mediaType') in ('application/vnd.oci.image.index.v1+json',
                               'application/vnd.docker.distribution.manifest.list.v2+json'):
        descriptors = root['manifests']
    elif root.get('schemaVersion') == 2 and 'config' in root:
        descriptors = [dict(mediaType=root['mediaType'], digest=digest,
                            size=archive.getmember('blobs/sha256/' + digest[7:]).size,
                            platform=dict(os='linux', architecture=architecture))]
    else:
        raise ValueError('Unsupported native OCI root')
    runtime = [d for d in descriptors if d.get('platform', {}).get('os') == 'linux']
    if len(runtime) != 1 or runtime[0]['platform'].get('architecture') != architecture:
        raise ValueError('Native export must contain the intended runtime platform')
    return descriptors, blob_json(archive, runtime[0]['digest'])['config']['digest']


def assemble(platform_dir, destination):
    """Merge validated exports without changing their manifest/config/layer bytes."""
    with tempfile.TemporaryDirectory(prefix='collector-oci-') as temporary:
        root = Path(temporary)
        (root / 'blobs/sha256').mkdir(parents=True)
        manifests = []
        total_bytes = 0
        for architecture in ('amd64', 'arm64'):
            digest = json_file(platform_dir / (architecture + '.json'))['digest']
            path = platform_dir / ('collector-' + architecture + '.oci.tar')
            if path.stat().st_size > MAX_ARCHIVE:
                raise ValueError('Oversized native export')
            with tarfile.open(path, 'r:') as archive:
                seen = set()
                for count, member in enumerate(archive):
                    if count > 10000 or member.name in seen:
                        raise ValueError('Duplicate or excessive OCI members')
                    seen.add(member.name)
                    total_bytes += member.size
                    if total_bytes > MAX_ARCHIVE:
                        raise ValueError('Combined OCI exports exceed bounds')
                    if member.isdir() and member.name.rstrip('/') in ('blobs', 'blobs/sha256'):
                        continue
                    if not member.isfile() or member.size > MAX_ARCHIVE:
                        raise ValueError('Invalid OCI member')
                    if member.name in ('index.json', 'oci-layout'):
                        if member.size > MAX_JSON:
                            raise ValueError('Oversized OCI metadata')
                        continue
                    if not re.fullmatch(r'blobs/sha256/[0-9a-f]{64}', member.name):
                        raise ValueError('Invalid OCI blob path')
                    target = root / member.name
                    digest_check = hashlib.sha256()
                    with archive.extractfile(member) as source, target.open('wb') as output:
                        while chunk := source.read(CHUNK):
                            digest_check.update(chunk)
                            output.write(chunk)
                    if digest_check.hexdigest() != target.name:
                        raise ValueError('Invalid OCI blob digest')
                entry = archive.getmember('index.json')
                if entry.size > MAX_JSON:
                    raise ValueError('Oversized OCI index')
                with archive.extractfile(entry) as source:
                    index = json.load(source)
                if not any(entry['digest'] == digest for entry in index['manifests']):
                    raise ValueError('Export differs from native build output')
                descriptors, _ = native_descriptors(archive, digest, architecture)
                manifests.extend(descriptors)
        index = dict(schemaVersion=2, mediaType='application/vnd.oci.image.index.v1+json', manifests=manifests)
        release.platforms(index)
        encoded = json.dumps(index).encode()
        digest = 'sha256:' + hashlib.sha256(encoded).hexdigest()
        (root / 'blobs/sha256' / digest[7:]).write_bytes(encoded)
        (root / 'oci-layout').write_text('{"imageLayoutVersion":"1.0.0"}')
        (root / 'index.json').write_text(json.dumps(dict(schemaVersion=2,
            mediaType=index['mediaType'], manifests=[dict(mediaType=index['mediaType'], digest=digest,
            size=len(encoded), annotations={'org.opencontainers.image.ref.name': 'release'})])))
        with tarfile.open(destination, 'w') as output:
            for path in sorted(root.rglob('*')):
                if path.is_file():
                    info = output.gettarinfo(str(path), arcname=str(path.relative_to(root)))
                    info.uid = info.gid = info.mtime = 0
                    info.uname = info.gname = ''
                    with path.open('rb') as source:
                        output.addfile(info, source)
        return digest


def saved_config_digest(path):
    """Read exact config bytes from Docker's portable save format, across stores."""
    if path.stat().st_size > MAX_ARCHIVE:
        raise ValueError('Oversized daemon export')
    with tarfile.open(path, 'r:') as archive:
        manifest = archive.getmember('manifest.json')
        if not manifest.isfile() or manifest.size > MAX_JSON:
            raise ValueError('Invalid daemon save manifest')
        with archive.extractfile(manifest) as source:
            entries = json.load(source)
        if len(entries) != 1:
            raise ValueError('Smoke daemon export must contain exactly one runtime')
        name = entries[0]['Config']
        if not re.fullmatch(r'(?:blobs/sha256/)?[0-9a-f]{64}(?:\.json)?', name):
            raise ValueError('Invalid daemon config path')
        member = archive.getmember(name)
        if not member.isfile() or member.size > MAX_JSON:
            raise ValueError('Invalid daemon config blob')
        with archive.extractfile(member) as source:
            return 'sha256:' + hashlib.sha256(source.read(MAX_JSON + 1)).hexdigest()


def daemon_config_digest():
    # containerd-backed Docker identifies images by their target manifest rather
    # than the config ID used by the classic store. Compare actual saved config
    # bytes instead of assuming that inspect .Id always means config digest.
    with tempfile.TemporaryDirectory(prefix='collector-smoke-identity-') as temporary:
        path = Path(temporary) / 'daemon.tar'
        def bound_file():
            resource.setrlimit(resource.RLIMIT_FSIZE, (MAX_ARCHIVE, MAX_ARCHIVE))
        subprocess.run(['docker', 'image', 'save', '--output', str(path), 'junk-collector:release-smoke'],
                       check=True, timeout=180, preexec_fn=bound_file)
        return saved_config_digest(path)


def smoke_identity(archive_path, architecture):
    """The daemon must run the exact config/diffIDs exported by the one build."""
    with tarfile.open(archive_path) as archive:
        with archive.extractfile('index.json') as source:
            digest = json.load(source)['manifests'][0]['digest']
        _, config = native_descriptors(archive, digest, architecture)
    actual = release.command('docker', 'image', 'inspect', '--format', '{{.Id}}', 'junk-collector:release-smoke')
    if actual != config and daemon_config_digest() != config:
        raise ValueError('Smoke config bytes differ from the exported runtime image (daemon ID: ' + actual + ')')


def seal(expected, image, archive, run_id, attempt):
    if image.get('anonymous_docker_pulls') is not False or image.get('channel') is not None:
        raise ValueError('Validation must be nonpublishing')
    for name in ('version', 'revision', 'source', 'image'):
        if image.get(name) != expected[name]:
            raise ValueError('Validation image provenance mismatch')
    release.platforms(dict(schemaVersion=2, mediaType='application/vnd.oci.image.index.v1+json',
        manifests=[dict(digest=digest, platform=dict(zip(('os', 'architecture'), platform.split('/'))))
                   for platform, digest in image['platforms'].items()]))
    if not release.DIGEST.fullmatch(image['digest']) or archive.stat().st_size > MAX_ARCHIVE:
        raise ValueError('Invalid validated export')
    return dict(schema=1, repository=expected['source'].removeprefix('https://github.com/'),
                version=expected['version'], revision=expected['revision'], source=expected['source'],
                image=expected['image'], run_id=int(run_id), attempt=int(attempt),
                digest=image['digest'], platforms=image['platforms'],
                archive_sha256=sha_file(archive), archive_bytes=archive.stat().st_size)


def validate_platform_map(platforms):
    if not isinstance(platforms, dict) or set(platforms) != {'linux/amd64', 'linux/arm64'}:
        raise ValueError('Validated evidence needs both native platforms')
    if any(not isinstance(d, str) or not release.DIGEST.fullmatch(d) for d in platforms.values()):
        raise ValueError('Invalid validated platform digest')


def compare(evidence, image):
    validate_platform_map(evidence.get('platforms'))
    for name in ('version', 'revision', 'source', 'image', 'digest', 'platforms'):
        if evidence.get(name) != image.get(name):
            raise ValueError('Restored OCI bytes disagree with sealed evidence: ' + name)


def check_run(run, expected, repository_id, workflow_id):
    if (run.get('repository', {}).get('id') != repository_id
        or run.get('head_repository', {}).get('id') != repository_id
        or run.get('repository', {}).get('full_name') != expected['source'].removeprefix('https://github.com/')
        or run.get('workflow_id') != workflow_id or run.get('path') != WORKFLOW
        or run.get('head_sha') != expected['revision'] or run.get('event') != 'workflow_dispatch'
        or run.get('display_title') != VALIDATION_TITLE
        or run.get('status') != 'completed' or run.get('conclusion') != 'success'
        or type(run.get('id')) is not int or run['id'] < 1
        or type(run.get('run_attempt')) is not int or run['run_attempt'] < 1):
        raise ValueError('Validation run is not successful trusted exact-source validation')


def check_artifact(artifact, run, repository_id, now):
    workflow_run = artifact.get('workflow_run', {})
    if (artifact.get('name') != 'validated-container-' + str(run['run_attempt'])
        or artifact.get('expired') is not False or artifact.get('size_in_bytes', MAX_ARCHIVE + 1) > MAX_ARCHIVE
        or not release.DIGEST.fullmatch(artifact.get('digest', ''))
        or workflow_run.get('id') != run['id'] or workflow_run.get('head_sha') != run['head_sha']
        or workflow_run.get('repository_id') != repository_id or workflow_run.get('head_repository_id') != repository_id
        or datetime.fromisoformat(artifact['expires_at'].replace('Z', '+00:00')) <= now):
        raise ValueError('Validation artifact is expired, mismatched or missing its immutable digest')


def unpack(path, destination, artifact):
    if sha_file(path) != artifact['digest']:
        raise ValueError('Downloaded GitHub artifact digest mismatch')
    with zipfile.ZipFile(path) as archive:
        members = archive.infolist()
        if len(members) != 2 or {m.filename for m in members} != {'validation.json', 'collector-multiarch.oci.tar'}:
            raise ValueError('Unexpected validation artifact members')
        for member in members:
            kind = stat.S_IFMT(member.external_attr >> 16)
            limit = MAX_JSON if member.filename == 'validation.json' else MAX_ARCHIVE
            if kind not in (0, stat.S_IFREG) or member.file_size > limit:
                raise ValueError('Invalid artifact member type or size')
            with archive.open(member) as source, (destination / member.filename).open('xb') as output:
                remaining = limit
                while chunk := source.read(min(CHUNK, remaining + 1)):
                    remaining -= len(chunk)
                    if remaining < 0:
                        raise ValueError('Expanded artifact exceeds bounds')
                    output.write(chunk)


def check_evidence(evidence, expected, run, archive):
    validate_platform_map(evidence.get('platforms'))
    if evidence.get('schema') != 1 or evidence.get('repository') != expected['source'].removeprefix('https://github.com/'):
        raise ValueError('Unsupported validation evidence')
    for name in ('version', 'revision', 'source', 'image'):
        if evidence.get(name) != expected[name]:
            raise ValueError('Validation evidence disagrees with release ' + name)
    if (evidence.get('run_id') != run['id'] or evidence.get('attempt') != run['run_attempt']
        or not release.DIGEST.fullmatch(evidence.get('digest', ''))
        or evidence.get('archive_bytes') != archive.stat().st_size
        or evidence.get('archive_sha256') != sha_file(archive)):
        raise ValueError('Validation evidence run/attempt/archive mismatch')


def api(path):
    data = subprocess.check_output(['gh', 'api', path], timeout=60)
    if len(data) > 16 * MAX_JSON:
        raise ValueError('Oversized GitHub metadata')
    return json.loads(data)


def restore(expected, destination, selected):
    repository = expected['source'].removeprefix('https://github.com/')
    repository_id = api('repos/' + repository)['id']
    workflow_id = api('repos/' + repository + '/actions/workflows/container-release.yml')['id']
    prefix = 'repos/' + repository + '/actions/'
    if selected:
        if not re.fullmatch(r'[1-9][0-9]{0,19}', selected):
            raise ValueError('Expected a numeric validation run ID')
        run_id = int(selected)
    else:
        run_id = None
        for page in range(1, 11):
            runs = api(prefix + 'workflows/container-release.yml/runs?event=workflow_dispatch&status=success&head_sha=' + expected['revision'] + '&per_page=100&page=' + str(page))['workflow_runs']
            found = next((r for r in runs if r.get('display_title') == VALIDATION_TITLE), None)
            if found:
                run_id = found['id']
                break
            if len(runs) < 100:
                break
        if run_id is None:
            raise ValueError('No successful exact-source validation. Dispatch validation before publication.')
    run = api(prefix + 'runs/' + str(run_id))
    check_run(run, expected, repository_id, workflow_id)
    name = 'validated-container-' + str(run['run_attempt'])
    artifacts = []
    for page in range(1, 11):
        entries = api(prefix + 'runs/' + str(run_id) + '/artifacts?per_page=100&page=' + str(page))['artifacts']
        artifacts.extend(a for a in entries if a.get('name') == name)
        if len(entries) < 100:
            break
    if len(artifacts) != 1:
        raise ValueError('Missing/ambiguous validation artifact. Explicitly run fresh validation.')
    artifact = artifacts[0]
    check_artifact(artifact, run, repository_id, datetime.now(timezone.utc))
    if destination.exists():
        raise ValueError('Restore destination must be fresh')
    destination.parent.mkdir(parents=True, exist_ok=True)
    # Failed downloads/evidence checks leave no apparently restored candidate.
    with tempfile.TemporaryDirectory(prefix='collector-validation-', dir=destination.parent) as temporary:
        staging = Path(temporary)
        download = staging / 'artifact.zip'
        recovered = staging / 'candidate'
        recovered.mkdir()
        def bound_file():
            resource.setrlimit(resource.RLIMIT_FSIZE, (MAX_ARCHIVE, MAX_ARCHIVE))
        with download.open('wb') as output:
            subprocess.run(['gh', 'api', prefix + 'artifacts/' + str(artifact['id']) + '/zip'],
                           stdout=output, check=True, timeout=600, preexec_fn=bound_file)
        unpack(download, recovered, artifact)
        evidence = json_file(recovered / 'validation.json')
        check_evidence(evidence, expected, run, recovered / 'collector-multiarch.oci.tar')
        provenance = dict(run_id=run['id'], attempt=run['run_attempt'], artifact_id=artifact['id'], artifact_digest=artifact['digest'])
        (recovered / 'provenance.json').write_text(json.dumps(provenance, indent=2) + '\n')
        current = api(prefix + 'runs/' + str(run_id))
        check_run(current, expected, repository_id, workflow_id)
        if current['run_attempt'] != run['run_attempt']:
            raise ValueError('Validation was rerun during artifact recovery; retry after completion')
        recovered.rename(destination)
    return evidence


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='operation', required=True)
    merged = sub.add_parser('assemble')
    merged.add_argument('platforms', type=Path)
    merged.add_argument('archive', type=Path)
    smoke = sub.add_parser('smoke-identity')
    smoke.add_argument('archive', type=Path)
    smoke.add_argument('architecture', choices=('amd64', 'arm64'))
    sub.add_parser('seal')
    record = sub.add_parser('record-provenance')
    record.add_argument('restored', type=Path)
    compared = sub.add_parser('compare')
    compared.add_argument('evidence', type=Path)
    recovered = sub.add_parser('restore')
    recovered.add_argument('destination', type=Path)
    args = parser.parse_args()
    expected = json_file(Path('release-plan.json'))
    if args.operation == 'assemble':
        digest = assemble(args.platforms, args.archive)
        with open(os.environ['GITHUB_OUTPUT'], 'a') as output:
            output.write('digest=' + digest + '\n')
    elif args.operation == 'smoke-identity':
        smoke_identity(args.archive, args.architecture)
    elif args.operation == 'record-provenance':
        image_path = Path('release-image.json')
        image = json_file(image_path)
        evidence = json_file(args.restored / 'validation.json')
        compare(evidence, image)
        image['validation'] = dict(json_file(args.restored / 'provenance.json'),
                                   archive_sha256=evidence['archive_sha256'])
        image_path.write_text(json.dumps(image, indent=2) + '\n')
    elif args.operation == 'compare':
        compare(json_file(args.evidence), json_file(Path('release-image.json')))
    elif args.operation == 'seal':
        evidence = seal(expected, json_file(Path('release-image.json')), Path('/tmp/collector-multiarch.oci.tar'), os.environ['GITHUB_RUN_ID'], os.environ['GITHUB_RUN_ATTEMPT'])
        Path('/tmp/validation').mkdir(exist_ok=False)
        Path('/tmp/validation/validation.json').write_text(json.dumps(evidence, indent=2) + '\n')
        shutil.move('/tmp/collector-multiarch.oci.tar', '/tmp/validation/collector-multiarch.oci.tar')
    else:
        evidence = restore(expected, args.destination, os.environ.get('VALIDATION_RUN_ID', ''))
        with open(os.environ['GITHUB_OUTPUT'], 'a') as output:
            output.write('digest=' + evidence['digest'] + '\n')


if __name__ == '__main__':
    main()
