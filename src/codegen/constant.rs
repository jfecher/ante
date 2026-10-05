//! Backend-neutral constant evaluation of MIR globals.
//!
//! A [mir::Definition] that [is a global](mir::Definition::is_global) is a single block whose
//! instructions are all constant-foldable and whose `Result` terminator names the value the
//! global holds. This module folds such a global into a [ConstantValue] tree without committing
//! to any particular backend representation, so each backend can render the result however it
//! likes (the C backend emits a file-scope initializer; the LLVM backend can later replace its
//! inline `codegen_constant_instruction` with a call here plus a `BasicValueEnum` renderer).

use rustc_hash::FxHashMap;

use crate::{
    lexer::token::{F64, FloatKind, IntegerKind},
    mir::{self, BlockId, DefinitionId, InstructionId, TerminatorInstruction, Type, Value},
};

/// The constant value a global evaluates to. Variants cover exactly the instructions that are
/// constant-foldable in a global initializer.
#[derive(Debug, Clone)]
pub(crate) enum ConstantValue {
    Unit,
    Bool(bool),
    Char(char),
    Int(mir::IntConstant),
    Float(mir::FloatConstant),
    Tuple(Vec<ConstantValue>),
    Array {
        elements: Vec<ConstantValue>,
        #[cfg(feature = "llvm")]
        element_type: Type,
    },
    /// The bytes of a string literal in immutable static storage
    Bytes(Vec<u8>),
    /// A reference to a global or function
    Definition(DefinitionId),
    /// An external symbol
    Extern {
        name: String,
        typ: Type,
    },
    /// A value of a `shared type`
    Shared {
        value: Box<ConstantValue>,
        typ: Type,
        cell: SharedCell,
    },
    /// A value of `typ` whose bytes are all zero
    Zeroed {
        typ: Type,
    },
    /// A pointer of type `typ` with the address `bits`
    IntToPtr {
        bits: u64,
        typ: Type,
    },
    /// The low bytes of `pointer` as an integer no wider than a Usz
    PtrToInt {
        pointer: Box<ConstantValue>,
        kind: IntegerKind,
    },
    /// `value` of type `from` with its bytes reinterpreted as a `to`
    Reinterpret {
        value: Box<ConstantValue>,
        from: Type,
        to: Type,
    },
}

/// Used so that a global folded into another keeps its original pointer.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SharedCell {
    /// The global whose initializer allocates the cell
    pub(crate) global: DefinitionId,

    /// Numbers `global`'s cells in the order they are evaluated
    pub(crate) index: u32,
}

/// Fold a global definition into a [ConstantValue]. Panics if the definition contains an
/// instruction that is not constant-foldable in a global initializer.
pub(crate) fn evaluate_global(mir: &mir::Mir, global: &mir::Definition, ptr_size: u32) -> ConstantValue {
    let mut values = FxHashMap::default();
    let mut next_cell = SharedCell { global: global.id, index: 0 };
    for id in global.entry_block().instructions.iter().copied() {
        let value = evaluate_instruction(mir, global, id, &values, &mut next_cell, ptr_size);
        values.insert(Value::InstructionResult(id), value);
    }

    let TerminatorInstruction::Result(result) = global.entry_block().terminator.as_ref().unwrap() else {
        panic!("Global definition missing Result terminator");
    };
    constant_value(*result, &values)
}

/// Resolve a [Value] to a [ConstantValue], reading instruction/parameter results from `values`.
fn constant_value(value: Value, values: &FxHashMap<Value, ConstantValue>) -> ConstantValue {
    match value {
        Value::Unit => ConstantValue::Unit,
        Value::Bool(b) => ConstantValue::Bool(b),
        Value::Char(c) => ConstantValue::Char(c),
        Value::Integer(constant) => ConstantValue::Int(constant),
        Value::Float(constant) => ConstantValue::Float(constant),
        Value::InstructionResult(_) | Value::Parameter(..) => {
            values.get(&value).cloned().unwrap_or_else(|| panic!("constant value not cached: {value}"))
        },
        Value::Definition(id) => ConstantValue::Definition(id),
        Value::Error => unreachable!("Error value in global initializer"),
    }
}

