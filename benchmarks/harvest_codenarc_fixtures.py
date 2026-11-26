#!/usr/bin/env python3
"""Harvests CodeNarc's own rule tests into fixtures this project can assert on.

CodeNarc keeps its rule fixtures inline, as string constants passed to
``assertSingleViolation`` and friends in ``src/test/groovy/**/*RuleTest.groovy``.
The constants are passed to the rule verbatim, so the line numbers CodeNarc
asserts are real line numbers in the snippet. That makes them usable as-is:

    python3 benchmarks/harvest_codenarc_fixtures.py --codenarc-src ~/Code/CodeNarc-3.7.0

Writes:

* ``fixtures/codenarc/<Rule>/<case>.groovy``      the snippet
* ``fixtures/codenarc/<Rule>/<case>.properties``  the ``rule.x = ...`` the test applied
* ``fixtures/codenarc/expectations.tsv``          one row per case

``expectations.tsv`` columns: rule, case file, source path, expected lines,
properties. Expected lines are the 1-based line numbers that must hold a
violation; an empty field means "exactly one violation, any line", and ``-``
means "no violations at all".

CodeNarc is Apache-2.0; the harvested snippets stay under that license and the
generated ``NOTICE`` says so.
"""

from __future__ import annotations

import argparse
import collections
import json
import pathlib
import re
import shutil
import sys

CONTINUATION = '\\' + '\n'  # a backslash-newline pair, dropped by Groovy
ESCAPES = {'n': '\n', 't': '\t', 'r': '\r', 'b': '\b', 'f': '\f', '0': '\0', '\\': '\\', "'": "'", '"': '"', '$': '$'}


# --------------------------------------------------------------------------- #
# Reading Groovy string literals
# --------------------------------------------------------------------------- #

def literal_at(text: str, pos: int) -> tuple[str | None, int, bool]:
    """Returns the body and end offset of a string literal starting at ``pos``.

    ``pos`` must already be on the opening quote. Triple-quoted bodies are kept
    verbatim (Groovy preserves their newlines and indentation); single-line
    bodies are unescaped, because ``'a\\nb'`` is a two-line string.
    """
    for quote in ("'''", '"""'):
        if text.startswith(quote, pos):
            end = text.find(quote, pos + 3)
            if end < 0:
                return None, pos, False
            body = text[pos + 3:end]
            # A backslash-newline pair disappears, and the remaining escapes are
            # processed: Groovy treats a triple-quoted string as a plain string.
            body = unescape(body.replace(CONTINUATION, ''))
            body = post_process(body, text[end + 3:end + 40])
            return body, end + 3, quote == '"""'
    if pos < len(text) and text[pos] in '"\'':
        quote = text[pos]
        index = pos + 1
        out = []
        while index < len(text):
            char = text[index]
            if char == '\\':
                following = text[index + 1]
                out.append(ESCAPES.get(following, following))
                index += 2
                continue
            if char == quote:
                return ''.join(out), index + 1, quote == '"'
            if char == '\n':
                return None, pos, False
            out.append(char)
            index += 1
    return None, pos, False


def literal_spans(text: str) -> list[tuple[int, int]]:
    """Returns the spans of every string literal, comments included as code."""
    spans = []
    index = 0
    while index < len(text):
        char = text[index]
        if char in '"\'':
            body, end, _ = literal_at(text, index)
            if body is not None:
                spans.append((index, end))
                index = end
                continue
        index += 1
    return spans


def outside(spans: list[tuple[int, int]], pos: int) -> bool:
    return not any(start <= pos < end for start, end in spans)


def strip_margin(body: str, margin: str = '|') -> str:
    """Groovy's String#stripMargin: keep what follows the margin character."""
    lines = []
    for line in body.split('\n'):
        index = line.find(margin)
        lines.append(line if index < 0 else line[index + len(margin):])
    return '\n'.join(lines)


def strip_indent(body: str) -> str:
    """Groovy's String#stripIndent: drop the smallest common indentation."""
    lines = body.split('\n')
    widths = [len(line) - len(line.lstrip()) for line in lines if line.strip()]
    if not widths:
        return body
    smallest = min(widths)
    if smallest == 0:
        return body
    return '\n'.join(line[smallest:] if line.strip() else line for line in lines)


def post_process(body: str, trailing: str) -> str:
    """Applies the ``.stripIndent()`` / ``.stripMargin()`` the test wrote after the literal."""
    text = trailing.lstrip()
    while True:
        if text.startswith('.trim()'):
            body = body.strip()
            text = text[len('.trim()'):].lstrip()
            continue
        if text.startswith('.stripIndent()'):
            body = strip_indent(body)
            text = text[len('.stripIndent()'):].lstrip()
            continue
        if text.startswith('.stripMargin()'):
            body = strip_margin(body)
            text = text[len('.stripMargin()'):].lstrip()
            continue
        return body


