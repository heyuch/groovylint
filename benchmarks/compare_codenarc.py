#!/usr/bin/env python3
"""Run groovylint and CodeNarc on the same parseable Groovy corpus.

This script deliberately excludes files that groovylint cannot parse yet, so the
two tools do the same amount of lint work. It reports wall-clock medians and a
normalized finding comparison.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import statistics
import subprocess
import tempfile
import time
from collections import Counter
from pathlib import Path


def run(command: list[str], *, stdout=subprocess.PIPE, cwd: Path | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(
        command,
        cwd=cwd,
        stdout=stdout,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )


def scan_parseable(binary: Path, corpus: Path) -> tuple[list[Path], dict]:
    result = run([str(binary), "check", "--format", "json", str(corpus)])
    if not result.stdout:
        raise RuntimeError(f"groovylint produced no JSON:\n{result.stderr}")
    report = json.loads(result.stdout)
    parseable = []
    for file_report in report["files"]:
        if not any(d["rule"] == "ParseError" for d in file_report["diagnostics"]):
            parseable.append(Path(file_report["path"]))
    return parseable, report


def stage_corpus(files: list[Path], corpus: Path, workdir: Path) -> Path:
    staged = workdir / "corpus"
    staged.mkdir()
    for path in files:
        relative = path.relative_to(corpus)
        target = staged / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(path, target)
    return staged


def parse_codenarc_text(path: Path) -> set[tuple[str, str, int]]:
    findings = set()
    current = None
    for line in path.read_text(errors="replace").splitlines():
        file_match = re.match(r"File: (.+)$", line.strip())
        if file_match:
            current = file_match.group(1).replace("\\", "/")
            continue
        violation = re.search(r"Rule=(\w+) P=\d+ Line=(\d+)", line)
        if violation and current is not None:
            findings.add((current, violation.group(1), int(violation.group(2))))
    return findings


def parse_groovylint_json(data: dict, corpus: Path) -> set[tuple[str, str, int]]:
    findings = set()
    for file_report in data["files"]:
        path = Path(file_report["path"])
        try:
            relative = str(path.relative_to(corpus)).replace("\\", "/")
        except ValueError:
            relative = str(path)
        for diagnostic in file_report["diagnostics"]:
            findings.add((relative, diagnostic["rule"], diagnostic["start"]["line"]))
    return findings


def timed(command: list[str], cwd: Path | None, stdout=subprocess.DEVNULL) -> float:
    start = time.perf_counter()
    result = run(command, cwd=cwd, stdout=stdout)
    elapsed = time.perf_counter() - start
    if result.returncode not in (0, 1):
        raise RuntimeError(f"command failed ({result.returncode}): {' '.join(command)}\n{result.stderr}")
    return elapsed


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--binary", type=Path, default=Path("target/release/groovylint"))
    parser.add_argument("--codenarc-classpath", default=os.environ.get("CODENARC_CLASSPATH"))
    parser.add_argument("--iterations", type=int, default=3)
    parser.add_argument(
        "--ruleset",
        type=Path,
        help="CodeNarc XML ruleset to use for both tools (default: rulesets/basic.xml)",
    )
    parser.add_argument(
        "--skip-timing",
        action="store_true",
        help="only compare findings, do not measure wall-clock time",
    )
    parser.add_argument("--threads", type=int)
    parser.add_argument("--keep-workdir", action="store_true")
    args = parser.parse_args()

    if not args.corpus.is_dir():
        parser.error(f"corpus is not a directory: {args.corpus}")
    if not args.codenarc_classpath:
        parser.error("set --codenarc-classpath or CODENARC_CLASSPATH")
    binary = args.binary.resolve()
    corpus = args.corpus.resolve()
    if not binary.is_file():
        parser.error(f"binary not found: {binary} (run cargo build --release)")

    workdir = Path(tempfile.mkdtemp(prefix="groovylint-bench-"))
    try:
        files, _ = scan_parseable(binary, corpus)
        if not files:
            print("no parseable files")
            return 2
        staged = stage_corpus(files, corpus, workdir)
        print(f"corpus: {len(files)} parseable files (from {corpus})")

        ruleset = args.ruleset.resolve() if args.ruleset else None
        cn_report = workdir / "codenarc.txt"
        ruleset_arg = "rulesets/basic.xml"
        if ruleset is not None:
            ruleset_arg = f"file:{ruleset}"
        cn_base = [
            "java",
            "-cp",
            args.codenarc_classpath,
            "org.codenarc.CodeNarc",
            f"-basedir={staged}",
            "-includes=**/*.groovy",
            f"-rulesetfiles={ruleset_arg}",
            f"-report=text:{cn_report}",
            "-failOnError=false",
        ]
        our_cmd = [
            str(binary),
            "check",
            "--format",
            "json",
            "--threads",
            str(args.threads or os.cpu_count() or 1),
        ]
        if ruleset is not None:
            our_cmd += ["--config", str(ruleset)]
        our_cmd.append(str(staged))

        first = run(our_cmd)
        if first.returncode not in (0, 1):
            raise RuntimeError(first.stderr)
        skipped = re.search(
            r"(\d+) CodeNarc rule\(s\) are not implemented", first.stderr
        )
        if skipped:
            print(f"groovylint skipped {skipped.group(1)} unimplemented rules")
        our_data = json.loads(first.stdout)
        cn_first = run(cn_base)
        if not cn_report.exists():
            raise RuntimeError(f"CodeNarc produced no report:\n{cn_first.stderr}")

        cn_findings = parse_codenarc_text(cn_report)
        our_findings = parse_groovylint_json(our_data, staged)
        print(f"CodeNarc findings: {len(cn_findings)}")
        print(f"groovylint findings: {len(our_findings)}")
        only_cn = cn_findings - our_findings
        only_ours = our_findings - cn_findings
        print(f"findings only in CodeNarc: {len(only_cn)}")
        print(f"findings only in groovylint: {len(only_ours)}")
        if only_cn:
            print("  sample CodeNarc-only:", sorted(only_cn)[:5])
        if only_ours:
            print("  sample groovylint-only:", sorted(only_ours)[:5])
        missing_by_rule = Counter(rule for _, rule, _ in only_cn)
        extra_by_rule = Counter(rule for _, rule, _ in only_ours)
        if missing_by_rule:
            print("CodeNarc-only by rule:", missing_by_rule.most_common(15))
        if extra_by_rule:
            print("groovylint-only by rule:", extra_by_rule.most_common(15))
        shared = Counter(rule for _, rule, _ in cn_findings & our_findings)
        if shared:
            print("matched by rule:", shared.most_common(15))
        if args.skip_timing:
            return 0

        # Warm up both tools once.
        timed(our_cmd, None)
        timed(cn_base, None)

        ours = [timed(our_cmd, None) for _ in range(args.iterations)]
        cn = [timed(cn_base, None) for _ in range(args.iterations)]

        def summarize(name: str, samples: list[float]) -> None:
            median = statistics.median(samples)
            print(
                f"{name}: median={median:.3f}s min={min(samples):.3f}s "
                f"max={max(samples):.3f}s files/s={len(files) / median:.1f}"
            )

        print(f"iterations: {args.iterations}, files: {len(files)}")
        summarize("groovylint", ours)
        summarize("CodeNarc", cn)
        return 0
    finally:
        if args.keep_workdir:
            print(f"workdir: {workdir}")
        else:
            shutil.rmtree(workdir, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
