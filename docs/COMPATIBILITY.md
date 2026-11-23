# CodeNarc compatibility notes

Baseline: **CodeNarc 4.0.0** (released 2026-07-18). The CodeNarc 4.0.0 artifact
used for the comparisons depended on Groovy 5.0.3.

## Current coverage

All 44 rules in CodeNarc 4.0.0's `rulesets/basic.xml` are implemented, plus the
token-level `UnnecessarySemicolon` rule from CodeNarc's `unnecessary` ruleset.

Each implemented rule has a minimal Groovy fixture at
`crates/groovylint-cli/tests/fixtures/rules/<Rule>.groovy`. The end-to-end test
runs the CLI with a temporary ruleset containing exactly that rule and asserts
the rule name and line.

## Differential validation

For each rule fixture, CodeNarc and `groovylint` were run with the same
single-rule ruleset. The comparison normalizes findings to `(rule, line)`.

CodeNarc invocation shape:

```bash
java -cp "$CODENARC_CLASSPATH" org.codenarc.CodeNarc \
  -basedir=crates/groovylint-cli/tests/fixtures/rules \
  -sourcefiles=<Rule>.groovy \
  -rulesetfiles=file:<generated-single-rule-ruleset.xml> \
  -report=text:stdout
```

`groovylint` invocation shape:

```bash
cargo run -p groovylint -- check \
  --format json \
  --config <generated-single-rule-ruleset.xml> \
  crates/groovylint-cli/tests/fixtures/rules/<Rule>.groovy
```

Result at the time of writing: **45/45 fixtures matched** on rule name and line
(44 basic rules plus `UnnecessarySemicolon`).

The first fixture set is intentionally minimal. It proves that each rule is
wired through the parser, rule engine, configuration, and reporter; it does not
yet prove full edge-case equivalence. The next compatibility step is to port
more of CodeNarc's own positive/negative test cases per rule.

## Current compatibility gaps

- Rule properties, CodeNarc base-rule filters, `@SuppressWarnings`, and
  comment-based disable directives are not implemented yet.
- The parser is a common-syntax subset, not full Groovy. Unsupported syntax
  becomes a `ParseError` diagnostic for that file.
- Severity mapping is currently `priority 2 -> warning`, `priority 3 -> info`;
  priority-1 mapping and report-format parity are future work.
- Some rule messages are more generic than CodeNarc's. Rule name and line are
  the primary compatibility assertions today.
- The Sonar Groovy plugin may bundle a different CodeNarc version. The actual
  production version should be pinned before claiming compatibility there.
