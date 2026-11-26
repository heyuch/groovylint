//! TOML → [`ArchConfig`] parser.
//!
//! The target kind is a built-in constructor, written as `kind = "module"` in
//! TOML. The parser dispatches on it and builds the corresponding typed
//! [`Target`] variant, so a class target cannot contain a module-only selector.

use std::collections::{BTreeMap, BTreeSet};

use toml::Value;
use toml::map::Map;

use crate::error::{ParseError, Result};
use crate::model::{
    AnnotationArgument, ArchConfig, ClassKind, ClassPredicate, ClassTarget, Constraint, Count,
    DependencyRef, FieldPredicate, MemberKind, MemberPredicate, MethodPredicate, MethodTarget,
    Modifier, ModulePredicate, ModuleTarget, Must, PackagePredicate, PackageTarget, ParamPredicate,
    PatternList, ProjectConfig, Rule, Severity, SourceSet, Target, TargetRef, TargetType,
    TypeValue, Visibility,
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
            rules.push(parse_rule(&value, &format!("rule[{index}]"))?);
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

fn parse_targets(value: Value, path: &str) -> Result<BTreeMap<String, Target>> {
    let table = as_table(value, path)?;
    let mut targets = BTreeMap::new();
    for (name, value) in table {
        if name.is_empty() {
            return Err(ParseError::new(path, "target names must not be empty"));
        }
        let target = parse_target(value, &format!("{path}.{name}"), None)?;
        targets.insert(name, target);
    }
    Ok(targets)
}

fn parse_target(value: Value, path: &str, expected: Option<TargetType>) -> Result<Target> {
    let mut reader = Reader::new(path, as_table(value, path)?);
    let target_type = match reader.take("type") {
        Some(value) => {
            let name = as_string(value, &format!("{path}.type"))?;
            TargetType::parse(&name).ok_or_else(|| {
                ParseError::new(
                    format!("{path}.type"),
                    format!(
                        "unknown target type `{name}`; expected module, package, class, or method"
                    ),
                )
            })?
        }
        None => expected.ok_or_else(|| {
            ParseError::new(
                format!("{path}.type"),
                "target declarations and inline targets require `type`",
            )
        })?,
    };
    if let Some(expected) = expected
        && target_type != expected
    {
        return Err(ParseError::new(
            format!("{path}.type"),
            format!(
                "nested target type `{}` does not match parent type `{}`",
                target_type.as_str(),
                expected.as_str()
            ),
        ));
    }
    let target = match target_type {
        TargetType::Module => Target::Module(parse_module(&mut reader, path)?),
        TargetType::Package => Target::Package(parse_package(&mut reader, path)?),
        TargetType::Class => Target::Class(parse_class(&mut reader, path)?),
        TargetType::Method => Target::Method(parse_method(&mut reader, path)?),
    };
    reader.finish()?;
    Ok(target)
}

fn parse_module(reader: &mut Reader, path: &str) -> Result<ModuleTarget> {
    let mut predicates = Vec::new();
    while let Some((key, value)) = reader.next_entry() {
        let item = format!("{path}.{key}");
        let predicate = match key.as_str() {
            "name" => ModulePredicate::Name(parse_patterns(value, &item)?),
            "nameMatches" => ModulePredicate::NameMatches(as_string(value, &item)?),
            "path" => ModulePredicate::Path(parse_patterns(value, &item)?),
            "anyOf" => ModulePredicate::AnyOf(parse_refs(value, &item)?),
            "not" => ModulePredicate::Not(Box::new(parse_nested_module(value, &item)?)),
            _ => return Err(unknown_selector(&item, &key, TargetType::Module)),
        };
        predicates.push(predicate);
    }
    Ok(ModuleTarget { predicates })
}

fn parse_package(reader: &mut Reader, path: &str) -> Result<PackageTarget> {
    let mut predicates = Vec::new();
    while let Some((key, value)) = reader.next_entry() {
        let item = format!("{path}.{key}");
        let predicate = match key.as_str() {
            "name" => PackagePredicate::Name(parse_patterns(value, &item)?),
            "nameMatches" => PackagePredicate::NameMatches(as_string(value, &item)?),
            "in" => PackagePredicate::In(parse_ref(value, &item)?),
            "anyOf" => PackagePredicate::AnyOf(parse_refs(value, &item)?),
            "not" => PackagePredicate::Not(Box::new(parse_nested_package(value, &item)?)),
            _ => return Err(unknown_selector(&item, &key, TargetType::Package)),
        };
        predicates.push(predicate);
    }
    Ok(PackageTarget { predicates })
}

fn parse_class(reader: &mut Reader, path: &str) -> Result<ClassTarget> {
    let mut predicates = Vec::new();
    while let Some((key, value)) = reader.next_entry() {
        let item = format!("{path}.{key}");
        let predicate = match key.as_str() {
            "name" => ClassPredicate::Name(parse_patterns(value, &item)?),
            "nameMatches" => ClassPredicate::NameMatches(as_string(value, &item)?),
            "in" => ClassPredicate::In(parse_ref(value, &item)?),
            "anyOf" => ClassPredicate::AnyOf(parse_refs(value, &item)?),
            "not" => ClassPredicate::Not(Box::new(parse_nested_class(value, &item)?)),
            "kind" => {
                let name = as_string(value, &item)?;
                ClassPredicate::Kind(ClassKind::parse(&name).ok_or_else(|| {
                    ParseError::new(
                        &item,
                        format!(
                            "unknown class kind `{name}`; expected class, interface, trait, enum, annotation, or record"
                        ),
                    )
                })?)
            }
            "abstract" => ClassPredicate::Abstract(as_bool(value, &item)?),
            "nested" => ClassPredicate::Nested(as_bool(value, &item)?),
            "extends" => ClassPredicate::Extends(TypeValue(as_string(value, &item)?)),
            "implements" => ClassPredicate::Implements(parse_type_values(value, &item)?),
            "annotatedWith" => ClassPredicate::AnnotatedWith(parse_strings(value, &item)?),
            _ => return Err(unknown_selector(&item, &key, TargetType::Class)),
        };
        predicates.push(predicate);
    }
    Ok(ClassTarget { predicates })
}

fn parse_method(reader: &mut Reader, path: &str) -> Result<MethodTarget> {
    let mut predicates = Vec::new();
    while let Some((key, value)) = reader.next_entry() {
        let item = format!("{path}.{key}");
        let predicate = match key.as_str() {
            "name" => MethodPredicate::Name(parse_patterns(value, &item)?),
            "nameMatches" => MethodPredicate::NameMatches(as_string(value, &item)?),
            "in" => MethodPredicate::In(parse_ref(value, &item)?),
            "anyOf" => MethodPredicate::AnyOf(parse_refs(value, &item)?),
            "not" => MethodPredicate::Not(Box::new(parse_nested_method(value, &item)?)),
            "annotatedWith" => MethodPredicate::AnnotatedWith(parse_strings(value, &item)?),
            "modifiers" => MethodPredicate::Modifiers(parse_modifiers(value, &item)?),
            "paramCount" => MethodPredicate::ParamCount(as_usize(value, &item)?),
            "rawReturnType" => MethodPredicate::RawReturnType(TypeValue(as_string(value, &item)?)),
            _ => return Err(unknown_selector(&item, &key, TargetType::Method)),
        };
        predicates.push(predicate);
    }
    Ok(MethodTarget { predicates })
}

fn parse_field(reader: &mut Reader, path: &str) -> Result<Vec<FieldPredicate>> {
    let mut predicates = Vec::new();
    while let Some((key, value)) = reader.next_entry() {
        let item = format!("{path}.{key}");
        let predicate = match key.as_str() {
            "name" => FieldPredicate::Name(parse_patterns(value, &item)?),
            "nameMatches" => FieldPredicate::NameMatches(as_string(value, &item)?),
            "annotatedWith" => FieldPredicate::AnnotatedWith(parse_strings(value, &item)?),
            "modifiers" => FieldPredicate::Modifiers(parse_modifiers(value, &item)?),
            "type" => FieldPredicate::Type(TypeValue(as_string(value, &item)?)),
            _ => return Err(unknown_selector(&item, &key, TargetType::Class)),
        };
        predicates.push(predicate);
    }
    Ok(predicates)
}

fn parse_nested_module(value: Value, path: &str) -> Result<ModuleTarget> {
    match parse_target(value, path, Some(TargetType::Module))? {
        Target::Module(target) => Ok(target),
        _ => Err(ParseError::new(path, "expected a module target")),
    }
}

fn parse_nested_package(value: Value, path: &str) -> Result<PackageTarget> {
    match parse_target(value, path, Some(TargetType::Package))? {
        Target::Package(target) => Ok(target),
        _ => Err(ParseError::new(path, "expected a package target")),
    }
}

fn parse_nested_class(value: Value, path: &str) -> Result<ClassTarget> {
    match parse_target(value, path, Some(TargetType::Class))? {
        Target::Class(target) => Ok(target),
        _ => Err(ParseError::new(path, "expected a class target")),
    }
}

fn parse_nested_method(value: Value, path: &str) -> Result<MethodTarget> {
    match parse_target(value, path, Some(TargetType::Method))? {
        Target::Method(target) => Ok(target),
        _ => Err(ParseError::new(path, "expected a method target")),
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
    let target = parse_ref(reader.required("target")?, &format!("{path}.target"))?;
    let must = parse_must(reader.required("must")?, &format!("{path}.must"))?;
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
        constraints.push(parse_constraint(&key, value, &format!("{path}.{key}"))?);
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
            value, path,
        )?)),
        "allMethods" => Ok(Constraint::AllMethods(parse_method_predicates(
            value, path,
        )?)),
        "allFields" => Ok(Constraint::AllFields(parse_field_predicates(value, path)?)),
        "rawReturnType" => Ok(Constraint::RawReturnType(TypeValue(as_string(
            value, path,
        )?))),
        "notRawReturnType" => Ok(Constraint::NotRawReturnType(parse_type_values(
            value, path,
        )?)),
        "allParams" => Ok(Constraint::AllParams(parse_param_predicates(value, path)?)),
        "visibility" => Ok(Constraint::Visibility(parse_visibility(value, path)?)),
        "children" => Ok(Constraint::Children(parse_strings(value, path)?)),
        "counts" => Ok(Constraint::Counts(parse_counts(value, path)?)),
        "acyclic" => Ok(Constraint::Acyclic(as_bool(value, path)?)),
        _ => Err(ParseError::new(path, format!("unknown constraint `{key}`"))),
    }
}

