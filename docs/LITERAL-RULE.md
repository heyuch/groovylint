# Literal rule: analysis and implementation plan

Status: **proposal** (analysis only, nothing implemented).

`com.bytesforce.literal.LiteralRule` in `saas-service/buildSrc` flags places that
compare strings where an enum would do. This document records what the rule does,
how it behaves on the corpus, what it would take to build it here, and where we
can be more precise.

## 1. What the rule does

### Detection

For every class (including inner and anonymous classes) it looks at four shapes,
which is where Groovy asks "is this value a certain code":

| Shape | Example |
|---|---|
| `==` / `!=` | `if (status == "FAILED")` |
| `in` | `if (flowType?.code in ["NATIVE_LOGIN", "USER_SELF_REGISTRATION"])` |
| `equals` / `equalsIgnoreCase` | `if (type.equals("OIDC"))` |
| `case` | `case "PDF":` |

Everything string-shaped inside the compared value expression is collected:
literals in lists and argument tuples are recursed into, GStrings, numbers and
`null` are not reported, and property/method *names* are skipped (Groovy models
`Enum.CONST.code` and `"x".trim()` with `ConstantExpression` nodes too). A
comparison where the other side is also a constant or a list is left to CodeNarc's
`ComparisonOfTwoConstants`.

### Classification

A repository-scoped index maps **enum constant name → declaring enums + module**.
It is built by scanning `src/main/groovy` files of the checked modules with
regular expressions (enum head, then the text up to the first `;`, split on
commas, keep `[A-Z][A-Z0-9_]*` names). Four categories:

| Category | Meaning | Reported |
|---|---|---|
| A | the same module declares an enum constant with this name | yes, priority 2 |
| B | only another module does | no, summary only |
| C | nothing declares it, and the literal looks like a code (`[A-Z][A-Z0-9_]+`) | yes, priority 3 |
| D | not code-shaped (file suffix, separator, SQLState) | no, summary |

`A` is the actionable category ("use the enum"); `C` asks a human to decide
"enum or dictionary value". Scope is ratchet-friendly: only the modules owning the
checked files are indexed, so a changed-files run costs 0.1–0.3 s instead of 3.5 s.

### Measured behaviour on `saas-service`

`./gradlew staticChecks -Prule=literal -PstaticAll`, 206 modules, 20,825 files:

| | count |
|---|---|
| A (enum exists) | **199** |
| C (no enum anywhere) | **686** |
| B / D (not reported) | 0 / 1,371 |
| rule time (sum of modules) | **434 s** (~21 ms/file, Groovy compilation to CONVERSION) |
| wall time including Gradle | 2 m 42 s |

Sample findings:

```
SecretManagerApplication.groovy:599  C  `OIDC` compared with protocol via ==
PageFlowClientImpl.groovy:108        A  `NATIVE_LOGIN` compared with flow.flowType?.code via in; use PageFlowTypeEnum
ReleaseVersionApplication.groovy:396 C  `FAILURE` compared with taskProgress.taskStatus via !=
```

## 2. Is it implementable here?

Yes, and it is cheaper than the original. The rule needs exactly one thing the
current engine does not have: a **project-wide enum constant index**. Everything
else exists.

| Need | Status |
|---|---|
| `==`, `!=`, `in`, `case`, `equals`/`equalsIgnoreCase` | AST has `BinaryOp::{Eq, NotEq, In}`, `Stmt::Switch`, `ExprKind::Call` |
| String literals with spans; lists, tuples, parens | present |
| Skip property/method names | structural — our AST separates `Property{name}` and `Call{name}`, so no visitor tricks are needed |
| Exclude GStrings | needs one check (our GString is `Literal::String` with `${…}` text) |
| Enum names and constants per file | available after the enum-constant change (`parse_enum_constants` now emits `FieldDecl`s) |
| Cross-file index | **missing** — see §3 |
| Suppressions, severities, JSON/text output | present |

