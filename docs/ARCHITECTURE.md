# Architecture and performance design

**Status:** design accepted in part; first vertical slice implemented
(2026-11-24). The parser is a self-developed subset, the default CodeNarc
the default `basic.xml` ruleset is wired in, 318 rules are implemented through
the layered engine (the in-scope subsets of the 96-rule production
`codenarc-bug.xml` ruleset and the 323-rule `codenarc.xml` gate; Grails,
Jenkins, and Spock rules are unsupported), rule `<property>` values are applied,
and `@SuppressWarnings` is honoured before reporting.
CFG/data-flow/nullness is not implemented yet. The parser covers the syntax of a
118,470-file private Groovy corpus end to end, including newline continuation
rules, GString interpolation, closures/lambdas, spread and method references. The benchmark harness and first public-corpus CodeNarc comparison are in place (`benchmarks/`).

This document combines the original design draft with a review of how existing
linters and CodeNarc actually work. It records the performance model, the parser
decision, and the compatibility and benchmark plans.

## 1. Goals and constraints

`groovylint` has two jobs:

1. **Behavioral compatibility with CodeNarc** for the rules used in CI. A
   violation CodeNarc reports should be reported by `groovylint` as well, unless
   a documented difference is accepted.
2. **Deeper semantic checks** such as flow-sensitive nullness, type inference,
   and eventually interprocedural analysis.

Hard constraints:

- CodeNarc behavior is the baseline; extra precision must not silently remove
  CodeNarc findings.
- Performance is a first-class requirement, measured against a pinned CodeNarc
  version on identical inputs and rule sets.
- Diagnostics must be deterministic regardless of thread count.
- Syntax errors must not abort a whole run.
- The architecture must stay testable and understandable while 400+ rules are
  added incrementally.

## 2. What CodeNarc actually does

These facts matter because they define both the compatibility target and the
performance opportunity.

- CodeNarc's latest release at the time of writing is **4.0.0** (2026-07-18).
  The compatibility baseline should be pinned to a release, not `master`.
- A scan of the CodeNarc 4.0.0 source tree found **410 `*Rule.groovy` files**.
  Roughly **366 rule files are based on `AbstractAstVisitorRule`** or its
  variants; **12 rules explicitly request `Phases.SEMANTIC_ANALYSIS`**.
- The default AST compiler phase is **`Phases.CONVERSION`** (`SourceCode`), not
  semantic analysis. Most CodeNarc rules do not have resolved types to work
  with.
- For each file, CodeNarc loops over every enabled rule and calls
  `rule.applyTo(...)`. An AST-visitor rule creates a visitor and walks the AST
  for its class nodes. In other words: **one AST parse, but many rule-specific
  AST traversals**.
- `FilesystemSourceAnalyzer` recursively processes files **sequentially**.
- CodeNarc has base-rule behavior that applies to every rule: `enabled`,
  `priority`, file/class name filters, regex path filters, `violationMessage`,
  and `@SuppressWarnings` / comment-based suppression.

Implications:

- There is a large, low-risk performance win available from file-level
  parallelism and one shared AST traversal for syntax rules.
- Full type inference is unnecessary for the majority of CodeNarc rules. It
  must be an **opt-in layer**, not a mandatory pipeline stage.
- Compatibility includes the less glamorous parts: base-rule filters,
  suppressions, priority mapping, ruleset properties, and line numbers.
- CodeNarc's conversion-phase AST has Groovy-specific shapes (script wrapper
  classes, declaration expressions, property/method-call nodes). Our AST or a
  lowering layer must be able to model those shapes.

References:

