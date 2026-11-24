//! Control-flow graph construction for nullness analysis.
//!
//! The graph is intentionally small and analysis-agnostic. Expressions are
//! stored by value so a CFG can outlive the AST borrow; spans remain available
//! for diagnostics.

use groovylint_syntax::ast::{Annotation, Block, Expr, Item, MethodDecl, Script, Stmt};

/// A basic-block identifier.
pub type BlockId = usize;

/// A statement-like operation evaluated inside a block.
#[derive(Clone, Debug)]
pub enum Instruction {
    /// Evaluate an expression for its effects and discard its value.
    Eval(Expr),
    /// Declare a local variable and assign its initializer.
    Declare {
        name: String,
        init: Option<Expr>,
        annotations: Vec<Annotation>,
        type_name: Option<String>,
        /// For a for-each variable, the iterated expression.
        iterable: Option<Expr>,
    },
    /// Evaluate an assertion condition and continue on the true path.
    Assert {
        condition: Expr,
        message: Option<Expr>,
    },
}

/// How control leaves a basic block.
#[derive(Clone, Debug)]
pub enum Terminator {
    /// Unconditional jump.
    Goto(BlockId),
    /// Two-way branch on a boolean expression.
    Branch {
        condition: Expr,
        then_block: BlockId,
        else_block: BlockId,
    },
    /// Return from the current method/closure.
    Return(Option<Expr>),
    /// Throw an exception.
    Throw(Expr),
    /// Normal method exit.
    Exit,
    /// No terminator has been set yet.
    Pending,
}

/// A basic block.
#[derive(Clone, Debug)]
pub struct BasicBlock {
    /// Block identity, equal to its index in [`Cfg::blocks`].
    pub id: BlockId,
    /// Instructions in evaluation order.
    pub instructions: Vec<Instruction>,
    /// Normal control-flow terminator.
    pub terminator: Terminator,
    /// Additional conservative edges, used for exception handlers/loop entry.
    pub extra_successors: Vec<BlockId>,
}

impl BasicBlock {
    /// Returns the normal successors of this block.
    #[must_use]
    pub fn successors(&self) -> Vec<BlockId> {
        let mut successors = self.extra_successors.clone();
        match &self.terminator {
            Terminator::Goto(target) => successors.push(*target),
            Terminator::Branch {
                then_block,
                else_block,
                ..
            } => {
                successors.push(*then_block);
                successors.push(*else_block);
            }
            Terminator::Return(_)
            | Terminator::Throw(_)
            | Terminator::Exit
            | Terminator::Pending => {}
        }
        successors.sort_unstable();
        successors.dedup();
        successors
    }
}

/// A control-flow graph for one executable unit.
#[derive(Clone, Debug)]
pub struct Cfg {
    /// All blocks. Block ids are indices into this vector.
    pub blocks: Vec<BasicBlock>,
    /// Entry block.
    pub entry: BlockId,
}

impl Cfg {
    /// Creates an empty CFG with one entry block.
    #[must_use]
    pub fn new() -> Self {
        Self {
            blocks: vec![BasicBlock {
                id: 0,
                instructions: Vec::new(),
                terminator: Terminator::Pending,
                extra_successors: Vec::new(),
            }],
            entry: 0,
        }
    }

    /// Adds a block and returns its id.
    pub fn add_block(&mut self) -> BlockId {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock {
            id,
            instructions: Vec::new(),
            terminator: Terminator::Pending,
            extra_successors: Vec::new(),
        });
        id
    }

    /// Returns a block by id.
    #[must_use]
    pub fn block(&self, id: BlockId) -> &BasicBlock {
        &self.blocks[id]
    }

    /// Returns a mutable block by id.
    pub fn block_mut(&mut self, id: BlockId) -> &mut BasicBlock {
        &mut self.blocks[id]
    }
}

impl Default for Cfg {
    fn default() -> Self {
        Self::new()
    }
}

