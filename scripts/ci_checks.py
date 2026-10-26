#!/usr/bin/env python3
"""Reuse successful exact-source check receipts from the repository's Checks workflow."""
import argparse
import io
import json
from pathlib import Path
import re
import resource
import tempfile
import time
import subprocess
import sys
import zipfile
from check_changes import validate_report, covers_steps

MAX_BYTES = 4 * 1024 * 1024


def api(path):
    return json.loads(subprocess.check_output(['gh', 'api', path], timeout=60))


def check_run(run, revision, repository_id, workflow_id):
    if (run.get('head_sha') != revision or run.get('repository', {}).get('id') != repository_id
        or run.get('head_repository', {}).get('id') != repository_id
        or run.get('workflow_id') != workflow_id or run.get('path') != '.github/workflows/check.yml'
        or run.get('status') != 'completed' or run.get('conclusion') != 'success'
        or run.get('event') not in ('push', 'pull_request', 'workflow_dispatch')
        or type(run.get('run_attempt')) is not int or run['run_attempt'] < 1):
        raise ValueError('Source checks are not trusted successful exact-source evidence')


def recover(repository, revision, destination, *, base=None, require_full=False, selected=None, required=None):
    if not re.fullmatch(r'[\w.-]+/[\w.-]+', repository) or not re.fullmatch(r'[0-9a-f]{40}', revision):
        raise ValueError('Invalid repository or revision')
    prefix = f'repos/{repository}'
    repository_id = api(prefix)['id']
    workflow_id = api(prefix + '/actions/workflows/check.yml')['id']
    runs = [api(prefix + '/actions/runs/' + str(int(selected)))] if selected else api(
        prefix + '/actions/workflows/check.yml/runs?status=success&head_sha=' + revision + '&per_page=100')['workflow_runs']
    for candidate in runs:
        try:
            run = api(prefix + '/actions/runs/' + str(candidate['id']))
            check_run(run, revision, repository_id, workflow_id)
            artifacts = api(prefix + '/actions/runs/' + str(run['id']) + '/artifacts?per_page=100')['artifacts']
            artifacts = [a for a in artifacts if a['name'] == 'source-checks-' + str(run['run_attempt'])]
            if len(artifacts) != 1:
                continue
            artifact = artifacts[0]
            if artifact.get('expired') is not False or artifact['size_in_bytes'] > MAX_BYTES:
                continue
            def bound_download():
                resource.setrlimit(resource.RLIMIT_FSIZE, (MAX_BYTES, MAX_BYTES))
            with tempfile.TemporaryFile() as output:
                subprocess.run(['gh', 'api', prefix + '/actions/artifacts/' + str(artifact['id']) + '/zip'],
                               stdout=output, check=True, timeout=60, preexec_fn=bound_download)
                output.seek(0)
                data = output.read(MAX_BYTES + 1)
            if len(data) > MAX_BYTES:
                raise ValueError('Oversized source-check artifact')
            import hashlib
            if artifact.get('digest') != 'sha256:' + hashlib.sha256(data).hexdigest():
                raise ValueError('Source-check artifact digest mismatch')
            with zipfile.ZipFile(io.BytesIO(data)) as archive:
                entries = archive.infolist()
                if len(entries) != 1 or entries[0].filename != 'check-report.json' or entries[0].file_size > MAX_BYTES:
                    raise ValueError('Unexpected source-check artifact contents')
                report = json.loads(archive.read(entries[0]))
            validate_report(report, revision, require_full)
            if report['coverage'] != 'full' and (not base or report['base'] != base):
                continue
            if required:
                if not covers_steps(report['steps'], required['steps']):
                    continue
            current = api(prefix + '/actions/runs/' + str(run['id']))
            check_run(current, revision, repository_id, workflow_id)
            if current['run_attempt'] != run['run_attempt']:
                raise ValueError('Source checks were rerun during recovery')
            report['reused_from'] = dict(run_id=run['id'], attempt=run['run_attempt'], artifact_id=artifact['id'], artifact_digest=artifact['digest'])
            destination.write_text(json.dumps(report, indent=2) + '\n')
            return True
        except (ValueError, KeyError, subprocess.CalledProcessError, zipfile.BadZipFile) as error:
            print('Source evidence not reusable: ' + str(error), file=sys.stderr)
    return False


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('repository');parser.add_argument('revision');parser.add_argument('destination', type=Path)
    parser.add_argument('--wait-seconds', type=int, default=0);parser.add_argument('--plan', type=Path);parser.add_argument('--base');parser.add_argument('--full', action='store_true');parser.add_argument('--run-id')
    args = parser.parse_args()
    until = time.monotonic() + min(max(args.wait_seconds, 0), 900)
    while True:
        if recover(args.repository, args.revision, args.destination, base=args.base, require_full=args.full,
                   selected=args.run_id, required=json.loads(args.plan.read_text()) if args.plan else None):
            return 0
        if time.monotonic() >= until:
            return 1
        runs = api(f'repos/{args.repository}/actions/workflows/check.yml/runs?head_sha={args.revision}&per_page=100')['workflow_runs']
        if not any(run.get('status') != 'completed' for run in runs):
            return 1
        print('Waiting for already-running same-source checks rather than duplicating compilation.', flush=True)
        time.sleep(min(15, max(0, until - time.monotonic())))


if __name__ == '__main__':
    sys.exit(main())