def unescape(body: str) -> str:
    """Processes the escapes Groovy applies to every string literal."""
    out = []
    index = 0
    while index < len(body):
        char = body[index]
        if char == '\\' and index + 1 < len(body):
            following = body[index + 1]
            out.append(ESCAPES.get(following, '\\' + following))
            index += 2
            continue
        out.append(char)
        index += 1
    return ''.join(out)


def skip_space(text: str, pos: int) -> int:
    while pos < len(text) and text[pos] in ' \t\r\n':
        pos += 1
    return pos


# --------------------------------------------------------------------------- #
# Splitting a test class into methods
# --------------------------------------------------------------------------- #

METHOD = re.compile(r'(?:^|\n)[ \t]*(?:@\w+[^\n]*\n[ \t]*)*void[ \t]+(\w+)[ \t]*\(')
ASSIGNMENT = re.compile(r'(?:final\s+)?(?:String\s+|int\s+|boolean\s+|def\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?![=])')
PROPERTY = re.compile(r'\brule\.([A-Za-z][A-Za-z0-9_]*)\s*=\s*(?![=])')


def split_methods(text: str) -> list[tuple[str, str]]:
    spans = literal_spans(text)
    starts = [
        (match.start(), match.start(1))
        for match in METHOD.finditer(text)
        if outside(spans, match.start())
    ]
    if not starts:
        return [('class', text)]
    blocks = []
    for index, (start, name_start) in enumerate(starts):
        end = starts[index + 1][0] if index + 1 < len(starts) else len(text)
        name = text[name_start:METHOD.match(text, start).end(1)]
        blocks.append((name, text[start:end]))
    return blocks


def properties_in(block: str) -> dict[str, str]:
    found = {}
    for match in PROPERTY.finditer(block):
        pos = skip_space(block, match.end())
        body, _, _ = literal_at(block, pos)
        if body is not None:
            found[match.group(1)] = body
            continue
        value_match = re.match(r'[^,\n]+', block[pos:])
        if not value_match:
            continue
        value = strip_comment(value_match.group(0).strip())
        if value == 'null':
            # Groovy's null means "not set", which is how CodeNarc reads it.
            continue
        found[match.group(1)] = value
    return found


def strip_comment(value: str) -> str:
    """Removes a trailing `// ...` from a property value, keeping slashy strings."""
    if value.startswith('/'):
        # A slashy string ends at the first unescaped slash.
        index = 1
        while index < len(value):
            if value[index] == '\\':
                index += 2
                continue
            if value[index] == '/':
                return value[1:index]
            index += 1
    index = value.find('//')
    return value if index < 0 else value[:index].strip()


def source_location(block: str) -> tuple[str | None, str | None]:
    """Returns the ``sourceCodePath`` / ``sourceCodeName`` the test set."""
    return (
        _assigned_literal(block, 'sourceCodePath'),
        _assigned_literal(block, 'sourceCodeName'),
    )


def _assigned_literal(block: str, name: str) -> str | None:
    match = re.search(rf'\b{name}\s*=\s*(?![=])', block)
    if not match:
        return None
    body, _, _ = literal_at(block, skip_space(block, match.end()))
    return body


# --------------------------------------------------------------------------- #
# Assertions
# --------------------------------------------------------------------------- #

ASSERTIONS = (
    # name, pattern, expected count; Groovy allows the argument list without
    # parentheses, so the pattern stops at the method name.
    ('single', re.compile(r'assertSingleViolation\b'), 1),
    ('two', re.compile(r'assertTwoViolations\b'), 2),
    ('many', re.compile(r'assertViolations\b'), None),
    ('none', re.compile(r'assertNoViolations\b'), 0),
    ('inline', re.compile(r'assertInlineViolations\b'), None),
)

LINE_NUMBER = re.compile(r'line(?:Number)?\s*:\s*(\d+)')
POSITIONAL = re.compile(r'(?:^|,)\s*(\d+)\s*(?=[,)])')


def case_from_assertion(block: str, match: re.Match[str], rule_name: str,
                        expected_count: int | None, variables: dict[str, str],
                        interpolate) -> tuple[str, list[int]] | None:
    pos = skip_space(block, match.end())
    if pos < len(block) and block[pos] == '(':
        pos = skip_space(block, pos + 1)
    body, end, _ = literal_at(block, pos)
    if body is None:
        name_match = re.match(r'[A-Za-z_][A-Za-z0-9_]*', block[pos:])
        if not name_match:
            return None
        body = variables.get(name_match.group(0))
        end = pos + name_match.end()
        if body is None:
            return None
    if '\\$' not in body:
        body = interpolate(body)

    if expected_count == 0:
        return body, []

    rest = block[end:end + 600]
    if expected_count is None and 'assertInlineViolations' in block[match.start():match.end()]:
        lines = [index + 1 for index, line in enumerate(body.split('\n')) if '// violation' in line]
        return (body, lines) if lines else None

    numbers = LINE_NUMBER.findall(rest)
    if not numbers:
        numbers = POSITIONAL.findall(rest)
    if expected_count is None:
        return (body, [int(number) for number in numbers]) if numbers else None
    if len(numbers) < expected_count:
        # assertSingleViolation(SOURCE) without a line number: any line will do.
        return (body, []) if expected_count == 1 else None
    return body, [int(number) for number in numbers[:expected_count]]


