# Nullness analysis: design proposal

Status: **proposal for review; no implementation yet** (2026-11-25).

This document proposes a nullness checker for Groovy based on first principles:
control-flow graphs, abstract interpretation, access paths, and method summaries.
It uses `groovy-nullchecker/nullchecker/corpus` only as a black-box corpus and
specification of cases; it deliberately does **not** use that project's source
code.

The central decision is:

> Build a flow-sensitive nullness analysis with a Kotlin-like type model:
> unannotated declared types are non-null, `@Nullable T` is the surface form of
> `T?`, and flow analysis supplies smart casts. Annotations are optional in a
> migration mode, but the target model is default-non-null. The checker must be
> explicit about where inference stops and where dynamic Groovy/external code
> makes soundness impossible.

## 1. What the corpus says

The corpus contains 96 Groovy files (95 manifest entries) under
`positive/`, `negative/`, `strict/`, `compile-static/`, and `pkg/`. The manifest
defines 41 rule ids and expected finding counts, and distinguishes:

- `strict=never`: must stay silent in both modes (negative corpus);
- `strict=only`: must fire only in strict mode (flow-sensitive cases);
- `strict=any`: same expectation in both modes.

The cases cluster into these capabilities:

| Capability | Corpus examples |
|---|---|
| Annotation contracts | `positive/nonnull_*`, `nullable_*`, `unbox_*` |
| Flow-sensitive locals | `strict/flow_local_null_deref`, `flow_reassigned_null`, `flow_branch_merge` |
| Branches, loops, switch | `flow_loop_switch`, `flow_switch_fallthrough` |
| try/catch/finally | `flow_try_catch` |
| Closures/captures | `flow_closure_capture`, `closure_return_null`, `closure_nested_return` |
| Access paths/fields | `flow_access_path`, `guarded_field_path`, `assert_field_path` |
| Collection/Map lookups | `container_lookup_*`, `element_*`, `guarded_*` |
| Optional | `optional_or_else` |
| Guard functions/contracts | `guards_*`, `validation_*` |
| Initialization | `nonnull_field_no_init`, `nonnull_field_partial_ctors` |
| Package defaults | `pkg/*` |

The strict corpus is especially valuable because it describes behavior when
nothing is annotated. The annotation corpus is useful as an optional contract
mode, not as the foundation of the analysis.

## 2. What the industry does

### Checker Framework Dataflow Framework

The Checker Framework's dataflow manual describes the architecture we should
follow:

- lower the AST into a control-flow graph of basic blocks and explicit nodes;
- run a forward/backward dataflow analysis over that graph;
- keep a **store** mapping expressions/access paths to abstract values;
- use **transfer functions** to model assignment, calls, branches, and casts;
- produce **then/else stores** for boolean expressions, so short-circuiting and
  null tests are precise;
- merge stores at control-flow joins;
- iterate to a fixed point, then answer queries from the result.

The framework is intra-procedural by design: each method is analyzed in
isolation, with summaries/contracts connecting methods. It models exceptional
edges and duplicates `finally` blocks so abrupt completion still runs them.
It treats method calls as potentially changing field facts unless the callee is
pure or has a postcondition. This matches the corpus cases
`flow_access_path` (a call invalidates a field guard) and
`guards_ensures_nonnull` (a contract preserves it).

