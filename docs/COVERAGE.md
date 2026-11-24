# CodeNarc 4.0.0 coverage

Baseline: CodeNarc 4.0.0, 24 rulesets, **406 concrete rule classes**.

Current registry:

| Status | Count |
|---|---:|
| Unique registered rule classes | 327 |
| Missing from CodeNarc 4.0.0 | 79 |
| Grails rules (explicitly excluded) | 12 |
| Jenkins rules (registered, excluded from target) | 7 |
| Spock rules (excluded from target) | 3 |
| Non-framework/non-Spock missing | 66 |
| Non-framework registered stubs/partial | 8 |
| In-scope CodeNarc target | 384 |

The registry currently has 328 entries but 327 unique classes:
`ThrowExceptionFromFinallyBlock` is registered twice.

## Non-framework, non-Spock rules still missing (66)

### `groovyism` (33)

- `AssignCollectionSort`
- `AssignCollectionUnique`
- `ClosureAsLastMethodParameter`
- `CollectAllIsDeprecated`
- `ConfusingMultipleReturns`
- `ExplicitArrayListInstantiation`
- `ExplicitCallToAndMethod`
- `ExplicitCallToCompareToMethod`
- `ExplicitCallToDivMethod`
- `ExplicitCallToEqualsMethod`
- `ExplicitCallToGetAtMethod`
- `ExplicitCallToLeftShiftMethod`
- `ExplicitCallToMinusMethod`
- `ExplicitCallToModMethod`
- `ExplicitCallToMultiplyMethod`
- `ExplicitCallToOrMethod`
- `ExplicitCallToPlusMethod`
- `ExplicitCallToPowerMethod`
- `ExplicitCallToPutAtMethod`
- `ExplicitCallToRightShiftMethod`
- `ExplicitCallToXorMethod`
- `ExplicitHashMapInstantiation`
- `ExplicitHashSetInstantiation`
- `ExplicitLinkedHashMapInstantiation`
- `ExplicitLinkedListInstantiation`
- `ExplicitStackInstantiation`
- `ExplicitTreeSetInstantiation`
- `GStringAsMapKey`
- `GStringExpressionWithinString`
- `GetterMethodCouldBeProperty`
- `GroovyLangImmutable`
- `UseCollectMany`
- `UseCollectNested`

### `size` (7)

- `AbcMetric`
- `ClassSize`
- `CyclomaticComplexity`
- `MethodCount`
- `MethodSize`
- `NestedBlockDepth`
- `ParameterCount`

### `unnecessary` (7)

- `UnnecessaryCast`
- `UnnecessaryCollectCall`
- `UnnecessaryGString`
- `UnnecessaryGetter`
- `UnnecessaryReturnKeyword`
- `UnnecessarySetter`
- `UnnecessaryToString`

### `convention` (5)

- `CouldBeSwitchStatement`
- `IfStatementCouldBeTernary`
- `PublicMethodsBeforeNonPublicMethods`
- `StaticFieldsBeforeInstanceFields`
- `StaticMethodsBeforeInstanceMethods`

### `design` (5)

- `AssignmentToStaticFieldFromInstanceMethod`
- `BooleanMethodReturnsNull`
- `Instanceof`
- `NestedForLoop`
- `ReturnsNullInsteadOfEmptyCollection`

### `dry` (2)

- `DuplicateNumberLiteral`
- `DuplicateStringLiteral`

### `formatting` (2)

- `LineLength`
- `SpaceAroundMapEntryColon`

### Remaining singles (5)

- `concurrency`: `SynchronizedOnThis`
- `exceptions`: `CatchThrowable`
- `imports`: `MisorderedStaticImports`
- `naming`: `FactoryMethodName`
- `security`: `JavaIoPackageAccess`

## Registered but stub/partial (8 non-framework)

These load and accept configuration, but do not currently provide full behavior:

- `Indentation`
- `BusyWait`
- `DoubleCheckedLocking`
- `InconsistentPropertyLocking`
- `InconsistentPropertySynchronization`
- `NoScriptBindings`
- `ExceptionNotThrown`
- `IllegalClassMember`

## Excluded framework rules

### Grails (12, not registered)

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

### Spock (3, excluded)

- `SpockIgnoreRestUsed` (registered, excluded)
- `SpockMissingAssert` (registered, excluded)
- `SpockUseVerifyEach` (not registered, excluded)

### Jenkins (7, registered but excluded)

- `ParameterOrReturnTypeNotSerializable`
- `ObjectOverrideOnlyNonCpsMethods`
- `ForbiddenCallInCpsMethod`
- `ExpressionInCpsMethodNotSerializable`
- `CpsCallFromNonCpsMethod`
- `ClosureInGString`
- `ClassNotSerializable`

## How this list is maintained

The list is derived by:

1. collecting every `class="org.codenarc.rule...."` from CodeNarc 4.0.0's
   `src/main/resources/rulesets/*.xml`;
2. collecting the `RuleMeta::codenarc_class` values from `RuleRegistry::builtin()`;
3. comparing the two sets by fully-qualified class name.

A future `groovylint rules --coverage --format json` command will generate this
report automatically and gate it in CI, replacing this hand-maintained table.
