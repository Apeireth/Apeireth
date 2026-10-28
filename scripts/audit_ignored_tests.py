#!/usr/bin/env python3
"""Audit workspace ignore declarations against an observed native test set.

This inventory supports the repository's single-line ignore/cfg_attr forms. It
also compares an actual libtest --ignored --list output, so an unmatched native
entry fails instead of being silently omitted. Historical function names and
paths are retained; conditional declarations are reported separately. No tests,
credentials or external services are executed by this script.
"""
from __future__ import annotations

import argparse
import collections
import datetime
import hashlib
import json
import re
import subprocess
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
IGNORE = re.compile(r'^\s*#\[ignore(?:\s*=.*)?\]\s*$')
CONDITIONAL = re.compile(r'^\s*#\[cfg_attr\(miri,\s*ignore\s*=.*\)\]\s*$')
FUNCTION = re.compile(r'\bfn\s+(\w+)\s*\(')


def git(*args: str) -> str:
    return subprocess.check_output(['git', *args], cwd=ROOT, text=True)


def content(path: str, ref: str | None) -> str:
    return git('show', f'{ref}:{path}') if ref else (ROOT / path).read_text()


def scan(ref: str | None) -> list[dict]:
    members = tomllib.loads(content('Cargo.toml', ref))['workspace']['members']
    if any('*' in member for member in members):
        raise ValueError('expand wildcard workspace members before auditing')
    paths = (git('ls-tree', '-r', '--name-only', ref, '--', *members) if ref else
             git('ls-files', '--cached', '--others', '--exclude-standard', '--', *members))
    rows = []
    for path in sorted(set(paths.splitlines())):
        if not path.endswith('.rs'):
            continue
        lines = content(path, ref).splitlines()
        for index, line in enumerate(lines):
            conditional = bool(CONDITIONAL.fullmatch(line))
            if not IGNORE.fullmatch(line) and not conditional:
                continue
            function = next((FUNCTION.search(value) for value in lines[index + 1:index + 16]
                             if FUNCTION.search(value)), None)
            if not function:
                raise ValueError(f'ignore without a following function: {path}:{index + 1}')
            attrs = '\n'.join(lines[max(0, index - 4):index])
            rows.append({
                'path': path, 'line': index + 1, 'name': function.group(1),
                'attribute': line.strip(), 'conditional_miri': conditional,
                'windows_only': '#[cfg(windows)]' in attrs,
                'context': '\n'.join(lines[max(0, index - 6):index + 18]),
            })
    return rows


def identity(row: dict) -> tuple[str, str]:
    return row['path'], row['name']


def native(rows: list[dict]) -> list[dict]:
    return [row for row in rows if not row['conditional_miri'] and not row['windows_only']]


def history(row: dict, ref: str) -> dict:
    trace = git('log', ref, '--format=%H%x09%aI', '--no-patch',
                '-L', f"{row['line']},{row['line']}:{row['path']}")
    commits = [line.split('\t', 1) for line in trace.splitlines() if '\t' in line]
    if not commits:
        raise ValueError(f'no introducing attribute history: {identity(row)}')
    return {'sha': commits[-1][0], 'date': commits[-1][1], 'line_history': commits,
            'method': 'oldest change from git log -L of the ignore attribute'}


