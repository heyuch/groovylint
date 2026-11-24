//! `CodeNarc` size and complexity metrics.

use groovylint_core::{Severity, Span};
use groovylint_syntax::ast::{ClassDecl, Item, MethodDecl, Stmt};

use crate::registry::{RuleDescriptor, RuleInstanceImpl, descriptor};
use crate::rule::{
    AstRule, Category, DiagnosticSink, FileContext, FileRule, NodeKind, NodeMask, NodeRef, Rule,
    RuleMeta, RuleProperties,
};

fn line_span(context: &FileContext<'_>, span: Span) -> usize {
    let start = context.source.line_col(span.start).line;
    let end = context
        .source
        .line_col(span.end.saturating_sub(1).max(span.start))
        .line;
    end.saturating_sub(start) + 1
}

fn classes(script: &groovylint_syntax::ast::Script) -> Vec<&ClassDecl> {
    script
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Class(class) => Some(class),
            _ => None,
        })
        .collect()
}

fn methods(class: &ClassDecl) -> Vec<&MethodDecl> {
    class
        .members
        .iter()
        .filter_map(|member| match member {
            Item::Method(method) => Some(method),
            _ => None,
        })
        .collect()
}

fn count_branches(method: &MethodDecl) -> usize {
    let Some(body) = &method.body else {
        return 0;
    };
    let mut counter = BranchCounter { count: 1 };
    crate::traverse::walk_block(body, &mut counter);
    counter.count
}

struct BranchCounter {
    count: usize,
}

impl crate::traverse::AstVisitor for BranchCounter {
    fn enter(&mut self, node: NodeRef<'_>) {
        match node {
            NodeRef::Stmt(
                Stmt::If(_)
                | Stmt::While(_)
                | Stmt::DoWhile(_)
                | Stmt::For(_)
                | Stmt::ForEach(_)
                | Stmt::Try(_)
                | Stmt::Switch(_),
            ) => self.count += 1,
            NodeRef::Expr(expr) => {
                if matches!(
                    expr.kind,
                    groovylint_syntax::ast::ExprKind::Binary {
                        op: groovylint_syntax::ast::BinaryOp::And
                            | groovylint_syntax::ast::BinaryOp::Or,
                        ..
                    }
                ) {
                    self.count += 1;
                }
            }
            _ => {}
        }
    }
    fn exit(&mut self, _node: NodeRef<'_>) {}
}

fn count_abc(method: &MethodDecl) -> usize {
    let Some(body) = &method.body else {
        return 0;
    };
    let mut counter = AbcCounter { count: 0 };
    crate::traverse::walk_block(body, &mut counter);
    counter.count
}

struct AbcCounter {
    count: usize,
}

impl crate::traverse::AstVisitor for AbcCounter {
    fn enter(&mut self, node: NodeRef<'_>) {
        match node {
            NodeRef::Stmt(Stmt::VarDecl(_)) => self.count += 1,
            NodeRef::Expr(expr) => match &expr.kind {
                groovylint_syntax::ast::ExprKind::Assign { .. } => self.count += 1,
                groovylint_syntax::ast::ExprKind::Binary { op, .. } => {
                    if matches!(
                        op,
                        groovylint_syntax::ast::BinaryOp::And
                            | groovylint_syntax::ast::BinaryOp::Or
                            | groovylint_syntax::ast::BinaryOp::Eq
                            | groovylint_syntax::ast::BinaryOp::NotEq
                            | groovylint_syntax::ast::BinaryOp::Lt
                            | groovylint_syntax::ast::BinaryOp::Le
                            | groovylint_syntax::ast::BinaryOp::Gt
                            | groovylint_syntax::ast::BinaryOp::Ge
                    ) {
                        self.count += 1;
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    fn exit(&mut self, _node: NodeRef<'_>) {}
}

/// Flags classes that exceed a line-count limit.
pub struct ClassSizeRule {
    max_lines: usize,
}

impl Default for ClassSizeRule {
    fn default() -> Self {
        Self { max_lines: 1000 }
    }
}

impl Rule for ClassSizeRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        if let Some(value) = properties.get("maxLines") {
            self.max_lines = value
                .parse()
                .map_err(|_| "maxLines must be a number".to_string())?;
        }
        Ok(())
    }
}

impl FileRule for ClassSizeRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for class in classes(context.script) {
            let lines = line_span(context, class.span);
            if lines > self.max_lines {
                sink.report(
                    class.span,
                    format!(
                        "The class {} has {lines} lines (maximum {})",
                        class.name, self.max_lines
                    ),
                );
            }
        }
    }
}

/// Flags classes with too many methods.
pub struct MethodCountRule {
    max_methods: usize,
}

impl Default for MethodCountRule {
    fn default() -> Self {
        Self { max_methods: 30 }
    }
}

impl Rule for MethodCountRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        if let Some(value) = properties.get("maxMethods") {
            self.max_methods = value
                .parse()
                .map_err(|_| "maxMethods must be a number".to_string())?;
        }
        Ok(())
    }
}

impl FileRule for MethodCountRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for class in classes(context.script) {
            let count = methods(class).len();
            if count > self.max_methods {
                sink.report(
                    class.span,
                    format!(
                        "The class {} has {count} methods (maximum {})",
                        class.name, self.max_methods
                    ),
                );
            }
        }
    }
}

