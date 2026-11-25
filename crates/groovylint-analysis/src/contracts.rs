//! Unified nullness contracts.
//!
//! Contracts from method annotations, the external TOML DSL, and built-in JDK
//! models are normalized into this one domain. The analysis never cares where a
//! contract came from.

use std::path::Path;

use serde::Deserialize;

use crate::nullability::Nullness;

/// A path used in a postcondition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractPath {
    /// Zero-based parameter index.
    Param(usize),
    /// `this`.
    This,
    /// A field or property on `this`.
    Field(String),
}

/// A conditional postcondition, as in `@EnsuresNonNullIf`.
#[derive(Clone, Debug)]
pub struct ConditionalEnsures {
    /// The boolean result that makes the paths non-null.
    pub result: bool,
    /// Paths that become non-null.
    pub paths: Vec<ContractPath>,
}

/// A normalized method contract.
#[derive(Clone, Debug, Default)]
pub struct Contract {
    /// Method name, optionally qualified (`Map.get`). Matching is by suffix.
    pub method: String,
    /// Expected arity; `None` matches any arity.
    pub arity: Option<usize>,
    /// Declared return nullness.
    pub returns: Option<Nullness>,
    /// If set, the result has the nullness of this argument (`Optional.orElse`).
    pub returns_param: Option<usize>,
    /// Parameters that must be non-null at call sites.
    pub required_params: Vec<usize>,
    /// Parameters that are nullable by contract; informational for now.
    pub nullable_params: Vec<usize>,
    /// Unconditional postconditions.
    pub ensures: Vec<ContractPath>,
    /// Conditional postconditions.
    pub ensures_if: Vec<ConditionalEnsures>,
    /// The method never returns normally.
    pub never_returns: bool,
    /// Return is non-null when this argument is non-null (`!null -> !null`).
    pub return_nonnull_if_param: Option<usize>,
    /// After a normal return, this argument must have the given boolean value
    /// (`false -> fail`, `true -> fail`).
    pub condition_param: Option<(usize, bool)>,
    /// For fluent guards: the receiver call's argument at this index is non-null
    /// after the call (`assertThat(x).isNotNull()`).
    pub receiver_param: Option<usize>,
    /// Source label for diagnostics/debugging.
    pub origin: String,
}

impl Contract {
    /// Whether this contract matches a call.
    #[must_use]
    pub fn matches(&self, name: &str, arity: usize) -> bool {
        if self.arity.is_some_and(|expected| expected != arity) {
            return false;
        }
        self.method == name
            || self.method.ends_with(&format!(".{name}"))
            || name.ends_with(&format!(".{}", self.method))
    }
}

/// A registry of contracts from all sources.
#[derive(Clone, Debug, Default)]
pub struct ContractRegistry {
    contracts: Vec<Contract>,
}

impl ContractRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one contract.
    pub fn push(&mut self, contract: Contract) {
        self.contracts.push(contract);
    }

    /// Extends the registry.
    pub fn extend(&mut self, contracts: impl IntoIterator<Item = Contract>) {
        self.contracts.extend(contracts);
    }

    /// Returns contracts matching a call.
    #[must_use]
    pub fn matching(&self, name: &str, arity: usize) -> Vec<&Contract> {
        self.contracts
            .iter()
            .filter(|contract| contract.matches(name, arity))
            .collect()
    }

    /// Finds the first matching contract.
    #[must_use]
    pub fn find(&self, name: &str, arity: usize) -> Option<&Contract> {
        self.contracts
            .iter()
            .rev()
            .find(|contract| contract.matches(name, arity))
    }

    /// Loads a TOML contract file and returns its contracts.
    pub fn load_toml(path: &Path) -> Result<Vec<Contract>, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let file: ContractFile = toml::from_str(&text)
            .map_err(|error| format!("invalid contract file {}: {error}", path.display()))?;
        file.contracts
            .into_iter()
            .map(ContractSpec::into_contract)
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct ContractFile {
    #[serde(default)]
    contracts: Vec<ContractSpec>,
}

