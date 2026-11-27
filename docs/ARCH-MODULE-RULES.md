# Module structure & dependency rules: design v2

Status: **superseded** by [`ARCH-RULE-MODEL.md`](ARCH-RULE-MODEL.md) (v4).

Kept because v2 is where the modelling problems were found and measured: the node /
kind / edge model, the pattern grammar, the ambiguity and validation rules, and the
first attempt at the target project's constraint mapping. v3 keeps the matcher and
the validation rules, and replaces the `allow` / `allowSelf` / `allowOther` matrix
with a target type × selector × constraint vocabulary, so read v3 first and come
back here only for the history.

## Original v2 design (history)

## 1. Review of the proposed model

The proposal (declared name patterns, names reusable as variables in rules) is
right, and better than v1's fixed vocabulary, for three reasons:

1. **It removes the last business assumption.** With kinds declared in config, a
   project that does not use DDD layers (frontend/backend, app/core/data) is
   expressible without touching the engine. v1's role table would have needed an
   alias for every such project.
2. **Patterns are the natural matcher.** The target project's role *is* already in
   the name (`acme-billing-domain`), and 19,464 of 19,465 main-source files agree
   between name and package root. A pattern over names is therefore an accurate
   description, not an approximation.
3. **One config, no per-module repetition.** 208 sub-modules × 2–3 assertions is
   today's status quo in hand-written ArchUnit tests; a kind table collapses it.

Three things the proposal leaves under-specified, and which must be decided or the
rules silently weaken:

| Gap | Why it matters | Decision in §4 |
|---|---|---|
| **Instance identity** — a pattern `{\w+}-domain` matches the domain of *every* module, so `depend_on: [domain]` cannot express "my own domain yes, another module's domain no". The conventions require exactly that distinction, and the corpus has 229 cross-module `domain→domain` edges. | This is the difference between a rule that catches boundary violations and one that forbids all layering. | Instance = the **containing module node**, not the regex capture. Rules say `allowSelf` / `allowOther` / `allow` explicitly (§4.3). |
| **Pattern semantics** — precedence, anchoring, and what happens when two kinds match one node (`x-shared-models` matches both `*-models` and `*-shared-models`). | Ambiguity here changes results between runs. | Anchored full match, `{slot}` = one path segment, declaration order wins, ambiguity reported by `arch lint-config` (§4.2). |
| **Validation** — a typo in a rule's kind name (`depend_on: [domian]`) must not silently disable the rule. | A weak rule is worse than a missing rule: it looks enforced. | Config load fails on unknown kind names; unused kinds and unconstrained kinds are warnings (§5.4). |

Also worth stating: `depend_on` is an *allow* list, so it needs a documented
meaning for kinds that are not listed (forbidden), for itself, for test sources,
and for imports. Those are §6.

## 2. Measured shape of the target project

Aggregates only. Module names, per-module counts and the constraint-by-constraint
mapping live in the local, unversioned note under `docs/local/`.

| Fact | Value |
|---|---:|
| Top-level Gradle modules | 41 |
| — with sub-modules | 32 |
| — single modules, no sub-module (`base-*`, `boot-app`, `gateway-app`) | 12 |
| Sub-modules | 208, at depth 2 or 3 (`a:b:b-domain`: 6 cases) |
| Role suffixes in sub-module names | `domain` 30, `infrastructure` 29, `application` 29, `client` 28, `adapter` 28, `models` 17, `metadata-context` 10, `shared-models` 7, `dsl` 3, `model`/`core`/`kit`/`common`/`share-models` 1 each |
| Main-source files whose name-derived role matches the package root | 19,464 |
| … mismatch | 1 |
| Hand-written ArchUnit tests re-stating 2–3 dependency assertions each | 11 files |

Cross-module edges by (from suffix → to suffix), main sources:

| Pair | Edges | Pair | Edges |
|---|---:|---|---:|
| `domain → models` | 2,628 | `adapter → models` | 543 |
| `application → models` | 2,024 | `domain → client` | 478 |
| `infrastructure → models` | 1,189 | `client → models` | 280 |
| `models → models` | 1,075 | **`domain → domain`** | **229** |
| `application → client` | 1,047 | **`application → domain`** | **161** |
| | | Edges into an internal sub-module of another business module | **506** |

Consequences for the design: patterns must be depth-agnostic, single modules must
be expressible without sub-modules, and a baseline is mandatory at adoption.

