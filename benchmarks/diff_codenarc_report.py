#!/usr/bin/env python3
"""Diff groovylint against a previously generated CodeNarc XML report.

CodeNarc is slow (minutes on a large module) while groovylint takes seconds, so
the CodeNarc report is produced once and reused. Pass it with
``--codenarc-report`` and save it alongside the corpus.

Example:

    java -cp "$CODENARC_CLASSPATH" org.codenarc.CodeNarc \
        -basedir=. -includes='foo/**/*.groovy' \
        -rulesetfiles=file:buildScript/codenarc/codenarc-bug.xml \
        -report=xml:/tmp/foo-codenarc.xml

    python3 benchmarks/diff_codenarc_report.py \
        --codenarc-report /tmp/foo-codenarc.xml \
        --corpus . \
        --config buildScript/codenarc/codenarc-bug.xml \
        --paths foo/src

The comparison ignores messages and compares ``(file, rule, line)``.
"""

from __future__ import annotations

import argparse
import collections
import json
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path


def parse_codenarc(path: Path) -> set[tuple[str, str, int]]:
    findings = set()
    for package in ET.parse(path).getroot().iter("Package"):
        folder = package.get("path") or ""
        for file_node in package.iter("File"):
            name = file_node.get("name") or ""
            if not name.endswith(".groovy"):
                continue
            relative = f"{folder}/{name}".strip("/")
            for violation in file_node.iter("Violation"):
                findings.add(
                    (
                        relative,
                        violation.get("ruleName") or "",
                        int(violation.get("lineNumber") or 0),
                    )
                )
    return findings


def run_groovylint(
    binary: Path, corpus: Path, config: Path | None, paths: list[str]
) -> tuple[set[tuple[str, str, int]], str]:
    command = [str(binary), "check", "--format", "json"]
    if config is not None:
        command += ["--config", str(config)]
    command += [str(corpus / path) for path in paths]
    result = subprocess.run(command, capture_output=True, text=True, check=False)
    if not result.stdout:
        raise RuntimeError(f"groovylint produced no JSON:\n{result.stderr}")
    report = json.loads(result.stdout)
    findings = set()
    for file_report in report["files"]:
        path = Path(file_report["path"])
        try:
            relative = str(path.relative_to(corpus)).replace("\\", "/")
        except ValueError:
            relative = str(path)
        for diagnostic in file_report["diagnostics"]:
            findings.add((relative, diagnostic["rule"], diagnostic["start"]["line"]))
    return findings, result.stderr


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--codenarc-report", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--config", type=Path)
    parser.add_argument("--binary", type=Path, default=Path("target/release/groovylint"))
    parser.add_argument(
        "--paths",
        nargs="+",
        default=["."],
        help="corpus-relative paths to scan (must match the CodeNarc -includes)",
    )
    parser.add_argument("--show", type=int, default=8, help="sample size per bucket")
    parser.add_argument("--ignore", nargs="*", default=[], help="rules to leave out")
    args = parser.parse_args()

    codenarc = parse_codenarc(args.codenarc_report)
    ours, stderr = run_groovylint(
        args.binary.resolve(),
        args.corpus.resolve(),
        args.config.resolve() if args.config else None,
        args.paths,
    )
    for rule in args.ignore:
        codenarc = {finding for finding in codenarc if finding[1] != rule}
        ours = {finding for finding in ours if finding[1] != rule}

    for line in stderr.splitlines():
        if "not implemented" in line:
            print(line.strip())

    only_codenarc = codenarc - ours
    only_ours = ours - codenarc
    print(f"CodeNarc: {len(codenarc)}  groovylint: {len(ours)}  shared: {len(codenarc & ours)}")
    print(
        "only CodeNarc:",
        len(only_codenarc),
        collections.Counter(rule for _, rule, _ in only_codenarc).most_common(12),
    )
    print(
        "only groovylint:",
        len(only_ours),
        collections.Counter(rule for _, rule, _ in only_ours).most_common(12),
    )

    for title, findings in (("only CodeNarc", only_codenarc), ("only groovylint", only_ours)):
        if not findings:
            continue
        print(f"--- {title} ---")
        for relative, rule, line in sorted(findings)[: args.show]:
            print(f"  {rule:28} {relative}:{line}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