#[derive(Debug, Deserialize)]
struct ContractSpec {
    method: String,
    arity: Option<usize>,
    /// A `@Contract`-style value, e.g. `"null, _ -> fail"` or
    /// `"null -> false; !null -> true"`.
    value: Option<String>,
    returns: Option<String>,
    #[serde(default)]
    params: Vec<String>,
    #[serde(default)]
    required: Vec<usize>,
    #[serde(default)]
    ensures: Vec<String>,
    #[serde(default)]
    ensures_if: Vec<EnsuresIfSpec>,
    #[serde(default)]
    never_returns: bool,
    #[serde(default)]
    returns_param: Option<usize>,
    #[serde(default)]
    return_nonnull_if_param: Option<usize>,
    /// One-based parameter that is asserted to be true when the call returns
    /// (an `assertTrue`-style guard).
    #[serde(default)]
    condition: Option<usize>,
    /// Whether the condition parameter is expected to be true (default) or
    /// false when the call returns.
    #[serde(default)]
    condition_result: Option<bool>,
    /// One-based parameter of a receiver expression that is non-null after the
    /// call (`assertThat(x).isNotNull()`).
    #[serde(default)]
    receiver: Option<usize>,
    origin: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EnsuresIfSpec {
    result: bool,
    expression: String,
}

impl ContractSpec {
    fn into_contract(self) -> Result<Contract, String> {
        let mut contract = Contract {
            method: self.method,
            arity: self.arity,
            returns: self.returns.as_deref().map(parse_nullness).transpose()?,
            returns_param: self.returns_param,
            required_params: self.required,
            nullable_params: Vec::new(),
            ensures: self
                .ensures
                .iter()
                .map(|path| parse_path(path))
                .collect::<Result<_, _>>()?,
            ensures_if: self
                .ensures_if
                .iter()
                .map(|spec| {
                    Ok(ConditionalEnsures {
                        result: spec.result,
                        paths: vec![parse_path(&spec.expression)?],
                    })
                })
                .collect::<Result<_, String>>()?,
            never_returns: self.never_returns,
            return_nonnull_if_param: self.return_nonnull_if_param,
            condition_param: self.condition.map(|index| {
                (
                    index.saturating_sub(1),
                    self.condition_result.unwrap_or(true),
                )
            }),
            receiver_param: self.receiver.map(|index| index.saturating_sub(1)),
            origin: self.origin.unwrap_or_else(|| "dsl".to_string()),
        };
        if let Some(value) = &self.value {
            parse_contract_value(value, &mut contract);
        }
        for (index, value) in self.params.iter().enumerate() {
            match parse_nullness(value)? {
                Nullness::NonNull => contract.required_params.push(index),
                Nullness::MaybeNull => contract.nullable_params.push(index),
                _ => {}
            }
        }
        Ok(contract)
    }
}

fn parse_nullness(value: &str) -> Result<Nullness, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "nonnull" | "non-null" | "non_null" => Ok(Nullness::NonNull),
        "nullable" | "null" | "null?" => Ok(Nullness::MaybeNull),
        "unknown" => Ok(Nullness::Unknown),
        other => Err(format!("unknown contract nullness: {other}")),
    }
}

fn parse_path(value: &str) -> Result<ContractPath, String> {
    let value = value.trim().trim_matches('"').trim_matches('\'');
    if let Some(index) = value.strip_prefix('#') {
        let index: usize = index
            .parse()
            .map_err(|_| format!("invalid parameter path: {value}"))?;
        return Ok(ContractPath::Param(index.saturating_sub(1)));
    }
    if value == "this" {
        return Ok(ContractPath::This);
    }
    if let Some(field) = value.strip_prefix("this.") {
        return Ok(ContractPath::Field(field.to_string()));
    }
    if value.is_empty() {
        return Err("empty contract path".to_string());
    }
    Ok(ContractPath::Field(value.to_string()))
}

