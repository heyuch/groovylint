# Rule fixtures taken from CodeNarc

Coverage of the real codebase proves a rule is *useful*; it does not prove the rule
is *correct*. CodeNarc ships a fixture for every rule it tests, and those fixtures are
what this corpus is made of.

## Where the corpus comes from

CodeNarc keeps its fixtures inline in `src/test/groovy/**/*RuleTest.groovy`, as string
constants handed to `assertSingleViolation` and friends:

```groovy
final SOURCE = '''
    class MyClass {
        def myClosure = {
            if (x==23) {
            }
        }
    }
'''
assertSingleViolation(SOURCE, 4, 'if (x==23) {')
```

The constant is passed to the rule verbatim, so the line number CodeNarc asserts is a
real line number in the snippet. `benchmarks/harvest_codenarc_fixtures.py` extracts
them:

```bash
git clone --depth 1 --branch v3.7.0 https://github.com/CodeNarc/CodeNarc /tmp/CodeNarc-3.7.0
python3 benchmarks/harvest_codenarc_fixtures.py --codenarc-src /tmp/CodeNarc-3.7.0
```

It writes `fixtures/codenarc/`:

| Path | Contents |
|---|---|
| `<Rule>/<NNN>.groovy` | one snippet |
| `<Rule>/<NNN>.properties` | the `rule.x = ...` the test applied before asserting |
| `expectations.tsv` | `rule`, `case`, `source path`, `expected lines`, `properties` |
| `known-differences.tsv` | cases that do not match yet, with a cause |
| `NOTICE` | Apache-2.0 attribution |

What the harvester has to undo, because the tests are Groovy rather than plain text:

- escapes are processed in every literal, triple-quoted included (`'''` is a plain
  string, not a raw one), and a backslash-newline pair disappears;
- `.stripIndent()` / `.stripMargin()` / `.trim()` after a literal run before the rule
  sees the string;
- some rule families keep their snippets in an abstract test case and fill in
  `getClassName()` / `getTag()`; the subclass supplies the value;
- `assertNoViolations` proves the rule stays quiet, `assertSingleViolation` without a
  line number only requires one violation, and `0` means "the line before the file".

## What it says today

```
2959 cases (1565 expected violations, 1394 expected silence) over 398 rules
2199 match CodeNarc      617 recorded as known differences      143 for rules out of scope
```

Of the 382 rules this project registers, 378 have fixtures. The 20 corpus rules without
a match here are the Grails/Jenkins/CPS rules that `docs/COVERAGE.md` already declares
out of scope.

Known differences, by cause:

| Cause | Cases | What it means |
|---|---|---|
| `missed` | 256 | the rule does not report a case CodeNarc reports |
| `false-positive` | 201 | the rule reports a case CodeNarc allows |
| `other-line` | 104 | the rule fires, but points at a different line |
| `ours-parser` | 56 | the snippet uses syntax our parser rejects |

The `ours-parser` cases are a parser work list, not a rule work list. The first pass
over them removed the largest clusters: `def` in a parameter list, modifiers after a
type (`def static X`), method names that are strings (`def 'a test'()`), destructuring
declarations (`def (a, b) = ...`), `{ -> ... }` as a statement, labels
(`given: 'x'`) and SPOCK-style specs. What is left is rarer syntax. The count is pinned by
`PARSER_BUDGET` in the test, so a parser regression fails the build instead of quietly
joining the ledger.

`MethodSize`, `ClassSize`, `AbcMetric` and `CyclomaticComplexity` now use `GMetrics`'
definitions: assignment/branch/condition vectors for ABC (magnitude rounded to one
decimal), decision points for cyclomatic, closure fields and the script body measured as
methods, class thresholds of 0 disabled the way Groovy truth disables them, and
`ignoreMethodNames` read as wildcards.

The rules with the most failing cases, and why they are worth starting with:

| Rule | Failing | Cause |
|---|---|---|
| `Indentation` | 8 | CodeNarc's indentation state machine differs on nested blocks |
| `CrapMetric` | 3 | needs cyclomatic complexity plus test coverage per method |
| `IllegalClassMember`, `ImplementationAsType`, `MissingOverrideAnnotation` | 5 each | need type resolution; see `ROADMAP.md` §7.5 |
| `NestedBlockDepth` | 5 | which statements count as a block |
| `Space*Brace` | 4 each | the character-level skipping rules |
| `IllegalPackageReference` | 4 | line attribution inside multi-line declarations |

Metric rules (`AbcMetric`, `CyclomaticComplexity`, `CrapMetric`) differ in how they
count, which is a different discussion from correctness.

## Running it

`cargo test -p groovylint-rules --test codenarc_fixtures` — about 0.3 s, in-process, no
subprocess per case. It prints:

```
codenarc fixtures: 1964 match, 879 known differences (124 blocked by our parser), 143 for rules out of scope
```

## Fixing a difference

Fix the rule, then delete its row from `known-differences.tsv`. The test fails when a
listed case starts matching, and when a listed case is no longer in the corpus, so the
ledger cannot hide progress or rot. Rows may name a single case
(`EmptyIfStatement/000.groovy`) or a whole rule (`AbcMetric/*`) when every failing case
of that rule shares one cause.

Regenerate the corpus after changing the harvester, not after fixing a rule. To rebuild
the ledger from the current code in one step — it fails if the result is not green —

```bash
python3 benchmarks/refresh_known_differences.py
```

A case where our answer is the correct one and CodeNarc's is not is recorded with the
`better:` reason instead of being fixed. `refresh_known_differences.py` keeps those rows
across a refresh, so a deliberate difference cannot be lost by accident.

## What this does not cover

- Properties outside a test method: a fixture only carries what the test set in the
  method that asserts.
- Rules whose fixtures are generated from other sources (the class-reference rules are
  expanded through their abstract test case, but a new abstract base would need adding
  to the harvester).
- Message wording. Only `(file, rule, line)` is compared, the same key the corpus
  comparison in `docs/COVERAGE.md` uses.
