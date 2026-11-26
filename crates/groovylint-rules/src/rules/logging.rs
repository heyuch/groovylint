//! `CodeNarc` logging rules.

use groovylint_core::Severity;
use groovylint_syntax::ast::{ClassDecl, Expr, ExprKind, Item};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, NodeKind, NodeMask, NodeRef, Rule, RuleMeta,
};
use crate::rules::util::ClassStack;

fn declares_println(class: &ClassDecl) -> bool {
    for member in &class.members {
        match member {
            Item::Method(method) if method.name == "println" || method.name == "print" => {
                return true;
            }
            Item::Field(field) if field.name == "println" || field.name == "print" => {
                return true;
            }
            _ => {}
        }
    }
    false
}

fn is_implicit_this(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::This => true,
        ExprKind::Variable(name) => name == "this",
        _ => false,
    }
}

/// Matches a call on `this` and returns its method name and arguments.
fn this_call(expr: &Expr) -> Option<(&str, &[Expr])> {
    let ExprKind::Call {
        receiver,
        name,
        args,
        safe: false,
    } = &expr.kind
    else {
        return None;
    };
    if let Some(receiver) = receiver
        && !is_implicit_this(receiver)
    {
        return None;
    }
    Some((name.as_str(), args))
}

/// Matches `System.<stream>.<method>(...)`.
fn system_stream_call<'a>(expr: &'a Expr, stream: &'a str) -> Option<(&'a str, &'a [Expr])> {
    let ExprKind::Call {
        receiver: Some(receiver),
        name,
        args,
        safe: false,
    } = &expr.kind
    else {
        return None;
    };
    let ExprKind::Property {
        receiver: owner,
        name: stream_name,
        safe: false,
    } = &receiver.kind
    else {
        return None;
    };
    if stream_name != stream {
        return None;
    }
    match &owner.kind {
        ExprKind::Variable(owner_name) if owner_name == "System" => Some((name.as_str(), args)),
        _ => None,
    }
}

/// Flags `println`/`print`/`printf` calls that write to standard output.
#[derive(Default)]
pub struct PrintlnRule {
    /// Whether an enclosing class shadows `println`/`print`.
    shadowed: Vec<bool>,
}

impl Rule for PrintlnRule {}

impl AstRule for PrintlnRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Expr).union(NodeMask::of(NodeKind::Class))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        match node {
            NodeRef::Class(class) => {
                self.shadowed.push(declares_println(class));
            }
            NodeRef::Expr(expr) => {
                if self.shadowed.iter().any(|shadowed| *shadowed) {
                    return;
                }
                let Some((name, args)) = this_call(expr) else {
                    return;
                };
                let is_print = match name {
                    "println" => args.len() <= 1,
                    "print" => args.len() == 1,
                    "printf" => args.len() > 1,
                    _ => false,
                };
                if is_print {
                    sink.report(
                        expr.span,
                        "println should be replaced with something more robust",
                    );
                }
            }
            _ => {}
        }
    }

    fn exit(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        _sink: &mut DiagnosticSink<'_>,
    ) {
        if matches!(node, NodeRef::Class(_)) {
            self.shadowed.pop();
        }
    }
}

/// Flags `System.out.print*` calls.
pub struct SystemOutPrintRule;

impl Rule for SystemOutPrintRule {}

impl AstRule for SystemOutPrintRule {
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
        let Some((name, _)) = system_stream_call(expr, "out") else {
            return;
        };
        if !name.starts_with("print") {
            return;
        }
        sink.report(
            expr.span,
            format!("System.out.{name} should be replaced with something more robust"),
        );
    }
}

/// Flags `System.err.print*` calls.
pub struct SystemErrPrintRule;

impl Rule for SystemErrPrintRule {}

impl AstRule for SystemErrPrintRule {
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
        let Some((name, _)) = system_stream_call(expr, "err") else {
            return;
        };
        if !name.starts_with("print") {
            return;
        }
        sink.report(
            expr.span,
            format!("System.err.{name} should be replaced with something more robust"),
        );
    }
}

/// Flags `printStackTrace()` and `printSanitizedStackTrace(x)` calls.
pub struct PrintStackTraceRule;

impl Rule for PrintStackTraceRule {}

impl AstRule for PrintStackTraceRule {
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
        if name == "printStackTrace" && args.is_empty() {
            sink.report(
                expr.span,
                "printStackTrace() should be replaced with something more robust",
            );
        }
        if name == "printSanitizedStackTrace" && args.len() == 1 {
            sink.report(
                expr.span,
                "printSanitizedStackTrace() should be replaced with something more robust",
            );
        }
    }
}