fn parse_member_predicates(value: Value, path: &str) -> Result<Vec<MemberPredicate>> {
    let mut reader = Reader::new(path, as_table(value, path)?);
    let mut predicates = Vec::new();
    while let Some((key, value)) = reader.next_entry() {
        let item = format!("{path}.{key}");
        let predicate = match key.as_str() {
            "name" => MemberPredicate::Name(parse_patterns(value, &item)?),
            "nameMatches" => MemberPredicate::NameMatches(as_string(value, &item)?),
            "annotatedWith" => MemberPredicate::AnnotatedWith(parse_strings(value, &item)?),
            "modifiers" => MemberPredicate::Modifiers(parse_modifiers(value, &item)?),
            "kind" => {
                let name = as_string(value, &item)?;
                MemberPredicate::Kind(MemberKind::parse(&name).ok_or_else(|| {
                    ParseError::new(
                        &item,
                        format!("unknown member kind `{name}`; expected field or method"),
                    )
                })?)
            }
            "not" => MemberPredicate::Not(Box::new(parse_member_predicate(value, &item)?)),
            _ => {
                return Err(ParseError::new(
                    item,
                    format!("unknown member selector `{key}`"),
                ));
            }
        };
        predicates.push(predicate);
    }
    if predicates.is_empty() {
        return Err(ParseError::new(path, "member selector must not be empty"));
    }
    reader.finish()?;
    Ok(predicates)
}