/// Builds the complete registry from built-ins, annotations, optional package
/// defaults, and an optional TOML contract file.
pub fn registry(
    script: &groovylint_syntax::ast::Script,
    toml_path: Option<&Path>,
) -> Result<ContractRegistry, String> {
    registry_with_defaults(script, toml_path, crate::nullability::Defaults::default())
}

/// Like [`registry`], with package/compiler defaults.
pub fn registry_with_defaults(
    script: &groovylint_syntax::ast::Script,
    toml_path: Option<&Path>,
    defaults: crate::nullability::Defaults,
) -> Result<ContractRegistry, String> {
    let mut registry = ContractRegistry::new();
    registry.extend(builtins());
    registry.extend(from_annotations(script));
    registry.extend(from_defaults(script, defaults));
    if let Some(path) = toml_path {
        registry.extend(ContractRegistry::load_toml(path)?);
    }
    Ok(registry)
}

/// Generates default contracts for unannotated parameters/returns.
#[must_use]
pub fn from_defaults(
    script: &groovylint_syntax::ast::Script,
    defaults: crate::nullability::Defaults,
) -> Vec<Contract> {
    if !defaults.any() {
        return Vec::new();
    }
    let mut contracts = Vec::new();
    collect_defaults(&script.items, defaults, &mut contracts);
    contracts
}

fn collect_defaults(
    items: &[groovylint_syntax::ast::Item],
    defaults: crate::nullability::Defaults,
    contracts: &mut Vec<Contract>,
) {
    for item in items {
        match item {
            groovylint_syntax::ast::Item::Class(class) => {
                let unmarked = class.annotations.iter().any(|annotation| {
                    annotation == "NullUnmarked" || annotation.ends_with(".NullUnmarked")
                });
                if !unmarked {
                    for member in &class.members {
                        if let groovylint_syntax::ast::Item::Method(method) = member {
                            let mut contract = Contract {
                                method: method.name.clone(),
                                arity: Some(method.params.len()),
                                origin: "package-default".to_string(),
                                ..Contract::default()
                            };
                            if defaults.returns_nonnull
                                && method.return_type.as_deref() != Some("void")
                                && !method
                                    .return_type
                                    .as_deref()
                                    .is_some_and(is_primitive_type_name)
                                && annotation_nullness(&method.annotations).is_none()
                            {
                                contract.returns = Some(Nullness::NonNull);
                            }
                            if defaults.params_nonnull {
                                for (index, param) in method.params.iter().enumerate() {
                                    if annotation_nullness(&param.annotations).is_none() {
                                        contract.required_params.push(index);
                                    }
                                }
                            }
                            if contract.returns.is_some() || !contract.required_params.is_empty() {
                                contracts.push(contract);
                            }
                        }
                    }
                }
                collect_defaults(&class.members, defaults, contracts);
            }
            groovylint_syntax::ast::Item::Group(items) => {
                collect_defaults(items, defaults, contracts);
            }
            _ => {}
        }
    }
}

