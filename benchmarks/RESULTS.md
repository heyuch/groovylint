# Local benchmark results

These are local, indicative results, not a controlled lab measurement. They were
collected with the same staged corpus and CodeNarc's `rulesets/basic.xml` for
both tools.

## Environment

- Apple Mac mini, Apple M4, 10 logical cores, 16 GB RAM
- macOS 27.2.1
- Rust 1.100.0 (`cargo build --release`)
- Java 23.0.1
- CodeNarc 4.0.0 (Groovy 5.0.3 runtime)
- `groovylint` default all-basic-rules engine, 10 worker threads
- CodeNarc runs its normal single-process, sequential file pipeline

## Corpus 1: Apache Groovy subprojects

- Source: <https://github.com/apache/groovy>
- Commit: `4f83940a46200dbe632569069516c6fd096c8731` (2026-11-22)
- Corpus path: `subprojects/` (all `.groovy` files)
- Total `.groovy` files: 913
- Parseable by current `groovylint`: **734** (80.4%)
- Findings: **100** in both tools
- Normalized `(file, rule, line)` differences: **0**
- 3 iterations after warm-up

| Tool | Median | Min | Max | Files/s |
| --- | ---: | ---: | ---: | ---: |
| `groovylint` | 0.034 s | 0.034 s | 0.035 s | 21,573 |
| CodeNarc 4.0.0 | 8.142 s | 7.950 s | 8.325 s | 90 |

Approximate end-to-end ratio on this corpus: **239x**.

## Corpus 2: CodeNarc source

- Source: <https://github.com/CodeNarc/CodeNarc>
- Commit: `9f4c706` (2026-11-19)
- Corpus path: `src/main/groovy`
- Total `.groovy` files: 535
- Parseable by current `groovylint`: **479** (89.5%)
- Findings: **0** in both tools (the codebase is clean under this ruleset)
- Normalized differences: **0**
- 3 iterations after warm-up

| Tool | Median | Min | Max | Files/s |
| --- | ---: | ---: | ---: | ---: |
| `groovylint` | 0.013 s | 0.012 s | 0.018 s | 36,465 |
| CodeNarc 4.0.0 | 4.718 s | 4.189 s | 5.187 s | 101 |

Approximate end-to-end ratio on this corpus: **363x**.

## Criterion microbenchmarks

Command:

```bash
cargo bench -p groovylint --bench linter -- --quick
```

- `parse_200_groovy_units`: 9.66–10.06 ms
- `lint_basic_rules` (all 44 basic rules, 200 repeated units): 44.1–45.2 µs

## Interpretation

- The parser excludes files it cannot yet parse; the staged corpus therefore
  measures rule execution and parsing on the supported subset, not full-language
  coverage.
- CodeNarc is sequential and JVM-based; `groovylint` uses file-level parallelism.
  A large part of the end-to-end gap is architectural, not a claim that a single
  rule is 200x faster.
- JVM startup is included in each CodeNarc invocation. The gap would shrink on a
  corpus small enough for startup to dominate.
- Finding parity here is strong but limited to the two corpora and the basic
  ruleset. It should be re-run after parser or rule changes.
- Performance should only be compared when finding parity holds; the script
  reports differences before timing for that reason.