## 3. Model

```
Node        a location: a Gradle project path, a package, or an external package
  ├── kind      the user-declared name used by rules          "domain"
  ├── instance  the nearest enclosing node of a declared kind  acme-billing
  └── files     classes whose file lives under the node
Edge        (from node, to symbol, kind)
  ├── kind      TypeReference | Import | DeclaredDependency
  └── resolved  the target node, or None for external/unresolved symbols
```

- A **node** carries no business meaning of its own. Two nodes with the same kind in
  different modules are different nodes; that is what `self`/`other` talk about.
- **Containment is structural**: the parent of a node is the Gradle path minus its
  last segment — not the pattern capture. `acme-mgmt:acme-adapter` has parent
  `acme-mgmt` although its name captures `acme`; identity must not depend on a
  regex group.
- **External nodes** exist for packages nobody in the project declares
  (`org.springframework.web.**`). They have no files, no instance, and only ever
  participate as targets of `deny` lists — that is how "domain must not touch Web,
  MyBatis or Servlet" is expressed without a second rule mechanism.

## 4. The DSL

### 4.1 Nodes and kinds

```toml
version = 1

[project]
sourceSets = ["main"]
unknownKind = "warn"          # warn | error       (what to do with unmatched locations)

# One kind per location shape. `path` matches the Gradle path, `name` matches its
# last segment, `external` matches a package that the project does not declare.
[kinds.module]
path = "{module}"

[kinds.nested-module]
path = "{tree}:{module}"

[kinds.adapter]
name = "*-adapter"

[kinds.application]
name = "*-application"

[kinds.domain]
name = "*-domain"

[kinds.infrastructure]
name = "*-infrastructure"

[kinds.client]
name = "*-client"

[kinds.models]
name = "*-models"
nameAliases = ["*-model", "*-shared-models", "*-share-models", "*-metadata-context"]

[kinds.shared]                 # libraries every module may use
path = "component-*"
pathAliases = ["buildSrc", "boot-app", "gateway-app"]

[kinds.spring-web]
external = "org.springframework.web.**"

[kinds.persistence]
external = ["org.apache.ibatis.**", "javax.persistence.**", "jakarta.persistence.**"]
```

Rules of the matcher (§4.2 has the precise grammar):

- a location matches a kind when **all** of its given selectors match;
- kinds are tried in declaration order, **first match wins**;
- a location matched by two kinds is reported by `arch lint-config` as ambiguous,
  so `*-models` vs `*-shared-models` is a visible decision, not a coin flip;
- a location matched by no kind is `unknown`, reported per §5.4.

### 4.2 Pattern grammar

| Token | Matches |
|---|---|
| `*` | any run of characters except `:` |
| `**` | any run of characters, including `:` |
| `{slot}` | one run of characters except `:`; usable for diagnostics and for rules that need the captured text |
| literal | itself |

Patterns are anchored (full match). `name` and `path` follow the same grammar; a
`name` pattern never sees a `:`. Optional escape hatch for projects with hostile
naming: `regex = "..."` on a kind, with the same anchoring rule.

### 4.3 Dependency rules

```toml
[[rule]]
id = "layer-lattice"
from = "domain"
because = "02: domain 不依赖 application/adapter/infrastructure，也不依赖其他模块的 domain"

# `allow`       any instance (external nodes, shared libraries, the module itself)
# `allowSelf`   same instance only   (`domain` of my own module)
# `allowOther`  another instance     (`client` of the module that provides it)
# `deny`        checked first, always wins
allow      = ["models", "shared"]
allowSelf  = ["domain"]
allowOther = ["client"]
deny       = ["spring-web", "persistence"]
```

`allow*`/`deny` lists are the *complete* statement about that source kind: a target
kind that appears in none of them is forbidden. That is what makes the lattice
closed — a newly declared kind cannot start depending on `domain` without a config
change.

Four more rules reproduce the conventions, all with the same shape:

```toml
[[rule]]
id = "adapter-only-reaches-application"
from = "adapter"
allow      = ["models", "shared"]
allowSelf  = ["application"]
because = "02: adapter 只调用 application"

[[rule]]
id = "cross-module-only-through-client"
from = ["adapter", "application", "domain", "infrastructure", "models"]
allowOther = ["client", "models"]
because = "02: 模块外调用方只能显式依赖提供方 xxx-client"

[[rule]]
id = "no-internal-exposure"
from = "*"
allowOther = ["client", "models", "shared"]
because = "02: 不得依赖其他模块的 application/domain/infrastructure"
```