def observed(log: Path, rows: list[dict], listing: bool = False) -> list[dict]:
    by_name = collections.defaultdict(list)
    for row in rows:
        by_name[row['name']].append(row)
    names = []
    for line in log.read_text().splitlines():
        pattern = r'^(.+): test$' if listing else r'^test (.+?) \.\.\. ignored(?:, .*)?$'
        match = re.match(pattern, line)
        if not match:
            continue
        full_name = match.group(1)
        candidates = by_name[full_name.split('::')[-1]]
        if len(candidates) != 1:
            raise ValueError(f'cannot uniquely map observed ignored test {full_name!r}')
        names.append(dict(candidates[0], full_name=full_name))
    ids = [identity(row) for row in names]
    if len(ids) != len(set(ids)):
        raise ValueError('duplicate observed ignored identity; select one native run log')
    return names


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline-ref', required=True)
    parser.add_argument('--before-ref', required=True)
    parser.add_argument('--baseline-log', type=Path, required=True)
    parser.add_argument('--before-list', type=Path, required=True)
    parser.add_argument('--current-list', type=Path)
    parser.add_argument('--history', action='store_true')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    baseline = scan(args.baseline_ref)
    before = scan(args.before_ref)
    current = scan(None)
    observed_baseline = observed(args.baseline_log, baseline)
    before_native = observed(args.before_list, before, listing=True)
    for actual, expected, label in [(observed_baseline, native(baseline), 'baseline'),
                                    (before_native, native(before), 'before')]:
        if {identity(row) for row in actual} != {identity(row) for row in expected}:
            raise ValueError(f'{label} native source/observed inventory differs')
    current_native = observed(args.current_list, current, listing=True) if args.current_list else native(current)
    if {identity(row) for row in current_native} != {identity(row) for row in native(current)}:
        raise ValueError('current native source/observed inventory differs')
    current_ids_for_check = {identity(row) for row in current}
    for row in before:
        if identity(row) in current_ids_for_check:
            continue
        text = content(row['path'], None)
        matches = [match for match in re.finditer(
            r'(?m)^\s*(?:async\s+)?fn\s+' + re.escape(row['name']) + r'\s*\(', text)]
        if len(matches) != 1:
            raise ValueError(f'removed/renamed test instead of restoration: {identity(row)}')
        preceding = '\n'.join(text[:matches[0].start()].splitlines()[-5:])
        if not re.search(r'#\[(?:test\]|tokio::test\b)', preceding):
            raise ValueError(f'restored function lost its test attribute: {identity(row)}')
    before_ids = {identity(row) for row in before}
    current_ids = {identity(row) for row in current}
    baseline_ids = {identity(row) for row in baseline}
    if args.history:
        for row in before:
            row['introduced'] = history(row, args.before_ref)
    full_names = {identity(row): row['full_name'] for row in before_native}
    for row in before:
        row['full_name'] = full_names.get(identity(row), row['name'])
        row['restored'] = identity(row) not in current_ids
    result = {
        'date_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'baseline_log': str(args.baseline_log),
        'baseline_log_sha256': hashlib.sha256(args.baseline_log.read_bytes()).hexdigest(),
        'before_list_sha256': hashlib.sha256(args.before_list.read_bytes()).hexdigest(),
        'current_list_sha256': hashlib.sha256(args.current_list.read_bytes()).hexdigest() if args.current_list else None,
        'baseline_ref': git('rev-parse', args.baseline_ref).strip(),
        'before_ref': git('rev-parse', args.before_ref).strip(),
        'before_native_count': len(before_native), 'current_native_count': len(current_native),
        'before_unconditional_count': sum(not row['conditional_miri'] for row in before),
        'current_unconditional_count': sum(not row['conditional_miri'] for row in current),
        'conditional_miri_count': sum(row['conditional_miri'] for row in current),
        'baseline_to_before_added': sorted(before_ids - baseline_ids),
        'baseline_to_before_removed': sorted(baseline_ids - before_ids),
        'restored': sorted(before_ids - current_ids), 'new_ignores': sorted(current_ids - before_ids),
        'before': before, 'current': current,
        'rename_policy': 'source function names/paths are retained; removals are verified restorations, not renames',
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(f"native ignored: {len(before_native)} -> {len(current_native)}")
    print(f"unconditional declarations: {result['before_unconditional_count']} -> {result['current_unconditional_count']}")
    print(f"conditional Miri declarations: {result['conditional_miri_count']}")
    print(f"baseline -> before set added/removed: {result['baseline_to_before_added']} / {result['baseline_to_before_removed']}")
    print(f"restored: {result['restored']}")
    print(f"new ignores: {result['new_ignores']}")
    print(args.output)


if __name__ == '__main__':
    main()
