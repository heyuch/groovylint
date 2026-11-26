# Architecture rule model: design v3

Status: **design accepted, keywords frozen, not implemented.** The settling decisions
are in §10. Supersedes the module DSL of
[`ARCH-MODULE-RULES.md`](ARCH-MODULE-RULES.md) (v2) and the DSL sketch in
[`ARCHITECTURE-RULES.md`](ARCHITECTURE-RULES.md) §3.

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
3. **Ownership is part of a selector.** A method's owner is a class, a class's owner
   is a module. "Methods in client contracts must not be `@Transactional`" is
   `target = method`, selector `in = <the client target>`.

## 2. Declaring targets

A target is declared once and referenced by name. Naming is the reuse mechanism:
change a suffix in one place, and every rule that uses the target follows.

```toml
version = 1

[targets]
# ─ by module kind ─
domain         = { on = "class", in = "*-domain" }
application    = { on = "class", in = "*-application" }
adapter        = { on = "class", in = "*-adapter" }
infrastructure = { on = "class", in = "*-infrastructure" }
client         = { on = "class", in = "*-client" }
models         = { on = "class", in = ["*-models", "*-shared-models"] }

# ─ platform libraries every module may use ─
shared-domain  = { on = "class", in = "shared-core:shared-core-domain" }

# ─ packages inside a module: the three levels a role actually lives at ─
domain-api     = { on = "package", name = "**.domain.**.api", under = "*.domain" }
domain-spi     = { on = "package", name = "**.domain.**.gateway" }
service-impl   = { on = "package", name = "**.domain.**.service.impl" }
http-entry     = { on = "package", name = "**.adapter.**.http" }

# ─ external packages and single classes ─
web            = { on = "package", name = ["org.springframework.web.**", "javax.servlet.**"] }
persistence    = { on = "package", name = "org.apache.ibatis.**" }
spring         = { on = "package", name = "org.springframework.**" }
app-context    = { on = "class",   name = "com.acme.platform.AppContext" }
page           = { on = "class",   name = "com.acme.platform.vo.Page" }

# ─ shapes ─
domain-service = { on = "class",  in = "domain", name = ["*Service", "*ServiceImpl"],
                   not = { kind = "interface" } }
service-impl-class = { on = "class", in = "domain", name = "*ServiceImpl" }
repository     = { on = "class",  in = "domain", name = "*Repository" }
repository-query = { on = "method", in = "repository", name = "query*" }
repository-load  = { on = "method", in = "repository", name = "load*" }
business-enum  = { on = "class",  in = ["domain", "client"], kind = "enum" }
use-case       = { on = "class",  in = ["domain", "client"], name = "*Application" }
```

Rules for the matcher (unchanged from v2, they were the good part):

- a location matches a target when **all** of its given selectors match;
- targets are tried in declaration order, **first match wins**;
- a location matched by two targets is reported by `arch lint-config` as ambiguous,
  so `*-models` vs `*-shared-models` is a decision, not a coin flip;
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
  draft of v3; with packages as targets it is just `in = <package target>`. Adding the
  missing level removed a keyword instead of adding one.
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
target = { on = "class", in = "*-adapter" }
must   = { notDependOn = ["domain"] }
```

`arch lint-config` suggests extracting a name when the same selector appears twice.

## 3. Rules, grouped by target

A rule block is `target` + `must` (a map of constraint → arguments) + `because`.

```toml
[[rule]]
target  = "domain"
must    = { notDependOn   = ["application", "adapter", "infrastructure", "web", "persistence"],
            dependOnlyOn  = ["self:domain", "any:shared-domain", "any:models", "any:client"] }
because = "02 §3.1 domain 只承载领域规则，不能依赖 application/adapter/infrastructure，也不能依赖其他模块的 domain"
```

One target may appear in several blocks (different `because`, different severity
later); grouping is the default shape, not a restriction. `arch fmt` sorts blocks by
target so that all constraints on one object read together.

Package-level rules read the same way, because the mechanism is the same:

```toml
[[rule]]
target  = "domain-spi"                                  # ..domain..gateway
must    = { notDependOn  = ["persistence", "web", "spring"],
            dependOnlyOn = ["any:domain-api", "any:models"] }
because = "02 §3.7 领域仓储接口不依赖任何技术实现，由 infrastructure 实现"

[[rule]]
target  = "service-impl"                                # ..domain..service.impl
must    = { visible = ["self:module"] }
because = "02 §3.8 Service 实现只在本文档的模块内可见，跨模块只能用 client"
```

`visible` is the target-side form of a dependency rule — "who may depend on me" — and
it is what a package-level boundary needs, because writing the same thing as
`notDependOn` on every possible source is the matrix problem again. Its values are a
target list plus the keywords `self:`, `any:` and `none` (default: any node in the
same module).

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

## 4. Relations: `self:`, `any:` and `other`

Most dependency rules are about the module boundary, so a dependency argument is
either a target name (any module) or a prefixed relation:

| Form | Means |
|---|---|
| `models` | any module's `models` (the default) |
| `self:domain` | the `domain` of the *same* module as the source |
| `other:client` | another module's `client` — same as plain `client` |

`self:` is what makes a lattice rule closed: "domain may depend on its own domain
and on any `models`, nothing else" is one line instead of an allow/allowSelf pair.
v2 needed `allowSelf` and `allowOther` because it had no way to say *same instance*
inside a list; one prefix is cheaper to read than two axes, and `arch lint-config`
warns when `self:` is written for a target that has no module (a script, a shared
library).

Kinds that are reachable from everywhere (`client`, `models`, `shared-domain`) are
declared by listing them in each rule, not by a global allow switch: the rule stays
the complete statement about its target, so a new target cannot silently become
reachable.

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
collection = { on = "type", name = ["java.lang.Iterable", "java.util.Collection",
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
per module, plus module-specific extras. Expressed in v3 the whole catalog is one
config, because a target is a pattern rather than a literal package. Target
definitions are abbreviated: the ones from §2 are reused, `api-class`, `api-method`,
`mapper` and `tenant-model` follow the same shapes.

```toml
# ── dependency direction ────────────────────────────────────────────────
[[rule]]
target  = "domain"
must    = { notDependOn  = ["application", "adapter", "infrastructure", "web", "persistence",
                            "app-context"],
            dependOnlyOn = ["self:domain", "any:shared-domain", "any:models", "any:client"] }