### 4.4 Containment rules — "which sub-modules may a module have"

The sketch's `root { submodule* }` becomes a rule over containment:

```toml
[[rule]]
id = "module-composition"
parent = ["module", "nested-module"]
children = ["adapter", "application", "domain", "infrastructure", "client", "models"]
counts   = { adapter = "<=1", client = "<=1", domain = ">=1" }
required = { adapter = ["application"] }      # an adapter implies an application
because = "02: 模块组合只有 adapter/application/domain/infrastructure/client/models"
```

`children` is an allow list too: a sub-module whose kind is not listed (including
`unknown`) is a violation, which is the "模块里能有哪些子模块" check enforced rather
than documented. `counts` covers `*` (zero or more), `+` (one or more) and `<=n`.

### 4.5 Exceptions: node-level and self-kind

Kinds are coarse. Two real constraints cannot be written with kinds alone:

- **one specific node may do anything** — the application module that wires every
  other module together (`boot-app`, `gateway-app`);
- **one specific node may be reached by everyone** — `shared-core:shared-core-models`,
  which every business module reads.

So an allow entry is either a kind or a kind plus a node pattern, and a rule may
exempt source nodes:

```toml
allowOther = ["client", "shared-models",
              { kind = "models", node = "shared-core:shared-core-models" }]

[[rule]]
id = "cross-module-only-through-client"
from = ["adapter", "application", "domain", "infrastructure", "models"]
allowOther = ["client", "shared-models", { kind = "models", node = "shared-core:*-models" }]
exemptFrom = ["boot-app", "gateway-app"]              # these may depend on everything
```

`exemptFrom` is a source-side pattern list; an explicitly unconstrained kind is
written `allow = ["*"]` so that `lint-config` does not report it as dead config.

**Self-kind default.** A class depending on another class *of its own kind in its
own module* is never a boundary violation (a service calling a sibling service).
`allowSelf` therefore always includes the source kind; `allowSelfKind = false`
turns that off for a genuinely layered module. Without this default the four
constraints below produce 650 false positives on the corpus.

### 4.6 Agreement rules (cheap, optional)

```toml
[[rule]]
id = "kind-matches-package-root"
check = "package-root"        # the class's package root must expose the node's kind
because = "02: 包结构表达分层，类不能落在名实不符的子模块里"

[[rule]]
id = "declared-dependency-matches-observed"
check = "declared-dependency" # build.gradle `project('…')` vs type references
because = "02: 模块依赖必须在 build.gradle 显式声明，不得通过传递依赖取得类型"
```

Measured today: 1 violation for `package-root`; `declared-dependency` needs a
Gradle reader and is phase 5.

### 4.7 The proposal's block syntax as a facade

The Groovy-looking blocks in the proposal are a good *surface* for the Gradle
plugin; they must not become a second parser. The facade lowers to the same kind
table and the same rule records:

```groovy
arch {
  kinds {
    domain { name '*-domain'; dependsOn models: any, client: other, domain: self }
    adapter { name '*-adapter'; dependsOn application: self, models: any }
  }
}
```

`groovylint arch --print-config` prints the TOML a facade produced, so the two
surfaces cannot drift.

### 4.8 Worked example: four constraints from a real project

The four constraints below were given as an acceptance test for the DSL. All four
are expressible; three of them need the features added above (§4.5) and the first
needs the kind aliases.

```toml
# 1. A module may only contain these sub-module kinds; shared-models is optional.
[[rule]]
id = "module-composition"
parent   = ["module", "nested-module"]
children = ["adapter", "application", "domain", "infrastructure", "infra",
            "client", "models", "model", "shared-models"]
counts   = { adapter = "<=1", application = "<=1", domain = ">=1", client = "<=1", models = "<=1", "shared-models" = "<=1" }
except   = ["component-*"]          # the 9 single-module libraries declare no children

# 2. Inside one module.
[[rule]]
id = "adapter-intra"
from = "adapter"
allowSelf = ["application", "models"]

[[rule]]
id = "application-intra"
from = "application"
allowSelf = ["domain", "models"]

# 3. Across modules: application reaches another module only through its client.
[[rule]]
id = "cross-module-only-through-client"
from = "application"
allowOther = ["client", "shared-models"]

# 4a. The wiring module may depend on everything.
[[rule]]
id = "entry-unconstrained"
from = "entry"                      # kind: path = "boot-app" / "gateway-app"
allow = ["*"]

# 4b. One shared model module is readable from everywhere.
[[rule]]
id = "cross-module-only-through-client"
from = "application"
allowOther = ["client", "shared-models",
              { kind = "models", node = "shared-core:shared-core-models" }]
```

