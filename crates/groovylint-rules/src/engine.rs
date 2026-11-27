//! Rule execution engine.

use std::collections::BTreeSet;

use groovylint_arch::{FactCollector, FactPlan, FileFacts};
use groovylint_core::{Diagnostic, Severity, SourceFile};
use groovylint_syntax::ast::Stmt;
use groovylint_syntax::parser::ParseOutput;

use crate::registry::{RuleDescriptor, RuleInstanceImpl, RuleRegistry};
use crate::rule::{AstRule, DiagnosticSink, FileContext, FileRule, RuleProperties, TokenRule};
use crate::traverse::{self, AstVisitor};

/// A rule selected for execution with its effective severity.
#[derive(Clone)]
struct ConfiguredRule {
    descriptor: RuleDescriptor,
    severity: Severity,
    properties: RuleProperties,
}

/// The rule engine.
///
/// The engine is immutable and safe to share across worker threads. A fresh
/// instance of each rule is created for every file, so rules can keep local
/// traversal state.
pub struct Engine {
    rules: Vec<ConfiguredRule>,
    unsupported: Vec<String>,
}

/// Diagnostics plus optional file facts from one analysis pass.
pub struct FileAnalysis {
    pub diagnostics: Vec<Diagnostic>,
    pub facts: Option<FileFacts>,
}

impl Engine {
    /// Builds an engine from `CodeNarc` class names. Unknown rules are recorded
    /// in [`Engine::unsupported`] instead of failing the run.
    #[must_use]
    pub fn from_codenarc_classes(registry: &RuleRegistry, class_names: &[String]) -> Self {
        let entries: Vec<(String, RuleProperties)> = class_names
            .iter()
            .map(|name| (name.clone(), RuleProperties::new()))
            .collect();
        Self::from_codenarc_entries(registry, &entries)
    }

    /// Builds an engine from `CodeNarc` class names with configured properties.
    #[must_use]
    pub fn from_codenarc_entries(
        registry: &RuleRegistry,
        entries: &[(String, RuleProperties)],
    ) -> Self {
        let mut rules = Vec::new();
        let mut unsupported = Vec::new();
        let mut seen = BTreeSet::new();

        for (class_name, properties) in entries {
            if !seen.insert(class_name.clone()) {
                continue;
            }
            if let Some(descriptor) = registry.find_codenarc_class(class_name) {
                rules.push(ConfiguredRule {
                    descriptor: *descriptor,
                    severity: descriptor.meta.default_severity,
                    properties: properties.clone(),
                });
            } else {
                unsupported.push(class_name.clone());
            }
        }

        Self { rules, unsupported }
    }

    /// Builds an engine containing every built-in rule.
    #[must_use]
    pub fn recommended(registry: &RuleRegistry) -> Self {
        let rules = registry
            .descriptors()
            .iter()
            .map(|descriptor| ConfiguredRule {
                descriptor: *descriptor,
                severity: descriptor.meta.default_severity,
                properties: RuleProperties::new(),
            })
            .collect();
        Self {
            rules,
            unsupported: Vec::new(),
        }
    }

    /// Returns `CodeNarc` rules that are not implemented yet.
    #[must_use]
    pub fn unsupported(&self) -> &[String] {
        &self.unsupported
    }

    /// Returns the number of enabled rules.
    #[must_use]
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// An engine with no `CodeNarc` rules; useful for fact extraction alone.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            rules: Vec::new(),
            unsupported: Vec::new(),
        }
    }

    /// Runs all enabled rules against one parsed file.
    #[must_use]
    pub fn lint(&self, source: &SourceFile, parsed: &ParseOutput) -> Vec<Diagnostic> {
        self.analyze(source, parsed, "", None).diagnostics
    }

    /// Runs rules and collects file facts in the same AST traversal.
    #[must_use]
    pub fn lint_with_facts(
        &self,
        source: &SourceFile,
        parsed: &ParseOutput,
        module: &str,
        plan: FactPlan,
    ) -> FileAnalysis {
        self.analyze(source, parsed, module, Some(plan))
    }

    fn analyze(
        &self,
        source: &SourceFile,
        parsed: &ParseOutput,
        module: &str,
        facts_plan: Option<FactPlan>,
    ) -> FileAnalysis {
        let mut diagnostics = Vec::new();
        let mut ast_rules = Vec::new();
        let mut token_rules = Vec::new();
        let mut file_rules = Vec::new();
        for configured in &self.rules {
            let mut instance = (configured.descriptor.factory)();
            if instance.configure(&configured.properties).is_err() {
                continue;
            }

            let meta = configured.descriptor.meta;
            match instance {
                RuleInstanceImpl::Ast(rule) => {
                    let mask = rule.interests();
                    ast_rules.push(ActiveAstRule {
                        rule,
                        mask,
                        severity: configured.severity,
                        name: meta.name,
                    });
                }
                RuleInstanceImpl::Token(rule) => token_rules.push(ActiveTokenRule {
                    rule,
                    severity: configured.severity,
                    name: meta.name,
                }),
                RuleInstanceImpl::File(rule) => file_rules.push(ActiveFileRule {
                    rule,
                    severity: configured.severity,
                    name: meta.name,
                }),
            }
        }

        let mut facts = facts_plan.map(|plan| FactCollector::new(source.path(), module, plan));
        let context = FileContext {
            source,
            script: &parsed.script,
            tokens: &parsed.tokens,
        };

        for active in &mut token_rules {
            let mut sink = DiagnosticSink::new(active.name, active.severity, &mut diagnostics);
            active.rule.check(&context, &mut sink);
        }

        {
            let mut dispatcher = Dispatcher {
                rules: &mut ast_rules,
                facts: facts.as_mut(),
                context: &context,
                diagnostics: &mut diagnostics,
            };
            traverse::walk_script(&parsed.script, &mut dispatcher);
        }

        for active in &mut file_rules {
            let mut sink = DiagnosticSink::new(active.name, active.severity, &mut diagnostics);
            active.rule.check(&context, &mut sink);
        }

        apply_suppressions(&parsed.script, &mut diagnostics);

        diagnostics.sort_by(|left, right| {
            left.span
                .start
                .cmp(&right.span.start)
                .then_with(|| left.span.end.cmp(&right.span.end))
                .then_with(|| left.rule.cmp(&right.rule))
                .then_with(|| left.message.cmp(&right.message))
        });
        FileAnalysis {
            diagnostics,
            facts: facts.map(FactCollector::finish),
        }
    }
}

