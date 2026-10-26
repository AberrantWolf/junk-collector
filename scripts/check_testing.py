#!/usr/bin/env python3
"""Meaningful test-selection and evidence regressions; no browser or provider required."""
import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import check_changes as checks
import ci_checks
import hashlib
import io
import zipfile


class SelectionTests(unittest.TestCase):
    def names(self, paths, kind='auto', level='local', known=True):
        return {s['name'] for s in checks.select(paths, kind, level, known)[1]}

    def test_text_only_never_schedules_units_including_release_level(self):
        for paths, kind in [(['docs/getting-started.md'], 'docs'),
                            (['web/src/metadata-review.tsx', 'web/dist/curation.js'], 'ui-copy'),
                            (['crates/collector-server/src/main.rs'], 'cli-copy')]:
            for level in ('local', 'beta', 'full'):
                names = self.names(paths, kind, level)
                self.assertFalse({'workspace-tests', 'affected-tests', 'web-models'} & names)
        self.assertIn('web-build', self.names(['web/src/metadata-review.tsx'], 'ui-copy'))
        self.assertIn('browser', self.names(['web/src/metadata-review.tsx'], 'ui-copy'))

    def test_mixed_unknown_and_missing_baseline_cannot_hide_behind_copy(self):
        with self.assertRaises(ValueError):
            self.names(['web/src/app.tsx', 'Cargo.lock'], 'ui-copy')
        for paths, known in [(['unknown.file'], True), (['docs/getting-started.md'], False),
                             (['Cargo.lock'], True), (['crates/collector-store/src/lib.rs'], True)]:
            self.assertIn('workspace-tests', self.names(paths, known=known))
        self.assertIn('web-models', self.names(['web/src/app.tsx']))
        self.assertNotIn('workspace-tests', self.names(['web/src/app.css']))

    def test_affected_rust_tests_include_actual_consumers(self):
        _, steps = checks.select(['crates/collector-modules/src/lib.rs'], 'rust')
        command = next(s['command'] for s in steps if s['name'] == 'affected-tests')
        self.assertIn('collector-modules', command)
        self.assertIn('collector-app', command)
        self.assertIn('collector-server', command)
        self.assertNotIn('collector-client', command)

    def test_text_rc_reuses_full_baseline_but_cannot_inflate_coverage(self):
        base = dict(policy=checks.POLICY, status='passed', dirty=False, revision='b'*40,
                    coverage='full', classification='infra', level='full', paths=['Dockerfile'],
                    steps=[dict(step, status='passed') for step in checks.select(['Dockerfile'], 'infra', 'full')[1]])
        paths = ['web/src/metadata-review.tsx']
        copy_report = dict(policy=checks.POLICY, status='passed', dirty=False, revision='a'*40,
                           base='b'*40, coverage='changed', classification='ui-copy', level='full', paths=paths,
                           steps=[dict(step, status='passed') for step in checks.select(paths, 'ui-copy', 'full')[1]])
        with self.assertRaises(ValueError): checks.validate_report(copy_report, require_full=True)
        copy_report['baseline_qualification'] = base
        checks.validate_report(copy_report, require_full=True)
        for field, value in [('base','c'*40), ('classification','ui'), ('coverage','full')]:
            with self.subTest(field=field), self.assertRaises(ValueError):
                checks.validate_report(dict(copy_report, **{field:value}), require_full=True)
        bad = copy.deepcopy(copy_report);bad['steps'][0]['command']=['true']
        with self.assertRaises(ValueError): checks.validate_report(bad)

    def test_harness_reaps_processes_on_interruption(self):
        subprocess.run(['node', str(checks.ROOT/'web/tests/harness-check.mjs')], check=True, timeout=20)

    def test_complete_diff_includes_committed_staged_unstaged_untracked_and_deletion(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            def git(*args):
                return subprocess.check_output(['git', '-C', temporary, *args], text=True).strip()
            git('init', '-q');git('config', 'user.name', 'Fixture');git('config', 'user.email', 'fixture@example.invalid')
            (root/'first').write_text('initial');(root/'deleted').write_text('retained')
            git('add', '.');git('commit', '-qm', 'base');base=git('rev-parse', 'HEAD')
            (root/'committed').write_text('committed');git('add', '.');git('commit', '-qm', 'later')
            (root/'staged').write_text('staged');git('add', 'staged')
            (root/'first').write_text('working');(root/'untracked').write_text('new');(root/'deleted').unlink()
            revision, paths = checks.changed(base, root)
            self.assertEqual(revision, base)
            self.assertEqual(set(paths), {'first', 'deleted', 'committed', 'staged', 'untracked'})

    def test_failed_command_stops_selection_and_records_failure(self):
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary)/'report.json'
            with patch('sys.argv', ['check_changes', '--classification', 'docs', '--report', str(destination)]), \
                 patch.object(checks, 'changed', return_value=('a'*40, ['docs/example.md'])), \
                 patch.object(checks, 'git', return_value=''), \
                 patch.object(checks.subprocess, 'run', return_value=subprocess.CompletedProcess([], 7)):
                self.assertEqual(checks.main(), 1)
            self.assertEqual(json.loads(destination.read_text())['status'], 'failed')

    def test_evidence_rejects_partial_wrong_revision_and_inflated_coverage(self):
        report = dict(policy=checks.POLICY, status='passed', dirty=False, revision='a'*40,
                      coverage='full', classification='infra', level='full', paths=['Dockerfile'],
                      steps=[dict(name=s['name'], status='passed', command=s['command']) for s in checks.select(['Dockerfile'], 'infra', 'full')[1]])
        checks.validate_report(report, 'a'*40, True)
        for field, value in [('status', 'failed'), ('dirty', True), ('policy', 'old'), ('revision', 'b'*40), ('steps', [])]:
            with self.subTest(field=field), self.assertRaises(ValueError):
                checks.validate_report(dict(report, **{field:value}), 'a'*40, True)
        partial = copy.deepcopy(report);partial['steps'][0]['status']='skipped'
        with self.assertRaises(ValueError): checks.validate_report(partial)
        partial = copy.deepcopy(report);partial['steps']=[s for s in partial['steps'] if s['name'] != 'workspace-tests']
        with self.assertRaises(ValueError): checks.validate_report(partial, require_full=True)
        with self.assertRaises(ValueError): checks.validate_report(dict(report, coverage='changed'), require_full=True)