fn parse_member_predicate(value: Value, path: &str) -> Result<MemberPredicate> {
    let mut predicates = parse_member_predicates(value, path)?;
    if predicates.len() != 1 {
        return Err(ParseError::new(
            path,
            "`not` currently accepts exactly one member selector",
        ));
    }
    Ok(predicates.remove(0))
}

fn parse_method_predicates(value: Value, path: &str) -> Result<Vec<MethodPredicate>> {
    let mut reader = Reader::new(path, as_table(value, path)?);
    let target = parse_method(&mut reader, path)?;
    reader.finish()?;
    Ok(target.predicates)
}

fn parse_field_predicates(value: Value, path: &str) -> Result<Vec<FieldPredicate>> {
    let mut reader = Reader::new(path, as_table(value, path)?);
    let predicates = parse_field(&mut reader, path)?;
    reader.finish()?;
    Ok(predicates)
}

fn parse_param_predicates(value: Value, path: &str) -> Result<Vec<ParamPredicate>> {
    let mut reader = Reader::new(path, as_table(value, path)?);
    let mut predicates = Vec::new();
    while let Some((key, value)) = reader.next_entry() {
        let item = format!("{path}.{key}");
        let predicate = match key.as_str() {
            "name" => ParamPredicate::Name(parse_patterns(value, &item)?),
            "nameMatches" => ParamPredicate::NameMatches(as_string(value, &item)?),
            "annotatedWith" => ParamPredicate::AnnotatedWith(parse_strings(value, &item)?),
            "type" => ParamPredicate::Type(TypeValue(as_string(value, &item)?)),
            "not" => ParamPredicate::Not(Box::new(parse_param_predicate(value, &item)?)),
            _ => {
                return Err(ParseError::new(
                    item,
                    format!("unknown parameter selector `{key}`"),
                ));
            }
        };
        predicates.push(predicate);
    }
    if predicates.is_empty() {
        return Err(ParseError::new(
            path,
            "parameter selector must not be empty",
        ));
    }
    reader.finish()?;
    Ok(predicates)
}

