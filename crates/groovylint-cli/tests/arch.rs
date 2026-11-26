use std::fs;
use std::process::Command;

use tempfile::TempDir;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_groovylint"))
}

#[test]
fn reports_architecture_dependency_violation() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    fs::write(root.join("build.gradle"), "").expect("root build file");
    fs::create_dir_all(root.join("acme-billing-domain/src/main/groovy/com/acme/domain"))
        .expect("domain dirs");
    fs::create_dir_all(root.join("acme-billing-adapter/src/main/groovy/com/acme/adapter"))
        .expect("adapter dirs");
    fs::write(root.join("acme-billing-domain/build.gradle"), "").expect("domain build file");
    fs::write(root.join("acme-billing-adapter/build.gradle"), "").expect("adapter build file");
    fs::write(
        root.join("acme-billing-domain/src/main/groovy/com/acme/domain/OrderService.groovy"),
        "package com.acme.domain\nclass OrderService {}\n",
    )
    .expect("domain source");
    fs::write(
        root.join("acme-billing-adapter/src/main/groovy/com/acme/adapter/OrderController.groovy"),
        "package com.acme.adapter\nimport com.acme.domain.OrderService\nclass OrderController {\n  OrderService service\n}\n",
    )
    .expect("adapter source");
    let config = root.join("architecture.toml");
    fs::write(
        &config,
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
[rule.must]
notDependOn = ["domain-class"]
"#,
    )
    .expect("config");

    let output = binary()
        .arg("arch")
        .arg("--config")
        .arg(&config)
        .arg("--format")
        .arg("json")
        .arg(root)
        .output()
        .expect("run arch");
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid JSON output");
    assert_eq!(
        json["files"][0]["diagnostics"][0]["rule"],
        "adapter-must-not-reach-domain"
    );
}