/// Flags methods that exceed a line-count limit.
pub struct MethodSizeRule {
    max_lines: usize,
}

impl Default for MethodSizeRule {
    fn default() -> Self {
        Self { max_lines: 100 }
    }
}

impl Rule for MethodSizeRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        if let Some(value) = properties.get("maxLines") {
            self.max_lines = value
                .parse()
                .map_err(|_| "maxLines must be a number".to_string())?;
        }
        Ok(())
    }
}

impl FileRule for MethodSizeRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for class in classes(context.script) {
            for method in methods(class) {
                let lines = line_span(context, method.span);
                if lines > self.max_lines {
                    sink.report(
                        method.span,
                        format!(
                            "The method {} has {lines} lines (maximum {})",
                            method.name, self.max_lines
                        ),
                    );
                }
            }
        }
    }
}

/// Flags methods with too many parameters.
pub struct ParameterCountRule {
    max_parameters: usize,
}

impl Default for ParameterCountRule {
    fn default() -> Self {
        Self { max_parameters: 5 }
    }
}

impl Rule for ParameterCountRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        if let Some(value) = properties.get("maxParameters") {
            self.max_parameters = value
                .parse()
                .map_err(|_| "maxParameters must be a number".to_string())?;
        }
        Ok(())
    }
}

impl FileRule for ParameterCountRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for class in classes(context.script) {
            for method in methods(class) {
                if method.params.len() > self.max_parameters {
                    sink.report(
                        method.span,
                        format!(
                            "The method {} has {} parameters (maximum {})",
                            method.name,
                            method.params.len(),
                            self.max_parameters
                        ),
                    );
                }
            }
        }
    }
}

/// Flags methods with high cyclomatic complexity.
pub struct CyclomaticComplexityRule {
    max_method_complexity: usize,
}

impl Default for CyclomaticComplexityRule {
    fn default() -> Self {
        Self {
            max_method_complexity: 20,
        }
    }
}

impl Rule for CyclomaticComplexityRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        if let Some(value) = properties.get("maxMethodComplexity") {
            self.max_method_complexity = value
                .parse()
                .map_err(|_| "maxMethodComplexity must be a number".to_string())?;
        }
        Ok(())
    }
}

impl FileRule for CyclomaticComplexityRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for class in classes(context.script) {
            for method in methods(class) {
                let complexity = count_branches(method);
                if complexity > self.max_method_complexity {
                    sink.report(
                        method.span,
                        format!(
                            "The method {} has cyclomatic complexity {complexity} (maximum {})",
                            method.name, self.max_method_complexity
                        ),
                    );
                }
            }
        }
    }
}

/// Flags methods with a high assignment/branch/condition score.
pub struct AbcMetricRule {
    max_method_score: usize,
}

impl Default for AbcMetricRule {
    fn default() -> Self {
        Self {
            max_method_score: 60,
        }
    }
}

