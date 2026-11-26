use groovylint_arch::{
    ClassKind, ClassPredicate, ClassTarget, Constraint, Count, DependencyRef, FieldPredicate,
    Severity, SourceSet, Target, TargetRef, TargetType, parse,
};

const CONFIG: &str = r#"
version = 1

[project]
severity = "warn"

[targets]
domain-module   = { kind = "module", name = "*-domain" }
adapter-module  = { kind = "module", name = "*-adapter" }
domain-class    = { kind = "class", in = "domain-module" }
domain-service  = { kind = "class", in = "domain-module", name = ["*Service", "*ServiceImpl"], not = { classKind = "interface" } }
repository      = { kind = "class", in = "domain-module", name = "*Repository" }
repository-query = { kind = "method", in = "repository", name = "query*" }
domain-api      = { kind = "package", in = "domain-module", name = "**.domain.**.api" }
domain-api-class = { kind = "class", in = "domain-api" }
web             = { kind = "package", name = "org.springframework.web.**" }

[[rule]]
id = "domain-dependencies"
target = "domain-class"
because = "domain only depends on itself"

[rule.must]
dependOnlyOn = ["self:domain-module", "any:domain-module"]
notDependOn = ["web"]

[[rule]]
id = "service-placement"
target = "domain-service"
severity = "warn"
on = "test"

[rule.must]
notAnnotatedWith = ["Deprecated"]

[[rule]]
id = "repository-query-page"
target = "repository-query"

[rule.must]
rawReturnType = "com.acme.Page"

[[rule]]
id = "module-shape"
target = "domain-module"

[rule.must]
children = ["adapter-module", "domain-module"]
acyclic = true

[rule.must.counts]
"domain-module" = ">=1"
"adapter-module" = "<=1"

[[rule]]
id = "class-shape"
target = "domain-class"

[rule.must]
annotatedWith = ["com.acme.Marker"]

[rule.must.allMembers]
modifiers = ["private"]
"#;

fn class_target<'a>(config: &'a groovylint_arch::ArchConfig, name: &str) -> &'a ClassTarget {
    match &config.targets[name] {
        Target::Class(target) => target,
        other => panic!("expected class target, got {other:?}"),
    }
}

#[test]
fn parses_revised_targets_and_rules() {
    let config = parse(CONFIG).expect("configuration parses");
    assert_eq!(config.version, 1);
    assert_eq!(config.project.severity, Severity::Warn);
    assert_eq!(
        config.targets["domain-class"].target_type(),
        TargetType::Class
    );
    let service = class_target(&config, "domain-service");
    assert!(matches!(service.predicates[1], ClassPredicate::Name(_)));
    assert!(matches!(service.predicates[2], ClassPredicate::Not(_)));
    assert_eq!(
        class_target(&config, "domain-service").predicates[2],
        ClassPredicate::Not(Box::new(ClassTarget {
            predicates: vec![ClassPredicate::Kind(ClassKind::Interface)],
        }))
    );
    assert!(matches!(
        class_target(&config, "domain-class").predicates[0],
        ClassPredicate::In(TargetRef::Named(_))
    ));
    assert_eq!(config.rules.len(), 5);
    assert_eq!(config.rules[0].severity, Severity::Error);
    assert_eq!(config.rules[1].severity, Severity::Warn);
    assert_eq!(config.rules[1].on, SourceSet::Test);
    assert!(matches!(
        config.rules[2].must.constraints[0],
        Constraint::RawReturnType(_)
    ));
    assert!(
        config.rules[3]
            .must
            .constraints
            .iter()
            .any(|constraint| matches!(constraint, Constraint::Children(_)))
    );
    assert!(
        config.rules[3]
            .must
            .constraints
            .iter()
            .any(|constraint| matches!(constraint, Constraint::Counts(_)))
    );
    assert!(
        config.rules[3]
            .must
            .constraints
            .iter()
            .any(|constraint| matches!(constraint, Constraint::Acyclic(true)))
    );
    assert!(
        config.rules[4]
            .must
            .constraints
            .iter()
            .any(|constraint| matches!(constraint, Constraint::AllMembers(_)))
    );
}

