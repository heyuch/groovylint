//! `CodeNarc` size and complexity metrics.

use groovylint_core::{Severity, Span};
use groovylint_syntax::ast::{BinaryOp, ClassDecl, Expr, ExprKind, Item, MethodDecl, Stmt};

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

/// One thing a method metric measures: a method, a closure field or a script body.
struct Unit {
    name: String,
    span: Span,
    body: groovylint_syntax::ast::Block,
}

/// The measurable units of a class, including closures stored in fields.
fn class_units(class: &ClassDecl) -> Vec<Unit> {
    let mut units = Vec::new();
    for member in &class.members {
        match member {
            Item::Method(method) => {
                if let Some(body) = &method.body {
                    units.push(Unit {
                        name: method.name.clone(),
                        span: method.span,
                        body: body.clone(),
                    });
                }
            }
            Item::Field(field) => {
                if let Some(init) = &field.init
                    && let ExprKind::Closure { body, .. } = &init.kind
                {
                    units.push(Unit {
                        name: field.name.clone(),
                        span: field.span,
                        body: body.clone(),
                    });
                }
            }
            _ => {}
        }
    }
    units
}

/// The measurable units of a script: its methods, closure-valued variables and the
/// script body itself, which Groovy compiles into a `run()` method.
fn script_units(script: &groovylint_syntax::ast::Script) -> Vec<Unit> {
    let mut units = Vec::new();
    let statements: Vec<Stmt> = script
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Statement(statement) => Some(statement.clone()),
            _ => None,
        })
        .collect();
    if let (Some(first), Some(last)) = (statements.first(), statements.last()) {
        let span = Span::new(first.span().start, last.span().end);
        units.push(Unit {
            name: "run".to_string(),
            span,
            body: groovylint_syntax::ast::Block { span, statements },
        });
    }
    for item in &script.items {
        match item {
            Item::Method(method) => {
                if let Some(body) = &method.body {
                    units.push(Unit {
                        name: method.name.clone(),
                        span: method.span,
                        body: body.clone(),
                    });
                }
            }
            Item::Statement(Stmt::VarDecl(decl)) => {
                if let Some(init) = &decl.init
                    && let ExprKind::Closure { body, .. } = &init.kind
                {
                    units.push(Unit {
                        name: decl.name.clone(),
                        span: decl.span,
                        body: body.clone(),
                    });
                }
            }
            _ => {}
        }
    }
    units
}

/// The three ABC vectors of one method.
#[derive(Default)]
struct Counts {
    assignments: usize,
    branches: usize,
    conditions: usize,
}

impl Counts {
    /// `|ABC| = sqrt(A*A + B*B + C*C)`, rounded to one decimal like `GMetrics`.
    fn magnitude(&self) -> f64 {
        let sum = self.assignments * self.assignments
            + self.branches * self.branches
            + self.conditions * self.conditions;
        // The counts are small integers; the conversion cannot lose anything here.
        let magnitude = f64::from(u32::try_from(sum).unwrap_or(u32::MAX)).sqrt();
        (magnitude * 10.0).floor() / 10.0
    }
}

/// Counts assignments, branches and conditions the way `GMetrics` does.
fn abc_counts(body: &groovylint_syntax::ast::Block) -> Counts {
    let mut counter = MetricCounter::default();
    crate::traverse::walk_block(body, &mut counter);
    counter.counts
}

/// Counts decision points the way `GMetrics`' cyclomatic metric does.
fn cyclomatic(body: &groovylint_syntax::ast::Block) -> usize {
    let mut counter = MetricCounter::default();
    crate::traverse::walk_block(body, &mut counter);
    1 + counter.decisions
}

#[derive(Default)]
struct MetricCounter {
    counts: Counts,
    decisions: usize,
}

impl crate::traverse::AstVisitor for MetricCounter {
    fn enter(&mut self, node: NodeRef<'_>) {
        match node {
            NodeRef::Stmt(Stmt::If(statement)) => {
                self.decisions += 1;
                if statement.else_branch.is_some() {
                    self.counts.conditions += 1;
                }
            }
            NodeRef::Stmt(Stmt::While(_) | Stmt::DoWhile(_) | Stmt::For(_) | Stmt::ForEach(_)) => {
                self.decisions += 1;
            }
            NodeRef::Stmt(Stmt::Switch(statement)) => {
                let labels = statement
                    .cases
                    .iter()
                    .filter(|case| case.label.is_some())
                    .count();
                self.decisions += labels;
                self.counts.conditions += labels;
                if statement.cases.iter().any(|case| case.label.is_none()) {
                    self.counts.conditions += 1;
                }
            }
            NodeRef::Stmt(Stmt::Try(statement)) => {
                self.decisions += statement.catches.len();
                self.counts.conditions += 1 + statement.catches.len();
            }
            NodeRef::Expr(expr) => self.count_expression(expr),
            _ => {}
        }
    }

