#!/usr/bin/env python3
"""Independent provenance, byte identity and failed-recovery controls."""
import copy
from datetime import datetime, timezone
import hashlib
import io
import json
import os
import shutil
import subprocess
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import container_release as release
import container_validation as validation
import check_changes


def digest(data):
    return 'sha256:' + hashlib.sha256(data).hexdigest()


def fixture(path, architecture, *, bad_blob=False, extra=None):
    """Small real OCI graph, rather than a successful-looking tool response."""
    blobs = {}
    def blob(value):
        data = json.dumps(value).encode()
        key = digest(data)
        blobs['blobs/sha256/' + key[7:]] = data
        return dict(mediaType='application/vnd.oci.image.manifest.v1+json', digest=key, size=len(data))
    config = blob(dict(architecture=architecture, os='linux', rootfs=dict(type='layers', diff_ids=[])))
    config['mediaType'] = 'application/vnd.oci.image.config.v1+json'
    runtime = blob(dict(schemaVersion=2, mediaType='application/vnd.oci.image.manifest.v1+json', config=config, layers=[]))
    runtime['platform'] = dict(os='linux', architecture=architecture)
    index = blob(dict(schemaVersion=2, mediaType='application/vnd.oci.image.index.v1+json', manifests=[runtime]))
    index['mediaType'] = 'application/vnd.oci.image.index.v1+json'
    blobs['index.json'] = json.dumps(dict(schemaVersion=2, manifests=[index])).encode()
    blobs['oci-layout'] = b'{"imageLayoutVersion":"1.0.0"}'
    if bad_blob:
        blobs['blobs/sha256/' + config['digest'][7:]] = b'changed'
    if extra:
        blobs[extra] = b'unexpected'
    with tarfile.open(path, 'w') as archive:
        for name, data in blobs.items():
            member = tarfile.TarInfo(name)
            member.size = len(data)
            archive.addfile(member, io.BytesIO(data))
    return index['digest'], runtime['digest'], config['digest']


class ValidationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.expected = release.plan('Owner/collector', 'a' * 40, '0.1.0-beta.5', 'workflow_dispatch', {})
        self.run = dict(id=123, repository=dict(id=7, full_name='Owner/collector'), head_repository=dict(id=7),
                        workflow_id=19, path=validation.WORKFLOW, head_sha='a' * 40,
                        event='workflow_dispatch', display_title=validation.VALIDATION_TITLE,
                        status='completed', conclusion='success', run_attempt=2)
        self.artifact = dict(id=80, name='validated-container-2', expired=False, size_in_bytes=100,
                             digest='sha256:' + 'b' * 64, expires_at='2099-01-01T00:00:00Z',
                             workflow_run=dict(id=123, head_sha='a' * 40, repository_id=7, head_repository_id=7))
        self.image = dict(self.expected, digest='sha256:' + 'c' * 64,
                          platforms={'linux/amd64': 'sha256:' + 'd' * 64, 'linux/arm64': 'sha256:' + 'e' * 64},
                          anonymous_docker_pulls=False)
        self.archive = self.root / 'collector-multiarch.oci.tar'
        self.archive.write_bytes(b'OCI archive bytes')
        self.checks = dict(policy=check_changes.POLICY, revision='a'*40, dirty=False, source_unchanged=True, status='passed',
                           classification='infra', level='full', coverage='full', paths=['Dockerfile'],
                           steps=[dict(step, status='passed') for step in check_changes.select(['Dockerfile'], 'infra', 'full')[1]])
        self.platform_checks = {arch:dict(runtime='passed',browser='passed' if arch == 'amd64' else 'covered-on-amd64') for arch in ('amd64','arm64')}
        self.evidence = validation.seal(self.expected, self.image, self.archive, '123', '2', self.checks, self.platform_checks)

    def test_release_level_and_native_browser_prerequisites_are_enforced(self):
        for field, value in [('status','failed'), ('dirty',True), ('revision','b'*40)]:
            with self.subTest(field=field), self.assertRaises(ValueError):
                validation.seal(self.expected, self.image, self.archive, '123', '2',
                                dict(self.checks, **{field:value}), self.platform_checks)
        for architecture in ('amd64','arm64'):
            for field in ('runtime','browser'):
                results = copy.deepcopy(self.platform_checks);results[architecture][field]='skipped'
                with self.assertRaises(ValueError):
                    validation.seal(self.expected, self.image, self.archive, '123', '2', self.checks, results)
        paths=['web/src/app.tsx']
        changed=dict(self.checks, classification='ui-copy', coverage='changed', paths=paths,
                     steps=[dict(step,status='passed') for step in check_changes.select(paths,'ui-copy','full')[1]])
        # Beta accepts an explicitly reviewed copy-only scope. RC/stable require
        # applicable full functional evidence; a successful beta run is insufficient.
        validation.check_qualification(self.expected, changed, self.platform_checks)
        for version in ('0.1.0-rc.1','0.1.0'):
            with self.assertRaises(ValueError):
                validation.check_qualification(dict(self.expected,version=version), changed, self.platform_checks)

    def test_source_version_and_run_attempt_bind_validation(self):
        validation.check_run(self.run, self.expected, 7, 19)
        validation.check_evidence(self.evidence, self.expected, self.run, self.archive)
        public = release.plan('Owner/collector', 'a' * 40, '0.1.0-beta.5', 'release',
                              dict(tag_name='v0.1.0-beta.5', draft=False, prerelease=True))
        # A dry-run reference/channel is deliberately different from publication.
        validation.check_evidence(self.evidence, public, self.run, self.archive)
        for field, value in [('version', '0.1.0-beta.6'), ('revision', 'b' * 40), ('source', 'https://github.com/Other/repo'),
                             ('image', 'ghcr.io/other/repo'), ('attempt', 1), ('run_id', 124), ('schema', 1),
                             ('archive_sha256', 'sha256:' + '0' * 64), ('archive_bytes', 0), ('platforms', {})]:
            with self.subTest(field=field), self.assertRaises(ValueError):
                validation.check_evidence(dict(self.evidence, **{field: value}), self.expected, self.run, self.archive)
        self.archive.write_bytes(b'Tampered archive')
        with self.assertRaises(ValueError):
            validation.check_evidence(self.evidence, self.expected, self.run, self.archive)

    def test_untrusted_failed_partial_and_wrong_workflow_runs_are_rejected(self):
        for field, value in [('repository', dict(id=8, full_name='Owner/collector')), ('head_repository', dict(id=8)),
                             ('workflow_id', 20), ('path', '.github/workflows/check.yml'), ('head_sha', 'b' * 40),
                             ('event', 'pull_request'), ('display_title', 'Rehearse validated container promotion'),
                             ('status', 'in_progress'), ('conclusion', 'failure'), ('conclusion', 'cancelled'),
                             ('run_attempt', 0), ('run_attempt', True), ('id', 0)]:
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                validation.check_run(dict(self.run, **{field: value}), self.expected, 7, 19)

    def test_expiry_current_attempt_immutable_digest_and_artifact_provenance(self):
        now = datetime(2026, 10, 22, tzinfo=timezone.utc)
        validation.check_artifact(self.artifact, self.run, 7, now)
        for field, value in [('name', 'validated-container-1'), ('expired', True), ('expires_at', '2026-10-21T00:00:00Z'),
                             ('digest', ''), ('size_in_bytes', validation.MAX_ARCHIVE + 1)]:
            with self.subTest(field=field), self.assertRaises(ValueError):
                validation.check_artifact(dict(self.artifact, **{field: value}), self.run, 7, now)
        for field, value in [('id', 124), ('head_sha', 'b' * 40), ('repository_id', 8), ('head_repository_id', 8)]:
            changed = copy.deepcopy(self.artifact)
            changed['workflow_run'][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                validation.check_artifact(changed, self.run, 7, now)

    def zip_fixture(self, extra=None):
        path = self.root / 'artifact.zip'
        with zipfile.ZipFile(path, 'w') as archive:
            archive.writestr('validation.json', json.dumps(self.evidence))
            archive.writestr('collector-multiarch.oci.tar', self.archive.read_bytes())
            if extra:
                archive.writestr(extra, 'untrusted')
        return path

    def test_artifact_zip_digest_paths_and_expansion_bounds(self):
        path = self.zip_fixture()
        destination = self.root / 'recovered'
        destination.mkdir()
        artifact = dict(self.artifact, digest=validation.sha_file(path))
        validation.unpack(path, destination, artifact)
        self.assertEqual((destination / self.archive.name).read_bytes(), self.archive.read_bytes())
        with self.assertRaises(ValueError):
            validation.unpack(path, destination, self.artifact)
        for member in ('../escape', '/tmp/escape', 'unexpected.sh'):
            path = self.zip_fixture(member)
            with self.subTest(member=member), self.assertRaises(ValueError):
                validation.unpack(path, destination, dict(artifact, digest=validation.sha_file(path)))
        path = self.zip_fixture()
        with patch.object(validation, 'MAX_JSON', 10), self.assertRaises(ValueError):
            validation.unpack(path, destination, dict(artifact, digest=validation.sha_file(path)))

    def test_oci_merge_retains_exact_runtime_bytes_and_smoke_identity(self):
        expected = {}
        for architecture in ('amd64', 'arm64'):
            path = self.root / ('collector-' + architecture + '.oci.tar')
            index, runtime, config = fixture(path, architecture)
            (self.root / (architecture + '.json')).write_text(json.dumps(dict(digest=index)))
            expected['linux/' + architecture] = runtime
            with patch.object(release, 'command', return_value=config):
                validation.smoke_identity(path, architecture)
            with patch.object(release, 'command', return_value='sha256:' + '0' * 64), \
                 patch.object(validation, 'daemon_config_digest', return_value='sha256:' + '0' * 64), self.assertRaises(ValueError):
                validation.smoke_identity(path, architecture)
        merged = self.root / 'merged.tar'
        merged_digest = validation.assemble(self.root, merged)
        with tarfile.open(merged) as archive:
            self.assertEqual(release.platforms(validation.blob_json(archive, merged_digest)), expected)
        self.assertEqual(validation.assemble(self.root, self.root / 'merged-again.tar'), merged_digest)
        self.assertEqual(validation.sha_file(merged), validation.sha_file(self.root / 'merged-again.tar'))

    def test_containerd_manifest_id_still_requires_exact_saved_config_bytes(self):
        path = self.root / 'native.tar'
        index, runtime, config = fixture(path, 'amd64')
        with patch.object(release, 'command', return_value=runtime), \
             patch.object(validation, 'daemon_config_digest', return_value=config):
            validation.smoke_identity(path, 'amd64')
        with tarfile.open(path) as archive:
            config_bytes = archive.extractfile('blobs/sha256/' + config[7:]).read()
        for name in (config[7:] + '.json', 'blobs/sha256/' + config[7:]):
            saved = self.root / 'daemon.tar'
            for payload in (config_bytes, config_bytes + b'changed'):
                with tarfile.open(saved, 'w') as archive:
                    for member_name, data in [('manifest.json', json.dumps([dict(Config=name)]).encode()), (name, payload)]:
                        member = tarfile.TarInfo(member_name)
                        member.size = len(data)
                        archive.addfile(member, io.BytesIO(data))
                self.assertEqual(validation.saved_config_digest(saved), digest(payload))
                if payload != config_bytes:
                    self.assertNotEqual(validation.saved_config_digest(saved), config)

    @unittest.skipUnless(shutil.which('skopeo') and os.geteuid() != 0, 'requires rootless skopeo')
    def test_rootless_oci_transport_preserves_merged_digest_and_both_platforms(self):
        expected = {}
        for architecture in ('amd64', 'arm64'):
            index, runtime, _ = fixture(self.root / ('collector-' + architecture + '.oci.tar'), architecture)
            (self.root / (architecture + '.json')).write_text(json.dumps(dict(digest=index)))
            expected['linux/' + architecture] = runtime
        archive = self.root / 'rootless.tar'
        merged = validation.assemble(self.root, archive)
        source = 'oci-archive:' + str(archive)
        raw = subprocess.check_output(['skopeo', 'inspect', '--raw', '--no-creds', source], timeout=30)
        self.assertEqual(digest(raw), merged)
        self.assertEqual(release.platforms(json.loads(raw)), expected)
        destination = 'oci:' + str(self.root / 'copied') + ':release'
        subprocess.run(['skopeo', 'copy', '--all', '--preserve-digests', source, destination], check=True, timeout=30,
                       stdout=subprocess.DEVNULL)
        copied = subprocess.check_output(['skopeo', 'inspect', '--raw', destination], timeout=30)
        self.assertEqual(digest(copied), merged)

    def test_oci_merge_rejects_wrong_native_platform_tamper_and_paths(self):
        for architecture in ('amd64', 'arm64'):
            key, _, _ = fixture(self.root / ('collector-' + architecture + '.oci.tar'), architecture)
            (self.root / (architecture + '.json')).write_text(json.dumps(dict(digest=key)))
        for options in [dict(bad_blob=True), dict(extra='../escape')]:
            fixture(self.root / 'collector-amd64.oci.tar', 'amd64', **options)
            with self.subTest(options=options), self.assertRaises(ValueError):
                validation.assemble(self.root, self.root / 'bad.tar')
        key, _, _ = fixture(self.root / 'collector-amd64.oci.tar', 'arm64')
        (self.root / 'amd64.json').write_text(json.dumps(dict(digest=key)))
        with self.assertRaises(ValueError):
            validation.assemble(self.root, self.root / 'wrong-platform.tar')

    def test_sealed_platforms_must_agree_with_reverified_oci(self):
        validation.compare(self.evidence, self.image)
        for field, value in [('platforms', dict(self.image['platforms'], **{'linux/amd64': 'sha256:' + '0' * 64})),
                             ('digest', 'sha256:' + '0' * 64), ('revision', 'b' * 40)]:
            with self.subTest(field=field), self.assertRaises(ValueError):
                validation.compare(self.evidence, dict(self.image, **{field: value}))

    def test_failed_recovery_leaves_no_candidate_and_retry_rechecks_remote_run(self):
        archive = self.zip_fixture()
        artifact = dict(self.artifact, digest=validation.sha_file(archive))
        destination = self.root / 'candidate'
        def api(path):
            if path == 'repos/Owner/collector':
                return dict(id=7)
            if '/workflows/' in path:
                return dict(id=19)
            if '/artifacts?' in path:
                return dict(artifacts=[artifact])
            return self.run
        def download(arguments, **kwargs):
            kwargs['stdout'].write(archive.read_bytes())
        with patch.object(validation, 'api', side_effect=api), patch.object(validation.subprocess, 'run', side_effect=download):
            artifact['digest'] = 'sha256:' + '0' * 64
            with self.assertRaises(ValueError):
                validation.restore(self.expected, destination, '123')
            self.assertFalse(destination.exists())
            artifact['digest'] = validation.sha_file(archive)
            recovered = validation.restore(self.expected, destination, '123')
            self.assertEqual(recovered, self.evidence)
            self.assertEqual(json.loads((destination / 'provenance.json').read_text())['artifact_id'], 80)
            with self.assertRaises(ValueError):
                validation.restore(self.expected, destination, '123')

    def test_missing_artifacts_never_trigger_build_or_registry_write(self):
        def api(path):
            if path == 'repos/Owner/collector':
                return dict(id=7)
            if '/workflows/' in path:
                return dict(id=19)
            if '/artifacts?' in path:
                return dict(artifacts=[])
            return self.run
        with patch.object(validation, 'api', side_effect=api), patch.object(validation.subprocess, 'run') as command:
            with self.assertRaisesRegex(ValueError, 'fresh validation'):
                validation.restore(self.expected, self.root / 'missing', '123')
            command.assert_not_called()


if __name__ == '__main__':
    unittest.main()
