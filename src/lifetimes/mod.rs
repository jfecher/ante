use crate::parser::ast::Ast;
use crate::types::TypeVariableId;
use crate::cache::ModuleCache;

/// A lifetime variable is represented simply as a type variable for ease of unification
/// during the type inference pass.
pub type LifetimeVariableId = TypeVariableId;

// struct LifetimeAnalyzer {
//     pub level: StackFrameIndex,
// 
//     /// Map from RegionVariableId -> StackFrame
//     /// Contains the stack frame index each region should be allocated in
//     pub lifetimes: Vec<StackFrameIndex>,
// }
// 
// struct StackFrameIndex(usize);

pub fn infer<'c>(_ast: &mut Ast<'c>, _cache: &mut ModuleCache<'c>) {

}

// trait InferableLifetime {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>);
// }
// 
// impl<'ast> InferableLifetime for Ast<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
//         dispatch_on_expr!(self, InferableLifetime::infer_lifetime, analyzer, cache)
//     }
// }
// 
// impl<'ast> InferableLifetime for ast::Literal<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
//         // Do nothing: literals cannot contain a ref type
//     }
// }
// 
// impl<'ast> InferableLifetime for ast::Variable<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
// 
//     }
// }
// 
// impl<'ast> InferableLifetime for ast::Lambda<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
// 
//     }
// }
// 
// impl<'ast> InferableLifetime for ast::FunctionCall<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
// 
//     }
// }
// 
// impl<'ast> InferableLifetime for ast::Definition<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
// 
//     }
// }
// 
// impl<'ast> InferableLifetime for ast::If<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
// 
//     }
// }
// 
// impl<'ast> InferableLifetime for ast::Match<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
// 
//     }
// }
// 
// impl<'ast> InferableLifetime for ast::TypeDefinition<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
// 
//     }
// }
// 
// impl<'ast> InferableLifetime for ast::TypeAnnotation<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
// 
//     }
// }
// 
// impl<'ast> InferableLifetime for ast::Import<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
// 
//     }
// }
// 
// impl<'ast> InferableLifetime for ast::TraitDefinition<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
// 
//     }
// }
// 
// impl<'ast> InferableLifetime for ast::TraitImpl<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
// 
//     }
// }
// 
// impl<'ast> InferableLifetime for ast::Return<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
// 
//     }
// }
// 
// impl<'ast> InferableLifetime for ast::Sequence<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
// 
//     }
// }
// 
// impl<'ast> InferableLifetime for ast::Extern<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
// 
//     }
// }
// 
// impl<'ast> InferableLifetime for ast::MemberAccess<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
// 
//     }
// }
// 
// impl<'ast> InferableLifetime for ast::Tuple<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
// 
//     }
// }
// 
// impl<'ast> InferableLifetime for ast::Assignment<'ast> {
//     fn infer_lifetime<'c>(&mut self, analyzer: &mut LifetimeAnalyzer, cache: &mut ModuleCache<'c>) {
// 
//     }
// }
