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
use crate::contracts::{self, ContractPath, ContractRegistry};
use crate::nullability::{AccessPath, Defaults, Finding, Mode, Nullness, Store};

/// Analyzes all executable units in a script.
#[must_use]
pub fn analyze(script: &Script, mode: Mode) -> Vec<Finding> {
    let registry = contracts::registry(script, None).unwrap_or_default();
    analyze_with_contracts(script, mode, registry)
}

/// Analyzes a script with contracts loaded from annotations, built-ins, and/or
/// an external DSL registry.
#[must_use]
pub fn analyze_with_contracts(
    script: &Script,
    mode: Mode,
    contracts: ContractRegistry,
) -> Vec<Finding> {
    analyze_with_defaults(script, mode, contracts, Defaults::default())
}

/// Analyzes with contracts and package/compiler defaults.
#[must_use]
pub fn analyze_with_defaults(
    script: &Script,
    mode: Mode,
    contracts: ContractRegistry,
    defaults: Defaults,
) -> Vec<Finding> {
    let mut analyzer = Analyzer::new(mode, contracts, defaults);
    analyzer.analyze_script(script);
    analyzer.finish()
}

struct Analyzer {
    mode: Mode,
    findings: Vec<Finding>,
    seen: BTreeSet<(usize, usize, &'static str)>,
    summaries: std::collections::BTreeMap<(String, usize), Nullness>,
    contracts: ContractRegistry,
    return_expectation: Option<Nullness>,
    param_requirements: std::collections::BTreeMap<(String, usize), Vec<usize>>,
    collect_required: bool,
    param_names: Vec<String>,
    required_params: BTreeSet<usize>,
    getters: std::collections::BTreeMap<String, Nullness>,
    primitive_params: std::collections::BTreeMap<(String, usize), Vec<bool>>,
    return_primitive: bool,
    defaults: Defaults,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct State {
    store: Store,
    closures: std::collections::BTreeMap<AccessPath, Nullness>,
    fields: BTreeSet<String>,
    locals: BTreeSet<String>,
    local_expectations: std::collections::BTreeMap<String, Nullness>,
    local_types: std::collections::BTreeMap<String, String>,
    field_expectations: std::collections::BTreeMap<String, Nullness>,
    monotonic_fields: BTreeSet<String>,
    monotonic_nonnull: BTreeSet<String>,
    unreachable: bool,
}

impl State {
    fn new() -> Self {
        let mut store = Store::new();
        store.set(AccessPath::This, Nullness::NonNull);
        Self {
            store,
            closures: std::collections::BTreeMap::new(),
            fields: BTreeSet::new(),
            locals: BTreeSet::new(),
            local_expectations: std::collections::BTreeMap::new(),
            local_types: std::collections::BTreeMap::new(),
            field_expectations: std::collections::BTreeMap::new(),
            monotonic_fields: BTreeSet::new(),
            monotonic_nonnull: BTreeSet::new(),
            unreachable: false,
        }
    }

    fn with_params(params: &[Param]) -> Self {
        let mut state = Self::new();
        for param in params {
            let value = annotation_nullness(&param.annotations).unwrap_or(Nullness::Unknown);
            state.locals.insert(param.name.clone());
            if let Some(type_name) = &param.type_name {
                state
                    .local_types
                    .insert(param.name.clone(), type_name.clone());
            }
            if let Some(expected) = annotation_nullness(&param.annotations) {
                state
                    .local_expectations
                    .insert(param.name.clone(), expected);
            }
            state
                .store
                .set(AccessPath::Local(param.name.clone()), value);
        }
        state
    }

    fn with_monotonic(mut self, fields: &BTreeSet<String>) -> Self {
        self.monotonic_fields.clone_from(fields);
        self
    }

    fn restore_monotonic(&mut self) {
        for name in &self.monotonic_nonnull {
            self.store.set(
                AccessPath::field(AccessPath::This, name.clone()),
                Nullness::NonNull,
            );
        }
    }

    fn with_params_marked(params: &[Param]) -> Self {
        let mut state = Self::with_params(params);
        for param in params {
            if annotation_nullness(&param.annotations).is_none() {
                state
                    .store
                    .set(AccessPath::Local(param.name.clone()), Nullness::NonNull);
                state
                    .local_expectations
                    .insert(param.name.clone(), Nullness::NonNull);
            }
        }
        state
    }

    fn with_field_expectations(mut self, expectations: &[(String, Nullness)]) -> Self {
        for (name, value) in expectations {
            self.field_expectations.insert(name.clone(), *value);
        }
        self
    }

    fn with_params_nullable(params: &[Param]) -> Self {
        let mut state = Self::new();
        for param in params {
            state.locals.insert(param.name.clone());
            if let Some(type_name) = &param.type_name {
                state
                    .local_types
                    .insert(param.name.clone(), type_name.clone());
            }
            state
                .store
                .set(AccessPath::Local(param.name.clone()), Nullness::MaybeNull);
        }
        state
    }

    fn with_fields(mut self, fields: &[(String, Nullness)]) -> Self {
        for (name, value) in fields {
            self.fields.insert(name.clone());
            self.store
                .set(AccessPath::field(AccessPath::This, name.clone()), *value);
        }
        self
    }

    fn join(&self, other: &Self) -> Self {
        let mut closures = self.closures.clone();
        for (path, value) in &other.closures {
            closures
                .entry(path.clone())
                .and_modify(|existing| *existing = existing.join(*value))
                .or_insert(*value);
        }
        let mut fields = self.fields.clone();
        fields.extend(other.fields.iter().cloned());
        let mut locals = self.locals.clone();
        locals.extend(other.locals.iter().cloned());
        let mut local_expectations = self.local_expectations.clone();
        for (name, value) in &other.local_expectations {
            local_expectations
                .entry(name.clone())
                .and_modify(|existing| *existing = existing.join(*value))
                .or_insert(*value);
        }
        let mut local_types = self.local_types.clone();
        local_types.extend(
            other
                .local_types
                .iter()
                .map(|(k, v)| (k.clone(), v.clone())),
        );
        let mut monotonic_fields = self.monotonic_fields.clone();
        monotonic_fields.extend(other.monotonic_fields.iter().cloned());
        let mut monotonic_nonnull = self.monotonic_nonnull.clone();
        monotonic_nonnull.retain(|name| other.monotonic_nonnull.contains(name));
        Self {
            store: self.store.join(&other.store),
            closures,
            fields,
            locals,
            local_expectations,
            local_types,
            field_expectations: self.field_expectations.clone(),
            monotonic_fields,
            monotonic_nonnull,
            unreachable: self.unreachable && other.unreachable,
        }
    }
}

impl Analyzer {
    fn new(mode: Mode, contracts: ContractRegistry, defaults: Defaults) -> Self {
        Self {
            mode,
            findings: Vec::new(),
            seen: BTreeSet::new(),
            summaries: std::collections::BTreeMap::new(),
            contracts,
            return_expectation: None,
            param_requirements: std::collections::BTreeMap::new(),
            collect_required: false,
            param_names: Vec::new(),
            required_params: BTreeSet::new(),
            getters: std::collections::BTreeMap::new(),
            primitive_params: std::collections::BTreeMap::new(),
            return_primitive: false,
            defaults,
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
                    self.primitive_params.insert(
                        (method.name.clone(), method.params.len()),
                        method
                            .params
                            .iter()
                            .map(|param| param.type_name.as_deref().is_some_and(is_primitive_type))
                            .collect(),
                    );
                    let required = self.summarize_parameters(method);
                    if !required.is_empty() {
                        self.param_requirements
                            .entry((method.name.clone(), method.params.len()))
                            .or_insert(required);
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
        if let Some(value) = annotation_nullness(&method.annotations) {
            return Some(value);
        }
        if let Some(contract) = self.contracts.find(&method.name, method.params.len())
            && let Some(value) = contract.returns
        {
            return Some(value);
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

    fn summarize_block(&self, body: &groovylint_syntax::ast::Block) -> Nullness {
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
        values
            .into_iter()
            .reduce(Nullness::join)
            .unwrap_or(Nullness::Unknown)
    }

    fn summarize_parameters(&self, method: &MethodDecl) -> Vec<usize> {
        let Some(cfg) = build_method(method) else {
            return Vec::new();
        };
        let mut child = Analyzer::new(self.mode, self.contracts.clone(), self.defaults);
        child.collect_required = true;
        child.param_names = method
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect();
        child.run_cfg(&cfg, State::with_params_nullable(&method.params));
        child.required_params.into_iter().collect()
    }

    fn record_required(&mut self, receiver: &Expr, state: &State) {
        if !self.collect_required {
            return;
        }
        let Some(AccessPath::Local(name)) = path_of_in(receiver, state) else {
            return;
        };
        if let Some(index) = self.param_names.iter().position(|param| param == &name) {
            self.required_params.insert(index);
        }
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
            Item::Method(method) => {
                self.analyze_method(method, &[], &[], false, false, &BTreeSet::new());
            }
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
        let class_marked = class.annotations.iter().any(|annotation| {
            annotation == "NullMarked"
                || annotation.ends_with(".NullMarked")
                || annotation == "NonNullApi"
                || annotation.ends_with(".NonNullApi")
        });
        let unmarked = class.annotations.iter().any(|annotation| {
            annotation == "NullUnmarked" || annotation.ends_with(".NullUnmarked")
        });
        let params_marked = !unmarked && (class_marked || self.defaults.params_nonnull);
        let returns_marked = !unmarked && (class_marked || self.defaults.returns_nonnull);
        let fields = self.field_values(class);
        if self.mode == Mode::Strict {
            self.check_field_initialization(class, class_marked || self.defaults.fields_nonnull);
        }
        let expectations: Vec<(String, Nullness)> = class
            .members
            .iter()
            .filter_map(|member| match member {
                Item::Field(field) => {
                    annotation_nullness(&field.annotations).map(|value| (field.name.clone(), value))
                }
                _ => None,
            })
            .collect();
        let monotonic: BTreeSet<String> = class
            .members
            .iter()
            .filter_map(|member| match member {
                Item::Field(field)
                    if field.annotations.iter().any(|annotation| {
                        annotation == "MonotonicNonNull"
                            || annotation.ends_with(".MonotonicNonNull")
                    }) =>
                {
                    Some(field.name.clone())
                }
                _ => None,
            })
            .collect();
        let previous_getters = self.getters.clone();
        for member in &class.members {
            if let Item::Method(method) = member
                && let Some(property) = property_name(&method.name)
            {
                let value = annotation_nullness(&method.annotations)
                    .or_else(|| self.summarize_method(method));
                if let Some(value) = value {
                    self.getters.insert(property, value);
                }
            }
        }
        for member in &class.members {
            match member {
                Item::Method(method) => self.analyze_method(
                    method,
                    &fields,
                    &expectations,
                    params_marked,
                    returns_marked,
                    &monotonic,
                ),
                Item::Initializer(initializer) => {
                    let cfg = build_closure(&initializer.body);
                    self.run_cfg(&cfg, State::new().with_fields(&fields));
                }
                Item::Class(nested) => self.analyze_class(nested),
                Item::Field(_) => {}
                Item::Statement(_) | Item::Group(_) => self.analyze_item(member),
            }
        }
        self.getters = previous_getters;
    }

    fn check_field_initialization(&mut self, class: &ClassDecl, fields_default: bool) {
        let constructors: Vec<&MethodDecl> = class
            .members
            .iter()
            .filter_map(|member| match member {
                Item::Method(method) if method.name == class.name => Some(method),
                _ => None,
            })
            .collect();
        for member in &class.members {
            let Item::Field(field) = member else {
                continue;
            };
            if field.init.is_some() {
                continue;
            }
            let expected = annotation_nullness(&field.annotations);
            let required =
                expected == Some(Nullness::NonNull) || (fields_default && expected.is_none());
            if !required {
                continue;
            }
            let missing = if constructors.is_empty() {
                true
            } else {
                constructors
                    .iter()
                    .any(|constructor| !method_assigns_field(constructor, &field.name))
            };
            if missing {
                self.report(
                    field.span,
                    "FieldNotInitialized",
                    &format!(
                        "Non-null field `{}` is not initialized by every constructor",
                        field.name
                    ),
                );
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
            let value = if let Some(init) = &field.init {
                self.eval(init, &mut state)
            } else {
                annotation_nullness(&field.annotations).unwrap_or(Nullness::Unknown)
            };
            values.push((field.name.clone(), value));
            state.store.set(
                AccessPath::field(AccessPath::This, field.name.clone()),
                value,
            );
        }
        values
    }

    fn analyze_method(
        &mut self,
        method: &MethodDecl,
        fields: &[(String, Nullness)],
        expectations: &[(String, Nullness)],
        params_marked: bool,
        returns_marked: bool,
        monotonic: &BTreeSet<String>,
    ) {
        let Some(cfg) = build_method(method) else {
            return;
        };
        let previous = self.return_expectation;
        self.return_primitive = method.return_type.as_deref().is_some_and(is_primitive_type);
        self.return_expectation = annotation_nullness(&method.annotations)
            .or_else(|| {
                self.contracts
                    .find(&method.name, method.params.len())
                    .and_then(|contract| contract.returns)
            })
            .or(if returns_marked {
                Some(Nullness::NonNull)
            } else {
                None
            });
        self.run_cfg(
            &cfg,
            if params_marked {
                State::with_params_marked(&method.params)
            } else {
                State::with_params(&method.params)
            }
            .with_fields(fields)
            .with_field_expectations(expectations)
            .with_monotonic(monotonic),
        );
        self.return_expectation = previous;
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
                if state.unreachable {
                    break;
                }
            }
            if state.unreachable {
                continue;
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
            Instruction::Declare {
                name,
                init,
                annotations,
                type_name,
            } => {
                let value = init
                    .as_ref()
                    .map_or(Nullness::Null, |init| self.eval(init, state));
                state.locals.insert(name.clone());
                if let Some(type_name) = type_name {
                    state.local_types.insert(name.clone(), type_name.clone());
                }
                if let Some(expected) = annotation_detail_nullness(annotations) {
                    state.local_expectations.insert(name.clone(), expected);
                }
                if type_name.as_deref().is_some_and(is_primitive_type) && value.is_nullish() {
                    self.report(
                        init.as_ref()
                            .map_or_else(|| Span::empty(0), |expr| expr.span),
                        if value.is_null() {
                            "UnboxingNull"
                        } else {
                            "PossibleUnboxingNull"
                        },
                        "A nullable value is unboxed into a primitive local",
                    );
                }
                let path = AccessPath::Local(name.clone());
                if annotation_detail_nullness(annotations) == Some(Nullness::NonNull)
                    && value.is_nullish()
                {
                    self.report(
                        init.as_ref()
                            .map_or_else(|| Span::empty(0), |expr| expr.span),
                        if value.is_null() {
                            "NullAssignment"
                        } else {
                            "PossibleNullAssignment"
                        },
                        "A nullable value is assigned to a non-null local",
                    );
                }
                state.store.set(path.clone(), value);
                if let Some(Expr {
                    kind: ExprKind::Closure { body, .. },
                    ..
                }) = init
                {
                    let summary = self.summarize_block(body);
                    state.closures.insert(path, summary);
                }
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
                    let returned = self.eval(value, &mut state);
                    if self.return_primitive && returned.is_nullish() {
                        self.report(
                            value.span,
                            if returned.is_null() {
                                "UnboxingNull"
                            } else {
                                "PossibleUnboxingNull"
                            },
                            "A nullable value is returned from a primitive method",
                        );
                    }
                    if !self.return_primitive
                        && let Some(expected) = self.return_expectation
                        && expected == Nullness::NonNull
                        && returned.is_nullish()
                    {
                        self.report(
                            value.span,
                            if returned.is_null() {
                                "NullReturn"
                            } else {
                                "PossibleNullReturn"
                            },
                            "A nullable value is returned from a non-null method",
                        );
                    }
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
                    path_of_in(expr, state).map_or(Nullness::Unknown, |path| state.store.get(&path))
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
                    self.record_required(receiver, state);
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
                    let value = path_of_in(expr, state)
                        .map_or(Nullness::Unknown, |path| state.store.get(&path));
                    if value == Nullness::Unknown {
                        self.getters.get(name).copied().unwrap_or(Nullness::Unknown)
                    } else {
                        value
                    }
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
                    self.record_required(receiver, state);
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
                if !is_pure_method(name) {
                    if let Some(receiver) = receiver {
                        if let Some(path) = path_of_in(receiver, state) {
                            state.store.invalidate_fields(&path);
                        }
                    } else {
                        state.store.invalidate_fields(&AccessPath::This);
                    }
                    state.restore_monotonic();
                }
                let mut arg_values = Vec::new();
                for arg in args {
                    let value = self.eval(arg, state);
                    if !matches!(name.as_str(), "call" | "invoke")
                        && let Some(path) = path_of_in(arg, state)
                    {
                        state.closures.remove(&path);
                    }
                    arg_values.push((arg, value));
                }
                if receiver.is_none()
                    && let Some(primitives) = self
                        .primitive_params
                        .get(&(name.clone(), args.len()))
                        .cloned()
                {
                    for (index, primitive) in primitives.iter().enumerate() {
                        if *primitive
                            && let Some((arg, value)) = arg_values.get(index)
                            && value.is_nullish()
                        {
                            self.report(
                                arg.span,
                                if value.is_null() {
                                    "UnboxingNull"
                                } else {
                                    "PossibleUnboxingNull"
                                },
                                &format!(
                                    "Argument #{} of {name} is a primitive and cannot be null",
                                    index + 1
                                ),
                            );
                        }
                    }
                }
                let contract = self.contracts.find(name, args.len()).cloned();
                if let Some(contract) = contract {
                    for index in &contract.required_params {
                        let primitive = self
                            .primitive_params
                            .get(&(name.clone(), args.len()))
                            .and_then(|params| params.get(*index))
                            .copied()
                            .unwrap_or(false);
                        if primitive {
                            continue;
                        }
                        if let Some((arg, value)) = arg_values.get(*index)
                            && value.is_nullish()
                        {
                            self.report(
                                arg.span,
                                if value.is_null() {
                                    "NullArgument"
                                } else {
                                    "PossibleNullArgument"
                                },
                                &format!("Argument #{} of {name} is declared non-null", index + 1),
                            );
                        }
                    }
                    for path in &contract.ensures {
                        Self::apply_contract_path(path, state, &arg_values);
                    }
                    if let Some(index) = contract.receiver_param
                        && let Some(receiver) = receiver
                        && let ExprKind::Call { args, .. } = &unparen(receiver).kind
                        && let Some(arg) = args.get(index)
                        && let Some(path) = path_of_in(arg, state)
                    {
                        state.store.set(path, Nullness::NonNull);
                    }
                    if let Some((index, expected)) = contract.condition_param
                        && let Some((arg, _)) = arg_values.get(index)
                    {
                        let (then_state, else_state) = self.condition_states(arg, state);
                        *state = if expected { then_state } else { else_state };
                    }
                    if contract.never_returns {
                        state.unreachable = true;
                    }
                    let lookup = path_of_in(expr, state);
                    let known_lookup = lookup
                        .as_ref()
                        .is_some_and(|path| state.store.values().contains_key(path));
                    let result = if let Some(path) = &lookup
                        && known_lookup
                    {
                        state.store.get(path)
                    } else if let Some(value) = contract.returns {
                        value
                    } else if let Some(index) = contract.returns_param {
                        arg_values
                            .get(index)
                            .map_or(Nullness::Unknown, |(_, value)| *value)
                    } else if let Some(index) = contract.return_nonnull_if_param {
                        match arg_values.get(index).map(|(_, value)| *value) {
                            Some(Nullness::NonNull) => Nullness::NonNull,
                            Some(Nullness::Null) => Nullness::Null,
                            _ => Nullness::Unknown,
                        }
                    } else if contract.ensures.is_empty() {
                        self.fallback_call_result(
                            name,
                            args.len(),
                            receiver.as_deref(),
                            *safe,
                            state,
                        )
                    } else {
                        Nullness::Unknown
                    };
                    if known_lookup {
                        return result;
                    }
                    receiver
                        .as_ref()
                        .and_then(|receiver| local_type(receiver, state))
                        .map_or(result, |type_name| {
                            if (type_name.contains("Map")
                                || type_name.contains("List")
                                || type_name.contains("Collection")
                                || type_name.contains("Set"))
                                && matches!(name.as_str(), "get" | "find" | "max" | "min")
                            {
                                Nullness::MaybeNull
                            } else if type_name.contains("Optional") && name == "get" {
                                Nullness::NonNull
                            } else {
                                result
                            }
                        })
                } else {
                    if receiver.is_none()
                        && let Some(required) = self
                            .param_requirements
                            .get(&(name.clone(), args.len()))
                            .cloned()
                    {
                        for index in &required {
                            if let Some((arg, value)) = arg_values.get(*index)
                                && value.is_nullish()
                            {
                                self.report(
                                    arg.span,
                                    if value.is_null() {
                                        "NullArgument"
                                    } else {
                                        "PossibleNullArgument"
                                    },
                                    &format!(
                                        "Argument #{} of {name} is required to be non-null by inferred control flow",
                                        index + 1
                                    ),
                                );
                            }
                        }
                    }
                    self.fallback_call_result(name, args.len(), receiver.as_deref(), *safe, state)
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
                let assigned = self.eval(value, state);
                if let Some(path) = path_of_in(target, state) {
                    if let AccessPath::Field(_, name) = &path
                        && state.monotonic_fields.contains(name)
                    {
                        if assigned.is_nullish() && state.monotonic_nonnull.contains(name) {
                            self.report(
                                value.span,
                                if assigned.is_null() {
                                    "NullAssignment"
                                } else {
                                    "PossibleNullAssignment"
                                },
                                "A @MonotonicNonNull field is assigned a nullable value after being initialized",
                            );
                        }
                        if assigned == Nullness::NonNull {
                            state.monotonic_nonnull.insert(name.clone());
                        }
                    }
                    let expected = match &path {
                        AccessPath::Local(name) => state.local_expectations.get(name),
                        AccessPath::Field(_, name) => state.field_expectations.get(name),
                        _ => None,
                    };
                    if expected == Some(&Nullness::NonNull) && assigned.is_nullish() {
                        self.report(
                            value.span,
                            if assigned.is_null() {
                                "NullAssignment"
                            } else {
                                "PossibleNullAssignment"
                            },
                            "A nullable value is assigned to a non-null location",
                        );
                    }
                    state.store.set(path.clone(), assigned);
                    if let ExprKind::Closure { body, .. } = &unparen(value).kind {
                        let summary = self.summarize_block(body);
                        state.closures.insert(path, summary);
                    }
                }
                assigned
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
                    self.record_required(target, state);
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
                if let Some(path) = path_of_in(expr, state)
                    && state.store.values().contains_key(&path)
                {
                    return state.store.get(&path);
                }
                if *safe
                    || local_type(target, state).is_some_and(|type_name| {
                        type_name.contains("Map")
                            || type_name.contains("List")
                            || type_name.contains("Collection")
                    })
                {
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
        if let ExprKind::Call {
            receiver,
            name,
            args,
            ..
        } = &expr.kind
            && matches!(name.as_str(), "containsKey" | "nonNull" | "isNull")
        {
            let mut evaluated = state.clone();
            self.eval(expr, &mut evaluated);
            let mut then_state = evaluated.clone();
            let mut else_state = evaluated;
            if let Some(arg) = args.first() {
                if name == "nonNull" {
                    if let Some(path) = path_of_in(arg, &then_state) {
                        then_state.store.set(path, Nullness::NonNull);
                    }
                } else if name == "isNull" {
                    if let Some(path) = path_of_in(arg, &else_state) {
                        else_state.store.set(path, Nullness::NonNull);
                    }
                } else if name == "containsKey"
                    && let Some(receiver) = receiver
                    && let Some(base) = path_of_in(receiver, &then_state)
                    && let Some(key) = key_id(arg)
                {
                    then_state
                        .store
                        .set(AccessPath::Lookup(Box::new(base), key), Nullness::NonNull);
                }
            }
            return (then_state, else_state);
        }
        if let ExprKind::Call { name, args, .. } = &expr.kind
            && let Some(contract) = self.contracts.find(name, args.len()).cloned()
            && !contract.ensures_if.is_empty()
        {
            let mut evaluated = state.clone();
            self.eval(expr, &mut evaluated);
            let mut arg_values = Vec::new();
            for arg in args {
                let value = self.eval(arg, &mut evaluated);
                arg_values.push((arg, value));
            }
            let mut then_state = evaluated.clone();
            let mut else_state = evaluated;
            for conditional in &contract.ensures_if {
                let target = if conditional.result {
                    &mut then_state
                } else {
                    &mut else_state
                };
                for path in &conditional.paths {
                    Self::apply_contract_path(path, target, &arg_values);
                }
            }
            return (then_state, else_state);
        }
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
                let null_side = if is_null_literal(left) {
                    Some(right)
                } else if is_null_literal(right) {
                    Some(left)
                } else {
                    None
                };
                if let Some(other) = null_side {
                    if let Some(path) = safe_base_path(other, &evaluated) {
                        let mut then_state = evaluated.clone();
                        let mut else_state = evaluated;
                        if matches!(op, BinaryOp::NotEq) {
                            then_state.store.set(path, Nullness::NonNull);
                        } else {
                            else_state.store.set(path, Nullness::NonNull);
                        }
                        return (then_state, else_state);
                    }
                    let Some(path) = path_of_in(other, &evaluated) else {
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
                    return (then_state, else_state);
                }
                if let Some(path) = safe_base_path(left, &evaluated) {
                    let mut then_state = evaluated.clone();
                    let mut else_state = evaluated;
                    if matches!(op, BinaryOp::Eq) {
                        then_state.store.set(path, Nullness::NonNull);
                    } else {
                        else_state.store.set(path, Nullness::NonNull);
                    }
                    return (then_state, else_state);
                }
                (evaluated.clone(), evaluated)
            }
            ExprKind::Binary {
                op: BinaryOp::InstanceOf,
                left,
                ..
            } => {
                let mut then_state = state.clone();
                self.eval(left, &mut then_state);
                if let Some(path) = path_of_in(left, &then_state) {
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
                if let Some(path) = path_of_in(expr, &then_state) {
                    then_state.store.set(path, Nullness::NonNull);
                }
                (then_state, evaluated)
            }
        }
    }

    fn apply_contract_path(path: &ContractPath, state: &mut State, args: &[(&Expr, Nullness)]) {
        match path {
            ContractPath::Param(index) => {
                if let Some((arg, _)) = args.get(*index)
                    && let Some(path) = path_of_in(arg, state)
                {
                    state.store.set(path, Nullness::NonNull);
                }
            }
            ContractPath::This => state.store.set(AccessPath::This, Nullness::NonNull),
            ContractPath::Field(name) => state.store.set(
                AccessPath::field(AccessPath::This, name.clone()),
                Nullness::NonNull,
            ),
        }
    }

    fn fallback_call_result(
        &self,
        name: &str,
        arity: usize,
        receiver: Option<&Expr>,
        safe: bool,
        state: &State,
    ) -> Nullness {
        if matches!(name, "call" | "invoke")
            && let Some(receiver) = receiver
            && let Some(path) = path_of_in(receiver, state)
            && let Some(summary) = state.closures.get(&path)
        {
            return *summary;
        }
        if safe {
            return Nullness::MaybeNull;
        }
        if receiver.is_none() {
            return self
                .summaries
                .get(&(name.to_string(), arity))
                .copied()
                .unwrap_or(Nullness::Unknown);
        }
        Nullness::Unknown
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

fn method_assigns_field(method: &MethodDecl, field: &str) -> bool {
    method.body.as_ref().is_some_and(|block| {
        block
            .statements
            .iter()
            .any(|stmt| stmt_assigns_field(stmt, field))
    })
}

fn block_assigns_field(block: &groovylint_syntax::ast::Block, field: &str) -> bool {
    block
        .statements
        .iter()
        .any(|stmt| stmt_assigns_field(stmt, field))
}

fn stmt_assigns_field(stmt: &Stmt, field: &str) -> bool {
    match stmt {
        Stmt::Block(block) => block_assigns_field(block, field),
        Stmt::Expr(expr) => expr_assigns_field(expr, field),
        Stmt::VarDecl(decl) => decl
            .init
            .as_ref()
            .is_some_and(|expr| expr_assigns_field(expr, field)),
        Stmt::If(branch) => {
            stmt_assigns_field(&branch.then_branch, field)
                || branch
                    .else_branch
                    .as_ref()
                    .is_some_and(|stmt| stmt_assigns_field(stmt, field))
        }
        Stmt::While(statement) => stmt_assigns_field(&statement.body, field),
        Stmt::DoWhile(statement) => stmt_assigns_field(&statement.body, field),
        Stmt::For(statement) => stmt_assigns_field(&statement.body, field),
        Stmt::ForEach(statement) => stmt_assigns_field(&statement.body, field),
        Stmt::Try(statement) => {
            block_assigns_field(&statement.body, field)
                || statement
                    .catches
                    .iter()
                    .any(|clause| block_assigns_field(&clause.body, field))
                || statement
                    .finally
                    .as_ref()
                    .is_some_and(|block| block_assigns_field(block, field))
        }
        Stmt::Synchronized(statement) => stmt_assigns_field(&statement.body, field),
        Stmt::Switch(statement) => statement
            .cases
            .iter()
            .any(|case| case.body.iter().any(|stmt| stmt_assigns_field(stmt, field))),
        Stmt::Return(statement) => statement
            .value
            .as_ref()
            .is_some_and(|expr| expr_assigns_field(expr, field)),
        _ => false,
    }
}

fn expr_assigns_field(expr: &Expr, field: &str) -> bool {
    match &expr.kind {
        ExprKind::Assign { target, .. } => {
            matches!(&target.kind, ExprKind::Variable(name) if name == field)
                || matches!(
                    &target.kind,
                    ExprKind::Property {
                        receiver,
                        name,
                        ..
                    } if name == field && matches!(receiver.kind, ExprKind::This)
                )
        }
        ExprKind::Binary { left, right, .. } => {
            expr_assigns_field(left, field) || expr_assigns_field(right, field)
        }
        ExprKind::Unary { operand, .. } => expr_assigns_field(operand, field),
        ExprKind::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            expr_assigns_field(condition, field)
                || expr_assigns_field(then_expr, field)
                || expr_assigns_field(else_expr, field)
        }
        _ => false,
    }
}

fn property_name(method: &str) -> Option<String> {
    let rest = method
        .strip_prefix("get")
        .or_else(|| method.strip_prefix("is"))?;
    let mut chars = rest.chars();
    let first = chars.next()?.to_lowercase().to_string();
    Some(format!("{first}{}", chars.as_str()))
}

fn is_pure_method(name: &str) -> bool {
    matches!(
        name,
        "get"
            | "getOrDefault"
            | "containsKey"
            | "contains"
            | "size"
            | "length"
            | "isEmpty"
            | "isBlank"
            | "trim"
            | "toString"
            | "equals"
            | "hashCode"
            | "values"
            | "keySet"
            | "entrySet"
            | "first"
            | "head"
            | "max"
            | "min"
            | "find"
    )
}

fn is_primitive_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "boolean" | "byte" | "short" | "int" | "long" | "float" | "double" | "char"
    )
}

fn annotation_detail_nullness(
    annotations: &[groovylint_syntax::ast::Annotation],
) -> Option<Nullness> {
    let names: Vec<String> = annotations
        .iter()
        .map(|annotation| annotation.name.clone())
        .collect();
    annotation_nullness(&names)
}

fn annotation_nullness(annotations: &[String]) -> Option<Nullness> {
    if annotations.iter().any(|annotation| {
        annotation == "Nullable"
            || annotation.ends_with(".Nullable")
            || annotation == "MonotonicNonNull"
            || annotation.ends_with(".MonotonicNonNull")
    }) {
        Some(Nullness::MaybeNull)
    } else if annotations.iter().any(|annotation| {
        matches!(
            annotation.as_str(),
            "NonNull" | "NotNull" | "NotBlank" | "NotEmpty"
        ) || annotation.ends_with(".NonNull")
            || annotation.ends_with(".NotNull")
            || annotation.ends_with(".NotBlank")
            || annotation.ends_with(".NotEmpty")
    }) {
        Some(Nullness::NonNull)
    } else {
        None
    }
}

fn merge_state(
    block: BlockId,
    state: &State,
    incoming: &mut [Option<State>],
    queue: &mut VecDeque<BlockId>,
) {
    if state.unreachable {
        return;
    }
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

fn safe_base_path(expr: &Expr, state: &State) -> Option<AccessPath> {
    match &unparen(expr).kind {
        ExprKind::Property {
            receiver,
            safe: true,
            ..
        }
        | ExprKind::Call {
            receiver: Some(receiver),
            safe: true,
            ..
        } => path_of_in(receiver, state).or_else(|| safe_base_path(receiver, state)),
        ExprKind::Property {
            receiver,
            safe: false,
            ..
        } => safe_base_path(receiver, state),
        _ => None,
    }
}

fn local_type<'a>(expr: &Expr, state: &'a State) -> Option<&'a str> {
    let ExprKind::Variable(name) = &unparen(expr).kind else {
        return None;
    };
    state.local_types.get(name).map(String::as_str)
}

fn key_id(expr: &Expr) -> Option<String> {
    match &unparen(expr).kind {
        ExprKind::Literal(Literal::String(value) | Literal::Number(value)) => {
            Some(value.trim_matches('"').trim_matches('\'').to_string())
        }
        ExprKind::Variable(name) => Some(format!("${name}")),
        _ => None,
    }
}

fn path_of_in(expr: &Expr, state: &State) -> Option<AccessPath> {
    match &unparen(expr).kind {
        ExprKind::Variable(name) => {
            if state.locals.contains(name) {
                Some(AccessPath::Local(name.clone()))
            } else if state.fields.contains(name) {
                Some(AccessPath::field(AccessPath::This, name.clone()))
            } else {
                Some(AccessPath::Local(name.clone()))
            }
        }
        ExprKind::This => Some(AccessPath::This),
        ExprKind::Call {
            receiver: Some(receiver),
            name,
            args,
            ..
        } if name == "get" && args.len() == 1 => {
            let key = key_id(&args[0])?;
            path_of_in(receiver, state).map(|base| AccessPath::Lookup(Box::new(base), key))
        }
        ExprKind::Index {
            target,
            index,
            safe: false,
        } => {
            let key = key_id(index)?;
            path_of_in(target, state).map(|base| AccessPath::Lookup(Box::new(base), key))
        }
        ExprKind::Property {
            receiver,
            name,
            safe: false,
        } => path_of_in(receiver, state).map(|base| AccessPath::field(base, name.clone())),
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
    @Nullable String value
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

#[cfg(test)]
mod contract_tests {
    use super::{Mode, analyze, analyze_with_contracts};
    use crate::contracts::ContractRegistry;
    use groovylint_syntax::parser::parse;

    #[test]
    fn nullable_annotations_and_contracts_are_applied() {
        let source = r"
class Example {
    @Nullable String maybe() { return null }
    @NonNull String bad() { return null }
    int deref(@Nullable String value) { return value.length() }
    void take(@NonNull String value) { value.length() }
    void caller() { take(null) }
}
";
        let parsed = parse(source).expect("source parses");
        let findings = analyze(&parsed.script, Mode::Inferred);
        let codes: Vec<_> = findings.iter().map(|finding| finding.code).collect();
        assert!(codes.contains(&"NullReturn"));
        assert!(codes.contains(&"NullArgument"));
        assert!(codes.contains(&"PossibleNullDereference"));
    }

    #[test]
    fn toml_contracts_use_the_same_domain() {
        let directory = tempfile::TempDir::new().expect("tempdir");
        let path = directory.path().join("contracts.toml");
        std::fs::write(
            &path,
            r#"
[[contracts]]
method = "maybe"
arity = 0
returns = "nullable"
origin = "test"
"#,
        )
        .expect("write contracts");
        let source = r"
def maybe() { 'not null in the body' }
def run() { return maybe().length() }
";
        let parsed = parse(source).expect("source parses");
        let mut registry = ContractRegistry::new();
        registry.extend(ContractRegistry::load_toml(&path).expect("load contracts"));
        let findings = analyze_with_contracts(&parsed.script, Mode::Inferred, registry);
        assert!(
            findings
                .iter()
                .any(|finding| finding.code == "PossibleNullDereference")
        );
    }
}
