//! The MIR builder will translate a single top-level item into the equivalent MIR for the item.
//! This is meant to work in parallel for each item.
//!
//! For more on the Medium-level IR (MIR) itself, see [super].
//!
//! Although the MIR may eventually be monomorphized, the initial output of this builder keeps
//! the original generics, relying on a later pass to manually either specialize each function
//! or existentialize it.
//!
//! The MIR-builder will however, perform closure conversion on any functions with closure types
//! it finds.
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, LazyLock},
};

use dashmap::DashMap;
use inc_complete::DbGet;
use rustc_hash::FxHashMap;

use crate::{
    incremental::{GetItem, GetItemRaw, TypeCheck},
    iterator_extensions::mapvec,
    lexer::token::{FloatKind, IntegerKind},
    mir::{
        Block, BlockId, Definition, DefinitionId, FloatConstant, Generic, Instruction, IntConstant, Mir,
        TerminatorInstruction, Type, Value, next_definition_id,
    },
    name_resolution::Origin,
    parser::{
        cst::{self, Literal, Name, SequenceItem},
        ids::{ExprId, NameId, PathId, PatternId, TopLevelId, TopLevelName},
    },
    type_inference::{
        self,
        dependency_graph::TypeCheckResult,
        fresh_expr::ExtendedTopLevelContext,
        patterns::{Case, Constructor, DecisionTree},
        types::Type as TCType,
    },
};

mod intrinsics;
mod types;

/// Maps each TopLevelName to a unique DefinitionId
pub(crate) type SharedIdsMap = DashMap<TopLevelName, DefinitionId>;

/// A map from [TopLevelName] to [DefinitionId] shared between concurrent calls of
/// [build_initial_mir].
static NAME_IDS: LazyLock<SharedIdsMap> = LazyLock::new(DashMap::new);

/// Builds the MIR with the default shared global [SharedIdsMap].
pub(crate) fn build_initial_mir_with_shared_map<T>(compiler: &T, item_id: TopLevelId) -> Option<Mir>
where
    T: DbGet<TypeCheck> + DbGet<GetItem> + DbGet<GetItemRaw>,
{
    build_initial_mir(compiler, &NAME_IDS, item_id)
}

/// Convert the given item to an initial MIR representation. This may be done in parallel with all
/// other items in the program.
///
/// The initial MIR representation has no special handling of generics and requires another pass
/// afterward to reshape them into something the runtime can handle. Examples include either
/// monomorphization to specialize generics out of the code or an existential generics approach
/// which will pass around unsized values by reference.
pub(crate) fn build_initial_mir<T>(compiler: &T, ids: &SharedIdsMap, item_id: TopLevelId) -> Option<Mir>
where
    T: DbGet<TypeCheck> + DbGet<GetItem> + DbGet<GetItemRaw>,
{
    let types = TypeCheck(item_id).get(compiler);
    let (item, _) = GetItem(item_id).get(compiler);
    let mut context = Context::new(compiler, &types, item_id, ids);

    match &item.kind {
        cst::TopLevelItemKind::Definition(definition) => {
            context.definition(definition, true);
            Some(context.finish())
        },
        cst::TopLevelItemKind::TypeDefinition(type_definition) => {
            context.type_definition(type_definition);
            Some(context.finish())
        },
        cst::TopLevelItemKind::TraitDefinition(_) => unreachable!("Traits should be desguared to types"),
        cst::TopLevelItemKind::TraitImpl(_) => unreachable!("Trait impls should be desguared to definitions"),
        cst::TopLevelItemKind::EffectDefinition(effect_definition) => {
            context.effect_definition(effect_definition);
            Some(context.finish())
        },
        cst::TopLevelItemKind::Comptime(_) => None,
    }
}

enum LhsKind {
    LocalVar(NameId),
    DerefCall(ExprId),
    Annotation(ExprId),
    FieldAccess(ExprId, ExprId),
    Other,
}

/// The per-[TopLevelId] context. This pass is designed so that we can convert every top-level item
/// to MIR in parallel.
struct Context<'local, Db> {
    compiler: &'local Db,
    types: &'local TypeCheckResult,

    top_level_id: TopLevelId,

    /// The number of generics in scope
    generics_in_scope: FxHashMap<type_inference::generics::Generic, Generic>,

    current_function: Option<Definition>,
    current_block: BlockId,

    global_variables: FxHashMap<Origin, Value>,
    local_variables: FxHashMap<NameId, Value>,

    /// Names of locally-declared mutable variables (`var name = expr`).
    /// These have a StackAlloc'd pointer as their MIR value.
    mutable_locals: rustc_hash::FxHashSet<NameId>,

    /// Stack of `(continue_target, break_target)` for each enclosing `while`/`for` loop.
    /// Innermost loop is last. Swapped out across lambda boundaries.
    loop_targets: Vec<(BlockId, BlockId)>,

    finished_functions: FxHashMap<DefinitionId, Definition>,
    name_to_id: &'local SharedIdsMap,

    /// Any external items will have their name & type stored here
    external: FxHashMap<DefinitionId, super::Extern>,

    /// Cache of whether a given top-level item is an effect definition.
    ///
    /// For each Call we have to decide if it is effectful or not to potentially
    /// issue a Perform instead. This cache is used to avoid issuing a GetItemRaw
    /// query in some more cases but could be improved more.
    effect_defs: FxHashMap<TopLevelId, bool>,
}

impl<'local, Db> Context<'local, Db> {
    fn new(
        compiler: &'local Db, types: &'local TypeCheckResult, top_level_id: TopLevelId,
        name_mappings: &'local SharedIdsMap,
    ) -> Self {
        Self {
            compiler,
            types,
            top_level_id,
            generics_in_scope: Default::default(),
            global_variables: FxHashMap::default(),
            local_variables: FxHashMap::default(),
            mutable_locals: Default::default(),
            loop_targets: Vec::new(),
            current_block: BlockId::ENTRY_BLOCK,
            current_function: None,
            finished_functions: Default::default(),
            name_to_id: name_mappings,
            external: Default::default(),
            effect_defs: Default::default(),
        }
    }

    /// Returns the current function being built. Panics if thre is none.
    fn current_function(&mut self) -> &mut Definition {
        self.current_function.as_mut().unwrap()
    }

    fn type_of_value(&self, value: &Value) -> Type {
        self.current_function.as_ref().unwrap().type_of_value(value, &self.external, &self.finished_functions)
    }

    /// Returns the current block being inserted into. Panics if there is no current function.
    fn current_block(&mut self) -> &mut Block {
        &mut self.current_function.as_mut().unwrap().blocks[self.current_block]
    }

    fn context(&self) -> &'local ExtendedTopLevelContext {
        &self.types.result.context
    }
}

