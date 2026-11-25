# Module structure & dependency rules: design

Status: **design for review** (nothing implemented).

Scope: the first family of the architecture catalog — **which sub-modules a module
may contain** and **which dependencies may run between them**. Annotation
presence, type shape and call-graph rules are phase A/C of
[`ARCHITECTURE-RULES.md`](./ARCHITECTURE-RULES.md); this document only fixes the
project model, the rule kinds and the executor they need.

The model is derived from what `saas-service` actually is, because a design that
does not fit 41 modules × 208 sub-modules is useless there.

## 1. Measured shape of the target project

| Fact | Value |
|---|---:|
| Top-level Gradle modules (`include 'x'`) | 41 |
| Nested sub-modules (`include 'x:y'`) | 208 |
| Module depth | 1 or 2 (`gi-policy:gi-policy-object-mgmt:…`) |
| Sources: `main` files whose sub-module role and package root agree | 19,464 |
| … disagree | 1 |
| Existing per-sub-module ArchUnit tests | 11 files, each re-stating the same 2–3 dependency assertions |

Role vocabulary found in sub-module names (last dash segment):

| Role | Count | Role | Count |
|---|---:|---|---:|
| `domain` | 30 | `models` | 17 |
| `infrastructure` | 29 | `metadata-context` | 10 |
| `application` | 29 | `shared-models` | 7 |
| `client` | 28 | `dsl` | 3 |
| `adapter` | 28 | `model`, `core`, `kit`, `common`, `share-models` | 1 each |

Three consequences drive the design:

1. **The role is a property of the sub-module name, not of the package.** The 11
   ArchUnit tests hard-code `com.bytesforce.domain.fileprocessor..`; the same fact
   is already in the path (`file-processor/file-processor-domain`). Everything can
   be derived from the Gradle path, so no per-module configuration is needed.
2. **Role derivation cannot be "strip the module prefix".** `tenant-mgmt:tenant-adapter`
   has module `tenant-mgmt` and stem `tenant`; the role is the last dash segment.
   `metadata-context` and `share-models` show that the vocabulary is not closed, so
   unknown roles must be a first-class outcome (and a rule).
3. **The rules repeat.** 208 sub-modules × 2–3 assertions, hand-copied per module,
   is the status quo. One lattice declaration replaces all of it — that is the
   value proposition, not a nicer syntax.

## 2. Project model

```
Project
├── Module            Gradle path minus the last segment   file-processor
│   ├── Submodule     last segment of the Gradle path      file-processor-domain
│   │   │             → role = last dash segment           domain
│   │   └── Class     one .groovy file                     GiPolicyLimitDed
│   └── …
└── …
```

Five entities and two edge kinds:

| Entity | Identity | Comes from |
|---|---|---|
| `Project` | root dir | CLI argument |
| `Module` | `moduleDepth` segments of the Gradle path, or a dir path | `settings.gradle`, else directory scan |
| `Submodule` | full Gradle path | same |
| `Role` | vocabulary entry, or `unknown` | last dash segment + `roleAliases` |
| `Class` | fully-qualified name | parsed source |
| `Edge` | `(from class, to symbol, kind)` | declared types, `extends`/`implements`, `new`, static access, annotations, and **separately** the Gradle `project(':x')` dependency |

Two edge kinds keep two different questions apart:

- **observed** edges come from source references. They answer "does this module
  reach into that one", which is what the architecture rules forbid.
- **declared** edges come from `build.gradle` `project('…')` lines. They answer
  "did the build file agree", a different rule that catches the two failure modes
  the conventions call out: a type used without a declared dependency (comes in
  through a transitive one) and a declared dependency nobody uses.

## 3. Deriving the structure

```toml
[project]
moduleDepth = -1                  # -1 = parent path of the include, 1 = first segment
moduleNames = "settings.gradle"   # fallback: directory scan of */ and */*/
sourceSets = ["main"]
sharedModules = ["component-*", "buildSrc", "api-gateway", "insmate-main"]

[roles]                            # vocabulary + aliases
domain = ["domain"]
application = ["application"]
adapter = ["adapter"]
infrastructure = ["infrastructure", "infra"]
client = ["client"]
models = ["models", "model", "share-models", "shared-models", "metadata-context"]
dsl = ["dsl"]
```

