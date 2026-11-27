# Architecture rule model: design v4

Status: **v3 is implemented as a prototype; v4 is the model being settled.** See
[`ARCH-IMPLEMENTATION.md`](ARCH-IMPLEMENTATION.md) for current behaviour, coverage and
limits, and §4.1 for what v4 changes and why. The code still carries v3's `visible`
constraint and its `self:` / `other:` prefixes; §10.1 lists that as the open decision.

Supersedes the module DSL of [`ARCH-MODULE-RULES.md`](ARCH-MODULE-RULES.md) (v2) and the
DSL sketch in [`ARCHITECTURE-RULES.md`](ARCHITECTURE-RULES.md) §3.

v2 put every constraint into one matrix: a source kind on one side, target kinds on
the other, with `allow` / `allowSelf` / `allowOther` / `deny` as the four axes. It
could express the rules, but reading one meant simulating a matrix, and because the
matrix was the only mechanism, things that are not a matrix (an ordered layer stack,
who may see a module, a graph property) had to be bent into it.

v3 separates what a rule is made of.

## 1. What a rule is

An architecture rule is a triple:

```
(target type T) × (selector over T) × (constraint vocabulary of T)
```

- **Target type** — one of a closed set: `module`, `package`, `class`, `method`,
  `field`. Different types can be constrained in different ways, and the
  implementation differs, so the type is part of the model.
- **Selector** — "which of these": name suffix, inheritance, required interface,
  annotation, signature, containment. Selectors compose with `and` / `or` / `not`.
- **Constraint** — "what they must or must not do". The vocabulary is **closed per
  target type**: modules have dependencies and composition, classes have
  dependencies, supertypes, annotations, placement, methods have annotations and
  signatures, statements have emptiness and literals.