/// Builds the built-in JDK/collection contracts.
#[must_use]
pub fn builtins() -> Vec<Contract> {
    let mut contracts = Vec::new();
    for name in ["requireNonNull", "checkNotNull"] {
        contracts.push(Contract {
            method: name.to_string(),
            arity: Some(1),
            returns: Some(Nullness::NonNull),
            ensures: vec![ContractPath::Param(0)],
            origin: "builtin:Objects.requireNonNull".to_string(),
            ..Contract::default()
        });
    }
    contracts.push(Contract {
        method: "assertNotNull".to_string(),
        arity: None,
        ensures: vec![ContractPath::Param(0)],
        origin: "builtin:assertNotNull".to_string(),
        ..Contract::default()
    });
    contracts.push(Contract {
        method: "isNotNull".to_string(),
        arity: Some(0),
        receiver_param: Some(0),
        origin: "builtin:fluent-assert".to_string(),
        ..Contract::default()
    });
    contracts.push(Contract {
        method: "orElse".to_string(),
        arity: Some(1),
        returns_param: Some(0),
        origin: "builtin:Optional.orElse".to_string(),
        ..Contract::default()
    });
    contracts.push(Contract {
        method: "getOrDefault".to_string(),
        arity: Some(2),
        returns: Some(Nullness::NonNull),
        origin: "builtin:Map.getOrDefault".to_string(),
        ..Contract::default()
    });
    contracts.push(Contract {
        method: "first".to_string(),
        arity: Some(0),
        returns: Some(Nullness::NonNull),
        origin: "builtin:List.first".to_string(),
        ..Contract::default()
    });
    contracts.push(Contract {
        method: "head".to_string(),
        arity: Some(0),
        returns: Some(Nullness::NonNull),
        origin: "builtin:List.head".to_string(),
        ..Contract::default()
    });
    for name in ["get", "orElseThrow", "findFirst"] {
        contracts.push(Contract {
            method: name.to_string(),
            arity: None,
            returns: Some(Nullness::NonNull),
            origin: format!("builtin:Optional.{name}"),
            ..Contract::default()
        });
    }
    for name in ["get", "max", "min", "find"] {
        contracts.push(Contract {
            method: name.to_string(),
            arity: None,
            returns: Some(Nullness::MaybeNull),
            origin: format!("builtin:nullable-lookup.{name}"),
            ..Contract::default()
        });
    }
    contracts
}

/// Extracts contracts from annotations on methods/parameters.
#[must_use]
pub fn from_annotations(script: &groovylint_syntax::ast::Script) -> Vec<Contract> {
    let mut contracts = Vec::new();
    collect_from_items(&script.items, &mut contracts);
    contracts
}

fn collect_from_items(items: &[groovylint_syntax::ast::Item], contracts: &mut Vec<Contract>) {
    for item in items {
        match item {
            groovylint_syntax::ast::Item::Class(class) => {
                let marked = class.annotations.iter().any(|annotation| {
                    annotation == "NullMarked"
                        || annotation.ends_with(".NullMarked")
                        || annotation == "NonNullApi"
                        || annotation.ends_with(".NonNullApi")
                });
                if marked {
                    for member in &class.members {
                        if let groovylint_syntax::ast::Item::Method(method) = member {
                            let mut contract = contract_from_method(method);
                            if contract.returns.is_none()
                                && method.return_type.as_deref() != Some("void")
                                && !method
                                    .return_type
                                    .as_deref()
                                    .is_some_and(is_primitive_type_name)
                            {
                                contract.returns = Some(Nullness::NonNull);
                            }
                            for (index, param) in method.params.iter().enumerate() {
                                if annotation_nullness(&param.annotations).is_none()
                                    && !contract.required_params.contains(&index)
                                {
                                    contract.required_params.push(index);
                                }
                            }
                            contracts.push(contract);
                        }
                    }
                }
                collect_from_items(&class.members, contracts);
            }
            groovylint_syntax::ast::Item::Method(method) => {
                let contract = contract_from_method(method);
                if contract.returns.is_some()
                    || !contract.required_params.is_empty()
                    || !contract.ensures.is_empty()
                    || !contract.ensures_if.is_empty()
                    || contract.never_returns
                    || contract.condition_param.is_some()
                    || contract.return_nonnull_if_param.is_some()
                    || contract.receiver_param.is_some()
                {
                    contracts.push(contract);
                }
            }
            groovylint_syntax::ast::Item::Group(items) => collect_from_items(items, contracts),
            _ => {}
        }
    }
}

