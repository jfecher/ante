//! This module checks that the layouts existentialization computes at runtime from type infos
//! match the layouts of the same types once their type arguments are substituted in.
use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::{
    iterator_extensions::mapvec,
    lexer::token::{FloatKind, IntegerKind},
    mir::{
        BlockId, FunctionType, Instruction, Mir, PrimitiveType, TerminatorInstruction, Type, Value,
        existentialization::{Shared, builder::FunctionBuilder, types::Types},
    },
};

#[derive(Debug, Clone, PartialEq)]
enum Constant {
    Int(u64),
    Tuple(Vec<Constant>),
    /// A byte offset into the type info table
    Pointer(u64),
}

impl Constant {
    fn int(&self) -> u64 {
        match self {
            Constant::Int(n) => *n,
            other => panic!("{other:?} is not an integer"),
        }
    }
}

/// The size of a `(size, align, id)` type info with 8-byte pointers
const TYPE_INFO_SIZE: u64 = 24;

/// Evaluates the `value` in `definition`, which may only use arithmetic and tuples. Its
/// parameter points to a type info table of the type infos in `parameter`.
fn evaluate(definition: &crate::mir::Definition, value: &Value, parameter: &Constant) -> Constant {
    let mut values = FxHashMap::default();
    for id in &definition.entry_block().instructions {
        let get = |value: &Value| match value {
            Value::Integer(constant) => Constant::Int(constant.as_u64()),
            Value::Parameter(..) => Constant::Pointer(0),
            Value::InstructionResult(id) => values.get(id).cloned().unwrap_or_else(|| panic!("{id:?} not evaluated")),
            other => panic!("cannot evaluate `{other}`"),
        };
        let binary = |a: &Value, b: &Value, f: fn(u64, u64) -> u64| Constant::Int(f(get(a).int(), get(b).int()));
        let result = match &definition.instructions[*id] {
            Instruction::IndexTuple { tuple, index } => match get(tuple) {
                Constant::Tuple(fields) => fields[*index as usize].clone(),
                other => panic!("{other:?} is not a tuple"),
            },
            Instruction::MakeTuple(fields) => Constant::Tuple(mapvec(fields, get)),
            Instruction::PointerOffset { pointer, offset } => match get(pointer) {
                Constant::Pointer(base) => Constant::Pointer(base + get(offset).int()),
                other => panic!("{other:?} is not a pointer"),
            },
            Instruction::Deref(pointer) => match (get(pointer), parameter) {
                (Constant::Pointer(offset), Constant::Tuple(infos)) => {
                    let info = infos[(offset / TYPE_INFO_SIZE) as usize].clone();
                    match info {
                        Constant::Tuple(fields) => fields[(offset % TYPE_INFO_SIZE / 8) as usize].clone(),
                        other => panic!("{other:?} is not a type info"),
                    }
                },
                other => panic!("cannot load from {other:?}"),
            },
            Instruction::AddInt(a, b) => binary(a, b, u64::wrapping_add),
            Instruction::SubInt(a, b) => binary(a, b, u64::wrapping_sub),
            Instruction::MulInt(a, b) => binary(a, b, u64::wrapping_mul),
            Instruction::BitwiseAnd(a, b) => binary(a, b, |a, b| a & b),
            Instruction::BitwiseOr(a, b) => binary(a, b, |a, b| a | b),
            Instruction::BitwiseXor(a, b) => binary(a, b, |a, b| a ^ b),
            Instruction::LessUnsigned(a, b) => binary(a, b, |a, b| (a < b) as u64),
            Instruction::BitwiseNot(a) => Constant::Int(!get(a).int()),
            Instruction::ZeroExtend(a) => get(a),
            other => panic!("cannot evaluate `{other:?}`"),
        };
        values.insert(*id, result);
    }
    match value {
        Value::InstructionResult(id) => values[id].clone(),
        Value::Integer(constant) => Constant::Int(constant.as_u64()),
        other => panic!("cannot evaluate `{other}`"),
    }
}

/// The `(size, align, id)` type info of `typ` computed by `shared` with `bindings` given at runtime
fn type_info(shared: &Shared, typ: &Type, bindings: &[Type]) -> Constant {
    let needed = mapvec(0..bindings.len() as u32, |generic| generic);
    let name = Arc::new("test".to_string());
    let mut builder = FunctionBuilder::new(shared, name, crate::mir::next_definition_id(), Type::UNIT, &needed);
    builder.set_metadata(Value::Parameter(BlockId::ENTRY_BLOCK, 0), &[]);
    let info = builder.type_info(typ);

    // Returned so it is not removed as unused
    builder.function.blocks[BlockId::ENTRY_BLOCK].terminator = Some(TerminatorInstruction::Return(info));
    let definition = builder.finish();

    // The concrete bindings' own type infos, which need no runtime information
    let concrete = mapvec(bindings, |binding| type_info(shared, binding, &[]));
    evaluate(&definition, &info, &Constant::Tuple(concrete))
}

