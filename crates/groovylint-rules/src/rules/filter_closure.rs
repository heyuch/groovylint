//! Rule for assignments to parameters in filtering closures.

use groovylint_core::{Severity, Span};
use groovylint_syntax::ast::{Expr, ExprKind, Param};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, NodeKind, NodeMask, NodeRef, Rule, RuleMeta,
};
use crate::rules::util::unparen;

const FILTER_METHODS: &[&str] = &[
    "find",
    "findAll",
    "findIndexOf",
    "every",
    "any",
    "filter",
    "grep",
    "dropWhile",
    "takeWhile",
];

/// Detects assignments to closure parameters inside filtering methods such as
/// `findAll` and `every`.
#[derive(Default)]
pub struct ParameterAssignmentInFilterClosureRule {
    pending: Vec<(Span, Vec<String>)>,
    closure_stack: Vec<Option<Vec<String>>>,
}

impl Rule for ParameterAssignmentInFilterClosureRule {}

impl AstRule for ParameterAssignmentInFilterClosureRule {
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
            ExprKind::Call { name, args, .. } if FILTER_METHODS.contains(&name.as_str()) => {
                if let Some(Expr {
                    kind: ExprKind::Closure { params, .. },
                    span,
                }) = args.last()
                {
                    let names = closure_param_names(params);
                    self.pending.push((*span, names));
                }
            }
            ExprKind::Closure { .. } => {
                let position = self.pending.iter().position(|(span, _)| *span == expr.span);
                let names = position.map(|index| self.pending.remove(index).1);
                self.closure_stack.push(names);
            }
            ExprKind::Assign { target, .. } => {
                let Some(name) = root_variable(target) else {
                    return;
                };
                let assigned = self
                    .closure_stack
                    .iter()
                    .rev()
                    .flatten()
                    .any(|parameters| parameters.iter().any(|parameter| parameter == name));
                if assigned {
                    sink.report(
                        expr.span,
                        "An assignment operator was used on a parameter in a filtering closure. This is usually a typo, and the comparison operator (==) was intended.",
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
        if let NodeRef::Expr(Expr {
            kind: ExprKind::Closure { .. },
            ..
        }) = node
        {
            self.closure_stack.pop();
        }
    }
}

fn closure_param_names(params: &[Param]) -> Vec<String> {
    if params.is_empty() {
        vec!["it".to_string()]
    } else {
        params.iter().map(|param| param.name.clone()).collect()
    }
}

fn root_variable(expr: &Expr) -> Option<&str> {
    match &unparen(expr).kind {
        ExprKind::Variable(name) => Some(name),
        ExprKind::Property { receiver, .. } => root_variable(receiver),
        _ => None,
    }
}

/// Descriptors for the filter-closure rule.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![descriptor(
        RuleMeta::new(
            "ParameterAssignmentInFilterClosure",
            "org.codenarc.rule.basic.ParameterAssignmentInFilterClosureRule",
            Category::Basic,
            Severity::Warning,
            "Checks for assignments to parameters in filter closures.",
        ),
        || RuleInstanceImpl::Ast(Box::new(ParameterAssignmentInFilterClosureRule::default())),
    )]
}