Users compose selectors and constraints. They do not invent constraints, because a
closed vocabulary is what makes a violation explainable, baseline-able, and
actionable for the author — and for an agent editing the code. The closed
vocabularies are enumerated in [Appendix A](#appendix-a-reference-vocabulary); what
the engine has to implement to run them is [Appendix B](#appendix-b-what-the-engine-must-implement).

Three properties the model needs to be worth anything:

1. **Selectors are composable predicates.** "Enums that implement `BaseEnum`" is a
   conjunction, not a special case.
2. **Edges are collected at the finest granularity and projected upwards.** A
   dependency is first a class → class edge; a module → module edge is that same
   edge with both ends projected. "Module A must not depend on B" and "classes in
   `client` must not depend on `application`" are one edge model evaluated at two
   levels, not two implementations.
3. **Ownership is part of a selector; boundaries are a constraint.** A method's owner
   is a class, a class's owner is a module, and `in` is how a selector uses that. The
   *module boundary* is the one thing a selector must not try to express: it belongs to
   the module as a `visibility` constraint (§4), so a rule never has to ask "is this the
   same module as me".

## 2. Declaring targets

A target is declared once and referenced by name. Naming is the reuse mechanism:
change a suffix in one place, and every rule that uses the target follows.

```toml
version = 1

[targets]
# ─ by module kind ─
domain         = { type = "class", in = "*-domain" }
application    = { type = "class", in = "*-application" }
adapter        = { type = "class", in = "*-adapter" }
infrastructure = { type = "class", in = "*-infrastructure" }
client         = { type = "class", in = "*-client" }
models         = { type = "class", in = ["*-models", "*-shared-models"] }

# ─ platform libraries every module may use ─
shared-domain  = { type = "class", in = "shared-core:shared-core-domain" }

# ─ packages inside a module: the three levels a role actually lives at ─
domain-api     = { type = "package", name = "**.domain.**.api", under = "*.domain" }
domain-spi     = { type = "package", name = "**.domain.**.gateway" }
service-impl   = { type = "package", name = "**.domain.**.service.impl" }
http-entry     = { type = "package", name = "**.adapter.**.http" }

# ─ external packages and single classes ─
web            = { type = "package", name = ["org.springframework.web.**", "javax.servlet.**"] }
persistence    = { type = "package", name = "org.apache.ibatis.**" }
spring         = { type = "package", name = "org.springframework.**" }
app-context    = { type = "class",   name = "com.acme.platform.AppContext" }
page           = { type = "class",   name = "com.acme.platform.vo.Page" }

# ─ shapes ─
domain-service = { type = "class",  in = "domain", name = ["*Service", "*ServiceImpl"],
                   not = { kind = "interface" } }
service-impl-class = { type = "class", in = "domain", name = "*ServiceImpl" }
repository     = { type = "class",  in = "domain", name = "*Repository" }
repository-query = { type = "method", in = "repository", name = "query*" }
repository-load  = { type = "method", in = "repository", name = "load*" }
business-enum  = { type = "class",  in = ["domain", "client"], kind = "enum" }
use-case       = { type = "class",  in = ["domain", "client"], name = "*Application" }
```

Rules for the matcher:

- a location matches a target when **all** of its given selectors match;
- **a node may match several targets, and every rule on each of them applies.** v3 said
  "first match wins", which made a narrow target (`domain-service`) shadow a broad one
  (`domain-class`) and silently drop the broad target's rules. Nesting is the normal
  case — a service class is also a domain class — so the engine evaluates all matches;
- a location matched by several targets is reported by `arch validate` as information,
  not as an error: the author usually meant it. It is an error only when the overlapping
  targets both carry `visibility` (two module boundaries for one node);
- a location matched by no target is `unknown` and reported;
- patterns are anchored; `*` is any run of characters except `:`, `**` crosses `:`,
  `?` is one character, `{slot}` captures one segment for diagnostics; `regex = "…"`
  is the escape hatch for hostile naming.

### 2.1 Containment: module ⊃ package ⊃ class ⊃ member

The target types are not a flat list, they are the containment chain of the language:

```
module ─┬─ package ─┬─ class ─┬─ field
        │           │         └─ method ─── parameter
        │           └─ (sub-packages, recursively)
        └─ (packages declared by that module)
```

This is why `package` is a target type and not an attribute of one. In Java and
Groovy the package *is* the architectural unit: it carries the role (`..service`,
`..gateway`, `..http`), it is what `package-private` access enforces, what
`module-info` exports, and what a slice is cut along. A module is the ownership and
deployment boundary; a package is the role inside it.

Consequences for the model:

- **`in` is containment, at any level**: a class is `in` a package or `in` a module,
  a method is `in` a class, a package is `in` a module. One keyword, one meaning.
- **Package patterns are hierarchical**: `com.acme.billing.domain.*` is one segment,
  `com.acme.billing.domain.**` is that package and everything below it, and
  `under = "com.acme.billing.domain"` is shorthand for the latter. A class belongs to
  a package target when its package matches, sub-packages included when the pattern
  says so.
- **Package edges are the same edges, projected to packages** — a third projection of
  the class → class edge, exactly like modules (principle 2 in §1).
- **`packageMatches` disappears.** It was a class constraint in v2 and an earlier
  draft; with packages as targets it is just `in = <package target>`. Adding the
  missing level removed a keyword instead of adding one.
- **A package carries no `visibility`.** Java already has package-private, and a config
  must not invent a second meaning for the same word: the module is the only construct
  with no access modifier of its own (§4).
- **Slices are within reach**: a package pattern with `{slot}` captures a segment, which
  is all a slice needs. The cycle check that makes slices useful is module-only in the
  first release (§10), so this arrives with it.

Which level should a rule use? The one whose meaning the team already agreed on:
cross-module contracts at `module` level (who may depend on whom), intra-module roles
at `package` level (adapter may not reach `..gateway..`), and `class`/member level for
conventions that are about the declaration itself.

**Naming is not mandatory.** A selector used once is written inline, so a small
project does not maintain a name table:

```toml
[[rule]]
target = { type = "class", in = "*-adapter" }
must   = { notDependOn = ["domain"] }
```

`arch lint-config` suggests extracting a name when the same selector appears twice.

## 3. Rules, grouped by target

A rule block is `target` + `must` (a map of constraint → arguments) + `because`.

```toml
[[rule]]
target  = "domain"
must    = { notDependOn = ["application", "adapter", "infrastructure", "web", "persistence"] }
because = "02 §3.1 domain 只承载领域规则，不能依赖 application/adapter/infrastructure"

# 跨模块的那一半不是规则，是模块自身的可见性（§4）：
[[rule]]
target  = "domain-module"
must    = { visibility = "private" }
because = "02 §3.1 domain 只在所属模块内可见，跨模块必须走 client"
```

Read together the two blocks say: domain may not reach the other layers at all, and
nothing outside its own module may reach domain. The first half is a rule, the second is
a module property — which is why neither needs a "same module or not" qualifier.

One target may appear in several blocks (different `because`, different severity
later); grouping is the default shape, not a restriction. `arch fmt` sorts blocks by
target so that all constraints on one object read together.

Package-level rules read the same way, because the mechanism is the same:

```toml
[[rule]]
target  = "domain-spi"                                  # ..domain..gateway
must    = { notDependOn = ["persistence", "web", "spring"] }
because = "02 §3.7 领域仓储接口不依赖任何技术实现，由 infrastructure 实现"
```

A package is not a boundary the engine can hide behind: no visibility constraint
applies to packages, because Java already has package-private and a config should not
invent a second meaning for it. Isolation *inside* a module is a plain deny rule
(`notDependOn`), which needs no relation: the classes a package may still reach are the
ones the rule does not name.

`except` is the node-level escape, and only that — a list of concrete nodes, not a
second selector language:

```toml
[[rule]]
target  = "domain-method"
must    = { notAnnotatedWith = ["org.springframework.transaction.annotation.Transactional"] }
except  = ["com.acme.billing.domain.service.impl.BillingFileServiceImpl"]   # 存量，见 baseline
because = "02 §3.4 事务边界只在 application 层声明，domain 由调用方决定事务"
```

A rule block also carries two optional scoping keywords:

```toml
[[rule]]
target   = "domain-spi"
must     = { notDependOn = ["persistence"] }
on       = "test"         # default "main"
severity = "warn"         # default from [project].severity, which defaults to "error"
```

Rules check **main** unless a block says otherwise, so a test-architecture rule can
never be reached by accident, and a rule that is still being introduced can start at
`warn` without weakening every other rule. There is no per-target severity: a
constraint either applies or it does not, and its weight belongs to the rule that
states it.

## 4. Cross-module boundaries: `visibility` on modules

A module is the one construct the language gives no access modifier, and the one place
an architecture rule needs one: "this is mine, the rest of the project may not reach
into it". That is a property of the module, so it is declared as a constraint on a
module target:

```toml
[[rule]]
target  = "domain-module"
must    = { visibility = "private" }
because = "02 §3.1 domain 只在所属模块内可见，跨模块必须走 client"

[[rule]]
target  = "client-module"
must    = { visibility = "public" }        # written out, so nobody has to guess the default

[[rule]]
target  = "domain-module"                  # the wiring modules may see everything
must    = { visibility = { to = ["self", "boot-app", "gateway-app"] } }
```

| Value | Means |
|---|---|
| `"public"` | any module may depend on it (the default when a rule says nothing) |
| `"private"` | only its own module may depend on it |
| `{ to = [...] }` | the listed module targets, plus the keyword `self` |

`self` here is a keyword of the visibility value, not a prefix on a dependency: it
means "the module this node lives in", and the arrow has one direction only.

### 4.1 Why there is no `other:`

v3 had `self:` / `other:` prefixes on dependency targets: "my own module's domain yes,
another module's domain no". Two things were wrong with them.

1. **They mixed a predicate with an instance.** A target is a pattern; `self:` asks the
   engine to compare the *instance* behind the match with the source. That is a relation
   wearing a selector's clothes, and it reads as one list while meaning two.
2. **They were only ever about the module boundary.** Every case in the real catalog that
   needed them was "another module must not reach into my layer" — which is exactly what
   module visibility says, once, at the module instead of in every rule.

With visibility declared on modules, the whole lattice is two independent mechanisms and
no relation anywhere:

- **who may cross my boundary** → `visibility` on the module (§4);
- **how far my layer may reach** → a plain deny rule, naming the targets it must not
  reach. A target that is not named is allowed, which is how "my own module's domain is
  fine" is expressed without a relation: it is simply not in the deny list.

The catalog's three cross-module rules collapse to two `visibility` declarations, and
`DependencyRef` loses three variants: a dependency argument is a target name, nothing
else.

Two consequences worth stating:

- **Deny lists, not allow lists, for intra-module layering.** An allow list would have to
  enumerate the JDK, the framework annotations and every permitted sibling; a deny list
  names the three layers that matter. `dependOnlyOn` still exists for the closed-world
  cases (a models module that may reach nothing), and `any:` is enough there.
- **Visibility is checked on main sources by default.** Cross-module *test* code
  legitimately reaches into other modules; a visibility rule that also applied to tests
  would be switched off within a week. `on = "test"` opts in.

## 5. Existence, counts, and quantified members

Two constraint shapes are not about a single node but about a set or about members:

```toml
[[rule]]
target  = "use-case"
must    = { exist = false }
because = "02 §2 Application 是用例编排，属于 application 层，不能落在 domain 或 client"

[[rule]]
target  = "module"
must    = { children = ["adapter", "application", "domain", "infrastructure", "client", "models"],
            counts   = { domain = ">=1", adapter = "<=1", client = "<=1" } }
because = "02 §2 模块组合只有这几类子模块"

[[rule]]
target  = "mapper"
must    = { allParams = { annotatedWith = ["org.apache.ibatis.annotations.Param"] } }
because = "08 Mapper 的每个参数必须 @Param 显式声明"

[[rule]]
target  = "tenant-model"
must    = { extend    = "com.acme.platform.BaseDataVerPoModel",
            implement = ["com.acme.platform.TenantCodeAware"],
            annotatedWith = ["com.acme.platform.PrimaryKey"] }
because = "08 租户表 PO 继承基类、实现 TenantCodeAware、并声明主键"
```

`all` / `any` / `none` over the members of a target (`allMembers`, `allMethods`,
`allParams`) is a **generic quantifier**, not one constraint per rule: a project that
later needs "every field is `private`" writes `allMembers = { modifier = "private" }`
without a code change. Several constraints in one `must` map are a conjunction, which
is what the ArchUnit `should().andShould()` chains were spelling out.

## 6. Types as values, and the limit of names

Some constraints take a type: `rawReturnType`, `implement`, `extend`. A target may be
used as that value, which is how "`load*` must not return a collection" is written:

```toml
[targets]
collection = { type = "type", name = ["java.lang.Iterable", "java.util.Collection",
                                    "java.util.List", "java.util.Set", "java.util.Map"],
               array = true }

[[rule]]
target  = "repository-load"
must    = { notRawReturnType = ["collection", "page", "java.util.Optional"],
            notAnnotatedWith  = ["Nullable"] }
because = "02 §4 load 承诺数据存在，不返回集合/分页/包装器，也不允许 @Nullable"
```

This is the one place where a real type system would buy something: ArchUnit asks
`assignableTo(Iterable)`, we compare written names. The approximation is honest and
usually exact, because the rule is written against the *declared* type anyway
(`haveRawReturnType` is raw, generics erased), but a method declaring an exotic
implementation type would slip through. Declaring the list is also a check on the
rule: it says out loud which types count.

**What the model never needs is inference.** Every constraint here is a judgement
about a written name, an annotation, a shape, or an edge between names — not about
what a dynamic expression evaluates to. `def x = service.find(…)` hides an edge; that
is a miss, not a false positive, and closing it needs the type model described in
[`ROADMAP.md`](../ROADMAP.md) §7.5, which is a separate decision.

## 7. Worked example: a real catalog, de-duplicated

A production codebase encodes its architecture as 112 ArchUnit rule methods across 18
per-module test files (2,772 lines of Groovy). They are the same ~45 intents repeated
per module, plus module-specific extras. Expressed in v4 the whole catalog is one
config, because a target is a pattern rather than a literal package. Target
definitions are abbreviated: the ones from §2 are reused, `api-class`, `api-method`,
`mapper` and `tenant-model` follow the same shapes.

```toml
# ── dependency direction ────────────────────────────────────────────────
[[rule]]
target  = "domain"
must    = { notDependOn = ["application", "adapter", "infrastructure", "web", "persistence",
                           "app-context"] }
because = "02 §3.1 domain 只承载领域规则"

[[rule]]
target  = "domain-module"
must    = { visibility = "private" }
because = "02 §3.1 domain 只在所属模块内可见"

[[rule]]
target  = "models"
must    = { notDependOn = ["application", "adapter", "infrastructure", "domain"] }
because = "02 §3.2 模型模块只放稳定业务模型"

[[rule]]
target  = "client"
must    = { notDependOn = ["application", "adapter", "infrastructure", "domain", "spring",
                           "app-context"] }
because = "02 §3.3 client 只暴露跨模块契约"

[[rule]]
target  = "adapter"
must    = { notDependOn = ["domain", "infrastructure"] }
because = "02 §3.4 adapter 只调用 application（本模块的 application 不在 deny 列表里）"

[[rule]]
target  = "application"
must    = { notDependOn = ["adapter", "infrastructure"] }
because = "02 §3.5 application 只依赖本模块 domain 与提供方 client"

[[rule]]
target  = "application-module"
must    = { visibility = "private" }
because = "02 §3.5 application 不对其他模块暴露"

[[rule]]
target  = "infrastructure"
must    = { notDependOn = ["adapter", "application"] }
because = "02 §3.6 infrastructure 只向下依赖本模块 domain"

[[rule]]
target  = "infrastructure-module"
must    = { visibility = "private" }

[[rule]]
target  = "domain-service"
must    = { notDependOn = ["app-context"] }
because = "07 §2 domain service 用依赖注入，上下文由入参传入"

# ── annotations and placement ──────────────────────────────────────────
[[rule]]
target  = "domain-service"
must    = { annotatedWith = "org.springframework.stereotype.Service" }

[[rule]]
target  = "service-impl-outside"        # *ServiceImpl classes, wherever they live
because = "07 §3 Service 实现统一放 service.impl，与接口包分开"
[rule.must]
exist = false                           # ... outside the ..service.impl package

[targets.service-impl-outside]
type = "class"
in   = "domain"
name = "*ServiceImpl"
not  = { type = "class", in = "service-impl" }

[[rule]]
target  = "use-case"
must    = { exist = false }

[[rule]]
target  = "business-enum"
must    = { implement = "com.acme.platform.enums.BaseEnum" }
because = "07 §5 业务枚举必须 implements BaseEnum"

[[rule]]
target  = "api-method"
must    = { annotatedWith = ["com.acme.platform.ApiSecurity"] }
because = "07 §6 每个启用的业务 HTTP 方法必须声明 @ApiSecurity"

[[rule]]
target  = "api-class"
must    = { annotatedWith    = ["com.acme.platform.ApiScope"],
            notAnnotatedWith = ["TenantScoped", "PlatformScoped"] }
because = "07 §6 @ApiScope 决定请求上下文，旧注解不再参与解析"

# ── repository naming contract ─────────────────────────────────────────
[[rule]]
target  = "repository-query"
must    = { rawReturnType = "page" }
because = "02 §4 query 前缀表示分页查询"

[[rule]]
target  = "repository-get"
must    = { notRawReturnType = ["page"] }

[[rule]]
target  = "repository-load"
must    = { notRawReturnType = ["collection", "page", "java.util.Optional"],
            notAnnotatedWith  = ["Nullable"] }

# ── package layout inside a module ─────────────────────────────────────
[[rule]]
target  = "api-class"                   # classes in the ..adapter..http packages
must    = { notDependOn = ["repository", "persistence"] }
because = "02 §3.9 HTTP 入口只编排用例，不直接访问仓储与持久化"

# ── module composition ─────────────────────────────────────────────────
[[rule]]
target  = "module"
must    = { children = ["adapter", "application", "domain", "infrastructure", "client", "models"],
            counts   = { domain = ">=1", adapter = "<=1", client = "<=1" } }

[[rule]]
target  = "module"
must    = { acyclic = true }
because = "02 §5 模块之间不得成环"
```

What the exercise showed:

- **The repetition is gone.** One target definition covers every module, so 112
  hand-written rules become ~40 blocks; `because` reads like the document it quotes.
- **The three ad-hoc escape mechanisms collapse.** The codebase currently uses
  hand-maintained `LEGACY_*_CLASSES` sets, `resideOutsideOfPackages(...)` carve-outs
  and `allowEmptyShould(true)`. In v3 they are `except`, `baseline`, and an
  `arch lint-config` warning, respectively.
- **`orShould().beAnnotatedWith(A).orShould().beAnnotatedWith(B)`** becomes a list
  argument, which is what it always meant.
- **Four things v3 has and v2 did not:** `visibility` on modules, `exist`/`counts`,
  the `all*` quantifiers, and named targets reusable across rule blocks.

Two intents in that catalog still need a decision, both because they reach past a
single node:

- **Call-graph rules** — "adapter calls application only through transactional
  methods", "HTTP entry points check permissions". These need resolved call targets
  *and* the callee's annotations, i.e. a project call graph, not a dependency edge.
  They are out of scope for v4 and stay in [`ARCHITECTURE-RULES.md`](ARCHITECTURE-RULES.md) §5
  phase 5.
- **`assignableTo`-style type questions** — see §6.

## 8. Validation and baseline

`arch lint-config` reads the config without analysing code and reports:

- unknown target name in a rule or constraint;
- a selector that matches nothing in the project (`allowEmptyShould` in ArchUnit
  terms — a rule that silently checks nothing is worse than no rule);
- a node matched by two targets that both declare `visibility` (two boundaries, one node);
- a target that matches nothing at all;
- an unused target, a target with no rule, a rule with an empty `must`;
- `visibility` on anything but a module target (it is a module-level concept);
- a `to` entry that names something other than a module target;
- a module matched by two visibility rules (reported, not merged: merging would let one
  declaration silently weaken another);
- a `private` declaration in a single-module project, which constrains nothing.

`baseline` records today's violations by `rule id + target name + node`, so adoption
starts at zero friction while the debt list only shrinks. Hiding a new violation must
require editing the baseline file, which is reviewable.

## 9. Migration from v2

| v2 | this model |
|---|---|
| `[kinds.x] name = "*-domain"` | `[targets] domain = { type = "class", in = "*-domain" }` (same idea, now typed) |
| `allow` / `allowSelf` / `allowOther` | a deny list for the layer, plus `visibility` on the module |
| `self:` / `other:` on a dependency | `visibility` on the module (cross-module) or nothing (in-module) |
| `visible = [...]` on a package | `visibility` on the module; a package needs a deny rule |
| `deny = [...]` | `notDependOn = [...]` on the same target |
| `exemptFrom = [...]` on a rule | `except = [...]` node list, or `baseline` |
| `parent` / `children` / `counts` containment rule | `children` / `counts` constraints on `target = "module"` |
| `check = "package-root"` | a negative class target (`not = { type = "class", in = <package> }`) with `exist = false` |
| `check = "declared-dependency"` | unchanged, phase 5 |
| `nameAliases` / `pathAliases` | a list in `name` / `in` |

## 10. Decisions

Settled:

| Question | Decision | Consequence |
|---|---|---|
| Scope of `acyclic` | **Modules only** in the first release | Only the module rule in §7 is MVP; package cycles wait for a decision about which containment edge closes one |
| Severity | One project default, `severity` overridable per rule block | The default keeps today's behaviour: a violation fails the build |
| Source set | Rules check **main** by default; `on = "test"` is explicit | Test architecture needs its own rule blocks, and can never inherit a `*-domain` target by accident |
| `target = "any"` | **Not part of the language** | A constraint is written per target type, so every diagnostic names the kind of node it found. Reconsider only with a concrete case |
| Where the config lives | **One file at the repository root** | Module ownership is expressed by target names and `except`, not by file ownership; `arch lint-config` reports unused targets so a stale section is visible |

Still open, and why they can wait:

1. **Call-graph scope.** Whether resolved call targets ship with dependency edges or
   later. Two catalog intents need them; both are listed as phase 5+ in
   [`ARCHITECTURE-RULES.md`](ARCHITECTURE-RULES.md).
2. **Package patterns that span modules.** `com.acme.**.api` matches packages in every
   module, which is fine for conventions (nothing relational is evaluated on a package
   any more). Whether to allow it for *dependency* rules is still worth a sentence in the
   docs, but it no longer hides an ambiguity.
3. **Slices as a first-class concept.** A `{slot}` package pattern plus a cycle check
   gets the behaviour; whether to also surface `slices = { by = "...", of = "..." }`
   as sugar. Blocked behind decision 1 anyway, since slices are only useful with a
   cycle check.

### 10.1 Raised by the first field run

The prototype ran against a production monorepo (24,481 files, 217 modules); the
mechanics are recorded in [`ARCH-IMPLEMENTATION.md`](ARCH-IMPLEMENTATION.md) §8. Four of
the findings are design questions, not bugs:

4. ~~**`other:` as a dependency part.**~~ Resolved in v4: no relation. Module
   visibility (§4) states the cross-module half once, and deny lists state the
   in-module half; `self:` / `other:` / `none` are gone from the vocabulary.

    The prototype still implements v3 here: `visible` is a constraint on packages with a
    `self:` / `any:` / `none` value, and `DependencyRef` still has the three prefixes and
    the alias. Moving to v4 means: `visibility` becomes a module-only constraint whose
    value is `public` / `private` / `{ to = [...] }`, `DependencyRef` collapses to a
    target reference, and the package form is removed. Deliberately not done yet — the
    DSL is still being reviewed.
5. **Does `x.**` include `x`?** In the prototype it does not, which silently reported
   every class of the package a rule was written to protect. The common meaning is
   "this package and everything below it", so the answer should be yes, with `x.*`
   left as the single-segment form.
6. **Meta-annotation matching.** `@GetMapping` and friends are `@RequestMapping` at one
   remove; catalogs express "HTTP entry point" that way. Either add a
   `metaAnnotatedWith` selector, or leave it to explicit annotation lists and say so in
   the vocabulary.
7. **Target-side `except`.** Exemptions in real catalogs are written against the
   *forbidden target* ("these two enums are fine to reference"), while `except` exempts
   sources. Decide whether `except` accepts both sides or whether `notDependOn` entries
   carry their own exceptions.

### 10.2 Scope settled by the field run

Decisions taken while encoding a real catalog (the full mapping is in the local,
unversioned note; the shape is in §7):

- **Groovy sources only.** No Gradle metadata is read: "the dependency must be declared in
  `build.gradle`" is out of scope, and `acyclic` therefore judges the *observed* graph.
  A cycle that exists only in the declared graph is invisible, which is acceptable while
  the rules are advisory.
- **Annotation-argument combinations are out of scope.** `@ApiScope(TENANT)` requiring a
  non-empty `apps`, or `@ApiSecurity(PUBLIC)` forbidding permission groups, is a rule
  about an annotation's internals; `annotationArgument` compares a written fragment and
  stops there.
- **Return-type constraints accept a target, not only a type name.** "A client method must
  not return a `*DO`" is `notRawReturnType = ["do-class"]` where `do-class` is a class
  target. No new target type is needed: a return type is a name that resolves to a class,
  so the existing class targets answer it.
- **`visibility` keeps its name on both target types.** It means method accessibility on a
  `method` and boundary visibility on a `module`. Constraint resolution is already keyed
  by target type, so there is no ambiguity to resolve — but a wrongly paired constraint
  must fail with a message that says which meaning it expected.
- **Baseline is a prerequisite, not a nicety.** A ruleset that is right about an existing
  codebase will report hundreds of violations on day one; adoption needs the freeze from
  §8 before any of it can be enforced.
- **Body predicates are a new family.** Two real rules ask about a method's *return
  expression* — "the table name must come from a constant, not a string literal" and "the
  master-table name must be a constant declared in a `*MasterTableConst` class, or null".
  Neither is a signature question, so the vocabulary needs `allMethods = { match = ...,
  must = ... }` (a quantified member constraint, not only a member selector) plus
  `returnsConstant` / `returnsConstantOf = { from = [...], orNull = true }`. That requires
  return-expression facts and a small amount of symbol resolution (a bare identifier must
  be traced to a static final field or a static import), which is the first place the
  architecture rules need something closer to §7.5 tier 2.
- **Transitive supertypes.** `BasePoModel` subclasses reach it through two intermediate
  classes, so `extends` needs a `transitive` flag; the project index already holds the
  closure for project classes.

## Appendix A: reference vocabulary

The complete keyword surface. Everything not listed here does not exist, which is
the point: a config can be validated by reading it, and a violation can name the one
keyword it broke.

### A.1 Target types

| Type | Covers |
|---|---|
| `module` | a Gradle module |
| `package` | a package and, when the pattern says so, its sub-packages |
| `class` | class, interface, trait, enum, annotation type, including nested ones |
| `method` | methods and constructors |
| `field` | fields and properties |

The five form the containment chain of §2.1. Deliberately absent from the first
release: `statement`, `expression` (no architecture rule needs them) and `parameter`
(`allParams` covers it).

### A.2 Selectors

Common to every type:

| Keyword | Meaning |
|---|---|
| `name` | name pattern over the simple name (`*`, `**`, `?`, `{slot}`) |
| `nameMatches` | the same, as an anchored regex |
| `in` | containing target: module for class/field/method, class for field/method |
| `anyOf` | union of target names |
| `not` | negation of another selector |

Additional per type:

| Type | Selectors |
|---|---|
| `module` | `path` |
| `class` | `kind` (`class`/`interface`/`trait`/`enum`/`annotation`), `abstract`, `extends`, `implements`, `annotatedWith`, `nested` |
| `method` | `annotatedWith`, `rawReturnType`, `modifiers` (`static`/`public`/`protected`/`private`/`abstract`/`final`), `paramCount` |
| `field` | `annotatedWith`, `type`, `modifiers` |

`packageMatches` is gone: with packages as targets it is `in = <package target>`, and
`under` is the shorthand for "this package and everything below it". A package pattern
that contains `{slot}` is a slice, and `slot` names the capture so rules and
diagnostics can talk about it.

### A.3 Constraints

| Type | Constraints |
|---|---|
| `module` | `dependOnlyOn`, `notDependOn`, `visibility`, `children`, `counts`, `acyclic` |
| `package` | `dependOnlyOn`, `notDependOn` |
| `class` | `dependOnlyOn`, `notDependOn`, `annotatedWith`, `notAnnotatedWith`, `annotationArgument`, `extend`, `implement`, `exist`, `allMembers`, `allMethods`, `allFields` |
| `method` | `annotatedWith`, `notAnnotatedWith`, `rawReturnType`, `notRawReturnType`, `allParams`, `visibility` |
| `field` | `annotatedWith`, `notAnnotatedWith`, `type`, `notType` |

Notes that decide behaviour, not syntax:

- `dependOnlyOn` / `notDependOn` take **target names only**, in any module. No prefix,
  no relation: "my own module's copy is allowed" is expressed by *not naming it* in a
  deny list (§4.1).
- `visibility` is module-only and states who may depend on the module: `"public"` (the
  default), `"private"`, or `{ to = [...] }` with module target names plus the keyword
  `self`. It is a constraint on a module target, never an attribute of a target — a
  target answers "who matches", a rule answers "what must hold".
- `acyclic` is available on `module` only in the first release. Package-level cycles
  need a decision about which containment edge closes a cycle, so they wait.
- `children` and `counts` are allow-lists: a child whose target is not listed is a
  violation, which is what makes module composition closed.
- `exist = false` states "no node may match this target"; `counts` covers `>=n`,
  `<=n`, `*`, `+`.
- The `all*` quantifiers evaluate over the members of a match (methods, fields,
  parameters). An empty member set is **true**, and `arch lint-config` reports the
  target instead — a vacuous truth must never read as a pass.
- `annotationArgument = { annotation, name, value }` compares the written argument
  text, not a resolved constant.

### A.4 Rule keywords

| Keyword | Meaning |
|---|---|
| `target` | the target whose nodes the rule applies to |
| `must` | a map of constraint → arguments; several entries are a conjunction |
| `except` | node-level exemption list on a rule (sources, not targets) |
| `because` | the reason printed with every violation |
| `severity` | `error` (default) or `warn`, per rule block |
| `on` | `main` (default) or `test`, per rule block |
| `self` | a `visibility` value meaning "the module this node lives in" |

### A.5 Reserved words

Every keyword above is reserved: `arch lint-config` rejects a target name that
shadows one, because `name = "target"` plus a target called `target` is a config bug
that would otherwise surface as a silent mis-match.

## Appendix B: what the engine must implement

The DSL is a shell; the work is underneath it.

1. **Fact extraction** — package, imports, declarations, `extends`/`implements`,
   field and parameter types, annotations with arguments, `new`, casts, class
   literals, call sites; every fact keeps a span.
2. **Name resolution** — fully qualified name → explicit import → same package →
   wildcard import → static import → project index. Unresolved names stay symbolic;
   nothing is guessed.
3. **Project index** — class → module, package, target classification, supertypes,
   annotations; package → sub-packages, module; module → children, package roots.
4. **Edges and projection** — collect class → class, project to package → package and
   module → module. One edge model, evaluated at any of the three levels.
5. **Target matching** — declaration order, first match wins, ambiguity reported.
6. **Constraint evaluation and diagnostics** — finding site, `because`, baseline key
   (`rule id` + target name + node).
7. **`arch lint-config`** — unknown target, selector that matches nothing, ambiguous
   classification, unused target, empty rule.
8. **Baseline** — record today's findings, fail only on new ones.

### B.1 Phasing, and what each phase unlocks

| Phase | Implements | ArchUnit intents covered |
|---|---|---:|
| 1 | facts, resolution, index (module + package tree), class dependencies / annotations / placement | 24 |
| 2 | method signature constraints (`rawReturnType`, `allParams`, `@Nullable`) | 29 |
| 3 | field annotations, `annotationArgument`, `extend` / `implement` / `exist` | 34 |
| 4 | module composition (`children`, `counts`, `acyclic`) | 37 |
| 5 | quantifiers, `arch lint-config`, baseline | 43 |
| later | call graph | +2 |

Coverage refers to the de-duplicated catalog in §7. Phase 1 alone is a usable
product: it catches every dependency-direction violation in that catalog.