class SourceReceiptTests(unittest.TestCase):
    def test_reuse_binds_source_workflow_attempt_baseline_and_required_checks(self):
        report=dict(policy=checks.POLICY, revision='a'*40, base='b'*40, dirty=False,
                    classification='ui-copy', level='beta', coverage='changed', paths=['web/src/app.tsx'],
                    steps=[dict(step,status='passed') for step in checks.select(['web/src/app.tsx'],'ui-copy','beta')[1]], status='passed')
        run=dict(id=123,head_sha='a'*40,repository=dict(id=7),head_repository=dict(id=7),
                 workflow_id=19,path='.github/workflows/check.yml',status='completed',conclusion='success',event='push',run_attempt=1)
        def recover(*, base='b'*40, full=False, required=None, mutate=None, rerun=False):
            receipt=copy.deepcopy(report)
            if mutate: mutate(receipt)
            stream=io.BytesIO()
            with zipfile.ZipFile(stream,'w') as archive: archive.writestr('check-report.json',json.dumps(receipt))
            data=stream.getvalue()
            artifact=dict(id=80,name='source-checks-1',expired=False,size_in_bytes=len(data),digest='sha256:'+hashlib.sha256(data).hexdigest())
            fetched=0
            def api(path):
                nonlocal fetched
                if path=='repos/Owner/repo': return dict(id=7)
                if path.endswith('/workflows/check.yml'): return dict(id=19)
                if '/workflows/check.yml/runs?' in path: return dict(workflow_runs=[run])
                if '/artifacts?' in path: return dict(artifacts=[artifact])
                fetched+=1
                return dict(run,run_attempt=2) if rerun and fetched>1 else run
            with tempfile.TemporaryDirectory() as temporary, patch.object(ci_checks,'api',side_effect=api), \
                 patch.object(ci_checks.subprocess,'run',side_effect=lambda *a,**k:k['stdout'].write(data)):
                destination=Path(temporary)/'receipt.json'
                result=ci_checks.recover('Owner/repo','a'*40,destination,base=base,require_full=full,required=required)
                self.assertEqual(destination.exists(),result)
                return result
        self.assertTrue(recover())
        self.assertFalse(recover(base=None))
        self.assertFalse(recover(base='c'*40))
        self.assertFalse(recover(full=True))
        self.assertFalse(recover(rerun=True))
        self.assertFalse(recover(mutate=lambda r:r.update(revision='c'*40)))
        self.assertFalse(recover(required=dict(steps=checks.select(['web/src/app.tsx'],'ui','beta')[1])))
        for field,value in [('head_sha','c'*40),('workflow_id',20),('conclusion','cancelled'),('head_repository',dict(id=8))]:
            with self.subTest(field=field), self.assertRaises(ValueError):
                ci_checks.check_run(dict(run,**{field:value}),'a'*40,7,19)


if __name__ == '__main__':
    unittest.main()