Day-0 findings of exactly these four rules (main sources, imports resolved through
the derived node map; type-reference extraction will move the numbers, not the
ranking):

| Constraint | What it reports today | Reading |
|---|---|---|
| 1 composition | **22** sub-modules outside the list: `-metadata-context` 11, `-dsl` 4, `-model` 2, `-mgmt` 2, `-core`, `-kit`, `-common` 1 each | Either add these kinds to the list, or the 22 are real violations. Nothing else can express "legacy sub-module, keep it". |
| 2 `adapter` intra | `adapter → infrastructure` 21, `→ domain` 20, `→ client` 3 (`→ model` 10 disappear once `model` aliases to `models`) | Real debt: adapter bypassing application. |
| 2 `application` intra | `application → infrastructure` **254**, `→ client` 185 | `→ infrastructure` is real debt. **`→ client` is likely legitimate**: 02 puts the Client *implementation* in `-application`, so it implements the module's own `-client` contract. Needs `allowSelf` to include `client`. |
| 3 cross-module | `application → models` **2,024**, `→ domain` 161, `→ model` 3 | As written, this rule alone produces ~2,188 violations: business modules read each other's `-models` directly. Either `models` joins `allowOther` (recommended) or the baseline carries 2k entries. |
| 4a `entry` | unconstrained by construction | — |
| 4b shared model exception | the exempted node is the most reached target in the project, so the exception is load-bearing, not cosmetic | Without it the rule cannot run at all. |

Two constraints need a decision before they are written into config:

- **`application → client` inside the same module** (185 edges): 02 requires the
  Client implementation to live in `-application`, so this direction must be
  allowed; the constraint as stated forbids it.
- **`domain → client` across modules** (478 edges): 02 allows calling a client from
  Domain, the constraint only names `application`. Encoding the constraint as
  written would flag 478 legitimate calls.

## 5. Executor

```
groovylint arch --config architecture.toml [paths]
    [--baseline arch-baseline.txt] [--format text|json|sarif]
    [--print-layout] [--print-graph] [--verbose]
groovylint arch lint-config --config architecture.toml     # static check only
```

1. **Locate** — read `settings.gradle` (else scan directories) → the node tree:
   every Gradle path, its parent, its kind, its files.
2. **Parse** — existing parser, parallel; `check` and `arch` share discovery and
   parsing, `arch` reuses the ASTs instead of re-reading them.
3. **Classify** — attach `(node, kind, instance, sourceSet)` to every class.
4. **Extract edges** — declared types, `extends`/`implements`, `new`, casts, static
   access, annotations; imports recorded with `EdgeKind::Import`; unresolved symbols
   kept symbolic and matched against `external` kinds.
5. **Evaluate** — kind-level rules fold the edge set into
   `(from kind, from node, to kind, to node)` counts; containment and agreement rules
   read facts only.
6. **Report** — existing `Diagnostic` shape with `rule = <id>`, `because` appended to
   the message, one diagnostic per violating pair plus the first location and an
   `occurrences` count (`--verbose` expands to per-edge); sorted by
   `(file, line, rule, id)` for determinism; baseline filtering; exit code 1 on
   violations at or above the configured severity.

### 5.1 Validation (`arch lint-config`)

Fails or warns without parsing the project:

| Check | Severity |
|---|---|
| rule references an undeclared kind | error |
| `parent`/`children`/`allow*` contain a kind that can never be a child/target of that direction | error |
| two kinds match the same location | error, listing location and kinds |
| kind matched by no location (dead pattern, likely a typo) | warn |
| kind with no rule constraining it (a rule with `allow = ["*"]` silences it) | warn |
| cycle in the kind-level allow graph | warn (may be intentional) |
| external kind that is also declared by the project | warn |

### 5.2 Folding and noise

2,628 `domain → models` edges must not become 2,628 diagnostics. One diagnostic per
`(rule, from node, to node)` keeps the report readable; the message carries the
count and the pattern names, `--verbose` prints every edge with file and line.

