use std::fs;
use std::path::Path;

use groovylint_arch::{Project, check, collect, module_for, parse as parse_config};
use groovylint_core::SourceFile;
use groovylint_syntax::parser::parse;

fn project(root: &Path, sources: &[(&str, &str)]) -> Project {
    let mut files = Vec::new();
    for (relative, text) in sources {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create source dir");
        }
        fs::write(&path, text).expect("write source");
        let source = SourceFile::new(path.clone(), *text);
        let parsed = parse(source.text()).expect("source parses");
        let module = module_for(&path, root);
        files.push(collect(&path, &module, &parsed.script));
    }
    Project::new(root, files)
}

#[test]
fn detects_forbidden_class_dependency() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    let sources = [
        ("acme-billing-domain/build.gradle", ""),
        (
            "acme-billing-domain/src/main/groovy/com/acme/domain/OrderService.groovy",
            "package com.acme.domain\nclass OrderService {}\n",
        ),
        ("acme-billing-adapter/build.gradle", ""),
        (
            "acme-billing-adapter/src/main/groovy/com/acme/adapter/OrderController.groovy",
            "package com.acme.adapter\nimport com.acme.domain.OrderService\nclass OrderController {\n  OrderService service\n}\n",
        ),
    ];
    let project = project(root, &sources);
    let config = parse_config(
        r#"
version = 1
[targets]
domain-module = { type = "module", name = "*-domain" }
adapter-module = { type = "module", name = "*-adapter" }
domain-class = { type = "class", in = "domain-module" }
adapter-class = { type = "class", in = "adapter-module" }
[[rule]]
id = "adapter-must-not-reach-domain"
target = "adapter-class"
because = "adapter calls application, not domain"
[rule.must]
notDependOn = ["domain-class"]
"#,
    )
    .expect("config parses");
    let findings = check(&config, &project);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "adapter-must-not-reach-domain"),
        "{findings:?}"
    );
}

#[test]
fn checks_annotations_and_counts() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();
    let sources = [
        ("app-domain/build.gradle", ""),
        ("app-adapter/build.gradle", ""),
        (
            "app-domain/src/main/groovy/com/acme/domain/OrderService.groovy",
            "package com.acme.domain\n@Service\nclass OrderService {\n  private String name\n  void run() {}\n}\n",
        ),
        (
            "app-adapter/src/main/groovy/com/acme/adapter/Controller.groovy",
            "package com.acme.adapter\nclass Controller {}\n",
        ),
    ];
    let project = project(root, &sources);
    let config = parse_config(
        r#"
version = 1
[targets]
domain-module = { type = "module", name = "*-domain" }
domain-class = { type = "class", in = "domain-module" }
service = { type = "class", in = "domain-module", name = "*Service" }
[[rule]]
id = "services-are-components"
target = "service"
[rule.must]
annotatedWith = ["Service"]
[rule.must.allFields]
modifiers = ["private"]
[[rule]]
id = "domain-shape"
target = "domain-module"
[rule.must]
children = ["domain-module"]
[rule.must.counts]
"domain-module" = ">=1"
"#,
    )
    .expect("config parses");
    let findings = check(&config, &project);
    assert!(
        findings
            .iter()
            .all(|finding| finding.rule != "services-are-components"),
        "service annotation and private field should pass: {findings:?}"
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == "domain-shape"),
        "domain module has no domain child: {findings:?}"
    );
}
