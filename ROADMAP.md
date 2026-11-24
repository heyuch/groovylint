# groovylint Roadmap

Status: proposal, 2026-11-25.

This roadmap orders the work after the current CodeNarc rule-expansion phase. It
is deliberately opinionated about what to build first, what to defer, and what
not to build.

## 1. Scope decisions

### In scope

- CodeNarc 4.0.0 behavior for all non-framework rules.
- Spock-related rules (the project still uses Spock-style tests).
- A lossless syntax layer that keeps the semantic AST and a future formatter
  independent.
- Safe autofix with explicit suggestions and text edits.
- ArchUnit-like source dependency rules:
  - module-to-module dependencies;
  - package-to-package dependencies;
  - class-to-class dependencies;
  - class-in-package and package-layering constraints;
  - cycle detection for modules/packages/slices.
- A declarative architecture DSL, with a Gradle DSL facade.
- A custom-rule plugin mechanism.
- Flow-sensitive nullness analysis without requiring `@Nullable`.
- Performance and differential testing as first-class gates.

### Explicitly out of scope

- Grails rules: all 12 Grails rule classes.
- Jenkins rules: all 7 Jenkins rule classes.
- Windows and Intel macOS distribution for now.
- A full Groovy compiler, bytecode generator, or complete nominal type system.
- Exact ArchUnit bytecode-level semantics in the first architecture release.
- Whole-program sound nullness for dynamic Groovy and external libraries.
- In-process execution of arbitrary Groovy scripts as rules.
- Native dynamic-library plugins (`.so`/`.dylib`) with an unstable Rust ABI.
- A formatter implementation in the near-term milestones. The syntax layer is
  made formatter-ready, but the formatter is a later milestone.
- Language-server integration in this roadmap.

## 2. Current baseline

CodeNarc 4.0.0 exposes **406 concrete rule classes** across its 24 rulesets.
The current registry contains:

| Item | Count |
|---|---:|
| Registered entries | 328 |
| Unique registered rule classes | 327 |
| CodeNarc 4.0.0 classes not registered | 79 |
| Grails classes not registered (excluded) | 12 |
| Jenkins classes registered but excluded from the target | 7 |
| Non-framework missing rules | 67 |
| Non-framework registered rules that are stubs/partial | 8 |
| In-scope target | 387 |

There is one duplicate registry entry:
`ThrowExceptionFromFinallyBlock` is registered from both `finally.rs` and
`design.rs`. It is deduplicated when loaded by CodeNarc class name, but the
registry should have one canonical descriptor.

The detailed missing-rule list is in [`docs/COVERAGE.md`](docs/COVERAGE.md).

### What “implemented” means

A rule is only counted as implemented when:

1. it is registered under the correct CodeNarc class name;
2. it has at least one positive fixture;
3. it has at least one negative fixture (does not report an equivalent clean
   case);
4. its configured properties are honoured;
5. it passes differential validation against CodeNarc 4.0.0 on a fixture and,
   where practical, a real corpus.

The 8 non-framework stubs do **not** meet this definition. They are registered so
rulesets load, but they currently report nothing or only approximate a check.

## 3. Target architecture

The current pipeline is per-file and AST-only. Architecture, autofix, and
nullness need facts that are computed once and shared.

```text
source
  -> lossless lexer (tokens + trivia)
  -> parser events
      -> semantic AST                         (rules, analysis)
      -> lossless CST                         (autofix, formatter)
  -> FileFacts                                (classes, imports, references)
  -> ProjectIndex                             (symbols, packages, modules)
  -> DependencyGraph
  -> ProjectRule / ArchitectureRule
  -> CFG per method
  -> data-flow / nullness
  -> FixEngine (text edits)                  (autofix)
```

New core concepts:

