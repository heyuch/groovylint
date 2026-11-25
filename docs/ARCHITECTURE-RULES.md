# Architecture rules: proposal

Status: **proposal for review** (design only, nothing implemented).

`saas-service-giis247` encodes its architecture as 40-odd ArchUnit tests
(`docs/archunit-rule-catalog.md`). This document proposes how `groovylint` should
implement the same constraints natively, with a declarative rule model reusable
across projects, and where source-level analysis beats bytecode.

This fills in milestone **M4** and the DSL sketch in [`ROADMAP.md`](../ROADMAP.md)
§5; it does not change the roadmap's milestones.

## 1. What the catalog actually needs

Reading the catalog rule by rule, the constraints fall into five families. Each
needs a different amount of machinery, and the split drives the phasing in §5.

| Family | Rules (catalog) | Needs |
|---|---|---|
| **Placement** | `clientImplementationsLiveInTheClientPackage`, `useCasesDoNotLiveInTheClientOrImplPackage`, `serviceImplementationsLiveInTheImplPackage` | declaration name pattern + own package. **Per file.** |
| **Annotation presence / attributes** | `everyApiMethodDeclaresSecurity`, `everyApiClassDeclaresScope`, `legacyScopeAnnotationsAreNotUsed`, `domainMustNotDeclareTransactions`, `domainServicesAreSpringComponents`, `repositoryImplementationsAreSpringRepositories`, `mapStructConvertersUseTheSharedConfig`, `mybatisMapperParametersAreExplicitlyNamed` | annotations on classes/methods/parameters, annotation arguments (`@Mapper(config = …)`), method signatures. **Per file.** |
| **Type shape** | `businessEnumsImplementBaseEnum`, `repository{Query,Get,Load}…`, `repositoryLoadMethodsAreNeverNullable`, `persistenceModelsDeclareTheirTableAndTenant`, `infrastructureImplementationClassesImplementADomainContract` | own signatures + (one case) "is this interface declared in this module's domain". **Per file + index.** |
| **Dependency direction** | `adapterMustRespectModuleDependencies`, `applicationMustRespectModuleDependencies`, `clientMustNotDependOnAnyImplementationLayer`, `domainMustRespectModuleDependencies`, `infrastructureMustRespectModuleDependencies`, `modelsMustNotDependOnAnyImplementationLayer`, `domainMustNotDependOnWebOrPersistenceFrameworks`, `clientContractsMustNotDependOnSpring`, `clientContractsMustNotReadRuntimeContext`, `onlyRepositoriesReadRuntimeContext`, `applicationMustNotUseBeanLookup`, `domainServiceMustNotUseBeanLookup` | imports, field/param/return/local types, `extends`/`implements`, `new`, class literals, static call targets, **name resolution + project index**. |
| **Call-graph** | `adapterCallsApplicationThroughTransactionalMethods`, `applicationUseCasesCalledFromHttpCheckPermissions` | resolved call targets **and the callee's body** (annotations on the callee, or "references `Panic`/`PermissionCodes`/`*Access`"). |

Two consequences:

1. Two thirds of the catalog (families 1–3) is per-file and can ship without any
   project graph. That is a cheap, high-value first release.
2. The remaining third is where the real work is: name resolution, a project
   index, and a dependency graph.

## 2. Source-level vs bytecode: where we differ from ArchUnit

We analyse `.groovy` sources, ArchUnit analyses compiled classes. Both directions
of the difference matter and should be explicit in the rule model.

| Concern | ArchUnit | groovylint (source) |
|---|---|---|
| `@CompileStatic` as a constraint | impossible (SOURCE retention) | visible — the catalog currently delegates this to CodeNarc strict; we can fold it in |
| Interface `default` methods => `Xxx$Trait$Helper` | must be excluded by name | does not exist |
| MapStruct / generated `.java` classes | visible, annotations unreadable | invisible (we do not parse `.java`), which is what the catalog emulates with "source file is `.groovy`" |
| Dynamic Groovy calls | only static call targets are visible | declarations and dynamic call names are visible; resolution is best-effort |
| Unused imports | edges still exist unless the rule says otherwise | the same — the model must state whether imports count as dependencies (they do for `applicationMustNotDependOnMyBatis`, they should not for a "no wildcard" style rule) |
| Unresolved external types | imported class, package known | keep as a symbolic name (M4 wording: "unresolved external references retained as symbolic names") |

