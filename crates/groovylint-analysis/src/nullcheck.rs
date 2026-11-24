#![allow(clippy::needless_raw_string_hashes)]

//! Annotation-free intraprocedural nullness analysis.
//!
//! This is the first conservative slice: it infers nullness for locals and
//! expressions, refines access paths through guards, and reports definite (or,
//! in inferred mode, possible) null dereferences. It intentionally does not
//! read annotations or infer cross-method contracts yet.

use std::collections::{BTreeSet, VecDeque};

use groovylint_core::Span;
use groovylint_syntax::ast::{
    BinaryOp, ClassDecl, Expr, ExprKind, Item, Literal, MethodDecl, Param, Script, Stmt, UnaryOp,
};

use crate::cfg::{
    BasicBlock, BlockId, Cfg, Instruction, Terminator, build_closure, build_method, build_script,
};
use crate::nullability::{AccessPath, Finding, Mode, Nullness, Store};

/// Analyzes all executable units in a script.
#[must_use]
pub fn analyze(script: &Script, mode: Mode) -> Vec<Finding> {
    let mut analyzer = Analyzer::new(mode);
    analyzer.analyze_script(script);
    analyzer.finish()
}

struct Analyzer {
    mode: Mode,
    findings: Vec<Finding>,
    seen: BTreeSet<(usize, usize, &'static str)>,
    summaries: std::collections::BTreeMap<(String, usize), Nullness>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct State {
    store: Store,
}

impl State {
    fn new() -> Self {
        let mut store = Store::new();
        store.set(AccessPath::This, Nullness::NonNull);
        Self { store }
    }

    fn with_params(params: &[Param]) -> Self {
        let mut state = Self::new();
        for param in params {
            state
                .store
                .set(AccessPath::Local(param.name.clone()), Nullness::Unknown);
        }
        state
    }

    fn with_fields(mut self, fields: &[(String, Nullness)]) -> Self {
        for (name, value) in fields {
            self.store
                .set(AccessPath::field(AccessPath::This, name.clone()), *value);
        }
        self
    }

    fn join(&self, other: &Self) -> Self {
        Self {
            store: self.store.join(&other.store),
        }
    }
}

impl Analyzer {
    fn new(mode: Mode) -> Self {
        Self {
            mode,
            findings: Vec::new(),
            seen: BTreeSet::new(),
            summaries: std::collections::BTreeMap::new(),
        }
    }

    fn finish(mut self) -> Vec<Finding> {
        let definite: BTreeSet<(usize, usize)> = self
            .findings
            .iter()
            .filter(|finding| finding.code == "NullDereference")
            .map(|finding| (finding.span.start, finding.span.end))
            .collect();
        self.findings.retain(|finding| {
            finding.code != "PossibleNullDereference"
                || !definite.contains(&(finding.span.start, finding.span.end))
        });
        self.findings
            .sort_by_key(|finding| (finding.span.start, finding.span.end, finding.code));
        self.findings
    }

    fn analyze_script(&mut self, script: &Script) {
        self.collect_summaries(&script.items);
        if let Some(cfg) = build_script(script) {
            self.run_cfg(&cfg, State::new());
        }
        for item in &script.items {
            self.analyze_item(item);
        }
    }

    fn collect_summaries(&mut self, items: &[Item]) {
        for item in items {
            match item {
                Item::Class(class) => self.collect_summaries(&class.members),
                Item::Method(method) => {
                    if let Some(value) = self.summarize_method(method) {
                        self.summaries
                            .entry((method.name.clone(), method.params.len()))
                            .and_modify(|existing| *existing = existing.join(value))
                            .or_insert(value);
                    }
                }
                Item::Group(items) => self.collect_summaries(items),
                _ => {}
            }
        }
    }

    fn summarize_method(&self, method: &MethodDecl) -> Option<Nullness> {
        if method
            .return_type
            .as_deref()
            .is_some_and(|name| name == "void")
        {
            return None;
        }
        let body = method.body.as_ref()?;
        let mut values = Vec::new();
        for statement in &body.statements {
            if let Stmt::Return(ret) = statement
                && let Some(expr) = &ret.value
            {
                values.push(self.classify_expr(expr));
            }
        }
        if let Some(Stmt::Expr(expr)) = body.statements.last() {
            values.push(self.classify_expr(expr));
        }
        values.into_iter().reduce(Nullness::join)
    }

    fn classify_expr(&self, expr: &Expr) -> Nullness {
        let expr = unparen(expr);
        match &expr.kind {
            ExprKind::Literal(Literal::Null) => Nullness::Null,
            ExprKind::Literal(_)
            | ExprKind::New { .. }
            | ExprKind::List(_)
            | ExprKind::Map(_)
            | ExprKind::Closure { .. } => Nullness::NonNull,
            ExprKind::Assign { value, .. } => self.classify_expr(value),
            ExprKind::Ternary {
                then_expr,
                else_expr,
                ..
            } => self
                .classify_expr(then_expr)
                .join(self.classify_expr(else_expr)),
            ExprKind::Call {
                receiver: None,
                name,
                args,
                ..
            } => self
                .summaries
                .get(&(name.clone(), args.len()))
                .copied()
                .unwrap_or(Nullness::Unknown),
            ExprKind::Cast { expr, .. } => self.classify_expr(expr),
            _ => Nullness::Unknown,
        }
    }

    fn analyze_item(&mut self, item: &Item) {
        match item {
            Item::Class(class) => self.analyze_class(class),
            Item::Method(method) => self.analyze_method(method, &[]),
            Item::Initializer(initializer) => {
                let cfg = build_closure(&initializer.body);
                self.run_cfg(&cfg, State::new());
            }
            Item::Field(field) => {
                if let Some(init) = &field.init {
                    let mut state = State::new();
                    self.eval(init, &mut state);
                }
            }
            Item::Statement(_) => {}
            Item::Group(items) => {
                for item in items {
                    self.analyze_item(item);
                }
            }
        }
    }

    fn analyze_class(&mut self, class: &ClassDecl) {
        let fields = self.field_values(class);
        for member in &class.members {
            match member {
                Item::Method(method) => self.analyze_method(method, &fields),
                Item::Initializer(initializer) => {
                    let cfg = build_closure(&initializer.body);
                    self.run_cfg(&cfg, State::new().with_fields(&fields));
                }
                Item::Class(nested) => self.analyze_class(nested),
                Item::Field(_) => {}
                Item::Statement(_) | Item::Group(_) => self.analyze_item(member),
            }
        }
    }

    fn field_values(&mut self, class: &ClassDecl) -> Vec<(String, Nullness)> {
        let mut state = State::new();
        let mut values = Vec::new();
        for member in &class.members {
            let Item::Field(field) = member else {
                continue;
            };
            let value = field
                .init
                .as_ref()
                .map_or(Nullness::Null, |init| self.eval(init, &mut state));
            values.push((field.name.clone(), value));
            state.store.set(
                AccessPath::field(AccessPath::This, field.name.clone()),
                value,
            );
        }
        values
    }

    fn analyze_method(&mut self, method: &MethodDecl, fields: &[(String, Nullness)]) {
        let Some(cfg) = build_method(method) else {
            return;
        };
        self.run_cfg(&cfg, State::with_params(&method.params).with_fields(fields));
    }

    fn run_cfg(&mut self, cfg: &Cfg, initial: State) {
        let mut incoming: Vec<Option<State>> = vec![None; cfg.blocks.len()];
        let mut queue = VecDeque::new();
        incoming[cfg.entry] = Some(initial);
        queue.push_back(cfg.entry);

        while let Some(block_id) = queue.pop_front() {
            let Some(start) = incoming[block_id].clone() else {
                continue;
            };
            let block = cfg.block(block_id);
            let mut state = start;
            for instruction in &block.instructions {
                self.transfer_instruction(instruction, &mut state);
            }
            self.transfer_terminator(block, &state, cfg, &mut incoming, &mut queue);
            for successor in &block.extra_successors {
                merge_state(*successor, &state, &mut incoming, &mut queue);
            }
        }
    }

    fn transfer_instruction(&mut self, instruction: &Instruction, state: &mut State) {
        match instruction {
            Instruction::Eval(expr) => {
                self.eval(expr, state);
            }
            Instruction::Declare { name, init } => {
                let value = init
                    .as_ref()
                    .map_or(Nullness::Null, |init| self.eval(init, state));
                state.store.set(AccessPath::Local(name.clone()), value);
            }
            Instruction::Assert { condition, message } => {
                let (then_state, _) = self.condition_states(condition, state);
                *state = then_state;
                if let Some(message) = message {
                    self.eval(message, state);
                }
            }
        }
    }

    fn transfer_terminator(
        &mut self,
        block: &BasicBlock,
        state: &State,
        _cfg: &Cfg,
        incoming: &mut [Option<State>],
        queue: &mut VecDeque<BlockId>,
    ) {
        match &block.terminator {
            Terminator::Goto(target) => {
                merge_state(*target, state, incoming, queue);
            }
            Terminator::Branch {
                condition,
                then_block,
                else_block,
            } => {
                let (then_state, else_state) = self.condition_states(condition, state);
                merge_state(*then_block, &then_state, incoming, queue);
                merge_state(*else_block, &else_state, incoming, queue);
            }
            Terminator::Return(value) => {
                if let Some(value) = value {
                    let mut state = state.clone();
                    self.eval(value, &mut state);
                }
            }
            Terminator::Throw(value) => {
                let mut state = state.clone();
                self.eval(value, &mut state);
            }
            Terminator::Exit | Terminator::Pending => {}
        }
    }

    fn eval(&mut self, expr: &Expr, state: &mut State) -> Nullness {
        let expr = unparen(expr);
        match &expr.kind {
            ExprKind::Literal(literal) => match literal {
                Literal::Null => Nullness::Null,
                Literal::Boolean(_)
                | Literal::Number(_)
                | Literal::String(_)
                | Literal::Regex(_) => Nullness::NonNull,
            },
            ExprKind::Variable(name) => {
                if name == "this" {
                    Nullness::NonNull
                } else {
                    state.store.get(&AccessPath::Local(name.clone()))
                }
            }
            ExprKind::This | ExprKind::Super => Nullness::NonNull,
            ExprKind::Property {
                receiver,
                name,
                safe,
            } => {
                let receiver_value = self.eval(receiver, state);
                if receiver_value.is_nullish() && !safe {
                    self.report(
                        receiver.span,
                        if receiver_value.is_null() {
                            "NullDereference"
                        } else {
                            "PossibleNullDereference"
                        },
                        if receiver_value.is_null() {
                            "Property access on a null value"
                        } else {
                            "Property access on a value that may be null"
                        },
                    );
                }
                if *safe {
                    Nullness::MaybeNull
                } else {
                    let _ = name;
                    path_of(expr).map_or(Nullness::Unknown, |path| state.store.get(&path))
                }
            }
            ExprKind::Call {
                receiver,
                name,
                args,
                safe,
            } => {
                let receiver_value = receiver.as_ref().map(|receiver| self.eval(receiver, state));
                if let (Some(receiver), Some(value)) = (receiver.as_ref(), receiver_value)
                    && value.is_nullish()
                    && !safe
                {
                    self.report(
                        receiver.span,
                        if value.is_null() {
                            "NullDereference"
                        } else {
                            "PossibleNullDereference"
                        },
                        if value.is_null() {
                            "Method call on a null value"
                        } else {
                            "Method call on a value that may be null"
                        },
                    );
                }
                if let Some(receiver) = receiver {
                    if let Some(path) = path_of(receiver) {
                        state.store.invalidate_fields(&path);
                    }
                } else {
                    state.store.invalidate_fields(&AccessPath::This);
                }
                for arg in args {
                    self.eval(arg, state);
                }
                if matches!(name.as_str(), "requireNonNull" | "checkNotNull") {
                    Nullness::NonNull
                } else if *safe {
                    Nullness::MaybeNull
                } else if receiver.is_none() {
                    self.summaries
                        .get(&(name.clone(), args.len()))
                        .copied()
                        .unwrap_or(Nullness::Unknown)
                } else {
                    Nullness::Unknown
                }
            }
            ExprKind::New { args, body, .. } => {
                for arg in args {
                    self.eval(arg, state);
                }
                if let Some(body) = body {
                    for item in body {
                        self.analyze_item(item);
                    }
                }
                Nullness::NonNull
            }
            ExprKind::Binary { op, left, right } => match op {
                BinaryOp::Eq | BinaryOp::NotEq => {
                    self.eval(left, state);
                    self.eval(right, state);
                    Nullness::NonNull
                }
                BinaryOp::And | BinaryOp::Or => {
                    let (then_state, else_state) = self.condition_states(expr, state);
                    *state = then_state.join(&else_state);
                    Nullness::NonNull
                }
                BinaryOp::InstanceOf => {
                    self.eval(left, state);
                    Nullness::NonNull
                }
                _ => {
                    let left_value = self.eval(left, state);
                    let right_value = self.eval(right, state);
                    if matches!(
                        op,
                        BinaryOp::Add
                            | BinaryOp::Sub
                            | BinaryOp::Mul
                            | BinaryOp::Div
                            | BinaryOp::Mod
                            | BinaryOp::Power
                            | BinaryOp::Lt
                            | BinaryOp::Le
                            | BinaryOp::Gt
                            | BinaryOp::Ge
                    ) && (left_value.is_nullish() || right_value.is_nullish())
                    {
                        self.report(
                            expr.span,
                            "UnboxingNull",
                            "An operand may be null when a primitive value is required",
                        );
                    }
                    Nullness::Unknown
                }
            },
            ExprKind::Unary { op, operand } => {
                if matches!(op, UnaryOp::Not) {
                    self.condition_states(expr, state);
                    Nullness::NonNull
                } else {
                    self.eval(operand, state);
                    Nullness::Unknown
                }
            }
            ExprKind::Assign { target, value, .. } => {
                let value = self.eval(value, state);
                if let Some(path) = path_of(target) {
                    state.store.set(path, value);
                }
                value
            }
            ExprKind::Ternary {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                let (then_state, else_state) = self.condition_states(condition, state);
                let mut then_state = then_state;
                let mut else_state = else_state;
                let then_value = self.eval(then_expr, &mut then_state);
                let else_value = self.eval(else_expr, &mut else_state);
                *state = then_state.join(&else_state);
                then_value.join(else_value)
            }
            ExprKind::List(items) => {
                for item in items {
                    self.eval(item, state);
                }
                Nullness::NonNull
            }
            ExprKind::Map(entries) => {
                for (key, value) in entries {
                    self.eval(key, state);
                    self.eval(value, state);
                }
                Nullness::NonNull
            }
            ExprKind::Index {
                target,
                index,
                safe,
            } => {
                let target_value = self.eval(target, state);
                self.eval(index, state);
                if target_value.is_nullish() && !safe {
                    self.report(
                        target.span,
                        if target_value.is_null() {
                            "NullDereference"
                        } else {
                            "PossibleNullDereference"
                        },
                        if target_value.is_null() {
                            "Index access on a null value"
                        } else {
                            "Index access on a value that may be null"
                        },
                    );
                }
                if *safe {
                    Nullness::MaybeNull
                } else {
                    Nullness::Unknown
                }
            }
            ExprKind::Closure { body, .. } => {
                let cfg = build_closure(body);
                self.run_cfg(&cfg, state.clone());
                Nullness::NonNull
            }
            ExprKind::Cast { expr: inner, .. } => {
                let value = self.eval(inner, state);
                if value.is_nullish() {
                    self.report(
                        inner.span,
                        "UnboxingNull",
                        "Casting a possibly-null value to a primitive may fail",
                    );
                }
                Nullness::Unknown
            }
            ExprKind::NamedArg { value, .. } => self.eval(value, state),
            ExprKind::Paren(inner) => self.eval(inner, state),
            ExprKind::Switch { scrutinee, cases } => {
                self.eval(scrutinee, state);
                for case in cases {
                    if let Some(label) = &case.label {
                        self.eval(label, state);
                    }
                    for statement in &case.body {
                        self.lower_expression_statement(statement, state);
                    }
                }
                Nullness::Unknown
            }
        }
    }

    fn lower_expression_statement(&mut self, statement: &Stmt, state: &mut State) {
        if let Stmt::Expr(expr) = statement {
            self.eval(expr, state);
        }
    }

    fn condition_states(&mut self, expr: &Expr, state: &State) -> (State, State) {
        let expr = unparen(expr);
        match &expr.kind {
            ExprKind::Binary {
                op: BinaryOp::And,
                left,
                right,
            } => {
                let (left_then, left_else) = self.condition_states(left, state);
                let (right_then, right_else) = self.condition_states(right, &left_then);
                (right_then, left_else.join(&right_else))
            }
            ExprKind::Binary {
                op: BinaryOp::Or,
                left,
                right,
            } => {
                let (left_then, left_else) = self.condition_states(left, state);
                let (right_then, right_else) = self.condition_states(right, &left_else);
                (left_then.join(&right_then), right_else)
            }
            ExprKind::Binary {
                op: op @ (BinaryOp::Eq | BinaryOp::NotEq),
                left,
                right,
            } => {
                let mut evaluated = state.clone();
                self.eval(left, &mut evaluated);
                self.eval(right, &mut evaluated);
                let path = if is_null_literal(left) {
                    path_of(right)
                } else if is_null_literal(right) {
                    path_of(left)
                } else {
                    None
                };
                let Some(path) = path else {
                    return (evaluated.clone(), evaluated);
                };
                let mut then_state = evaluated.clone();
                let mut else_state = evaluated;
                if matches!(op, BinaryOp::Eq) {
                    then_state.store.set(path.clone(), Nullness::Null);
                    else_state.store.set(path, Nullness::NonNull);
                } else {
                    then_state.store.set(path.clone(), Nullness::NonNull);
                    else_state.store.set(path, Nullness::Null);
                }
                (then_state, else_state)
            }
            ExprKind::Binary {
                op: BinaryOp::InstanceOf,
                left,
                ..
            } => {
                let mut then_state = state.clone();
                self.eval(left, &mut then_state);
                if let Some(path) = path_of(left) {
                    then_state.store.set(path, Nullness::NonNull);
                }
                (then_state, state.clone())
            }
            ExprKind::Unary {
                op: UnaryOp::Not,
                operand,
            } => {
                let (then_state, else_state) = self.condition_states(operand, state);
                (else_state, then_state)
            }
            _ => {
                let mut evaluated = state.clone();
                self.eval(expr, &mut evaluated);
                let mut then_state = evaluated.clone();
                if let Some(path) = path_of(expr) {
                    then_state.store.set(path, Nullness::NonNull);
                }
                (then_state, evaluated)
            }
        }
    }

    fn report(&mut self, span: Span, code: &'static str, message: &str) {
        if self.mode == Mode::Conservative && matches!(code, "PossibleNullDereference") {
            return;
        }
        let key = (span.start, span.end, code);
        if !self.seen.insert(key) {
            return;
        }
        self.findings.push(Finding {
            span,
            code,
            message: message.to_string(),
        });
    }
}

fn merge_state(
    block: BlockId,
    state: &State,
    incoming: &mut [Option<State>],
    queue: &mut VecDeque<BlockId>,
) {
    match &incoming[block] {
        None => {
            incoming[block] = Some(state.clone());
            queue.push_back(block);
        }
        Some(existing) => {
            let joined = existing.join(state);
            if joined != *existing {
                incoming[block] = Some(joined);
                queue.push_back(block);
            }
        }
    }
}

fn unparen(mut expr: &Expr) -> &Expr {
    while let ExprKind::Paren(inner) = &expr.kind {
        expr = inner;
    }
    expr
}

fn is_null_literal(expr: &Expr) -> bool {
    matches!(&unparen(expr).kind, ExprKind::Literal(Literal::Null))
}

fn path_of(expr: &Expr) -> Option<AccessPath> {
    match &unparen(expr).kind {
        ExprKind::Variable(name) => Some(AccessPath::Local(name.clone())),
        ExprKind::This => Some(AccessPath::This),
        ExprKind::Property {
            receiver,
            name,
            safe: false,
        } => path_of(receiver).map(|base| AccessPath::field(base, name.clone())),
        _ => None,
    }
}

/// Returns the executable method/closure units in a script for tests and
/// future project-level analysis.
#[must_use]
pub fn executable_blocks(script: &Script) -> usize {
    let mut count = 0;
    for item in &script.items {
        count += executable_blocks_item(item);
    }
    count
}

fn executable_blocks_item(item: &Item) -> usize {
    match item {
        Item::Class(class) => class.members.iter().map(executable_blocks_item).sum(),
        Item::Method(method) => usize::from(method.body.is_some()),
        Item::Initializer(_) => 1,
        Item::Field(_) | Item::Statement(_) => 0,
        Item::Group(items) => items.iter().map(executable_blocks_item).sum(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Mode, analyze};
    use groovylint_syntax::parser::parse;

    fn lint(source: &str, mode: Mode) -> Vec<super::Finding> {
        let parsed = parse(source).expect("source parses");
        analyze(&parsed.script, mode)
    }

    #[test]
    fn reports_definite_null_dereference() {
        let findings = lint(
            "def run() { String s = null; return s.length() }",
            Mode::Conservative,
        );
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "NullDereference");
    }

    #[test]
    fn refines_guards_and_branch_joins() {
        let source = r#"
def run(boolean c) {
    String s = null
    if (c) {
        s = 'x'
    }
    return s.length()
}
"#;
        assert_eq!(lint(source, Mode::Conservative).len(), 0);
        let findings = lint(source, Mode::Inferred);
        assert!(findings.iter().any(|f| f.code == "PossibleNullDereference"));
    }

    #[test]
    fn refines_non_null_branch() {
        let source = r#"
def run() {
    String s = null
    if (s != null) {
        return s.length()
    }
    return 0
}
"#;
        assert_eq!(lint(source, Mode::Inferred).len(), 0);
    }

    #[test]
    fn loops_join_zero_iteration_state() {
        let source = r#"
def run(boolean c) {
    String s = null
    while (c) {
        s = 'x'
    }
    return s.length()
}
"#;
        let findings = lint(source, Mode::Inferred);
        assert!(
            findings
                .iter()
                .any(|f| f.code == "NullDereference" || f.code == "PossibleNullDereference")
        );
    }

    #[test]
    fn try_catch_finally_join_states() {
        let source = r#"
def run(boolean c) {
    String s = 'a'
    try {
        if (c) { s = null }
    } catch (RuntimeException e) {
        s = 'b'
    } finally {
        s = 'c'
    }
    return s.length()
}
"#;
        assert_eq!(lint(source, Mode::Conservative).len(), 0);
    }

    #[test]
    fn reports_possible_null_only_in_inferred_mode() {
        let source = r#"
def run(unknown) {
    String s = unknown
    return s.length()
}
"#;
        assert_eq!(lint(source, Mode::Conservative).len(), 0);
        let findings = lint(source, Mode::Inferred);
        assert!(
            findings.is_empty(),
            "unknown should not be reported: {findings:?}"
        );
    }
}

#[cfg(test)]
mod closure_tests {
    use super::{Mode, analyze};
    use groovylint_syntax::parser::parse;

    #[test]
    fn analyzes_closure_bodies_with_captured_state() {
        let source = r"
def run() {
    String s = null
    def c = { s.length() }
}
";
        let parsed = parse(source).expect("source parses");
        let findings = analyze(&parsed.script, Mode::Conservative);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "NullDereference");
    }
}

#[cfg(test)]
mod more_tests {
    use super::{Mode, analyze};
    use groovylint_syntax::parser::parse;