- `Trivia` and lossless tokens: comments, whitespace, shebang, line endings.
- `SyntaxNode`/CST: exact source reconstruction for formatting and safe edits.
- `FileFacts`: immutable per-file summary; project rules do not need the AST.
- `ProjectIndex`: class/package/module symbols and containment.
- `DependencyGraph`: typed edges with source spans.
- `ProjectRule`: a rule capability that runs once over the project.
- `Cfg` and a worklist data-flow framework: on-demand per method.
- `TextEdit`/`Fix`: explicit source edits with safety levels.
- `FixEngine`: selection, conflict handling, application, reparse, re-lint.
- `RuleStatus`: `Implemented`, `Partial`, `Stub`, `Excluded`, `Missing`.
- `Fixability`: `None`, `Suggestion`, `Safe`, `Unsafe`.

The CLI must be able to run in two modes:

- **file mode**: current behavior, all per-file rules, no project graph;
- **project mode**: build the index and run project rules; enabled automatically
  when architecture/nullness rules or project plugins are configured.

## 4. Milestones

Estimates are rough person-weeks for one experienced engineer and exclude large
parser regressions.

### M0 — Coverage accounting and cleanup (0.2, 2–3 days)

- Add `RuleStatus` to metadata and a `groovylint rules --coverage` command.
- Mark the 7 Jenkins classes as `Excluded`; do not report them as missing.
- Keep the 12 Grails classes as `Excluded`.
- Remove the duplicate `ThrowExceptionFromFinallyBlock` registration.
- Add `Fixability` metadata (the edit engine itself lands in M3).
- Add a generated coverage test so the expected 387-rule target cannot silently
  regress.
- Add negative fixtures for every currently implemented rule.
- Correct the stale 327/323 counts in existing documentation.

Exit criteria:

- `groovylint rules --coverage --format json` reports exact counts by status and
  fixability.
- CI fails if a rule is added/removed without updating the expected manifest.
- No duplicate CodeNarc classes.

### M1 — Complete non-framework CodeNarc behavior (0.3, 3–4 weeks)

Implement the 67 missing non-framework rules and replace the 8 stubs.

Suggested batches:

1. **Groovy idioms and unnecessary code (40 rules)**
   - all 33 `groovyism` rules;
   - all 7 missing `unnecessary` rules.
2. **Low-cost structure rules (14 rules)**
   - `dry` 2;
   - `formatting` 2;
   - `convention` 5;
   - `design` 5.
3. **Metrics and remaining singles (13 rules)**
   - `size` 7;
   - `SynchronizedOnThis`;
   - `CatchThrowable`;
   - `MisorderedStaticImports`;
   - `SpockUseVerifyEach`;
   - `FactoryMethodName`;
   - `JavaIoPackageAccess`.
4. **Stubs (8 rules)**
   - `Indentation`;
   - `BusyWait`;
   - `DoubleCheckedLocking`;
   - `InconsistentPropertyLocking`;
   - `InconsistentPropertySynchronization`;
   - `NoScriptBindings`;
   - `ExceptionNotThrown`;
   - `IllegalClassMember`.

Some size rules need method/class metrics, and some stubs need CFG or
project-specific knowledge. Implement the simple, high-confidence form first and
mark the rule `Partial` until its differential coverage is acceptable.

Exit criteria:

- 387/387 in-scope classes have a non-stub implementation.
- Differential tests pass for all rule fixtures.
- On the benchmark corpus, no unexplained CodeNarc-only findings.
- No performance regression beyond the agreed budget.

### M2 — Lossless syntax foundation (0.4, 2–3 weeks)

Make the syntax layer ready for autofix and a future formatter without coupling
the formatter to rules.

- Add trivia-preserving tokenization: whitespace, line comments, block comments,
  shebang, newlines.
- Add a lossless CST with exact source reconstruction.
- Add `parse_lossless` while keeping `parse` as the AST-only fast path.
- Add parser error recovery and collected errors.
- Guarantee `groovylint-syntax` has no dependency on rules, config, analysis,
  reporting, or CLI.
- Add round-trip, comment-preservation, Unicode, CRLF, and fuzz tests.
- Add AST-only and CST parse benchmarks.

Design: [`docs/SYNTAX.md`](docs/SYNTAX.md).

Exit criteria:

