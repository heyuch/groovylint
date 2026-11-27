# Architecture rules: working implementation

Status: **prototype implemented** (2026-11-27). The model and parser are in
[`crates/groovylint-arch`](../crates/groovylint-arch); the design vocabulary is
[`ARCH-RULE-MODEL.md`](ARCH-RULE-MODEL.md). This document describes what the
current runtime actually does, which rules are covered, and where the prototype
deliberately stops.

## 1. Usage

```bash
groovylint arch --config architecture.toml .
groovylint arch --config architecture.toml --format json .
```

The command parses every discovered Groovy file once, extracts file facts, builds
a project index and dependency graph, evaluates the configured rules, and prints
the existing diagnostic text or JSON format. Findings are errors unless the rule
sets `severity = "warn"`.

## 2. Configuration

Canonical TOML uses nested tables. Inline tables are accepted only when they fit
on one line.

```toml
version = 1

[project]
severity = "error"

[targets]
domain-module  = { type = "module", name = "*-domain" }
adapter-module = { type = "module", name = "*-adapter" }

[targets.domain-class]
type = "class"
in = "domain-module"

[targets.adapter-class]
type = "class"
in = "adapter-module"

[[rule]]
id = "adapter-must-not-reach-domain"
target = "adapter-class"
because = "adapter calls application, not domain"

[rule.must]
notDependOn = ["domain-class"]
```

`type` is the built-in target type. The current set is:

- `module`
- `package`
- `class`
- `method`

`field` is no longer a target. Fields remain reachable as class members through
`allFields` and `allMembers`.

`in` is structural containment only:

- package → module;
- class → module or package;
- method → class.

A class target contained by another class target is rejected at parse time.

## 3. Runtime pipeline

```text
source files
  -> parse
  -> one AST traversal
       ├─ existing CodeNarc AstRule dispatch
       └─ FactCollector (enter/exit)
  -> FileFacts
  -> Project index         modules, packages, classes, methods
  -> Dependency graph      imports + declared type references
  -> target matcher        typed target predicates -> node sets
  -> constraint evaluator  dependency/annotation/shape/composition checks
  -> diagnostics           existing Diagnostic + FileReport format
```

The architecture command uses `Engine::empty().lint_with_facts(...)`, so facts
and ordinary rules share the same `walk_script` call. `FactCollector` is a
first-class analysis participant, not a registered `AstRule`: it has no rule
name, severity, suppression, or configuration entry.

### 3.1 FactPlan and FileFacts

`FactPlan::from_config` derives the needed fact categories from all targets and
constraints:

- `IMPORTS`
- `CLASSES`
- `METHODS`
- `FIELDS`
- `ANNOTATIONS`
- `TYPE_REFS`
- `PARAMS`

Expensive categories (`IMPORTS`, `TYPE_REFS`, member details) are skipped when
no rule needs them. Class declarations/packages are kept for target matching and
diagnostics. `Engine::lint` does not collect facts at all;
`Engine::lint_with_facts` takes a `FactPlan` and returns `FileAnalysis`.

[`facts.rs`](../crates/groovylint-arch/src/facts.rs) extracts:

- package and imports;
- class-like declarations: FQN, kind, abstract, nested, annotations,
  `extends`, `implements`;
- methods: name, return type, parameters, annotations, modifiers;
- fields: name, type, annotations, modifiers;
- references: imports, extends/implements, field/parameter/return/local types,
  `new`, casts, annotations;
- source spans.

The collector is intentionally compact and does not retain the AST.

### 3.2 Modules

A source file belongs to the nearest ancestor directory containing
`build.gradle` or `build.gradle.kts`. A module id is its path relative to the
command root, joined with `:`. When no build file exists, the first directory
under the command root is used. This is sufficient for the prototype and for
the synthetic fixtures; Gradle settings metadata is a later refinement.

### 3.3 Dependency graph

Type names are resolved with this order:

1. already fully qualified project class;
2. explicit import or import alias;
3. same package;
4. wildcard import;
5. `java.lang` for capitalized simple names;
6. otherwise a symbolic name.

Unresolved names stay symbolic and still participate in package-prefix targets.
Imports are included as edges; declared type references are included as well.
`dependOnlyOn` ignores JDK/language runtime edges (`java.`, `javax.`,
`jakarta.`, `groovy.`, `org.codehaus.groovy.`, `kotlin.`); `notDependOn` still
sees them when a rule explicitly names one.

## 4. Target matching

Targets are predicates; a node may match several targets. There is no
first-match-wins classification in the runtime. This is deliberate:

- `domain-class` and `domain-service` can both match a service class;
- rules on both targets apply;
- the narrower target does not shadow the broader one.

Target predicate support:

| Type | Predicates |
|---|---|
| module | `name`, `nameMatches`, `path`, `anyOf`, `not` |
| package | `name`, `nameMatches`, `in`, `anyOf`, `not` |
| class | `name`, `nameMatches`, `in`, `anyOf`, `not`, `kind`, `abstract`, `nested`, `extends`, `implements`, `annotatedWith` |
| method | `name`, `nameMatches`, `in`, `anyOf`, `not`, `annotatedWith`, `modifiers`, `paramCount`, `rawReturnType` |

`kind` is the class/interface/trait/enum/annotation/record selector. It can be
combined with `type = "class"` without a key collision.

## 5. Constraint coverage

Implemented:

| Constraint | Behaviour |
|---|---|
| `dependOnlyOn` | every resolved edge from the target must match an allowed target |
| `notDependOn` | every matching edge is a violation |
| `visible` | target-side package visibility; sources outside the allowed targets are violations |
| `annotatedWith` / `notAnnotatedWith` | class/method annotation checks |
| `annotationArgument` | raw annotation argument contains the configured name/value |
| `extend` / `implement` | class supertype/interface checks |
| `exist` | target must be empty/non-empty |
| `allMembers` | class members must satisfy a member selector |
| `allMethods` / `allFields` | class methods/fields must satisfy their selectors |
| `rawReturnType` / `notRawReturnType` | method return type checks |
| `allParams` | method parameters must satisfy a parameter selector |
| `visibility` | method visibility check |
| `children` | module child modules must be in the allow-list |
| `counts` | module child counts against `>=n`, `<=n`, `*`, `+` |
| `acyclic` | module dependency cycles |

The `self:` / `any:` / `other:` dependency prefixes are supported. `none` is
accepted only by `visible`.

## 6. Diagnostics

Findings use the existing [`Diagnostic`](../crates/groovylint-core/src/diagnostic.rs)
and report format. Dependency findings point at the reference/import span;
class, method, and field findings point at the declaration. Target-level
findings (`counts`, `acyclic`, `children`) attach to the root because there is no
single source span.

## 7. Deliberate limits of this prototype

- **Groovy only.** Java sources are not parsed; mixed Java/Groovy projects will
  miss Java classes and references.
- **No call graph.** Method calls are not resolved to callees. Dependency rules
  see declared types, imports, `new`, casts, annotations, and type references.
- **Best-effort name resolution.** Static imports, nested class spelling, and
  ambiguous wildcard imports are simplified.
- **No baseline.** `severity` works; a baseline file and `arch lint-config` /
  `arch doctor` are not implemented yet.
- **No Gradle settings integration.** Module ownership comes from the nearest
  build file and command root.
- **FactPlan is coarse.** It selects fact categories, not individual fields or
  references. More precise pruning is a later optimization.
- **Target order is not used.** Removing first-match semantics is required for
  the multi-predicate model.
- **Annotation arguments are raw text.** No constant resolution or expression
  evaluation.
- **`extend` / `implement` are direct/written relationships.** No transitive
  supertype closure.
- **No package/class cycle check.** `acyclic` is module-only.
- **Edge collection is not deduplicated** (D3): the same reference seen through an import
  and through a type reference produces two findings on the same line. Fixing it changes
  the count, so it is tracked rather than silently patched.
- **Facts and CodeNarc rules are fused only when requested.** `Engine::lint`
  skips facts; `Engine::lint_with_facts` runs both in one traversal. The
  architecture CLI uses the latter with an empty rule set.

## 8. Field run: a real project

First run against a production monorepo: 24,481 Groovy files, 217 Gradle modules, one
config of 24 rule blocks covering 16 of the 22 rule intents the project already encodes
as 112 ArchUnit test methods.

| Run | Findings | Files | Time |
|---|---:|---:|---:|
| whole repository | 2,951 | 885 | 3m55s |
| the three module groups that have ArchUnit tests | 695 | 230 | 13.8s |
| same repository with `check` (CodeNarc rules) | 972 | — | 4.6s |

Five findings were verified by hand: four are real violations (`models` depending on
another module's domain model, `application` calling `infrastructure` directly, domain
code using `org.springframework.web` utilities, an enum missing `BaseEnum`), and one
class of finding was ours, not theirs (§8.1 D2). So the engine finds what it promises;
the defects below make the output hard to act on, and two of them inflate it.

### 8.1 Defects, in the order they should be fixed

Ordered by correctness first, completeness second, speed last: a wrong diagnostic
teaches the team to ignore the tool, so nothing else matters until the first three are
gone.

| # | Symptom | Cause | Class |
|---|---|---|---|
| D1 | every class- and method-level finding points at line 1 | the fact spans for class/method declarations are not carried into the diagnostic; only dependency findings (which use the reference span) have a real position | correctness |
| D2 | `annotatedWith = ["com.acme.ApiSecurity"]` never matches, so every annotated method is reported | `has_annotations` (`eval.rs`) compares against the **simple** name only, with no suffix match; an FQN silently matches nothing | correctness |
| D3 | 935 of 2,951 findings (32%) are exact duplicates: same rule, line and message | edges are collected once per reference occurrence and never deduplicated before evaluation | correctness |
| D4 | `name = "com.acme.**.service.impl.**"` does not match the package `…service.impl` itself, so its own classes are reported | `**` compiles to `.*`, which needs at least one character after the dot | correctness (silent) |
| D5 | 3m55s for 24.7k files against 4.6s for `check` | three stacked causes, see §8.3 | **fixed** |