- Unknown sub-module role → `Role::Unknown`, and the `composition` rule reports it.
  That is the "模块里能有哪些子模块" check, enforced instead of documented.
- `sharedModules` are exempt from cross-module rules (they are libraries by design);
  the pattern is a glob over the Gradle path.
- `roleAliases` are data, not code: a new sub-module suffix is a one-line config
  change, which is the extension point that keeps the engine closed for
  modification.

## 4. What the index stores per class

Only what structural rules need; everything else stays in the AST:

```
ClassFact {
    fqn, module, submodule, role, sourceSet, file, line
    package, supertypes, annotations
    references: Vec<SymbolRef>          # resolved → ClassFact, or symbolic (external)
}
SymbolRef { name, kind, line, origin: Own | Import | Declared | External }
```

`origin` is what makes the import question answerable per rule instead of globally
(see §6): a reference that only exists as an unused import can be excluded without
re-parsing.

Resolution is **project-local by construction**: a symbol is resolved if its
fully-qualified form appears in the index (we know every class we parsed) or can be
derived from an import plus a known package. Anything else stays symbolic — a JDK
or third-party type has no module, so it only matters to package-pattern rules
(§5c), never to module rules.

## 5. Rule kinds (v1)

Four kinds cover the whole structural family. Each is a config record, not code:

### a. `composition` — what a module may contain

```toml
[[rules]]
id = "modules-use-known-roles-only"
kind = "composition"
modules = "*"
allowedRoles = ["adapter", "application", "domain", "infrastructure", "client", "models", "dsl"]
because = "02: 模块组合只有 adapter/application/domain/infrastructure/client/models"

[[rules]]
id = "an-adapter-needs-an-application"
kind = "composition"
modules = "*"
roleImplies = { adapter = ["application"], adapter_shared = ["application"] }
because = "02: adapter 只调用 application"
```

Also `requiredRoles`, `forbidRoles`, and `except = ["module-pattern"]`.
This is the only kind that reasons about *presence*, so it is also where "legacy
module has no `-client`" style exemptions live.

### b. `dependency`, scope `intra-module` — the layer lattice inside one module

```toml
[[rules]]
id = "module-layering"
kind = "dependency"
scope = "intra-module"
mode = "allow"
from = { role = "adapter" }
allow = [{ role = "application" }, { role = "models" }]
because = "02: adapter → application → domain；adapter 不碰 domain/infrastructure"
```

One rule per source role, five rules in total for the whole project (today: ~30
hand-written ArchUnit methods). `mode = "deny"` mirrors the catalog's
`noClasses()…dependOnClassesThat(forbidden)` phrasing for reviewers who prefer it;
`allow` is the default because "everything not listed is forbidden" is what makes
the lattice complete — a *new* role cannot sneak in silently.

### c. `dependency`, scope `cross-module` — boundaries between modules

```toml
[[rules]]
id = "cross-module-only-through-client"
kind = "dependency"
scope = "cross-module"
mode = "allow"
from = { role = "*" }
allow = [{ role = "client" }]
because = "02: 模块外调用方只能显式依赖提供方 xxx-client"

[[rules]]
id = "domain-never-crosses-modules"
kind = "dependency"
scope = "cross-module"
mode = "deny"
from = { role = "domain" }
to = { role = "domain" }
because = "02: domain 不能依赖其他业务模块的 domain"
```

`scope` values: `intra-module` (same module), `cross-module` (different modules),
`any`, and `same-role-tree` for nested modules
(`object-mgmt:shared-object-mgmt` being a *sub-tree* of `object-mgmt` — the
measured data has 506 such edges, so this needs an explicit answer, not a default).

### d. `packagePolicy` — technology leaks, same edge engine

The dependency rules above need roles; the catalog's "domain must not depend on
Web/MyBatis/Servlet" and "client contracts must not depend on Spring" are the same
extraction with a package matcher instead of a module matcher:

```toml
[[rules]]
id = "domain-stays-out-of-frameworks"
kind = "packagePolicy"
from = { role = "domain" }
denyPackages = ["org.springframework.web..", "org.apache.ibatis..", "jakarta.servlet.."]
```

### e. `agreement` — structure against reality (optional, cheap)

