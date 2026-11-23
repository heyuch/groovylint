//! A compact AST for the supported Groovy subset.
//!
//! The AST intentionally keeps source spans on every node and preserves
//! statement structure. Expressions are represented closely enough for
//! syntax-level rules; semantic information is added by later analysis layers.

use groovylint_core::Span;

/// A parsed Groovy script, class file, or Gradle build file.
#[derive(Clone, Debug)]
pub struct Script {
    /// Optional package declaration.
    pub package: Option<PackageDecl>,
    /// Import declarations.
    pub imports: Vec<ImportDecl>,
    /// Top-level classes, methods, fields, and statements.
    pub items: Vec<Item>,
    /// Whole-file span.
    pub span: Span,
}

/// A package declaration.
#[derive(Clone, Debug)]
pub struct PackageDecl {
    /// Package name.
    pub name: String,
    /// Source span.
    pub span: Span,
}

/// An import declaration.
#[derive(Clone, Debug)]
pub struct ImportDecl {
    /// Imported name or wildcard pattern.
    pub path: String,
    /// Optional alias.
    pub alias: Option<String>,
    /// Source span.
    pub span: Span,
}

/// A top-level or class-member item.
#[derive(Clone, Debug)]
pub enum Item {
    /// Class, interface, trait, enum, or record declaration.
    Class(ClassDecl),
    /// Method or constructor declaration.
    Method(MethodDecl),
    /// Field declaration.
    Field(FieldDecl),
    /// Static or instance initializer block.
    Initializer(InitializerDecl),
    /// Statement at script scope.
    Statement(Stmt),
}

/// The kind of a type declaration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClassKind {
    /// `class`
    Class,
    /// `interface`
    Interface,
    /// `trait`
    Trait,
    /// `enum`
    Enum,
    /// `record`
    Record,
}

/// A class-like declaration.
#[derive(Clone, Debug)]
pub struct ClassDecl {
    /// Type name.
    pub name: String,
    /// Declaration kind.
    pub kind: ClassKind,
    /// Whether the class is abstract.
    pub is_abstract: bool,
    /// Simple or qualified superclass name, if explicitly declared.
    pub extends: Option<String>,
    /// Annotation names, normalized to simple names.
    pub annotations: Vec<String>,
    /// Members.
    pub members: Vec<Item>,
    /// Source span.
    pub span: Span,
}

/// A method or constructor declaration.
#[derive(Clone, Debug)]
pub struct MethodDecl {
    /// Method name.
    pub name: String,
    /// Return type, if explicitly declared.
    pub return_type: Option<String>,
    /// Parameters.
    pub params: Vec<Param>,
    /// Annotation names, normalized to simple names.
    pub annotations: Vec<String>,
    /// Body, if present.
    pub body: Option<Block>,
    /// Source span.
    pub span: Span,
}

/// A method or closure parameter.
#[derive(Clone, Debug)]
pub struct Param {
    /// Parameter name.
    pub name: String,
    /// Parameter type, if explicitly declared.
    pub type_name: Option<String>,
    /// Annotation names, normalized to simple names.
    pub annotations: Vec<String>,
    /// Source span.
    pub span: Span,
}

/// A static or instance initializer block.
#[derive(Clone, Debug)]
pub struct InitializerDecl {
    /// Whether this is a `static { ... }` block.
    pub is_static: bool,
    /// Initializer body.
    pub body: Block,
    /// Source span.
    pub span: Span,
}

/// A field declaration.
#[derive(Clone, Debug)]
pub struct FieldDecl {
    /// Field name.
    pub name: String,
    /// Optional initializer.
    pub init: Option<Expr>,
    /// Source span.
    pub span: Span,
}

/// A block statement.
#[derive(Clone, Debug)]
pub struct Block {
    /// Statements in source order.
    pub statements: Vec<Stmt>,
    /// Span including braces.
    pub span: Span,
}

/// A statement.
#[derive(Clone, Debug)]
pub enum Stmt {
    /// Braced block.
    Block(Block),
    /// Local variable declaration.
    VarDecl(VarDecl),
    /// Expression statement.
    Expr(Expr),
    /// `if`/`else`.
    If(IfStmt),
    /// `while` loop.
    While(WhileStmt),
    /// Classic `for` loop.
    For(ForStmt),
    /// `for (x in y)` loop.
    ForEach(ForEachStmt),
    /// `try`/`catch`/`finally`.
    Try(TryStmt),
    /// `synchronized` statement.
    Synchronized(SynchronizedStmt),
    /// `switch` statement.
    Switch(SwitchStmt),
    /// `return`.
    Return(ReturnStmt),
    /// `throw`.
    Throw(ThrowStmt),
    /// `break`.
    Break(Span),
    /// `continue`.
    Continue(Span),
    /// `assert`.
    Assert(AssertStmt),
    /// Empty statement represented by a semicolon.
    Empty(Span),
}