fn shared(mir: &Mir, reserved: FxHashMap<usize, usize>) -> Shared<'_> {
    Shared::new(mir, Default::default(), Types::new(8, reserved))
}

fn function(arity: usize, environment: Type) -> Type {
    let parameters = vec![Type::UNIT; arity];
    Type::Function(Arc::new(FunctionType { parameters, environment, return_type: Type::UNIT }))
}

#[test]
fn runtime_layouts_match_static_layouts() {
    let mir = Mir::default();
    let reserved = [(1, 2)].into_iter().collect();
    let shared = shared(&mir, reserved);

    let (g0, g1) = (Type::generic(0), Type::generic(1));
    let templates = [
        Type::tuple(vec![g0.clone(), Type::int(IntegerKind::U8), g1.clone()]),
        Type::tuple(vec![
            Type::int(IntegerKind::U8),
            Type::tuple(vec![g0.clone(), Type::int(IntegerKind::U16)]),
            g1.clone(),
        ]),
        Type::tuple(vec![
            Type::tag_type(),
            Type::union(vec![
                Type::tuple(vec![Type::int(IntegerKind::U8)]),
                Type::tuple(vec![g0.clone(), g1.clone()]),
                Type::tuple(vec![Type::int(IntegerKind::U32)]),
            ]),
        ]),
        Type::tuple(vec![g0.clone()]),
        Type::tuple(vec![g0.clone(), g1.clone()]),
        Type::array_with_length(Type::U32(3), Type::tuple(vec![g0.clone(), Type::BOOL])),
        function(1, Type::tuple(vec![g0.clone(), Type::POINTER])),
        function(1, g1.clone()),
    ];
    let bindings = [
        Type::int(IntegerKind::U8),
        Type::int(IntegerKind::U16),
        Type::int(IntegerKind::U64),
        Type::CHAR,
        Type::BOOL,
        Type::POINTER,
        Type::UNIT,
        Type::float(FloatKind::F32),
        Type::tuple(Vec::new()),
        Type::NO_CLOSURE_ENV,
        Type::tuple(vec![Type::int(IntegerKind::U8), Type::int(IntegerKind::U64)]),
        Type::tuple(vec![Type::int(IntegerKind::U16), Type::int(IntegerKind::U8), Type::int(IntegerKind::U8)]),
        Type::array_with_length(Type::U32(3), Type::int(IntegerKind::U16)),
        Type::union(vec![
            Type::tuple(vec![Type::int(IntegerKind::U8)]),
            Type::tuple(vec![Type::int(IntegerKind::U32), Type::CHAR]),
        ]),
        function(1, Type::tuple(vec![Type::int(IntegerKind::U32)])),
    ];

    for template in &templates {
        for first in &bindings {
            for second in &bindings {
                let arguments = vec![first.clone(), second.clone()];
                let substituted = template.substitute(&arguments);
                let expected = type_info(&shared, &substituted, &[]);
                let actual = type_info(&shared, template, &arguments);
                assert_eq!(actual, expected, "the layout of `{template}` with `{first}` and `{second}`");
            }
        }
    }
}

#[test]
fn nat_generic_array_lengths() {
    let mir = Mir::default();
    let shared = shared(&mir, Default::default());
    let template = Type::array_with_length(Type::generic(0), Type::tuple(vec![Type::generic(1), Type::BOOL]));
    for length in [0, 1, 5] {
        for element in [Type::int(IntegerKind::U8), Type::int(IntegerKind::U32), Type::tuple(Vec::new())] {
            let arguments = vec![Type::U32(length), element];
            let expected = type_info(&shared, &template.substitute(&arguments), &[]);
            let actual = type_info(&shared, &template, &arguments);
            assert_eq!(actual, expected, "the layout of `{template}` with {arguments:?}");
        }
    }
}

#[test]
fn plain_functions_are_closures_over_an_empty_environment() {
    let types = Types::new(8, [(1, 1)].into_iter().collect());
    let plain = function(1, Type::Primitive(PrimitiveType::NoClosureEnv));
    let closure = function(1, Type::UNIT);
    assert_eq!(types.static_layout(&plain), types.static_layout(&closure));
}