- Every parseable corpus file round-trips exactly through the CST.
- Comments and shebangs are never discarded by the lossless path.
- AST-only parse performance stays within the existing budget.
- The formatter can be built later in a separate crate using only syntax APIs.

### M3 — Safe autofix (0.5, 2–3 weeks)

Implement the edit engine and a conservative first set of fixes. This can begin
with token-level fixes in parallel with M2, but it should not ship before
comment-preservation tests exist.

- Add `TextEdit`, `Fix`, `FixSafety`, and `Fixability`.
- Extend diagnostics and JSON output with fix metadata.
- Implement selection, overlap rejection, atomic writes, reparse, re-lint, and
  bounded fixpoint iteration.
- Add `--fix`, `--fix --dry-run`, and `--fix --unsafe`.
- Add explicit Gradle fix tasks; `check` stays read-only.
- Ship safe fixes for a small set of local rules first:
  - `UnnecessarySemicolon`;
  - `TrailingWhitespace`;
  - `UnusedImport` (after trivia handling);
  - `UnnecessaryPublicModifier`;
  - unambiguous spacing rules.
- Add suggestion-only fixes for rules where semantics are context-dependent.

Design: [`docs/AUTOFIX.md`](docs/AUTOFIX.md).

Exit criteria:

- A fixed file always reparses.
- Safe fixes do not change comments, shebangs, or unrelated formatting.
- Fix-then-lint reaches a fixed point or stops deterministically.
- No rule writes files directly.
- `check` never mutates source.

### M4 — Project graph and ArchUnit-like rules (0.6, 4–6 weeks)

Deliver the dependency graph and architecture rules. This also supplies facts
for project plugins.

Components:

1. `FileFacts` extraction:
   - package/imports;
   - class/interface/trait/enum names;
   - extends/implements;
   - field/parameter/return/local types;
   - annotations;
   - `new` and class literals;
   - method/static call references;
   - declaration and reference spans.
2. Name resolution:
   - explicit fully-qualified names;
   - imports, aliases, wildcard imports, static imports;
   - same package and `java.lang`;
   - project class index;
   - unresolved external references retained as symbolic names.
3. `ProjectIndex` and `DependencyGraph`.
4. Project rule execution after all files are parsed.
5. Configuration model and DSL.
6. Diagnostics at the exact dependency site, with fallback to the declaration.

Initial rule types:

- `module-dependency`: forbid, require, or allow-only.
- `package-dependency`: forbid, require, or allow-only.
- `class-dependency`: forbid or allow-only.
- `class-in-package`: classes matching a pattern must reside in a package.
- `layers`: a higher layer may depend only on allowed lower layers.
- `slices`: package/class slices may not create forbidden cycles.

Design summary: section 5 below.

Exit criteria:

- Graph built once per project and reused.
- A 10k-class synthetic project builds its graph in a reasonable time.
- Rules report module/package/class violations with precise source locations.
- Graph and rule output are deterministic under parallel parsing.
- No false dependency edges from comments, strings, or unused imports unless a
  rule explicitly asks for imports.

### M5 — CFG and nullness without annotations (0.7, 6–8 weeks)

Implement the analysis core and a separate `Nullness` rule pack.

Phase 5a: intraprocedural definite-null detection.

- CFG for methods, scripts, closures, branches, loops, try/catch/finally,
  return/throw/break/continue.
- Forward may/must analysis over a nullness lattice.
- Definite-null dereference diagnostics.
- Branch narrowing for `== null`, `!= null`, `instanceof`, truthiness, `assert`,
  early return/throw, `&&`/`||`, Elvis and safe navigation.
- Loop fixed points.
- No false positives by default: unresolved external calls are `Unknown`, not
  `MaybeNull`.

Phase 5b: interprocedural summaries.

- Method return-nullness summaries from all return statements.
- Parameter dereference requirements (a parameter dereferenced without a null
  guard is non-null at that point).
- Project call graph and summary fixpoint.
- Constructor/field initialization analysis for simple fields.
- Closure capture handling.
- Strict mode: report possible-null findings and contract violations.