fn evaluate_instruction(
    mir: &mir::Mir, definition: &mir::Definition, id: InstructionId, values: &FxHashMap<Value, ConstantValue>,
    next_cell: &mut SharedCell, ptr_size: u32,
) -> ConstantValue {
    match &definition.instructions[id] {
        mir::Instruction::MakeTuple(fields) => {
            ConstantValue::Tuple(fields.iter().map(|f| constant_value(*f, values)).collect())
        },
        mir::Instruction::MakeArray(elements) => ConstantValue::Array {
            elements: elements.iter().map(|e| constant_value(*e, values)).collect(),
            #[cfg(feature = "llvm")]
            element_type: match definition.instruction_result_type(id) {
                Type::Array { element, .. } => (**element).clone(),
                other => panic!("MakeArray result type is not an array: {other}"),
            },
        },
        mir::Instruction::IndexTuple { tuple, index } => {
            let typ = definition.instruction_result_type(id);
            let tuple_type = mir.type_of_value(tuple, definition);
            index_tuple(mir, constant_value(*tuple, values), &tuple_type, *index as usize, typ, ptr_size)
        },
        mir::Instruction::MakeBytes(bytes) => ConstantValue::Bytes(bytes.clone()),
        mir::Instruction::Id(value) => constant_value(*value, values),
        mir::Instruction::Transmute(value) => {
            let from = mir.type_of_value(value, definition);
            let to = definition.instruction_result_type(id);
            let value = constant_value(*value, values);
            let mut image = Image::new(mir, ptr_size);
            let folded = image.write(&value, &from, 0).and_then(|()| image.read(to, 0));
            folded.unwrap_or_else(|| ConstantValue::Reinterpret { value: Box::new(value), from, to: to.clone() })
        },
        mir::Instruction::Extern(name) => {
            ConstantValue::Extern { name: name.clone(), typ: definition.instruction_result_type(id).clone() }
        },
        mir::Instruction::AllocShared(value) => {
            let typ = mir.type_of_value(value, definition);
            let cell = *next_cell;
            next_cell.index += 1;
            ConstantValue::Shared { value: Box::new(constant_value(*value, values)), typ, cell }
        },
        mir::Instruction::Call { function, arguments } => {
            // Constructor-style calls appear in `implicit` globals after monomorphization. The
            // callee is a single-block, constant-foldable function, so inline it: bind its entry
            // parameters to the argument values and fold its body.
            let (callee_id, _) = definition
                .definition_of(*function)
                .unwrap_or_else(|| panic!("Call in global initializer to non-resolvable function value: {function}"));
            let callee = mir
                .definitions
                .get(&callee_id)
                .unwrap_or_else(|| panic!("Call in global initializer: target definition {callee_id} not found"));
            assert!(
                callee.blocks.len() == 1,
                "Call in global initializer to non-constant-evaluable function `{}`: callee has multiple blocks",
                callee.name
            );
            let callee_result = match callee.entry_block().terminator.as_ref().expect("missing callee terminator") {
                TerminatorInstruction::Return(v) | TerminatorInstruction::Result(v) => *v,
                _ => panic!(
                    "Call in global initializer to non-constant-evaluable function `{}`: terminator is not Result/Return",
                    callee.name
                ),
            };

            let mut callee_values = FxHashMap::default();
            for (i, argument) in arguments.iter().enumerate() {
                let value = constant_value(*argument, values);
                callee_values.insert(Value::Parameter(BlockId::ENTRY_BLOCK, i as u32), value);
            }
            for instr_id in callee.entry_block().instructions.iter().copied() {
                let value = evaluate_instruction(mir, callee, instr_id, &callee_values, next_cell, ptr_size);
                callee_values.insert(Value::InstructionResult(instr_id), value);
            }
            constant_value(callee_result, &callee_values)
        },
        other => panic!("Unsupported instruction in global initializer: {other:?}"),
    }
}

/// Whether `value` can be emitted as a C file-scope (static) initializer, which must be a
/// constant expression. Reading another global *variable's* stored value is not constant, so a
/// [ConstantValue::Definition] referring to a global is rejected; a reference to a function decays
/// to its (constant) address and is fine. A [ConstantValue::Shared] backs itself with a `static`
/// whose own initializer must likewise be constant, so it inherits its inner value's constness.
pub(crate) fn is_c_constant(value: &ConstantValue, mir: &mir::Mir) -> bool {
    match value {
        ConstantValue::PtrToInt { .. } | ConstantValue::Reinterpret { .. } => false,
        // A function's address is constant but reading an extern variable is not
        ConstantValue::Extern { typ, .. } => matches!(typ, mir::Type::Function(_)),
        ConstantValue::Definition(id) => !mir.definitions.get(id).is_some_and(|d| d.is_global()),
        other => other.children().iter().all(|v| is_c_constant(v, mir)),
    }
}

/// Collect into `out` every other global variable this value reads by value.
/// These are the globals whose runtime initialization must precede this one's. Functions are skipped.
pub(crate) fn referenced_globals(value: &ConstantValue, mir: &mir::Mir, out: &mut Vec<DefinitionId>) {
    if let ConstantValue::Definition(id) = value
        && mir.definitions.get(id).is_some_and(|d| d.is_global())
    {
        out.push(*id);
    }
    value.children().iter().for_each(|v| referenced_globals(v, mir, out));
}

