# CodeNarc compatibility notes

Baseline: **CodeNarc 4.0.0** (released 2026-07-18). The CodeNarc 4.0.0 artifact
used for the comparisons depended on Groovy 5.0.3.

## Current coverage

125 rules are registered. That includes all 44 rules in CodeNarc 4.0.0's
`rulesets/basic.xml`, `UnnecessarySemicolon`, and every rule a production
codebase uses in its `codenarc-bug.xml` quality gate (96 rule classes across
`basic`, `braces`, `convention`, `dry`, `enhanced`, `generic`, `imports`,
`jdbc`, `logging`, `naming`, `security`, `serialization` and `unused`).

64 rules have a minimal Groovy fixture at
`crates/groovylint-cli/tests/fixtures/rules/<Rule>.groovy`; the rest are covered
by the corpus comparison above. The end-to-end test
runs the CLI with a temporary ruleset containing exactly that rule and asserts
the rule name and line.

## Comparing against CodeNarc on a real codebase

CodeNarc needs minutes where `groovylint` needs seconds, so the comparison runs
CodeNarc once and reuses its XML report:

```bash
# once per corpus change
java -cp "$CODENARC_CLASSPATH" org.codenarc.CodeNarc \
    -basedir=. -includes='claim/claim-domain/src/**/*.groovy,component-utils/src/**/*.groovy' \
    -rulesetfiles=file:buildScript/codenarc/codenarc-bug.xml \
    -report=xml:/tmp/codenarc.xml

# after every groovylint change (< 1 s)
python3 benchmarks/diff_codenarc_report.py \
    --codenarc-report /tmp/codenarc.xml --corpus . \
    --config buildScript/codenarc/codenarc-bug.xml \
    --paths claim/claim-domain/src component-utils/src
```

Latest result on that two-module slice with the production `codenarc-bug.xml`:
**CodeNarc 913 findings, `groovylint` 951, 909 of them identical on
`(file, rule, line)`** (99.6%). The differences are listed below.

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
(44 basic rules plus `UnnecessarySemicolon`). On a 734-file parseable subset of
Apache Groovy subprojects, both tools reported the same 100 findings
`(file, rule, line)` with zero differences. On a 479-file subset of CodeNarc's
own source, both reported zero findings. See
[`benchmarks/RESULTS.md`](../benchmarks/RESULTS.md).

The first fixture set is intentionally minimal. It proves that each rule is
wired through the parser, rule engine, configuration, and reporter; it does not
yet prove full edge-case equivalence. The next compatibility step is to port
more of CodeNarc's own positive/negative test cases per rule.

## Current compatibility gaps

- Rule properties are supported for the rules that declare them.
  `@SuppressWarnings` on classes, methods and fields is honoured, including
  `"all"`. CodeNarc base-rule filters (`applyToClassNames`,
  `doNotApplyToFilesMatching`, ...) and comment-based disable directives are not
  implemented yet.
- Four deliberate differences from CodeNarc remain, all of them cases where
  CodeNarc's own implementation under-reports:
  - `DuplicateListLiteral`/`DuplicateMapLiteral`: we flag repeated collection
    literals built only from constants; CodeNarc's visitor only inspects
    constant nodes, so it almost never fires.
  - `MissingOverrideAnnotation`: we flag `equals`/`hashCode`/`toString`/`clone`/
    `finalize` overrides without `@Override`; CodeNarc depends on resolved types
    and misses most of them.
  - `InsecureRandom`: we flag `new Random()`; CodeNarc only flags types that
    implement `java.util.Random`, so the direct allocation slips through.
  - `JdbcConnectionReference`: we report declarations of `java.sql.Connection`,
    CodeNarc additionally reports the import line.
- The parser is a common-syntax subset, not full Groovy. Unsupported syntax
  becomes a `ParseError` diagnostic for that file. On a 118,470-file private
  Groovy corpus (Spring/Grails services, Spock tests, Gradle build logic) every
  file now parses; the corpus reported 4081 parse failures before the syntax
  work. Remaining approximations are listed below.
- Syntax that intentionally has no dedicated AST node: `try`-with-resources
  drops the resource declarations, `??` and `??.` are modelled as `?:` and
  `?.`, and `switch` expressions store each arm as an expression statement.
- Severity mapping is currently `priority 2 -> warning`, `priority 3 -> info`;
  priority-1 mapping and report-format parity are future work.
- Some rule messages are more generic than CodeNarc's. Rule name and line are
  the primary compatibility assertions today.
- The Sonar Groovy plugin may bundle a different CodeNarc version. The actual
  production version should be pinned before claiming compatibility there.
