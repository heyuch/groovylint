//! TOML → [`ArchConfig`] parser.
//!
//! The parser is deliberately manual: the DSL is closed, and unknown keys must
//! fail with a path. This also makes the places where the DSL is awkward
//! obvious in code instead of hiding them behind `serde(flatten)`.

use std::collections::{BTreeMap, BTreeSet};

use toml::Value;
use toml::map::Map;

use crate::error::{ParseError, Result};
use crate::model::{
    AnnotationArgument, ArchConfig, ClassKind, Constraint, Count, DependencyRef, Modifier, Must,
    PatternList, Predicate, ProjectConfig, Rule, Severity, SourceSet, TargetRef, TargetSpec,
    TargetType, TypeValue, Visibility,
};

/// Parse an architecture configuration from TOML.
pub fn parse(source: &str) -> Result<ArchConfig> {
    let value: Value = toml::from_str(source)
        .map_err(|error| ParseError::new("", format!("invalid TOML: {error}")))?;
    let table = value
        .as_table()
        .ok_or_else(|| ParseError::new("", "configuration root must be a table"))?
        .clone();
    let mut reader = Reader::new("", table);

    let version = reader.required_integer("version")?;
    let version = u32::try_from(version)
        .map_err(|_| ParseError::new("version", "version must be a positive integer"))?;
    if version != 1 {
        return Err(ParseError::new(
            "version",
            format!("unsupported configuration version {version}; expected 1"),
        ));
    }

    let project = reader
        .take("project")
        .map(|value| parse_project(value, "project"))
        .transpose()?
        .unwrap_or_default();

    let mut targets = BTreeMap::new();
    if let Some(value) = reader.take("targets") {
        targets = parse_targets(value, "targets")?;
    }

    let mut rules = Vec::new();
    if let Some(value) = reader.take("rule") {
        for (index, value) in as_array(value, "rule")?.into_iter().enumerate() {
            let path = format!("rule[{index}]");
            rules.push(parse_rule(&value, &path)?);
        }
    }
    reader.finish()?;

    let config = ArchConfig {
        version,
        project,
        targets,
        rules,
    };
    validate(&config)?;
    Ok(config)
}

fn parse_project(value: Value, path: &str) -> Result<ProjectConfig> {
    let mut reader = Reader::new(path, as_table(value, path)?);
    let severity = reader
        .take("severity")
        .map(|value| parse_severity(&value, &format!("{path}.severity")))
        .transpose()?
        .unwrap_or_default();
    reader.finish()?;
    Ok(ProjectConfig { severity })
}

fn parse_targets(value: Value, path: &str) -> Result<BTreeMap<String, TargetSpec>> {
    let table = as_table(value, path)?;
    let mut targets = BTreeMap::new();
    for (name, value) in table {
        if name.is_empty() {
            return Err(ParseError::new(path, "target names must not be empty"));
        }
        let target_path = format!("{path}.{name}");
        let spec = parse_target_spec(value, &target_path, None)?;
        targets.insert(name, spec);
    }
    Ok(targets)
}

fn parse_target_spec(
    value: Value,
    path: &str,
    expected_type: Option<TargetType>,
) -> Result<TargetSpec> {
    let mut reader = Reader::new(path, as_table(value, path)?);
    let target_type = match reader.take("type") {
        Some(value) => {
            let name = as_string(value, &format!("{path}.type"))?;
            TargetType::parse(&name).ok_or_else(|| {
                ParseError::new(
                    format!("{path}.type"),
                    format!(
                        "unknown target type `{name}`; expected module, package, class, method, or field"
                    ),
                )
            })?
        }
        None => expected_type.ok_or_else(|| {
            ParseError::new(
                format!("{path}.type"),
                "target declarations and inline rule targets require `type`",
            )
        })?,
    };
    if let Some(expected_type) = expected_type
        && target_type != expected_type
    {
        return Err(ParseError::new(
            format!("{path}.type"),
            format!(
                "nested selector type `{}` does not match parent type `{}`",
                target_type.as_str(),
                expected_type.as_str()
            ),
        ));
    }
    let predicates = parse_predicates(&mut reader, target_type, path, false)?;
    reader.finish()?;
    Ok(TargetSpec {
        target_type,
        predicates,
    })
}