### 5.3 Baseline

`arch-baseline.txt`, one line per accepted violation:
`<rule-id>\t<from node>\t<to node>\t<target kind>`. `--update-baseline` rewrites it;
entries that no longer violate are pruned with a warning, so debt cannot be frozen
silently forever. A committed baseline is what allows adoption on a repo whose
day-0 state has hundreds of violations (§2).

### 5.4 Determinism and performance

Discovery from a sorted list; edges as a set; diagnostics sorted; the same output
under `--threads 1` and `--threads 8`. `check` parses 24,345 files in 3.7 s;
`arch` adds one index pass and a fold over distinct node pairs, so it should stay
in the same order of magnitude. Unknown kinds and unmatched nodes are reported
once per location, not per rule.

## 6. Semantics that must be decided explicitly

| Question | Proposal | Why |
|---|---|---|
| Do imports count as dependencies? | Edges from **type references**; imports recorded separately with `imports = true` per rule. Default: excluded. | An unused import would otherwise create a boundary violation the compiler never produces. |
| Test sources | `sourceSets = ["main"]` default; per rule override. | Test code legitimately reaches into other modules' internals. |
| Transitive edges | Not followed in v1; documented. | Transitive closure over 24k files is expensive and the conventions forbid relying on it anyway. |
| Unresolved symbols | Kept symbolic; matched only against `external` kinds. | A JDK type has no node. |
| Self-node edges (class → class in the same node) | Dropped before evaluation. | Always allowed by construction. |
| Nested module trees (`a:b`) | Instance = nearest enclosing node of a `parent` kind; the tree is reachable through the parent chain, so "same tree" rules are expressible later without a new concept. | 506 edges cross this boundary; the model must not guess. |
| Kinds for classes/packages inside a sub-module (`*Repository`, `*Application`) | Out of v1; the selector already allows it later. | Call it a second phase of the same engine rather than a second engine. |

## 7. Testing

1. **Fixture project** `tests/fixtures/arch/project-a`: 5 modules (one nested, one
   single, one shared library), all kinds, one violation per rule kind, one
   ambiguous location, one unknown location, an unused import, a test source.
   Asserts exact diagnostics — this fixture *is* the DSL specification.
2. **Corpus golden**: the §2 counts as a snapshot, so extraction changes that alter
   edge semantics fail loudly.
3. **ArchUnit parity**: the 11 existing `*ArchTest` dependency assertions rewritten
   as one config, both tools run, `(file, line, rule)` diffed — the method used for
   the CodeNarc comparison.
4. **Determinism**: identical output at 1 and 8 threads.

## 8. Phasing

| Step | Content | Effort |
|---|---|---|
| 1 | Nodes + kinds + matcher + `--print-layout` (shows each Gradle path, its kind, its instance, unknown/ambiguous locations) | 1 d |
| 2 | `ProjectIndex` + edge extraction + `--print-graph` (kind-pair counts, debuggable without rules) | 1 d |
| 3 | Dependency rules (`allow`/`allowSelf`/`allowOther`/`deny`) + folding + `lint-config` | 1 d |
| 4 | Containment rules (`children`, `counts`, `required`) + agreement rule | 0.5 d |
| 5 | Baseline, fixtures, corpus golden, ArchUnit parity | 1 d |
| 6 | Gradle facade lowering to the same model + `declared-dependency` check | 1 d |

Steps 1–2 are useful on their own: `--print-layout` answers "did we name and nest
the sub-modules the way the conventions say", which is half of the original request.

## 9. Open questions

1. **Pattern dialect**: `{slot}`/`*`/`**` as above, with `regex` as an escape
   hatch — or raw regex everywhere? Globs keep the common case readable and
   reviewable; regex is available where needed.
2. **Rule shape**: separate `allow`/`allowSelf`/`allowOther`/`deny` keys (proposed,
   explicit) versus `"kind:instance"` strings (`allow = ["domain:self"]`, shorter
   but easy to misread). Which reads better to the people who will maintain it?
3. **Instance boundary**: nearest enclosing `module`/`nested-module` node (proposed)
   — is there a rule that must reach further, e.g. "same top-level tree"
   (`tree-a` vs `tree-b`)?
4. **Strictness at adoption**: `unknownKind = "warn"` on day 0 (12 single modules +
   6 nested modules must be declared, and a missed pattern would otherwise fail the
   build) — or `"error"` from the start with the full kind table committed?