impl Rule for AbcMetricRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        if let Some(value) = properties.get("maxMethodAbcScore") {
            self.max_method_score = value
                .parse()
                .map_err(|_| "maxMethodAbcScore must be a number".to_string())?;
        }
        Ok(())
    }
}

impl FileRule for AbcMetricRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for class in classes(context.script) {
            for method in methods(class) {
                let score = count_abc(method);
                if score > self.max_method_score {
                    sink.report(
                        method.span,
                        format!(
                            "The method {} has ABC score {score} (maximum {})",
                            method.name, self.max_method_score
                        ),
                    );
                }
            }
        }
    }
}

/// Flags deeply nested blocks.
pub struct NestedBlockDepthRule {
    max_depth: usize,
    depth: usize,
}

impl Default for NestedBlockDepthRule {
    fn default() -> Self {
        Self {
            max_depth: 5,
            depth: 0,
        }
    }
}

impl Rule for NestedBlockDepthRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        if let Some(value) = properties.get("maxNestedBlockDepth") {
            self.max_depth = value
                .parse()
                .map_err(|_| "maxNestedBlockDepth must be a number".to_string())?;
        }
        Ok(())
    }
}

impl AstRule for NestedBlockDepthRule {
    fn interests(&self) -> NodeMask {
        NodeMask::of(NodeKind::If)
            .union(NodeMask::of(NodeKind::While))
            .union(NodeMask::of(NodeKind::For))
            .union(NodeMask::of(NodeKind::ForEach))
            .union(NodeMask::of(NodeKind::Try))
            .union(NodeMask::of(NodeKind::Switch))
    }

    fn enter(
        &mut self,
        node: NodeRef<'_>,
        _context: &FileContext<'_>,
        sink: &mut DiagnosticSink<'_>,
    ) {
        self.depth += 1;
        if self.depth > self.max_depth
            && let NodeRef::Stmt(stmt) = node
        {
            sink.report(
                stmt.span(),
                format!("The block is nested deeper than {} levels", self.max_depth),
            );
        }
    }

    fn exit(
        &mut self,
        _node: NodeRef<'_>,
        _context: &FileContext<'_>,
        _sink: &mut DiagnosticSink<'_>,
    ) {
        self.depth = self.depth.saturating_sub(1);
    }
}

/// Descriptors for size rules.
#[must_use]
pub fn descriptors() -> Vec<RuleDescriptor> {
    vec![
        descriptor(
            RuleMeta::new(
                "AbcMetric",
                "org.codenarc.rule.size.AbcMetricRule",
                Category::Size,
                Severity::Warning,
                "Assignment/Branch/Condition metric.",
            ),
            || RuleInstanceImpl::File(Box::new(AbcMetricRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "ClassSize",
                "org.codenarc.rule.size.ClassSizeRule",
                Category::Size,
                Severity::Info,
                "Class line count.",
            ),
            || RuleInstanceImpl::File(Box::new(ClassSizeRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "CyclomaticComplexity",
                "org.codenarc.rule.size.CyclomaticComplexityRule",
                Category::Size,
                Severity::Warning,
                "Cyclomatic complexity.",
            ),
            || RuleInstanceImpl::File(Box::new(CyclomaticComplexityRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "MethodCount",
                "org.codenarc.rule.size.MethodCountRule",
                Category::Size,
                Severity::Warning,
                "Method count per class.",
            ),
            || RuleInstanceImpl::File(Box::new(MethodCountRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "MethodSize",
                "org.codenarc.rule.size.MethodSizeRule",
                Category::Size,
                Severity::Info,
                "Method line count.",
            ),
            || RuleInstanceImpl::File(Box::new(MethodSizeRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "NestedBlockDepth",
                "org.codenarc.rule.size.NestedBlockDepthRule",
                Category::Size,
                Severity::Warning,
                "Nested block depth.",
            ),
            || RuleInstanceImpl::Ast(Box::new(NestedBlockDepthRule::default())),
        ),
        descriptor(
            RuleMeta::new(
                "ParameterCount",
                "org.codenarc.rule.size.ParameterCountRule",
                Category::Size,
                Severity::Warning,
                "Parameter count.",
            ),
            || RuleInstanceImpl::File(Box::new(ParameterCountRule::default())),
        ),
    ]
}