```toml
[[rules]]
id = "submodule-role-matches-package-root"
kind = "agreement"
check = "role-vs-package"        # or "declared-vs-observed"
because = "02: 包结构表达分层，类不能落在角色不符的子模块里"
```

Measured today: 1 violation. Worth having because it is the rule that catches the
next `ClientImpl` dropped into a `-domain` sub-module, and because it costs one
comparison over facts we already hold.

## 6. Rule semantics that must be decided explicitly

| Question | Proposal | Why |
|---|---|---|
| Do imports count as dependencies? | Edges come from **type references**; imports are recorded with `origin = Import` and rules may set `includeImports = true`. Default: excluded. | An unused import (`import` nothing references) would otherwise report a boundary violation that the compiler never creates. The catalog wants imports counted in `*MustNotDependOnMyBatis`; that is one flag, not a different engine. |
| Are test sources checked? | `sourceSets = ["main"]` by default, `["main", "test"]` per rule. | Architecture rules are about production code; test code legitimately reaches into other modules' internals. |
| Unresolved (external) symbols | Kept symbolic; participate in `denyPackages` only. | A JDK type has no module and cannot violate a module rule. |
| Does a nested module inherit its parent? | `gi-policy-object-mgmt` is its own module; `parent` is available as an extra matcher (`{ moduleTree = "gi-policy" }`). | 506 measured edges cross exactly this boundary; both readings must be expressible. |
| Self-edges, duplicates | Collapsed to a set of `(from module, from role, to module, to role)`; **one diagnostic per distinct pair plus the first file/line**, with `occurrences = n` in the message. | 2,628 `domain→models` edges would otherwise bury the report. A per-edge report is available via `--verbose`. |
| Where do exemptions live? | `except` (name patterns) in config, plus a `baseline` file for "current debt, frozen". | Baseline is what allows adoption on a 24k-file repo without a big-bang cleanup; `except` is for intentional, permanent exceptions. |

## 7. Executor

New subcommand, separate from `check`:

```
groovylint arch --config architecture.toml [paths] [--baseline arch-baseline.txt] [--format text|json|sarif]
```

`check` stays per-file and fast; `arch` is the only command that needs the whole
project. Pipeline:

1. **Discover** — read `settings.gradle` (or scan) → modules, sub-modules, roles;
   read each `build.gradle` → declared edges. Missing `build.gradle` is not an
   error (not every layout is Gradle).
2. **Parse** — existing parser, parallel over files, same discovery code as `check`.
3. **Classify** — attach `ClassFact` (module, role, source set).
4. **Extract edges** — declared types, `extends`/`implements`, `new`, casts, static
   access, annotations, and imports as a separate origin. Unresolved stays symbolic.
5. **Evaluate** — `composition`, `packagePolicy`, `agreement` need facts only;
   `dependency` rules fold the edge set into pairs and compare against the lattice.
6. **Report** — existing `Diagnostic` shape, `rule = <id>`, `because` in the
   message tail; baseline filtering; deterministic order `(file, line, rule, id)`.

Reuse, not re-implementation: discovery, parser, `Diagnostic`, formatters and the
rayon pool are shared with `check`. New code is `ProjectIndex`, the layout
classifier and the rule evaluator.

Performance: `check` parses 24,345 files in 3.7 s; `arch` does one extra pass over
the same ASTs (no re-parse) plus a set-based fold, so the target is the same order
of magnitude. The edge set is O(distinct module/role pairs), not O(edges), which
keeps both memory and the fold trivial.

## 8. Day-0 state of `saas-service`

Measured with declared types + imports resolved through the derived module map
(main sources only):

| Cross-module role pair | Edges |
|---|---:|
| `domain → models` | 2,628 |
| `application → models` | 2,024 |
| `infrastructure → models` | 1,189 |
| `models → models` | 1,075 |
| `application → client` | 1,047 |
| `adapter → models` | 543 |
| `domain → client` | 478 |
| `client → models` | 280 |
| **`domain → domain`** | **229** |
| **`application → domain`** | **161** |
| **`models → domain`** | **81** |
| Edges into an internal role of another business module | **506** |

So the lattice rules are violated hundreds of times today; the client rule
(`cross-module-only-through-client`) a few hundred more. **Adoption therefore
requires the baseline on day 0**, and the first useful deliverable is not "zero
violations" but "no new violations", which is exactly what the CI gate needs.