/// Lowers a method body to a CFG.
#[must_use]
pub fn build_method(method: &MethodDecl) -> Option<Cfg> {
    let body = method.body.as_ref()?;
    let mut builder = Builder::new();
    builder.lower_block(body);
    Some(builder.finish())
}

/// Lowers a closure body to a CFG.
#[must_use]
pub fn build_closure(body: &Block) -> Cfg {
    let mut builder = Builder::new();
    builder.lower_block(body);
    builder.finish()
}

/// Lowers the executable top-level statements of a script to a CFG.
#[must_use]
pub fn build_script(script: &Script) -> Option<Cfg> {
    let mut builder = Builder::new();
    let mut has_statement = false;
    for item in &script.items {
        match item {
            Item::Statement(statement) => {
                has_statement = true;
                builder.lower_stmt(statement);
            }
            Item::Group(items) => {
                for item in items {
                    if let Item::Statement(statement) = item {
                        has_statement = true;
                        builder.lower_stmt(statement);
                    }
                }
            }
            _ => {}
        }
    }
    if !has_statement {
        return None;
    }
    Some(builder.finish())
}

struct Builder {
    cfg: Cfg,
    current: BlockId,
    break_targets: Vec<BlockId>,
    continue_targets: Vec<BlockId>,
}

impl Builder {
    fn new() -> Self {
        Self {
            cfg: Cfg::new(),
            current: 0,
            break_targets: Vec::new(),
            continue_targets: Vec::new(),
        }
    }

    fn finish(mut self) -> Cfg {
        if matches!(self.cfg.block(self.current).terminator, Terminator::Pending) {
            let exit = self.cfg.add_block();
            self.cfg.block_mut(self.current).terminator = Terminator::Goto(exit);
            self.cfg.block_mut(exit).terminator = Terminator::Exit;
        }
        self.cfg
    }

    fn emit(&mut self, instruction: Instruction) {
        self.cfg
            .block_mut(self.current)
            .instructions
            .push(instruction);
    }

    fn terminate(&mut self, terminator: Terminator) {
        self.cfg.block_mut(self.current).terminator = terminator;
    }

    fn goto(&mut self, target: BlockId) {
        self.terminate(Terminator::Goto(target));
    }

    fn lower_block(&mut self, block: &Block) {
        for statement in &block.statements {
            self.lower_stmt(statement);
        }
    }