/// A local variable declaration.
#[derive(Clone, Debug)]
pub struct VarDecl {
    /// Variable name.
    pub name: String,
    /// Optional initializer.
    pub init: Option<Expr>,
    /// Source span.
    pub span: Span,
}

/// An `if` statement.
#[derive(Clone, Debug)]
pub struct IfStmt {
    /// Condition expression.
    pub condition: Expr,
    /// Then branch.
    pub then_branch: Box<Stmt>,
    /// Optional else branch.
    pub else_branch: Option<Box<Stmt>>,
    /// Span of the `else` keyword and branch.
    pub else_span: Option<Span>,
    /// Whole-statement span.
    pub span: Span,
}

/// A `while` statement.
#[derive(Clone, Debug)]
pub struct WhileStmt {
    /// Condition expression.
    pub condition: Expr,
    /// Loop body.
    pub body: Box<Stmt>,
    /// Whole-statement span.
    pub span: Span,
}

/// A classic `for (init; condition; update)` statement.
#[derive(Clone, Debug)]
pub struct ForStmt {
    /// Initializer.
    pub init: Option<Box<Stmt>>,
    /// Condition.
    pub condition: Option<Expr>,
    /// Update expression.
    pub update: Option<Expr>,
    /// Loop body.
    pub body: Box<Stmt>,
    /// Whole-statement span.
    pub span: Span,
}

/// A `for (x in iterable)` statement.
#[derive(Clone, Debug)]
pub struct ForEachStmt {
    /// Loop variable name.
    pub var_name: String,
    /// Iterated expression.
    pub iterable: Expr,
    /// Loop body.
    pub body: Box<Stmt>,
    /// Whole-statement span.
    pub span: Span,
}

/// A `try` statement.
#[derive(Clone, Debug)]
pub struct TryStmt {
    /// Try body.
    pub body: Block,
    /// Catch clauses.
    pub catches: Vec<CatchClause>,
    /// Optional finally block.
    pub finally: Option<Block>,
    /// Span of the `finally` keyword and block.
    pub finally_span: Option<Span>,
    /// Whole-statement span.
    pub span: Span,
}

/// A catch clause.
#[derive(Clone, Debug)]
pub struct CatchClause {
    /// Catch body.
    pub body: Block,
    /// Span including `catch (...)`.
    pub span: Span,
}

/// A `synchronized` statement.
#[derive(Clone, Debug)]
pub struct SynchronizedStmt {
    /// Synchronized expression.
    pub expression: Expr,
    /// Guarded statement.
    pub body: Box<Stmt>,
    /// Whole-statement span.
    pub span: Span,
}

/// A `switch` statement.
#[derive(Clone, Debug)]
pub struct SwitchStmt {
    /// Switched expression.
    pub expression: Expr,
    /// Case clauses in source order.
    pub cases: Vec<SwitchCase>,
    /// Whole-statement span.
    pub span: Span,
}

/// One `case` or `default` clause.
#[derive(Clone, Debug)]
pub struct SwitchCase {
    /// Case label expression, or `None` for `default`.
    pub label: Option<Expr>,
    /// Statements in the clause.
    pub body: Vec<Stmt>,
    /// Span including the label and body.
    pub span: Span,
}

/// A `return` statement.
#[derive(Clone, Debug)]
pub struct ReturnStmt {
    /// Optional returned expression.
    pub value: Option<Expr>,
    /// Source span.
    pub span: Span,
}

/// A `throw` statement.
#[derive(Clone, Debug)]
pub struct ThrowStmt {
    /// Thrown expression.
    pub value: Expr,
    /// Source span.
    pub span: Span,
}

/// An `assert` statement.
#[derive(Clone, Debug)]
pub struct AssertStmt {
    /// Asserted condition.
    pub condition: Expr,
    /// Optional message expression.
    pub message: Option<Expr>,
    /// Source span.
    pub span: Span,
}

/// An expression with a source span.
#[derive(Clone, Debug)]
pub struct Expr {
    /// Expression kind.
    pub kind: ExprKind,
    /// Source span.
    pub span: Span,
}