## 9. Testing

1. **Synthetic fixture project** (`tests/fixtures/arch/project-a`): 5 modules,
   roles per module, one violation per rule kind, including a nested module, a
   shared module, an unknown role, an unused import and a test source. Asserts
   exact diagnostics — this is the specification of the DSL.
2. **Corpus golden**: the counts in §8 as a snapshot test, so extraction changes
   that silently alter edge semantics fail loudly.
3. **ArchUnit parity**: re-implement the 11 existing `*ArchTest` dependency
   assertions as one config, run both, diff `(file, line, rule)` — the same method
   used for the CodeNarc comparison.
4. **Determinism**: same output with `--threads 1` and `--threads 8`.

## 10. Phasing

| Step | Content | Effort |
|---|---|---|
| 1 | `ProjectLayout` (settings.gradle/scan, `moduleDepth`, roles, aliases, shared modules) + `arch --print-layout` | 1 d |
| 2 | `ProjectIndex` + edge extraction + `--print-edges` (debuggable without rules) | 1 d |
| 3 | `composition` + `agreement` rules | 0.5 d |
| 4 | `dependency` (intra/cross, allow/deny, `includeImports`, pair folding) + baseline | 1 d |
| 5 | `packagePolicy`, fixtures, corpus golden, ArchUnit parity run | 1 d |
| 6 | Gradle facade task (`archCheck`) mapping config to a task input | 0.5 d |

Steps 1–2 are useful on their own: `--print-layout` already answers "did we name
and nest the sub-modules the way the conventions say", which is half of what the
user asked for.

## 11. Decisions needed before implementation

1. **`moduleDepth` default.** `-1` (parent of the include: `object-mgmt:shared-object-mgmt`
   is one module) or `1` (`object-mgmt` owns both)? The 506 measured edges make
   this the difference between a clean rule and an exemption list.
2. **Are `models` / `shared-models` / `metadata-context` one role or three?**
   They alias to `models` in §3; keeping them apart costs nothing in the engine and
   buys sharper rules (`models` may not reach `domain` is true; whether
   `metadata-context` may reach `client` is a separate question).
3. **Should `cross-module-only-through-client` be one allow rule, or a deny rule
   per target role?** Allow is complete (new roles are forbidden by default) but
   the diagnostic for a newly added role is unhelpful; deny gives better messages.
   Proposal: allow, with the violation message naming the allowed target set.
4. **Baseline format and ownership.** Proposal: a sorted text file
   `rule-id<TAB>from-fqn<TAB>to-symbol`, regenerated with `--update-baseline`;
   entries that no longer violate are removed automatically with a warning. Is a
   committed baseline acceptable in the repo, or should it be generated in CI?
5. **Declared-vs-observed Gradle rule now or later?** Reading `build.gradle`
   `project('…')` lines is cheap and catches the "type came in through a transitive
   dependency" failure the conventions explicitly forbid. It also means the engine
   needs a Gradle-file reader. Proposal: phase 5, behind `[[rules]] kind = "agreement"`.
6. **One config per project, or per module?** 208 sub-modules × a config each would
   recreate the problem. Proposal: one project config; per-module variation only
   through `modules = "glob"` selectors, and the Gradle facade passes `--slice`
   for incremental runs.

## 12. Design checklist

- **SRP**: `ProjectLayout` (naming/layout facts), `ProjectIndex` (facts + edges),
  `ArchEvaluator` (rule kinds), `Baseline` (debt file). Notation is not mixed into
  the evaluator; the evaluator never parses.
- **GRASP Information Expert**: the index answers "who is this class and what does
  it reach"; rules never re-walk the AST. **Protected Variations**: the role
  vocabulary and the shared-module list are config data — the extension point that
  makes a new layout a config change instead of a code change.
- **Pattern usage — five questions**: a strategy trait per rule kind would add a
  vtable and a registry to serve four variants; instead one `enum RuleKind` plus a
  match. It simplifies the config-to-check mapping and keeps the diagnostic path
  single; it complicates nothing (no object lifetimes, no registry). Verdict: **no
  pattern**, extension is a new enum variant + config validation.
- **Reuse**: parser, discovery, diagnostics, formatting, thread pool — shared with
  `check`. New: two modules of code (`layout`, `index`) plus one evaluator.
