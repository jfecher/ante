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
    lexer::token::{FloatKind, Integer, IntegerKind},
    mir::{
        Block, BlockId, Definition, DefinitionId, FloatConstant, FunctionType, Generic, Instruction, IntConstant, Mir,
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

/// Look up a MIR function by a [TopLevelName]
pub(crate) fn lookup_definition_id(name: &TopLevelName) -> Option<DefinitionId> {
    NAME_IDS.get(name).map(|entry| *entry.value())
}

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
        cst::TopLevelItemKind::TraitDefinition(_) | cst::TopLevelItemKind::EffectDefinition(_) => {
            unreachable!("Traits/effects should be desugared to types")
        },
        cst::TopLevelItemKind::TraitImpl(_) => unreachable!("TraitImpls should be desugared to definitions"),
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

/// True if `typ` is a function whose return type is `Never`. Checked before `convert_type`
/// erases `Never`, since we need the divergence info to emit `Unreachable` after the call.
fn function_returns_never<'a>(mut typ: &'a TCType, bindings: &'a type_inference::types::TypeBindings) -> bool {
    use type_inference::types::PrimitiveType::Never;
    loop {
        typ = typ.follow(bindings);
        match typ {
            TCType::Forall(_, inner) => typ = inner,
            TCType::Function(f) => {
                return matches!(f.return_type.follow(bindings), TCType::Primitive(Never));
            },
            _ => return false,
        }
    }
}

