//! `CodeNarc` security rules.

use groovylint_core::Severity;
use groovylint_syntax::ast::{ExprKind, Item};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, FileRule, NodeKind, NodeMask, NodeRef, Rule,
    RuleMeta,
};

fn simple_name(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

/// Flags calls to a method on a named type, e.g. `System.exit(0)`.
pub struct StaticCallRule {
    type_names: &'static [&'static str],
    method: &'static str,
    message: &'static str,
}

impl Rule for StaticCallRule {}

impl AstRule for StaticCallRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let ExprKind::Call {
            receiver: Some(receiver),
            name,
            ..
        } = &expr.kind
        else {
            return;
        };
        if name != self.method {
            return;
        }
        let ExprKind::Variable(owner) = &receiver.kind else {
            return;
        };
        if !self.type_names.contains(&owner.as_str()) {
            return;
        }
        sink.report(expr.span, self.message);
    }
}

/// Flags `new` allocations of a named type.
pub struct ForbiddenAllocationRule {
    type_names: &'static [&'static str],
    message: &'static str,
    skip_test_files: bool,
}

impl Rule for ForbiddenAllocationRule {}

impl AstRule for ForbiddenAllocationRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        if self.skip_test_files
            && crate::rules::util::is_test_file(context.source.path().to_string_lossy().as_ref())
        {
            return;
        }
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let ExprKind::New { type_name, .. } = &expr.kind else {
            return;
        };
        if !self.type_names.contains(&simple_name(type_name)) {
            return;
        }
        sink.report(expr.span, self.message);
    }
}

/// Flags `[a: 1] as Interface` coercions.
pub struct UnsafeImplementationAsMapRule;

impl Rule for UnsafeImplementationAsMapRule {}

impl AstRule for UnsafeImplementationAsMapRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        // `CodeNarc` does not apply this rule to test sources.
        if crate::rules::util::is_test_file(context.source.path().to_string_lossy().as_ref()) {
            return;
        }
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let ExprKind::Cast {
            expr: inner,
            type_name,
        } = &expr.kind
        else {
            return;
        };
        if !matches!(inner.kind, ExprKind::Map(_)) {
            return;
        }
        if matches!(simple_name(type_name), "Map" | "HashMap" | "LinkedHashMap") {
            return;
        }
        sink.report(
            expr.span,
            format!(
                "Creating an implementation of {type_name} from a Map is unsafe unless every member is implemented"
            ),
        );
    }
}

/// Flags public fields that are not `final`.
pub struct NonFinalPublicFieldRule;

impl Rule for NonFinalPublicFieldRule {}

impl FileRule for NonFinalPublicFieldRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for item in &context.script.items {
            let Item::Class(class) = item else {
                continue;
            };
            for member in &class.members {
                let Item::Field(field) = member else {
                    continue;
                };
                if !field.is_public || field.is_final {
                    continue;
                }
                sink.report(
                    field.span,
                    format!(
                        "The field {} is public but not final, which violates secure coding principles",
                        field.name
                    ),
                );
            }
        }
    }
}

/// Flags calls to `finalize()`; only the JVM should call it.
pub struct ObjectFinalizeCallRule;

impl Rule for ObjectFinalizeCallRule {}

impl AstRule for ObjectFinalizeCallRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Expr(expr) = node else {
            return;
        };
        let ExprKind::Call { name, args, .. } = &expr.kind else {
            return;
        };
        if name != "finalize" || !args.is_empty() {
            return;
        }
        sink.report(
            expr.span,
            "The finalize() method should only be called by the JVM",
        );
    }
}

/// Flags `finalize()` methods, optionally only when they are public.
pub struct FinalizeRule {
    require_not_private: bool,
}

impl Rule for FinalizeRule {}

impl FileRule for FinalizeRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for item in &context.script.items {
            let Item::Class(class) = item else {
                continue;
            };
            for member in &class.members {
                let Item::Method(method) = member else {
                    continue;
                };
                if method.name != "finalize" {
                    continue;
                }
                if self.require_not_private && !method.is_private {
                    sink.report(
                        method.span,
                        format!(
                            "The finalize() method of class {} should not be declared public",
                            class.name
                        ),
                    );
                    continue;
                }
                if !self.require_not_private {
                    // The declaration check lives in `PublicFinalizeMethod`; this
                    // rule reports calls, see `ObjectFinalizeCallRule`.
                }
            }
        }
    }
}

/// Flags `public static final` array declarations.
pub struct UnsafeArrayDeclarationRule;

impl Rule for UnsafeArrayDeclarationRule {}

impl FileRule for UnsafeArrayDeclarationRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for item in &context.script.items {
            let Item::Class(class) = item else {
                continue;
            };
            for member in &class.members {
                let Item::Field(field) = member else {
                    continue;
                };
                let Some(type_name) = &field.type_name else {
                    continue;
                };
                if !type_name.ends_with("[]") {
                    continue;
                }
                if !field.is_public || !field.is_static || !field.is_final {
                    continue;
                }
                sink.report(
                    field.span,
                    format!(
                        "The array field {} is public, static and final; arrays are mutable objects",
                        field.name
                    ),
                );
            }
        }
    }
}

/// Flags non-final subclasses of sensitive security types.
pub struct NonFinalSubclassOfSensitiveInterfaceRule;

const SENSITIVE_TYPES: &[&str] = &[
    "Permission",
    "BasicPermission",
    "PrivilegedAction",
    "PrivilegedActionException",
];

impl Rule for NonFinalSubclassOfSensitiveInterfaceRule {}