/// Descriptors for logging rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "Println",
                "org.codenarc.rule.logging.PrintlnRule",
                Category::Logging,
                Severity::Warning,
                "Checks for println calls.",
            ),
            || RuleInstanceImpl::Ast(Box::new(PrintlnRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "SystemOutPrint",
                "org.codenarc.rule.logging.SystemOutPrintRule",
                Category::Logging,
                Severity::Warning,
                "Checks for System.out.print calls.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SystemOutPrintRule)),
        ),
        descriptor(
            RuleMeta::new(
                "SystemErrPrint",
                "org.codenarc.rule.logging.SystemErrPrintRule",
                Category::Logging,
                Severity::Warning,
                "Checks for System.err.print calls.",
            ),
            || RuleInstanceImpl::Ast(Box::new(SystemErrPrintRule)),
        ),
        descriptor(
            RuleMeta::new(
                "PrintStackTrace",
                "org.codenarc.rule.logging.PrintStackTraceRule",
                Category::Logging,
                Severity::Warning,
                "Checks for printStackTrace calls.",
            ),
            || RuleInstanceImpl::Ast(Box::new(PrintStackTraceRule)),
        ),
    ]
}

fn simple_name(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

/// Matches `Logger log = LoggerFactory.getLogger(...)` style declarations, the
/// only ones `CodeNarc`'s `LogUtil` recognises: the initializer must be a
/// direct call on a logger factory, so a cast or another receiver hides it.
fn is_factory_logger(field: &groovylint_syntax::ast::FieldDecl) -> bool {
    let Some(init) = &field.init else {
        return false;
    };
    let ExprKind::Call {
        receiver: Some(receiver),
        name,
        ..
    } = &init.kind
    else {
        return false;
    };
    if name != "getLogger" && name != "getLog" {
        return false;
    }
    let ExprKind::Variable(owner) = &receiver.kind else {
        return false;
    };
    matches!(
        simple_name(owner),
        "Logger" | "LogFactory" | "LoggerFactory" | "Log"
    )
}

fn logger_fields(class: &ClassDecl) -> Vec<&groovylint_syntax::ast::FieldDecl> {
    class
        .members
        .iter()
        .filter_map(|member| match member {
            Item::Field(field) if is_factory_logger(field) => Some(field),
            _ => None,
        })
        .collect()
}

/// Flags classes that declare more than one logger.
pub struct MultipleLoggersRule;

impl Rule for MultipleLoggersRule {}

impl AstRule for MultipleLoggersRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Class)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Class(class) = node else {
            return;
        };
        for field in logger_fields(class).into_iter().skip(1) {
            sink.report(
                field.span,
                format!(
                    "The class {} has multiple logger fields; only one is needed",
                    class.name
                ),
            );
        }
    }
}

/// Flags logger fields that are not `private static final`.
pub struct LoggerWithWrongModifiersRule;

impl Rule for LoggerWithWrongModifiersRule {}

impl AstRule for LoggerWithWrongModifiersRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Field)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Field(field) = node else {
            return;
        };
        if !is_factory_logger(field) {
            return;
        }
        let standard = field.is_private && field.is_static && field.is_final;
        let protected_style = !field.is_private && !field.is_static && field.is_final;
        if standard || protected_style {
            return;
        }
        sink.report(
            field.span,
            format!(
                "The logger field {} should be private, static and final",
                field.name
            ),
        );
    }
}

/// Flags loggers created for a different class.
#[derive(Default)]
pub struct LoggerForDifferentClassRule {
    classes: ClassStack,
}

impl Rule for LoggerForDifferentClassRule {}