/// Whether a top-level item is a trait, an effect, or neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AbilityKind {
    NotAbility,
    Trait,
    Effect,
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

    /// Cache of whether/how a given top-level item is an ability definition.
    ///
    /// For each Call we have to decide if it dispatches through an ability so we
    /// can potentially issue a Perform instead. This cache avoids re-issuing a
    /// GetItemRaw query for every call site.
    ability_defs: FxHashMap<TopLevelId, AbilityKind>,

    /// Position of each effect op within its ability's body. Populated as we encounter
    /// effect operations during MIR building and propagated to [Mir::preserved_op_indices]
    /// at the end so [crate::mir::effects::effect_lowering] can look up the slot of an op
    /// in the cap tuple without re-walking the ability declaration.
    effect_op_indices: FxHashMap<DefinitionId, u32>,

    /// This function's own capability values, keyed by concrete effect. Swapped like
    /// `local_variables` across lambda boundaries.
    capabilities: FxHashMap<TCType, Value>,

    /// This function's own capability-bundle parameter for its open effect-row tail, if any.
    capability_bundle: Option<Value>,
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
            ability_defs: Default::default(),
            effect_op_indices: Default::default(),
            capabilities: Default::default(),
            capability_bundle: None,
        }
    }

    /// Panics if there is no current function.
    fn current_function(&mut self) -> &mut Definition {
        self.current_function.as_mut().unwrap()
    }

    fn type_of_value(&self, value: &Value) -> Type {
        self.current_function.as_ref().unwrap().type_of_value(value, &self.external, &self.finished_functions)
    }

    /// Panics if there is no current function.
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
    fn push_instruction(&mut self, instruction: Instruction, result_type: Type) -> Value {
        // Check if the block is already sealed and ignore any instructions if so
        if self.current_block().terminator.is_some() {
            return Value::Unit;
        }
        let current_block = self.current_block;
        let function = self.current_function();
        let id = function.instructions.push(instruction);
        function.instruction_result_types.push_existing(id, result_type);
        function.blocks[current_block].instructions.push(id);
        Value::InstructionResult(id)
    }

    fn push_block(&mut self, parameter_types: Vec<Type>) -> BlockId {
        self.current_function.as_mut().unwrap().blocks.push(Block::new(parameter_types))
    }

    fn push_block_no_params(&mut self) -> BlockId {
        self.push_block(Vec::new())
    }

    fn switch_to_block(&mut self, block: BlockId) {
        self.current_block = block;
    }

    fn push_parameter(&mut self, parameter_type: Type) {
        self.current_block().parameter_types.push(parameter_type);
    }

    /// Pushes a parameter and returns the `Value` referring to it.
    fn push_capability_parameter(&mut self, parameter_type: Type) -> Value {
        self.push_parameter(parameter_type);
        let index = self.current_block().parameter_types.len() as u32 - 1;
        Value::Parameter(self.current_block, index)
    }

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

    /// If `typ` is a `shared` user-defined type, dereference `value`, otherwise return it unchanged.
    fn deref_if_shared(&mut self, value: Value, typ: &TCType) -> Value {
        match self.shared_inner_layout_of(typ) {
            Some(inner_layout) => self.push_instruction(Instruction::Deref(value), inner_layout),
            None => value,
        }
    }

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
            cst::Expr::Do(_) => unreachable!("Expr::Do should be desugared during GetItem"),
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
            cst::Expr::ArrayLiteral(elements) => self.array_literal(elements, expr),
        }
    }

    fn array_literal(&mut self, elements: &[ExprId], expr: ExprId) -> Value {
        let result_type = self.expr_type(expr);
        let values = elements.iter().map(|e| self.expression(*e)).collect();
        self.push_instruction(Instruction::MakeArray(values), result_type)
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
            Literal::String(x) => self.string_literal(x),
            Literal::Char(x) => Value::Char(*x),
        }
    }

    /// Lower a string literal to the prelude String representation:
    /// `(data: Ptr Char, refcount: Ptr U32, length: U32, offset: U32)`.
    fn string_literal(&mut self, s: &str) -> Value {
        let len = s.len() as u32;

        let mut bytes = Vec::with_capacity(s.len() + 1);
        bytes.extend_from_slice(s.as_bytes());
        bytes.push(0);

        let data_ptr = self.push_instruction(Instruction::MakeBytes(bytes), Type::POINTER);
        let null_rc = self.push_instruction(Instruction::Transmute(Value::Integer(IntConstant::Usz(0))), Type::POINTER);
        let length = Value::Integer(IntConstant::U32(len));
        let offset = Value::Integer(IntConstant::U32(0));

        self.push_instruction(Instruction::MakeTuple(vec![data_ptr, null_rc, length, offset]), Type::string())
    }

    fn integer(value: Integer, kind: IntegerKind) -> Value {
        let m = value.magnitude;
        match kind {
            IntegerKind::I8 => {
                Value::Integer(IntConstant::I8(if value.negative { (m as i8).wrapping_neg() } else { m as i8 }))
            },
            IntegerKind::I16 => {
                Value::Integer(IntConstant::I16(if value.negative { (m as i16).wrapping_neg() } else { m as i16 }))
            },
            IntegerKind::I32 => {
                Value::Integer(IntConstant::I32(if value.negative { (m as i32).wrapping_neg() } else { m as i32 }))
            },
            IntegerKind::I64 => {
                Value::Integer(IntConstant::I64(if value.negative { (m as i64).wrapping_neg() } else { m as i64 }))
            },
            IntegerKind::Isz => {
                Value::Integer(IntConstant::Isz(if value.negative { (m as isize).wrapping_neg() } else { m as isize }))
            },
            IntegerKind::U8 => Value::Integer(IntConstant::U8(m as u8)),
            IntegerKind::U16 => Value::Integer(IntConstant::U16(m as u16)),
            IntegerKind::U32 => Value::Integer(IntConstant::U32(m as u32)),
            IntegerKind::U64 => Value::Integer(IntConstant::U64(m)),
            IntegerKind::Usz => Value::Integer(IntConstant::Usz(m as usize)),
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
        if let Value::Definition(id) = value
            && let Some(bindings) = self.types.result.context.get_instantiation(path_id)
        {
            let typ = self.convert_path_type(path_id);
            let bindings = Arc::new(mapvec(bindings, |typ| self.convert_type(typ, None)));
            let instruction = Instruction::Instantiate(id, bindings);
            value = self.push_instruction(instruction, typ);
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
            self.set_generics_in_scope(typ);
        }

        let previous_state = self.is_non_function_global(definition).then(|| {
            let generic_count = self.generics_in_scope.len() as u32;
            let typ = self.convert_pattern_type(definition.pattern);
            self.start_global(name, name_id, generic_count, typ)
        });

        let value = match &self.context()[definition.rhs] {
            cst::Expr::Lambda(lambda) => {
                let name = self.try_find_name(definition.pattern).map(|(name, _)| name);
                self.lambda(lambda, name_id, name, definition.rhs, is_global)
            },
            _ => self.expression(definition.rhs),
        };

        self.bind_pattern(definition.pattern, value);

        // TODO: Globals should probably never be stack allocated
        if definition.mutable {
            let mut names = Vec::new();
            self.collect_pattern_names(definition.pattern, &mut names);
            for name in names {
                let raw = *self.local_variables.get(&name).expect("var binding missing local after bind_pattern");
                let alloc = self.push_instruction(Instruction::StackAlloc(raw), Type::POINTER);
                self.local_variables.insert(name, alloc);
                self.mutable_locals.insert(name);
            }
        }

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
            let value = self.expression(object_expr);
            let tuple = self.deref_if_shared(value, &object_type);
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

        let diverges = self.callee_diverges(call.function);
        let result_type = if diverges { Type::UNIT } else { self.expr_type(id) };

        // Trait & effect method calls dispatch through `IndexTuple cap op_index + CallClosure`,
        // so we don't depend on the method's own DefinitionId having a globally consistent function
        // type for its environment parameter.
        //
        // For traits, it is an implicit argument at the end of `call.arguments`.
        // For effects, there is no argument, it comes from this function's capability parameter(s) instead.
        if let cst::Expr::Variable(path_id) = &self.context()[call.function]
            && let Some((effect_op, op_index, kind)) = self.try_resolve_ability_method(*path_id)
        {
            let path_id = *path_id;
            let arguments = mapvec(&call.arguments, |expr| self.expression(expr.expr));
            return match kind {
                AbilityKind::Trait => self.emit_trait_method_call(effect_op, op_index, arguments, result_type, diverges),
                AbilityKind::Effect => {
                    self.emit_effect_op_call(path_id, effect_op, op_index, arguments, result_type, diverges)
                },
                AbilityKind::NotAbility => unreachable!(),
            };
        }

        let function = self.expression(call.function);
        let expected_parameter_types = self.callee_declared_parameter_types(call.function);
        let mut arguments = mapvec(call.arguments.iter().enumerate(), |(i, expr)| {
            let expected = expected_parameter_types.as_ref().and_then(|types| types.get(i));
            self.adapt_argument(expr.expr, expected, call.function)
        });
        self.append_capability_arguments(call.function, &mut arguments);

        let instruction = if self.type_of_value(&function).is_closure() {
            Instruction::CallClosure { closure: function, arguments }
        } else {
            Instruction::Call { function, arguments }
        };

        let value = self.push_instruction(instruction, result_type);
        if diverges {
            self.terminate_block(TerminatorInstruction::Unreachable);
        }
        value
    }

    /// The callee's effects row, in declaration order.
    fn callee_effects_row(&self, callee_expr: ExprId) -> (Vec<TCType>, Option<TCType>) {
        // Using `path_types` because `expr_types` for a `Variable` may be overwritten with the
        // post-unification expected type rather than the callee's own instantiated type.
        let typ = match &self.context()[callee_expr] {
            cst::Expr::Variable(path_id) => self.types.result.maps.path_types[path_id].follow(&self.types.bindings),
            _ => self.types.result.maps.expr_types[&callee_expr].follow(&self.types.bindings),
        };
        let TCType::Function(function_type) = typ else {
            panic!("callee_effects_row: callee is not a function type");
        };
        let TCType::Effects(list, tail) = function_type.effects.follow(&self.types.bindings) else {
            panic!("callee_effects_row: callee has no effects row");
        };
        (list.as_ref().clone(), tail.as_deref().cloned())
    }

    /// Appends capability arguments for a call to `callee_expr`, sourced from this function's
    /// capabilities in the callee's declaration order.
    fn append_capability_arguments(&mut self, callee_expr: ExprId, arguments: &mut Vec<Value>) {
        let (list, tail) = self.callee_effects_row(callee_expr);

        for effect_ty in &list {
            arguments.push(self.capability_for(effect_ty));
        }

        let Some(tail_ty) = tail else { return };
        match tail_ty.follow_all(&self.types.bindings) {
            TCType::Effects(concrete_list, None) if concrete_list.is_empty() => {},
            TCType::Effects(concrete_list, None) => {
                let fields = mapvec(concrete_list.iter(), |effect_ty| self.capability_for(effect_ty));
                let followed = TCType::Effects(concrete_list, None);
                let bundle_type = self.convert_type(&followed, None);
                let bundle = self.push_instruction(Instruction::MakeTuple(fields), bundle_type);
                arguments.push(bundle);
            },
            // Still open from the caller's perspective: forward our own bundle. Not captured for a suppressed lambda.
            _ => {
                if let Some(bundle) = self.capability_bundle {
                    arguments.push(bundle);
                }
            },
        }
    }

    /// Canonicalizes a concrete effect type to a stable capabilities map key: fully resolved,
    /// with a closed singleton row like `can Log` unwrapped to just `Log`.
    fn capability_key(&self, effect_ty: &TCType) -> TCType {
        let followed = effect_ty.follow_all(&self.types.bindings);
        match &followed {
            TCType::Effects(list, None) if list.len() == 1 => list[0].clone(),
            _ => followed,
        }
    }

    /// Looks up a concrete effect's capability value in this function's own scope.
    fn capability_for(&self, effect_ty: &TCType) -> Value {
        let key = self.capability_key(effect_ty);
        *self.capabilities.get(&key).unwrap_or_else(|| {
            panic!(
                "no capability for effect {key:?} in scope at this call site \
                 - handler branches/bodies and trait-method implementations can't receive \
                 capabilities besides the one a handler body/branch handles"
            )
        })
    }

    /// Capabilities a handler body/branch needs beyond what it handles, to thread through its
    /// closure environment since it can't gain new capability parameters.
    fn suppressed_lambda_needed_capabilities(
        &self, tc_effects: &TCType, handle_body_handler_name: Option<NameId>,
    ) -> Vec<TCType> {
        let TCType::Effects(list, tail) = tc_effects else { return Vec::new() };
        // The tail is often an unresolved unification variable at this point even though it's
        // actually bound to a concrete row; fully resolve it to see the effects it's hiding.
        let mut effects: Vec<TCType> = list.as_ref().clone();
        if let Some(tail) = tail
            && let TCType::Effects(tail_list, _) = tail.follow_all(&self.types.bindings)
        {
            effects.extend(tail_list.as_ref().clone());
        }

        // A body excludes its own handled effect(s) (sourced from the placeholder instead);
        // a branch excludes nothing since a reperform legitimately targets the next outer handler.
        let excluded_keys: Vec<TCType> = match handle_body_handler_name {
            Some(handler_name) => {
                let h_tc_type = self.types.result.maps.name_types[&handler_name].follow_all(&self.types.bindings);
                let h_list = match h_tc_type {
                    TCType::Effects(h_list, _) => h_list.as_ref().clone(),
                    other => vec![other],
                };
                h_list.iter().map(|t| self.capability_key(t)).collect()
            },
            None => Vec::new(),
        };

        effects.iter().map(|t| self.capability_key(t)).filter(|key| !excluded_keys.contains(key)).collect()
    }

    /// Extends a lambda's environment type with a trailing field per needed capability.
    fn extend_environment_with_capabilities(&self, full_type: Type, needed_capability_keys: &[TCType]) -> Type {
        if needed_capability_keys.is_empty() {
            return full_type;
        }
        let Type::Function(ft) = &full_type else { unreachable!("Lambda does not have a function type") };
        let mut env_fields: Vec<Type> = match &ft.environment {
            Type::Tuple(fields) => (**fields).clone(),
            _ => Vec::new(),
        };
        env_fields.extend(needed_capability_keys.iter().map(|key| self.effect_capability_tuple_type_of(key)));
        Type::Function(Arc::new(FunctionType {
            parameters: ft.parameters.clone(),
            environment: Type::tuple(env_fields),
            return_type: ft.return_type.clone(),
        }))
    }

    /// A top-level function reference's declared un-instantiated parameter types, used by
    /// [Self::adapt_argument] to detect an effects-shape mismatch.
    fn callee_declared_parameter_types(&self, callee_expr: ExprId) -> Option<Vec<TCType>> {
        let cst::Expr::Variable(path_id) = &self.context()[callee_expr] else { return None };
        let Some(Origin::TopLevelDefinition(name)) = self.context().path_origin(*path_id) else { return None };
        let checked = TypeCheck(name.top_level_item).get(self.compiler);
        let declared = checked.get_generalized(name.local_name_id).ignore_forall().clone();
        let TCType::Function(function_type) = declared else { return None };
        Some(function_type.parameters.iter().map(|p| p.typ.clone()).collect())
    }

    /// Evaluates `arg_expr`, adapting it if it's a bare reference to a top-level function with
    /// real capability parameters being passed where `expected` has an open effect-row tail
    /// expecting the single opaque bundle-parameter convention instead.
    fn adapt_argument(&mut self, arg_expr: ExprId, expected: Option<&TCType>, callee_expr: ExprId) -> Value {
        let value = self.expression(arg_expr);
        let Some(expected) = expected else { return value };
        let cst::Expr::Variable(path_id) = &self.context()[arg_expr] else { return value };
        let path_id = *path_id;
        let Some(Origin::TopLevelDefinition(name)) = self.context().path_origin(path_id) else { return value };

        let TCType::Function(expected_fn) = expected.follow(&self.types.bindings) else { return value };
        let TCType::Effects(expected_list, Some(_)) = expected_fn.effects.follow(&self.types.bindings) else {
            return value;
        };
        if !expected_list.is_empty() {
            // A mixed concrete+polymorphic expected row is out of scope for this adapter.
            return value;
        }

        let checked = TypeCheck(name.top_level_item).get(self.compiler);
        let declared = checked.get_generalized(name.local_name_id).ignore_forall().clone();
        let TCType::Function(own_fn) = &declared else { return value };
        let TCType::Effects(own_list, own_tail) = own_fn.effects.follow(&self.types.bindings) else { return value };
        if own_list.is_empty() || own_tail.is_some() {
            // Pure, or already row-polymorphic itself: already matches the bundle convention.
            return value;
        }
        let own_list = own_list.clone();

        self.build_capability_adapter(path_id, expected, own_fn, &own_list, callee_expr)
    }

    /// Effects normally translate to 1 capability argument per effect, but effect polymorphic
    /// functions have a single generic parameter for the generic effect `e`. If `e` is
    /// instantiated to multiple effects, we'll have to build an adapter function so we can pass a
    /// function with e.g. 5 effect parameters to one expecting only 2 + a generic effect. The 3
    /// remaining effects in this case get bundled in a tuple and passed into `e`'s corresponding parameter.
    fn build_capability_adapter(
        &mut self, path_id: PathId, expected: &TCType, own_fn: &type_inference::types::FunctionType,
        own_list: &[TCType], callee_expr: ExprId,
    ) -> Value {
        let TCType::Function(expected_fn) = expected.follow(&self.types.bindings) else {
            unreachable!("checked by caller");
        };

        // Reuse the outer call's own effects-row resolution instead of re-deriving the tail from
        // `expected` independently. The outer call's instantiation bindings resolve the callee's
        // own row generic, which isn't guaranteed to share identity with the generic on this
        // specific parameter's type.
        let (_, outer_tail) = self.callee_effects_row(callee_expr);
        let resolved_tail = outer_tail
            .unwrap_or_else(|| panic!("capability adapter: callee has no open effects row to bridge into"))
            .follow_all(&self.types.bindings);
        let bundle_type = self.convert_type(&resolved_tail, None);
        let return_type = self.convert_type(&own_fn.return_type, None);
        let surface_types: Vec<Type> = mapvec(expected_fn.parameters.iter(), |p| self.convert_type(&p.typ, None));

        // The bundle is a real positional parameter (this adapter is called directly, not through
        // a captured environment), so it must be reflected in the declared type too.
        let environment = match self.convert_type(expected, None) {
            Type::Function(ft) => ft.environment.clone(),
            _ => Type::NO_CLOSURE_ENV,
        };
        let mut declared_parameters = surface_types.clone();
        declared_parameters.push(bundle_type.clone());
        let declared_type =
            Type::Function(Arc::new(FunctionType { parameters: declared_parameters, environment, return_type: return_type.clone() }));

        let generics_count = self.generics_in_scope.len() as u32;
        let old_scope = std::mem::take(&mut self.local_variables);
        let old_mutables = std::mem::take(&mut self.mutable_locals);
        let old_capabilities = std::mem::take(&mut self.capabilities);
        let old_capability_bundle = self.capability_bundle.take();

        let surface_count = surface_types.len() as u32;
        let own_list = own_list.to_vec();
        let name = Arc::new("capability_adapter".to_string());
        let id = self.new_definition(name.clone(), None, generics_count, declared_type.clone(), |this| {
            for typ in &surface_types {
                this.push_parameter(typ.clone());
            }
            let bundle = this.push_capability_parameter(bundle_type.clone());

            let mut real_arguments: Vec<Value> =
                (0..surface_count).map(|i| Value::Parameter(this.current_block, i)).collect();
            for (index, effect_ty) in own_list.iter().enumerate() {
                let cap_type = this.effect_capability_tuple_type_of(effect_ty);
                let cap = this.push_instruction(Instruction::IndexTuple { tuple: bundle, index: index as u32 }, cap_type);
                real_arguments.push(cap);
            }

            let function_value = this.variable(path_id);
            let instruction = if this.type_of_value(&function_value).is_closure() {
                Instruction::CallClosure { closure: function_value, arguments: real_arguments }
            } else {
                Instruction::Call { function: function_value, arguments: real_arguments }
            };
            let result = this.push_instruction(instruction, return_type.clone());
            this.terminate_block(TerminatorInstruction::Return(result));
        });

        self.local_variables = old_scope;
        self.mutable_locals = old_mutables;
        self.capabilities = old_capabilities;
        self.capability_bundle = old_capability_bundle;

        self.make_definition_value(id, name, declared_type)
    }

    /// Emits the `IndexTuple cap op_index + CallClosure` sequence for a trait or effect method call.
    fn emit_indexed_method_call(
        &mut self, cap_value: Value, op_index: u32, arguments: Vec<Value>, result_type: Type, diverges: bool,
    ) -> Value {
        let cap_type = self.type_of_value(&cap_value);
        let method_type = match &cap_type {
            Type::Tuple(fields) => fields.get(op_index as usize).cloned().unwrap_or_else(|| {
                panic!("ability method call: cap tuple has no slot {op_index} (cap_type = {cap_type})")
            }),
            // Single-method abilities can collapse to a bare function when type inference
            // strips the surrounding tuple. Fall back to the call's expected result-type wiring.
            _ => cap_type.clone(),
        };

        let method =
            self.push_instruction(Instruction::IndexTuple { tuple: cap_value, index: op_index }, method_type.clone());

        let instruction = if method_type.is_closure() {
            Instruction::CallClosure { closure: method, arguments }
        } else {
            Instruction::Call { function: method, arguments }
        };

        let value = self.push_instruction(instruction, result_type);
        if diverges {
            self.terminate_block(TerminatorInstruction::Unreachable);
        }
        value
    }

    /// `arguments` must contain the operation args followed by the implicit dictionary value
    fn emit_trait_method_call(
        &mut self, effect_op: DefinitionId, op_index: u32, mut arguments: Vec<Value>, result_type: Type, diverges: bool,
    ) -> Value {
        self.effect_op_indices.insert(effect_op, op_index);
        let cap_value = arguments.pop().expect("trait method call: no implicit cap argument");
        self.emit_indexed_method_call(cap_value, op_index, arguments, result_type, diverges)
    }

    fn emit_effect_op_call(
        &mut self, path_id: PathId, effect_op: DefinitionId, op_index: u32, arguments: Vec<Value>, result_type: Type,
        diverges: bool,
    ) -> Value {
        self.effect_op_indices.insert(effect_op, op_index);
        let effect_type = self.effect_type_of_op(path_id);
        let cap_value = *self.capabilities.get(&effect_type).unwrap_or_else(|| {
            panic!(
                "no capability for effect {effect_type:?} in scope at this call site \
                 (handler branches/bodies can't receive capabilities besides the one they handle)"
            )
        });
        self.emit_indexed_method_call(cap_value, op_index, arguments, result_type, diverges)
    }

    /// The concrete effect an operation reference performs, read off its own singleton effects row.
    fn effect_type_of_op(&self, path_id: PathId) -> TCType {
        let typ = self.types.result.maps.path_types[&path_id].follow(&self.types.bindings);
        let TCType::Function(function_type) = typ else {
            panic!("effect_type_of_op: operation is not a function type");
        };
        let TCType::Effects(list, _) = function_type.effects.follow(&self.types.bindings) else {
            panic!("effect_type_of_op: operation has no effects row");
        };
        let effect_type = list.first().unwrap_or_else(|| panic!("effect_type_of_op: operation has an empty effects row"));
        self.capability_key(effect_type)
    }

    /// Resolves a concrete effect `Type` to its capability tuple type.
    fn effect_capability_tuple_type_of(&self, effect_type: &TCType) -> Type {
        self.convert_context().effect_capability_tuple_type_of(effect_type)
    }

    /// Whether a top-level item is a trait, effect, or neither
    fn ability_kind(&mut self, item: TopLevelId) -> AbilityKind {
        if let Some(kind) = self.ability_defs.get(&item).copied() {
            return kind;
        }
        let (cst_item, _) = GetItemRaw(item).get(self.compiler);
        let kind = match &cst_item.kind {
            cst::TopLevelItemKind::TraitDefinition(_) => AbilityKind::Trait,
            cst::TopLevelItemKind::EffectDefinition(_) => AbilityKind::Effect,
            _ => AbilityKind::NotAbility,
        };
        self.ability_defs.insert(item, kind);
        kind
    }

    /// Like [Self::try_resolve_effect_op] but also returns the op's position within its
    /// ability's body, suitable for `IndexTuple` against the cap value.
    fn try_resolve_ability_method(&mut self, path: PathId) -> Option<(DefinitionId, u32, AbilityKind)> {
        let origin = self.context().path_origin(path)?;
        let Origin::TopLevelDefinition(name) = origin else { return None };

        let kind = self.ability_kind(name.top_level_item);
        if kind == AbilityKind::NotAbility {
            return None;
        }

        let (item, _) = GetItemRaw(name.top_level_item).get(self.compiler);
        if let cst::TopLevelItemKind::TraitDefinition(effect) | cst::TopLevelItemKind::EffectDefinition(effect) = &item.kind
            && let Some(op_index) = effect.body.iter().position(|d| d.name == name.local_name_id)
        {
            let id = self.get_definition_id(&name);
            return Some((id, op_index as u32, kind));
        }
        None
    }

    /// Looks up the callee via `path_types` rather than `expr_types`: the latter is overwritten
    /// with the post-unification expected type, which may have erased `Never`.
    fn callee_diverges(&self, callee_expr: ExprId) -> bool {
        // TODO: Test whether this and `Never` handling in MIR in general holds up for generic functions.
        // if a return type is a generic bound to Never by monomorphization we won't find it here.
        let cst::Expr::Variable(path_id) = &self.context()[callee_expr] else {
            return false;
        };
        let Some(typ) = self.types.result.maps.path_types.get(path_id) else {
            return false;
        };
        function_returns_never(typ, &self.types.bindings)
    }

    /// If `path` resolves to an effect-operation declaration, return its
    /// [DefinitionId]. Otherwise return None.
    ///
    /// We use a cache to avoid hitting inc-complete's locks for every
    /// call instruction but this can likely be improved further.
    fn try_resolve_effect_op(&mut self, path: PathId) -> Option<DefinitionId> {
        let origin = self.context().path_origin(path)?;
        let Origin::TopLevelDefinition(name) = origin else { return None };

        if self.ability_kind(name.top_level_item) == AbilityKind::NotAbility {
            return None;
        }

        // Cold path: this TopLevelId is an effect definition. Verify that the
        // referenced NameId is one of its ops. Paths can also resolve to the
        // ability's type-constructor name or to a non-function field on the
        // ability (sub-ability reference)
        let (item, _) = GetItemRaw(name.top_level_item).get(self.compiler);
        if let cst::TopLevelItemKind::TraitDefinition(effect) | cst::TopLevelItemKind::EffectDefinition(effect) = &item.kind
            && let Some(op_index) = effect.body.iter().position(|d| d.name == name.local_name_id)
        {
            let id = self.get_definition_id(&name);
            self.effect_op_indices.insert(id, op_index as u32);
            return Some(id);
        }
        None
    }

    fn try_find_name(&self, pattern: PatternId) -> Option<(Name, NameId)> {
        match &self.context()[pattern] {
            cst::Pattern::Error => None,
            cst::Pattern::Literal(_) => None,
            cst::Pattern::Constructor(..) => None,
            cst::Pattern::TypeAnnotation(pattern, _) => self.try_find_name(*pattern),
            cst::Pattern::Variable(name)
            | cst::Pattern::MethodName { item_name: name, .. }
            | cst::Pattern::Alias(name, _) => Some((self.context()[*name].clone(), *name)),
            cst::Pattern::Or(alts) => alts.first().and_then(|alt| self.try_find_name(*alt)),
        }
    }

    fn collect_pattern_names(&self, pattern: PatternId, out: &mut Vec<NameId>) {
        match &self.context()[pattern] {
            cst::Pattern::Error | cst::Pattern::Literal(_) => (),
            cst::Pattern::Variable(name) | cst::Pattern::MethodName { item_name: name, .. } => {
                out.push(*name);
            },
            cst::Pattern::TypeAnnotation(pattern, _) => self.collect_pattern_names(*pattern, out),
            cst::Pattern::Constructor(_, arguments) => {
                for argument in arguments.clone() {
                    self.collect_pattern_names(argument, out);
                }
            },
            // Each alternative of a valid OR-pattern binds the same names, so we only
            // need to inspect the first alternative.
            cst::Pattern::Or(alts) => {
                if let Some(alt) = alts.first() {
                    self.collect_pattern_names(*alt, out);
                }
            },
            cst::Pattern::Alias(name, inner) => {
                out.push(*name);
                self.collect_pattern_names(*inner, out);
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
        self.lambda_impl(lambda, name_id, name, expr, is_global, None, false)
    }

    /// When `handle_body_handler_name` is `Some(h)`, this lambda is the body of a `handle` expression.
    /// Inject a prelude that loads the capability from the coroutine's user_data and binds it
    /// to `h` in `local_variables` so the body's references to `h` resolve.
    ///
    /// `is_handler_branch` and `handle_body_handler_name` both prevent the addition of capability parameters.
    /// Handler bodies & branches run under the coroutine ABI's fixed arity.
    fn lambda_impl(
        &mut self, lambda: &cst::Lambda, name_id: Option<NameId>, name: Option<Name>, expr: ExprId, is_global: bool,
        handle_body_handler_name: Option<NameId>, is_handler_branch: bool,
    ) -> Value {
        let name = name.unwrap_or_else(|| Arc::new("lambda".to_string()));
        let suppress_capabilities = handle_body_handler_name.is_some() || is_handler_branch;
        let tc_effects = match self.types.result.maps.expr_types[&expr].follow(&self.types.bindings) {
            TCType::Function(tc_function_type) => tc_function_type.effects.follow(&self.types.bindings),
            _ => unreachable!("Lambda does not have a function type"),
        };
        let needed_capability_keys = if suppress_capabilities {
            self.suppressed_lambda_needed_capabilities(&tc_effects, handle_body_handler_name)
        } else {
            Vec::new()
        };

        let full_type = self.extend_environment_with_capabilities(self.convert_expr_type(expr), &needed_capability_keys);
        let Type::Function(function_type) = &full_type else { unreachable!("Lambda does not have a function type") };

        let is_move = self.context().is_move_closure(expr);

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
        let old_capabilities = std::mem::take(&mut self.capabilities);
        let old_capability_bundle = self.capability_bundle.take();
        let needed_capability_values: Vec<Value> = needed_capability_keys
            .iter()
            .map(|key| {
                *old_capabilities.get(key).unwrap_or_else(|| {
                    panic!("handler body/branch needs outer capability {key:?} but it isn't available in the enclosing scope")
                })
            })
            .collect();

        let id = self.new_definition(name.clone(), name_id, generics_count, full_type.clone(), |this| {
            for (i, parameter) in lambda.parameters.iter().enumerate() {
                let parameter_type = &this.types.result.maps.pattern_types[&parameter.pattern];
                this.push_parameter(this.convert_type(parameter_type, None));

                let parameter_value = Value::Parameter(this.current_block, i as u32);
                this.bind_pattern(parameter.pattern, parameter_value);

                if parameter.is_mutable
                    && let Some((_, name_id)) = this.try_find_name(parameter.pattern)
                {
                    let alloc = this.push_instruction(Instruction::StackAlloc(parameter_value), Type::POINTER);
                    this.local_variables.insert(name_id, alloc);
                    this.mutable_locals.insert(name_id);
                }
            }

            let env_is_pointer =
                matches!(function_type.environment, Type::Primitive(crate::mir::PrimitiveType::Pointer));
            let free_vars = this.context().get_closure_environment(expr);
            let has_captures = free_vars.is_some() || !needed_capability_keys.is_empty();
            let needs_env_param = has_captures || env_is_pointer;
            if needs_env_param {
                if let Some(env) = function_type.environment() {
                    this.push_parameter(env.clone());
                }

                if has_captures {
                    let empty_free_vars = BTreeSet::new();
                    let free_vars = free_vars.unwrap_or(&empty_free_vars);
                    let environment = Value::Parameter(this.current_block, lambda.parameters.len() as u32);
                    this.unpack_closure_environment(free_vars.iter().copied(), &needed_capability_keys, environment);

                    // For regular closures, mutable captures are pointers (by reference).
                    if !is_move {
                        for var in free_vars.iter() {
                            if mutable_captures.contains(var) {
                                this.mutable_locals.insert(*var);
                            }
                        }
                    }
                }
            }

            // One trailing parameter per concrete effect in this lambda's own effects row, plus
            // one more opaque bundle parameter if the row has an open, polymorphic tail.
            if !suppress_capabilities
                && let TCType::Effects(list, tail) = &tc_effects
            {
                for effect_ty in list.iter() {
                    let cap_type = this.effect_capability_tuple_type_of(effect_ty);
                    let cap_value = this.push_capability_parameter(cap_type);
                    let key = this.capability_key(effect_ty);
                    this.capabilities.insert(key, cap_value);
                }
                if let Some(tail_ty) = tail {
                    let followed_tail = tail_ty.follow(&this.types.bindings);
                    if matches!(followed_tail, TCType::Generic(_) | TCType::Variable(_)) {
                        let bundle_type = this.convert_type(&followed_tail, None);
                        this.capability_bundle = Some(this.push_capability_parameter(bundle_type));
                    }
                }
            }

            // For a `handle` expression's body lambda, bind `h` to a placeholder
            // [Instruction::Capability]. The lowering passes are responsible for replacing it:
            // [crate::mir::effects::effect_lowering] expands it into a coroutine `user_data`
            // fetch, while [crate::mir::effects::tail_resume_optimization] rewrites it to refer
            // to a directly-built capability tuple.
            if let Some(handler_name) = handle_body_handler_name {
                let h_tc_type = this.types.result.maps.name_types[&handler_name].follow_all(&this.types.bindings);
                let h_type = this.effect_capability_tuple_type_of(&h_tc_type);
                let cap = this.push_instruction(Instruction::Capability, h_type);
                this.local_variables.insert(handler_name, cap);
                let key = this.capability_key(&h_tc_type);
                this.capabilities.insert(key, cap);
            }

            // Pre-populate the self-reference so recursive calls within the body
            // (e.g. `recur` in desugared loop expressions) can resolve via Origin::Local.
            // The entry is discarded automatically when `old_scope` is restored after
            // `new_definition` returns.
            if let Some(self_name_id) = name_id {
                let self_def_id = this.current_function.as_ref().unwrap().id;
                let mut self_value = Value::Definition(self_def_id);
                if needs_env_param {
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
        self.capabilities = old_capabilities;
        self.capability_bundle = old_capability_bundle;

        // Generic lambdas inherit generics from the surrounding context; instantiate manually.
        let mut value = self.make_definition_value(id, name, full_type.clone());
        if !is_global && !self.generics_in_scope.is_empty() {
            let bindings = Arc::new(mapvec(0..self.generics_in_scope.len() as u32, |i| Type::Generic(Generic(i))));
            value = self.push_instruction(Instruction::Instantiate(id, bindings), full_type.clone());
        }
        let free_vars = self.context().get_closure_environment(expr).cloned();
        let env_type = function_type.environment.clone();
        let env_is_pointer = matches!(env_type, Type::Primitive(crate::mir::PrimitiveType::Pointer));
        let has_captures = free_vars.is_some() || !needed_capability_values.is_empty();
        if has_captures || env_is_pointer {
            let environment = if has_captures {
                let free_vars = free_vars.unwrap_or_default();
                self.pack_closure_environment(&free_vars, &needed_capability_values, is_move, &env_type)
            } else {
                // Pointer-env slot with no captures (e.g. an ability impl assigning a plain function):
                // use a null pointer for the env. Transmute from Unit so constant-folding works
                // even when this PackClosure ends up in a global initializer.
                self.push_instruction(Instruction::Transmute(Value::Unit), Type::POINTER)
            };
            value = self.push_instruction(Instruction::PackClosure { function: value, environment }, full_type);
        }
        value
    }

    /// Packs each given variable, plus any needed capability values, into a closure environment.
    /// When `env_type` is a pointer, the capture tuple is heap-allocated (via [Instruction::AllocShared])
    /// and the returned value is the resulting pointer. Otherwise returns the tuple directly.
    fn pack_closure_environment(
        &mut self, free_vars: &BTreeSet<NameId>, capability_values: &[Value], is_move: bool, env_type: &Type,
    ) -> Value {
        assert!(!free_vars.is_empty() || !capability_values.is_empty());

        let mut values = mapvec(free_vars, |var| {
            let value = *self.local_variables.get(var).unwrap();

            if is_move && self.mutable_locals.contains(var) {
                // `move` closures capture mutable variables by value: dereference the
                // StackAlloc pointer to load the current value into the environment.
                let tc_type = &self.types.result.maps.name_types[var];
                let val_type = self.convert_type(tc_type, None);
                self.push_instruction(Instruction::Deref(value), val_type)
            } else {
                value
            }
        });
        values.extend_from_slice(capability_values);
        let types = mapvec(&values, |value| self.type_of_value(value));
        let tuple = self.push_instruction(Instruction::MakeTuple(values), Type::tuple(types));

        if matches!(env_type, Type::Primitive(crate::mir::PrimitiveType::Pointer)) {
            self.push_instruction(Instruction::AllocShared(tuple), Type::POINTER)
        } else {
            tuple
        }
    }

    /// Unpack a closure environment parameter, binding each captured name and needed capability
    /// to its value. When the env value is a `Pointer` (ability-method-style heap env), it is
    /// dereferenced first to recover the capture tuple.
    fn unpack_closure_environment(
        &mut self, free_vars: impl ExactSizeIterator<Item = NameId> + Clone, capability_keys: &[TCType],
        environment: Value,
    ) {
        let free_vars_len = free_vars.len();
        let env_value =
            if matches!(self.type_of_value(&environment), Type::Primitive(crate::mir::PrimitiveType::Pointer)) {
                let mut field_types: Vec<Type> = free_vars
                    .clone()
                    .map(|var| {
                        let tc_type = &self.types.result.maps.name_types[&var];
                        self.convert_type(tc_type, None)
                    })
                    .collect();
                field_types.extend(capability_keys.iter().map(|key| self.effect_capability_tuple_type_of(key)));
                let tuple_type = Type::tuple(field_types);
                self.push_instruction(Instruction::Deref(environment), tuple_type)
            } else {
                environment
            };

        let Type::Tuple(env_fields) = self.type_of_value(&env_value) else { unreachable!() };
        assert_eq!(env_fields.len(), free_vars_len + capability_keys.len());

        for (i, (var, env_field)) in free_vars.zip(env_fields.iter().cloned()).enumerate() {
            let index = Instruction::IndexTuple { tuple: env_value, index: i as u32 };
            let result = self.push_instruction(index, env_field);
            let existing = self.local_variables.insert(var, result);
            assert!(existing.is_none(), "Closure is overwriting values from the outer scope");
        }
        for (i, key) in capability_keys.iter().enumerate() {
            let field_ty = env_fields[free_vars_len + i].clone();
            let index = Instruction::IndexTuple { tuple: env_value, index: (free_vars_len + i) as u32 };
            let result = self.push_instruction(index, field_ty);
            self.capabilities.insert(key.clone(), result);
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
            Instruction::LessSigned(variable, end_value)
        } else {
            Instruction::LessUnsigned(variable, end_value)
        };
        let cond = self.push_instruction(cmp, Type::BOOL);
        self.terminate_block(TerminatorInstruction::if_(cond, body, exit, exit));

        self.switch_to_block(body);
        self.loop_targets.push((step, exit));
        let _ = self.expression(for_.body);
        self.loop_targets.pop();
        self.terminate_block(TerminatorInstruction::jmp_no_args(step));

        self.switch_to_block(step);
        let one = Self::integer(Integer::positive(1), kind);
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
        let path_type = self.types.result.maps.path_types[&tag].clone();
        let value_being_matched = self.variable(tag);
        let value_being_matched = self.deref_if_shared(value_being_matched, &path_type);
        let int_value = self.extract_tag_value(value_being_matched);
        let start = self.current_block;

        // The case key must be the actual value to compare against `int_value`. For variants
        // the tag value is the variant index; for int constants it's the constant itself.
        let case_blocks = mapvec(cases.iter(), |case| {
            let key = match &case.constructor {
                Constructor::False | Constructor::Unit => 0,
                Constructor::True => 1,
                Constructor::Int(value) => *value as u32,
                Constructor::Variant(_, variant_index) => *variant_index as u32,
                // Range constructors aren't lowered through a Switch directly; if we ever
                // encounter one here it's a compiler bug.
                Constructor::Range(_, _) => unreachable!("Range constructor in MIR switch lowering"),
            };
            (key, (self.push_block_no_params(), None))
        });

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

        let else_block = self.push_block_no_params();
        self.switch_to_block(else_block);
        let terminator = match else_ {
            Some(else_) => {
                let result = self.decision_tree(*else_, match_expr);
                TerminatorInstruction::jmp(end, result)
            },
            None => TerminatorInstruction::Unreachable,
        };
        self.terminate_block(terminator);

        self.switch_to_block(start);
        self.terminate_block(TerminatorInstruction::Switch {
            int_value,
            cases: case_blocks,
            else_: (else_block, None),
            end,
        });
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
            Type::Array { .. } => unreachable!("Cannot match on an array type"),
            Type::U32(_) => unreachable!("Cannot match on a type-level integer"),
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

        // Tagged unions have the form `(u8_tag, Union[...])` while
        // product types are plain tuples and should be returned directly
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

        // The parser wraps the handled expression in `fn () -> <body>` so it can serve as a
        // coroutine entry. Build that body lambda with a prelude that loads `h` from the
        // coroutine's user_data.
        let body = match &self.context()[handle.expression] {
            cst::Expr::Lambda(body_lambda) => {
                let body_lambda = body_lambda.clone();
                self.lambda_impl(&body_lambda, None, None, handle.expression, false, Some(handle.handler_name), false)
            },
            _ => unreachable!("handle.expression should be a Lambda after parsing"),
        };

        let cases = mapvec(&handle.cases, |(pattern, branch)| {
            let effect_op =
                self.try_resolve_effect_op(pattern.function).expect("Couldn't find effect op in MIR handle");
            let handler = match &self.context()[*branch] {
                cst::Expr::Lambda(branch_lambda) => {
                    let branch_lambda = branch_lambda.clone();
                    self.lambda_impl(&branch_lambda, None, None, *branch, false, None, true)
                },
                _ => unreachable!("handler branch should be a Lambda after parsing"),
            };
            crate::mir::HandlerCase { effect_op, handler }
        });

        self.push_instruction(Instruction::Handle { body, cases }, result_type)
    }

    fn reference(&mut self, reference: &cst::Reference) -> Value {
        let rhs = reference.rhs;
        let context = self.context();

        // If the RHS is a locally mutable variable, its value in local_variables
        // is already the StackAlloc pointer.
        if let cst::Expr::Variable(path_id) = &context[rhs] {
            let path_id = *path_id;
            if let Some(Origin::Local(name)) = context.path_origin(path_id)
                && self.mutable_locals.contains(&name)
            {
                return *self.local_variables.get(&name).expect("mutable local variable not found in local_variables");
            }
        }

        // Reborrow: if the rhs already has a reference type its value is already a pointer
        // (e.g. a `mut Array` parameter), so `mut x` reborrows that pointer directly instead of
        // taking the address of the local holding it. Mirrors the type-level reborrow in
        // `check_reference`.
        let rhs_type = self.types.result.maps.expr_types[&rhs].follow(&self.types.bindings);
        if rhs_type.reference_element(&self.types.bindings).is_some() {
            return self.expression(rhs);
        }

        if let cst::Expr::MemberAccess(_) = &self.context()[rhs]
            && self.reference_target_is_addressable(rhs)
        {
            return self.lhs_as_pointer(rhs);
        }

        // For all other cases (non-mutable local, temporary): evaluate the expression and
        // allocate a new stack slot for it.
        let value = self.expression(rhs);
        self.push_instruction(Instruction::StackAlloc(value), Type::POINTER)
    }

    /// Returns true if `expr` is a field-access chain (or annotated variant) rooted at a
    /// mutable local.
    fn reference_target_is_addressable(&self, expr: ExprId) -> bool {
        match &self.context()[expr] {
            cst::Expr::Variable(path_id) => {
                let path_id = *path_id;
                matches!(self.context().path_origin(path_id), Some(Origin::Local(name)) if self.mutable_locals.contains(&name))
            },
            cst::Expr::MemberAccess(ma) => self.reference_target_is_addressable(ma.object),
            cst::Expr::TypeAnnotation(ta) => self.reference_target_is_addressable(ta.lhs),
            _ => false,
        }
    }

    fn lhs_as_pointer(&mut self, lhs: ExprId) -> Value {
        // Storing a shared mut type should mutate the inner element
        let lhs_type = self.types.result.maps.expr_types[&lhs].follow(&self.types.bindings);
        let shared_mut = self.shared_mut_inner_layout_of(lhs_type).is_some();
        if shared_mut {
            return self.expression(lhs);
        }

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
                let struct_type = if let Some(inner) = self.shared_mut_inner_layout_of(struct_type) {
                    inner
                } else if let Some(element) = struct_type.reference_or_pointer_element(&self.types.bindings) {
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

            let function = self.expression(op_expr);
            let instruction = if self.type_of_value(&function).is_closure() {
                Instruction::CallClosure { closure: function, arguments: vec![current, rhs] }
            } else {
                Instruction::Call { function, arguments: vec![current, rhs] }
            };
            self.push_instruction(instruction, value_type)
        } else {
            let rhs = self.expression(assignment.rhs);

            // Overwriting a whole `shared mut` cell: copy the RHS's contents into the LHS rather than rebinding the pointer.
            let lhs_type = self.types.result.maps.expr_types[&assignment.lhs].follow(&self.types.bindings);
            match self.shared_mut_inner_layout_of(lhs_type) {
                Some(inner_layout) => self.push_instruction(Instruction::Deref(rhs), inner_layout),
                None => rhs,
            }
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
            None => self.convert_type(lhs_type, None),
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

        // For ability impls, the struct's MIR type tells us each field's expected closure shape.
        // If a field receives a bare function (env = NoClosureEnv) where a `Ptr Unit`-env closure
        // is required, pack it with a null pointer so the produced value matches.
        let struct_type = self.convert_expr_type(expr);
        let expected_field_types: Vec<Type> =
            if let Type::Tuple(fields) = &struct_type { fields.iter().cloned().collect() } else { Vec::new() };

        let field_values = mapvec(fields.iter().enumerate(), |(i, (_n, v))| {
            self.coerce_field_to_pointer_env(*v, expected_field_types.get(i))
        });
        let tuple_type = Type::Tuple(Arc::new(mapvec(&field_values, |v| self.type_of_value(v))));

        self.push_instruction(Instruction::MakeTuple(field_values), tuple_type)
    }

    fn coerce_field_to_pointer_env(&mut self, value: Value, expected: Option<&Type>) -> Value {
        let Some(expected) = expected else { return value };
        let Type::Function(expected_fn) = expected else { return value };
        if !matches!(expected_fn.environment, Type::Primitive(crate::mir::PrimitiveType::Pointer)) {
            return value;
        }
        let actual = self.type_of_value(&value);
        let Type::Function(actual_fn) = actual else { return value };
        if !matches!(actual_fn.environment, Type::Primitive(crate::mir::PrimitiveType::NoClosureEnv)) {
            return value;
        }

        // Resolve `value` down to a (DefinitionId, optional GenericBindings) pair we can
        // re-reference inside a freshly-generated wrapper definition. If the source value
        // isn't a definition reference (e.g. it's a synthesized instruction with no
        // top-level identity) we can't safely build a wrapper here, so fall back to packing
        // the raw fn-ptr; downstream may still hit a shape mismatch but most cases (impls
        // like `cast = transmute` / `print = print_float`) resolve to Definition values.
        let (inner_def_id, inner_bindings) = match value {
            Value::Definition(id) => (id, None),
            Value::InstructionResult(iid) => {
                let bindings = match &self.current_function.as_ref().unwrap().instructions[iid] {
                    Instruction::Instantiate(id, bindings) => Some((*id, Some(bindings.clone()))),
                    Instruction::Id(Value::Definition(id)) => Some((*id, None)),
                    _ => None,
                };
                match bindings {
                    Some(p) => p,
                    None => {
                        let null_ptr = self.push_instruction(Instruction::Transmute(Value::Unit), Type::POINTER);
                        return self.push_instruction(
                            Instruction::PackClosure { function: value, environment: null_ptr },
                            expected.clone(),
                        );
                    },
                }
            },
            _ => return value,
        };

        let generic_count = self.generics_in_scope.len() as u32;
        let wrapper_type = expected.clone();
        let expected_params = expected_fn.parameters.clone();
        let expected_env = expected_fn.environment.clone();
        let expected_return = expected_fn.return_type.clone();
        let inner_typ = actual_fn.as_ref().clone();
        let inner_typ = Type::Function(Arc::new(inner_typ));
        let wrapper_id = self.new_definition(
            Arc::new("ability_field_wrapper".to_string()),
            None,
            generic_count,
            wrapper_type.clone(),
            |this| {
                for pt in &expected_params {
                    this.push_parameter(pt.clone());
                }
                this.push_parameter(expected_env.clone());
                let forward_args =
                    mapvec(0..expected_params.len(), |j| Value::Parameter(BlockId::ENTRY_BLOCK, j as u32));
                let inner_value = if let Some(bindings) = inner_bindings {
                    this.push_instruction(Instruction::Instantiate(inner_def_id, bindings), inner_typ.clone())
                } else {
                    Value::Definition(inner_def_id)
                };
                let result = this.push_instruction(
                    Instruction::Call { function: inner_value, arguments: forward_args },
                    expected_return.clone(),
                );
                this.terminate_block(TerminatorInstruction::Return(result));
            },
        );

        let wrapper_value =
            self.make_definition_value(wrapper_id, Arc::new("ability_field_wrapper".to_string()), wrapper_type.clone());
        let null_ptr = self.push_instruction(Instruction::Transmute(Value::Unit), Type::POINTER);
        self.push_instruction(Instruction::PackClosure { function: wrapper_value, environment: null_ptr }, wrapper_type)
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
            cst::Pattern::Constructor(_type, arguments) => {
                let pattern_tc_type = self.types.result.maps.pattern_types[&pattern].clone();
                let value = self.deref_if_shared(value, &pattern_tc_type);
                match self.type_of_value(&value) {
                    Type::Union(_variants) => todo!("Deconstruct union"),
                    Type::Tuple(fields) => {
                        for (i, (field_type, argument)) in fields.iter().zip(arguments).enumerate() {
                            let instruction = Instruction::IndexTuple { tuple: value, index: i as u32 };
                            let field = self.push_instruction(instruction, field_type.clone());
                            self.bind_pattern(*argument, field);
                        }
                    },
                    other => unreachable!("Expected tuple or union when deconstructing pattern, found {other}"),
                }
            },
            cst::Pattern::TypeAnnotation(pattern, _) => self.bind_pattern(*pattern, value),
            cst::Pattern::MethodName { type_name: _, item_name } => {
                if let Some(origin) = self.context().name_origin(*item_name) {
                    self.define_variable(origin, value);
                }
            },
            cst::Pattern::Or(_) => {
                unreachable!("OR-patterns must be expanded by the decision tree compiler before MIR lowering")
            },
            cst::Pattern::Alias(name, inner) => {
                if let Some(origin) = self.context().name_origin(*name) {
                    self.define_variable(origin, value.clone());
                }
                self.bind_pattern(*inner, value);
            },
        }
    }

    fn finish(self) -> Mir {
        Mir {
            definitions: self.finished_functions,
            externals: self.external,
            preserved_op_indices: self.effect_op_indices,
        }
        .remove_internal_externs()
    }

    /// Sets [self.generics_in_scope] to a map mapping each generic from the given type to a
    /// `[mir::Generic]` used when translating [type_inference::types::Type]s into [crate::mir::Type]s.
    fn set_generics_in_scope(&mut self, definition_type: &TCType) {
        use type_inference::types::Type;
        self.generics_in_scope.clear();

        if let Type::Forall(generics, _) = definition_type {
            for (i, generic) in generics.iter().enumerate() {
                self.generics_in_scope.insert(*generic, Generic(i as u32));
            }
        }
    }

    /// For type definitions we need to define their constructors
    fn type_definition(&mut self, type_definition: &cst::TypeDefinition) {
        if type_definition.kind.is_effect() {
            return;
        }

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

            let parameters = match constructor_type.ignore_forall() {
                TCType::Function(function) => function.parameters.as_slice(),
                _ => &[],
            };

            let shared = type_definition.shared;
            self.define_type_constructor(constructor_name, constructor_type, parameters, tag, shared);
        }

        // Abilities are sugar for a struct of function-typed fields, however each "field" is treated
        // as a function by the frontend so we must generate actual functions for each field such
        // that `Cast.cast` is an actual function accepting a `Cast` instance and forwarding the
        // appropriate arguments to the `cast` field.
        if type_definition.kind.is_ability() {
            self.define_ability_methods(type_definition);
        }
    }

    fn define_type_constructor(
        &mut self, name_id: NameId, constructor_type: &TCType,
        field_types: &[crate::type_inference::types::ParameterType], tag: Option<u8>, shared: bool,
    ) {
        let top_level_name = TopLevelName::new(self.top_level_id, name_id);
        let name = self.context()[name_id].clone();
        let typ = self.convert_type(constructor_type, None);

        // The unboxed layout constructor body builds. For non-shared types this is the same as
        // the final type. For shared types, we wrap this in a pointer at the end.
        let payload_type = if shared {
            let return_typ =
                constructor_type.ignore_forall().return_type().unwrap_or_else(|| constructor_type.ignore_forall());
            self.shared_inner_layout_of(return_typ).unwrap_or_else(|| {
                let name = self.context()[name_id].clone();
                panic!("shared constructor {name} has no inner layout")
            })
        } else {
            typ.function_return_type().cloned().unwrap_or_else(|| typ.clone())
        };

        let raw_union_type = payload_type.without_union_tag();
        let generic_count = self.generics_in_scope.len() as u32;
        let is_zero_arg = field_types.is_empty();

        let id = self.new_definition(name, Some(name_id), generic_count, typ, |this| {
            let mut result = this.build_constructor_payload(field_types, tag, &payload_type, raw_union_type, name_id);
            if shared {
                result = this.push_instruction(Instruction::AllocShared(result), Type::POINTER);
            }

            // 0-arg constructors are globals (`Result` terminator → `is_global()` is true).
            // For shared 0-arg constructors the AllocShared lowers to a backing static in
            // the constant codegen path.
            let terminator = if is_zero_arg { TerminatorInstruction::Result } else { TerminatorInstruction::Return };
            this.terminate_block(terminator(result));
        });
        self.name_to_id.insert(top_level_name, id);
    }

    /// Build the value a constructor returns *before* any shared-pointer wrap.
    /// Materializes block parameters, packs them into a tuple, and (for sum-type
    /// variants) transmutes & wraps `(tag, union)`.
    fn build_constructor_payload(
        &mut self, field_types: &[crate::type_inference::types::ParameterType], tag: Option<u8>, payload_type: &Type,
        raw_union_type: Option<Type>, name_id: NameId,
    ) -> Value {
        let field_types = mapvec(field_types, |param| self.convert_type(&param.typ, None));
        let fields = mapvec(field_types.iter().enumerate(), |(i, field_type)| {
            self.push_parameter(field_type.clone());
            Value::Parameter(BlockId::ENTRY_BLOCK, i as u32)
        });

        let mut payload = self.push_instruction(Instruction::MakeTuple(fields), Type::tuple(field_types));

        if let Some(tag) = tag {
            let raw_union_type = raw_union_type.unwrap_or_else(|| {
                let name = self.context()[name_id].clone();
                panic!("Failed to unwrap raw union type. Full result type is: {payload_type} for constructor {name}")
            });
            let casted = self.push_instruction(Instruction::Transmute(payload), raw_union_type);
            payload = self
                .push_instruction(Instruction::MakeTuple(vec![Value::tag_value(tag), casted]), payload_type.clone());
        }

        payload
    }

    fn define_ability_methods(&mut self, type_definition: &cst::TypeDefinition) {
        if let cst::TypeDefinitionBody::Struct(fields) = &type_definition.body {
            let constructor_type = self.types.get_generalized(type_definition.name);
            self.set_generics_in_scope(constructor_type);
            let generic_count = self.generics_in_scope.len() as u32;
            let constructor_mir_type = self.convert_type(constructor_type, None);

            let struct_type =
                constructor_mir_type.function_return_type().cloned().unwrap_or_else(|| constructor_mir_type.clone());

            let field_mir_types: Vec<Type> =
                if let Type::Function(fn_type) = &constructor_mir_type { fn_type.parameters.clone() } else { vec![] };

            for (i, (field_name_id, _)) in fields.iter().enumerate() {
                let Some(field_type) = field_mir_types.get(i) else { continue };

                // Only generate wrappers for function-typed fields (all ability methods).
                // TODO: We should still generate wrappers for other types
                let Type::Function(fn_type) = field_type else { continue };
                let (value_param_types, return_type) = (fn_type.parameters.clone(), fn_type.return_type.clone());

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
                    // Last parameter is the implicit struct receiver.
                    let struct_param = Value::Parameter(BlockId::ENTRY_BLOCK, n_params as u32 - 1);
                    let extracted = this.push_instruction(
                        Instruction::IndexTuple { tuple: struct_param, index: i as u32 },
                        field_type_clone,
                    );
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