because = "02 §3.1 domain 只承载领域规则"

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
must    = { dependOnlyOn = ["self:application", "any:client", "any:models"] }
because = "02 §3.4 adapter 只调用 application"

[[rule]]
target  = "application"
must    = { dependOnlyOn = ["self:domain", "any:shared-domain", "any:client", "any:models"] }
because = "02 §3.5 application 只依赖本模块 domain 与提供方 client"

[[rule]]
target  = "infrastructure"
must    = { dependOnlyOn = ["self:domain", "any:models", "any:persistence"] }
because = "02 §3.6 infrastructure 只向下依赖本模块 domain"

[[rule]]
target  = "domain-service"
must    = { notDependOn = ["app-context"] }
because = "07 §2 domain service 用依赖注入，上下文由入参传入"

# ── annotations and placement ──────────────────────────────────────────
[[rule]]
target  = "domain-service"
must    = { annotatedWith = "org.springframework.stereotype.Service" }

[[rule]]
target  = "service-impl-class"          # classes named *ServiceImpl
must    = { in = "service-impl" }       # the ..service.impl package target
because = "07 §3 Service 实现统一放 service.impl，与接口包分开"

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
- **Four things v3 has and v2 did not:** `self:`/`any:`, `exist`/`counts`,
  the `all*` quantifiers, and named targets reusable across rule blocks.

Two intents in that catalog still need a decision, both because they reach past a
single node:

- **Call-graph rules** — "adapter calls application only through transactional
  methods", "HTTP entry points check permissions". These need resolved call targets
  *and* the callee's annotations, i.e. a project call graph, not a dependency edge.
  They are out of scope for v3 and stay in [`ARCHITECTURE-RULES.md`](ARCHITECTURE-RULES.md) §5
  phase 5.
- **`assignableTo`-style type questions** — see §6.

## 8. Validation and baseline

`arch lint-config` reads the config without analysing code and reports:

- unknown target name in a rule or constraint;
- a selector that matches nothing in the project (`allowEmptyShould` in ArchUnit
  terms — a rule that silently checks nothing is worse than no rule);
- a location matched by two targets (ambiguous classification);
- an unused target, a target with no rule, a rule with an empty `must`;
- `self:` inside a target that cannot have a module owner.

`baseline` records today's violations by `rule id + target name + node`, so adoption
starts at zero friction while the debt list only shrinks. Hiding a new violation must
require editing the baseline file, which is reviewable.

## 9. Migration from v2

| v2 | v3 |
|---|---|
| `[kinds.x] name = "*-domain"` | `[targets] domain = { on = "class", in = "*-domain" }` (same idea, now typed) |
| `allow` / `allowSelf` / `allowOther` | one `dependOnlyOn` list with `self:` / `any:` prefixes |
| `deny = [...]` | `notDependOn = [...]` on the same target |
| `exemptFrom = [...]` on a rule | `except = [...]` node list, or `baseline` |
| `parent` / `children` / `counts` containment rule | `children` / `counts` constraints on `target = "module"` |
| `check = "package-root"` | `target = "package"` + `must = { in = ... }` (was a class constraint) |
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
   module. Useful for conventions, but it makes `self:` ambiguous (whose module?).
   Options: forbid crossing modules, resolve `self:` against each node's own module,
   or require an explicit `in`.
3. **Slices as a first-class concept.** A `{slot}` package pattern plus a cycle check
   gets the behaviour; whether to also surface `slices = { by = "...", of = "..." }`
   as sugar. Blocked behind decision 1 anyway, since slices are only useful with a
   cycle check.

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
| `module` | `dependOnlyOn`, `notDependOn`, `children`, `counts`, `acyclic` |
| `package` | `dependOnlyOn`, `notDependOn`, `visible` |
| `class` | `dependOnlyOn`, `notDependOn`, `annotatedWith`, `notAnnotatedWith`, `annotationArgument`, `extend`, `implement`, `exist`, `allMembers`, `allMethods`, `allFields` |
| `method` | `annotatedWith`, `notAnnotatedWith`, `rawReturnType`, `notRawReturnType`, `allParams`, `visibility` |
| `field` | `annotatedWith`, `notAnnotatedWith`, `type`, `notType` |

Notes that decide behaviour, not syntax:

- `dependOnlyOn` / `notDependOn` take **target names only**, optionally prefixed
  with `self:` or `any:`. A pattern string is not accepted, so there is exactly one
  matching semantics in the whole language.
- `visible` is the target-side form: who may depend on this package. Values are
  target names plus `self:`, `any:`, `none`; the default is `self:` (the same module),
  which matches what package-private access already enforces at the language level.
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

### A.4 Composition and relations

| Keyword | Meaning |
|---|---|
| `self:` | the same module as the source node |
| `any:` | any module (the default when no prefix is written) |
| `other:` | accepted alias for `any:`, used when a rule lists `self:` and `any:` together |
| `except` | node-level exemption list on a rule |
| `because` | the reason printed with every violation |
| `severity` | `error` (default) or `warn`, per rule block |
| `on` | `main` (default) or `test`, per rule block |

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