impl AstRule for LoggerForDifferentClassRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Field).union(NodeMask::of(NodeKind::Class))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        match node {
            NodeRef::Class(_) => self.classes.enter(node),
            NodeRef::Field(field) => {
                if !is_factory_logger(field) {
                    return;
                }
                let Some(init) = &field.init else {
                    return;
                };
                let ExprKind::Call { args, .. } = &init.kind else {
                    return;
                };
                let Some(first) = args.first() else {
                    return;
                };
                // A logger named by a string is not tied to this class either.
                if let ExprKind::Literal(groovylint_syntax::ast::Literal::String(name)) =
                    &first.kind
                {
                    sink.report(
                        init.span,
                        format!("The logger for {name} should be created for the class it is declared in"),
                    );
                    return;
                }
                // `getLogger(Foo)`, `getLogger(Foo.class)`, `getLogger(Foo.class.name)`
                // and `getLogger(Foo.name)` all name the same class; anything else
                // (a computed name) is left alone.
                let Some(owner) = logger_owner(first) else {
                    return;
                };
                if owner == self.classes.current() || owner.is_empty() {
                    return;
                }
                sink.report(
                    init.span,
                    format!(
                        "The logger for {owner} should be created for the class it is declared in"
                    ),
                );
            }
            _ => {}
        }
    }

    fn exit(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        _sink: &mut DiagnosticSink<'_>,
    ) {
        if matches!(node, NodeRef::Class(_)) {
            self.classes.exit(node);
        }
    }
}

/// The class a logger argument names, when the argument is just a name.
fn logger_owner(argument: &Expr) -> Option<&str> {
    match &argument.kind {
        ExprKind::Variable(name) => Some(name.as_str()),
        ExprKind::Property {
            receiver,
            name,
            safe: false,
        } if name == "class" || name == "name" => match &receiver.kind {
            ExprKind::Variable(owner) => Some(owner.as_str()),
            ExprKind::Property {
                receiver: inner,
                name: property,
                ..
            } if property == "class" => match &inner.kind {
                ExprKind::Variable(owner) => Some(owner.as_str()),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

/// Flags logging calls inside a `catch` block that drop the exception.
pub struct LoggingSwallowsStacktraceRule;

impl Rule for LoggingSwallowsStacktraceRule {}

impl AstRule for LoggingSwallowsStacktraceRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::Try)
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        let NodeRef::Stmt(groovylint_syntax::ast::Stmt::Try(try_stmt)) = node else {
            return;
        };
        for catch in &try_stmt.catches {
            let Some(exception_name) = &catch.param_name else {
                continue;
            };
            check_catch_body(&catch.body.statements, exception_name, sink);
        }
        let _ = try_stmt;
    }
}

fn check_catch_body(
    statements: &[groovylint_syntax::ast::Stmt],
    exception_name: &str,
    sink: &mut DiagnosticSink<'_>,
) {
    for statement in statements {
        let groovylint_syntax::ast::Stmt::Expr(expr) = statement else {
            continue;
        };
        let ExprKind::Call {
            receiver: Some(_),
            name,
            args,
            ..
        } = &expr.kind
        else {
            continue;
        };
        if name != "error" {
            continue;
        }
        if args.len() != 1 {
            continue;
        }
        let Some(message) = crate::rules::util::string_literal(&args[0]) else {
            continue;
        };
        let mentions_exception = message
            .split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
            .any(|word| word == exception_name);
        if mentions_exception {
            continue;
        }
        sink.report(
            expr.span,
            format!(
                "The error logging may hide the stacktrace from the exception named {exception_name}"
            ),
        );
    }
}

/// Descriptors for the logging rules that inspect logger declarations.
#[must_use]
pub fn logger_descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "MultipleLoggers",
                "org.codenarc.rule.logging.MultipleLoggersRule",
                Category::Logging,
                Severity::Warning,
                "Checks for classes with more than one logger.",
            ),
            || RuleInstanceImpl::Ast(Box::new(MultipleLoggersRule)),
        ),
        descriptor(
            RuleMeta::new(
                "LoggerWithWrongModifiers",
                "org.codenarc.rule.logging.LoggerWithWrongModifiersRule",
                Category::Logging,
                Severity::Warning,
                "Checks logger field modifiers.",
            ),
            || RuleInstanceImpl::Ast(Box::new(LoggerWithWrongModifiersRule)),
        ),
        descriptor(
            RuleMeta::new(
                "LoggerForDifferentClass",
                "org.codenarc.rule.logging.LoggerForDifferentClassRule",
                Category::Logging,
                Severity::Warning,
                "Checks that loggers are created for their own class.",
            ),
            || RuleInstanceImpl::Ast(Box::new(LoggerForDifferentClassRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "LoggingSwallowsStacktrace",
                "org.codenarc.rule.logging.LoggingSwallowsStacktraceRule",
                Category::Logging,
                Severity::Warning,
                "Checks catch blocks that log without the exception.",
            ),
            || RuleInstanceImpl::Ast(Box::new(LoggingSwallowsStacktraceRule)),
        ),
    ]
}
