use std::fmt::{Display, Formatter, Result};

use crate::{
    iterator_extensions::vecmap,
    mir::{self, Block, BlockId, FloatConstant, FunctionId, IntConstant, PrimitiveType, Type, Value},
};

impl Display for mir::Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt_function(self, f)
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Value::Error => write!(f, "#error"),
            Value::Unit => write!(f, "()"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Char(c) => write!(f, "{c}"),
            Value::Integer(int) => write!(f, "{int}"),
            Value::Float(float) => write!(f, "{float}"),
            Value::InstructionResult(instruction_id) => write!(f, "v{}", instruction_id.0),
            Value::Parameter(block_id, i) => write!(f, "b{}_{}", block_id.0, i),
            Value::Function(id) => write!(f, "f{id}"),
            Value::Global(name) => write!(f, "g{name}"),
        }
    }
}

impl Display for IntConstant {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            IntConstant::U8(x) => write!(f, "{x}_u8"),
            IntConstant::U16(x) => write!(f, "{x}_u16"),
            IntConstant::U32(x) => write!(f, "{x}_u32"),
            IntConstant::U64(x) => write!(f, "{x}_u64"),
            IntConstant::Usz(x) => write!(f, "{x}_usz"),
            IntConstant::I8(x) => write!(f, "{x}_i8"),
            IntConstant::I16(x) => write!(f, "{x}_i16"),
            IntConstant::I32(x) => write!(f, "{x}_i32"),
            IntConstant::I64(x) => write!(f, "{x}_i64"),
            IntConstant::Isz(x) => write!(f, "{x}_isz"),
        }
    }
}

impl Display for FloatConstant {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            FloatConstant::F32(x) => write!(f, "{x}_f32"),
            FloatConstant::F64(x) => write!(f, "{x}_f64"),
        }
    }
}

impl Display for FunctionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "f{}", self.item)?;
        if self.index != 0 {
            write!(f, "_{}", self.index)?;
        }
        Ok(())
    }
}

impl Display for BlockId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "b{}", self.0)
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let is_atom = |t: &Type| matches!(t, Type::Primitive(_) | Type::Generic(_) | Type::Union(_));

        match self {
            Type::Primitive(primitive_type) => primitive_type.fmt(f),
            Type::Tuple(items) => {
                let mut type_string =
                    vecmap(items.iter(), |typ| if is_atom(typ) { typ.to_string() } else { format!("({typ})") })
                        .join(", ");

                // Make single-element tuples distinct from other types
                if items.len() == 1 {
                    type_string.push(',');
                }

                if type_string.is_empty() { write!(f, "#empty_tuple") } else { write!(f, "{type_string}") }
            },
            Type::Function(function_type) => {
                write!(f, "fn")?;
                for parameter in &function_type.parameters {
                    write!(f, " ")?;
                    if is_atom(parameter) {
                        write!(f, "{parameter}")?;
                    } else {
                        write!(f, "({parameter})")?;
                    }
                }

                if is_atom(&function_type.return_type) {
                    write!(f, " -> {}", function_type.return_type)
                } else {
                    write!(f, " -> ({})", function_type.return_type)
                }
            },
            Type::Generic(id) => write!(f, "{id}"),
            Type::Union(variants) => {
                write!(f, "{{")?;
                for (i, variant) in variants.iter().enumerate() {
                    if i != 0 {
                        write!(f, " | ")?;
                    }
                    if is_atom(variant) {
                        write!(f, "{variant}")?;
                    } else {
                        write!(f, "({variant})")?;
                    }
                }
                write!(f, "}}")
            },
        }
    }
}

impl Display for PrimitiveType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            PrimitiveType::Error => write!(f, "error"),
            PrimitiveType::Unit => write!(f, "Unit"),
            PrimitiveType::Bool => write!(f, "Bool"),
            PrimitiveType::Pointer => write!(f, "Pointer"),
            PrimitiveType::Char => write!(f, "Char"),
            PrimitiveType::Int(kind) => kind.fmt(f),
            PrimitiveType::Float(kind) => kind.fmt(f),
        }
    }
}