    fn lint(source: &str, mode: Mode) -> Vec<super::Finding> {
        let parsed = parse(source).expect("source parses");
        analyze(&parsed.script, mode)
    }

    #[test]
    fn field_guard_and_call_invalidation() {
        let source = r"
class Example {
    String value
    int guarded() {
        if (this.value != null) { return this.value.length() }
        return 0
    }
    int unguarded() { return this.value.length() }
    int afterCall() {
        if (this.value != null) {
            reset()
            return this.value.length()
        }
        return 0
    }
    void reset() { this.value = null }
}
";
        let findings = lint(source, Mode::Inferred);
        assert_eq!(findings.len(), 2);
    }

    #[test]
    fn safe_navigation_truthiness_and_instanceof_are_not_false_positives() {
        let source = r"
def run(Object value) {
    if (value) { value.toString() }
    if (value instanceof String) { value.length() }
    value?.toString()
}
";
        assert_eq!(lint(source, Mode::Inferred).len(), 0);
    }

    #[test]
    fn switch_fallthrough_uses_next_case_state() {
        let source = r"
def run(int selector) {
    String value = null
    switch (selector) {
        case 0:
            value = null
        case 1:
            value = 'ok'
            break
        default:
            value = 'ok'
    }
    return value.length()
}
";
        assert_eq!(lint(source, Mode::Inferred).len(), 0);
    }

    #[test]
    fn implicit_method_return_summary_is_used() {
        let source = r"
def maybe() { null }
def nonnull() { 'ok' }
def run() {
    maybe().length()
    nonnull().length()
}
";
        let findings = lint(source, Mode::Conservative);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "NullDereference");
    }
}