/// Expression variants.
#[derive(Clone, Debug)]
pub enum ExprKind {
    /// Literal value.
    Literal(Literal),
    /// Variable reference.
    Variable(String),
    /// `this`.
    This,
    /// `super`.
    Super,
    /// Property access (`receiver.name` or `receiver?.name`).
    Property {
        /// Receiver expression.
        receiver: Box<Expr>,
        /// Property name.
        name: String,
        /// Whether the access is safe (`?.`).
        safe: bool,
    },
    /// Method call.
    Call {
        /// Optional receiver.
        receiver: Option<Box<Expr>>,
        /// Method name.
        name: String,
        /// Positional arguments.
        args: Vec<Expr>,
        /// Whether the call is safe (`?.`).
        safe: bool,
    },
    /// Object construction.
    New {
        /// Constructed type name.
        type_name: String,
        /// Constructor arguments.
        args: Vec<Expr>,
    },
    /// Binary expression.
    Binary {
        /// Operator.
        op: BinaryOp,
        /// Left operand.
        left: Box<Expr>,
        /// Right operand.
        right: Box<Expr>,
    },
    /// Unary expression.
    Unary {
        /// Operator.
        op: UnaryOp,
        /// Operand.
        operand: Box<Expr>,
    },
    /// Assignment expression.
    Assign {
        /// Assignment target.
        target: Box<Expr>,
        /// Assignment operator.
        op: AssignOp,
        /// Assigned value.
        value: Box<Expr>,
    },
    /// Ternary expression.
    Ternary {
        /// Condition.
        condition: Box<Expr>,
        /// Then expression.
        then_expr: Box<Expr>,
        /// Else expression.
        else_expr: Box<Expr>,
    },
    /// List literal.
    List(Vec<Expr>),
    /// Map literal.
    Map(Vec<(Expr, Expr)>),
    /// Index expression.
    Index {
        /// Indexed expression.
        target: Box<Expr>,
        /// Index expression.
        index: Box<Expr>,
        /// Whether the index access is safe (`?[`).
        safe: bool,
    },
    /// Closure expression.
    Closure {
        /// Explicit parameters. Empty means implicit `it`.
        params: Vec<Param>,
        /// Closure body.
        body: Block,
    },
    /// Type coercion (`expr as Type`).
    Cast {
        /// Coerced expression.
        expr: Box<Expr>,
        /// Target type.
        type_name: String,
    },
    /// Parenthesized expression.
    Paren(Box<Expr>),
}

/// Literal values.
#[derive(Clone, Debug)]
pub enum Literal {
    /// Numeric literal source text.
    Number(String),
    /// String literal source text.
    String(String),
    /// Boolean literal.
    Boolean(bool),
    /// Null literal.
    Null,
}

/// Binary operators.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Mod,
    /// `**`
    Power,
    /// `==`
    Eq,
    /// `!=`
    NotEq,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `&&`
    And,
    /// `||`
    Or,
    /// `&`
    BitAnd,
    /// `|`
    BitOr,
    /// `^`
    BitXor,
    /// `<<`
    ShiftLeft,
    /// `>>`
    ShiftRight,
    /// `..`
    Range,
    /// `..<`
    RangeExclusive,
    /// `instanceof`
    InstanceOf,
}

/// Unary operators.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOp {
    /// `!`
    Not,
    /// Unary `-`
    Neg,
    /// Unary `+`
    Pos,
    /// `~`
    BitNot,
    /// Prefix `++`
    PreInc,
    /// Prefix `--`
    PreDec,
    /// Postfix `++`
    PostInc,
    /// Postfix `--`
    PostDec,
}

/// Assignment operators.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssignOp {
    /// `=`
    Assign,
    /// `+=`
    Add,
    /// `-=`
    Sub,
    /// `*=`
    Mul,
    /// `/=`
    Div,
    /// `%=`
    Mod,
}

impl Stmt {
    /// Returns the statement span.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Block(block) => block.span,
            Self::VarDecl(decl) => decl.span,
            Self::Expr(expr) => expr.span,
            Self::If(if_stmt) => if_stmt.span,
            Self::While(while_stmt) => while_stmt.span,
            Self::For(for_stmt) => for_stmt.span,
            Self::ForEach(for_each) => for_each.span,
            Self::Try(try_stmt) => try_stmt.span,
            Self::Synchronized(synchronized) => synchronized.span,
            Self::Switch(switch) => switch.span,
            Self::Return(return_stmt) => return_stmt.span,
            Self::Throw(throw_stmt) => throw_stmt.span,
            Self::Break(span) | Self::Continue(span) | Self::Empty(span) => *span,
            Self::Assert(assert_stmt) => assert_stmt.span,
        }
    }

    /// Returns whether this is an empty braced block.
    #[must_use]
    pub fn is_empty_block(&self) -> bool {
        matches!(self, Self::Block(block) if block.statements.is_empty())
    }
}

impl Expr {
    /// Returns the expression span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}