We can also fix the original's weak spot: its index is regex-scraped text. Our
parser reads 24k files in ~1.6 s single-threaded (0.05 s for 838 files, measured),
so indexing can use the real AST — no missed enums behind comments, strings, nested
classes or `public enum` modifiers, and no mis-splitting on a `;` inside a constant
expression (a `shortcut:` the original documents).

## 3. Design

### Where the index lives

The CLI already collects the file list before linting. Add a two-pass step:

```
run_check:
  1. collect files
  2. parse + collect (Module, EnumClass, Constant) per file   <- parallel, no rules
  3. build EnumIndex { constant -> {module, enum} }
  4. lint each file with &EnumIndex available to rules
```

The index is read-only and shared, so it fits the existing per-file parallel
engine. `FileContext` gains one field (`enum_index: Option<&EnumIndex>`), or the
engine keeps it and passes it through the rule instance at construction. Files
that no rule needs an index for pay nothing.

Memory: a few thousand entries for a 24k-file corpus (measured 199+686 hits from
an index of ~3k constant names) — negligible, and no ASTs are retained between
passes.

### Rule inputs

```
LiteralRule (new, category `convention`, severity warning)
  properties:
    reportCategories   = "A,C"            # which categories are diagnostics
    constantPattern    = "[A-Z][A-Z0-9_]*"  # names indexed from enums
    codePattern        = "[A-Z][A-Z0-9_]+"  # shape required for category C
    indexPaths         = "**/src/main/groovy/**"  # what the index scans
    moduleDepth        = 1                # first path segment = module
    ignoreLiterals     = ""               # comma separated, e.g. ".json,UTF-8"
```

Defaults mirror the original, so the same corpus should produce the same 885
findings. `moduleDepth` covers repositories without a monorepo layout.

### Semantics to keep (and where to diverge)

Keep:

- category A matches **by constant name only**, not by value — the intent is "this
  name already exists as a constant, use it", and value comparison would need
  resolved enum types;
- category C requires an all-caps code shape, so `"OIDC"` is reported but
  `"application/json"` is not;
- test sources are not indexed (`indexPaths` above), so a test-only enum never
  produces an `A`.

Diverge (configurable, off by default):

- index **all** analyzed paths instead of only `src/main/groovy`;
- index mixed-case constant names (`constantPattern` property);
- report `B` ("another module has this name") as a hint.

### Reporting

Diagnostic per literal, at the literal's span (the original reports the enclosing
comparison's line; our spans are exact), with the receiver text:

```
PageFlowClientImpl.groovy:108:41  warning[LiteralRule] `NATIVE_LOGIN` is compared with
    flow.flowType?.code via in; PageFlowTypeEnum declares this constant in this module
```

Category in the code suffix (`LiteralRule/A`, `LiteralRule/C`) keeps JSON output
machine-readable and lets `--format json` consumers filter, mirroring the
original's `--fail-on`.

## 4. Work estimate

| Step | Scope |
|---|---|
| `EnumIndex` (parse + collect + module mapping) | ~120 lines + tests |
| `LiteralRule` (4 shapes, literal collection, classification) | ~200 lines |
| Engine plumbing (`FileContext` field, two-pass CLI, plugin) | ~40 lines |
| Fixtures: one per shape, A/B/C/D, GString exclusion, nested enum, comments | ~12 cases |
| Parity harness: parse the original's `literal.txt` report, diff `(file, rule, line)` | ~60 lines (reuses `diff_codenarc_report.py` shape) |

Roughly one day. The parity target is measurable: the original's report files are
already on disk under `<module>/build/reports/static-checks/literal.txt`.

## 5. Open questions

1. Should `C` be reported at all here? It is 686 of the 885 findings and asks for
   a human decision; the original reports it because the report is read, not
   enforced. If we run it in CI we may want `reportCategories = "A"` by default.
2. Index scope: only `src/main` (original) or also `src/test`? Indexing tests
   would turn some `C` into `A` against test fixtures, which is noise.
3. Do we want the `B` hint in JSON output (computed but not a diagnostic)?
4. Cross-check against the new nullness work: the same two-pass `ProjectIndex`
   will be needed by the architecture rules, so the literal rule is a good first
   consumer of it and should not invent its own mechanism.