fn parse_predicates(
    reader: &mut Reader,
    target_type: TargetType,
    path: &str,
    member: bool,
) -> Result<Vec<Predicate>> {
    let mut predicates = Vec::new();
    while let Some((key, value)) = reader.next_entry() {
        let predicate_path = format!("{path}.{key}");
        let predicate = match key.as_str() {
            "name" => Predicate::Name(parse_patterns(value, &predicate_path)?),
            "nameMatches" => Predicate::NameMatches(as_string(value, &predicate_path)?),
            "in" => Predicate::In(parse_target_ref(value, &predicate_path)?),
            "anyOf" => Predicate::AnyOf(parse_target_refs(value, &predicate_path)?),
            "not" => Predicate::Not(Box::new(parse_target_spec(
                value,
                &predicate_path,
                Some(target_type),
            )?)),
            "path" if target_type == TargetType::Module => {
                Predicate::Path(parse_patterns(value, &predicate_path)?)
            }
            "kind" if target_type == TargetType::Class => {
                let name = as_string(value, &predicate_path)?;
                Predicate::Kind(ClassKind::parse(&name).ok_or_else(|| {
                    ParseError::new(
                        &predicate_path,
                        format!(
                            "unknown class kind `{name}`; expected class, interface, trait, enum, annotation, or record"
                        ),
                    )
                })?)
            }
            "abstract" if target_type == TargetType::Class => {
                Predicate::Abstract(as_bool(value, &predicate_path)?)
            }
            "nested" if target_type == TargetType::Class => {
                Predicate::Nested(as_bool(value, &predicate_path)?)
            }
            "extends" if target_type == TargetType::Class => {
                Predicate::Extends(TypeValue(as_string(value, &predicate_path)?))
            }
            "implements" if target_type == TargetType::Class => {
                Predicate::Implements(parse_type_values(value, &predicate_path)?)
            }
            "annotatedWith"
                if target_type != TargetType::Module && target_type != TargetType::Package =>
            {
                Predicate::AnnotatedWith(parse_strings(value, &predicate_path)?)
            }
            "modifiers"
                if member || matches!(target_type, TargetType::Method | TargetType::Field) =>
            {
                Predicate::Modifiers(parse_modifiers(value, &predicate_path)?)
            }
            "paramCount" if target_type == TargetType::Method => {
                Predicate::ParamCount(as_usize(value, &predicate_path)?)
            }
            "rawReturnType" if target_type == TargetType::Method => {
                Predicate::RawReturnType(TypeValue(as_string(value, &predicate_path)?))
            }
            "fieldType" if target_type == TargetType::Field => {
                Predicate::FieldType(TypeValue(as_string(value, &predicate_path)?))
            }
            _ => {
                return Err(ParseError::new(
                    predicate_path,
                    selector_key_error(&key, target_type),
                ));
            }
        };
        predicates.push(predicate);
    }
    Ok(predicates)
}

fn selector_key_error(key: &str, target_type: TargetType) -> String {
    match (key, target_type) {
        ("path", _) => "`path` is a module selector".to_string(),
        ("kind" | "abstract" | "nested" | "extends" | "implements", _) => {
            "this selector is only valid on a class target".to_string()
        }
        ("paramCount" | "rawReturnType", _) => {
            "this selector is only valid on a method target".to_string()
        }
        ("fieldType", _) => {
            "this selector is only valid on a field target; note that `type` is the target type, the field selector is `fieldType`"
                .to_string()
        }
        ("modifiers", _) => {
            "`modifiers` is only valid on method and field targets".to_string()
        }
        _ => format!("unknown selector `{key}` for a {} target", target_type.as_str()),
    }
}