#[test]
fn parses_dependency_prefixes() {
    let config = parse(CONFIG).expect("configuration parses");
    let Constraint::DependOnlyOn(dependencies) = config.rules[0]
        .must
        .constraints
        .iter()
        .find(|constraint| matches!(constraint, Constraint::DependOnlyOn(_)))
        .expect("expected dependOnlyOn")
    else {
        panic!("expected dependOnlyOn");
    };
    assert!(matches!(dependencies[0], DependencyRef::Self_(_)));
    assert!(matches!(dependencies[1], DependencyRef::Any(_)));
    assert!(matches!(
        &config.rules[1].must.constraints[0],
        Constraint::NotAnnotatedWith(_)
    ));
}

#[test]
fn rejects_class_contained_by_class() {
    let error = parse(
        r#"
version = 1
[targets]
domain = { kind = "class", name = "**" }
service = { kind = "class", in = "domain" }
"#,
    )
    .expect_err("class cannot contain class");
    assert!(error.to_string().contains("cannot be contained by"));
}

#[test]
fn parses_field_type_selector() {
    let config = parse(
        r#"
version = 1
[targets]
field = { kind = "field", type = "String" }
"#,
    )
    .expect("type is the field selector");
    let Target::Field(field) = &config.targets["field"] else {
        panic!("expected field");
    };
    assert!(matches!(field.predicates[0], FieldPredicate::Type(_)));
}

#[test]
fn rejects_unknown_keys_and_bad_constraints() {
    let error = parse(
        r#"
version = 1
[targets]
domain = { kind = "class", nmae = "*" }
"#,
    )
    .expect_err("unknown selector");
    assert!(error.to_string().contains("nmae"));

    let error = parse(
        r#"
version = 1
[targets]
domain = { kind = "class", name = "*" }
[[rule]]
id = "bad"
target = "domain"
must = { counts = { domain = ">=1" } }
"#,
    )
    .expect_err("counts on class");
    assert!(error.to_string().contains("not valid"));
}

#[test]
fn parses_inline_rule_target() {
    let config = parse(
        r#"
version = 1
[targets]
domain-module = { kind = "module", name = "*-domain" }
[[rule]]
id = "inline"
target = { kind = "class", in = "domain-module" }
must = { exist = false }
"#,
    )
    .expect("inline target");
    assert!(matches!(config.rules[0].target, TargetRef::Inline(_)));
}

#[test]
fn parses_counts_forms() {
    let config = parse(
        r#"
version = 1
[targets]
m = { kind = "module", name = "**" }
d = { kind = "module", name = "*-domain" }
a = { kind = "module", name = "*-adapter" }
[[rule]]
id = "shape"
target = "m"
must = { counts = { d = ">=1", a = "<=1" } }
"#,
    )
    .expect("counts parse");
    let Constraint::Counts(counts) = &config.rules[0].must.constraints[0] else {
        panic!("counts");
    };
    assert!(matches!(counts["d"], Count::AtLeast(1)));
    assert!(matches!(counts["a"], Count::AtMost(1)));
}

#[test]
fn parses_nested_target_tables() {
    let config = parse(
        r#"
version = 1

[targets.domain-module]
kind = "module"
name = "*-domain"

[targets.domain-class]
kind = "class"
in = "domain-module"

[targets.domain-service]
kind = "class"
in = "domain-module"
name = ["*Service", "*ServiceImpl"]

[targets.domain-service.not]
classKind = "interface"
"#,
    )
    .expect("nested target tables");
    assert_eq!(
        config.targets["domain-class"].target_type(),
        TargetType::Class
    );
    let Target::Class(service) = &config.targets["domain-service"] else {
        panic!("expected class");
    };
    assert!(matches!(
        service.predicates.last(),
        Some(ClassPredicate::Not(_))
    ));
}
