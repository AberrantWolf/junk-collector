#!/usr/bin/env python3
"""Select and run checks for a complete diff. Copy classifications require human review."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import tomllib

ROOT = Path(__file__).resolve().parents[1]
POLICY = 'change-checks-v1'
POLICY_DOCS = {'docs/architecture.md', 'docs/release-requirements.md', 'docs/release-toolchain.md', 'docs/container-release-pipeline.md', 'docs/testing-and-packaging-plan.md', 'docs/testing.md'}
KINDS = ('auto', 'docs', 'ui-copy', 'cli-copy', 'style', 'ui', 'rust', 'infra')
BROWSER = ('smoke', 'integration', 'browse', 'audio-checks', 'accuraterip-snapshots', 'actions',
           'scans', 'work', 'fx04', 'fx05', 'fx06', 'fx04-review', 'local-playback', 'lists',
           'accounts', 'pair', 'navigation', 'certificate', 'numeric-ip', 'game-catalog',
           'purchases', 'intake-review', 'game-audio', 'documents', 'backup-reminder', 'remote-media')
FIXTURE = {'audio-checks', 'accuraterip-snapshots', 'actions', 'lists'}
SENSITIVE = ('crates/collector-core/', 'crates/collector-api-types/', 'crates/collector-store/',
             'crates/collector-client/', '.github/', 'scripts/', 'deploy/')


def git(*args, root=ROOT, strip=True):
    data = subprocess.check_output(['git', '-C', str(root), *args], text=True)
    return data.strip() if strip else data


def changed(base=None, root=ROOT):
    revision = git('rev-parse', '--verify', (base or 'HEAD') + '^{commit}', root=root)
    # Compare base to the working tree, including staged changes and deletions.
    paths = set(filter(None, git('diff', '--name-only', '-z', revision, '--', root=root, strip=False).split('\0')))
    paths.update(filter(None, git('ls-files', '--others', '--exclude-standard', '-z', root=root, strip=False).split('\0')))
    return revision, sorted(paths)


def automatic(paths):
    if not paths:
        return 'docs'
    if any(p in POLICY_DOCS for p in paths):
        return 'infra'
    if all((p == 'README.md' or p.startswith('docs/') and p.endswith('.md')) and not p.startswith(('docs/decisions/',)) for p in paths):
        return 'docs'
    if all(p.startswith('web/src/') and p.endswith('.css') or p == 'web/dist/app.css' for p in paths):
        return 'style'
    if any(p.startswith(SENSITIVE) or p in ('Cargo.toml', 'Cargo.lock', 'Dockerfile', 'AGENTS.md',
                                           'web/package.json', 'web/package-lock.json') for p in paths):
        return 'infra'
    if all(p.startswith(('web/src/', 'web/dist/', 'web/tests/')) or p.startswith('docs/') and p.endswith('.md') for p in paths):
        return 'ui'
    if all(p.startswith('crates/') or p.startswith('docs/') and p.endswith('.md') for p in paths):
        if any(any(token in p for token in ('auth', 'permission', 'backup', 'restore', 'recovery', 'Cargo.toml')) for p in paths):
            return 'infra'
        return 'rust'
    return 'infra'


def classify(paths, declared):
    inferred = automatic(paths)
    if declared == 'auto':
        return inferred
    allowed = {
        'docs': lambda p: p.endswith('.md'),
        'ui-copy': lambda p: p.startswith(('web/src/', 'web/dist/', 'web/tests/')) and p.endswith(('.tsx', '.ts', '.js', '.mjs', '.css', '.html')) or p.startswith('docs/') and p.endswith('.md'),
        'cli-copy': lambda p: p.startswith('crates/collector-server/src/') and p.endswith('.rs') or p.startswith('docs/') and p.endswith('.md'),
        'style': lambda p: p.startswith('web/') and p.endswith('.css') or p.startswith('docs/') and p.endswith('.md'),
        'ui': lambda p: p.startswith(('web/src/', 'web/dist/', 'web/tests/')) or p.startswith('docs/') and p.endswith('.md'),
        'rust': lambda p: p.startswith('crates/') and not p.endswith('Cargo.toml') or p.startswith('docs/') and p.endswith('.md'),
        'infra': lambda p: True,
    }
    if any(not allowed[declared](p) for p in paths):
        raise ValueError(f'{declared} contradicts changed paths; use auto or a broader classification')
    if declared not in ('infra', 'docs') and any(p in POLICY_DOCS or p.startswith('docs/decisions/') or p == 'AGENTS.md' for p in paths):
        raise ValueError('Testing/release/architecture policy changes require infra coverage')
    if declared == 'rust' and inferred == 'infra':
        return 'infra'
    return declared


def select(paths, kind='auto', level='local', base_known=True, journeys=(), external_server=False):
    if any(name not in BROWSER for name in journeys):
        raise ValueError('Unknown browser journey')
    kind = classify(paths, kind)
    if not base_known:
        kind = 'infra'
    full = kind == 'infra' or level == 'full' and kind not in ('docs', 'ui-copy', 'cli-copy')
    steps = []
    def add(name, command):
        steps.append(dict(name=name, command=command))
    add('architecture', ['python3', 'scripts/check_architecture.py'])
    if kind == 'docs' and not full:
        return kind, steps
    if kind not in ('ui-copy', 'style', 'ui') or full:
        add('format', ['cargo', 'fmt', '--all', '--', '--check'])
    if kind not in ('cli-copy', 'rust') or full or journeys:
        add('web-types', ['npm', '--prefix', 'web', 'run', 'check'])
        add('web-build', ['npm', '--prefix', 'web', 'run', 'build'])
    if full:
        add('tooling', ['python3', '-B', 'scripts/check_testing.py'])
        add('container-tools', ['python3', '-B', 'scripts/check_container_tools.py'])
        add('release-policy', ['python3', '-B', 'scripts/check_container_release.py'])
        add('container-validation', ['python3', '-B', 'scripts/check_container_validation.py'])
        add('workspace-tests', ['cargo', 'test', '--workspace', '--locked'])
        add('workspace-clippy', ['cargo', 'clippy', '--workspace', '--all-targets', '--locked', '--', '-D', 'warnings'])
    elif kind == 'rust':
        packages = {p.split('/')[1] for p in paths if p.startswith('crates/')}
        # Include transitive in-repository consumers, not only the edited crate.
        manifests = {p.parent.name: tomllib.loads(p.read_text()) for p in (ROOT / 'crates').glob('*/Cargo.toml')}
        while True:
            enlarged = packages | {name for name, data in manifests.items() if packages.intersection(data.get('dependencies', {}))}
            if enlarged == packages:
                break
            packages = enlarged
        flags = [part for name in sorted(packages) for part in ('-p', name)]
        if not flags:
            raise ValueError('Rust classification needs an affected crate')
        add('affected-tests', ['cargo', 'test', '--locked', *flags])
        add('affected-clippy', ['cargo', 'clippy', '--locked', *flags, '--all-targets', '--', '-D', 'warnings'])
    if full or kind == 'ui':
        add('web-models', ['npm', '--prefix', 'web', 'run', 'test:models'])
    if kind == 'cli-copy':
        add('cli-build', ['cargo', 'build', '--locked', '-p', 'collector-server', *(['--release'] if level != 'local' else [])])
    if full:
        selected = list(BROWSER)
    elif kind == 'ui' and not journeys:
        selected = list(BROWSER)
    elif kind in ('ui-copy', 'style', 'ui'):
        selected = list(dict.fromkeys(['smoke', *journeys]))
    else:
        selected = list(journeys)
    if selected:
        if not external_server and any(name not in FIXTURE for name in selected):
            add('browser-server', ['cargo', 'build', '--locked', '-p', 'collector-server', '-p', 'collector-media'])
        add('browser', ['node', 'web/tests/run-suite.mjs', *selected])
    return kind, steps


def container_source_steps(kind, steps, journeys=()):
    # The image build supplies the release executable, including CLI-only edits.
    omitted = {'cli-build'}
    if kind in ('docs', 'ui-copy', 'style', 'cli-copy') and not journeys:
        omitted |= {'browser-server', 'browser'}
    return [step for step in steps if step['name'] not in omitted]


def covers_steps(actual_steps, required_steps):
    actual = {step['name']: step['command'] for step in actual_steps}
    for required in required_steps:
        name, command = required['name'], required['command']
        if actual.get(name) == command:
            continue
        if name in ('affected-tests', 'affected-clippy') and ('workspace-tests' if name == 'affected-tests' else 'workspace-clippy') in actual:
            continue
        browser = actual.get('browser', [])
        if name == 'browser' and browser[:2] == command[:2] == ['node', 'web/tests/run-suite.mjs'] and set(command[2:]) <= set(browser[2:]):
            continue
        return False
    return True


def validate_report(report, revision=None, require_full=False):
    if report.get('policy') != POLICY or report.get('status') != 'passed' or report.get('dirty') is not False or report.get('source_unchanged') is not True:
        raise ValueError('Check evidence must be successful, current-policy and committed')
    if revision and report.get('revision') != revision:
        raise ValueError('Check evidence source revision mismatch')
    if require_full and report.get('coverage') != 'full':
        baseline = report.get('baseline_qualification')
        if report.get('classification') not in ('docs', 'ui-copy', 'cli-copy') or not baseline:
            raise ValueError('RC/stable requires full functional qualification or an applicable full baseline for text-only edits')
        validate_report(baseline, report.get('base'), True)
    steps = report.get('steps', [])
    _, expected = select(report.get('paths', []), report.get('classification', 'auto'), report.get('level', 'local'), journeys=report.get('journeys', []), external_server=report.get('external_server', False))
    if report.get('source_only'):
        expected = container_source_steps(report.get('classification'), expected, report.get('journeys', []))
    actual_names = {s.get('name') for s in steps}
    if report.get('coverage') not in ('changed', 'full'):
        raise ValueError('Unknown qualification coverage')
    if report.get('coverage') == 'full' and not any(s['name'] == 'workspace-tests' for s in expected):
        raise ValueError('Full coverage is inconsistent with selected scope')
    if actual_names != {s['name'] for s in expected} or len(steps) != len(expected):
        raise ValueError('Check evidence omits required checks for its declared scope')
    for required in expected:
        actual = next(s for s in steps if s.get('name') == required['name'])
        if actual.get('command') != required['command']:
            raise ValueError('Check evidence command disagrees with selected policy')
    if not steps or any(step.get('status') != 'passed' for step in steps):
        raise ValueError('Check evidence has missing, failed or skipped prerequisites')
    if report.get('coverage') == 'full' and not {'workspace-tests', 'workspace-clippy'}.issubset({s['name'] for s in steps}):
        raise ValueError('Full coverage is missing workspace qualification')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', help='Reviewed validated baseline; local defaults to HEAD')
    parser.add_argument('--level', choices=('local', 'beta', 'full'), default='local')
    parser.add_argument('--classification', choices=KINDS, default='auto')
    parser.add_argument('--journey', action='append', choices=BROWSER, default=[])
    parser.add_argument('--dry-run', action='store_true')
    parser.add_argument('--report', type=Path)
    parser.add_argument('--baseline-report', type=Path, help='Successful full check receipt for --base, obtained through trusted CI for releases')
    parser.add_argument('--source-only', action='store_true', help='CI source checks; browser acceptance is performed on the release image')
    args = parser.parse_args()
    try:
        base, paths = changed(args.base)
        kind, steps = select(paths, args.classification, args.level, args.base is not None or args.level == 'local', args.journey, bool(os.environ.get('COLLECTOR_TEST_ORIGIN') or os.environ.get('COLLECTOR_SERVER_BINARY')))
        if args.source_only:
            steps = container_source_steps(kind, steps, args.journey)
        report = dict(policy=POLICY, revision=git('rev-parse', 'HEAD'), base=base,
                      dirty=bool(git('status', '--porcelain')), classification=kind,
                      level=args.level, coverage='full' if any(s['name'] == 'workspace-tests' for s in steps) else 'changed',
                      paths=paths, steps=steps, source_only=args.source_only, journeys=args.journey, external_server=bool(os.environ.get('COLLECTOR_TEST_ORIGIN') or os.environ.get('COLLECTOR_SERVER_BINARY')), status='planned')
        if args.baseline_report:
            baseline_report = json.loads(args.baseline_report.read_text())
            validate_report(baseline_report, base, True)
            report['baseline_qualification'] = baseline_report
        print(f"{args.level}: {kind}; baseline {base}; {len(paths)} changed paths", flush=True)
        for step in steps:
            print(step['name'] + ': ' + ' '.join(step['command']), flush=True)
        print('Unit tests omitted for reviewed text-only edits.' if kind in ('docs', 'ui-copy', 'cli-copy') else 'Coverage selected from the complete diff and affected consumers.', flush=True)
        if not args.dry_run:
            for step in steps:
                start = time.monotonic()
                result = subprocess.run(step['command'], cwd=ROOT, check=False)
                step.update(status='passed' if result.returncode == 0 else 'failed', seconds=round(time.monotonic() - start, 2))
                if result.returncode:
                    report['status'] = 'failed'
                    break
            else:
                report['status'] = 'passed'
        if report['status'] == 'passed':
            report['source_unchanged'] = not report['dirty'] and git('rev-parse', 'HEAD') == report['revision'] and not git('status', '--porcelain')
            if not report['dirty'] and not report['source_unchanged']:
                report['status'] = 'failed'
                print('Source changed during validation; do not attribute these results to the original revision.', file=sys.stderr)
        if args.report:
            args.report.write_text(json.dumps(report, indent=2) + '\n')
        return 1 if report['status'] == 'failed' else 0
    except (ValueError, subprocess.CalledProcessError) as error:
        print(str(error), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
