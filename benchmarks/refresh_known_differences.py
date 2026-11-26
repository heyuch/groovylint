#!/usr/bin/env python3
"""Rebuilds fixtures/codenarc/known-differences.tsv from the current state.

The fixture test refuses to run while the ledger disagrees with the code: a listed
case that now passes, or a listed case that no longer exists, fails the build. This
script produces the ledger the test asks for, which is what you want after fixing
rules:

    python3 benchmarks/refresh_known_differences.py

It writes one row per case, or per rule (``Rule/*``) when every failing case of that
rule shares a cause, and fails if the corpus is not green afterwards.
"""

from __future__ import annotations

import collections
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
LEDGER = ROOT / 'fixtures/codenarc/known-differences.tsv'
FAILURE = re.compile(r'^([A-Za-z][A-Za-z0-9]*)/(\d+)\.groovy: (.*)$')
REASONS = {
    'missed': 'missed: we do not report this case yet',
    'false-positive': 'false-positive: we report a case CodeNarc allows',
    'other-line': 'other-line: the rule fires but on a different line',
    'ours-parser': 'ours-parser: our parser rejects this syntax',
    'better': 'better: our answer is right and CodeNarc does the opposite',
}
HEADER = """# Cases in fixtures/codenarc that do not match CodeNarc yet.
#
# One row per case, or per rule ("Rule/*") when every failing case of that rule has
# the same cause. A row that starts matching must be deleted: the fixture test fails
# when a listed case passes, and when a listed case is no longer in the corpus.
# docs/RULE-FIXTURES.md explains the buckets.
#
# `better:` rows are deliberate: groovylint's answer is the correct one there, so the
# difference is kept rather than fixed. Regenerate with
# benchmarks/refresh_known_differences.py.
#
"""


def failing_cases() -> dict[str, str]:
    LEDGER.write_text('')
    result = subprocess.run(
        ['cargo', 'test', '-p', 'groovylint-rules', '--test', 'codenarc_fixtures',
         '--release', '--', '--nocapture'],
        cwd=ROOT, capture_output=True, text=True, check=False)
    output = result.stdout + result.stderr
    if 'first failures' not in output:
        print(output[-4000:], file=sys.stderr)
        raise SystemExit('the fixture test did not report failures; nothing to do')
    cases = {}
    for line in output.split('first failures:\n', 1)[1].split('\n'):
        match = FAILURE.match(line.strip())
        if not match:
            continue
        rule, number, rest = match.groups()
        if 'parser rejected' in rest:
            bucket = 'ours-parser'
        elif 'got lines {}' in rest or 'exactly one violation, got lines {}' in rest:
            bucket = 'missed'
        elif rest.startswith('expected no violations'):
            bucket = 'false-positive'
        else:
            bucket = 'other-line'
        cases[f'{rule}/{number}.groovy'] = bucket
    return cases


def previous_better() -> set[str]:
    """Keeps the `better:` rows a human already judged, so a refresh cannot undo them."""
    if not LEDGER.is_file():
        return set()
    keys = set()
    for line in LEDGER.read_text().split('\n'):
        if not line or line.startswith('#'):
            continue
        key, _, reason = line.partition('\t')
        if reason.startswith('better'):
            keys.add(key)
    return keys


def main() -> int:
    better = previous_better()
    cases = failing_cases()
    for key, bucket in cases.items():
        if key in better:
            cases[key] = 'better'

    per_rule = collections.defaultdict(collections.Counter)
    for key, bucket in cases.items():
        per_rule[key.split('/')[0]][bucket] += 1

    entries = []
    for rule, buckets in sorted(per_rule.items()):
        if len(buckets) == 1:
            entries.append(f'{rule}/*\t{REASONS[next(iter(buckets))]}')
            continue
        for key in sorted(k for k, bucket in cases.items() if k.split('/')[0] == rule):
            entries.append(f'{key}\t{REASONS[cases[key]]}')
    LEDGER.write_text(HEADER + '\n'.join(entries) + '\n')
    print(f'{len(cases)} failing cases, {len(entries)} ledger rows')
    print(dict(collections.Counter(cases.values())))

    check = subprocess.run(
        ['cargo', 'test', '-p', 'groovylint-rules', '--test', 'codenarc_fixtures',
         '--release', '--', '--nocapture'],
        cwd=ROOT, capture_output=True, text=True, check=False)
    combined = check.stdout + check.stderr
    print(next((line for line in combined.split('\n') if line.startswith('codenarc fixtures:')), ''))
    return 0 if check.returncode == 0 else 1


if __name__ == '__main__':
    raise SystemExit(main())