fn contract_from_method(method: &groovylint_syntax::ast::MethodDecl) -> Contract {
    let mut contract = Contract {
        method: method.name.clone(),
        arity: Some(method.params.len()),
        origin: format!("annotation:{}", method.name),
        ..Contract::default()
    };
    match annotation_nullness(&method.annotations) {
        Some(Nullness::MaybeNull) => contract.returns = Some(Nullness::MaybeNull),
        Some(Nullness::NonNull) => contract.returns = Some(Nullness::NonNull),
        _ => {}
    }
    for (index, param) in method.params.iter().enumerate() {
        match annotation_nullness(&param.annotations) {
            Some(Nullness::NonNull) => contract.required_params.push(index),
            Some(Nullness::MaybeNull) => contract.nullable_params.push(index),
            _ => {}
        }
    }
    for annotation in &method.annotation_details {
        if annotation.name == "NoReturn" {
            contract.never_returns = true;
            continue;
        }
        let Some(raw) = annotation.raw_arguments.as_deref() else {
            continue;
        };
        match annotation.name.as_str() {
            "EnsuresNonNull" => {
                let expression = raw.split_once('=').map_or(raw, |(_, value)| value);
                if let Ok(path) = parse_path(expression) {
                    contract.ensures.push(path);
                }
            }
            "EnsuresNonNullIf" => {
                if let Some((expression, result)) = parse_ensures_if(raw)
                    && let Ok(path) = parse_path(&expression)
                {
                    contract.ensures_if.push(ConditionalEnsures {
                        result,
                        paths: vec![path],
                    });
                }
            }
            "Contract" => parse_contract_value(raw, &mut contract),
            _ => {}
        }
    }
    contract
}

fn parse_ensures_if(raw: &str) -> Option<(String, bool)> {
    let mut expression = None;
    let mut result = None;
    for part in raw.split(',') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"').trim_matches('\'');
        match key {
            "expression" => expression = Some(value.to_string()),
            "result" => result = value.parse().ok(),
            _ => {}
        }
    }
    Some((expression?, result?))
}

/// Applies a `@Contract`-style value, e.g. `"null -> fail"` or
/// `"null -> false; !null -> true"`. Clauses are separated by `;`.
fn parse_contract_value(value: &str, contract: &mut Contract) {
    for clause in value.split(';') {
        parse_contract_clause(clause, contract);
    }
}

/// Applies one `@Contract` clause.
///
/// The left-hand side holds one pattern per parameter (`_`, `null`, `!null`,
/// `true`, `false`) and the right-hand side the outcome (`fail`, `null`,
/// `!null`, `true`, `false`, `this`, `param1`). Only the shapes the analysis can
/// express are recorded; the rest is ignored rather than rejected.
fn parse_contract_clause(raw: &str, contract: &mut Contract) {
    let raw = raw.trim().trim_matches('"').trim_matches('\'');
    let Some((lhs, rhs)) = raw.split_once("->") else {
        return;
    };
    let patterns: Vec<&str> = lhs.split(',').map(str::trim).collect();
    let rhs = rhs.trim();
    let concrete = patterns
        .iter()
        .position(|pattern| !pattern.is_empty() && *pattern != "_");

    if rhs == "fail" || rhs == "never" {
        let Some(index) = concrete else {
            contract.never_returns = true;
            return;
        };
        match patterns[index] {
            // `null -> fail`: the argument is non-null after a normal return.
            "null" => contract.ensures.push(ContractPath::Param(index)),
            // `false -> fail`: the argument is true after a normal return.
            "false" => contract.condition_param = Some((index, true)),
            // `true -> fail`: the argument is false after a normal return.
            "true" => contract.condition_param = Some((index, false)),
            _ => {}
        }
        return;
    }

    match rhs {
        // `_ -> !null` / `_ -> null`: the result is fixed.
        "!null" if concrete.is_none() => contract.returns = Some(Nullness::NonNull),
        "null" if concrete.is_none() => contract.returns = Some(Nullness::MaybeNull),
        // `!null -> !null`: a non-null argument produces a non-null result.
        "!null" => {
            if let Some(index) = concrete
                && patterns[index] == "!null"
            {
                contract.return_nonnull_if_param = Some(index);
            }
        }
        // `_ -> this` is always non-null.
        "this" => contract.returns = Some(Nullness::NonNull),
        // `_ -> param1` returns that argument unchanged.
        "true" | "false" => {
            let Some(index) = concrete else {
                return;
            };
            if patterns[index] == "null" {
                // `null -> true`: a normal return of the other value proves the
                // argument was non-null.
                contract.ensures_if.push(ConditionalEnsures {
                    result: rhs == "false",
                    paths: vec![ContractPath::Param(index)],
                });
            }
        }
        _ => {
            if let Some(rest) = rhs.strip_prefix("param")
                && let Ok(position) = rest.trim().parse::<usize>()
                && position > 0
            {
                contract.returns_param = Some(position - 1);
            }
        }
    }
}