fn parse_rule(value: &Value, path: &str) -> Result<Rule> {
    let mut reader = Reader::new(path, as_table(value.clone(), path)?);
    let id = reader.required_string("id")?;
    if id.trim().is_empty() {
        return Err(ParseError::new(
            format!("{path}.id"),
            "rule id must not be empty",
        ));
    }
    let target = reader.required("target")?;
    let target = parse_target_ref(target, &format!("{path}.target"))?;
    let must = reader.required("must")?;
    let must = parse_must(value_clone(must), &format!("{path}.must"))?;
    let because = reader
        .take("because")
        .map(|value| as_string(value, &format!("{path}.because")))
        .transpose()?;
    let severity = reader
        .take("severity")
        .map(|value| parse_severity(&value, &format!("{path}.severity")))
        .transpose()?
        .unwrap_or_default();
    let on = reader
        .take("on")
        .map(|value| parse_source_set(&value, &format!("{path}.on")))
        .transpose()?
        .unwrap_or_default();
    let except = reader
        .take("except")
        .map(|value| parse_strings(value, &format!("{path}.except")))
        .transpose()?
        .unwrap_or_default();
    reader.finish()?;
    Ok(Rule {
        id,
        target,
        must,
        because,
        severity,
        on,
        except,
    })
}

fn parse_must(value: Value, path: &str) -> Result<Must> {
    let mut reader = Reader::new(path, as_table(value, path)?);
    let mut constraints = Vec::new();
    while let Some((key, value)) = reader.next_entry() {
        let constraint_path = format!("{path}.{key}");
        let constraint = parse_constraint(&key, value, &constraint_path)?;
        constraints.push(constraint);
    }
    if constraints.is_empty() {
        return Err(ParseError::new(
            path,
            "`must` must contain at least one constraint",
        ));
    }
    Ok(Must { constraints })
}

fn parse_constraint(key: &str, value: Value, path: &str) -> Result<Constraint> {
    match key {
        "dependOnlyOn" => Ok(Constraint::DependOnlyOn(parse_dependencies(value, path)?)),
        "notDependOn" => Ok(Constraint::NotDependOn(parse_dependencies(value, path)?)),
        "visible" => Ok(Constraint::Visible(parse_dependencies(value, path)?)),
        "annotatedWith" => Ok(Constraint::AnnotatedWith(parse_strings(value, path)?)),
        "notAnnotatedWith" => Ok(Constraint::NotAnnotatedWith(parse_strings(value, path)?)),
        "annotationArgument" => Ok(Constraint::AnnotationArgument(parse_annotation_argument(
            value, path,
        )?)),
        "extend" => Ok(Constraint::Extend(TypeValue(as_string(value, path)?))),
        "implement" => Ok(Constraint::Implement(parse_type_values(value, path)?)),
        "exist" => Ok(Constraint::Exist(as_bool(value, path)?)),
        "allMembers" => Ok(Constraint::AllMembers(parse_member_predicates(
            value,
            path,
            TargetType::Class,
        )?)),
        "allMethods" => Ok(Constraint::AllMethods(parse_member_predicates(
            value,
            path,
            TargetType::Method,
        )?)),
        "allFields" => Ok(Constraint::AllFields(parse_member_predicates(
            value,
            path,
            TargetType::Field,
        )?)),
        "rawReturnType" => Ok(Constraint::RawReturnType(TypeValue(as_string(
            value, path,
        )?))),
        "notRawReturnType" => Ok(Constraint::NotRawReturnType(parse_type_values(
            value, path,
        )?)),
        "allParams" => Ok(Constraint::AllParams(parse_member_predicates(
            value,
            path,
            TargetType::Method,
        )?)),
        "visibility" => Ok(Constraint::Visibility(parse_visibility(value, path)?)),
        "fieldType" => Ok(Constraint::FieldType(TypeValue(as_string(value, path)?))),
        "notFieldType" => Ok(Constraint::NotFieldType(parse_type_values(value, path)?)),
        "children" => Ok(Constraint::Children(parse_strings(value, path)?)),
        "counts" => Ok(Constraint::Counts(parse_counts(value, path)?)),
        "acyclic" => Ok(Constraint::Acyclic(as_bool(value, path)?)),
        // `type` is the target declaration key on field targets; the field
        // selector / constraint is deliberately named fieldType.
        "type" => Err(ParseError::new(
            path,
            "use `fieldType` for a field type; `type` is the target kind",
        )),
        _ => Err(ParseError::new(path, format!("unknown constraint `{key}`"))),
    }
}