    fn exit(&mut self, _node: NodeRef<'_>) {}
}

impl MetricCounter {
    fn count_expression(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Call { safe, .. } | ExprKind::Property { safe, .. } => {
                self.counts.branches += 1;
                if *safe {
                    self.decisions += 1;
                }
            }
            ExprKind::New { .. } => self.counts.branches += 1,
            ExprKind::Assign { .. } => self.counts.assignments += 1,
            ExprKind::Ternary { .. } => {
                self.decisions += 1;
                self.counts.conditions += 1;
            }
            ExprKind::Unary { op, .. } => {
                if matches!(
                    op,
                    groovylint_syntax::ast::UnaryOp::PreInc
                        | groovylint_syntax::ast::UnaryOp::PreDec
                        | groovylint_syntax::ast::UnaryOp::PostInc
                        | groovylint_syntax::ast::UnaryOp::PostDec
                ) {
                    self.counts.assignments += 1;
                }
            }
            ExprKind::Binary { op, left, right } => {
                if matches!(op, BinaryOp::And | BinaryOp::Or) {
                    self.decisions += 1;
                    if is_single_variable(left) {
                        self.counts.conditions += 1;
                    }
                    if is_single_variable(right) {
                        self.counts.conditions += 1;
                    }
                } else if is_comparison(*op) {
                    self.counts.conditions += 1;
                }
            }
            _ => {}
        }
    }
}

fn is_single_variable(expr: &Expr) -> bool {
    matches!(expr.kind, ExprKind::Variable(_))
}

fn is_comparison(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Eq
            | BinaryOp::NotEq
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::RegexFind
            | BinaryOp::RegexMatch
            | BinaryOp::InstanceOf
    )
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn is_ignored(patterns: &[String], name: &str) -> bool {
    patterns
        .iter()
        .any(|pattern| crate::rules::types::wildcard_matches(name, pattern))
}

/// Reports on the declaration line itself, which is where `GMetrics` points.
fn report_plain(
    context: &FileContext<'_>,
    sink: &mut DiagnosticSink<'_>,
    span: Span,
    message: String,
) {
    let line = context.source.line_col(span.start).line;
    let Some(span) = context.source.line_span(line) else {
        return;
    };
    sink.report(span, message);
}

/// A span that reports nothing useful, for group-level metrics on a script.
const fn default_span() -> Span {
    Span::new(0, 0)
}

/// The start of a declaration, including the annotations written above it.
fn declaration_start(context: &FileContext<'_>, span: Span) -> usize {
    let mut start = span.start;
    let mut line = context.source.line_col(span.start).line;
    while line > 1 {
        let Some(previous) = context.source.line_text(line - 1) else {
            break;
        };
        if !previous.trim_start().starts_with('@') {
            break;
        }
        line -= 1;
        if let Some(span) = context.source.line_span(line) {
            start = span.start;
        }
    }
    start
}

/// Reports on the declaration's first line, annotations included.
fn report_at_declaration(
    context: &FileContext<'_>,
    sink: &mut DiagnosticSink<'_>,
    span: Span,
    message: String,
) {
    let start = declaration_start(context, span);
    let line = context.source.line_col(start).line;
    let Some(span) = context.source.line_span(line) else {
        return;
    };
    sink.report(span, message);
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
    ignored: Vec<String>,
}

impl Default for MethodSizeRule {
    fn default() -> Self {
        Self {
            max_lines: 100,
            ignored: Vec::new(),
        }
    }
}

impl Rule for MethodSizeRule {
    fn configure(&mut self, properties: &RuleProperties) -> Result<(), String> {
        if let Some(value) = properties.get("maxLines") {
            self.max_lines = value
                .parse()
                .map_err(|_| "maxLines must be a number".to_string())?;
        }
        if let Some(value) = properties.get("ignoreMethodNames") {
            self.ignored = split_list(value);
        }
        Ok(())
    }
}

impl FileRule for MethodSizeRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        for class in classes(context.script) {
            for method in methods(class) {
                if is_ignored(&self.ignored, &method.name) {
                    continue;
                }
                // Annotations belong to the method: CodeNarc counts and reports
                // from the first annotation line.
                let start = declaration_start(context, method.span);
                let span = Span::new(start, method.span.end);
                let lines = line_span(context, span);
                if lines <= self.max_lines {
                    continue;
                }
                report_at_declaration(
                    context,
                    sink,
                    span,
                    format!(
                        "The method {} has {lines} lines (maximum {})",
                        method.name, self.max_lines
                    ),
                );
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
    max_class_complexity: Option<usize>,
    max_class_average: Option<f64>,
    ignored: Vec<String>,
}

impl Default for CyclomaticComplexityRule {
    fn default() -> Self {
        Self {
            max_method_complexity: 20,
            max_class_complexity: None,
            max_class_average: None,
            ignored: Vec::new(),
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
        if let Some(value) = properties.get("maxClassComplexity") {
            self.max_class_complexity = Some(
                value
                    .parse()
                    .map_err(|_| "maxClassComplexity must be a number".to_string())?,
            );
        }
        if let Some(value) = properties.get("maxClassAverageMethodComplexity") {
            self.max_class_average = Some(
                value
                    .parse()
                    .map_err(|_| "maxClassAverageMethodComplexity must be a number".to_string())?,
            );
        }
        if let Some(value) = properties.get("ignoreMethodNames") {
            self.ignored = split_list(value);
        }
        Ok(())
    }
}

impl FileRule for CyclomaticComplexityRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let mut groups: Vec<(&str, Span, bool, Vec<Unit>)> = Vec::new();
        for class in classes(context.script) {
            groups.push((class.name.as_str(), class.span, false, class_units(class)));
        }
        let units = script_units(context.script);
        if !units.is_empty() {
            // A script has no class of its own to measure: CodeNarc skips it.
            groups.push(("script", default_span(), true, units));
        }

