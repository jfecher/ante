//! The MIR builder will translate a single top-level item into the equivalent MIR for the item.
//! This is meant to work in parallel for each item.
//!
//! For more on the Medium-level IR (MIR) itself, see [super].
//!
//! Although the MIR may eventually be monomorphized, the initial output of this builder uses a
//! uniform representation instead, relying on a later pass to manually specialize each function
//! if desired.
use std::collections::BTreeMap;

use rustc_hash::FxHashMap;

use crate::{
    incremental::{Db, GetItem, TypeCheck},
    iterator_extensions::vecmap,
    lexer::token::{FloatKind, IntegerKind},
    mir::{
        Block, BlockId, FloatConstant, Function, FunctionId, Instruction, IntConstant, TerminatorInstruction, Value,
    },
    name_resolution::Origin,
    parser::{
        cst::{self, Literal, SequenceItem},
        ids::{ExprId, PathId, PatternId, TopLevelId},
    },
    type_inference::{
        dependency_graph::TypeCheckResult,
        fresh_expr::ExtendedTopLevelContext,
        patterns::{Case, DecisionTree},
    },
};

/// Convert the given item to an initial MIR representation. This may be done in parallel with all
/// other items in the program.
///
/// The initial MIR representation uses a uniform-representation for generics, rather than
/// a monomorphized one. If monomorphization is required, a separate monomorphization pass should
/// be run on the MIR after collecting it all.
pub(crate) fn build_initial_mir(compiler: &Db, item_id: TopLevelId) -> Option<FxHashMap<FunctionId, Function>> {
    let types = TypeCheck(item_id).get(compiler);
    let (item, _) = GetItem(item_id).get(compiler);

    match &item.kind {
        cst::TopLevelItemKind::Definition(definition) => {
            let mut context = Context::new(&types, item_id);
            let _ = context.definition(definition);
            Some(context.finish())
        },
        cst::TopLevelItemKind::TypeDefinition(_) => None,
        cst::TopLevelItemKind::TraitDefinition(_) => None,
        cst::TopLevelItemKind::TraitImpl(_) => None,
        cst::TopLevelItemKind::EffectDefinition(_) => None,
        cst::TopLevelItemKind::Extern(_) => None, // TODO
        cst::TopLevelItemKind::Comptime(_) => None,
    }
}

/// The per-[TopLevelId] context. This pass is designed so that we can convert every top-level item
/// to MIR in parallel.
struct Context<'local> {
    types: &'local TypeCheckResult,

    top_level_id: TopLevelId,

    current_function: Option<Function>,
    current_block: BlockId,

    variables: FxHashMap<Origin, Value>,

    next_function_id: u32,
    finished_functions: FxHashMap<FunctionId, Function>,
}