impl FileRule for NonFinalSubclassOfSensitiveInterfaceRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for item in &context.script.items {
            let Item::Class(class) = item else {
                continue;
            };
            let mut derives = false;
            if let Some(parent) = &class.extends
                && SENSITIVE_TYPES.contains(&simple_name(parent))
            {
                derives = true;
            }
            for interface in &class.implements {
                if SENSITIVE_TYPES.contains(&simple_name(interface)) {
                    derives = true;
                }
            }
            if !derives || class.is_abstract {
                continue;
            }
            sink.report(
                class.span,
                format!(
                    "The class {} derives from a sensitive type and should be final",
                    class.name
                ),
            );
        }
    }
}

/// Flags direct java.io file access.
pub struct JavaIoPackageAccessRule;

impl Rule for JavaIoPackageAccessRule {}

impl AstRule for JavaIoPackageAccessRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Expr(expr) = node else {
            return;
        };
        match &expr.kind {
            ExprKind::New { type_name, .. } => {
                let name = simple_name(type_name);
                if matches!(
                    name,
                    "File" | "FileOutputStream" | "FileReader" | "RandomAccessFile"
                ) {
                    sink.report(
                        expr.span,
                        format!("The use of java.io.{name} violates the EJB specification"),
                    );
                }
            }
            ExprKind::Property { receiver, name, .. }
                if name == "FileSystem"
                    && matches!(&receiver.kind, ExprKind::Variable(variable) if variable == "FileSystem") =>
            {
                sink.report(
                    expr.span,
                    "The use of java.io.FileSystem violates the EJB specification",
                );
            }
            _ => {}
        }
    }
}

/// Descriptors for security rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "JavaIoPackageAccess",
                "org.codenarc.rule.security.JavaIoPackageAccessRule",
                Category::Security,
                Severity::Warning,
                "Direct java.io access violates the EJB specification.",
            ),
            || RuleInstanceImpl::Ast(Box::new(JavaIoPackageAccessRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SystemExit",
                "org.codenarc.rule.security.SystemExitRule",
                Category::Security,
                Severity::Warning,
                "Checks for calls to System.exit().",
            ),
            || {
                RuleInstanceImpl::Ast(Box::new(StaticCallRule {
                    type_names: &["System", "java.lang.System"],
                    method: "exit",
                    message: "System.exit() should not be called",
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "InsecureRandom",
                "org.codenarc.rule.security.InsecureRandomRule",
                Category::Security,
                Severity::Warning,
                "Checks for java.util.Random usage.",
            ),
            || {
                RuleInstanceImpl::Ast(Box::new(ForbiddenAllocationRule {
                    type_names: &["Random"],
                    message: "Using java.util.Random is insecure - use java.security.SecureRandom instead",
                    skip_test_files: true,
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "NonFinalPublicField",
                "org.codenarc.rule.security.NonFinalPublicFieldRule",
                Category::Security,
                Severity::Warning,
                "Checks for public fields that are not final.",
            ),
            || RuleInstanceImpl::File(Box::new(NonFinalPublicFieldRule)),
        ),
        descriptor(
            RuleMeta::new(
                "ObjectFinalize",
                "org.codenarc.rule.security.ObjectFinalizeRule",
                Category::Security,
                Severity::Warning,
                "Checks for finalize() methods.",
            ),
            || RuleInstanceImpl::Ast(Box::new(ObjectFinalizeCallRule)),
        ),
        descriptor(
            RuleMeta::new(
                "PublicFinalizeMethod",
                "org.codenarc.rule.security.PublicFinalizeMethodRule",
                Category::Security,
                Severity::Warning,
                "Checks for public finalize() methods.",
            ),
            || {
                RuleInstanceImpl::File(Box::new(FinalizeRule {
                    require_not_private: true,
                }))
            },
        ),
        descriptor(
            RuleMeta::new(
                "UnsafeArrayDeclaration",
                "org.codenarc.rule.security.UnsafeArrayDeclarationRule",
                Category::Security,
                Severity::Warning,
                "Checks for public static final arrays.",
            ),
            || RuleInstanceImpl::File(Box::new(UnsafeArrayDeclarationRule)),
        ),
        descriptor(
            RuleMeta::new(
                "NonFinalSubclassOfSensitiveInterface",
                "org.codenarc.rule.security.NonFinalSubclassOfSensitiveInterfaceRule",
                Category::Security,
                Severity::Warning,
                "Checks for subclasses of sensitive security types.",
            ),
            || RuleInstanceImpl::File(Box::new(NonFinalSubclassOfSensitiveInterfaceRule)),
        ),
        descriptor(
            RuleMeta::new(
                "UnsafeImplementationAsMap",
                "org.codenarc.rule.security.UnsafeImplementationAsMapRule",
                Category::Security,
                Severity::Warning,
                "Checks for map-to-interface coercions.",
            ),
            || RuleInstanceImpl::Ast(Box::new(UnsafeImplementationAsMapRule)),
        ),
        descriptor(
            RuleMeta::new(
                "FileCreateTempFile",
                "org.codenarc.rule.security.FileCreateTempFileRule",
                Category::Security,
                Severity::Warning,
                "Checks for File.createTempFile() calls.",
            ),
            || {
                RuleInstanceImpl::Ast(Box::new(StaticCallRule {
                    type_names: &["File", "java.io.File"],
                    method: "createTempFile",
                    message: "File.createTempFile() is insecure",
                }))
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::simple_name;

    #[test]
    fn simple_name_strips_packages() {
        assert_eq!(simple_name("java.util.Random"), "Random");
        assert_eq!(simple_name("Random"), "Random");
    }
}