fn parse_param_predicate(value: Value, path: &str) -> Result<ParamPredicate> {
    let mut predicates = parse_param_predicates(value, path)?;
    if predicates.len() != 1 {
        return Err(ParseError::new(
            path,
            "`not` currently accepts exactly one parameter selector",
        ));
    }
    Ok(predicates.remove(0))
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
        let count = parse_count(&value, &format!("{path}.{name}"))?;
        counts.insert(name, count);
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
        let item = format!("{path}[{index}]");
        let text = as_string(value, &item)?;
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
            return Err(ParseError::new(item, "dependency target must not be empty"));
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

fn parse_ref(value: Value, path: &str) -> Result<TargetRef> {
    match value {
        Value::String(name) => {
            if name.is_empty() {
                Err(ParseError::new(path, "target name must not be empty"))
            } else {
                Ok(TargetRef::Named(name))
            }
        }
        Value::Table(table) => Ok(TargetRef::Inline(Box::new(parse_target(
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

fn parse_refs(value: Value, path: &str) -> Result<Vec<TargetRef>> {
    as_array(value, path)?
        .into_iter()
        .enumerate()
        .map(|(index, value)| parse_ref(value, &format!("{path}[{index}]")))
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

fn unknown_selector(path: &str, key: &str, target_type: TargetType) -> ParseError {
    ParseError::new(
        path,
        format!(
            "unknown selector `{key}` for a {} target",
            target_type.as_str()
        ),
    )
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
    for (name, target) in &config.targets {
        validate_target(target, config, &format!("targets.{name}"))?;
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
        if let TargetRef::Inline(target) = &rule.target {
            validate_target(target, config, &format!("{path}.target"))?;
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

fn validate_target(target: &Target, config: &ArchConfig, path: &str) -> Result<()> {
    match target {
        Target::Module(target) => {
            for predicate in &target.predicates {
                if let ModulePredicate::AnyOf(refs) = predicate {
                    validate_refs(refs, TargetType::Module, config, &format!("{path}.anyOf"))?;
                } else if let ModulePredicate::Not(inner) = predicate {
                    validate_target(
                        &Target::Module((**inner).clone()),
                        config,
                        &format!("{path}.not"),
                    )?;
                }
            }
        }
        Target::Package(target) => {
            for predicate in &target.predicates {
                match predicate {
                    PackagePredicate::In(reference) => {
                        validate_container(
                            reference,
                            TargetType::Package,
                            config,
                            &format!("{path}.in"),
                        )?;
                    }
                    PackagePredicate::AnyOf(refs) => {
                        validate_refs(refs, TargetType::Package, config, &format!("{path}.anyOf"))?;
                    }
                    PackagePredicate::Not(inner) => {
                        validate_target(
                            &Target::Package((**inner).clone()),
                            config,
                            &format!("{path}.not"),
                        )?;
                    }
                    _ => {}
                }
            }
        }
        Target::Class(target) => {
            for predicate in &target.predicates {
                match predicate {
                    ClassPredicate::In(reference) => {
                        validate_container(
                            reference,
                            TargetType::Class,
                            config,
                            &format!("{path}.in"),
                        )?;
                    }
                    ClassPredicate::AnyOf(refs) => {
                        validate_refs(refs, TargetType::Class, config, &format!("{path}.anyOf"))?;
                    }
                    ClassPredicate::Not(inner) => {
                        validate_target(
                            &Target::Class((**inner).clone()),
                            config,
                            &format!("{path}.not"),
                        )?;
                    }
                    _ => {}
                }
            }
        }
        Target::Method(target) => {
            for predicate in &target.predicates {
                match predicate {
                    MethodPredicate::In(reference) => {
                        validate_container(
                            reference,
                            TargetType::Method,
                            config,
                            &format!("{path}.in"),
                        )?;
                    }
                    MethodPredicate::AnyOf(refs) => {
                        validate_refs(refs, TargetType::Method, config, &format!("{path}.anyOf"))?;
                    }
                    MethodPredicate::Not(inner) => {
                        validate_target(
                            &Target::Method((**inner).clone()),
                            config,
                            &format!("{path}.not"),
                        )?;
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn validate_container(
    reference: &TargetRef,
    child: TargetType,
    config: &ArchConfig,
    path: &str,
) -> Result<()> {
    if let TargetRef::Inline(target) = reference {
        validate_target(target, config, path)?;
    }
    let parent = resolve_target_type(reference, config, path)?;
    if !parent.can_contain(child) {
        return Err(ParseError::new(
            path,
            format!(
                "a {} target cannot be contained by a {} target",
                child.as_str(),
                parent.as_str()
            ),
        ));
    }
    Ok(())
}

fn validate_refs(
    refs: &[TargetRef],
    expected: TargetType,
    config: &ArchConfig,
    path: &str,
) -> Result<()> {
    for reference in refs {
        if let TargetRef::Inline(target) = reference {
            validate_target(target, config, path)?;
        }
        let actual = resolve_target_type(reference, config, path)?;
        if actual != expected {
            return Err(ParseError::new(
                path,
                format!(
                    "anyOf target type `{}` does not match `{}`",
                    actual.as_str(),
                    expected.as_str()
                ),
            ));
        }
    }
    Ok(())
}

fn validate_constraint(constraint: &Constraint, config: &ArchConfig, path: &str) -> Result<()> {
    match constraint {
        Constraint::DependOnlyOn(dependencies)
        | Constraint::NotDependOn(dependencies)
        | Constraint::Visible(dependencies) => {
            for dependency in dependencies {
                match dependency {
                    DependencyRef::None => {
                        if !matches!(constraint, Constraint::Visible(_)) {
                            return Err(ParseError::new(
                                format!("{path}.visible"),
                                "`none` is only valid in visible",
                            ));
                        }
                    }
                    DependencyRef::Any(reference)
                    | DependencyRef::Self_(reference)
                    | DependencyRef::Other(reference) => {
                        let TargetRef::Named(name) = reference else {
                            return Err(ParseError::new(
                                path,
                                "dependency targets must be named, not inline",
                            ));
                        };
                        if !config.targets.contains_key(name) {
                            return Err(ParseError::new(
                                path,
                                format!("unknown target name `{name}`"),
                            ));
                        }
                    }
                }
            }
        }
        Constraint::Children(children) => {
            for name in children {
                let target = TargetRef::Named(name.clone());
                let target_type = resolve_target_type(&target, config, path)?;
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
                let target_type = resolve_target_type(&target, config, path)?;
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
    resolve_target_type(target, config, path)
}

fn resolve_target_type(
    reference: &TargetRef,
    config: &ArchConfig,
    path: &str,
) -> Result<TargetType> {
    match reference {
        TargetRef::Named(name) => config
            .targets
            .get(name)
            .map(Target::target_type)
            .ok_or_else(|| ParseError::new(path, format!("unknown target name `{name}`"))),
        TargetRef::Inline(target) => Ok(target.target_type()),
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
        | Constraint::AnnotationArgument(_) => {
            matches!(target_type, TargetType::Class | TargetType::Method)
        }
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
        Constraint::Children(_) => "children",
        Constraint::Counts(_) => "counts",
        Constraint::Acyclic(_) => "acyclic",
    }
}
