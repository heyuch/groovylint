//! Single shared AST traversal.
//!
//! Rules do not implement their own tree walks. The engine dispatches enter and
//! exit events from this traversal and filters by each rule's [`NodeMask`].
//!
//! [`NodeMask`]: crate::rule::NodeMask

use groovylint_syntax::ast::{
    Block, ClassDecl, Expr, ExprKind, FieldDecl, InitializerDecl, Item, MethodDecl, Script, Stmt,
};

use crate::rule::NodeRef;

/// Receives AST enter/exit events.
pub trait AstVisitor {
    /// Called before children are visited.
    fn enter(&mut self, node: NodeRef<'_>);

    /// Called after children are visited.
    fn exit(&mut self, node: NodeRef<'_>);
}

/// Walks a whole script.
pub fn walk_script(script: &Script, visitor: &mut impl AstVisitor) {
    let root = NodeRef::Script(script);
    visitor.enter(root);
    for item in &script.items {
        walk_item(item, visitor);
    }
    visitor.exit(root);
}

fn walk_item(item: &Item, visitor: &mut impl AstVisitor) {
    match item {
        Item::Class(class) => walk_class(class, visitor),
        Item::Method(method) => walk_method(method, visitor),
        Item::Field(field) => walk_field(field, visitor),
        Item::Initializer(initializer) => walk_initializer(initializer, visitor),
        Item::Statement(stmt) => walk_stmt(stmt, visitor),
    }
}

fn walk_class(class: &ClassDecl, visitor: &mut impl AstVisitor) {
    let node = NodeRef::Class(class);
    visitor.enter(node);
    for member in &class.members {
        walk_item(member, visitor);
    }
    visitor.exit(node);
}

fn walk_method(method: &MethodDecl, visitor: &mut impl AstVisitor) {
    let node = NodeRef::Method(method);
    visitor.enter(node);
    if let Some(body) = &method.body {
        walk_block(body, visitor);
    }
    visitor.exit(node);
}

fn walk_field(field: &FieldDecl, visitor: &mut impl AstVisitor) {
    let node = NodeRef::Field(field);
    visitor.enter(node);
    if let Some(init) = &field.init {
        walk_expr(init, visitor);
    }
    visitor.exit(node);
}

fn walk_initializer(initializer: &InitializerDecl, visitor: &mut impl AstVisitor) {
    let node = NodeRef::Initializer(initializer);
    visitor.enter(node);
    walk_block(&initializer.body, visitor);
    visitor.exit(node);
}

fn walk_block(block: &Block, visitor: &mut impl AstVisitor) {
    let node = NodeRef::Block(block);
    visitor.enter(node);
    for stmt in &block.statements {
        walk_stmt(stmt, visitor);
    }
    visitor.exit(node);
}

fn walk_stmt(stmt: &Stmt, visitor: &mut impl AstVisitor) {
    match stmt {
        Stmt::Block(block) => walk_block(block, visitor),
        Stmt::VarDecl(decl) => {
            let node = NodeRef::Stmt(stmt);
            visitor.enter(node);
            if let Some(init) = &decl.init {
                walk_expr(init, visitor);
            }
            visitor.exit(node);
        }
        Stmt::Expr(expr) => {
            let node = NodeRef::Stmt(stmt);
            visitor.enter(node);
            walk_expr(expr, visitor);
            visitor.exit(node);
        }
        Stmt::If(if_stmt) => {
            let node = NodeRef::Stmt(stmt);
            visitor.enter(node);
            walk_expr(&if_stmt.condition, visitor);
            walk_stmt(&if_stmt.then_branch, visitor);
            if let Some(else_branch) = &if_stmt.else_branch {
                walk_stmt(else_branch, visitor);
            }
            visitor.exit(node);
        }
        Stmt::While(while_stmt) => {
            let node = NodeRef::Stmt(stmt);
            visitor.enter(node);
            walk_expr(&while_stmt.condition, visitor);
            walk_stmt(&while_stmt.body, visitor);
            visitor.exit(node);
        }
        Stmt::For(for_stmt) => {
            let node = NodeRef::Stmt(stmt);
            visitor.enter(node);
            if let Some(init) = &for_stmt.init {
                walk_stmt(init, visitor);
            }
            if let Some(condition) = &for_stmt.condition {
                walk_expr(condition, visitor);
            }
            if let Some(update) = &for_stmt.update {
                walk_expr(update, visitor);
            }
            walk_stmt(&for_stmt.body, visitor);
            visitor.exit(node);
        }
        Stmt::ForEach(for_each) => {
            let node = NodeRef::Stmt(stmt);
            visitor.enter(node);
            walk_expr(&for_each.iterable, visitor);
            walk_stmt(&for_each.body, visitor);
            visitor.exit(node);
        }
        Stmt::Try(try_stmt) => {
            let node = NodeRef::Stmt(stmt);
            visitor.enter(node);
            walk_block(&try_stmt.body, visitor);
            for catch in &try_stmt.catches {
                walk_block(&catch.body, visitor);
            }
            if let Some(finally) = &try_stmt.finally {
                let finally_node = NodeRef::Finally(finally);
                visitor.enter(finally_node);
                walk_block(finally, visitor);
                visitor.exit(finally_node);
            }
            visitor.exit(node);
        }
        Stmt::Synchronized(synchronized) => {
            let node = NodeRef::Stmt(stmt);
            visitor.enter(node);
            walk_expr(&synchronized.expression, visitor);
            walk_stmt(&synchronized.body, visitor);
            visitor.exit(node);
        }
        Stmt::Switch(switch_stmt) => {
            let node = NodeRef::Stmt(stmt);
            visitor.enter(node);
            walk_expr(&switch_stmt.expression, visitor);
            for case in &switch_stmt.cases {
                if let Some(label) = &case.label {
                    walk_expr(label, visitor);
                }
                for child in &case.body {
                    walk_stmt(child, visitor);
                }
            }
            visitor.exit(node);
        }
        Stmt::Return(return_stmt) => {
            let node = NodeRef::Stmt(stmt);
            visitor.enter(node);
            if let Some(value) = &return_stmt.value {
                walk_expr(value, visitor);
            }
            visitor.exit(node);
        }
        Stmt::Throw(throw_stmt) => {
            let node = NodeRef::Stmt(stmt);
            visitor.enter(node);
            walk_expr(&throw_stmt.value, visitor);
            visitor.exit(node);
        }
        Stmt::Assert(assert_stmt) => {
            let node = NodeRef::Stmt(stmt);
            visitor.enter(node);
            walk_expr(&assert_stmt.condition, visitor);
            if let Some(message) = &assert_stmt.message {
                walk_expr(message, visitor);
            }
            visitor.exit(node);
        }
        Stmt::Break(_) | Stmt::Continue(_) | Stmt::Empty(_) => {
            let node = NodeRef::Stmt(stmt);
            visitor.enter(node);
            visitor.exit(node);
        }
    }
}

fn walk_expr(expr: &Expr, visitor: &mut impl AstVisitor) {
    let node = NodeRef::Expr(expr);
    visitor.enter(node);

    match &expr.kind {
        ExprKind::Property { receiver, .. } => walk_expr(receiver, visitor),
        ExprKind::Call { receiver, args, .. } => {
            if let Some(receiver) = receiver {
                walk_expr(receiver, visitor);
            }
            for arg in args {
                walk_expr(arg, visitor);
            }
        }
        ExprKind::New { args, body, .. } => {
            for arg in args {
                walk_expr(arg, visitor);
            }
            if let Some(body) = body {
                for item in body {
                    walk_item(item, visitor);
                }
            }
        }
        ExprKind::Binary { left, right, .. } => {
            walk_expr(left, visitor);
            walk_expr(right, visitor);
        }
        ExprKind::Unary { operand, .. } => walk_expr(operand, visitor),
        ExprKind::Assign { target, value, .. } => {
            walk_expr(target, visitor);
            walk_expr(value, visitor);
        }
        ExprKind::Ternary {
            condition,
            then_expr,
            else_expr,
        } => {
            walk_expr(condition, visitor);
            walk_expr(then_expr, visitor);
            walk_expr(else_expr, visitor);
        }
        ExprKind::List(items) => {
            for item in items {
                walk_expr(item, visitor);
            }
        }
        ExprKind::Map(entries) => {
            for (key, value) in entries {
                walk_expr(key, visitor);
                walk_expr(value, visitor);
            }
        }
        ExprKind::Index { target, index, .. } => {
            walk_expr(target, visitor);
            walk_expr(index, visitor);
        }
        ExprKind::Closure { body, .. } => walk_block(body, visitor),
        ExprKind::Cast { expr, .. } => walk_expr(expr, visitor),
        ExprKind::NamedArg { value, .. } => walk_expr(value, visitor),
        ExprKind::Paren(inner) => walk_expr(inner, visitor),
        ExprKind::Literal(_) | ExprKind::Variable(_) | ExprKind::This | ExprKind::Super => {}
    }

    visitor.exit(node);
}