fn fmt_function(function: &mir::Function, f: &mut Formatter) -> Result {
    write!(f, "fun {} {}", function.name, function.id)?;
    for (block_id, block) in function.blocks.iter() {
        writeln!(f)?;
        fmt_block(block_id, function, block, f)?;
    }
    Ok(())
}

fn fmt_block(id: BlockId, function: &mir::Function, block: &Block, f: &mut Formatter) -> Result {
    write!(f, "  b{}(", id.0)?;
    for (i, typ) in block.parameter_types.iter().enumerate() {
        if i != 0 {
            write!(f, ", ")?;
        }
        write!(f, "{}: {}", Value::Parameter(id, i as u32), typ)?;
    }
    writeln!(f, "):")?;

    for instruction_id in block.instructions.iter().copied() {
        let instruction = &function.instructions[instruction_id];
        fmt_instruction(instruction_id, instruction, function, f)?;
    }

    match block.terminator.as_ref() {
        Some(terminator) => fmt_terminator(terminator, f)?,
        None => write!(f, "  (no terminator)")?,
    }

    Ok(())
}

fn fmt_terminator(terminator: &mir::TerminatorInstruction, f: &mut Formatter<'_>) -> Result {
    write!(f, "    ")?;
    match terminator {
        mir::TerminatorInstruction::Jmp((block_id, arguments)) => {
            write!(f, "jmp {block_id}")?;
            for argument in arguments {
                write!(f, " {argument}")?;
            }
            Ok(())
        },
        mir::TerminatorInstruction::If { condition, then, else_ } => {
            write!(f, "if {condition} then {}", then.0)?;
            for argument in &then.1 {
                write!(f, " {argument}")?;
            }
            write!(f, " else {}", else_.0)?;
            for argument in &else_.1 {
                write!(f, " {argument}")?;
            }
            Ok(())
        },
        mir::TerminatorInstruction::Unreachable => write!(f, "unreachable"),
        mir::TerminatorInstruction::Return(value) => write!(f, "return {value}"),
        mir::TerminatorInstruction::Switch { int_value, cases, else_ } => {
            writeln!(f, "switch {int_value}")?;
            for (i, (case_block, case_args)) in cases.iter().enumerate() {
                if i != 0 {
                    writeln!(f)?;
                }
                write!(f, "    | {i} -> {case_block}")?;
                for arg in case_args {
                    write!(f, " {arg}")?;
                }
            }
            if let Some((else_block, else_args)) = else_ {
                write!(f, "\n    | _ -> {else_block}")?;
                for arg in else_args {
                    write!(f, " {arg}")?;
                }
            }
            Ok(())
        },
    }
}

fn fmt_instruction(
    instruction_id: mir::InstructionId, instruction: &mir::Instruction, function: &mir::Function, f: &mut Formatter<'_>,
) -> Result {
    let result_type = &function.instruction_result_types[instruction_id];
    write!(f, "    {}: {result_type} = ", Value::InstructionResult(instruction_id))?;

    match instruction {
        mir::Instruction::Call { function, arguments } => {
            write!(f, "{function}")?;
            for argument in arguments {
                write!(f, " {argument}")?;
            }
        },
        mir::Instruction::IndexTuple { tuple, index } => write!(f, "{tuple}.{index}")?,
        mir::Instruction::MakeTuple(fields) => write!(f, "({})", comma_separated(fields))?,
        mir::Instruction::MakeString(s) => write!(f, "\"{s}\"")?,
        mir::Instruction::StackAlloc(value) => write!(f, "alloca {value}")?,
        mir::Instruction::Transmute(value) => write!(f, "transmute {value}")?,
    }

    writeln!(f)
}

fn comma_separated<T: ToString>(items: &[T]) -> String {
    vecmap(items, ToString::to_string).join(", ")
}