def cases_in_class(path: pathlib.Path, rule_name: str, interpolate,
                   shared: dict[str, str] | None = None) -> list[dict]:
    """Collects one case per assertion, pairing it with the source in scope.

    Assignments and assertions are walked in source order: a method that reassigns
    ``SOURCE`` twice must not pair the first assertion with the second snippet.
    """
    text = path.read_text()
    methods = split_methods(text)
    shared_literals = dict(shared or {})
    shared_literals.update(class_level_literals(text, methods))

    cases = []
    for method, block in methods:
        spans = literal_spans(block)
        events = []
        for match in ASSIGNMENT.finditer(block):
            if not outside(spans, match.start()):
                continue
            body, _, _ = literal_at(block, skip_space(block, match.end()))
            if body is not None:
                events.append((match.start(), 'assign', match.group(1), body, None))
        for kind, pattern, expected_count in ASSERTIONS:
            for match in pattern.finditer(block):
                events.append((match.start(), 'assert', kind, match, expected_count))
        events.sort(key=lambda event: event[0])

        properties = properties_in(block)
        source_path, source_name = source_location(block)
        variables = dict(shared_literals)
        for _, event, name, payload, expected_count in events:
            if event == 'assign':
                variables[name] = payload
                continue
            result = case_from_assertion(block, payload, rule_name, expected_count, variables, interpolate)
            if result is None:
                continue
            body, lines = result
            cases.append({
                'method': method,
                'kind': name,
                'source': body,
                'lines': lines,
                'properties': properties,
                'path': source_path,
                'name': source_name,
            })
    return cases


def class_level_literals(text: str, methods: list[tuple[str, str]]) -> dict[str, str]:
    """Literals declared outside every method, which every test method can see."""
    covered = []
    for _, block in methods:
        start = text.find(block)
        if start >= 0:
            covered.append((start, start + len(block)))
    variables = {}
    for match in ASSIGNMENT.finditer(text):
        if any(start <= match.start() < end for start, end in covered):
            continue
        body, _, _ = literal_at(text, skip_space(text, match.end()))
        if body is not None:
            variables[match.group(1)] = body
    return variables


# --------------------------------------------------------------------------- #
# Abstract test cases that subclasses parameterise
# --------------------------------------------------------------------------- #


SUPERCLASS = re.compile(r'\bclass\s+\w+\s+extends\s+([A-Za-z_][A-Za-z0-9_]*)\s*<')
FIELD_LITERAL = re.compile(r"\b(?:final\s+)?String\s+(\w+)\s*=\s*'([^'\n]*)'")
GETTER_LITERAL = re.compile(r"\b(\w+)\(\)\s*\{\s*return\s*([^\n]+?)\s*\}")


def parameterised_cases(project: pathlib.Path, test_path: pathlib.Path, rule_name: str) -> list[dict]:
    """Expands a shared abstract test case for one subclass.

    Several rule families share their snippets in an abstract ``*RuleTestCase``:
    the class-reference rules fill in ``getClassName()``, the Javadoc rules fill in
    ``getTag()``. The subclass names the value; this joins the two.
    """
    subclass = test_path.read_text()
    superclass = SUPERCLASS.search(subclass)
    if not superclass:
        return []
    base = next((candidate for candidate in project.glob(f'src/test/groovy/**/{superclass.group(1)}.groovy')), None)
    if base is None:
        return []

    values = {name: literal for name, literal in FIELD_LITERAL.findall(subclass)}
    for getter, expression in GETTER_LITERAL.findall(subclass):
        literal, _, _ = literal_at(expression, 0)
        if literal is None:
            literal, _, _ = literal_at(expression, 1) if expression[:1] in '"\'' else (None, 0, False)
        if literal is not None:
            values[getter] = literal

    def interpolate(body: str) -> str:
        body = body.replace('${rule.name}', rule_name)
        for name, value in values.items():
            body = body.replace('${%s()}' % name, value)
            body = body.replace('${%s}' % name, value)
        body = body.replace('${getClassName()}', values.get('className', ''))
        body = body.replace('${className}', values.get('className', ''))
        return body

    cases = cases_in_class(base, rule_name, interpolate)
    for case in cases:
        case['method'] = f'{superclass.group(1)}_{case["method"]}'
    return cases