impl ConstantValue {
    fn children(&self) -> &[ConstantValue] {
        match self {
            ConstantValue::Tuple(values) | ConstantValue::Array { elements: values, .. } => values,
            ConstantValue::Shared { value, .. } | ConstantValue::Reinterpret { value, .. } => {
                std::slice::from_ref(value)
            },
            ConstantValue::PtrToInt { pointer, .. } => std::slice::from_ref(pointer),
            _ => &[],
        }
    }
}

/// Field `index` of the constant `tuple`
fn index_tuple(
    mir: &mir::Mir, tuple: ConstantValue, tuple_type: &Type, index: usize, field_type: &Type, ptr_size: u32,
) -> ConstantValue {
    match tuple {
        ConstantValue::Definition(id) if mir.definitions.get(&id).is_some_and(|d| d.is_global()) => {
            let tuple = evaluate_global(mir, &mir.definitions[&id], ptr_size);
            index_tuple(mir, tuple, tuple_type, index, field_type, ptr_size)
        },
        ConstantValue::Tuple(mut fields) => fields.swap_remove(index),
        ConstantValue::Zeroed { .. } => ConstantValue::Zeroed { typ: field_type.clone() },
        other => {
            let Type::Tuple(fields) = tuple_type else { panic!("IndexTuple of a non-tuple `{tuple_type}`") };
            let offset = Type::field_offsets(fields, ptr_size)[index] as usize;
            let mut image = Image::new(mir, ptr_size);
            let field = image.write(&other, tuple_type, 0).and_then(|()| image.read(field_type, offset));
            field.unwrap_or_else(|| panic!("cannot fold a field of {other:?} in a global initializer"))
        },
    }
}

/// The bytes of a constant being transmuted, with pointers kept symbolically. Unwritten bytes are zero.
struct Image<'a> {
    mir: &'a mir::Mir,
    ptr_size: u32,
    bytes: Vec<u8>,
    pointers: std::collections::BTreeMap<usize, ConstantValue>,
}

impl<'a> Image<'a> {
    fn new(mir: &'a mir::Mir, ptr_size: u32) -> Self {
        Self { mir, ptr_size, bytes: Vec::new(), pointers: Default::default() }
    }

    fn field_offsets(&self, fields: &[Type]) -> Vec<u32> {
        Type::field_offsets(fields, self.ptr_size)
    }

    fn write_bytes(&mut self, offset: usize, bytes: &[u8]) {
        if self.bytes.len() < offset + bytes.len() {
            self.bytes.resize(offset + bytes.len(), 0);
        }
        self.bytes[offset..offset + bytes.len()].copy_from_slice(bytes);
    }

    /// The pointers overlapping `length` bytes at `offset`
    fn overlapping_pointers(&self, offset: usize, length: usize) -> impl Iterator<Item = (&usize, &ConstantValue)> {
        self.pointers.range(offset.saturating_sub(self.ptr_size as usize - 1)..offset + length)
    }

    fn read_bytes(&self, offset: usize, length: usize) -> Option<[u8; 8]> {
        if self.overlapping_pointers(offset, length).next().is_some() {
            return None;
        }
        let mut bytes = [0; 8];
        for (i, byte) in bytes.iter_mut().take(length).enumerate() {
            *byte = self.bytes.get(offset + i).copied().unwrap_or(0);
        }
        Some(bytes)
    }

    fn write(&mut self, value: &ConstantValue, typ: &Type, offset: usize) -> Option<()> {
        use mir::PrimitiveType as P;
        match (value, typ) {
            (ConstantValue::Definition(id), _) if self.mir.definitions.get(id).is_some_and(|d| d.is_global()) => {
                let value = evaluate_global(self.mir, &self.mir.definitions[id], self.ptr_size);
                self.write(&value, typ, offset)?;
            },
            (ConstantValue::Zeroed { .. }, _) => (),
            (ConstantValue::Int(int), _) => {
                let size = int.kind().size_in_bytes(self.ptr_size) as usize;
                self.write_bytes(offset, &int.as_u64().to_le_bytes()[..size]);
            },
            (ConstantValue::Float(mir::FloatConstant::F32(float)), _) => {
                self.write_bytes(offset, &(float.0 as f32).to_bits().to_le_bytes());
            },
            (ConstantValue::Float(mir::FloatConstant::F64(float)), _) => {
                self.write_bytes(offset, &float.0.to_bits().to_le_bytes());
            },
            (ConstantValue::Bool(b), _) => self.write_bytes(offset, &[*b as u8]),
            (ConstantValue::Char(c), _) => self.write_bytes(offset, &[*c as u8]),
            (ConstantValue::Unit, _) => (),
            (ConstantValue::Tuple(values), Type::Tuple(fields)) => {
                for ((value, field), field_offset) in values.iter().zip(fields.iter()).zip(self.field_offsets(fields)) {
                    self.write(value, field, offset + field_offset as usize)?;
                }
            },
            (ConstantValue::Array { elements, .. }, Type::Array { element, .. }) => {
                let stride = element.stride(self.ptr_size) as usize;
                for (i, value) in elements.iter().enumerate() {
                    self.write(value, element, offset + i * stride)?;
                }
            },
            (ConstantValue::IntToPtr { bits, .. }, _) => {
                self.write_bytes(offset, &bits.to_le_bytes()[..self.ptr_size as usize]);
            },
            (ConstantValue::PtrToInt { pointer, kind }, _) if kind.size_in_bytes(self.ptr_size) == self.ptr_size => {
                self.pointers.insert(offset, (**pointer).clone());
            },
            (ConstantValue::Reinterpret { value, from, .. }, _) => self.write(value, from, offset)?,
            (_, Type::Primitive(P::Pointer) | Type::Function(_)) => {
                self.pointers.insert(offset, value.clone());
            },
            _ => return None,
        }
        Some(())
    }