Reference: [Checker Framework Dataflow Manual](https://checkerframework.org/manual/checker-framework-dataflow-manual.pdf).

### Checker Framework Nullness Checker

The Nullness Checker contributes these lessons:

- three orthogonal concerns: **nullness**, **initialization**, and **map keys**;
- the nullness lattice includes `@Nullable`, `@NonNull`, `@MonotonicNonNull`,
  and `@PolyNull`;
- flow-sensitive refinement applies from an assignment, a test, or a method
  postcondition, and is discarded when the value may change;
- method calls normally invalidate facts about fields;
- initialization is checked separately: a non-null field must be initialized on
  every constructor path;
- method annotations (`@RequiresNonNull`, `@EnsuresNonNull`,
  `@EnsuresNonNullIf`) express contracts that flow analysis cannot infer;
- map lookup precision needs a `containsKey`/`get` relationship or a key model.

Reference: [Checker Framework Manual, Nullness Checker](https://checkerframework.org/manual/#nullness-checker).

### NullAway

NullAway is an annotation-based, intra-procedural dataflow analysis optimized
for build-time feedback. It contributes:

- an abstract state is a map from **access paths** to nullness;
- access paths include locals, fields, and zero-argument getter calls;
- guards such as `x != null`, `this.next != null`, and
  `map.containsKey(k)` refine access paths;
- field initialization is checked at constructor exits;
- lambdas inherit the nullability of the functional-interface method;
- the tool deliberately assumes callees do not mutate state to keep the
  analysis fast and practical;
- library models fill in unannotated third-party APIs.

Reference: [How NullAway Works](https://github.com/uber/NullAway/wiki/How-NullAway-Works).

The important lesson is not to copy NullAway's assumptions. It is to make the
assumptions explicit and configurable.

## 3. Design principles

1. **Inference first, annotations optional.**
   The analysis should work on unannotated Groovy. Declarations, when present,
   are additional facts and validation constraints.
2. **Definite facts before possible facts.**
   Conservative mode reports only nullness that is provable from the program
   text. Strict mode reports possible-null and inferred-contract violations.
3. **No false soundness claim.**
   Dynamic dispatch, `method_missing`, metaclass changes, reflection, and
   unmodelled external libraries make whole-program soundness impossible.
4. **Flow-sensitive, path-sensitive where it matters.**
   Null checks, short-circuiting, early exits, loops, switch, try/finally, and
   closures all affect the state.
5. **Access paths, not just variables.**
   `this.value`, `x.field`, `map.get(key)`, and repeated getters must refine.
6. **Invalidate on change.**
   Assignments, unknown calls, escaping closures, and concurrency invalidate
   facts that are no longer guaranteed.
7. **Summaries connect methods.**
   Each method has return/parameter/effect summaries. Unknown external calls
   remain `Unknown`; they do not silently become non-null.
8. **A finding must be explainable.**
   Every diagnostic should show the path/guard/contract that was (or was not)
   established, with source spans.
9. **Performance is part of the design.**
   Analyze per method, cap access-path depth, cache summaries, and avoid
   whole-program fixpoints where possible.

## 3.1 Kotlin-like `T` / `T?` model

The target model can be described as:

```text
T       non-null
T?      nullable
T <: T?
T? !<: T
null <: T?        (and null !<: T)
```

In Groovy source, `T?` is written as `@Nullable T` (or the project's chosen
nullable annotation). We are not adding a new type syntax to Groovy; we are
giving an existing annotation a precise type-system meaning.

### Where the type system acts

The type system checks declarations and boundaries:

- a value of type `T?` cannot be assigned to a location of type `T`;
- `null` cannot be assigned to `T`;
- a `T?` argument cannot be passed to a `T` parameter;
- a `T?` value cannot be returned from a `T` method;
- a `T` value is assignable to `T?`;
- overriding methods must preserve or strengthen the contract (return `T` may
  become `T?` only if the caller contract permits it; parameters follow the
  usual contravariance rule);
- non-null fields must be initialized on every constructor path;
- unboxing a `T?` wrapper to a primitive is an error.

### Where dataflow acts

The type system alone is not enough. Kotlin-style smart casts are dataflow:

```groovy
@Nullable String x = maybe()
x.length()            // type error: String? has no length()
if (x != null) {
    x.length()        // smart cast to String
}
x = 'ok'
x.length()            // smart cast after assignment
```

The store maps access paths to **refined types** (`T` or `T?`), not merely
booleans. A branch join computes the least upper bound of refined types:

- `T` join `T` = `T`;
- `T` join `T?` = `T?`;
- `T?` join `T?` = `T?`;
- `Null` join `T?` = `T?`;
- `Unknown` join anything = `Unknown`.

A smart cast is revoked when the underlying location may change:

- assignment to the variable/path;
- a call that may mutate the receiver's fields;
- a closure escaping or being invoked by unknown code;
- concurrency mode without monotonic fields.

### Unannotated declarations and `def`

Target policy for a `@NullMarked` scope or a project that enables
`default = nonnull`:

- `String` parameter, return, field, local type -> `String` (non-null);
- `@Nullable String` -> `String?`;
- `def x = 'a'` -> inferred `String`;
- `def x = null` -> inferred `Nothing?`/`Object?` (nullable);
- `def x = maybeNull()` -> inferred return type, possibly `T?`;
- untyped method parameters (`def`) are `Unknown`/platform-like unless a call-site
  or body analysis provides evidence.

This creates an important distinction:

- **Declared typed code** is Kotlin-like and checked strictly.
- **Dynamic/untyped Groovy** remains partially unknown; it must not be silently
  treated as non-null.

### External and legacy code: platform types

Unannotated Java/Groovy dependencies are not automatically non-null. Model them
as a Kotlin-like **platform type** `T!`:

- assignable to both `T` and `T?`, with a configurable warning;
- dereference allowed, but flagged in a stricter mode;
- actual nullability remains `Unknown` until a library model/contract is
  supplied.

`@NullUnmarked`/legacy scopes use the same platform-like treatment. This is the
migration escape hatch; a global default-non-null rule applied to every
dependency would produce a flood of false positives.

### What this means for defaults

There are now two distinct policies:

1. **Type policy**: what a declaration means (`T` vs `T?`), and whether the
   default is non-null.
2. **Reporting policy**: whether a possible-null dereference is reported as an
   error, a warning, or not at all.

The recommended target is:

- `kotlin`/`strict`: default non-null in marked scopes; type mismatches and
  smart-cast failures are errors; possible-null dereferences are warnings/errors.
- `inferred`: infer local/return nullness for migration; no project-wide
  default-non-null assumption.
- `conservative`: only definite null; used until a project is ready for the
  Kotlin-like migration.

### Kotlin parity gaps

We can share the core semantics but should not claim full Kotlin parity:

- Kotlin has `?` in the language syntax; Groovy uses annotations.
- Kotlin inserts runtime parameter/return checks; a static analyzer does not.
- Groovy dynamic dispatch, `method_missing`, metaclasses, and reflection remain
  outside the type system.
- Generics, arrays, and variance need separate rules; `List<String>` vs
  `List<String?>` is not fully Kotlin-like at the first milestone.
- Kotlin's platform types and Java interop are richer than a source-only
  analyzer; we can approximate them with `Unknown`.

### Corpus implications

The corpus mixes two expectations:

- annotation fixtures already match a `T`/`T?` model;
- strict flow fixtures use `String x = null` and expect a dereference finding,
  which under default-non-null would also be a type error at the declaration.

Do not change the corpus. Run it under multiple profiles:

- `inferred` profile: reproduce the corpus's flow expectations;
- `kotlin` profile: add type-mismatch findings and adjust expected counts per
  fixture;
- `annotated` profile: validate contracts and package defaults.

This keeps the corpus as a semantic oracle while allowing the stricter target
model.

## 4. Analysis architecture

```text
semantic AST
  -> lowering to analysis IR
      -> one CFG per method / constructor / initializer / closure / script
  -> project index and call graph
  -> intraprocedural dataflow
      -> store of access path -> nullness
      -> then/else stores for branches
  -> method summaries (fixpoint over call graph)
  -> diagnostics
  -> optional incremental caches
```

The analysis belongs in `groovylint-analysis`, not in a CodeNarc rule module.
It consumes the semantic AST and a small project index; it must not depend on
`groovylint-rules` or the CLI.

### 4.1 Analysis IR

The semantic AST is convenient for rules but awkward for dataflow. Lower it to
an analysis IR with:

- explicit temporaries for nested expressions;
- explicit evaluation order;
- explicit reads/writes of variables and fields;
- explicit calls, `new`, casts, unboxing, and type tests;
- explicit control-flow nodes for branch, switch, return, throw, break, and
  continue;
- source spans on every node.

Proposed function unit:

```rust
struct AnalysisUnit {
    kind: UnitKind,             // method, constructor, initializer, closure, script
    params: Vec<ParamId>,
    locals: Vec<LocalId>,
    body: Cfg,
    captures: Vec<Capture>,
    span: Span,
}
```

### 4.2 CFG

Use basic blocks with explicit successors:

- entry, normal exit, exceptional exit;
- conditional block with then/else successors;
- exception edges for calls, field/index access, casts, `throw`, and `new`;
- `finally` duplication for all abrupt exits;
- `switch` cases with fall-through;
- loops with zero-iteration and back edges;
- closures as separate CFGs, invoked through summaries.

The CFG must be reusable across analyses; do not put nullness values directly
in CFG nodes.

### 4.3 Dataflow solver

A generic forward worklist:

```text
entry state
repeat:
  pop block
  transfer nodes
  join at successors
until no state changes
```

A state contains:

- a **store** of access paths to nullness values;
- an initialization store for fields in constructors;
- a set of facts invalidated by the current path.

Join is the least upper bound of the nullness lattice and intersection of
definite facts. Stores are immutable/persistent or cheaply cloned.

Boolean expressions produce **conditional stores**. A null comparison refines
the then and else stores; `&&`/`||` use the precise short-circuit propagation
rules from the Checker dataflow manual.

## 5. Nullness lattice and stores

### 5.1 Nullness lattice

```text
                 Unknown
                    |
                MaybeNull
                 /     \
              Null    NonNull
                 \     /
               Unreachable
```

- `Unreachable`: no path reaches this point (bottom).
- `Null`: definitely null.
- `NonNull`: definitely not null.
- `MaybeNull`: at least one path may be null.
- `Unknown`: no usable information, typically an unresolved external call or a
  dynamic receiver.

Join (`⊔`):

| Join | Null | NonNull | MaybeNull | Unknown |
|---|---|---|---|---|
| Null | Null | MaybeNull | MaybeNull | Unknown |
| NonNull | MaybeNull | NonNull | MaybeNull | Unknown |
| MaybeNull | MaybeNull | MaybeNull | MaybeNull | Unknown |
| Unknown | Unknown | Unknown | Unknown | Unknown |

This lattice is deliberately simple. It can be refined later with:

- `MonotonicNonNull` (nullable until first non-null write);
- `PolyNull` (a value whose nullness depends on a call argument);
- generic type arguments and element nullness.

Those refinements should not be required for the first implementation.

### 5.2 Store and access paths

The store maps **access paths** to nullness:

```text
x
this
this.value
x.field
x.getField()
map.get(key)
list[i]          (optional, later)
```

Access paths are canonicalized, length-capped (initially 2–3 segments), and
invalidated conservatively.

Rules:

- local variable assignment updates/removes the local path;
- field assignment updates `base.field` and aliases where possible;
- `y = x` copies facts for `x` to `y`; later mutation of either invalidates
  aliases;
- unknown method calls invalidate field paths rooted at `this` and at values
  reachable from arguments unless the callee has a pure/no-side-effect model;
- a call cannot change a local variable's identity, but it can change the
  contents of an object a local points to.

### 5.3 Side effects and purity

Default behavior:

- local variables survive calls;
- fields are invalidated by calls;
- a contract `@EnsuresNonNull("this.value")` or a configured library model can
  preserve a field fact;
- a `@SideEffectFree`/pure call can preserve facts if the project declares it;
- closures invoked by unknown code invalidate captured field facts.

The first implementation may use a conservative "unknown calls invalidate all
field paths" rule. That is slower for analysis precision but safer than
NullAway's assumption that callees do not mutate.

## 6. Transfer functions

### 6.1 Expressions and assignments

- `null` -> `Null`.
- literal values, `new`, collections, closures -> `NonNull`.
- `x` -> store lookup, or the declared/inferred type if absent.
- `x = e` -> evaluate `e`, write to `x`, invalidate aliases of `x`.
- `this.f = e` -> write `this.f`, invalidate aliases and paths that may observe
  the field.
- `x as T` -> if `T` is primitive/wrapper, model the cast; otherwise preserve.
- unboxing -> report if the operand may be null (wrapper -> primitive).
- string concatenation/GString -> null contributes `"null"`, not an NPE; do not
  report a dereference for interpolation itself.

### 6.2 Branches and booleans

- `x == null`:
  - then store: `x = Null`;
  - else store: `x = NonNull`.
- `x != null`: inverse.
- `x` in boolean context:
  - then store: `x = NonNull` (Groovy truth implies non-null);
  - else store: no nullness refinement (false can mean null, empty, zero, or
    false).
- `!x`: the true branch does not imply null; it only rejects truthiness.
- `x instanceof T`: then branch `NonNull`; else unchanged.
- `x?.f`: receiver may be null; result `MaybeNull`; do not report on the
  receiver.
- `x ?: y`: then branch is `x` when truthy (NonNull); else branch is `y`;
  result joins both.
- `x ? y : z`: join of branch values.
- `&&`, `||`: preserve then/else facts using short-circuit flow rules.
- `assert`: Groovy assertions execute in Groovy; refine the then path. A helper
  call only refines if its contract is known.

### 6.3 Loops and switch

- `while`: zero-iteration path and back edge; join entry and body states.
- `do/while`: body at least once.
- classic `for`: init, zero-or-more condition, update.
- `for-each`: zero-or-more iterations; element nullness comes from the
  collection's element type or model.
- `switch`: zero or one matching case; fall-through reaches the next case;
  merge all reachable exits.
- `break`/`continue`: edges to the loop/switch targets, with `finally` blocks
  executed on the way.

### 6.4 try/catch/finally

- exceptional edges from potentially throwing nodes;
- catch entry joins all exceptional paths for matching types;
- finally is duplicated on normal and abrupt exits;
- a `return`/`throw`/`break`/`continue` inside `try` runs `finally` first;
- after `finally`, the outgoing state is the join of all completed paths.

This directly addresses `strict/flow_try_catch` and the corpus's finally cases.

## 7. Groovy-specific semantics

### 7.1 Truthiness

Groovy truth is not Java truth. `false`, `null`, zero, empty strings, empty
collections, and empty maps are falsey. For nullness:

- truthy => `NonNull`;
- falsey does **not** imply `Null`; preserve `MaybeNull`/`Unknown`.

This matters for `guards_groovy_truth` and for avoiding false positives on
`if (x)`.

### 7.2 Safe navigation, Elvis, and GStrings

- `a?.b` returns null if `a` is null; the result is `MaybeNull`.
- `a ?: b` returns `a` when truthy, else `b`.
- `"${x}"` calls Groovy string conversion; null becomes `"null"` rather than
  throwing. Do not flag interpolation as a null dereference.
- Property access and method calls on a null receiver are dereferences.

### 7.3 Dynamic dispatch

Groovy calls are dynamic by default. The analysis can:

- resolve calls when the receiver's type is known and the project index has a
  unique method;
- otherwise use a name/arity summary and treat the result as `Unknown`;
- never assume an unannotated external method returns non-null in conservative
  mode.

`@CompileStatic`/`@TypeChecked` can improve resolution, but the checker should
not require them.

### 7.4 Closures

Lower each closure to its own analysis unit:

- parameters/`it` have inferred or declared nullness;
- captured variables become explicit capture cells;
- direct local invocation uses the closure summary;
- invocation by unknown code may execute zero or more times, joining the
  pre-call state with the post-call state;
- an escaping closure's return/effects are not trusted across unknown calls;
- a closure `return` exits the closure, not the enclosing method.

This covers `closure_nested_return`, `closure_escaped_return`,
`flow_closure_capture`, and `closure_return_null`.

### 7.5 Optional and collections

These are not special syntax, but need small library models:

- `Optional.get()`/`orElseThrow()` -> non-null (when present);
- `Optional.orElse(null)` -> nullable;
- `Stream.findFirst().orElse(null)` -> nullable;
- `Map.get`, subscript, `first`, `head`, `max` -> nullable unless guarded;
- `Map.containsKey(k)` refines the corresponding `get(k)` on the same map/key;
- `List.get(i)` is nullable if the element type is nullable; bounds are a
  separate analysis;
- spread operations produce collections; `x*.f` does not dereference `x`
  unsafely unless the collection itself is null.

### 7.6 Fields and initialization

Strict mode adds a separate initialization analysis:

- non-null fields must be initialized in every constructor path, or have an
  initializer;
- `this.field` reads before initialization are errors;
- helper methods can establish `@EnsuresNonNull` (optional contract mode);
- `@MonotonicNonNull` allows lazy initialization, but not re-nulling;
- static fields are checked with the same rule, with a note about lifecycle.

## 8. Interprocedural inference

### 8.1 Method summaries

For each project method:

```rust
struct MethodSummary {
    return_nullness: Nullness,
    params: Vec<ParamContract>,       // required non-null, nullable, unknown
    field_effects: Vec<AccessPath>,   // fields potentially invalidated
    never_returns: bool,
    is_pure: bool,
}
```

Inference:

- return: join all explicit `return` values and the implicit last expression
  (Groovy methods return their last expression);
- parameters: a parameter dereferenced before a guard must be non-null; a
  parameter that receives null or is returned as null may be nullable;
- effects: assignments to fields and calls to unknown/effectful methods;
- `never_returns`: calls that always throw, e.g. `fail()` models;
- purity: only trusted annotations/models, not guessed from names.

Closures produce analogous summaries with capture effects.

### 8.2 Call graph and fixpoint

- Start with direct project calls whose receiver type/method is resolvable.
- For dynamic dispatch, union summaries of possible overrides or mark Unknown.
- Analyze summaries to a fixed point; start recursive/unknown summaries at
  `Unknown` to avoid unsound non-null conclusions.
- External/unresolved calls are `Unknown`, with optional library models.

### 8.3 Default assumptions (important review point)

The corpus has both annotation-based and unannotated flow cases. The proposal
is to ship two modes:

| Mode | Defaults | Reports |
|---|---|---|
| `conservative` | locals inferred; project/API boundaries `Unknown` | definite-null only |
| `inferred`/strict | project methods inferred; no global non-null assumption | possible-null and contract violations |
| `kotlin` | declared `T` is non-null; `@Nullable T` is `T?`; smart casts from dataflow | type mismatches, smart-cast failures, initialization |
| `annotated` | annotations/contracts are authoritative; unmarked code follows a configured default | declared-contract violations + flow |

Conservative mode should be the default for CI. Strict mode is opt-in until its
false-positive rate is measured. `assumeProjectNonNull` must be explicit and
must not apply to external libraries.

## 9. Optional annotations and contracts

Annotations are not required, but if the corpus and real projects have them,
the checker should read them. Match by simple name, not by a compile-time
dependency:

- `Nullable`, `NonNull`, `NullMarked`, `NullUnmarked`, `MonotonicNonNull`;
- `EnsuresNonNull`, `EnsuresNonNullIf`, `RequiresNonNull`;
- `Contract` (at least `null -> fail`, `!null -> !null`, `_-> fail`);
- `NoReturn`;
- Bean Validation `NotNull`, `NotBlank`, `NotEmpty`;
- JSR-305/Spring package defaults.

This requires syntax support for type-use annotations and local-variable
annotations. The current parser records declaration annotations but not full
type-use annotations or `package-info.java`; that is a prerequisite for the
annotation corpus, not for the first inferred-flow milestone.

Annotation mode should be developed after the inferred analysis is stable, so
the analysis does not accidentally depend on annotation availability.

## 10. Diagnostics and integration

Proposed diagnostic codes:

- `NullDereference` — receiver/field/index is definitely null;
- `PossibleNullDereference` — may be null in strict mode;
- `NullAssignment` — null assigned to a non-null location;
- `NullReturn` — null returned from a non-null method;
- `NullArgument` — null passed to a non-null parameter;
- `UnboxingNull` — nullable wrapper converted to primitive;
- `FieldNotInitialized` — non-null field not initialized on all paths;
- `RedundantNullCheck` — optional lint, not enabled by default.

Each diagnostic includes:

- rule code;
- path (receiver, field, argument);
- evidence: guard/contract/summary that established or failed to establish
  non-nullness;
- source span;
- optional remediation hint (add guard, add contract, return empty).

Integration:

```bash
groovylint nullcheck [paths]
groovylint nullcheck --mode conservative|inferred|annotated .
groovylint nullcheck --format json .
```

The Gradle plugin gets `groovylintNullnessMain`, `groovylintNullnessTest`, and
an aggregate task. Do not attach nullness to `check` until the false-positive
rate is acceptable; make it opt-in initially.

## 11. Corpus harness and test strategy

Use the corpus only as fixtures, never as implementation:

1. Parse `MANIFEST.txt` into fixture metadata (rule, strict, expected count).
2. Run the checker in the matching mode.
3. Compare finding counts, not just presence.
4. Split the gate:
   - **N2 gate:** strict flow fixtures (`strict/*`) plus flow negatives;
   - **N4 gate:** unannotated positive/negative cases;
   - **N5 gate:** annotation/contract/package cases.
5. Add a “no false positive” negative gate.
6. Add mutation fixtures for every transfer-function branch.
7. Run on a real Groovy corpus and record false positives/negatives and time.

The corpus's `strict=only` and `strict=never` metadata gives us a strong
regression contract. A fixture is not considered supported just because it
parses; the expected count must match.

## 12. Performance and incremental analysis

- Build CFGs per method on demand.
- Analyze methods in parallel; summaries use a deterministic worklist.
- Cache method summaries keyed by method source hash, signature, callee
  summary keys, and analyzer version.
- Cache file/CFG results separately; invalidate the reverse call graph when a
  summary changes.
- Cap access-path depth and alias sets.
- Avoid SMT/whole-program points-to analysis in the first version.
- Benchmark cold/warm/one-file-change on the corpus and a large project.

## 13. Phased implementation plan

### N0 — Design review (this document)

No code. Review defaults, scope, diagnostics, and corpus split.

### N1 — Analysis IR, CFG, corpus harness (2–3 weeks)

- Lower methods/constructors/closures/scripts.
- CFG with branches, loops, switch fallthrough, try/catch/finally, exceptions.
- Corpus manifest parser and count-based harness.
- CFG visualization for debugging.

### N2 — Conservative intraprocedural nullness (3–4 weeks)

- Nullness lattice and store.
- Assignments, literals, calls, properties, safe navigation, Elvis, truthiness.
- Guard refinement, short-circuiting, loops, switch, try/finally.
- Definite-null dereference only.
- Gate: `strict/*` flow cases where no annotations are required; negative flow
  cases stay silent.

### N3 — Access paths, fields, calls, library models (3–4 weeks)

- Access paths and alias invalidation.
- Field reads/writes and call side effects.
- Map/Optional/collection models.
- Gate: `flow_access_path`, `assert_field_path`, container/element fixtures.

### N4 — Interprocedural summaries and inferred mode (4–6 weeks)

- Return/parameter/effect summaries; call graph.
- Implicit returns; recursive fixpoint.
- Direct local closures; escaping closures.
- Strict inferred mode with configurable defaults.
- Gate: unannotated positive/negative corpus where possible.

### N5 — Optional annotations/contracts (3–4 weeks)

- Type-use/local annotations, package-info, contracts.
- Annotation/Spring/JSR-305/Bean Validation models.
- Gate: annotation corpus and package-default fixtures.

### N6 — Initialization, hardening, incremental (3–4 weeks)

- Non-null field initialization and read-before-init.
- Strict-mode possible-null tuning.
- False-positive corpus, performance, summary caching.
- Decide whether to attach to `check`/Gradle.

## 14. Non-goals and limits

- No soundness claim for dynamic Groovy, reflection, metaclass changes, or
  external libraries.
- No full type checker; only enough type information for nullness.
- No whole-program alias/points-to analysis in the first version.
- No annotations required for the first flow milestone.
- No autofix in the first nullness release; suggestions only.
- No architecture/ArchUnit work in this plan.

## 15. Review questions

1. **Default mode:** conservative (definite only) for CI, or inferred strict by
   default? Recommendation: conservative default, strict opt-in.
2. **Unannotated project defaults:** should project methods be assumed non-null
   when no evidence contradicts it? Recommendation: only in explicit inferred
   mode; never for external libraries.
3. **Annotations:** do we invest in the annotation corpus now, or after the
   inferred flow analysis? Recommendation: after N2/N4; keep the parser work
   isolated.
4. **Dynamic dispatch:** union overrides, or `Unknown` for anything not
   statically resolvable? Recommendation: union when the project index is
   closed; `Unknown` otherwise.
5. **Field invalidation:** invalidate all fields after unknown calls
   (conservative) or assume no mutation (NullAway-style, fast but unsound)?
   Recommendation: invalidate by default, with a pinned opt-in assumption.
6. **Closures:** trust direct local closure returns only; treat escaping
   closures as unknown. Confirm.
7. **Concurrency:** default sequential semantics; add `--concurrent` that
   invalidates fields. Confirm.
8. **Corpus gate:** accept count-based expectations and split annotation vs
   inferred vs Kotlin-profile fixtures. Confirm.
9. **Diagnostic ownership:** separate `nullcheck` command/task, not attached to
   `check` initially. Confirm.
10. **Default-non-null scope:** whole project, `@NullMarked` packages/classes, or
    a config-driven list? Recommendation: start with config/`@NullMarked`, then
    widen after a migration baseline.
11. **Untyped `def` policy:** infer from initializer/body, or treat as
    platform/`Unknown`? Recommendation: infer locals/returns; untyped parameters
    stay platform-like.
12. **Platform-type warning:** should assigning `T!` to `T` warn by default?
    Recommendation: warn in kotlin mode, silent in migration mode.

## 16. References

- [Checker Framework Dataflow Framework manual](https://checkerframework.org/manual/checker-framework-dataflow-manual.pdf)
- [Checker Framework Nullness Checker](https://checkerframework.org/manual/#nullness-checker)
- [NullAway: How it works](https://github.com/uber/NullAway/wiki/How-NullAway-Works)
- [NullAway README](https://github.com/uber/NullAway)
- [JSpecify user guide](https://jspecify.dev/docs/user-guide/)
- [Groovy operators: safe navigation, Elvis](https://groovy-lang.org/operators.html)
- [Groovy semantics: truth](https://groovy-lang.org/semantics.html)