fn parse_member_predicates(
    value: Value,
    path: &str,
    target_type: TargetType,
) -> Result<Vec<Predicate>> {
    let mut reader = Reader::new(path, as_table(value, path)?);
    let predicates = parse_predicates(&mut reader, target_type, path, true)?;
    reader.finish()?;
    Ok(predicates)
}

fn parse_annotation_argument(value: Value, path: &str) -> Result<AnnotationArgument> {
    let mut reader = Reader::new(path, as_table(value, path)?);
    let annotation = reader.required_string("annotation")?;
    let name = reader.required_string("name")?;
    let value = reader.required_string("value")?;
    reader.finish()?;
    Ok(AnnotationArgument {
        annotation,
        name,
        value,
    })
}

fn parse_counts(value: Value, path: &str) -> Result<BTreeMap<String, Count>> {
    let table = as_table(value, path)?;
    let mut counts = BTreeMap::new();
    for (name, value) in table {
        let value_path = format!("{path}.{name}");
        counts.insert(name, parse_count(&value, &value_path)?);
    }
    Ok(counts)
}

fn parse_count(value: &Value, path: &str) -> Result<Count> {
    let text = as_string(value.clone(), path)?;
    if text == "*" {
        return Ok(Count::Any);
    }
    if text == "+" {
        return Ok(Count::Some);
    }
    if let Some(number) = text.strip_prefix(">=") {
        return Ok(Count::AtLeast(parse_usize_text(number, path)?));
    }
    if let Some(number) = text.strip_prefix("<=") {
        return Ok(Count::AtMost(parse_usize_text(number, path)?));
    }
    Err(ParseError::new(
        path,
        format!("invalid count `{text}`; expected >=n, <=n, `*`, or `+`"),
    ))
}

fn parse_usize_text(text: &str, path: &str) -> Result<usize> {
    text.trim()
        .parse()
        .map_err(|_| ParseError::new(path, format!("invalid non-negative integer `{text}`")))
}

fn parse_dependencies(value: Value, path: &str) -> Result<Vec<DependencyRef>> {
    let mut dependencies = Vec::new();
    for (index, value) in as_array(value, path)?.into_iter().enumerate() {
        let item_path = format!("{path}[{index}]");
        let text = as_string(value, &item_path)?;
        if text == "none" {
            dependencies.push(DependencyRef::None);
            continue;
        }
        let (kind, rest) = if let Some(rest) = text.strip_prefix("self:") {
            ("self", rest)
        } else if let Some(rest) = text.strip_prefix("any:") {
            ("any", rest)
        } else if let Some(rest) = text.strip_prefix("other:") {
            ("other", rest)
        } else {
            ("any", text.as_str())
        };
        if rest.is_empty() {
            return Err(ParseError::new(
                item_path,
                "dependency target must not be empty",
            ));
        }
        let target = TargetRef::Named(rest.to_string());
        dependencies.push(match kind {
            "self" => DependencyRef::Self_(target),
            "other" => DependencyRef::Other(target),
            _ => DependencyRef::Any(target),
        });
    }
    Ok(dependencies)
}

fn parse_target_ref(value: Value, path: &str) -> Result<TargetRef> {
    match value {
        Value::String(name) => {
            if name.is_empty() {
                Err(ParseError::new(path, "target name must not be empty"))
            } else {
                Ok(TargetRef::Named(name))
            }
        }
        Value::Table(table) => Ok(TargetRef::Inline(Box::new(parse_target_spec(
            Value::Table(table),
            path,
            None,
        )?))),
        _ => Err(ParseError::new(
            path,
            "target reference must be a name or an inline target table",
        )),
    }
}

fn parse_target_refs(value: Value, path: &str) -> Result<Vec<TargetRef>> {
    as_array(value, path)?
        .into_iter()
        .enumerate()
        .map(|(index, value)| parse_target_ref(value, &format!("{path}[{index}]")))
        .collect()
}