- [CodeNarc `AbstractAstVisitorRule`](https://github.com/CodeNarc/CodeNarc/blob/master/src/main/java/org/codenarc/rule/AbstractAstVisitorRule.java)
- [CodeNarc `AbstractSourceAnalyzer`](https://github.com/CodeNarc/CodeNarc/blob/master/src/main/groovy/org/codenarc/analyzer/AbstractSourceAnalyzer.groovy)
- [CodeNarc `FilesystemSourceAnalyzer`](https://github.com/CodeNarc/CodeNarc/blob/master/src/main/groovy/org/codenarc/analyzer/FilesystemSourceAnalyzer.groovy)
- [CodeNarc `SourceCode`](https://github.com/CodeNarc/CodeNarc/blob/master/src/main/groovy/org/codenarc/source/SourceCode.groovy)
- [CodeNarc integration notes: SonarQube uses CodeNarc](https://github.com/CodeNarc/CodeNarc/blob/master/docs/codenarc-other-tools-frameworks.md)

## 3. Performance model

### 3.1 Pipeline

```text
discovery + per-file settings
        |
        v
read + lex + parse                         (parallel across files)
        |
        v
one AST traversal:
  build scopes/symbols/facts
  dispatch syntax-rule events
  collect diagnostics
        |
        +--> deferred file-level rules (unused imports, unused variables)
        |
        +--> on-demand type facts (only if an enabled rule needs them)
        |
        +--> per-function CFG + data-flow (only if an enabled rule needs it)
        |
        v
normalize -> suppress -> sort -> report    (single consumer)
```

The pipeline is deliberately staged. Do not fuse every stage before profiling;
each stage should have clear ownership and a measurable cost.

### 3.2 Parallelism

Parallelize **files**, not individual rule callbacks:

- Worker owns one file at a time: read, parse, build facts, run rules, emit a
  per-file report, drop the AST.
- Use a work-stealing pool (for example Rayon) or a bounded worker queue.
- Collect results per file; aggregate and sort at the end. Never write to
  stdout/stderr from workers.
- Do not depend on completion order. A run with one worker and a run with `N`
  workers must produce identical diagnostics.
- Avoid nested parallelism initially. A huge file can be parallelized later by
  function for CFG/data-flow, not by launching a rule per thread.
- File discovery can be parallel, but keep a deterministic path ordering for
  reporting.
- Bound memory: do not retain all ASTs for a project. If cross-file analysis is
  added later, retain compact summaries/indexes, not full trees.

Small-file workloads can be dominated by thread overhead. The CLI should expose
`--threads` and default to `available_parallelism`, with a batch threshold to be
determined by benchmarks.

### 3.3 “One pass per layer” — precise version

The proposal is directionally right, but “one pass” needs a precise meaning:

- **One AST traversal** for syntax rules and scope/symbol construction.
- **One computation** of each shared fact. Rules query facts; they do not
  recompute them.
- **One rule dispatch** per event layer.
- Not every analysis is linear: type inference and data-flow may require
  fixpoint iteration. That is still “computed once per file/function”, not one
  linear scan.

Proposed layers, ordered by dependency:

| Layer | Facts / work | Who consumes it | When |
| --- | --- | --- | --- |
| L0 | source lines, trivia, tokens | physical/token rules, suppressions | during/after lex |
| L1 | typed AST, parent links, spans | all syntax rules | after parse |
| L2 | scopes, symbols, declarations, references, imports | semantic naming/unused rules | single AST traversal |
| L3 | types, annotations, resolved calls | type-aware rules (opt-in) | lazy / on demand |
| L4 | CFG, dominance, nullness, liveness | flow rules (opt-in) | per function, lazy |
| L5 | file summaries (unused, metrics) | deferred rules | after L1–L4 |
| L6 | project index | cross-file rules (future) | after all files |

Two important adjustments:

1. **L2 can be fused into the single AST traversal**: maintain a scope stack and
   symbol table while visiting nodes, and dispatch AST rules at the same time.
   Rules that need symbols query the facts built so far. Start with separate
   traversals if fusing complicates correctness; fuse only if profiling shows
   traversal cost is material.
2. **L3/L4 must be lazy**. CodeNarc's default phase and its 366 AST-visitor
   rules show that most rules never need types or a CFG. The engine should
   derive an `AnalysisPlan` from enabled rules and skip unneeded phases entirely.
   This is a larger win than micro-optimizing a type checker that did not need
   to run.

A rule declares its requirements, for example:

```text
Requirement bits: TOKENS | AST | SCOPES | TYPES | CFG | FILE_SUMMARY
```

The engine unions requirements from enabled rules, builds the plan, and runs only
the required layers. Disabling one semantic rule can remove an entire analysis
phase from the run.

### 3.4 Rule dispatch

The single traversal must dispatch efficiently. A naive `for rule in rules`
inside every node callback multiplies overhead by the rule count.

Design points:

- Provide **enter** and **exit** callbacks. Exit is required for post-order
  rules (unused declarations, scope finalization) and for scope stacks.
- Precompute a dispatch table from node kind to interested rule indices, or use
  a bitset per node kind. A rule that is interested in every expression opts
  into a general bucket.
- Rule instances are per-file and mutable (`&mut self`); configuration is
  immutable and shared. No rule may hold state across files.
- Rules receive an immutable file context: source, tokens, AST, facts, parent
  queries, and a diagnostic sink. They must not mutate the AST.
- Some rules are **deferred**: they run at file end after all facts exist. The
  engine has an explicit `on_file_end` hook rather than letting such rules walk
  the AST again.
- Rule registries are explicit compile-time lists initially. Do not build a
  dynamic plugin ABI before there is a requirement for one.

This is close to ESLint's selector listeners, Clippy's lint passes, and Ruff's
single AST checker:

- [ESLint single-traversal visitor](https://github.com/eslint/eslint/blob/main/lib/linter/linter.js)
- [Clippy EarlyLintPass vs LateLintPass](https://github.com/rust-lang/rust/blob/master/src/tools/clippy/book/src/development/lint_passes.md)
- [Ruff compilation pipeline](https://github.com/astral-sh/ruff/blob/main/CONTRIBUTING.md)
- [Oxc linter architecture](https://www.oxcjs.com/learn/architecture/linter)
- [Biome architecture](https://biomejs.dev/internals/architecture/)

### 3.5 Data representation

Performance-sensitive choices to validate with benchmarks, not assumptions:

- Byte spans as `u32` offsets; one source file is far below 4 GiB.
- Arena-style AST (`Vec<Node>` plus `NodeId(u32)`) to avoid one allocation per
  node and to make parent/child navigation cheap.
- Keep the token stream and comments/trivia; token and line rules should not
  re-lex or re-walk the AST.
- Intern identifiers per file, not globally, to avoid cross-thread contention.
- Avoid cloning AST nodes for rule execution. Pass node IDs and immutable
  references.
- Store source text once (`Arc<str>` or equivalent) and precompute a line-start
  table.
- Measure allocator behavior before adopting a global custom allocator. A
  per-file arena/bump allocation is usually a better first step.

## 4. Review of the proposed rule-layer design

What is right:

- Shared immutable AST, one traversal for syntax rules.
- Explicit dependency layers (AST → symbols → types → CFG).
- Adding rules incrementally and refactoring as patterns emerge.
- Using SOLID/GRASP to keep rule, analysis, and reporting responsibilities apart.

What to adjust:

1. **Do not conflate “layer” with “one linear pass.”** Type inference and
   data-flow use fixpoints. Define layers by facts and dependencies.
2. **Do not run type/CFG analysis unconditionally.** Most CodeNarc rules do not
   need it. Make analysis demand-driven.
3. **Add token/line and deferred layers.** Not every rule is an AST visitor;
   CodeNarc itself has non-AST rules and whole-file checks.
4. **Separate fact construction from rule dispatch in the design**, even if they
   share a traversal later. Otherwise rule order can accidentally affect facts.
5. **Parallelize files first.** Intra-file rule parallelism multiplies AST
   traversals and usually loses to a shared single traversal.
6. **Make diagnostics deterministic.** Sort after parallel execution; do not
   rely on completion order.
7. **Do not build a generic plugin framework yet.** Rule traits and static
   registration are enough. Use the “rule of three” before extracting a new
   abstraction.
8. **Treat CodeNarc compatibility as an engine concern**, not a per-rule
   concern. Base filters, suppression, priority mapping, and aliases should be
   implemented once.

## 5. Parser options

### 5.1 Facts about the Groovy front end

- Groovy's current parser is ANTLR4. The grammar and `AstBuilder` live in
  [`src/antlr`](https://github.com/apache/groovy/tree/master/src/antlr); the
  [Groovy architecture guide](https://github.com/apache/groovy/blob/master/ARCHITECTURE.md)
  describes parsing (CST) and conversion (AST) as separate phases.
- The parser grammar is large and left-recursive, with semantic predicates,
  custom error strategies, and a very large AST builder. A direct grammar port
  is not mechanical; the grammar is only part of the parser.
- Groovy is newline-sensitive, has GStrings, slashy/dollar-slashy strings,
  closures, command expressions, named arguments, safe navigation, spread
  operators, and many DSL-friendly forms. These are exactly the areas where
  parser shortcuts cause rule false positives/negatives.

### 5.2 Option A — fully self-developed parser

**Advantages**

- Full control over spans, trivia, error recovery, and AST shape.
- Can model CodeNarc's conversion-phase AST deliberately, including script
  wrappers and declaration nodes.
- Best foundation for flow-sensitive analysis, precise diagnostics, and future
  fixes/LSP.
- No C toolchain or external grammar dependency.
- Can be optimized for allocation behavior and single-pass construction.

**Disadvantages**

- Highest implementation cost and long-tail risk.
- Newline handling, GStrings, command chains, and error recovery are substantial.
- Requires a serious conformance corpus to avoid silent misparses.
- Parser bugs block every rule and every analysis layer.

### 5.3 Option B — tree-sitter-based parser

**Advantages**

- Fast path to broad syntax coverage with error tolerance.
- Incremental parsing and editor tooling for free.
- Active Groovy grammar exists
  ([`dekobon-tree-sitter-groovy` 0.6.0](https://crates.io/crates/dekobon-tree-sitter-groovy),
  Apache-2.0/MIT).
- Tree-sitter's error recovery is strong for malformed files.

**Disadvantages**

- The grammar's CST still needs lowering to a typed AST/semantic model.
- The current Groovy grammar has documented divergences relevant to linting.
  Its README explicitly says newlines are whitespace and lists cases where a
  line break does not end a statement, typed locals require initializers, and
  annotation/modifier ordering is restricted. These are not cosmetic for
  CodeNarc behavior.
- Significant newlines are hard in tree-sitter; fixing them may require grammar
  and external-scanner work.
- C dependency, external grammar lifecycle, and less control over error
  messages and semantic predicates.
- Incremental parsing is not a major benefit for batch CI runs.

### 5.4 Option C — reuse an existing Rust Groovy parser crate

**Advantages**

- Fastest route to a prototype.
- Some crates already parse substantial Groovy syntax.

**Disadvantages**

- The available crates were not designed for linting:
  - `groovy` 0.1.15 is an interpreter frontend; its AST records only a 1-based
    statement line (no full spans/columns), skips or ignores several
    annotations, and pulls in a VM/JIT stack. It is not suitable as the linter's
    semantic core.
  - `oak-groovy` 0.0.11 is early-stage, uses a nightly feature, and has minimal
    tests in the published crate.
- Likely insufficient error recovery, trivia fidelity, and AST stability.
- Coupling to a young dependency's API and release cadence.
- May require forking anyway to reach CodeNarc compatibility.

### 5.5 Comparison

| Criterion | Self-developed | tree-sitter | Existing parser |
| --- | --- | --- | --- |
| CodeNarc behavior control | High | Medium–low without grammar work | Low |
| Time to first working rule | Slow | Fast | Fastest |
| Full Groovy coverage | High effort, high control | Medium; depends on grammar | Uncertain |
| Newline-sensitive correctness | Full control | Known gap; needs work | Varies |
| Error recovery | Build it | Strong | Varies |
| Semantic AST for CFG | Direct | Requires lowering | Requires adaptation |
| Performance ceiling | Highest, tunable | Good; CST/lowering overhead | Unknown |
| Dependency risk | None | C grammar + external repo | High |
| LSP/incremental future | Build later | Good | Varies |

### 5.6 Recommendation

**Preliminary lean: self-developed lexer + parser, with a time-boxed spike before
committing.**

Reasoning:

- The compatibility target is CodeNarc behavior, not “parses most Groovy”. The
  newline-sensitive and AST-shape differences in the tree-sitter grammar are
  directly on the critical path for rule parity.
- Deep semantic analysis needs a semantic AST we control. A tree-sitter CST
  still requires a lowering layer, so the “buy versus build” saving is smaller
  than it first appears.
- Performance is not a reason to choose tree-sitter: with file-level
  parallelism, parser throughput is important but correctness dominates. A
  hand-written parser can be competitive and can be tuned with arenas and
  zero-copy spans.
- The `groovy`/`oak-groovy` crates do not meet the span, trivia, annotation, or
  stability requirements.

However, self-development is a multi-month commitment for production-grade
Groovy. The project has decided to proceed incrementally with a self-developed
subset rather than wait for a full bake-off. Keep the following spike criteria
as a sanity check, and use tree-sitter only as an external oracle:

1. Implement a lexer and a parser subset sufficient for the first rules,
   including newlines, closures, GStrings, and class/method declarations.
2. Build a tree-sitter probe that parses the same corpus and lowers only the
   nodes needed by those rules.
3. Run both against CodeNarc's test fixtures and a real-world Groovy corpus.
4. Measure: parse success rate, ERROR nodes, AST-query fidelity, throughput,
   and engineering effort.
5. Decision gate:
   - Continue self-developed if it parses the first rule corpus and a high
     percentage of the real-world corpus with acceptable effort.
   - If not, fork the tree-sitter grammar and fix its newline/AST behavior
     rather than using it unmodified.

Use tree-sitter as a **differential oracle** and temporary prototype, not as the
production dependency by default. Do not maintain two parsers in production.

## 6. CodeNarc compatibility contract

“Basic behavior alignment” should be made concrete:

1. **Rule identity**
   - Canonical CodeNarc name plus CodeNarc class-name alias.
   - Same default priority/severity mapping.
   - Unsupported rules are reported as unsupported, never silently skipped.

2. **Configuration**
   - Parse CodeNarc XML rulesets (`<rule>`, `<property>`, `<ruleset-ref>`).
   - Support the inherited base options: `enabled`, `priority`,
     `applyToClassNames`, `doNotApplyToClassNames`, file-name/path filters, and
     `violationMessage`.
   - Track the CodeNarc version used for compatibility tests.

3. **Suppression**
   - `@SuppressWarnings` on classes, methods, fields, declarations, and imports.
   - CodeNarc's comment-based disable plugin behavior where applicable.
   - Suppression must run after rule evaluation and before reporting.

4. **Diagnostics**
   - Match rule, file, and line first; columns and messages can be richer, but
     line mismatches break CI expectations.
   - Map priority 1/2/3 to error/warning/info while preserving the original
     priority in JSON output.

5. **Differential testing**
   - Pin CodeNarc 4.0.0.
   - Run both tools with identical rule sets and files.
   - Compare normalized findings: `rule`, path, line.
   - Vendor CodeNarc test fixtures/expected violations with Apache-2.0
     attribution.
   - Track a per-rule compatibility matrix. A rule is “compatible” only when its
     differential tests pass.
   - SonarQube's Groovy plugin uses CodeNarc, so CodeNarc parity is the right
     primary target; Sonar-specific rule profiles are a separate concern.

## 7. Benchmarking

Introduce benchmarks immediately after the first vertical slice, as planned.

### 7.1 Layers of measurement

- **Microbenchmarks** (Criterion or `iai-callgrind`): lexing, parsing, AST
  traversal, symbol building, CFG construction, individual rules.
- **End-to-end** (`hyperfine`): release binary over a corpus, with cold and warm
  file caches.
- **Comparison**: pinned CodeNarc 4.0.0, same JVM, same ruleset, same corpus,
  same findings.
- **Profiling**: `perf`/Instruments, allocation profiling, and per-phase timers
  behind `--timings`.

### 7.2 Fair CodeNarc comparison

- Use CodeNarc 4.0.0 and the same ruleset.
- Use a corpus large enough that JVM startup is not the whole story.
- Report cold and warm runs separately; do not claim JVM startup as a speedup.
- Warm up both tools, run multiple iterations, report median and spread.
- Validate identical findings before comparing speed.
- Record hardware, OS, JVM version, CodeNarc version, and thread count with every
  baseline.
- Keep benchmark corpora out of the main artifact; download with a pinned commit
  and checksum, and record licenses/attribution.

### 7.3 CI policy

- Start with local baselines and nightly benchmarks; do not block PRs on noisy
  wall-clock numbers.
- Use deterministic instruction-count benchmarks (`iai-callgrind`) for
  regression gates where possible.
- Add a dedicated performance runner only after the metrics stabilize.
- Track files/second, MB/second, peak RSS, and per-phase time.

## 8. Vertical slices

Implemented in the first slice:

- `EmptyIfStatement`, `EmptyElseBlock`, `EmptyWhileStatement`,
  `EmptyForStatement`, `EmptyTryBlock`, `EmptyCatchBlock`,
  `EmptyFinallyBlock` (AST rules).
- `UnnecessarySemicolon` (token rule).
- CodeNarc XML default loading, parallel CLI, text/JSON reporters, tests.

Remaining sequence:

1. **`EmptyIfStatement`** — AST-only CodeNarc rule. Proves parse → traversal →
   rule → diagnostic → JSON/text output → exit code.
2. **`UnnecessarySemicolon`** — token/line rule. Proves the token/source layer
   and CodeNarc's line-end semantics.
3. **`UnusedVariable`** — scope/symbol + deferred file-end rule. Proves L2 and
   deferred execution.
4. **`NullDereference`** (custom) — CFG + nullness. Proves L4 and the
   differentiator.

Acceptance for the first slice:

- CodeNarc differential test passes on its rule fixtures.
- Diagnostics match at least rule/file/line level.
- `--threads 1` and `--threads N` produce identical output.
- Release benchmark baseline is recorded.
- A single AST traversal serves the AST rule(s); no per-rule full walks.

Do not start with `UnusedVariable` or a null checker: they are valuable, but
they would force scope/CFG design before the basic pipeline and CodeNarc
contract are validated.

## 9. Crate responsibilities (provisional)

| Crate | Responsibility | Must not depend on |
| --- | --- | --- |
| `groovylint-core` | spans, source, diagnostics, line maps | syntax, rules |
| `groovylint-syntax` | lexer, tokens, AST, parser | rules, CLI |
| `groovylint-analysis` | scopes, facts, types, CFG, data-flow | rules, config, CLI |
| `groovylint-config` | config model, CodeNarc XML, benchmark options | parser |
| `groovylint-rules` | rule model, dispatch, registry, engine | CLI rendering |
| `groovylint-cli` | discovery, scheduling, reporters, exit codes | rule internals |

The key dependency direction remains: **syntax → analysis → rules → CLI**.
Analysis must not depend on rule configuration or rendering.

## 10. Decisions and remaining questions

Decided for the first slice:

1. **Parser**: self-developed, incremental subset (approved). Tree-sitter is
   retained only as a possible differential oracle; the existing `groovy` and
   `oak-groovy` crates are not used.
2. **Default configuration**: CodeNarc 4.0.0 `rulesets/basic.xml`; unimplemented
   rules are reported and skipped.
3. **First rules**: seven `basic` empty-block rules plus the token-level
   `UnnecessarySemicolon`.
4. **Rule interfaces**: separate capability traits (`TokenRule`, `AstRule`,
   `FileRule`) with per-file rule instances and one shared AST traversal.
5. **Parallelism**: parallelize files; diagnostics are sorted before reporting.

Still open:

1. **Real compatibility target**: provide the production CodeNarc ruleset and
   the CodeNarc version used by the Sonar Groovy plugin. Sonar may bundle a
   different CodeNarc version from the latest release.
2. **Groovy baseline**: Groovy 3, 4, or 5 syntax first? Which file types matter
   (Gradle scripts and plain Groovy tests)?
3. **CodeNarc scope**: XML only first, or also Groovy DSL/JSON rulesets? Which
   base-rule options and suppression forms are mandatory for v0?
4. **AST representation**: the current implementation uses a boxed typed AST
   plus a token stream. Revisit arenas/`u32` spans only after profiling.
5. **Nullness contract**: annotation-driven, inference-driven, default for
   unannotated types, intraprocedural vs interprocedural.
6. **Primary workload**: batch CI only, or editor/LSP as well? Incremental
   parsing matters much more for the latter.
7. **Benchmark gate**: which metric and corpus become the official baseline,
   and when (if ever) a performance regression blocks CI.

## References

- [CodeNarc repository](https://github.com/CodeNarc/CodeNarc)
- [CodeNarc 4.0.0 release](https://github.com/CodeNarc/CodeNarc/releases/tag/v4.0.0)
- [Apache Groovy architecture guide](https://github.com/apache/groovy/blob/master/ARCHITECTURE.md)
- [Apache Groovy parser grammar](https://github.com/apache/groovy/blob/master/src/antlr/GroovyParser.g4)
- [Ruff compilation pipeline](https://github.com/astral-sh/ruff/blob/main/CONTRIBUTING.md)
- [ESLint single-traversal linter](https://github.com/eslint/eslint/blob/main/lib/linter/linter.js)
- [Clippy lint passes](https://github.com/rust-lang/rust/blob/master/src/tools/clippy/book/src/development/lint_passes.md)
- [Oxc linter architecture](https://www.oxcjs.com/learn/architecture/linter)
- [Biome architecture](https://biomejs.dev/internals/architecture/)
- [tree-sitter Groovy crate](https://crates.io/crates/dekobon-tree-sitter-groovy)
- [groovy crate on crates.io](https://crates.io/crates/groovy)
- [oak-groovy crate on crates.io](https://crates.io/crates/oak-groovy)
