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
2986 cases (1589 expected violations, 1397 expected silence) over 398 rules
1964 match CodeNarc      879 recorded as known differences      143 for rules out of scope
```

Of the 382 rules this project registers, 378 have fixtures. The 20 corpus rules without
a match here are the Grails/Jenkins/CPS rules that `docs/COVERAGE.md` already declares
out of scope.

Known differences, by cause:

| Cause | Cases | What it means |
|---|---|---|
| `missed` | 373 | the rule does not report a case CodeNarc reports |
| `false-positive` | 253 | the rule reports a case CodeNarc allows |
| `other-line` | 129 | the rule fires, but points at a different line |
| `ours-parser` | 124 | the snippet uses syntax our parser rejects |

The `ours-parser` cases are a parser work list, not a rule work list. They cluster on
`def` in a parameter list, modifiers after a type (`def static X`), annotations on
imports, `->` in closure parameter lists, and map-literal colons. The count is pinned by
`PARSER_BUDGET` in the test, so a parser regression fails the build instead of quietly
joining the ledger.

The rules with the most failing cases, and why they are worth starting with:

| Rule | Failing | Cause |
|---|---|---|
| `BracesForMethod` | 21 | we point at the brace, CodeNarc at the declaration; also parser cases |
| `IllegalPackageReference` | 17 | we only inspect imports; CodeNarc inspects type references anywhere |
| `Indentation` | 17 | counts indentation differently for nested closures |
| `PrivateFieldCouldBeFinal` | 17 | we report fields CodeNarc leaves alone |
| `UnnecessaryDefInMethodDeclaration` | 16 | mostly our parser (`def` in a parameter list) |
| `SynchronizedOnBoxedPrimitive` | 14 | we see fewer synchronised targets |
| `ClassEndsWithBlankLine`, `ClassStartsWithBlankLine` | 13 each | blank-line boundary handling |
| `SpaceAroundOperator` | 13 | mostly our parser (escapes in the snippets were the cause) |
| `UnnecessaryPackageReference` | 12 | `java.lang` default-import handling |

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

Regenerate the corpus after changing the harvester, not after fixing a rule.

## What this does not cover

- Properties outside a test method: a fixture only carries what the test set in the
  method that asserts.
- Rules whose fixtures are generated from other sources (the class-reference rules are
  expanded through their abstract test case, but a new abstract base would need adding
  to the harvester).
- Message wording. Only `(file, rule, line)` is compared, the same key the corpus
  comparison in `docs/COVERAGE.md` uses.