fn parse_patterns(value: Value, path: &str) -> Result<PatternList> {
    let patterns = match value {
        Value::String(pattern) => vec![pattern],
        Value::Array(values) => values
            .into_iter()
            .enumerate()
            .map(|(index, value)| as_string(value, &format!("{path}[{index}]")))
            .collect::<Result<Vec<_>>>()?,
        _ => {
            return Err(ParseError::new(
                path,
                "pattern must be a string or an array of strings",
            ));
        }
    };
    if patterns.is_empty() {
        return Err(ParseError::new(path, "pattern list must not be empty"));
    }
    if patterns.iter().any(String::is_empty) {
        return Err(ParseError::new(path, "patterns must not be empty"));
    }
    Ok(PatternList { patterns })
}

fn parse_type_values(value: Value, path: &str) -> Result<Vec<TypeValue>> {
    let values = match value {
        Value::String(value) => vec![value],
        Value::Array(values) => values
            .into_iter()
            .enumerate()
            .map(|(index, value)| as_string(value, &format!("{path}[{index}]")))
            .collect::<Result<Vec<_>>>()?,
        _ => {
            return Err(ParseError::new(
                path,
                "type value must be a string or an array of strings",
            ));
        }
    };
    if values.is_empty() {
        return Err(ParseError::new(path, "type value list must not be empty"));
    }
    Ok(values.into_iter().map(TypeValue).collect())
}

fn parse_modifiers(value: Value, path: &str) -> Result<Vec<Modifier>> {
    parse_strings(value, path)?
        .into_iter()
        .map(|name| {
            Modifier::parse(&name).ok_or_else(|| {
                ParseError::new(
                    path,
                    format!(
                        "unknown modifier `{name}`; expected static, public, protected, private, abstract, or final"
                    ),
                )
            })
        })
        .collect()
}

fn parse_visibility(value: Value, path: &str) -> Result<Visibility> {
    let text = as_string(value, path)?;
    Visibility::parse(&text).ok_or_else(|| {
        ParseError::new(
            path,
            format!("unknown visibility `{text}`; expected public, protected, private, or package"),
        )
    })
}

fn parse_severity(value: &Value, path: &str) -> Result<Severity> {
    match as_string(value.clone(), path)?.as_str() {
        "error" => Ok(Severity::Error),
        "warn" => Ok(Severity::Warn),
        other => Err(ParseError::new(
            path,
            format!("unknown severity `{other}`; expected error or warn"),
        )),
    }
}

fn parse_source_set(value: &Value, path: &str) -> Result<SourceSet> {
    match as_string(value.clone(), path)?.as_str() {
        "main" => Ok(SourceSet::Main),
        "test" => Ok(SourceSet::Test),
        other => Err(ParseError::new(
            path,
            format!("unknown source set `{other}`; expected main or test"),
        )),
    }
}

fn parse_strings(value: Value, path: &str) -> Result<Vec<String>> {
    let values = match value {
        Value::String(value) => vec![value],
        Value::Array(values) => values
            .into_iter()
            .enumerate()
            .map(|(index, value)| as_string(value, &format!("{path}[{index}]")))
            .collect::<Result<Vec<_>>>()?,
        _ => {
            return Err(ParseError::new(
                path,
                "value must be a string or an array of strings",
            ));
        }
    };
    if values.iter().any(String::is_empty) {
        return Err(ParseError::new(path, "values must not be empty"));
    }
    Ok(values)
}

fn value_clone(value: Value) -> Value {
    value
}

#[allow(clippy::needless_pass_by_value)]
fn as_table(value: Value, path: &str) -> Result<Map<String, Value>> {
    value
        .as_table()
        .cloned()
        .ok_or_else(|| ParseError::new(path, "expected a table"))
}

#[allow(clippy::needless_pass_by_value)]
fn as_array(value: Value, path: &str) -> Result<Vec<Value>> {
    value
        .as_array()
        .cloned()
        .ok_or_else(|| ParseError::new(path, "expected an array"))
}

#[allow(clippy::needless_pass_by_value)]
fn as_string(value: Value, path: &str) -> Result<String> {
    value
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| ParseError::new(path, "expected a string"))
}

#[allow(clippy::needless_pass_by_value)]
fn as_bool(value: Value, path: &str) -> Result<bool> {
    value
        .as_bool()
        .ok_or_else(|| ParseError::new(path, "expected a boolean"))
}

