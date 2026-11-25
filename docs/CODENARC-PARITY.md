# CodeNarc parity

Which of CodeNarc's findings groovylint reproduces, measured on a production
corpus, and where the two tools intentionally disagree.

## Why this is a requirement

The company CI runs CodeNarc over the changed files of every push. groovylint is
meant to replace it locally, so a change that passes groovylint locally must not
be rejected later. That makes the comparison asymmetric:

- **missing** findings are bugs: the local run said green and CI would say red;
- **extra** findings are acceptable in principle, but every one of them is noise a
  developer has to read, and CI lints only changed files, so a file being edited
  shows the extras that CI would never print. They are handled case by case.

## Method

`benchmarks/diff_codenarc_report.py` parses a cached CodeNarc XML report, runs
groovylint with the same ruleset on the same files, and compares the sets of
`(file, rule, line)`. One iteration takes about a second instead of the 16
minutes CodeNarc needs for the corpus.

- Ruleset: `buildScript/codenarc/codenarc-bug.xml` (the CI subset, 96 rules)
- Corpus: 24,345 `.groovy` files across 45 modules
- CodeNarc: 3.7.0-groovy-4.0, one JVM per module (a single JVM runs out of heap)

## Result

| | CodeNarc | groovylint |
|---|---:|---:|
| Findings | 5,117 | 5,259 |
| Identical `(file, rule, line)` | 5,115 | 5,115 |
| Coverage of CodeNarc | — | **99.96%** |
| Positions groovylint reports that CodeNarc does not | — | 132 |
| Wall clock | 15m46s | 4s |

The two remaining CodeNarc-only findings are artifacts, not gaps:

- `PropertyName` on an interface constant: CodeNarc applies the lowercase
  property pattern to one interface field but not the other 815 occurrences in
  the corpus. Mirroring it would add 815 false positives.
- `UnusedImport` reported at line `0`: the violation carries no position, so it
  cannot be matched. The import is genuinely unused.

## Where groovylint reports more

| Rule | Count | Why |
|---|---:|---|
| `JdbcConnectionReference`, `JdbcStatementReference`, `JdbcResultSetReference` | 70 | CodeNarc's visitor only reaches the import (and re-reports it once per nested class); it never sees field, parameter or return types. Every extra is a real `java.sql.*` reference in a file that imports `java.sql.*`. |
| `UnusedImport` | 34 | CodeNarc counts a member call on another object as a use of a static import, and a plain string as a use of a name. All 34 extras are imports whose name appears only in comments, strings, or nowhere. |
| `MissingOverrideAnnotation` | 18 | CodeNarc resolves superclasses from the analysed sources only, so it never sees `Object.toString`/`equals`/`hashCode`. The extras are methods that do override a JDK method. |
| `DuplicateMapLiteral` | 4 | Two of them are in a `build.gradle`, which CodeNarc does not scan; two are named arguments of a constructor, which CodeNarc also reports — the difference is CodeNarc's file-level test filter. |
| `LoggerWithWrongModifiers` | 2 | CodeNarc compares the field type against the resolved `org.slf4j.Logger`; in a single-file analysis the type is the simple name `Logger`, so it never fires. |
| `LoggingSwallowsStacktrace`, `ConfusingMethodName`, `UnsafeImplementationAsMap` | 3 | Narrower CodeNarc conditions (documented per rule in the code). |

These are deliberately kept: each one is a true finding by the rule's own
description. The alternative — suppressing them to match CodeNarc exactly —
would hide real problems.

## Rules that report less than their description suggests

Twelve registered rules are structurally unable to report on a single file, and
say so in a `shortcut:` comment: `Indentation`, `BusyWait`,
`DoubleCheckedLocking`, `InconsistentProperty*`, `NoScriptBindings`,
`ExceptionNotThrown`, `Cps*`, `ExpressionInCps*`, `ForbiddenCall*`,
`ParameterOrReturn*`, `Illegal*`/`Required*`/`StatelessClass`/`IllegalClassMember`.
They need type resolution, data flow, or a whole-project index. CodeNarc can only
implement them with a classpath; groovylint currently has none.

## Regenerating

```bash
python3 benchmarks/diff_codenarc_report.py \
  --codenarc-report /tmp/cn-bug-full.xml \
  --corpus ~/Code/SaaS/saas-service \
  --config buildScript/codenarc/codenarc-bug.xml \
  --paths $(cat /tmp/cn-modules.txt)
```

CodeNarc itself is run per module, otherwise the JVM exhausts the heap:

```bash
java -cp "$(cat /tmp/cn-cp.txt)" org.codenarc.CodeNarc \
  -rulesetfiles=file:$PWD/buildScript/codenarc/codenarc-bug.xml \
  -includes='**/*.groovy' -basedir=$PWD/<module> \
  -report=xml:/tmp/cn-bug-chunks/<module>.xml
```