# --------------------------------------------------------------------------- #
# Driver
# --------------------------------------------------------------------------- #

DECLARATION = re.compile(r'\bclass\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:extends|<)')


def rule_class_names(project: pathlib.Path) -> dict[str, str]:
    """Maps every rule class declared under src/main to its fully qualified name.

    A rule class is not always in a file named after itself (``DuplicateListLiteralRule``
    lives in ``DuplicateListLiteral.groovy``), so the declaration is what counts.
    """
    found = {}
    for path in (project / 'src/main/groovy').rglob('*.groovy'):
        package = '.'.join(path.relative_to(project / 'src/main/groovy').parts[:-1])
        for match in DECLARATION.finditer(path.read_text(errors='replace')):
            name = match.group(1)
            if name.endswith('Rule'):
                found.setdefault(name, f'{package}.{name}')
    return found


def harvest(project: pathlib.Path, output: pathlib.Path) -> dict[str, int]:
    test_root = project / 'src/test/groovy'
    counters: collections.Counter[str] = collections.Counter()
    known_classes = rule_class_names(project)
    rows = []

    for test_path in sorted(test_root.rglob('*RuleTest.groovy')):
        rule_name = test_path.stem[:-len('RuleTest')]
        if f'{rule_name}Rule' not in known_classes:
            counters['skipped: no rule class'] += 1
            continue
        cases = parameterised_cases(project, test_path, rule_name)
        if not cases:
            cases = cases_in_class(test_path, rule_name, lambda body: body)
        if not cases:
            counters['skipped: no cases'] += 1
            continue

        seen = set()
        kept = []
        for case in cases:
            key = (case['kind'], tuple(case['lines']), case['source'], case['path'] or '')
            if key in seen:
                continue
            seen.add(key)
            kept.append(case)

        for index, case in enumerate(kept):
            name = f'{index:03d}.groovy'
            folder = output / rule_name
            folder.mkdir(parents=True, exist_ok=True)
            (folder / name).write_text(case['source'])
            if case['properties']:
                counters['cases with properties'] += 1
            # CodeNarc numbers the line "before the file" as 0 (the blank line a
            # package should follow); our lines are one-based, so 0 is the top. That
            # can collapse two expected lines into one, so keep them unique.
            numbers = sorted({line if line else 1 for line in case['lines']})
            lines = '-'
            if case['kind'] != 'none':
                lines = ','.join(str(line) for line in numbers)
            rows.append([
                rule_name,
                f'{rule_name}/{name}',
                case['path'] or '',
                lines,
                ','.join(f'{key}={value}' for key, value in sorted(case['properties'].items())),
            ])
            counters['cases'] += 1
            counters['positive' if case['kind'] != 'none' else 'negative'] += 1
        counters['rules'] += 1

    rows.sort()
    manifest = output / 'expectations.tsv'
    manifest.write_text('\n'.join('\t'.join(row) for row in rows) + '\n')
    return dict(counters)


LEDGER = 'known-differences.tsv'

NOTICE = """\
The Groovy snippets in this directory are copied from CodeNarc's rule tests
(src/test/groovy/**/*RuleTest.groovy) and remain under the Apache License 2.0.

    CodeNarc - Static analysis for Groovy
    Copyright 2005-2024 the original author or authors.
    https://codenarc.org

expectations.tsv is generated by benchmarks/harvest_codenarc_fixtures.py; do not
edit it by hand. Each row is: rule, case file, source path, expected lines,
rule properties. The expected lines are the line numbers CodeNarc asserts a
violation on; an empty field means "exactly one violation, any line", and "-"
means "no violations".
"""


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--codenarc-src', type=pathlib.Path, required=True,
                        help='CodeNarc checkout, e.g. ~/Code/CodeNarc-3.7.0')
    parser.add_argument('--output', type=pathlib.Path,
                        default=pathlib.Path(__file__).resolve().parent.parent / 'fixtures/codenarc')
    arguments = parser.parse_args()

    project = arguments.codenarc_src.resolve()
    if not (project / 'src/test/groovy').is_dir():
        print(f'{project} does not look like a CodeNarc checkout', file=sys.stderr)
        return 2
    output = arguments.output.resolve()
    # The ledger of known differences is curated by hand, so it must survive a
    # regeneration of the snippets it talks about.
    ledger = None
    if (output / LEDGER).is_file():
        ledger = (output / LEDGER).read_text()
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True)
    if ledger is not None:
        (output / LEDGER).write_text(ledger)

    counters = harvest(project, output)
    (output / 'NOTICE').write_text(NOTICE)
    print(json.dumps(counters, indent=2, sort_keys=True))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
