use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

struct RuleCase {
    rule: &'static str,
    line: u64,
}

const CASES: &[RuleCase] = &[
    case("AssertWithinFinallyBlock", 6),
    case("AssignmentInConditional", 2),
    case("BigDecimalInstantiation", 1),
    case("BitwiseOperatorInConditional", 1),
    case("BooleanGetBoolean", 1),
    case("BrokenNullCheck", 1),
    case("BrokenOddnessCheck", 1),
    case("ClassForName", 1),
    case("ComparisonOfTwoConstants", 1),
    case("ComparisonWithSelf", 1),
    case("ConstantAssertExpression", 1),
    case("ConstantIfExpression", 1),
    case("ConstantTernaryExpression", 1),
    case("DeadCode", 3),
    case("DoubleNegative", 1),
    case("DuplicateCaseStatement", 5),
    case("DuplicateMapKey", 1),
    case("DuplicateSetValue", 1),
    case("EmptyClass", 1),
    case("EmptyInstanceInitializer", 2),
    case("EmptyMethod", 2),
    case("EmptyStaticInitializer", 2),
    case("EmptySwitchStatement", 1),
    case("EmptySynchronizedStatement", 2),
    case("EqualsAndHashCode", 1),
    case("EqualsOverloaded", 2),
    case("ExplicitGarbageCollection", 1),
    case("ForLoopShouldBeWhileLoop", 1),
    case("HardCodedWindowsFileSeparator", 1),
    case("HardCodedWindowsRootDirectory", 1),
    case("IntegerGetInteger", 1),
    case("MultipleUnaryOperators", 1),
    case("RemoveAllOnSelf", 2),
    case("RandomDoubleCoercedToZero", 1),
    case("ParameterAssignmentInFilterClosure", 1),
    case("ReturnFromFinallyBlock", 5),
    case("ThrowExceptionFromFinallyBlock", 5),
    case("UnnecessarySemicolon", 1),
    case("EmptyIfStatement", 1),
    case("EmptyElseBlock", 3),
    case("EmptyWhileStatement", 1),
    case("EmptyForStatement", 1),
    case("EmptyTryBlock", 1),
    case("EmptyCatchBlock", 3),
    case("EmptyFinallyBlock", 3),
    case("NoWildcardImports", 1),
    case("UnnecessaryGroovyImport", 1),
    case("ImportFromSamePackage", 3),
    case("ImportFromSunPackages", 1),
    case("UnusedImport", 1),
    case("TrailingWhitespace", 1),
    case("IfStatementBraces", 3),
    case("Println", 3),
    case("NoDef", 3),
    case("VariableTypeRequired", 3),
    case("MethodReturnTypeRequired", 2),
    case("MethodName", 2),
    case("ClassNameSameAsFilename", 1),
    case("UnusedPrivateField", 2),
    case("UnusedPrivateMethod", 2),
    case("DuplicateImport", 2),
    case("SerialVersionUID", 2),
    case("SystemExit", 3),
    case("ConfusingTernary", 3),
    case("ParameterReassignment", 3),
    case("ImplicitReturnStatement", 6),
    case("MissingOverrideAnnotation", 2),
    case("InsecureRandom", 3),
    case("UnusedObject", 3),
    case("MultipleLoggers", 3),
];

const fn case(rule: &'static str, line: u64) -> RuleCase {
    RuleCase { rule, line }
}

fn codenarc_class(case: &RuleCase) -> String {
    let category = match case.rule {
        "UnnecessarySemicolon" => "unnecessary",
        "NoWildcardImports"
        | "UnnecessaryGroovyImport"
        | "ImportFromSamePackage"
        | "ImportFromSunPackages"
        | "UnusedImport"
        | "DuplicateImport" => "imports",
        "TrailingWhitespace" => "formatting",
        "IfStatementBraces" | "ForStatementBraces" | "WhileStatementBraces" | "ElseBlockBraces" => {
            "braces"
        }
        "Println"
        | "SystemOutPrint"
        | "SystemErrPrint"
        | "PrintStackTrace"
        | "MultipleLoggers"
        | "LoggerWithWrongModifiers"
        | "LoggerForDifferentClass"
        | "LoggingSwallowsStacktrace" => "logging",
        "NoDef"
        | "VariableTypeRequired"
        | "FieldTypeRequired"
        | "MethodReturnTypeRequired"
        | "MethodParameterTypeRequired"
        | "ParameterReassignment"
        | "ImplicitReturnStatement"
        | "ConfusingTernary"
        | "InvertedIfElse"
        | "NoFloat"
        | "NoDouble"
        | "HashtableIsObsolete"
        | "VectorIsObsolete"
        | "LongLiteralWithLowerCaseL"
        | "NoTabCharacter" => "convention",
        "MethodName"
        | "VariableName"
        | "PropertyName"
        | "ParameterName"
        | "ClassName"
        | "InterfaceName"
        | "ClassNameSameAsFilename"
        | "ClassNameSameAsSuperclass"
        | "ConfusingMethodName"
        | "ObjectOverrideMisspelledMethodName" => "naming",
        "UnusedPrivateField"
        | "UnusedPrivateMethod"
        | "UnusedPrivateMethodParameter"
        | "UnusedObject"
        | "UnusedArray"
        | "UnusedVariable" => "unused",
        "SerialVersionUID" | "SerialPersistentFields" | "EnumCustomSerializationIgnored" => {
            "serialization"
        }
        "SystemExit"
        | "InsecureRandom"
        | "NonFinalPublicField"
        | "ObjectFinalize"
        | "PublicFinalizeMethod"
        | "UnsafeArrayDeclaration"
        | "NonFinalSubclassOfSensitiveInterface"
        | "UnsafeImplementationAsMap"
        | "FileCreateTempFile" => "security",
        "MissingOverrideAnnotation" => "enhanced",
        _ => "basic",
    };
    format!("org.codenarc.rule.{category}.{}Rule", case.rule)
}

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_groovylint"))
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rules")
}

fn run_case(case: &RuleCase, temp: &Path) -> (String, String, serde_json::Value) {
    let config = temp.join(format!("{}.xml", case.rule));
    fs::write(
        &config,
        format!(
            "<ruleset><rule class='{}'/></ruleset>",
            codenarc_class(case)
        ),
    )
    .expect("write config");

    let fixture = fixture_dir().join(format!("{}.groovy", case.rule));
    let output = binary()
        .arg("check")
        .arg("--format")
        .arg("json")
        .arg("--config")
        .arg(&config)
        .arg(&fixture)
        .output()
        .expect("run groovylint");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let json = serde_json::from_str(&stdout).unwrap_or_else(|error| {
        panic!(
            "invalid JSON for {}: {error}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            case.rule
        )
    });
    (stdout, stderr, json)
}

#[test]
fn every_rule_has_a_minimal_end_to_end_fixture() {
    let temp = TempDir::new().expect("tempdir");
    for case in CASES {
        let (stdout, stderr, json) = run_case(case, temp.path());
        let diagnostics = json["files"][0]["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("no diagnostics array for {}", case.rule));

        assert_eq!(
            diagnostics.len(),
            1,
            "{}: expected exactly one diagnostic\nstdout:\n{stdout}\nstderr:\n{stderr}",
            case.rule
        );
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic["rule"], case.rule, "{}", case.rule);
        assert_eq!(diagnostic["start"]["line"], case.line, "{}", case.rule);
    }
}