/// A range of the file that suppresses a set of rules.
struct Suppression {
    span: groovylint_core::Span,
    rules: Vec<String>,
}

/// Drops diagnostics inside a declaration whose `@SuppressWarnings` lists them.
fn apply_suppressions(script: &groovylint_syntax::ast::Script, diagnostics: &mut Vec<Diagnostic>) {
    let mut suppressions = Vec::new();
    for item in &script.items {
        collect_suppressions(item, &mut suppressions);
    }
    if suppressions.is_empty() {
        return;
    }

    diagnostics.retain(|diagnostic| {
        !suppressions.iter().any(|suppression| {
            suppression.span.start <= diagnostic.span.start
                && diagnostic.span.end <= suppression.span.end
                && suppression
                    .rules
                    .iter()
                    .any(|rule| rule == &diagnostic.rule || rule.eq_ignore_ascii_case("all"))
        })
    });
}

fn collect_suppressions(item: &groovylint_syntax::ast::Item, suppressions: &mut Vec<Suppression>) {
    use groovylint_syntax::ast::Item;
    match item {
        Item::Class(class) => {
            if !class.suppressed_rules.is_empty() {
                suppressions.push(Suppression {
                    span: class.span,
                    rules: class.suppressed_rules.clone(),
                });
            }
            for member in &class.members {
                collect_suppressions(member, suppressions);
            }
        }
        Item::Method(method) => {
            if !method.suppressed_rules.is_empty() {
                suppressions.push(Suppression {
                    span: method.span,
                    rules: method.suppressed_rules.clone(),
                });
            }
        }
        Item::Field(field) if !field.suppressed_rules.is_empty() => {
            suppressions.push(Suppression {
                span: field.span,
                rules: field.suppressed_rules.clone(),
            });
        }
        _ => {}
    }
}

struct ActiveAstRule {
    rule: Box<dyn AstRule>,
    mask: crate::rule::NodeMask,
    severity: Severity,
    name: &'static str,
}

struct ActiveTokenRule {
    rule: Box<dyn TokenRule>,
    severity: Severity,
    name: &'static str,
}

struct ActiveFileRule {
    rule: Box<dyn FileRule>,
    severity: Severity,
    name: &'static str,
}

struct Dispatcher<'a, 'b> {
    rules: &'b mut [ActiveAstRule],
    facts: Option<&'b mut FactCollector>,
    context: &'b FileContext<'a>,
    diagnostics: &'b mut Vec<Diagnostic>,
}

impl AstVisitor for Dispatcher<'_, '_> {
    fn enter(&mut self, node: crate::rule::NodeRef<'_>) {
        if let Some(facts) = &mut self.facts {
            match node {
                crate::rule::NodeRef::Script(script) => facts.enter_script(script),
                crate::rule::NodeRef::Class(class) => facts.enter_class(class),
                crate::rule::NodeRef::Method(method) => facts.enter_method(method),
                crate::rule::NodeRef::Field(field) => facts.enter_field(field),
                crate::rule::NodeRef::Stmt(stmt) => {
                    if matches!(stmt, Stmt::VarDecl(_) | Stmt::ForEach(_)) {
                        facts.enter_stmt(stmt);
                    }
                }
                crate::rule::NodeRef::Expr(expr) => {
                    if matches!(
                        expr.kind,
                        groovylint_syntax::ast::ExprKind::New { .. }
                            | groovylint_syntax::ast::ExprKind::Cast { .. }
                    ) {
                        facts.enter_expr(expr);
                    }
                }
                _ => {}
            }
        }
        let kind = node.kind();
        for active in self.rules.iter_mut() {
            if active.mask.contains(kind) {
                let mut sink = DiagnosticSink::new(active.name, active.severity, self.diagnostics);
                active.rule.enter(node, self.context, &mut sink);
            }
        }
    }

    fn exit(&mut self, node: crate::rule::NodeRef<'_>) {
        if let Some(facts) = &mut self.facts
            && matches!(node, crate::rule::NodeRef::Class(_))
        {
            facts.exit_class();
        }
        let kind = node.kind();
        for active in self.rules.iter_mut() {
            if active.mask.contains(kind) {
                let mut sink = DiagnosticSink::new(active.name, active.severity, self.diagnostics);
                active.rule.exit(node, self.context, &mut sink);
            }
        }
    }
}