D1 detail: 110 of 110, 80 of 80, 29 of 29, 7 of 7, 3 of 3 and 1 of 1 findings across six
class/method rules reported `line = 1`.

D4 detail: a package target is a predicate over package names, and the tree walk does not
visit the package that a class *declares* separately from its sub-packages — so a
trailing `.**` both excludes the package itself and reads as if it included it. Whatever
the fix, the pattern language must say which one it means, because "the package and
everything below it" is the common case.

### 8.3 D5: how the run got 34x faster

Nothing about the rules changed: the same config produces the same 2,951 findings, at
the same lines, in the same order. Three things were doing redundant work per edge or
per node:

1. **Every rule walked every edge.** Dependency rules asked "what does this node depend
   on" by scanning the whole edge list and filtering on the source. An edge index
   (`source node → edge positions`, one map per source granularity) turns that into one
   lookup per matched node, built lazily for the granularities a config actually uses.
2. **`target_nodes()` cloned its result set on every call**, and `in = <target>` calls it
   once per candidate node — so placing a class inside a module copied the set of every
   class in the project, per class. The cache now hands out `Rc<BTreeSet<String>>`, and a
   reference count bump replaced the copy.
3. **`package_module()` scanned every class to find a package's module**, and `self:`
   dependencies call it per edge. The mapping is built once when the project is built.

| Run | Before | After |
|---|---:|---:|
| whole repository (24.7k files, 2,951 findings) | 3m55s | **6.4s** |
| three module groups (230 files, 695 findings) | 13.8s | **1.4s** |

For reference, `check` over the same repository takes 4.6s, so the architecture pass now
costs about the parsing it shares with it plus 2s of rule evaluation.

### 8.2 Gaps found in the DSL, not the engine

These are absences in the model, not bugs in it. Each has a concrete rule from the
catalog that cannot be written today.

| # | Missing | The rule that needs it | Smallest addition |
|---|---|---|---|
| G1 | "another module's X" as a dependency part | "my own module may depend on its `domain`, no other module may" — the load-bearing rule of the whole lattice | `other:` prefix on `notDependOn` / `dependOnlyOn`, meaning "a node of this target that lives in a *different* module" |
| G2 | meta-annotation matching | HTTP entry points are found with `areMetaAnnotatedWith(RequestMapping)`, which covers `@GetMapping`, `@PostMapping`, … | `metaAnnotatedWith` selector, or an `annotationOn` indirection through the annotation's own declaration (needs the annotation class in the project index) |
| G3 | transitive supertypes | API classes extend `AbstractBaseAPI` through intermediate bases | `extends` currently compares written names; `assignableTo` needs the project's supertype closure, which the index already has for project classes |
| G4 | target-side exemption | a whitelist names two enum types that HTTP contracts may reference; `except` exempts *sources*, so writing targets there does nothing | allow `except` to name the forbidden target, or let `notDependOn` entries carry exceptions |

G1 is the one that decides how much of the catalog is expressible: without it the three
cross-module rules can only be approximated, and the approximation either over-reports
(the whole layer as forbidden) or under-reports (own module included).

## 9. Tests

```bash
cargo test -p groovylint-arch
cargo test -p groovylint-rules --test facts
cargo test -p groovylint --test arch
cargo test --workspace
```

The synthetic fixtures cover:

- forbidden dependency between an adapter and a domain module;
- target containment through module targets;
- annotation presence;
- `allFields` member constraints;
- module children and counts;
- CLI text/JSON reporting.

## 10. Next steps

Speed went first (§8.3) because a run measured in minutes cannot be iterated on. What
is left is correctness, then completeness:

1. ~~**Make it fast.**~~ D5 is done (§8.3): 34x, same findings. It went first because
   every later fix needs a run short enough to iterate on.
2. **Make the diagnostics land.** D1 (class/method spans), D2 (annotation names),
   D3 (duplicate edges). Without these three the tool produces a list nobody can act
   on, and every later feature is judged by that list.
3. **Make the pattern language honest.** D4, and the `x.**` question it raises: state
   whether `**` includes the node itself, implement it that way, and let
   `arch validate` reject a target that matches nothing.
4. **Add the one missing relation.** G1 (`other:`), which unblocks the three
   cross-module rules of the catalog.
5. **Close the small gaps.** G4 (`except` on targets), G2 (meta-annotation), G3
   (supertype closure) — each is a few lines once the model above is settled.
6. Add Java source facts or explicitly gate architecture rules to Groovy.
7. Add baseline and `arch validate` / `arch doctor`.
8. Add package-level cycle checks and slice support.
9. Add Gradle module/settings integration and a Gradle `arch` task.