#[allow(clippy::needless_pass_by_value)]
fn as_usize(value: Value, path: &str) -> Result<usize> {
    let number = value
        .as_integer()
        .ok_or_else(|| ParseError::new(path, "expected an integer"))?;
    usize::try_from(number).map_err(|_| ParseError::new(path, "expected a non-negative integer"))
}

struct Reader {
    path: String,
    table: Map<String, Value>,
}

impl Reader {
    fn new(path: impl Into<String>, table: Map<String, Value>) -> Self {
        Self {
            path: path.into(),
            table,
        }
    }

    fn take(&mut self, key: &str) -> Option<Value> {
        self.table.remove(key)
    }

    fn next_entry(&mut self) -> Option<(String, Value)> {
        let key = self.table.keys().next()?.clone();
        self.table.remove(&key).map(|value| (key, value))
    }

    fn required(&mut self, key: &str) -> Result<Value> {
        self.take(key)
            .ok_or_else(|| ParseError::new(self.child(key), "missing required key"))
    }

    fn required_string(&mut self, key: &str) -> Result<String> {
        let value = self.required(key)?;
        as_string(value, &self.child(key))
    }

    fn required_integer(&mut self, key: &str) -> Result<i64> {
        let value = self.required(key)?;
        value
            .as_integer()
            .ok_or_else(|| ParseError::new(self.child(key), "expected an integer"))
    }

    fn child(&self, key: &str) -> String {
        if self.path.is_empty() {
            key.to_string()
        } else {
            format!("{}.{key}", self.path)
        }
    }

    fn finish(self) -> Result<()> {
        if let Some(key) = self.table.keys().next() {
            return Err(ParseError::new(
                self.child(key),
                format!("unknown key `{key}`"),
            ));
        }
        Ok(())
    }
}

fn validate(config: &ArchConfig) -> Result<()> {
    for (name, spec) in &config.targets {
        validate_spec(spec, config, &format!("targets.{name}"))?;
    }
    let mut ids = BTreeSet::new();
    for (index, rule) in config.rules.iter().enumerate() {
        let path = format!("rule[{index}]");
        if !ids.insert(rule.id.clone()) {
            return Err(ParseError::new(
                format!("{path}.id"),
                format!("duplicate rule id `{}`", rule.id),
            ));
        }
        let target_type = resolve_rule_target(&rule.target, config, &format!("{path}.target"))?;
        for constraint in &rule.must.constraints {
            if !constraint_allowed(constraint, target_type) {
                return Err(ParseError::new(
                    format!("{path}.must"),
                    format!(
                        "constraint `{}` is not valid on a {} target",
                        constraint_name(constraint),
                        target_type.as_str()
                    ),
                ));
            }
            validate_constraint(constraint, config, &format!("{path}.must"))?;
        }
    }
    Ok(())
}

