# CodeNarc compatibility notes

Baseline: **CodeNarc 4.0.0** (released 2026-07-18). The CodeNarc 4.0.0 artifact
used for the first comparison depended on Groovy 5.0.3.

## First differential check

The bundled fixture
[`basic.groovy`](../crates/groovylint-cli/tests/fixtures/basic.groovy) was run
through both tools.

CodeNarc:

```bash
java -cp 'lib/*' org.codenarc.CodeNarc \
  -basedir=crates/groovylint-cli/tests/fixtures \
  -sourcefiles=basic.groovy \
  -rulesetfiles=rulesets/basic.xml \
  -report=text:stdout
```

`groovylint`:

```bash
cargo run -p groovylint -- check \
  crates/groovylint-cli/tests/fixtures/basic.groovy
```

Result for the seven implemented `basic` rules:

| Rule | Line | CodeNarc message | `groovylint` |
| --- | --- | --- | --- |
| `EmptyIfStatement` | 3 | The if statement is empty | match |
| `EmptyElseBlock` | 4 | The else block is empty | match |
| `EmptyWhileStatement` | 7 | The while statement is empty | match |
| `EmptyForStatement` | 10 | The for statement is empty | match |
| `EmptyTryBlock` | 13 | The try block is empty | match |
| `EmptyCatchBlock` | 14 | The catch block is empty | match |
| `EmptyFinallyBlock` | 15 | The finally block is empty | match |

CodeNarc reported: `P1=0 P2=7 P3=0`. `groovylint` reported seven warnings.
Rule name, line, and message matched for all seven findings.

A focused ruleset containing only `EmptyIfStatement` and
`UnnecessarySemicolon` was also compared. CodeNarc reported:

- `EmptyIfStatement`, line 3, priority 2, message
  `The if statement is empty`;
- `UnnecessarySemicolon`, line 21, priority 3, message
  `Semicolons as line endings can be removed safely`.

`groovylint` produced the same rule names, lines, and messages (severity
`warning` and `info` respectively).

## Current compatibility gaps

- The default `basic.xml` contains 44 rules; only seven are implemented.
  Unsupported rules are counted and skipped, not silently ignored.
- Rule properties, CodeNarc base-rule filters, `@SuppressWarnings`, and
  comment-based disable directives are not implemented yet.
- The parser is a common-syntax subset, not full Groovy. Invalid/unsupported
  syntax becomes a `ParseError` diagnostic for that file.
- Severity mapping is currently `priority 2 -> warning`, `priority 3 -> info`;
  priority-1 mapping and report-format parity are future work.
- The Sonar Groovy plugin may bundle a different CodeNarc version. The actual
  production version should be pinned before claiming compatibility there.

## Reproducing locally

1. Put the CodeNarc 4.0.0 jar and its runtime dependencies on a classpath.
2. Run the CodeNarc command above.
3. Run the `groovylint` command above.
4. Compare `(rule, line, message)` tuples.

The comparison is intentionally based on the same rule set and same source
files. Performance comparisons should only be made after finding parity is
confirmed.