Phase 5c: optional annotation/bytecode refinement.

- Honour `@Nullable`/`@NonNull` when present, but never require them.
- Optionally read Java/Groovy class-file annotations for external APIs later.
- Never claim soundness for dynamic dispatch, `method_missing`, metaclass
  changes, or unknown external libraries.

Exit criteria:

- A nullness fixture suite covers branches, loops, try/finally, closures,
  fields, returns, parameters, and external unknowns.
- Definite-null mode has a strict false-positive budget on real code.
- The checker is opt-in until precision is demonstrated.
- No crashes/timeouts on the benchmark corpora.

### M6 — Custom-rule plugin mechanism (0.8, 3–4 weeks)

Ship the architecture DSL first, then a process-based plugin protocol.

Why a process protocol, not an in-process ABI:

- no unstable Rust dynamic-library ABI;
- plugins can be written in Rust, Java, Groovy, Python, or any language;
- a plugin crash cannot crash the analyzer process;
- easier to version and validate than a native ABI.

Proposed manifest:

```toml
api_version = 1
name = "acme-rules"
command = ["acme-groovylint-plugin"]
```

The host invokes the plugin once per project with a versioned JSON request. The
request contains the project root, files, configuration, and normalized
`FileFacts`/dependency facts (not a raw full AST at first). The plugin responds
with diagnostics in the same schema as built-in rules. The host validates paths
and spans, applies severity/suppression, and sorts results.

Design summary: section 6 below.

Exit criteria:

- A sample plugin in Rust or Python passes an end-to-end test.
- Plugin failures are isolated and produce a clear diagnostic.
- Plugin rules are opt-in; no automatic downloads or discovery.
- API version mismatch is rejected cleanly.

### M7 — Formatter and hardening (post-1.0, ongoing)

- Build `groovylint-format` as a separate crate depending only on
  `groovylint-syntax`.
- Add style configuration and a safe formatting CLI.
- Require CST round-trip, comment preservation, and idempotence tests.
- Continue real-corpus differential testing and performance tuning.
- Add more CodeNarc positive/negative fixtures.
- Tune architecture rule patterns and module source mapping.
- Tune nullness precision and performance.
- Add plugin examples and SDK documentation.
- Revisit Windows/Intel macOS/Linux arm64 distribution.
- Consider LSP/auto-fix UI integration only after the core contracts are stable.

## 5. ArchUnit-like DSL proposal

Canonical configuration should be declarative and usable from the CLI. TOML is
the current preference because the tool is Rust-native and the format is easy to
validate:

```toml
[architecture]
modules = { web = ["web/**"], service = ["service/**"], repository = ["repository/**"] }

[[architecture.rules]]
id = "web-must-not-use-repository"
type = "module-dependency"
from = "web"
to = "repository"
effect = "forbid"

[[architecture.rules]]
id = "service-layer"
type = "layers"
layers = ["web", "service", "repository"]
transitive = false

[[architecture.rules]]
id = "controllers-in-web"
type = "class-in-package"
classes = "com.acme..*Controller"
package = "com.acme.web.."
```

The Gradle plugin should expose a Groovy/Kotlin DSL facade that produces the
same model:

```groovy
groovylint {
    architecture {
        module('web') { includes 'web/**' }
        module('service') { includes 'service/**' }
        module('repository') { includes 'repository/**' }

        rule('web-must-not-use-repository') {
            module('web').mayNotDependOn(module('repository'))
        }
    }
}
```

Design constraints:

- The DSL is declarative; it does not execute arbitrary Groovy.
- Package/class patterns use ArchUnit-like `..` and `*` semantics.
- The same rule model is used by the CLI and Gradle.
- Start with source-level dependencies; bytecode-level exactness is a later
  decision, not an assumed feature.

## 6. Plugin mechanism design

Two layers:

1. **Declarative custom rules** (M4/M6):
   - architecture rules;
   - simple forbidden/required pattern rules;
   - no code execution;
   - safest default for teams.
