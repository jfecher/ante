use std::collections::{BTreeMap, HashSet};

use crate::{cache::{DefinitionInfoId, ModuleCache}, parser::ast, types::{typed::Typed, Type}};


impl<'c> ast::Handle<'c> {
    pub fn find_free_variables(&self, cache: &ModuleCache<'c>) -> BTreeMap<DefinitionInfoId, Type> {
        let mut context = Context::new(cache);
        self.find_free_vars(&mut context);

        for variable in context.nonfree_variables {
            context.free_variables.remove(&variable);
        }

        context.free_variables
    }
}

struct Context<'local, 'cache> {
    /// This is a BTreeMap so that collecting it into a Vec later yields a
    /// deterministic ordering.
    free_variables: BTreeMap<DefinitionInfoId, Type>,
    nonfree_variables: HashSet<DefinitionInfoId>,
    cache: &'local ModuleCache<'cache>,
}

impl<'local, 'cache> Context<'local, 'cache> {
    fn new(cache: &'local ModuleCache<'cache>) -> Self {
        Self { cache, free_variables: Default::default(), nonfree_variables: Default::default() }
    }
}

trait FreeVars {
    fn find_free_vars(&self, ctx: &mut Context);

    fn remove_vars_defined_here(&self, ctx: &mut Context) {
        let mut remove_context = Context::new(ctx.cache);
        self.find_free_vars(&mut remove_context);

        for id in remove_context.free_variables.into_keys() {
            ctx.nonfree_variables.insert(id);
        }
    }
}

impl<'c> FreeVars for ast::Ast<'c> {
    fn find_free_vars(&self, ctx: &mut Context) {
        dispatch_on_expr!(self, FreeVars::find_free_vars, ctx)
    }
}

impl<'c> FreeVars for ast::Literal<'c> {
    fn find_free_vars(&self, _ctx: &mut Context) {
        // Nothing to do
    }
}

impl<'c> FreeVars for ast::Variable<'c> {
    fn find_free_vars(&self, ctx: &mut Context) {
        let id = self.definition.unwrap();

        if !ctx.cache.definition_infos[id.0].global && !ctx.free_variables.contains_key(&id) {
            ctx.free_variables.insert(id, self.get_type().unwrap().clone());
        }
    }
}

impl<'c> FreeVars for ast::Lambda<'c> {
    fn find_free_vars(&self, ctx: &mut Context) {
        self.body.find_free_vars(ctx);

        for parameter in &self.args {
            parameter.remove_vars_defined_here(ctx);
        }
    }
}

impl<'c> FreeVars for ast::FunctionCall<'c> {
    fn find_free_vars(&self, ctx: &mut Context) {
        self.function.find_free_vars(ctx);
        for arg in &self.args {
            arg.find_free_vars(ctx);
        }
    }
}

impl<'c> FreeVars for ast::Definition<'c> {
    fn find_free_vars(&self, ctx: &mut Context) {
        self.expr.find_free_vars(ctx);
        self.pattern.remove_vars_defined_here(ctx);
    }
}

impl<'c> FreeVars for ast::If<'c> {
    fn find_free_vars(&self, ctx: &mut Context) {
        self.condition.find_free_vars(ctx);
        self.then.find_free_vars(ctx);
        self.otherwise.find_free_vars(ctx);
    }
}

impl<'c> FreeVars for ast::Match<'c> {
    fn find_free_vars(&self, ctx: &mut Context) {
        self.expression.find_free_vars(ctx);

        for (pattern, branch) in &self.branches {
            branch.find_free_vars(ctx);
            pattern.remove_vars_defined_here(ctx);
        }
    }
}

impl<'c> FreeVars for ast::TypeDefinition<'c> {
    fn find_free_vars(&self, _ctx: &mut Context) {
        // Nothing to do
    }
}

impl<'c> FreeVars for ast::TypeAnnotation<'c> {
    fn find_free_vars(&self, ctx: &mut Context) {
        self.lhs.find_free_vars(ctx);
    }
}

impl<'c> FreeVars for ast::Import<'c> {
    fn find_free_vars(&self, _ctx: &mut Context) {
        // Nothing to do
    }
}

impl<'c> FreeVars for ast::TraitDefinition<'c> {
    fn find_free_vars(&self, _ctx: &mut Context) {
        // Nothing to do
    }
}

impl<'c> FreeVars for ast::TraitImpl<'c> {
    fn find_free_vars(&self, _ctx: &mut Context) {
        // Nothing to do
    }
}

impl<'c> FreeVars for ast::Return<'c> {
    fn find_free_vars(&self, ctx: &mut Context) {
        self.expression.find_free_vars(ctx);
    }
}

impl<'c> FreeVars for ast::Sequence<'c> {
    fn find_free_vars(&self, ctx: &mut Context) {
        for statement in &self.statements {
            statement.find_free_vars(ctx);
        }
    }
}

impl<'c> FreeVars for ast::Extern<'c> {
    fn find_free_vars(&self, _ctx: &mut Context) {
        // Nothing to do
    }
}

impl<'c> FreeVars for ast::MemberAccess<'c> {
    fn find_free_vars(&self, ctx: &mut Context) {
        self.lhs.find_free_vars(ctx);
    }
}

impl<'c> FreeVars for ast::Assignment<'c> {
    fn find_free_vars(&self, ctx: &mut Context) {
        self.lhs.find_free_vars(ctx);
        self.rhs.find_free_vars(ctx);
    }
}

impl<'c> FreeVars for ast::EffectDefinition<'c> {
    fn find_free_vars(&self, _ctx: &mut Context) {
        // Nothing to do
    }
}

impl<'c> FreeVars for ast::Handle<'c> {
    fn find_free_vars(&self, ctx: &mut Context) {
        self.expression.find_free_vars(ctx);

        for (pattern, branch) in &self.branches {
            branch.find_free_vars(ctx);
            pattern.remove_vars_defined_here(ctx);
        }

        for resume in &self.resumes {
            ctx.free_variables.remove(resume);
        }
    }
}

impl<'c> FreeVars for ast::NamedConstructor<'c> {
    fn find_free_vars(&self, ctx: &mut Context) {
        self.sequence.find_free_vars(ctx);
    }
}

impl<'c> FreeVars for ast::Reference<'c> {
    fn find_free_vars(&self, ctx: &mut Context) {
        self.expression.find_free_vars(ctx);
    }
}