fn validate_spec(spec: &TargetSpec, config: &ArchConfig, path: &str) -> Result<()> {
    for predicate in &spec.predicates {
        match predicate {
            Predicate::In(target) => {
                let target_type = resolve_target_type(target, config, &format!("{path}.in"))?;
                if !target_type.can_contain(spec.target_type) {
                    return Err(ParseError::new(
                        format!("{path}.in"),
                        format!(
                            "a {} target cannot be contained by a {} target",
                            spec.target_type.as_str(),
                            target_type.as_str()
                        ),
                    ));
                }
            }
            Predicate::AnyOf(targets) => {
                for target in targets {
                    let target_type =
                        resolve_target_type(target, config, &format!("{path}.anyOf"))?;
                    if target_type != spec.target_type {
                        return Err(ParseError::new(
                            format!("{path}.anyOf"),
                            format!(
                                "anyOf target type `{}` does not match `{}`",
                                target_type.as_str(),
                                spec.target_type.as_str()
                            ),
                        ));
                    }
                }
            }
            Predicate::Not(inner) => {
                if inner.target_type != spec.target_type {
                    return Err(ParseError::new(
                        format!("{path}.not"),
                        "nested selector type does not match parent target type",
                    ));
                }
                validate_spec(inner, config, &format!("{path}.not"))?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_constraint(constraint: &Constraint, config: &ArchConfig, path: &str) -> Result<()> {
    match constraint {
        Constraint::Children(children) => {
            for name in children {
                let target = TargetRef::Named(name.clone());
                let target_type =
                    resolve_target_type(&target, config, &format!("{path}.children"))?;
                if target_type != TargetType::Module {
                    return Err(ParseError::new(
                        format!("{path}.children"),
                        format!("child target `{name}` is not a module target"),
                    ));
                }
            }
        }
        Constraint::Counts(counts) => {
            for name in counts.keys() {
                let target = TargetRef::Named(name.clone());
                let target_type = resolve_target_type(&target, config, &format!("{path}.counts"))?;
                if target_type != TargetType::Module {
                    return Err(ParseError::new(
                        format!("{path}.counts"),
                        format!("count target `{name}` is not a module target"),
                    ));
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn resolve_rule_target(target: &TargetRef, config: &ArchConfig, path: &str) -> Result<TargetType> {
    match target {
        TargetRef::Named(name) => {
            let spec = config
                .targets
                .get(name)
                .ok_or_else(|| ParseError::new(path, format!("unknown target name `{name}`")))?;
            Ok(spec.target_type)
        }
        TargetRef::Inline(spec) => {
            validate_spec(spec, config, path)?;
            Ok(spec.target_type)
        }
    }
}

fn resolve_target_type(target: &TargetRef, config: &ArchConfig, path: &str) -> Result<TargetType> {
    match target {
        TargetRef::Named(name) => config
            .targets
            .get(name)
            .map(|spec| spec.target_type)
            .ok_or_else(|| ParseError::new(path, format!("unknown target name `{name}`"))),
        TargetRef::Inline(spec) => Ok(spec.target_type),
    }
}

fn constraint_allowed(constraint: &Constraint, target_type: TargetType) -> bool {
    match constraint {
        Constraint::DependOnlyOn(_) | Constraint::NotDependOn(_) => {
            matches!(
                target_type,
                TargetType::Module | TargetType::Package | TargetType::Class
            )
        }
        Constraint::Visible(_) => target_type == TargetType::Package,
        Constraint::AnnotatedWith(_)
        | Constraint::NotAnnotatedWith(_)
        | Constraint::AnnotationArgument(_) => matches!(
            target_type,
            TargetType::Class | TargetType::Method | TargetType::Field
        ),
        Constraint::Extend(_)
        | Constraint::Implement(_)
        | Constraint::Exist(_)
        | Constraint::AllMembers(_)
        | Constraint::AllMethods(_)
        | Constraint::AllFields(_) => target_type == TargetType::Class,
        Constraint::RawReturnType(_)
        | Constraint::NotRawReturnType(_)
        | Constraint::AllParams(_)
        | Constraint::Visibility(_) => target_type == TargetType::Method,
        Constraint::FieldType(_) | Constraint::NotFieldType(_) => target_type == TargetType::Field,
        Constraint::Children(_) | Constraint::Counts(_) | Constraint::Acyclic(_) => {
            target_type == TargetType::Module
        }
    }
}

fn constraint_name(constraint: &Constraint) -> &'static str {
    match constraint {
        Constraint::DependOnlyOn(_) => "dependOnlyOn",
        Constraint::NotDependOn(_) => "notDependOn",
        Constraint::Visible(_) => "visible",
        Constraint::AnnotatedWith(_) => "annotatedWith",
        Constraint::NotAnnotatedWith(_) => "notAnnotatedWith",
        Constraint::AnnotationArgument(_) => "annotationArgument",
        Constraint::Extend(_) => "extend",
        Constraint::Implement(_) => "implement",
        Constraint::Exist(_) => "exist",
        Constraint::AllMembers(_) => "allMembers",
        Constraint::AllMethods(_) => "allMethods",
        Constraint::AllFields(_) => "allFields",
        Constraint::RawReturnType(_) => "rawReturnType",
        Constraint::NotRawReturnType(_) => "notRawReturnType",
        Constraint::AllParams(_) => "allParams",
        Constraint::Visibility(_) => "visibility",
        Constraint::FieldType(_) => "fieldType",
        Constraint::NotFieldType(_) => "notFieldType",
        Constraint::Children(_) => "children",
        Constraint::Counts(_) => "counts",
        Constraint::Acyclic(_) => "acyclic",
    }
}