2. **Process plugins** (M6):
   - arbitrary logic in any language;
   - explicit configuration and trust;
   - versioned JSON protocol;
   - ideal for team-specific conventions and framework checks.

Do **not** start with:

- dynamic Rust `.so`/`.dylib` loading;
- in-process Groovy script evaluation;
- implicit plugin discovery from dependencies;
- a WASM ABI before the facts/protocol schema is stable.

## 7. Nullness design summary

Nullness lattice:

```text
Unreachable  <  Null / NonNull  <  MaybeNull  <  Unknown
```

- `Unreachable` is the data-flow bottom.
- `Null` and `NonNull` are precise.
- `MaybeNull` means at least one path can be null.
- `Unknown` means no usable information (external/dynamic); never warn by default.

Initial transfer rules:

- `null` -> `Null`; literals, `new`, collections, closures -> `NonNull`.
- variable read -> environment lookup.
- assignment -> right-hand value, with declared/annotation refinement.
- `== null` / `!= null` -> branch refinement.
- method call -> project summary or `Unknown` for external methods.
- ternary/Elvis -> join of both outcomes.
- safe navigation -> result `MaybeNull`, receiver not dereferenced unsafely.

Why annotations are optional but not sufficient to ignore:

- local data flow can prove many dereferences safe or unsafe;
- project method summaries can infer return nullness;
- external libraries and dynamic dispatch have no local proof;
- without annotations, the analyzer must choose conservative unknown handling;
- therefore the first release reports **definite null** only, with possible-null
  and contract checks in strict mode.

## 8. Testing and performance gates

Every milestone adds:

- positive and negative fixtures;
- differential tests against CodeNarc 4.0.0 where applicable;
- deterministic output tests for parallel runs;
- a benchmark on the pinned corpora;
- a regression budget for files/second and memory.

Syntax-specific tests:

- CST round-trip over parseable corpora;
- comment/shebang preservation;
- CRLF, Unicode, and byte-span tests;
- parser panic fuzzing;
- AST-only and CST benchmarks.

Autofix-specific tests:

- fix-then-parse and fix-then-lint;
- idempotence and conflict rejection;
- golden diffs;
- corpus dry-run must never produce invalid source.

Architecture-specific tests:

- synthetic graphs with known edges;
- cycles, layers, slices, wildcard imports, aliases, unresolved references;
- graph build determinism.

Nullness-specific tests:

- branch/loop/try-finally/closure/field/parameter fixtures;
- false-positive corpus and false-negative corpus;
- real-project smoke runs;
- no panics/timeouts.

## 9. Non-goals and anti-goals

- Do not implement Grails or Jenkins rules.
- Do not make the semantic AST carry comments/trivia or formatting policy.
- Do not make the CST the rule API.
- Do not implement autofix by reprinting the whole file.
- Do not chase 100% bytecode-level ArchUnit parity in the first graph release.
- Do not make nullness sound for dynamic Groovy or external libraries.
- Do not execute arbitrary Groovy in the analyzer process.
- Do not expose unstable Rust ABIs.
- Do not let plugin failures take down the main analysis.
- Do not add LSP or formatter UI before the syntax and analysis APIs stabilize.
- Do not optimize before the differential and benchmark suites exist.

## 10. Open decisions

1. Architecture config: TOML canonical, with Gradle DSL facade (current lean).
2. Should module mapping come from Gradle project metadata, path globs, or both?
3. Should architecture dependency extraction include unresolved external edges by
   default, or only resolvable project/source edges?
4. Plugin protocol: JSON process first (current lean); WASM later.
5. Nullness default for unannotated parameters/fields: `Unknown` (current lean)
   versus assumed non-null.
6. Whether to read JDK/classpath bytecode annotations in M5c or defer to 1.x.
7. Whether to remove Jenkins stubs entirely or keep them as `Excluded` metadata.
8. CST implementation: `rowan` versus a small custom tree; decide after a
   prototype and memory/performance benchmark.
9. Autofix default: safe-only (current lean) versus opt-in unsafe-by-default.
