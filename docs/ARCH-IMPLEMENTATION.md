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
  -> FileFacts             one recursive AST collection per file
  -> Project index         modules, packages, classes, methods
  -> Dependency graph      imports + declared type references
  -> target matcher        typed target predicates -> node sets
  -> constraint evaluator  dependency/annotation/shape/composition checks
  -> diagnostics           existing Diagnostic + FileReport format
```

### 3.1 FileFacts

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
- **No FactPlan optimization.** Facts are collected unselectively; the
  `FactPlan` discussion remains a later optimization.
- **Target order is not used.** Removing first-match semantics is required for
  the multi-predicate model.
- **Annotation arguments are raw text.** No constant resolution or expression
  evaluation.
- **`extend` / `implement` are direct/written relationships.** No transitive
  supertype closure.
- **No package/class cycle check.** `acyclic` is module-only.
- **Facts collector is not yet fused into `Engine`.** The `arch` command performs
  one AST collection pass; the existing `check` engine is unchanged.

## 8. Tests

```bash
cargo test -p groovylint-arch
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

## 9. Next steps

1. Add a `FactPlan` and fuse fact collection into the existing AST traversal.
2. Add Java source facts or explicitly gate architecture rules to Groovy.
3. Add baseline and `arch validate` / `arch doctor`.
4. Add package-level cycle checks and slice support.
5. Add Gradle module/settings integration and a Gradle `arch` task.
6. Benchmark a 10k-class project and optimize resolution/index memory.