    fn lower_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Block(block) => self.lower_block(block),
            Stmt::VarDecl(decl) => {
                self.emit(Instruction::Declare {
                    name: decl.name.clone(),
                    init: decl.init.clone(),
                    annotations: decl.annotation_details.clone(),
                    type_name: decl.type_name.clone(),
                    iterable: None,
                });
            }
            Stmt::Expr(expr) => self.emit(Instruction::Eval(expr.clone())),
            Stmt::Assert(assert_stmt) => {
                self.emit(Instruction::Assert {
                    condition: assert_stmt.condition.clone(),
                    message: assert_stmt.message.clone(),
                });
            }
            Stmt::If(if_stmt) => {
                let then_block = self.cfg.add_block();
                let else_block = self.cfg.add_block();
                let join_block = self.cfg.add_block();
                let condition = if_stmt.condition.clone();
                self.terminate(Terminator::Branch {
                    condition,
                    then_block,
                    else_block,
                });
                self.current = then_block;
                self.lower_stmt(&if_stmt.then_branch);
                if !self.is_terminated() {
                    self.goto(join_block);
                }
                self.current = else_block;
                if let Some(else_branch) = &if_stmt.else_branch {
                    self.lower_stmt(else_branch);
                }
                if !self.is_terminated() {
                    self.goto(join_block);
                }
                self.current = join_block;
            }
            Stmt::While(while_stmt) => {
                let condition_block = self.cfg.add_block();
                let body_block = self.cfg.add_block();
                let after_block = self.cfg.add_block();
                self.goto(condition_block);
                self.current = condition_block;
                self.terminate(Terminator::Branch {
                    condition: while_stmt.condition.clone(),
                    then_block: body_block,
                    else_block: after_block,
                });
                self.current = body_block;
                self.break_targets.push(after_block);
                self.continue_targets.push(condition_block);
                self.lower_stmt(&while_stmt.body);
                self.break_targets.pop();
                self.continue_targets.pop();
                if !self.is_terminated() {
                    self.goto(condition_block);
                }
                self.current = after_block;
            }
            Stmt::DoWhile(do_while) => {
                let body_block = self.cfg.add_block();
                let condition_block = self.cfg.add_block();
                let after_block = self.cfg.add_block();
                self.goto(body_block);
                self.current = body_block;
                self.break_targets.push(after_block);
                self.continue_targets.push(condition_block);
                self.lower_stmt(&do_while.body);
                self.break_targets.pop();
                self.continue_targets.pop();
                if !self.is_terminated() {
                    self.goto(condition_block);
                }
                self.current = condition_block;
                self.terminate(Terminator::Branch {
                    condition: do_while.condition.clone(),
                    then_block: body_block,
                    else_block: after_block,
                });
                self.current = after_block;
            }
            Stmt::For(for_stmt) => {
                if let Some(init) = &for_stmt.init {
                    self.lower_stmt(init);
                }
                let condition_block = self.cfg.add_block();
                let body_block = self.cfg.add_block();
                let update_block = self.cfg.add_block();
                let after_block = self.cfg.add_block();
                self.goto(condition_block);
                self.current = condition_block;
                if let Some(condition) = &for_stmt.condition {
                    self.terminate(Terminator::Branch {
                        condition: condition.clone(),
                        then_block: body_block,
                        else_block: after_block,
                    });
                } else {
                    self.goto(body_block);
                }
                self.current = body_block;
                self.break_targets.push(after_block);
                self.continue_targets.push(update_block);
                self.lower_stmt(&for_stmt.body);
                self.break_targets.pop();
                self.continue_targets.pop();
                if !self.is_terminated() {
                    self.goto(update_block);
                }
                self.current = update_block;
                if let Some(update) = &for_stmt.update {
                    self.emit(Instruction::Eval(update.clone()));
                }
                self.goto(condition_block);
                self.current = after_block;
            }
            Stmt::ForEach(for_each) => {
                let condition_block = self.cfg.add_block();
                let body_block = self.cfg.add_block();
                let after_block = self.cfg.add_block();
                self.emit(Instruction::Eval(for_each.iterable.clone()));
                // A for-each may execute zero times. The extra edge carries the
                // pre-loop state to the block after the loop.
                self.cfg
                    .block_mut(self.current)
                    .extra_successors
                    .push(after_block);
                self.goto(condition_block);
                self.current = condition_block;
                self.terminate(Terminator::Branch {
                    condition: Expr {
                        kind: groovylint_syntax::ast::ExprKind::Literal(
                            groovylint_syntax::ast::Literal::Boolean(true),
                        ),
                        span: for_each.span,
                    },
                    then_block: body_block,
                    else_block: after_block,
                });
                self.current = body_block;
                self.emit(Instruction::Declare {
                    name: for_each.var_name.clone(),
                    init: None,
                    annotations: Vec::new(),
                    type_name: None,
                    iterable: Some(for_each.iterable.clone()),
                });
                self.break_targets.push(after_block);
                self.continue_targets.push(condition_block);
                self.lower_stmt(&for_each.body);
                self.break_targets.pop();
                self.continue_targets.pop();
                if !self.is_terminated() {
                    self.goto(condition_block);
                }
                self.current = after_block;
            }
            Stmt::Try(try_stmt) => self.lower_try(try_stmt),
            Stmt::Synchronized(synchronized) => {
                self.emit(Instruction::Eval(synchronized.expression.clone()));
                self.lower_stmt(&synchronized.body);
            }
            Stmt::Switch(switch_stmt) => self.lower_switch(switch_stmt),
            Stmt::Return(return_stmt) => {
                self.terminate(Terminator::Return(return_stmt.value.clone()));
            }
            Stmt::Throw(throw_stmt) => {
                self.terminate(Terminator::Throw(throw_stmt.value.clone()));
            }
            Stmt::Break(_) => {
                if let Some(target) = self.break_targets.last().copied() {
                    self.goto(target);
                }
            }
            Stmt::Continue(_) => {
                if let Some(target) = self.continue_targets.last().copied() {
                    self.goto(target);
                }
            }
            Stmt::Empty(_) => {}
        }
    }

    fn lower_try(&mut self, try_stmt: &groovylint_syntax::ast::TryStmt) {
        let try_body = self.cfg.add_block();
        let after_try = self.cfg.add_block();
        let finally_entry = self.cfg.add_block();
        let finally_exit = if try_stmt.finally.is_some() {
            Some(self.cfg.add_block())
        } else {
            None
        };
        let catch_blocks: Vec<BlockId> = try_stmt
            .catches
            .iter()
            .map(|_| self.cfg.add_block())
            .collect();

        // Conservative exception edges: a catch may observe the state at the
        // start of the try body.
        self.goto(try_body);
        if let Some(first_catch) = catch_blocks.first() {
            self.cfg
                .block_mut(try_body)
                .extra_successors
                .push(*first_catch);
            for catch in catch_blocks.iter().skip(1) {
                self.cfg.block_mut(try_body).extra_successors.push(*catch);
            }
        }

        self.current = try_body;
        self.lower_block(&try_stmt.body);
        if !self.is_terminated() {
            self.goto(if try_stmt.finally.is_some() {
                finally_entry
            } else {
                after_try
            });
        }

        for (catch, block) in try_stmt.catches.iter().zip(&catch_blocks) {
            self.current = *block;
            self.lower_block(&catch.body);
            if !self.is_terminated() {
                self.goto(if try_stmt.finally.is_some() {
                    finally_entry
                } else {
                    after_try
                });
            }
        }

        if let Some(finally) = &try_stmt.finally {
            self.current = finally_entry;
            self.lower_block(finally);
            if !self.is_terminated() {
                self.goto(finally_exit.expect("finally exit exists"));
            }
            self.current = finally_exit.expect("finally exit exists");
            self.goto(after_try);
        }
        self.current = after_try;
    }

    fn lower_switch(&mut self, switch_stmt: &groovylint_syntax::ast::SwitchStmt) {
        let after_block = self.cfg.add_block();
        let case_blocks: Vec<BlockId> = switch_stmt
            .cases
            .iter()
            .map(|_| self.cfg.add_block())
            .collect();
        let default_block = case_blocks.last().copied().unwrap_or(after_block);

        self.emit(Instruction::Eval(switch_stmt.expression.clone()));
        if let Some(first) = case_blocks.first() {
            self.goto(*first);
            for case in case_blocks.iter().skip(1) {
                self.cfg
                    .block_mut(self.current)
                    .extra_successors
                    .push(*case);
            }
        } else {
            self.goto(after_block);
        }
        self.break_targets.push(after_block);
        for (index, (case, block)) in switch_stmt.cases.iter().zip(&case_blocks).enumerate() {
            self.current = *block;
            for statement in &case.body {
                self.lower_stmt(statement);
            }
            if !self.is_terminated() {
                if let Some(next) = case_blocks.get(index + 1) {
                    self.goto(*next);
                } else {
                    self.goto(after_block);
                }
            }
        }
        self.break_targets.pop();
        let _ = default_block;
        self.current = after_block;
    }

    fn is_terminated(&self) -> bool {
        !matches!(self.cfg.block(self.current).terminator, Terminator::Pending)
    }
}
