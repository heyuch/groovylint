# CodeNarc 4.0.0 coverage

Baseline: CodeNarc 4.0.0, 24 rulesets, **406 concrete rule classes**.

Current status:

| Status | Count |
|---|---:|
| Unique registered rule classes | 384 |
| Unsupported (Grails/Jenkins/Spock) | 22 |
| In-scope CodeNarc target | 384 |
| In-scope rules not registered | 0 |
| Unsupported framework rules registered | 0 |

All in-scope CodeNarc classes are registered. This is **coverage**, not a claim
of complete behavioral parity: the rules have unit/smoke coverage, while
CodeNarc differential fixtures for the newly added rules are still being
expanded.

## Unsupported rules

These classes are intentionally not supported and are not registered.

### Grails (12)

- `GrailsDomainGormMethods`
- `GrailsDomainHasEquals`
- `GrailsDomainHasToString`
- `GrailsDomainReservedSqlKeywordName`
- `GrailsDomainStringPropertyMaxSize`
- `GrailsDomainWithServiceReference`
- `GrailsDuplicateConstraint`
- `GrailsDuplicateMapping`
- `GrailsMassAssignment`
- `GrailsPublicControllerMethod`
- `GrailsServletContextReference`
- `GrailsStatelessService`

### Jenkins (7)

- `ParameterOrReturnTypeNotSerializable`
- `ObjectOverrideOnlyNonCpsMethods`
- `ForbiddenCallInCpsMethod`
- `ExpressionInCpsMethodNotSerializable`
- `CpsCallFromNonCpsMethod`
- `ClosureInGString`
- `ClassNotSerializable`

### Spock (3)

- `SpockIgnoreRestUsed`
- `SpockMissingAssert`
- `SpockUseVerifyEach`

## How this list is maintained

The list is derived by:

1. collecting every `class="org.codenarc.rule...."` from CodeNarc 4.0.0's
   `src/main/resources/rulesets/*.xml`;
2. collecting the `RuleMeta::codenarc_class` values from `RuleRegistry::builtin()`;
3. comparing the two sets by fully-qualified class name.

A future `groovylint rules --coverage --format json` command will generate this
report automatically and gate it in CI.
