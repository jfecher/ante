//! The MIR builder will translate a single top-level item into the equivalent MIR for the item.
//! This is meant to work in parallel for each item.
//!
//! For more on the Medium-level IR (MIR) itself, see [super].
//!
//! Although the MIR may eventually be monomorphized, the initial output of this builder uses a
//! uniform representation instead, relying on a later pass to manually specialize each function
//! if desired.
use std::{collections::BTreeMap, sync::Arc};

use inc_complete::DbGet;
use rustc_hash::FxHashMap;

use crate::{
    incremental::{GetItem, TypeCheck},
    iterator_extensions::vecmap,
    lexer::token::{FloatKind, IntegerKind},
    mir::{
        Block, BlockId, FloatConstant, Function, FunctionId, Instruction, IntConstant, Mir, TerminatorInstruction,
        Type, Value,
    },
    name_resolution::Origin,
    parser::{
        cst::{self, Literal, Name, SequenceItem},
        ids::{ExprId, PathId, PatternId, TopLevelId},
    },
    type_inference::{
        dependency_graph::TypeCheckResult,
        fresh_expr::ExtendedTopLevelContext,
        patterns::{Case, Constructor, DecisionTree},
        top_level_types::TopLevelType,
    },
};

mod types;

/// Convert the given item to an initial MIR representation. This may be done in parallel with all
/// other items in the program.
///
/// The initial MIR representation uses a uniform-representation for generics, rather than
/// a monomorphized one. If monomorphization is required, a separate monomorphization pass should
/// be run on the MIR after collecting it all.
pub(crate) fn build_initial_mir<T>(compiler: &T, item_id: TopLevelId) -> Option<Mir>
where
    T: DbGet<TypeCheck> + DbGet<GetItem>,
{
    let types = TypeCheck(item_id).get(compiler);
    let (item, _) = GetItem(item_id).get(compiler);

    let functions = match &item.kind {
        cst::TopLevelItemKind::Definition(definition) => {
            let mut context = Context::new(compiler, &types, item_id);
            context.definition(definition);
            Some(context.finish())
        },
        cst::TopLevelItemKind::TypeDefinition(type_definition) => {
            let mut context = Context::new(compiler, &types, item_id);
            context.type_definition(type_definition);
            Some(context.finish())
        },
        cst::TopLevelItemKind::TraitDefinition(_) => None,
        cst::TopLevelItemKind::TraitImpl(_) => None,
        cst::TopLevelItemKind::EffectDefinition(_) => None,
        cst::TopLevelItemKind::Extern(_) => None, // TODO
        cst::TopLevelItemKind::Comptime(_) => None,
    }?;

    Some(Mir { functions })
}

/// The per-[TopLevelId] context. This pass is designed so that we can convert every top-level item
/// to MIR in parallel.
struct Context<'local, Db> {
    compiler: &'local Db,
    types: &'local TypeCheckResult,

    top_level_id: TopLevelId,

    current_function: Option<Function>,
    current_block: BlockId,

    variables: FxHashMap<Origin, Value>,

    next_function_id: u32,
    finished_functions: FxHashMap<FunctionId, Function>,
}