impl<'local> Context<'local> {
    fn new(types: &'local TypeCheckResult, top_level_id: TopLevelId) -> Self {
        Self {
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

    fn current_block(&mut self) -> &mut Block {
        &mut self.current_function.as_mut().unwrap().blocks[self.current_block]
    }

    fn context(&self) -> &'local ExtendedTopLevelContext {
        &self.types.result.context
    }

    /// Push an instruction and return its result.
    fn push_instruction(&mut self, instruction: Instruction) -> Value {
        let id = self.current_block().instructions.push(instruction);
        Value::InstructionResult(id)
    }

    /// Create a block (although do not switch to it) and return it
    fn push_block(&mut self, parameter_count: u32) -> BlockId {
        self.current_function.as_mut().unwrap().blocks.push(Block::new(parameter_count))
    }

    /// Switch to a new block to start inserting instructions into
    fn switch_to_block(&mut self, block: BlockId) {
        self.current_block = block;
    }

    /// Terminate the current block with the given terminator instruction
    fn terminate_block(&mut self, terminator: TerminatorInstruction) {
        let block = self.current_block();
        assert!(block.terminator.is_none());
        block.terminator = Some(terminator);
    }

    fn expression(&mut self, expr: ExprId) -> Value {
        match &self.context()[expr] {
            cst::Expr::Error => unreachable!("Error expression encountered while generating boxed mir"),
            cst::Expr::Literal(literal) => self.literal(literal, expr),
            cst::Expr::Variable(path_id) => self.variable(*path_id),
            cst::Expr::Sequence(sequence) => self.sequence(sequence),
            cst::Expr::Definition(definition) => self.definition(definition),
            cst::Expr::MemberAccess(member_access) => self.member_access(member_access, expr),
            cst::Expr::Call(call) => self.call(call),
            cst::Expr::Lambda(lambda) => self.lambda(lambda),
            cst::Expr::If(if_) => self.if_(if_),
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
            Literal::Integer(_, None) => {
                panic!("TODO: polymorphic integers")
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
                // let location = self.context().expr_location(expr);
                // UnimplementedItem::PolymorphicFloats.issue(self.compiler, location);
                // Value::Unit
                panic!("TODO: polymorphic floats")
            },
            Literal::Float(x, Some(FloatKind::F32)) => Value::Float(FloatConstant::F32(x.0 as f32)),
            Literal::Float(x, Some(FloatKind::F64)) => Value::Float(FloatConstant::F64(x.0)),
            Literal::String(x) => self.push_instruction(Instruction::MakeString(x.clone())),
            Literal::Char(x) => Value::Char(*x),
        }
    }

    fn variable(&self, path_id: PathId) -> Value {
        // Deliberately allow us to reference variables not in the context.
        // This allows us to convert all definitions to MIR in parallel, trusting
        // that the links will work out later.
        match self.context().path_origin(path_id) {
            Origin::TopLevelDefinition(item) => Value::Global(item),
            origin => self.variables[&origin],
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
        let mut value = self.expression(definition.rhs);
        if definition.mutable {
            value = self.push_instruction(Instruction::StackAlloc(value));
        }
        self.bind_pattern(definition.pattern, value);
        Value::Unit
    }

    fn member_access(&mut self, member_access: &cst::MemberAccess, expr: ExprId) -> Value {
        let tuple = self.expression(member_access.object);
        let index = self.context().member_access_index(expr).unwrap_or(u32::MAX);
        self.push_instruction(Instruction::IndexTuple { tuple, index })
    }

    fn call(&mut self, call: &cst::Call) -> Value {
        let function = self.expression(call.function);
        let arguments = vecmap(&call.arguments, |expr| self.expression(*expr));
        self.push_instruction(Instruction::Call { function, arguments })
    }

    fn lambda(&mut self, lambda: &cst::Lambda) -> Value {
        let previous_function = self.current_function.take();
        let previous_block = std::mem::replace(&mut self.current_block, BlockId::ENTRY_BLOCK);
        let function_id = self.next_function_id();

        self.current_function = Some(Function::new(function_id));
        self.current_block().parameter_count = lambda.parameters.len() as u32;

        for (i, parameter) in lambda.parameters.iter().enumerate() {
            let parameter_value = Value::Parameter(self.current_block, i as u32);
            self.bind_pattern(parameter.pattern, parameter_value);
        }

        let return_value = self.expression(lambda.body);
        self.terminate_block(TerminatorInstruction::Return(return_value));

        // safety: `self.current_function` should always be set since we set it above and `lambda`
        // is the only method which modifies this field directly.
        let finished_function = std::mem::replace(&mut self.current_function, previous_function).unwrap();
        self.current_block = previous_block;

        self.finished_functions.insert(function_id, finished_function);
        Value::Function(function_id)
    }

    fn if_(&mut self, if_: &cst::If) -> Value {
        let condition = self.expression(if_.condition);

        let then = self.push_block(0);
        let else_ = self.push_block(0);
        self.terminate_block(TerminatorInstruction::If { condition, then, else_ });

        self.switch_to_block(then);
        let then_value = self.expression(if_.then);

        if let Some(else_expr) = if_.else_ {
            let end = self.push_block(1);
            self.terminate_block(TerminatorInstruction::Jmp(end, vec![then_value]));

            self.switch_to_block(else_);
            let else_value = self.expression(else_expr);
            self.terminate_block(TerminatorInstruction::Jmp(end, vec![else_value]));
            Value::Parameter(end, 0)
        } else {
            self.terminate_block(TerminatorInstruction::Jmp(else_, Vec::new()));
            Value::Unit
        }
    }

    fn match_(&mut self, expr: ExprId) -> Value {
        let tree = self.context().decision_tree(expr).clone();
        self.decision_tree(tree)
    }

    fn decision_tree(&mut self, tree: DecisionTree) -> Value {
        match tree {
            DecisionTree::Success(expr) => self.expression(expr),
            DecisionTree::Failure { .. } => {
                // Expect an error to already be issued
                Value::Unit
            },
            DecisionTree::Guard { condition, then, else_ } => self.match_if_guard(condition, then, *else_),
            DecisionTree::Switch(tag, cases, else_) => self.switch(tag, cases, else_),
        }
    }

    /// Almost identical to an if-then-else, the main difference being the else is required
    /// and is of type [DecisionTree]
    fn match_if_guard(&mut self, condition: ExprId, then_expr: ExprId, else_tree: DecisionTree) -> Value {
        let then = self.push_block(0);
        let else_ = self.push_block(0);
        let end = self.push_block(1);

        let condition = self.expression(condition);
        self.terminate_block(TerminatorInstruction::If { condition, then, else_ });

        self.switch_to_block(then);
        let then_value = self.expression(then_expr);
        self.terminate_block(TerminatorInstruction::Jmp(end, vec![then_value]));

        self.switch_to_block(else_);
        let else_value = self.decision_tree(else_tree);
        self.terminate_block(TerminatorInstruction::Jmp(end, vec![else_value]));

        self.switch_to_block(end);
        Value::Parameter(end, 0)
    }

    fn switch(&mut self, tag: PathId, cases: Vec<Case>, else_: Option<Box<DecisionTree>>) -> Value {
        let int_value = self.variable(tag);
        let start = self.current_block;

        let case_blocks = vecmap(&cases, |_| self.push_block(0));
        let mut else_block = None;
        let end = self.push_block(1);

        for (case_block, case) in case_blocks.iter().zip(cases) {
            self.switch_to_block(*case_block);
            // TODO: Bind constructor arguments
            let result = self.decision_tree(case.body);
            self.terminate_block(TerminatorInstruction::Jmp(end, vec![result]));
        }

        if let Some(else_) = else_ {
            let block = self.push_block(0);
            else_block = Some(block);
            self.switch_to_block(block);
            let result = self.decision_tree(*else_);
            self.terminate_block(TerminatorInstruction::Jmp(end, vec![result]));
        }

        self.switch_to_block(start);
        self.terminate_block(TerminatorInstruction::Switch { int_value, cases: case_blocks, else_: else_block });
        self.switch_to_block(end);
        Value::Parameter(end, 0)
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
        self.push_instruction(Instruction::MakeTuple(fields))
    }

    fn quoted(&self, _quoted: &cst::Quoted) -> Value {
        unreachable!("Should never convert a Quoted expr to mir")
    }

    /// Bind the given value to the given pattern
    fn bind_pattern(&mut self, pattern: PatternId, value: Value) {
        match &self.context()[pattern] {
            cst::Pattern::Error => unreachable!("Error pattern encountered in bind_pattern"),
            cst::Pattern::Variable(name) => {
                let origin = self.context().name_origin(*name);
                self.variables.insert(origin, value);
            },
            cst::Pattern::Literal(_) => (),
            cst::Pattern::Constructor(_type, _arguments) => {
                todo!("Constructors")
            },
            cst::Pattern::TypeAnnotation(pattern, _) => self.bind_pattern(*pattern, value),
            cst::Pattern::MethodName { type_name: _, item_name } => {
                let origin = self.context().name_origin(*item_name);
                self.variables.insert(origin, value);
            },
        }
    }

    fn finish(self) -> FxHashMap<FunctionId, Function> {
        self.finished_functions
    }
}