impl<'local, Db> Context<'local, Db>
where
    Db: DbGet<TypeCheck> + DbGet<GetItem> + DbGet<GetItemRaw>,
{
    /// Push an instruction and return its result.
    fn push_instruction(&mut self, instruction: Instruction, result_type: Type) -> Value {
        let current_block = self.current_block;
        let function = self.current_function();
        let id = function.instructions.push(instruction);
        function.instruction_result_types.push_existing(id, result_type);
        function.blocks[current_block].instructions.push(id);
        Value::InstructionResult(id)
    }

    /// Create a block (although do not switch to it) and return it
    fn push_block(&mut self, parameter_types: Vec<Type>) -> BlockId {
        self.current_function.as_mut().unwrap().blocks.push(Block::new(parameter_types))
    }

    /// Create a block with no block parameters (although do not switch to it) and return it
    fn push_block_no_params(&mut self) -> BlockId {
        self.push_block(Vec::new())
    }

    /// Switch to a new block to start inserting instructions into
    fn switch_to_block(&mut self, block: BlockId) {
        self.current_block = block;
    }

    /// Add a parameter to the current block
    fn push_parameter(&mut self, parameter_type: Type) {
        self.current_block().parameter_types.push(parameter_type);
    }

    /// Terminate the current block with the given terminator instruction
    fn terminate_block(&mut self, terminator: TerminatorInstruction) {
        let block = self.current_block();
        if block.terminator.is_none() {
            block.terminator = Some(terminator);
        }
    }

    fn expr_type(&self, expr: ExprId) -> Type {
        let typ = &self.types.result.maps.expr_types[&expr];
        self.convert_type(typ, None)
    }

    /// Defines the given value in local or global scope depending on its origin
    fn define_variable(&mut self, origin: Origin, value: Value) {
        match origin {
            Origin::Local(name) => self.local_variables.insert(name, value),
            other => self.global_variables.insert(other, value),
        };
    }

    /// Retrieves the corresponding [DefinitionId] for a particular [TopLevelName].
    /// Note that this uses a shared [DashMap] internally and the resulting id will be
    /// nondeterministic across multiple compiler runs.
    fn get_definition_id(&self, name: &TopLevelName) -> DefinitionId {
        *self.name_to_id.entry(*name).or_insert_with(next_definition_id)
    }

    fn get_definition_name(&self, name: &TopLevelName) -> Name {
        let (_, context) = GetItemRaw(name.top_level_item).get(self.compiler);
        context.names[name.local_name_id].clone()
    }

    fn make_definition_value(&mut self, id: DefinitionId, name: Name, typ: Type) -> Value {
        self.reference_definition(id, name, typ);
        Value::Definition(id)
    }

    fn reference_definition(&mut self, id: DefinitionId, name: Name, typ: Type) {
        if !self.finished_functions.contains_key(&id) && self.current_function.as_ref().is_none_or(|def| def.id != id) {
            self.external.insert(id, super::Extern { name, typ });
        }
    }

    fn expression(&mut self, expr: ExprId) -> Value {
        match &self.context()[expr] {
            cst::Expr::Error => unreachable!("Error expression encountered while generating boxed mir"),
            cst::Expr::Literal(literal) => self.literal(literal, expr),
            cst::Expr::Variable(path_id) => self.variable(*path_id),
            cst::Expr::Sequence(sequence) => self.sequence(sequence),
            cst::Expr::Definition(definition) => self.definition(definition, false),
            cst::Expr::MemberAccess(member_access) => self.member_access(member_access, expr),
            cst::Expr::Call(call) => self.call(call, expr),
            cst::Expr::Lambda(lambda) => self.lambda(lambda, None, None, expr, false),
            cst::Expr::If(if_) => self.if_(if_, expr),
            cst::Expr::Match(_) => self.match_(expr),
            cst::Expr::Is(_) => unreachable!("Expr::Is should be desugared during GetItem"),
            cst::Expr::Handle(handle) => self.handle(handle, expr),
            cst::Expr::Reference(reference) => self.reference(reference),
            cst::Expr::TypeAnnotation(type_annotation) => self.expression(type_annotation.lhs),
            cst::Expr::Constructor(constructor) => self.constructor(constructor, expr),
            cst::Expr::Quoted(quoted) => self.quoted(quoted),
            cst::Expr::Loop(_) => unreachable!("Loops should be desugared before MIR generation"),
            cst::Expr::While(while_) => self.while_(while_),
            cst::Expr::For(for_) => self.for_(for_),
            cst::Expr::Break => self.break_(),
            cst::Expr::Continue => self.continue_(),
            cst::Expr::Return(return_) => self.return_(return_.expression),
            cst::Expr::Assignment(assignment) => self.assignment(assignment),
            cst::Expr::Extern(extern_) => self.extern_(extern_, expr),
            cst::Expr::InterpolatedString(_) => {
                unreachable!("InterpolatedString should be desugared before MIR generation")
            },
        }
    }

    fn literal(&mut self, literal: &Literal, expr: ExprId) -> Value {
        match literal {
            Literal::Unit => Value::Unit,
            Literal::Bool(x) => Value::Bool(*x),
            Literal::Integer(x, None) => {
                let kind = match self.expr_type(expr) {
                    Type::Primitive(crate::mir::PrimitiveType::Int(kind)) => kind,
                    _ => IntegerKind::I32,
                };
                Self::integer(*x, kind)
            },
            Literal::Integer(x, Some(kind)) => Self::integer(*x, *kind),
            Literal::Float(x, None) => {
                match self.expr_type(expr) {
                    Type::Primitive(crate::mir::PrimitiveType::Float(FloatKind::F32)) => {
                        Value::Float(FloatConstant::F32(*x))
                    },
                    // Default to F64, there are cases when the type variable may still be unbound here.
                    // Generally it means it was unused or there was an error.
                    _ => Value::Float(FloatConstant::F64(*x)),
                }
            },
            Literal::Float(x, Some(FloatKind::F32)) => Value::Float(FloatConstant::F32(*x)),
            Literal::Float(x, Some(FloatKind::F64)) => Value::Float(FloatConstant::F64(*x)),
            Literal::String(x) => self.push_instruction(Instruction::MakeString(x.clone()), Type::string()),
            Literal::Char(x) => Value::Char(*x),
        }
    }

    fn integer(value: u64, kind: IntegerKind) -> Value {
        match kind {
            IntegerKind::I8 => Value::Integer(IntConstant::I8((value as i64).try_into().unwrap())),
            IntegerKind::I16 => Value::Integer(IntConstant::I16((value as i64).try_into().unwrap())),
            IntegerKind::I32 => Value::Integer(IntConstant::I32(value as i32)),
            IntegerKind::I64 => Value::Integer(IntConstant::I64(value as i64)),
            IntegerKind::Isz => Value::Integer(IntConstant::Isz(value.try_into().unwrap())),
            IntegerKind::U8 => Value::Integer(IntConstant::U8(value.try_into().unwrap())),
            IntegerKind::U16 => Value::Integer(IntConstant::U16(value.try_into().unwrap())),
            IntegerKind::U32 => Value::Integer(IntConstant::U32(value.try_into().unwrap())),
            IntegerKind::U64 => Value::Integer(IntConstant::U64(value.try_into().unwrap())),
            IntegerKind::Usz => Value::Integer(IntConstant::Usz(value.try_into().unwrap())),
        }
    }

    fn variable(&mut self, path_id: PathId) -> Value {
        // Deliberately allow us to reference variables not in the context.
        // This allows us to convert all definitions to MIR in parallel, trusting
        // that the links will work out later.
        let mut value =
            match self.context().path_origin(path_id) {
                Some(Origin::TopLevelDefinition(name)) => {
                    let id = self.get_definition_id(&name);
                    let name = self.get_definition_name(&name);
                    let typ = self.convert_path_type(path_id);
                    self.make_definition_value(id, name, typ)
                },
                Some(Origin::Local(name)) => {
                    let ptr = *self.local_variables.get(&name).unwrap_or_else(|| {
                        panic!("No cached variable for {} with name {name}", self.context()[path_id])
                    });
                    if self.mutable_locals.contains(&name) {
                        // Mutable locals are StackAlloc'd pointers; auto-deref to load the value.
                        let val_type = self.convert_path_type(path_id);
                        self.push_instruction(Instruction::Deref(ptr), val_type)
                    } else {
                        ptr
                    }
                },
                Some(origin @ Origin::Builtin(_)) => *self.global_variables.get(&origin).unwrap_or_else(|| {
                    panic!("No cached variable for {} with origin {origin}", self.context()[path_id])
                }),
                Some(Origin::TypeResolution) => unreachable!("Unresolved TypeResolution origin found"),
                // This is possible if there were errors during name resolution
                None => Value::Error,
            };

        // If this type was instantiated, then we need to recover the pre-instantiated
        // type and make an explicit [Instruction::Instantiate]. We cannot check this only in the
        // `Origin::TopLevelDefinition` case because local lambdas may also be polymorphic.
        // TODO: Closures may be wrapped in an Instruction result which would break this check
        if let Value::Definition(id) = value {
            if let Some(bindings) = self.types.result.context.get_instantiation(path_id) {
                let typ = self.convert_path_type(path_id);
                let bindings = Arc::new(mapvec(bindings, |typ| self.convert_type(typ, None)));
                let instruction = Instruction::Instantiate(id, bindings);
                value = self.push_instruction(instruction, typ);
            }
        }

        value
    }

    fn sequence(&mut self, sequence: &[SequenceItem]) -> Value {
        let mut result = Value::Unit;
        for item in sequence {
            result = self.expression(item.expr);
        }
        result
    }

    fn definition(&mut self, definition: &cst::Definition, is_global: bool) -> Value {
        let (name, name_id) = match self.try_find_name(definition.pattern) {
            Some((name, name_id)) => (name, Some(name_id)),
            None => (Arc::new("global".to_string()), None),
        };

        if is_global {
            let typ = &self.types.get_generalized(name_id.unwrap());
            self.set_generics_in_scope(&typ);
        }

        let previous_state = self.is_non_function_global(definition).then(|| {
            let generic_count = self.generics_in_scope.len() as u32;
            let typ = self.convert_pattern_type(definition.pattern);
            self.start_global(name, name_id, generic_count, typ)
        });

        let mut value = match &self.context()[definition.rhs] {
            cst::Expr::Lambda(lambda) => {
                let name = self.try_find_name(definition.pattern).map(|(name, _)| name);
                self.lambda(lambda, name_id, name, definition.rhs, is_global)
            },
            _ => self.expression(definition.rhs),
        };

        // TODO: Globals should probably never be stack allocated
        if definition.mutable {
            value = self.push_instruction(Instruction::StackAlloc(value), Type::POINTER);
            if let Some(id) = name_id {
                self.mutable_locals.insert(id);
            }
        }
        self.bind_pattern(definition.pattern, value);

        if let Some(state) = previous_state {
            self.terminate_block(TerminatorInstruction::Result(value));
            self.end_global(state);
        }
        Value::Unit
    }

    /// True if the given definition is syntactically a global non-function variable.
    fn is_non_function_global(&self, definition: &cst::Definition) -> bool {
        self.current_function.is_none() && !matches!(self.context()[definition.rhs], cst::Expr::Lambda(_))
    }

    fn member_access(&mut self, member_access: &cst::MemberAccess, expr: ExprId) -> Value {
        let index = self.context().member_access_index(expr).unwrap_or(u32::MAX);

        // If the object has a reference or pointer type (e.g. `p: mut Point`), the MIR value is a pointer.
        // Use GetFieldPtr to produce a pointer to the field (MIR rep of e.g. `mut I32`).
        let object_expr = member_access.object;
        let object_type = self.types.result.maps.expr_types[&object_expr].follow(&self.types.bindings);
        let reference_element =
            object_type.reference_or_pointer_element(&self.types.bindings).map(|typ| self.convert_type(typ, None));

        if let Some(struct_type) = reference_element {
            let struct_ptr = self.expression(object_expr);
            self.push_instruction(Instruction::GetFieldPtr { struct_ptr, struct_type, index }, Type::POINTER)
        } else {
            let tuple = self.expression(object_expr);
            let element_type = match self.type_of_value(&tuple) {
                Type::Tuple(elements) => elements.get(index as usize).cloned().unwrap_or(Type::ERROR),
                _ => Type::ERROR,
            };
            self.push_instruction(Instruction::IndexTuple { tuple, index }, element_type)
        }
    }

    fn call(&mut self, call: &cst::Call, id: ExprId) -> Value {
        // Intrinsics in the stdlib are written as a call `intrinsic "Name" arg1 ... argN`
        // We check for this case here since the arguments are required to lower to concrete
        // instructions rather than a function wrapper (which would be needed if this was handled
        // in the recursive `cst::Variable` case when lowering the function expression)
        if let Some(result) = self.try_lower_intrinsic(call, id) {
            return result;
        }

        let function = self.expression(call.function);
        let arguments = mapvec(&call.arguments, |expr| self.expression(expr.expr));
        let result_type = self.expr_type(id);

        // TODO: This doesn't handle effect functions used as first-class values
        // TODO: Effect functions in MIR could be wrapper functions over a single Perform
        // instruction. Then we wouldn't have to handle this case in each Call at all.
        if let cst::Expr::Variable(path_id) = &self.context()[call.function] {
            if let Some(effect_op) = self.try_resolve_effect_op(*path_id) {
                let arguments = mapvec(&call.arguments, |expr| self.expression(expr.expr));
                let result_type = self.expr_type(id);
                return self.push_instruction(Instruction::Perform { effect_op, arguments }, result_type);
            }
        }

        let instruction = if self.type_of_value(&function).is_closure() {
            Instruction::CallClosure { closure: function, arguments }
        } else {
            Instruction::Call { function, arguments }
        };

        self.push_instruction(instruction, result_type)
    }

    /// If `path` resolves to an effect-operation declaration, return its
    /// [DefinitionId]. Otherwise return None.
    ///
    /// We use a cache to avoid hitting inc-complete's locks for every
    /// call instruction but this can likely be improved further.
    fn try_resolve_effect_op(&mut self, path: PathId) -> Option<DefinitionId> {
        let origin = self.context().path_origin(path)?;
        let Origin::TopLevelDefinition(name) = origin else { return None };

        let is_effect = self.effect_defs.get(&name.top_level_item).copied().unwrap_or_else(|| {
            let (item, _) = GetItemRaw(name.top_level_item).get(self.compiler);
            let is_effect = matches!(&item.kind, cst::TopLevelItemKind::EffectDefinition(_));
            self.effect_defs.insert(name.top_level_item, is_effect);
            is_effect
        });
        if !is_effect {
            return None;
        }

        // Cold path: this TopLevelId is an effect definition. Verify that the
        // referenced NameId is one of its ops (defensive — every name pointing
        // into an effect def should be an op, but we confirm to be safe).
        let (item, _) = GetItemRaw(name.top_level_item).get(self.compiler);
        if let cst::TopLevelItemKind::EffectDefinition(effect) = &item.kind {
            if effect.body.iter().any(|d| d.name == name.local_name_id) {
                return Some(self.get_definition_id(&name));
            }
        }
        None
    }

    fn try_find_name(&self, pattern: PatternId) -> Option<(Name, NameId)> {
        match &self.context()[pattern] {
            cst::Pattern::Error => None,
            cst::Pattern::Literal(_) => None,
            cst::Pattern::Constructor(..) => None,
            cst::Pattern::TypeAnnotation(pattern, _) => self.try_find_name(*pattern),
            cst::Pattern::Variable(name) | cst::Pattern::MethodName { item_name: name, .. } => {
                Some((self.context()[*name].clone(), *name))
            },
        }
    }

    /// Save the current function state, create a new function,
    /// run `f` to fill in the function's body, then restore the previous state
    /// and return the new function value.
    fn new_definition(
        &mut self, name: Name, name_id: Option<NameId>, generic_count: u32, typ: Type, f: impl FnOnce(&mut Self),
    ) -> DefinitionId {
        let state = self.start_global(name, name_id, generic_count, typ);
        f(self);
        self.end_global(state)
    }

    fn start_global(
        &mut self, name: Name, name_id: Option<NameId>, generic_count: u32, typ: Type,
    ) -> (Option<Definition>, BlockId) {
        // Safety: This function must always be paired with [Self::end_global]
        let previous_function = self.current_function.take();
        let previous_block = std::mem::replace(&mut self.current_block, BlockId::ENTRY_BLOCK);

        let definition_id = if let Some(name_id) = name_id {
            let id = self.get_definition_id(&TopLevelName::new(self.top_level_id, name_id));
            self.external.remove(&id);
            id
        } else {
            next_definition_id()
        };

        self.current_function = Some(Definition::new(name, definition_id, generic_count, typ));
        (previous_function, previous_block)
    }

    fn end_global(&mut self, start_global_state: (Option<Definition>, BlockId)) -> DefinitionId {
        // Safety: This function must always be paired with [Self::start_global]
        let finished_function = std::mem::replace(&mut self.current_function, start_global_state.0).unwrap();

        let definition_id = finished_function.id;
        self.current_block = start_global_state.1;

        self.finished_functions.insert(definition_id, finished_function);
        definition_id
    }

    fn lambda(
        &mut self, lambda: &cst::Lambda, name_id: Option<NameId>, name: Option<Name>, expr: ExprId, is_global: bool,
    ) -> Value {
        let name = name.unwrap_or_else(|| Arc::new("lambda".to_string()));
        let full_type = self.convert_expr_type(expr);
        let Type::Function(function_type) = &full_type else { unreachable!("Lambda does not have a function type") };

        let is_move = self.context().is_move_closure(expr);

        // Determine which captured variables are mutable in the outer scope.
        let mutable_captures: rustc_hash::FxHashSet<NameId> =
            if let Some(free_vars) = self.context().get_closure_environment(expr) {
                free_vars.iter().filter(|v| self.mutable_locals.contains(v)).copied().collect()
            } else {
                Default::default()
            };

        // Lambdas aren't really generic but they may capture generics from their containing
        // function. Marking them as generic here effectively hoists out the generic parameters.
        // It also means we have to instantiate lambdas when they're used with the current generic
        // parameters.
        let generics_count = self.generics_in_scope.len() as u32;
        let old_scope = std::mem::take(&mut self.local_variables);
        let old_mutables = std::mem::take(&mut self.mutable_locals);
        let old_loop_targets = std::mem::take(&mut self.loop_targets);

        let id = self.new_definition(name.clone(), name_id, generics_count, full_type.clone(), |this| {
            for (i, parameter) in lambda.parameters.iter().enumerate() {
                let parameter_type = &this.types.result.maps.pattern_types[&parameter.pattern];
                this.push_parameter(this.convert_type(parameter_type, None));

                let parameter_value = Value::Parameter(this.current_block, i as u32);
                this.bind_pattern(parameter.pattern, parameter_value);

                if parameter.is_mutable {
                    if let Some((_, name_id)) = this.try_find_name(parameter.pattern) {
                        let alloc = this.push_instruction(Instruction::StackAlloc(parameter_value), Type::POINTER);
                        this.local_variables.insert(name_id, alloc);
                        this.mutable_locals.insert(name_id);
                    }
                }
            }

            if let Some(free_vars) = this.context().get_closure_environment(expr) {
                if let Some(env) = function_type.environment() {
                    this.push_parameter(env.clone());
                }

                let environment = Value::Parameter(this.current_block, lambda.parameters.len() as u32);
                this.unpack_closure_environment(free_vars.iter().copied(), environment);

                // For regular closures, mutable captures are pointers (by reference).
                // Register them for auto-deref on access.
                // For `move` closures, captures are values, so no auto-deref needed.
                if !is_move {
                    for var in free_vars.iter() {
                        if mutable_captures.contains(var) {
                            this.mutable_locals.insert(*var);
                        }
                    }
                }
            }

            // Pre-populate the self-reference so recursive calls within the body
            // (e.g. `recur` in desugared loop expressions) can resolve via Origin::Local.
            // The entry is discarded automatically when `old_scope` is restored after
            // `new_definition` returns.
            if let Some(self_name_id) = name_id {
                let self_def_id = this.current_function.as_ref().unwrap().id;
                let mut self_value = Value::Definition(self_def_id);
                if this.context().get_closure_environment(expr).is_some() {
                    let environment = Value::Parameter(this.current_block, lambda.parameters.len() as u32);
                    self_value = this.push_instruction(
                        Instruction::PackClosure { function: self_value, environment },
                        full_type.clone(),
                    );
                }
                this.local_variables.insert(self_name_id, self_value);
            }

            let return_value = this.expression(lambda.body);
            this.terminate_block(TerminatorInstruction::Return(return_value));
        });

        self.local_variables = old_scope;
        self.mutable_locals = old_mutables;
        self.loop_targets = old_loop_targets;

        // If this is a generic lambda, it will have generics forward from the currenty context
        // which we need to manually instantiate.
        let mut value = self.make_definition_value(id, name, full_type.clone());
        if !is_global && !self.generics_in_scope.is_empty() {
            let bindings = Arc::new(mapvec(0..self.generics_in_scope.len() as u32, |i| Type::Generic(Generic(i))));
            value = self.push_instruction(Instruction::Instantiate(id, bindings), full_type.clone());
        }
        if let Some(free_vars) = self.context().get_closure_environment(expr) {
            let environment = self.pack_closure_environment(free_vars, is_move);
            value = self.push_instruction(Instruction::PackClosure { function: value, environment }, full_type);
        }
        value
    }

    /// Packs each given variable into a closure environment tuple.
    /// Expects to be called after [Self::unpack_closure_environment]
    fn pack_closure_environment(&mut self, free_vars: &BTreeSet<NameId>, is_move: bool) -> Value {
        // We must match the packing done in [type_inference::free_vars::make_tuple_type]
        assert!(!free_vars.is_empty());

        let values = mapvec(free_vars, |var| {
            let value = *self.local_variables.get(var).unwrap();

            if is_move && self.mutable_locals.contains(var) {
                // `move` closures capture mutable variables by value: dereference the
                // StackAlloc pointer to load the current value into the environment.
                let tc_type = &self.types.result.maps.name_types[var];
                let val_type = self.convert_type(tc_type, None);
                self.push_instruction(Instruction::Deref(value), val_type)
            } else {
                // Regular closures: mutable variables are StackAlloc pointers, packed
                // directly for capture by reference. Immutable variables are packed by value.
                // Move closures: all immutable variables are packed by value.
                value
            }
        });
        let types = mapvec(&values, |value| self.type_of_value(value));
        let make_tuple = Instruction::MakeTuple(values);
        self.push_instruction(make_tuple, Type::tuple(types))
    }

    /// Unpack a closure environment tuple parameter, defining each name id captured in the
    /// closure. Expects `free_vars` to be non-empty.
    ///
    /// Note that this modifies `self.local_variables`
    fn unpack_closure_environment(&mut self, free_vars: impl ExactSizeIterator<Item = NameId>, environment: Value) {
        let Type::Tuple(env_fields) = self.type_of_value(&environment) else { unreachable!() };
        assert_eq!(env_fields.len(), free_vars.len());

        for (i, (var, env_field)) in free_vars.zip(env_fields.iter().cloned()).enumerate() {
            let index = Instruction::IndexTuple { tuple: environment, index: i as u32 };
            let result = self.push_instruction(index, env_field);
            let existing = self.local_variables.insert(var, result);
            assert!(existing.is_none(), "Closure is overwriting values from the outer scope");
        }
    }

    fn while_(&mut self, while_: &cst::While) -> Value {
        let header = self.push_block_no_params();
        let body = self.push_block_no_params();
        let exit = self.push_block_no_params();

        self.terminate_block(TerminatorInstruction::jmp_no_args(header));

        self.switch_to_block(header);
        let cond = self.expression(while_.condition);
        self.terminate_block(TerminatorInstruction::if_(cond, body, exit, exit));

        self.switch_to_block(body);
        self.loop_targets.push((header, exit));
        let _ = self.expression(while_.body);
        self.loop_targets.pop();
        self.terminate_block(TerminatorInstruction::jmp_no_args(header));

        self.switch_to_block(exit);
        Value::Unit
    }

    fn for_(&mut self, for_: &cst::For) -> Value {
        let variable_type = self.expr_type(for_.start);
        let kind = match &variable_type {
            Type::Primitive(crate::mir::PrimitiveType::Int(k)) => *k,
            _ => unreachable!("for-loop range was not inferred to an integer type: {variable_type:?}"),
        };

        let start_value = self.expression(for_.start);
        let end_value = self.expression(for_.end);

        let header = self.push_block(vec![variable_type.clone()]);
        let body = self.push_block_no_params();
        let step = self.push_block_no_params();
        let exit = self.push_block_no_params();

        let variable = Value::Parameter(header, 0);
        self.local_variables.insert(for_.variable, variable);

        self.terminate_block(TerminatorInstruction::jmp(header, start_value));

        self.switch_to_block(header);
        let cmp = if kind.is_signed() {
            Instruction::LessSigned(variable, end_value.clone())
        } else {
            Instruction::LessUnsigned(variable, end_value.clone())
        };
        let cond = self.push_instruction(cmp, Type::BOOL);
        self.terminate_block(TerminatorInstruction::if_(cond, body, exit, exit));

        self.switch_to_block(body);
        self.loop_targets.push((step, exit));
        let _ = self.expression(for_.body);
        self.loop_targets.pop();
        self.terminate_block(TerminatorInstruction::jmp_no_args(step));

        self.switch_to_block(step);
        let one = Self::integer(1, kind);
        let variable_plus_one = self.push_instruction(Instruction::AddInt(variable, one), variable_type);
        self.terminate_block(TerminatorInstruction::jmp(header, variable_plus_one));

        self.switch_to_block(exit);
        Value::Unit
    }

    fn break_(&mut self) -> Value {
        let exit = self.loop_targets.last().expect("`break` outside of a loop").1;
        self.terminate_block(TerminatorInstruction::jmp_no_args(exit));
        Value::Error
    }

    fn continue_(&mut self) -> Value {
        let cont = self.loop_targets.last().expect("`continue` outside of a loop").0;
        self.terminate_block(TerminatorInstruction::jmp_no_args(cont));
        Value::Error
    }

    fn if_(&mut self, if_: &cst::If, expr: ExprId) -> Value {
        let condition = self.expression(if_.condition);

        let then = self.push_block_no_params();
        let else_ = self.push_block_no_params();
        let end = if if_.else_.is_some() { self.push_block_no_params() } else { else_ };
        self.terminate_block(TerminatorInstruction::if_(condition, then, else_, end));

        self.switch_to_block(then);
        let then_value = self.expression(if_.then);

        if let Some(else_expr) = if_.else_ {
            let result_type = self.expr_type(expr);
            self.terminate_block(TerminatorInstruction::jmp(end, then_value));

            self.switch_to_block(else_);
            let else_value = self.expression(else_expr);
            self.terminate_block(TerminatorInstruction::jmp(end, else_value));
            self.switch_to_block(end);
            self.push_parameter(result_type);
            Value::Parameter(end, 0)
        } else {
            self.terminate_block(TerminatorInstruction::jmp_no_args(end));
            self.switch_to_block(end);
            Value::Unit
        }
    }

    fn match_(&mut self, expr: ExprId) -> Value {
        match self.context().decision_tree(expr) {
            Some((define_match_var, tree)) => {
                self.expression(*define_match_var);
                self.decision_tree(tree.clone(), expr)
            },
            None => Value::Error,
        }
    }

    fn decision_tree(&mut self, tree: DecisionTree, match_expr: ExprId) -> Value {
        match tree {
            DecisionTree::Success(expr) => self.expression(expr),
            // Expect an error to already be issued
            DecisionTree::Failure { .. } => Value::Error,
            DecisionTree::Guard { condition, then, else_ } => self.match_if_guard(condition, then, *else_, match_expr),
            DecisionTree::Switch(tag, cases, else_) => self.switch(tag, cases, else_, match_expr),
        }
    }

    /// Almost identical to an if-then-else, the main difference being the else is required
    /// and is of type [DecisionTree]
    fn match_if_guard(
        &mut self, condition: ExprId, then_expr: ExprId, else_tree: DecisionTree, match_expr: ExprId,
    ) -> Value {
        let then = self.push_block_no_params();
        let else_ = self.push_block_no_params();

        let result_type = self.expr_type(match_expr);
        let end = self.push_block(vec![result_type]);

        let condition = self.expression(condition);
        self.terminate_block(TerminatorInstruction::if_(condition, then, else_, else_));

        self.switch_to_block(then);
        let then_value = self.expression(then_expr);
        self.terminate_block(TerminatorInstruction::jmp(end, then_value));

        self.switch_to_block(else_);
        let else_value = self.decision_tree(else_tree, match_expr);
        self.terminate_block(TerminatorInstruction::jmp(end, else_value));

        self.switch_to_block(end);
        Value::Parameter(end, 0)
    }

    fn switch(&mut self, tag: PathId, cases: Vec<Case>, else_: Option<Box<DecisionTree>>, match_expr: ExprId) -> Value {
        let value_being_matched = self.variable(tag);
        let int_value = self.extract_tag_value(value_being_matched);
        let start = self.current_block;

        let case_blocks = mapvec(0..cases.len(), |i| (i as u32, (self.push_block_no_params(), None)));
        let mut else_block = None;

        let result_type = self.expr_type(match_expr);
        let end = self.push_block(vec![result_type]);

        for ((_, (case_block, _)), case) in case_blocks.iter().zip(cases) {
            self.switch_to_block(*case_block);

            if !case.arguments.is_empty() {
                let Constructor::Variant(_, variant_index) = &case.constructor else {
                    unreachable!("For this constructor to define arguments it must be a Constructor::Variant")
                };

                // Cast the whole value being matched `(tag, union)` to `(tag, this_variant)`
                // and extract the variant from the tuple.
                let variant = self.extract_variant(value_being_matched, *variant_index);
                let variant_type = self.type_of_value(&variant);

                // And for each variable, extract the relevant field of the variant
                for (i, argument) in case.arguments.iter().enumerate() {
                    if let Some(origin) = self.context().path_origin(*argument) {
                        let field_type = Self::tuple_field_type(&variant_type, i);
                        let index_tuple = Instruction::IndexTuple { tuple: variant, index: i as u32 };
                        let field = self.push_instruction(index_tuple, field_type);
                        self.define_variable(origin, field);
                    }
                }
            }

            let result = self.decision_tree(case.body, match_expr);
            self.terminate_block(TerminatorInstruction::jmp(end, result));
        }

        if let Some(else_) = else_ {
            let block = self.push_block_no_params();
            else_block = Some((block, None));
            self.switch_to_block(block);
            let result = self.decision_tree(*else_, match_expr);
            self.terminate_block(TerminatorInstruction::jmp(end, result));
        }

        self.switch_to_block(start);
        self.terminate_block(TerminatorInstruction::Switch { int_value, cases: case_blocks, else_: else_block, end });
        self.switch_to_block(end);
        Value::Parameter(end, 0)
    }

    fn extract_tag_value(&mut self, value_being_matched: Value) -> Value {
        match self.type_of_value(&value_being_matched) {
            Type::Primitive(_) => value_being_matched,
            Type::Tuple(fields) => {
                if fields.is_empty() {
                    unreachable!("Cannot match on an empty tuple")
                }
                // Tagged unions have the form `(u8_tag, Union[...])`. For product
                // types (pairs, structs) there is always exactly one case, so return
                // a constant 0 so the switch always selects case 0.
                if fields.len() == 2 && matches!(fields[1], Type::Union(_)) {
                    let tag_type = fields[0].clone();
                    let instruction = Instruction::IndexTuple { tuple: value_being_matched, index: 0 };
                    self.push_instruction(instruction, tag_type)
                } else {
                    Value::tag_value(0)
                }
            },
            Type::Union(_) => unreachable!("Cannot match on a raw union type"),
            Type::Function(_) => unreachable!("Cannot match on a function type"),
            Type::Generic(_) => unreachable!("Cannot match on a generic type"),
        }
    }

    /// Cast & Extract the variant value from the given `(tag, union)` tuple,
    /// or return the product type value directly if it has no union tag.
    /// Returns the variant (as a tuple, no longer a union).
    fn extract_variant(&mut self, value_being_matched: Value, variant_index: usize) -> Value {
        let fields = match self.type_of_value(&value_being_matched) {
            Type::Tuple(fields) => fields,
            _ => unreachable!("Only `(tag, union)` tuples may have fields to extract"),
        };

        // Tagged unions have the form `(u8_tag, Union[...])`.
        // Product types (pairs, structs) are plain tuples — return the value directly
        // so the caller can index its fields at positions 0, 1, etc.
        if fields.len() != 2 || !matches!(fields[1], Type::Union(_)) {
            return value_being_matched;
        }

        let union_type = fields[1].clone();
        let Type::Union(variants) = &union_type else { unreachable!() };

        let variant_type = variants
            .get(variant_index)
            .unwrap_or_else(|| {
                unreachable!("Expected variant index {variant_index} but only had {} variants", variants.len())
            })
            .clone();

        let extract_union = Instruction::IndexTuple { tuple: value_being_matched, index: 1 };
        let union = self.push_instruction(extract_union, union_type);
        self.push_instruction(Instruction::Transmute(union), variant_type)
    }

    fn handle(&mut self, handle: &cst::Handle, expr: ExprId) -> Value {
        let result_type = self.expr_type(expr);

        // The parser wraps both the handled expression and each branch in closures already
        let body = self.expression(handle.expression);
        let cases = mapvec(&handle.cases, |(pattern, branch)| {
            let effect_op =
                self.try_resolve_effect_op(pattern.function).expect("Couldn't find effect op in MIR handle");
            let handler = self.expression(*branch);
            crate::mir::HandlerCase { effect_op, handler }
        });

        self.push_instruction(Instruction::Handle { body, cases }, result_type)
    }

    /// Emit a MIR [Definition] for each operation of an effect definition. The
    /// body of each stub is `fn op args.. = perform op args..`, ensuring the
    /// operation can be used as a first-class value (passed to higher-order code)
    /// while also unambiguously naming the effect via its [DefinitionId].
    fn effect_definition(&mut self, effect: &cst::EffectDefinition) {
        for declaration in &effect.body {
            let name_id = declaration.name;
            let top_level_name = TopLevelName::new(self.top_level_id, name_id);

            // Skip non-function operations — unusual, treated as values elsewhere.
            let generalized = self.types.get_generalized(name_id);
            self.set_generics_in_scope(generalized);
            let typ = self.convert_type(generalized, None);
            let Type::Function(fn_type) = &typ else { continue };
            let fn_type = fn_type.clone();

            let name = self.context()[name_id].clone();
            let generic_count = self.generics_in_scope.len() as u32;
            let typ_for_closure = typ.clone();

            let id = self.new_definition(name, Some(name_id), generic_count, typ_for_closure, |this| {
                let param_values: Vec<Value> = fn_type
                    .parameters
                    .iter()
                    .enumerate()
                    .map(|(i, pt)| {
                        this.push_parameter(pt.clone());
                        Value::Parameter(BlockId::ENTRY_BLOCK, i as u32)
                    })
                    .collect();

                // Self-reference: the operation's own DefinitionId is the effect-op tag.
                let self_id = this.current_function.as_ref().unwrap().id;
                let perform = Instruction::Perform { effect_op: self_id, arguments: param_values };
                let result = this.push_instruction(perform, fn_type.return_type.clone());
                this.terminate_block(TerminatorInstruction::Return(result));
            });
            self.name_to_id.insert(top_level_name, id);
        }
    }

    fn reference(&mut self, reference: &cst::Reference) -> Value {
        let rhs = reference.rhs;
        let context = self.context();

        // If the RHS is a locally mutable variable, its value in local_variables is already the
        // StackAlloc pointer — return it directly as the reference.
        if let cst::Expr::Variable(path_id) = &context[rhs] {
            let path_id = *path_id;
            if let Some(Origin::Local(name)) = context.path_origin(path_id) {
                if self.mutable_locals.contains(&name) {
                    return *self
                        .local_variables
                        .get(&name)
                        .expect("mutable local variable not found in local_variables");
                }
            }
        }

        // For all other cases (non-mutable local, reference of reference, temporary):
        // evaluate the expression and allocate a new stack slot for it.
        let value = self.expression(rhs);
        self.push_instruction(Instruction::StackAlloc(value), Type::POINTER)
    }

    fn lhs_as_pointer(&mut self, lhs: ExprId) -> Value {
        let context = self.context();
        let lhs_kind = match &context[lhs] {
            cst::Expr::Variable(path_id) => {
                let path_id = *path_id;
                match context.path_origin(path_id) {
                    Some(Origin::Local(name)) => LhsKind::LocalVar(name),
                    _ => LhsKind::Other,
                }
            },
            cst::Expr::Call(call) => LhsKind::DerefCall(call.arguments[0].expr),
            cst::Expr::TypeAnnotation(ta) => LhsKind::Annotation(ta.lhs),
            cst::Expr::MemberAccess(ma) => LhsKind::FieldAccess(ma.object, lhs),
            _ => LhsKind::Other,
        };
        match lhs_kind {
            LhsKind::LocalVar(name) => {
                *self.local_variables.get(&name).expect("lhs_as_pointer: mutable local variable not found")
            },
            LhsKind::DerefCall(ptr_expr) => self.expression(ptr_expr),
            LhsKind::Annotation(inner) => self.lhs_as_pointer(inner),
            LhsKind::FieldAccess(object_expr, field_expr) => {
                let struct_ptr = self.lhs_as_pointer(object_expr);
                let index = self.context().member_access_index(field_expr).unwrap_or(u32::MAX);

                // If the object has a reference type, use the inner type for GEP.
                let struct_type = self.types.result.maps.expr_types[&object_expr].follow(&self.types.bindings);
                let struct_type = if let Some(element) = struct_type.reference_or_pointer_element(&self.types.bindings)
                {
                    self.convert_type(element, None)
                } else {
                    self.convert_type(struct_type, None)
                };
                self.push_instruction(Instruction::GetFieldPtr { struct_ptr, struct_type, index }, Type::POINTER)
            },
            LhsKind::Other => todo!("unhandled assignment LHS"),
        }
    }

    fn assignment(&mut self, assignment: &cst::Assignment) -> Value {
        let pointer = self.lhs_as_pointer(assignment.lhs);

        let value = if let Some((_, op_expr)) = assignment.op {
            // Compound assignment: load current value, apply operator, then store.
            // The LHS is evaluated only once via lhs_as_pointer above.
            let value_type = self.compound_assign_value_type(assignment.lhs);
            let current = self.push_instruction(Instruction::Deref(pointer), value_type.clone());
            let rhs = self.expression(assignment.rhs);

            // Resolve the operator function through normal trait dispatch
            let function = self.expression(op_expr);
            let instruction = if self.type_of_value(&function).is_closure() {
                Instruction::CallClosure { closure: function, arguments: vec![current, rhs] }
            } else {
                Instruction::Call { function, arguments: vec![current, rhs] }
            };
            self.push_instruction(instruction, value_type)
        } else {
            self.expression(assignment.rhs)
        };

        self.push_instruction(Instruction::Store { pointer, value }, Type::UNIT);
        Value::Unit
    }

    /// Get the value type for the LHS of a compound assignment.
    /// If the LHS has a reference type, returns the inner element type.
    fn compound_assign_value_type(&self, lhs: ExprId) -> Type {
        let lhs_type = &self.types.result.maps.expr_types[&lhs];

        match lhs_type.reference_element(&self.types.bindings) {
            Some((_, element)) => self.convert_type(&element, None),
            None => self.convert_type(&lhs_type, None),
        }
    }

    fn constructor(&mut self, constructor: &cst::Constructor, expr: ExprId) -> Value {
        // Side-effects are executed in source order but the type must
        // be packed in declaration order. So re-order fields afterward.
        let mut fields = mapvec(&constructor.fields, |(name, field)| (*name, self.expression(*field)));

        // We must be careful here so that we can still produce MIR even if type-checking failed
        let no_order = BTreeMap::new();
        let field_order = self.context().constructor_field_order(expr).unwrap_or(&no_order);
        fields.sort_unstable_by_key(|(name, _)| field_order.get(name).unwrap_or(&0));

        let fields = mapvec(fields, |(_name, value)| value);
        let tuple_type = Type::Tuple(Arc::new(mapvec(&fields, |value| self.type_of_value(value))));

        self.push_instruction(Instruction::MakeTuple(fields), tuple_type)
    }

    fn quoted(&self, _quoted: &cst::Quoted) -> Value {
        unreachable!("Should never convert a Quoted expr to mir")
    }

    fn return_(&mut self, returned_expression: ExprId) -> Value {
        let value = self.expression(returned_expression);
        self.terminate_block(TerminatorInstruction::Return(value));
        // TODO: We'll need to try to filter these return blocks from
        // matches & ifs, and potentially check for instructions after returns.
        Value::Error
    }

    fn extern_(&mut self, extern_: &cst::Extern, id: ExprId) -> Value {
        let typ = self.convert_expr_type(id);
        self.push_instruction(Instruction::Extern(extern_.name.clone()), typ)
    }

    /// Bind the given value to the given pattern
    fn bind_pattern(&mut self, pattern: PatternId, value: Value) {
        match &self.context()[pattern] {
            cst::Pattern::Error => unreachable!("Error pattern encountered in bind_pattern"),
            cst::Pattern::Variable(name) => {
                // This may be `None` if we had errors during name resolution
                if let Some(origin) = self.context().name_origin(*name) {
                    self.define_variable(origin, value);
                }
            },
            cst::Pattern::Literal(_) => (),
            cst::Pattern::Constructor(_type, arguments) => match self.type_of_value(&value) {
                Type::Union(_variants) => todo!("Deconstruct union"),
                Type::Tuple(fields) => {
                    for (i, (field_type, argument)) in fields.iter().zip(arguments).enumerate() {
                        let instruction = Instruction::IndexTuple { tuple: value, index: i as u32 };
                        let field = self.push_instruction(instruction, field_type.clone());
                        self.bind_pattern(*argument, field);
                    }
                },
                other => unreachable!("Expected tuple or union when deconstructing pattern, found {other}"),
            },
            cst::Pattern::TypeAnnotation(pattern, _) => self.bind_pattern(*pattern, value),
            cst::Pattern::MethodName { type_name: _, item_name } => {
                if let Some(origin) = self.context().name_origin(*item_name) {
                    self.define_variable(origin, value);
                }
            },
        }
    }

    fn finish(self) -> Mir {
        Mir { definitions: self.finished_functions, externals: self.external }.remove_internal_externs()
    }

    /// Sets [self.generics_in_scope] to a map mapping each generic from the given type to a
    /// `[mir::Generic]` used when translating [type_inference::types::Type]s into [crate::mir::Type]s.
    fn set_generics_in_scope(&mut self, definition_type: &TCType) {
        use type_inference::types::Type;
        self.generics_in_scope.clear();

        if let Type::Forall(generics, _) = definition_type {
            for (i, generic) in generics.iter().enumerate() {
                self.generics_in_scope.insert(generic.clone(), Generic(i as u32));
            }
        }
    }

    /// For type definitions we need to define their constructors
    fn type_definition(&mut self, type_definition: &cst::TypeDefinition) {
        let constructors = match &type_definition.body {
            cst::TypeDefinitionBody::Struct(_) => vec![(type_definition.name, None)],
            cst::TypeDefinitionBody::Enum(variants) => {
                if variants.len() == 1 {
                    // `type_body` translates single constructor enums into products, we need to mirror that here
                    vec![(variants[0].0, None)]
                } else {
                    mapvec(variants.iter().enumerate(), |(i, (name, _))| (*name, Some(i.try_into().unwrap())))
                }
            },
            cst::TypeDefinitionBody::Alias(_) | cst::TypeDefinitionBody::Error => return,
        };

        for (constructor_name, tag) in constructors {
            let constructor_type = self.types.get_generalized(constructor_name);
            self.set_generics_in_scope(constructor_type);

            if let crate::type_inference::types::Type::Function(function) = constructor_type.ignore_forall() {
                self.define_type_constructor(constructor_name, constructor_type, &function.parameters, tag)
            } else {
                self.define_type_constructor(constructor_name, constructor_type, &[], tag)
            }
        }

        // Traits are sugar for a struct of function-typed fields, however each "field" is treated
        // as a function by the frontend so we must generate actual functions for each field such
        // that `Cast.cast` is an actual function accepting a `Cast` instance and forwarding the
        // appropriate arguments to the `cast` field.
        if type_definition.is_trait {
            self.define_trait_methods(type_definition);
        }
    }

    fn define_type_constructor(
        &mut self, name_id: NameId, constructor_type: &TCType,
        field_types: &[crate::type_inference::types::ParameterType], tag: Option<u8>,
    ) {
        let top_level_name = TopLevelName::new(self.top_level_id, name_id);
        let name = self.context()[name_id].clone();
        let typ = self.convert_type(constructor_type, None);
        let result_type = match typ.function_return_type() {
            Some(return_type) => return_type.clone(),
            None => typ.clone(),
        };

        // This is the raw union type without the tag
        let raw_union_type = result_type.without_union_tag();

        let generic_count = self.generics_in_scope.len() as u32;
        let is_zero_arg = field_types.is_empty();

        let id = self.new_definition(name, Some(name_id), generic_count, typ, |this| {
            let (fields, field_types): (Vec<Value>, Vec<Type>) = field_types
                .iter()
                .enumerate()
                .map(|(i, field_type)| {
                    let field_type = this.convert_type(&field_type.typ, None);
                    this.push_parameter(field_type.clone());
                    (Value::Parameter(BlockId::ENTRY_BLOCK, i as u32), field_type)
                })
                .unzip();

            let tuple_type = Type::tuple(field_types);
            let mut result = this.push_instruction(Instruction::MakeTuple(fields), tuple_type);

            // If this is a union type we must also add the tag and cast to the union type
            if let Some(tag) = tag {
                let raw_union_type = raw_union_type.unwrap_or_else(|| {
                    let name = this.context()[name_id].clone();
                    panic!("Failed to unwrap raw union type. Full result type is: {result_type} for constructor {name}")
                });
                let casted = this.push_instruction(Instruction::Transmute(result), raw_union_type);
                let fields_with_tag = vec![Value::tag_value(tag), casted];
                result = this.push_instruction(Instruction::MakeTuple(fields_with_tag), result_type);
            }

            // 0-arg constructors are global constant values; use Result terminator so
            // is_global() returns true and validation treats them as globals (not functions).
            if is_zero_arg {
                this.terminate_block(TerminatorInstruction::Result(result));
            } else {
                this.terminate_block(TerminatorInstruction::Return(result));
            }
        });
        self.name_to_id.insert(top_level_name, id);
    }

    fn define_trait_methods(&mut self, type_definition: &cst::TypeDefinition) {
        if let cst::TypeDefinitionBody::Struct(fields) = &type_definition.body {
            let constructor_type = self.types.get_generalized(type_definition.name);
            self.set_generics_in_scope(constructor_type);
            let generic_count = self.generics_in_scope.len() as u32;
            let constructor_mir_type = self.convert_type(constructor_type, None);

            // The struct's type is the return type of the constructor
            let struct_type =
                constructor_mir_type.function_return_type().cloned().unwrap_or_else(|| constructor_mir_type.clone());

            // The field types are the parameter types of the constructor
            let field_mir_types: Vec<Type> =
                if let Type::Function(fn_type) = &constructor_mir_type { fn_type.parameters.clone() } else { vec![] };

            for (i, (field_name_id, _)) in fields.iter().enumerate() {
                let Some(field_type) = field_mir_types.get(i) else { continue };

                // Only generate wrappers for fields whose type is a function or closure (all trait methods)
                // TODO: We should still generate wrappers for other types
                let (value_param_types, return_type) = match field_type {
                    Type::Function(fn_type) => {
                        // Plain function (no captured environment)
                        (fn_type.parameters.clone(), fn_type.return_type.clone())
                    },
                    Type::Tuple(tuple_fields) if tuple_fields.len() == 2 => {
                        // Closure: Type::Tuple([fn(value_args..., env) -> ret, env])
                        let Type::Function(fn_type) = &tuple_fields[0] else { continue };
                        // Strip the trailing env parameter to get the value params
                        let n = fn_type.parameters.len().saturating_sub(1);
                        (fn_type.parameters[..n].to_vec(), fn_type.return_type.clone())
                    },
                    _ => continue,
                };

                // Wrapper: fn <value_args...> (struct_type) -> return_type
                let mut wrapper_params = value_param_types;
                wrapper_params.push(struct_type.clone());
                let wrapper_type = Type::Function(Arc::new(super::FunctionType {
                    parameters: wrapper_params.clone(),
                    environment: Type::NO_CLOSURE_ENV,
                    return_type: return_type.clone(),
                }));

                let name = self.context()[*field_name_id].clone();
                let top_level_name = TopLevelName::new(self.top_level_id, *field_name_id);
                let field_type_clone = field_type.clone();
                let field_is_closure = field_type_clone.is_closure();
                let n_params = wrapper_params.len();

                let id = self.new_definition(name.clone(), Some(*field_name_id), generic_count, wrapper_type, |this| {
                    for pt in &wrapper_params {
                        this.push_parameter(pt.clone());
                    }
                    // Last parameter is the struct (implicit receiver)
                    let struct_param = Value::Parameter(BlockId::ENTRY_BLOCK, n_params as u32 - 1);
                    // Extract the method (function or closure) stored at field index i inside the struct tuple
                    let extracted = this.push_instruction(
                        Instruction::IndexTuple { tuple: struct_param, index: i as u32 },
                        field_type_clone,
                    );
                    // Call the extracted function/closure with the value arguments (all params except last)
                    let value_args = mapvec(0..n_params - 1, |j| Value::Parameter(BlockId::ENTRY_BLOCK, j as u32));
                    let instruction = if field_is_closure {
                        Instruction::CallClosure { closure: extracted, arguments: value_args }
                    } else {
                        Instruction::Call { function: extracted, arguments: value_args }
                    };
                    let result = this.push_instruction(instruction, return_type.clone());
                    this.terminate_block(TerminatorInstruction::Return(result));
                });
                self.name_to_id.insert(top_level_name, id);
            }
        }
    }
}