5. **Scope of v1 kinds**: Gradle sub-modules + external packages only, or also
   in-project packages/classes (`*Repository`)? The latter would let the same engine
   cover "client contracts must not depend on Spring" at package granularity.
6. **Facade**: is the Groovy block syntax the Gradle-plugin surface (lowering to
   TOML), or should TOML be the only supported format?
7. **Self-kind default**: `allowSelf` implicitly includes the source kind (proposed,
   avoids 650 false positives) — or must every rule state it explicitly?
8. **Legacy sub-modules**: are `-metadata-context`, `-dsl`, `-model`, `-mgmt`,
   `-core`, `-kit`, `-common` declared as kinds (allowed by the composition rule) or
   accepted as 22 baseline entries?
9. **Acyclicity authority** (§11.1 C18): the observed module graph has one SCC with
   22 of 32 top-level modules. Is the rule applied to the *declared* Gradle graph
   (`build.gradle` + `--no-api`), or to the observed one with a cycle baseline? No
   other decision in this design changes the day-0 result by that much.


## 11. Appendix: mapping a project's architecture document

Turning a project's own architecture rules into this config is mechanical:

1. list the sub-module types the conventions allow → one kind per type, ordered
   narrowest pattern first, aliases for the historic suffixes;
2. for each layer, read what it "负责 / 不负责" and write the `allow` /
   `allowSelf` / `allowOther` / `deny` lists — an allow list is complete, so a
   type that is not listed is forbidden;
3. the conventions' "只能通过 X 调用" sentence is usually one cross-module rule
   with `allowOther = ["x"]` for the calling kinds;
4. anything phrased as "更基础的" (a partial order), "无环" (a graph property) or
   "必须在 build.gradle 声明" needs the features in §11.1 below, not a new rule
   family;
5. every remaining sentence either belongs to another family (naming, model
   placement, transaction/permission, call graph) or becomes baseline data.

Run `arch --print-layout` and `arch --print-graph` before writing any rule: they
show the real node tree and the real edge counts, which is what the `allow`/`deny`
lists have to be written against.

### 11.1 Two features such a mapping can expose

**`tier` — an order among siblings.** "更基础的模型只能被更上层的依赖" is a partial
order, which kinds cannot express:

```toml
[kinds]
models       = { name = "*-models", tiers = { "*" = 10 } }        # lower = more basic
public-model = { path = "...", tiers = { "*" = 0 } }
```

```toml
[[rule]]
id = "models-depend-downward-only"
from = ["models"]
allowSelf = ["shared-models"]
allow = ["base", "public-model"]
allowLowerTiersOnly = true
```

**`acyclic` — a graph property, not an edge rule.** Every other rule is local
(source kind × target kind); "no cycles" is global:

```toml
[[rule]]
id = "module-graph-is-acyclic"
check = "acyclic"
scope = "module"                                 # fold edges to the module graph first
source = "declared"                              # or "observed"
ignore = [{ from = "models", to = "client" }]    # a known, harmless cycle
```

The check folds edges to the node graph, runs Tarjan, and reports each cycle once
with its members and the shortest participating edge.

A project's own measurements, the constraint-by-constraint mapping (with real
violation counts) and the decisions still open live in a local, unversioned note
under `docs/local/`: they name internal modules and quote internal documents, so
they must not be committed.

## 12. Design checklist

- **SRP**: `NodeTree` (locations, kinds, containment, matching), `ProjectIndex`
  (facts + edges), `ArchEvaluator` (rule kinds), `Baseline` (debt). Matching never
  appears in the evaluator and evaluation never parses.
- **GRASP Information Expert**: the tree answers "which node and kind is this file";
  the index answers "what does this class reach"; rules only read those two.
  **Protected Variations**: the kind table is config data — a new layout, a renamed
  module or a new sub-module suffix is a config change, not a code change.
- **Pattern usage — five questions**: four rule kinds over one node/edge model do
  not justify a trait per kind plus a registry; one `enum RuleKind` and a match keep
  parsing, validation and diagnostics on a single path. It simplifies the config
  mapping, complicates nothing (no object lifetimes, no registry), and the extension
  point (a new kind of check) stays an explicit enum variant with config validation.
- **Reuse**: discovery, parser, `Diagnostic`, formatters and the thread pool are
  shared with `check`; new code is the node tree, the index and the evaluator.