    fn read(&self, typ: &Type, offset: usize) -> Option<ConstantValue> {
        use mir::{IntConstant as I, PrimitiveType as P};
        Some(match typ {
            Type::Primitive(P::Int(kind)) => {
                let size = kind.size_in_bytes(self.ptr_size) as usize;

                // TODO: This assumes the target is little-endian
                let mut pointers = self.overlapping_pointers(offset, size);
                if let Some((pointer_offset, pointer)) = pointers.next() {
                    let one_pointer = pointers.next().is_none();
                    return (*pointer_offset == offset && size <= self.ptr_size as usize && one_pointer)
                        .then(|| ConstantValue::PtrToInt { pointer: Box::new(pointer.clone()), kind: *kind });
                }
                let bits = u64::from_le_bytes(self.read_bytes(offset, size)?);
                ConstantValue::Int(match kind {
                    IntegerKind::U8 => I::U8(bits as u8),
                    IntegerKind::U16 => I::U16(bits as u16),
                    IntegerKind::U32 => I::U32(bits as u32),
                    IntegerKind::U64 => I::U64(bits),
                    IntegerKind::Usz => I::Usz(bits as usize),
                    IntegerKind::I8 => I::I8(bits as i8),
                    IntegerKind::I16 => I::I16(bits as i16),
                    IntegerKind::I32 => I::I32(bits as i32),
                    IntegerKind::I64 => I::I64(bits as i64),
                    IntegerKind::Isz => I::Isz(bits as isize),
                })
            },
            Type::Primitive(P::Float(FloatKind::F32)) => {
                let bits = u32::from_le_bytes(self.read_bytes(offset, 4)?[..4].try_into().unwrap());
                ConstantValue::Float(mir::FloatConstant::F32(F64(f32::from_bits(bits) as f64)))
            },
            Type::Primitive(P::Float(FloatKind::F64)) => {
                let bits = u64::from_le_bytes(self.read_bytes(offset, 8)?);
                ConstantValue::Float(mir::FloatConstant::F64(F64(f64::from_bits(bits))))
            },
            Type::Primitive(P::Bool) => ConstantValue::Bool(self.read_bytes(offset, 1)?[0] != 0),
            Type::Primitive(P::Char) => ConstantValue::Char(self.read_bytes(offset, 1)?[0] as char),
            Type::Primitive(P::Unit) => ConstantValue::Unit,
            Type::Primitive(P::Pointer) | Type::Function(_) => match self.pointers.get(&offset) {
                Some(pointer) if self.overlapping_pointers(offset, self.ptr_size as usize).count() == 1 => {
                    pointer.clone()
                },
                Some(_) => return None,
                None => match u64::from_le_bytes(self.read_bytes(offset, self.ptr_size as usize)?) {
                    0 => ConstantValue::Zeroed { typ: typ.clone() },
                    bits => ConstantValue::IntToPtr { bits, typ: typ.clone() },
                },
            },
            Type::Tuple(fields) => {
                let offsets = self.field_offsets(fields);
                let fields = fields.iter().zip(offsets);
                let fields = fields.map(|(field, field_offset)| self.read(field, offset + field_offset as usize));
                ConstantValue::Tuple(fields.collect::<Option<_>>()?)
            },
            Type::Array { length, element } => {
                let Type::U32(length) = length.as_ref() else { return None };
                let stride = element.stride(self.ptr_size) as usize;
                let elements = (0..*length as usize).map(|i| self.read(element, offset + i * stride));
                ConstantValue::Array {
                    elements: elements.collect::<Option<_>>()?,
                    #[cfg(feature = "llvm")]
                    element_type: (**element).clone(),
                }
            },
            _ => return None,
        })
    }
}