impl<'local, Db> Context<'local, Db>
where
    Db: DbGet<TypeCheck> + DbGet<GetItem>,
{
    fn new(compiler: &'local Db, types: &'local TypeCheckResult, top_level_id: TopLevelId) -> Self {
        Self {
            compiler,
            types,
            top_level_id,
            variables: Default::default(),
            current_block: BlockId::ENTRY_BLOCK,
            current_function: None,
            finished_functions: Default::default(),
            next_function_id: 0,
        }
    }

    /// Return the next free function id, and increment the id after.
    fn next_function_id(&mut self) -> FunctionId {
        let index = self.next_function_id;
        self.next_function_id += 1;
        FunctionId { item: self.top_level_id, index }
    }

    /// Returns the current function being built. Panics if thre is none.
    fn current_function(&mut self) -> &mut Function {
        self.current_function.as_mut().unwrap()
    }

    fn type_of_value(&self, value: Value) -> Type {
        self.current_function.as_ref().unwrap().type_of_value(value)
    }

    /// Returns the current block being inserted into. Panics if there is no current function.
    fn current_block(&mut self) -> &mut Block {
        &mut self.current_function.as_mut().unwrap().blocks[self.current_block]
    }

    fn context(&self) -> &'local ExtendedTopLevelContext {
        &self.types.result.context
    }

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
        assert!(block.terminator.is_none());
        block.terminator = Some(terminator);
    }

    fn expr_type(&self, expr: ExprId) -> Type {
        let typ = &self.types.result.maps.expr_types[&expr];
        self.convert_type(typ, None)
    }

    fn expression(&mut self, expr: ExprId) -> Value {
        match &self.context()[expr] {
            cst::Expr::Error => unreachable!("Error expression encountered while generating boxed mir"),
            cst::Expr::Literal(literal) => self.literal(literal, expr),
            cst::Expr::Variable(path_id) => self.variable(*path_id),
            cst::Expr::Sequence(sequence) => self.sequence(sequence),
            cst::Expr::Definition(definition) => self.definition(definition),
            cst::Expr::MemberAccess(member_access) => self.member_access(member_access, expr),
            cst::Expr::Call(call) => self.call(call, expr),
            cst::Expr::Lambda(lambda) => self.lambda(lambda, None),
            cst::Expr::If(if_) => self.if_(if_, expr),
            cst::Expr::Match(_) => self.match_(expr),
            cst::Expr::Handle(handle) => self.handle(handle),
            cst::Expr::Reference(reference) => self.reference(reference),
            cst::Expr::TypeAnnotation(type_annotation) => self.expression(type_annotation.lhs),
            cst::Expr::Constructor(constructor) => self.constructor(constructor, expr),
            cst::Expr::Quoted(quoted) => self.quoted(quoted),
        }
    }

    fn literal(&mut self, literal: &Literal, _expr: ExprId) -> Value {
        match literal {
            Literal::Unit => Value::Unit,
            Literal::Bool(x) => Value::Bool(*x),
            Literal::Integer(x, None) => {
                Value::Integer(IntConstant::U32((*x).try_into().unwrap()))
                // panic!("TODO: polymorphic integers")
            },
            Literal::Integer(x, Some(IntegerKind::I8)) => Value::Integer(IntConstant::I8((*x).try_into().unwrap())),
            Literal::Integer(x, Some(IntegerKind::I16)) => Value::Integer(IntConstant::I16((*x).try_into().unwrap())),
            Literal::Integer(x, Some(IntegerKind::I32)) => Value::Integer(IntConstant::I32((*x).try_into().unwrap())),
            Literal::Integer(x, Some(IntegerKind::I64)) => Value::Integer(IntConstant::I64((*x).try_into().unwrap())),
            Literal::Integer(x, Some(IntegerKind::Isz)) => Value::Integer(IntConstant::Isz((*x).try_into().unwrap())),
            Literal::Integer(x, Some(IntegerKind::U8)) => Value::Integer(IntConstant::U8((*x).try_into().unwrap())),
            Literal::Integer(x, Some(IntegerKind::U16)) => Value::Integer(IntConstant::U16((*x).try_into().unwrap())),
            Literal::Integer(x, Some(IntegerKind::U32)) => Value::Integer(IntConstant::U32((*x).try_into().unwrap())),
            Literal::Integer(x, Some(IntegerKind::U64)) => Value::Integer(IntConstant::U64((*x).try_into().unwrap())),
            Literal::Integer(x, Some(IntegerKind::Usz)) => Value::Integer(IntConstant::Usz((*x).try_into().unwrap())),
            Literal::Float(_, None) => {
                panic!("TODO: polymorphic floats")
            },
            Literal::Float(x, Some(FloatKind::F32)) => Value::Float(FloatConstant::F32(*x)),
            Literal::Float(x, Some(FloatKind::F64)) => Value::Float(FloatConstant::F64(*x)),
            Literal::String(x) => self.push_instruction(Instruction::MakeString(x.clone()), Type::string()),
            Literal::Char(x) => Value::Char(*x),
        }
    }

    fn variable(&self, path_id: PathId) -> Value {
        // Deliberately allow us to reference variables not in the context.
        // This allows us to convert all definitions to MIR in parallel, trusting
        // that the links will work out later.
        match self.context().path_origin(path_id) {
            Some(Origin::TopLevelDefinition(item)) => Value::Global(item),
            Some(origin) => self.variables[&origin],
            None => {
                println!("Warning: no origin for {path_id:?}: {}", self.context()[path_id]);
                Value::Error
            },
        }
    }

    fn sequence(&mut self, sequence: &[SequenceItem]) -> Value {
        let mut result = Value::Unit;
        for item in sequence {
            result = self.expression(item.expr);
        }
        result
    }

    fn definition(&mut self, definition: &cst::Definition) -> Value {
        let mut value = match &self.context()[definition.rhs] {
            cst::Expr::Lambda(lambda) => self.lambda(lambda, self.try_find_name(definition.pattern)),
            _ => self.expression(definition.rhs),
        };

        if definition.mutable {
            value = self.push_instruction(Instruction::StackAlloc(value), Type::POINTER);
        }
        self.bind_pattern(definition.pattern, value);
        Value::Unit
    }

    fn member_access(&mut self, member_access: &cst::MemberAccess, expr: ExprId) -> Value {
        let tuple = self.expression(member_access.object);
        let index = self.context().member_access_index(expr).unwrap_or(u32::MAX);
        let element_type = match self.type_of_value(tuple) {
            Type::Tuple(elements) => elements.get(index as usize).cloned().unwrap_or(Type::ERROR),
            _ => Type::ERROR,
        };
        self.push_instruction(Instruction::IndexTuple { tuple, index }, element_type)
    }

    fn call(&mut self, call: &cst::Call, id: ExprId) -> Value {
        let function = self.expression(call.function);
        let arguments = vecmap(&call.arguments, |expr| self.expression(*expr));
        let result_type = self.expr_type(id);
        self.push_instruction(Instruction::Call { function, arguments }, result_type)
    }

    fn try_find_name(&self, pattern: PatternId) -> Option<Name> {
        match &self.context()[pattern] {
            cst::Pattern::Error => None,
            cst::Pattern::Literal(_) => None,
            cst::Pattern::Constructor(..) => None,
            cst::Pattern::TypeAnnotation(pattern, _) => self.try_find_name(*pattern),
            cst::Pattern::Variable(name) | cst::Pattern::MethodName { item_name: name, .. } => {
                Some(self.context()[*name].clone())
            },
        }
    }

    /// Save the current function state, create a new function,
    /// run `f` to fill in the function's body, then restore the previous state
    /// and return the new function value.
    fn new_function(&mut self, name: Name, f: impl FnOnce(&mut Self)) -> Value {
        let previous_function = self.current_function.take();
        let previous_block = std::mem::replace(&mut self.current_block, BlockId::ENTRY_BLOCK);
        let function_id = self.next_function_id();

        self.current_function = Some(Function::new(name, function_id));

        f(self);

        // safety: `self.current_function` should always be set since we set it above and this
        // is the only method which modifies this field directly.
        let finished_function = std::mem::replace(&mut self.current_function, previous_function).unwrap();
        self.current_block = previous_block;

        self.finished_functions.insert(function_id, finished_function);
        Value::Function(function_id)
    }

    fn lambda(&mut self, lambda: &cst::Lambda, name: Option<Name>) -> Value {
        let name = name.unwrap_or_else(|| Arc::new("lambda".to_string()));
        self.new_function(name, |this| {
            for (i, parameter) in lambda.parameters.iter().enumerate() {
                let parameter_value = Value::Parameter(this.current_block, i as u32);
                this.bind_pattern(parameter.pattern, parameter_value);

                let parameter_type = &this.types.result.maps.pattern_types[&parameter.pattern];
                this.push_parameter(this.convert_type(parameter_type, None));
            }

            let return_value = this.expression(lambda.body);
            this.terminate_block(TerminatorInstruction::Return(return_value));
        })
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

        let case_blocks = vecmap(&cases, |_| (self.push_block_no_params(), None));
        let mut else_block = None;

        let result_type = self.expr_type(match_expr);
        let end = self.push_block(vec![result_type]);

        for ((case_block, _), case) in case_blocks.iter().zip(cases) {
            self.switch_to_block(*case_block);

            if !case.arguments.is_empty() {
                let Constructor::Variant(_, variant_index) = &case.constructor else {
                    unreachable!("For this constructor to define arguments it must be a Constructor::Variant")
                };

                // Cast the whole value being matched `(tag, union)` to `(tag, this_variant)`
                // and extract the variant from the tuple.
                let variant = self.extract_variant(value_being_matched, *variant_index);
                let variant_type = self.type_of_value(variant);

                // And for each variable, extract the relevant field of the variant
                for (i, argument) in case.arguments.iter().enumerate() {
                    if let Some(origin) = self.context().path_origin(*argument) {
                        let field_type = Self::tuple_field_type(&variant_type, i);
                        let index_tuple = Instruction::IndexTuple { tuple: variant, index: i as u32 };
                        let field = self.push_instruction(index_tuple, field_type);
                        self.variables.insert(origin, field);
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
        match self.type_of_value(value_being_matched) {
            Type::Primitive(_) => value_being_matched,
            Type::Tuple(fields) => {
                if fields.is_empty() {
                    unreachable!("Cannot match on an empty tuple")
                }
                let tag_type = fields[0].clone();
                let instruction = Instruction::IndexTuple { tuple: value_being_matched, index: 0 };
                self.push_instruction(instruction, tag_type)
            },
            Type::Union(_) => unreachable!("Cannot match on a raw union type"),
            Type::Function(_) => unreachable!("Cannot match on a function type"),
            Type::Generic(_) => unreachable!("Cannot match on a generic type"),
        }
    }

    /// Cast & Extract the variant value from the given `(tag, union)` tuple.
    /// Returns the variant (as a tuple, no longer a union).
    fn extract_variant(&mut self, value_being_matched: Value, variant_index: usize) -> Value {
        let fields = match self.type_of_value(value_being_matched) {
            Type::Tuple(fields) => fields,
            _ => unreachable!("Only `(tag, union)` tuples may have fields to extract"),
        };
        assert_eq!(fields.len(), 2);

        let union_type = &fields[1];
        let Type::Union(variants) = union_type else {
            unreachable!("The second field of a tagged union should always be either a union or unit value")
        };

        let variant_type = variants
            .get(variant_index)
            .unwrap_or_else(|| {
                unreachable!("Expected variant index {variant_index} but only had {} variants", variants.len())
            })
            .clone();

        let extract_union = Instruction::IndexTuple { tuple: value_being_matched, index: 1 };
        let union = self.push_instruction(extract_union, union_type.clone());
        self.push_instruction(Instruction::Transmute(union), variant_type)
    }

    fn handle(&self, _handle: &cst::Handle) -> Value {
        todo!("mir handle")
    }

    fn reference(&self, _reference: &cst::Reference) -> Value {
        todo!("mir reference")
    }

    fn constructor(&mut self, constructor: &cst::Constructor, expr: ExprId) -> Value {
        // Side-effects are executed in source order but the type must
        // be packed in declaration order. So re-order fields afterward.
        let mut fields = vecmap(&constructor.fields, |(name, field)| (*name, self.expression(*field)));

        // We must be careful here so that we can still produce MIR even if type-checking failed
        let no_order = BTreeMap::new();
        let field_order = self.context().constructor_field_order(expr).unwrap_or(&no_order);
        fields.sort_unstable_by_key(|(name, _)| field_order.get(name).unwrap_or(&0));

        let fields = vecmap(fields, |(_name, value)| value);
        let tuple_type = Type::Tuple(Arc::new(vecmap(&fields, |value| self.type_of_value(*value))));

        self.push_instruction(Instruction::MakeTuple(fields), tuple_type)
    }

    fn quoted(&self, _quoted: &cst::Quoted) -> Value {
        unreachable!("Should never convert a Quoted expr to mir")
    }

    /// Bind the given value to the given pattern
    fn bind_pattern(&mut self, pattern: PatternId, value: Value) {
        match &self.context()[pattern] {
            cst::Pattern::Error => unreachable!("Error pattern encountered in bind_pattern"),
            cst::Pattern::Variable(name) => {
                if let Some(origin) = self.context().name_origin(*name) {
                    self.variables.insert(origin, value);
                }
            },
            cst::Pattern::Literal(_) => (),
            cst::Pattern::Constructor(_type, _arguments) => {
                todo!("Constructors")
            },
            cst::Pattern::TypeAnnotation(pattern, _) => self.bind_pattern(*pattern, value),
            cst::Pattern::MethodName { type_name: _, item_name } => {
                if let Some(origin) = self.context().name_origin(*item_name) {
                    self.variables.insert(origin, value);
                }
            },
        }
    }

    fn finish(self) -> FxHashMap<FunctionId, Function> {
        self.finished_functions
    }

    /// For type definitions we need to define their constructors
    ///
    /// TODO: How to define these generic definitions in a way which the types still match?
    ///       We'll likely need some marker to allow parameter/argument type mismatches
    ///       past the marker and mark the type as pass-by-reference only if its size is unknown.
    ///       E.g. `Maybe a = None | Some a` becomes `unsized. (U8, ?)` or similar.
    ///       Maybe the `unsized` is unneeded though, and it is just the presense of any `?`
    ///       which would cause it to be unsized. These would naturally be hidden by pointers
    ///       anyway so `Vec a = (Ptr a, Usz, Usz)` becomes `Ptr, Usz, Usz` with opaque pointers.
    fn type_definition(&mut self, _type_definition: &cst::TypeDefinition) {
        // For a type definition, each generalized name will be each publically visible constructor
        for (constructor_name, constructor_type) in &self.types.result.generalized {
            let constructor_name = self.context()[*constructor_name].clone();

            if let TopLevelType::Function { parameters, .. } = &constructor_type.typ {
                self.define_type_constructor(constructor_name, parameters)
            } else {
                todo!("Non-function type constructors")
            }
        }
    }

    fn define_type_constructor(&mut self, name: Name, parameter_types: &[TopLevelType]) {
        self.new_function(name, |this| {
            let (fields, field_types) = parameter_types
                .iter()
                .enumerate()
                .map(|(i, field_type)| {
                    let field_type = this.convert_top_level_type(field_type, None);
                    this.push_parameter(field_type.clone());
                    (Value::Parameter(BlockId::ENTRY_BLOCK, i as u32), field_type)
                })
                .unzip();

            let result_type = Type::tuple(field_types);
            let tuple = this.push_instruction(Instruction::MakeTuple(fields), result_type);
            this.terminate_block(TerminatorInstruction::Return(tuple));
        });
    }
}