The catalog's "已知限制" section says Groovy dynamic calls can bypass ArchUnit.
With source analysis we can do better, but only if the dependency extraction
records *declared* types even when the call itself is dynamic. That is a
deliberate, documented behaviour difference.

## 3. Rule model

One declarative model, two front ends (TOML for the CLI, a Gradle DSL facade that
produces the same model). The model has four parts: **slices**, **selectors**,
**conditions**, **reporting**.

### 3.1 Slices

A slice is a named set of files/packages/classes used on both sides of a rule.
Project layouts differ (`<module>-<layer>/*` in `saas-service`, single-module
elsewhere), so a slice is defined by any combination of paths and packages:

```toml
[architecture.slices.adapter]
paths = ["*/adapter/**", "*/adapter-shared/**"]
packages = ["com.bytesforce.adapter.."]

[architecture.slices.application]
packages = ["com.bytesforce.application.."]

[architecture.allow.adapter]
domain-enums = "com.bytesforce.domain.*.*Enum"   # class whitelist inside a rule
```

`module = "first path segment"` is the default slice axis (that is what the
catalog's per-module tests encode), so `paths` handles it without extra concepts.

### 3.2 Selectors

ArchUnit's `classes()`, `methods()`, `fields()` become selector keys. Only what
the catalog needs, plus room to grow:

| Selector | Meaning |
|---|---|
| `classes` | class name pattern (`com.acme..*Repository`) |
| `annotatedWith` / `notAnnotatedWith` | annotation simple or FQ name |
| `implement` / `extend` | direct supertype |
| `resideIn` | package pattern |
| `methods` | method selector: `name`, `annotatedWith`, `returnType`, `parameters`, `parameterAnnotatedWith`, `isPublic`, `isStatic` |
| `fields` | field selector: `name`, `type`, `annotatedWith` |
| `sourceSets` | `main` / `test` (test sources excluded by default) |

Patterns use ArchUnit semantics: `..` = any subpackages, `*` = any characters
within a segment, `(*)`/`(..)` for parameters.

### 3.3 Conditions

```toml
[[architecture.rules]]
id = "domain-must-not-declare-transactions"
family = "annotation"
selector.classes = "com.bytesforce.domain..*"
selector.sourceSets = ["main"]
condition.mustNotBeAnnotatedWith = "org.springframework.transaction.annotation.Transactional"
condition.appliesTo.methods = "*"        # also check method-level annotations
because = "07: transaction boundaries live in application use cases"
```

Condition vocabulary for v1 (each maps onto a catalog rule):

| Condition | Used by |
|---|---|
| `mustDependOnlyOn`, `mustNotDependOn`, `mustNotDependOnPackages` | all `*MustRespectModuleDependencies`, `*MustNotDependOnSpring/MyBatis/Web…` |
| `mustResideIn` | the three placement rules |
| `mustBeAnnotatedWith`, `mustNotBeAnnotatedWith` | API security/scope, `@Transactional`, `@Service`, `@Repository` |
| `mustImplement`, `mustNotImplement` | `businessEnumsImplementBaseEnum`, `infrastructureImplementationClassesImplementADomainContract` |
| `mustHaveAnnotationMatching` (annotation + attribute) | `mapStructConvertersUseTheSharedConfig` (`config = MapperConfigWithoutMetaClass`) |
| `methodReturnType` (`Page<T>`, collection, `Optional`, `@Nullable`) | the `repository*` rules |
| `mustHaveField` / `mustImplement` combination | `persistenceModelsDeclareTheirTableAndTenant` |
| `parameterAnnotatedWith` | `mybatisMapperParametersAreExplicitlyNamed` |
| `callersOf` (call target's callee must satisfy …) | `adapterCallsApplicationThroughTransactionalMethods`, `applicationUseCasesCalledFromHttpCheckPermissions` |

### 3.4 Reporting and baselines

Every rule carries `because` (copied into the diagnostic, like the catalog's
`.because(...)`), `severity`, `allowEmptyShould` (ArchUnit's
`allowEmptyShould(true)`), and `exclude` lists for legacy baselines:

```toml
[[architecture.rules]]
id = "output-domain-no-transactions"
exclude.classes = ["com.bytesforce.domain.output.service.impl.OutputServiceImpl", "…"]
exclude.reason = "LEGACY_TRANSACTIONAL_CLASSES: 存量迁 application 前放行"
```

Exclusions live in config, not in code, so the baseline review is a diff of one
file, and a new violation cannot be silenced by widening a name pattern. Report
format stays the tool's existing diagnostic shape, so `--format json` and the
SARIF-ish consumers keep working; the `rule` field is the rule `id`.

## 4. Execution

```
groovylint arch --config architecture.toml [paths]
  1. discover files, parse (parallel)
  2. build ProjectIndex: declared types, packages, annotations, supertypes,
     member signatures, per-file dependency edges (with resolution state)
  3. per-file rules (families 1-3) run against the parsed AST + index
  4. project rules (families 4-5) run over the index: slice membership,
     dependency edges (ignoring self-edges and duplicates), call targets
  5. sort diagnostics by (file, line, rule) and emit
```

Determinism: index built from a sorted file list; edges as a set; diagnostics
sorted, so parallel parsing cannot reorder output.

Performance target: the graph for 24k files should stay in the low seconds. Our
parse-only baseline is 0.05 s per 838 files (single core), so extraction, not
parsing, is the cost; the M4 exit criterion ("10k-class synthetic project builds
its graph in reasonable time") should be turned into a benchmark early.

## 5. Phasing

| Phase | Content | Why first |
|---|---|---|
| **A** | Per-file families 1–3 (placement, annotations + attributes, type shape) with selectors `classes`/`methods`/`fields`/`annotatedWith`/`implement` | No graph. Covers ~24 of the catalog's 40 rules, and they are the ones that fail most often today. |
| **B** | `ProjectIndex` + dependency extraction + `mustDependOnlyOn` / `mustNotDependOn` per slice; module/layer rules; `exclude` baselines | The module-boundary rules are the catalog's core value. |
| **C** | Call-graph conditions (`callersOf` with callee annotation/body requirements), `runtimeContext` specialisations, `allowEmptyShould` tuning | Fewest rules, highest resolution cost. |
| **D** | Gradle DSL facade emitting the same TOML model; migration helper that prints a config skeleton per module | Needed before the `saas-service` switch-over. |

## 6. Decisions to settle before implementation

1. **Dependency semantics for imports.** `applicationMustNotDependOnMyBatis` wants
   imports counted; a hypothetical "unused type" rule does not. Proposal: edges
   from *type references* (fields, params, returns, locals, `new`, `extends`,
   `implements`, static calls) by default, with `includeImports = true` as a rule
   option, and imports always recorded in the index for rules that ask.
2. **Module determination** — first path segment (catalog behaviour) with an
   optional `moduleDepth` / `moduleRegex`.
3. **Where `arch` runs** — separate subcommand (proposal) versus folding into
   `check` as project rules; separate keeps `check` fast and per-file.
4. **Failure mode for unresolved references.** Proposal: unresolved edges are kept
   as symbolic names and *do* participate in package-prefix rules (that is how a
   missing dependency usually looks), but rules can require `resolvedOnly`.
5. **Gradle plugin parity** — the plugin currently forwards `--mode`/`--contracts`
   only; the DSL facade (§5 D) needs a task that runs `arch` and maps rule ids to
   Gradle task inputs, so a per-module run can pass `--slice`.
6. **Do we generate the TOML from the existing ArchUnit tests?** Tempting for the
   migration, but the two rule languages differ enough that a generated skeleton
   may be worse than the catalog's tables. Cheap alternative: a one-off script that
   prints, for each catalog row, the matching rule template.

## 7. What this is not

- Not bytecode-exact ArchUnit parity. We read sources; classes whose dependencies
  only exist after compilation (generated code, traits) behave differently, and
  that is documented rather than emulated.
- Not an arbitrary-Groovy DSL. The Gradle facade is declarative and lowers to the
  same model; it does not execute user code.
- Not a replacement for CodeNarc naming/style rules; those stay where they are.
