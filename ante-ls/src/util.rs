use ante::{error::location::Locatable, parser::ast::Ast};
use ropey::Rope;
use tower_lsp::lsp_types::*;

pub fn node_at_index<'a>(ast: &'a Ast<'a>, idx: usize) -> &'a Ast<'a> {
    let mut ast = ast;
    loop {
        match ast {
            Ast::Assignment(a) => {
                if a.lhs.locate().contains_index(&idx) {
                    ast = &a.lhs;
                } else if a.rhs.locate().contains_index(&idx) {
                    ast = &a.rhs;
                } else {
                    break;
                }
            },
            Ast::Definition(d) => {
                if d.pattern.locate().contains_index(&idx) {
                    ast = &d.pattern;
                } else if d.expr.locate().contains_index(&idx) {
                    ast = &d.expr;
                } else {
                    break;
                }
            },
            Ast::EffectDefinition(_) => {
                break;
            },
            Ast::Extern(_) => {
                break;
            },
            Ast::FunctionCall(f) => {
                if let Some(arg) = f.args.iter().find(|&arg| arg.locate().contains_index(&idx)) {
                    ast = arg;
                } else if f.function.locate().contains_index(&idx) {
                    ast = &f.function;
                } else {
                    break;
                }
            },
            Ast::Handle(h) => {
                if let Some(branch) = h.branches.iter().find_map(|(pat, body)| {
                    if pat.locate().contains_index(&idx) {
                        return Some(pat);
                    };
                    if body.locate().contains_index(&idx) {
                        return Some(body);
                    };
                    None
                }) {
                    ast = branch;
                } else if h.expression.locate().contains_index(&idx) {
                    ast = &h.expression;
                } else {
                    break;
                }
            },
            Ast::If(i) => {
                if i.condition.locate().contains_index(&idx) {
                    ast = &i.condition;
                } else if i.then.locate().contains_index(&idx) {
                    ast = &i.then;
                } else if i.otherwise.locate().contains_index(&idx) {
                    ast = &i.otherwise;
                } else {
                    break;
                }
            },
            Ast::Import(_) => {
                break;
            },
            Ast::Lambda(l) => {
                if let Some(arg) = l.args.iter().find(|&arg| arg.locate().contains_index(&idx)) {
                    ast = arg;
                } else if l.body.locate().contains_index(&idx) {
                    ast = &l.body;
                } else {
                    break;
                }
            },
            Ast::Literal(_) => {
                break;
            },
            Ast::Match(m) => {
                if let Some(branch) = m.branches.iter().find_map(|(pat, body)| {
                    if pat.locate().contains_index(&idx) {
                        return Some(pat);
                    };
                    if body.locate().contains_index(&idx) {
                        return Some(body);
                    };
                    None
                }) {
                    ast = branch;
                } else {
                    break;
                }
            },
            Ast::MemberAccess(m) => {
                if m.lhs.locate().contains_index(&idx) {
                    ast = &m.lhs;
                } else {
                    break;
                }
            },
            Ast::NamedConstructor(n) => {
                if let Some((_, arg)) = n.args.iter().find(|(_, arg)| arg.locate().contains_index(&idx)) {
                    ast = arg;
                } else if n.constructor.locate().contains_index(&idx) {
                    ast = &n.constructor;
                } else {
                    break;
                }
            },
            Ast::Return(r) => {
                if r.expression.locate().contains_index(&idx) {
                    ast = &r.expression;
                } else {
                    break;
                }
            },
            Ast::Sequence(s) => {
                if let Some(stmt) = s.statements.iter().find(|&stmt| stmt.locate().contains_index(&idx)) {
                    ast = stmt;
                } else {
                    break;
                }
            },
            Ast::TraitDefinition(_) => {
                break;
            },
            Ast::TraitImpl(t) => {
                if let Some(def) = t.definitions.iter().find_map(|def| {
                    if def.pattern.locate().contains_index(&idx) {
                        return Some(&def.pattern);
                    };
                    if def.expr.locate().contains_index(&idx) {
                        return Some(&def.expr);
                    };
                    None
                }) {
                    ast = def;
                } else {
                    break;
                }
            },
            Ast::TypeAnnotation(t) => {
                if t.lhs.locate().contains_index(&idx) {
                    ast = &t.lhs;
                } else {
                    break;
                }
            },
            Ast::TypeDefinition(_) => {
                break;
            },
            Ast::Variable(_) => {
                break;
            },
        }
    }
    ast
}

pub fn position_to_index(position: Position, rope: &Rope) -> Result<usize, ropey::Error> {
    let line = position.line as usize;
    let line = rope.try_line_to_char(line)?;
    Ok(line + position.character as usize)
}

pub fn index_to_position(index: usize, rope: &Rope) -> Result<Position, ropey::Error> {
    let line = rope.try_char_to_line(index)?;
    let char = index - rope.line_to_char(line);
    Ok(Position { line: line as u32, character: char as u32 })
}

pub fn lsp_range_to_rope_range(range: Range, rope: &Rope) -> Result<std::ops::Range<usize>, ropey::Error> {
    let start = position_to_index(range.start, rope)?;
    let end = position_to_index(range.end, rope)?;
    Ok(start..end)
}

pub fn rope_range_to_lsp_range(range: std::ops::Range<usize>, rope: &Rope) -> Result<Range, ropey::Error> {
    let start = index_to_position(range.start, rope)?;
    let end = index_to_position(range.end, rope)?;
    Ok(Range { start, end })
}