        for (name, span, is_script, units) in groups {
            let mut total = 0;
            let mut measured = 0;
            for unit in &units {
                if is_ignored(&self.ignored, &unit.name) {
                    continue;
                }
                let complexity = cyclomatic(&unit.body);
                total += complexity;
                measured += 1;
                if complexity <= self.max_method_complexity {
                    continue;
                }
                report_plain(
                    context,
                    sink,
                    unit.span,
                    format!(
                        "The method {} has cyclomatic complexity {complexity} (maximum {})",
                        unit.name, self.max_method_complexity
                    ),
                );
            }
            if measured == 0 || is_script {
                continue;
            }
            let average = f64::from(u32::try_from(total).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(measured).unwrap_or(u32::MAX));
            // CodeNarc reads the class thresholds as Groovy truth, so 0 disables them.
            let over_class = self
                .max_class_complexity
                .is_some_and(|limit| limit > 0 && total > limit);
            let over_average = self
                .max_class_average
                .is_some_and(|limit| limit > 0.0 && average > limit);
            if !over_class && !over_average {
                continue;
            }
            report_at_declaration(
                context,
                sink,
                span,
                format!(
                    "The class {name} has cyclomatic complexity {total} (average {average:.1})"
                ),
            );
        }
    }
}

/// Flags methods with a high assignment/branch/condition score.
pub struct AbcMetricRule {
    max_method_score: f64,
    max_class_score: Option<f64>,
    max_class_average: Option<f64>,
    ignored: Vec<String>,
}

impl Default for AbcMetricRule {
    fn default() -> Self {
        Self {
            max_method_score: 60.0,
            max_class_score: None,
            max_class_average: None,
            ignored: Vec::new(),
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
        if let Some(value) = properties.get("maxClassAbcScore") {
            self.max_class_score = Some(
                value
                    .parse()
                    .map_err(|_| "maxClassAbcScore must be a number".to_string())?,
            );
        }
        if let Some(value) = properties.get("maxClassAverageMethodAbcScore") {
            self.max_class_average = Some(
                value
                    .parse()
                    .map_err(|_| "maxClassAverageMethodAbcScore must be a number".to_string())?,
            );
        }
        if let Some(value) = properties.get("ignoreMethodNames") {
            self.ignored = split_list(value);
        }
        Ok(())
    }
}

impl FileRule for AbcMetricRule {
    fn check(&mut self, context: &FileContext<'_>, sink: &mut DiagnosticSink<'_>) {
        let mut groups: Vec<(&str, Span, bool, Vec<Unit>)> = Vec::new();
        for class in classes(context.script) {
            groups.push((class.name.as_str(), class.span, false, class_units(class)));
        }
        let units = script_units(context.script);
        if !units.is_empty() {
            // A script has no class of its own to measure: CodeNarc skips it.
            groups.push(("script", default_span(), true, units));
        }

        for (name, span, is_script, units) in groups {
            let mut totals = Counts::default();
            let mut scores: Vec<f64> = Vec::new();
            for unit in &units {
                if is_ignored(&self.ignored, &unit.name) {
                    continue;
                }
                let counts = abc_counts(&unit.body);
                let score = counts.magnitude();
                totals.assignments += counts.assignments;
                totals.branches += counts.branches;
                totals.conditions += counts.conditions;
                scores.push(score);
                if score <= self.max_method_score {
                    continue;
                }
                report_plain(
                    context,
                    sink,
                    unit.span,
                    format!(
                        "The method {} has ABC score {score} (maximum {})",
                        unit.name, self.max_method_score
                    ),
                );
            }
            if scores.is_empty() || is_script {
                continue;
            }
            let class_score = totals.magnitude();
            let count = f64::from(u32::try_from(scores.len()).unwrap_or(u32::MAX));
            let average = scores.iter().sum::<f64>() / count;
            let over_class = self
                .max_class_score
                .is_some_and(|limit| limit > 0.0 && class_score > limit);
            let over_average = self
                .max_class_average
                .is_some_and(|limit| limit > 0.0 && average > limit);
            if !over_class && !over_average {
                continue;
            }
            report_at_declaration(
                context,
                sink,
                span,
                format!("The class {name} has ABC score {class_score} (average {average:.1})"),
            );
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