fn is_primitive_type_name(name: &str) -> bool {
    matches!(
        name,
        "boolean" | "byte" | "short" | "int" | "long" | "float" | "double" | "char"
    )
}

fn annotation_nullness(annotations: &[String]) -> Option<Nullness> {
    if annotations
        .iter()
        .any(|annotation| annotation == "Nullable" || annotation.ends_with(".Nullable"))
    {
        return Some(Nullness::MaybeNull);
    }
    if annotations.iter().any(|annotation| {
        matches!(
            annotation.as_str(),
            "NonNull" | "NotNull" | "NotBlank" | "NotEmpty"
        ) || annotation.ends_with(".NonNull")
            || annotation.ends_with(".NotNull")
            || annotation.ends_with(".NotBlank")
            || annotation.ends_with(".NotEmpty")
    }) {
        return Some(Nullness::NonNull);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{Contract, ContractPath, ContractRegistry, Nullness, parse_contract_value};

    fn contract(value: &str) -> Contract {
        let mut contract = Contract {
            method: "m".to_string(),
            arity: Some(2),
            ..Contract::default()
        };
        parse_contract_value(value, &mut contract);
        contract
    }

    #[test]
    fn parses_guard_clauses() {
        let null_fail = contract("null, _ -> fail");
        assert_eq!(null_fail.ensures, vec![ContractPath::Param(0)]);
        assert!(!null_fail.never_returns);

        let second_null = contract("_, null -> fail");
        assert_eq!(second_null.ensures, vec![ContractPath::Param(1)]);

        let always_fails = contract("_, _ -> fail");
        assert!(always_fails.never_returns);

        assert_eq!(
            contract("null -> fail").ensures,
            vec![ContractPath::Param(0)]
        );
    }

    #[test]
    fn parses_boolean_guard_clauses() {
        assert_eq!(
            contract("false, _ -> fail").condition_param,
            Some((0, true))
        );
        assert_eq!(
            contract("true, _ -> fail").condition_param,
            Some((0, false))
        );
        assert_eq!(
            contract("_, false -> fail").condition_param,
            Some((1, true))
        );
    }

    #[test]
    fn parses_result_clauses() {
        let predicate = contract("null -> false; !null -> true");
        assert_eq!(predicate.ensures_if.len(), 1);
        assert!(predicate.ensures_if[0].result);
        assert_eq!(predicate.ensures_if[0].paths, vec![ContractPath::Param(0)]);

        let negated = contract("null -> true");
        assert!(!negated.ensures_if[0].result);

        assert_eq!(
            contract("!null, _ -> !null").return_nonnull_if_param,
            Some(0)
        );
        assert_eq!(contract("_, _ -> !null").returns, Some(Nullness::NonNull));
        assert_eq!(contract("_ -> param2").returns_param, Some(1));
        assert_eq!(contract("_, _ -> this").returns, Some(Nullness::NonNull));
    }

    #[test]
    fn toml_value_matches_annotation_syntax() {
        let mut registry = ContractRegistry::new();
        let spec: Contract = {
            let mut contract = Contract {
                method: "com.acme.Guard.notNull".to_string(),
                arity: Some(2),
                ..Contract::default()
            };
            parse_contract_value("null, _ -> fail", &mut contract);
            contract
        };
        registry.push(spec);
        let found = registry.find("notNull", 2).expect("contract matches");
        assert_eq!(found.ensures, vec![ContractPath::Param(0)]);
    }
}
