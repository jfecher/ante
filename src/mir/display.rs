use std::fmt::{Display, Formatter, Result};

use crate::{
    iterator_extensions::mapvec,
    mir::{
        self, Block, BlockId, Definition, DefinitionId, FloatConstant, InstructionId, IntConstant, PrimitiveType, Type,
        Value,
    },
};

impl Display for mir::Mir {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for (id, extern_) in self.externals.iter() {
            writeln!(f, "extern {} {id}: {}", extern_.name, extern_.typ)?;
        }

        if !self.externals.is_empty() {
            writeln!(f)?;
        }

        for function in self.definitions.values() {
            fmt_definition(function, Some(self), f)?;
            writeln!(f, "\n")?;
        }
        Ok(())
    }
}

impl Definition {
    /// Create a wrapper that can display this [Definition]. The optional [mir::Mir], if provided, allows
    /// definition names to be printed instead of just their ids.
    pub(crate) fn display<'a>(&'a self, mir: Option<&'a mir::Mir>) -> DefinitionDisplay<'a> {
        DefinitionDisplay(self, mir)
    }
}

pub struct DefinitionDisplay<'a>(&'a Definition, Option<&'a mir::Mir>);

impl Display for DefinitionDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        fmt_definition(self.0, self.1, f)
    }
}

impl InstructionId {
    /// Create a wrapper that can display this instruction. The optional [mir::Mir], if provided, allows
    /// definition names to be printed instead of just their ids.
    pub(crate) fn display<'a>(self, definition: &'a Definition, mir: Option<&'a mir::Mir>) -> InstructionDisplay<'a> {
        InstructionDisplay(self, definition, mir)
    }
}

pub struct InstructionDisplay<'a>(InstructionId, &'a Definition, Option<&'a mir::Mir>);

impl Display for InstructionDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let data = &self.1.instructions[self.0];
        fmt_instruction(self.0, data, self.2, self.1, f)
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

impl Display for DefinitionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "d{}", self.0)
    }
}

impl Display for BlockId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "b{}", self.0)
    }
}

fn is_atom(t: &Type) -> bool {
    matches!(t, Type::Primitive(_) | Type::Generic(_) | Type::Union(_))
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Type::Primitive(primitive_type) => primitive_type.fmt(f),
            Type::Tuple(items) => {
                let mut type_string =
                    mapvec(items.iter(), |typ| if is_atom(typ) { typ.to_string() } else { format!("({typ})") })
                        .join(", ");

                // Make single-element tuples distinct from other types
                if items.len() == 1 {
                    type_string.push(',');
                }

                if type_string.is_empty() { write!(f, "#empty_tuple") } else { write!(f, "{type_string}") }
            },
            Type::Function(function_type) => write!(f, "{function_type}"),
            Type::Generic(id) => write!(f, "'{}", id.0),
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

impl Display for crate::mir::FunctionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "fn")?;
        for parameter in &self.parameters {
            write!(f, " ")?;
            if is_atom(parameter) {
                write!(f, "{parameter}")?;
            } else {
                write!(f, "({parameter})")?;
            }
        }

        if let Some(env) = self.environment() {
            write!(f, " [{env}]")?;
        }

        if is_atom(&self.return_type) {
            write!(f, " -> {}", self.return_type)
        } else {
            write!(f, " -> ({})", self.return_type)
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
            PrimitiveType::NoClosureEnv => write!(f, "NoClosureEnv"),
        }
    }
}

fn fmt_definition(function: &mir::Definition, mir: Option<&mir::Mir>, f: &mut Formatter) -> Result {
    if function.is_global() {
        write!(f, "let ")?;
    } else {
        write!(f, "fn ")?;
    }
    write!(f, "{} {}: ", function.name, function.id)?;

    if function.generic_count != 0 {
        write!(f, "forall")?;
        for i in 0..function.generic_count {
            write!(f, " {}", Type::generic(i))?;
        }
        write!(f, ". ")?;
    }

    write!(f, "{}", function.typ)?;

    for (block_id, block) in function.blocks.iter() {
        writeln!(f)?;
        fmt_block(block_id, mir, function, block, f)?;
    }
    Ok(())
}

fn fmt_block(
    id: BlockId, mir: Option<&mir::Mir>, function: &mir::Definition, block: &Block, f: &mut Formatter,
) -> Result {
    write!(f, "  b{}(", id.0)?;
    let v = |value: &Value| ValueDisplay { value: *value, mir };

    for (i, typ) in block.parameter_types.iter().enumerate() {
        if i != 0 {
            write!(f, ", ")?;
        }
        write!(f, "{}: {typ}", v(&Value::Parameter(id, i as u32)))?;
    }
    writeln!(f, "):")?;

    for instruction_id in block.instructions.iter().copied() {
        let instruction = &function.instructions[instruction_id];
        fmt_instruction(instruction_id, instruction, mir, function, f)?;
    }

    match block.terminator.as_ref() {
        Some(terminator) => fmt_terminator(terminator, mir, f)?,
        None => write!(f, "  (no terminator)")?,
    }

    Ok(())
}

fn fmt_terminator(terminator: &mir::TerminatorInstruction, mir: Option<&mir::Mir>, f: &mut Formatter<'_>) -> Result {
    let v = |value: &Value| ValueDisplay { value: *value, mir };
    write!(f, "    ")?;

    match terminator {
        mir::TerminatorInstruction::Jmp((block_id, argument)) => {
            write!(f, "jmp {block_id}")?;
            if let Some(argument) = argument {
                write!(f, " {}", v(argument))?;
            }
            Ok(())
        },
        mir::TerminatorInstruction::If { condition, then, else_, end } => {
            write!(f, "if {} then {}", v(condition), then.0)?;
            if let Some(argument) = then.1 {
                write!(f, " {}", v(&argument))?;
            }
            write!(f, " else {}", else_.0)?;
            if let Some(argument) = else_.1 {
                write!(f, " {}", v(&argument))?;
            }
            write!(f, " end {end}")
        },
        mir::TerminatorInstruction::Unreachable => write!(f, "unreachable"),
        mir::TerminatorInstruction::Return(value) => write!(f, "return {}", v(value)),
        mir::TerminatorInstruction::Result(value) => write!(f, "result {}", v(value)),
        mir::TerminatorInstruction::Switch { int_value, cases, else_, end } => {
            writeln!(f, "switch {}", v(int_value))?;
            for (i, (case_value, (case_block, case_arg))) in cases.iter().enumerate() {
                if i != 0 {
                    writeln!(f)?;
                }
                write!(f, "    | {case_value} -> {case_block}")?;
                if let Some(arg) = case_arg {
                    write!(f, " {}", v(arg))?;
                }
            }
            if let Some((else_block, else_arg)) = else_ {
                write!(f, "\n    | _ -> {else_block}")?;
                if let Some(arg) = else_arg {
                    write!(f, " {}", v(arg))?;
                }
            }
            write!(f, "\n    end {end}")
        },
    }
}

fn fmt_instruction(
    instruction_id: mir::InstructionId, instruction: &mir::Instruction, mir: Option<&mir::Mir>,
    function: &mir::Definition, f: &mut Formatter<'_>,
) -> Result {
    let v = |value: &Value| ValueDisplay { value: *value, mir };

    let result_type = &function.instruction_result_types[instruction_id];
    write!(f, "    {}: {result_type} = ", v(&Value::InstructionResult(instruction_id)))?;

    match instruction {
        mir::Instruction::Call { function, arguments } => {
            write!(f, "{}", v(function))?;
            for argument in arguments {
                write!(f, " {}", v(argument))?;
            }
        },
        mir::Instruction::CallClosure { closure, arguments } => {
            write!(f, "closure-call {}", v(closure))?;
            for argument in arguments {
                write!(f, " {}", v(argument))?;
            }
        },
        mir::Instruction::Perform { effect_op, arguments } => {
            write!(f, "perform {effect_op}")?;
            for argument in arguments {
                write!(f, " {}", v(argument))?;
            }
        },
        mir::Instruction::Handle { body, cases } => {
            write!(f, "handle {}", v(body))?;
            for case in cases {
                write!(f, " | {} -> {}", case.effect_op, v(&case.handler))?;
            }
        },
        mir::Instruction::HandlerCap => write!(f, "handler_cap")?,
        mir::Instruction::PackClosure { function, environment } => {
            write!(f, "pack-closure {}, {}", function, environment)?;
        },
        mir::Instruction::IndexTuple { tuple, index } => write!(f, "{}.{index}", v(tuple))?,
        mir::Instruction::MakeTuple(fields) => write!(f, "({})", comma_separated(fields, mir))?,
        mir::Instruction::MakeString(s) => write!(f, "\"{s}\"")?,
        mir::Instruction::StackAlloc(value) => write!(f, "alloca {}", v(value))?,
        mir::Instruction::Store { pointer, value } => write!(f, "store {}, {}", v(pointer), v(value))?,
        mir::Instruction::GetFieldPtr { struct_ptr, index, .. } => write!(f, "field_ptr {}.{index}", v(struct_ptr))?,
        mir::Instruction::Transmute(value) => write!(f, "transmute {}", v(value))?,
        mir::Instruction::Id(value) => write!(f, "id {}", v(value))?,
        mir::Instruction::Instantiate(definition_id, generics) => {
            write!(f, "instantiate {definition_id}")?;
            for generic in generics.iter() {
                write!(f, " {}", generic)?;
            }
        },
        mir::Instruction::AddInt(a, b) => write!(f, "add_int {}, {}", v(a), v(b))?,
        mir::Instruction::AddFloat(a, b) => write!(f, "add_float {}, {}", v(a), v(b))?,
        mir::Instruction::SubInt(a, b) => write!(f, "sub_int {}, {}", v(a), v(b))?,
        mir::Instruction::SubFloat(a, b) => write!(f, "sub_float {}, {}", v(a), v(b))?,
        mir::Instruction::MulInt(a, b) => write!(f, "mul_int {}, {}", v(a), v(b))?,
        mir::Instruction::MulFloat(a, b) => write!(f, "mul_float {}, {}", v(a), v(b))?,
        mir::Instruction::DivSigned(a, b) => write!(f, "div_signed {}, {}", v(a), v(b))?,
        mir::Instruction::DivUnsigned(a, b) => write!(f, "div_unsigned {}, {}", v(a), v(b))?,
        mir::Instruction::DivFloat(a, b) => write!(f, "div_float {}, {}", v(a), v(b))?,
        mir::Instruction::ModSigned(a, b) => write!(f, "mod_signed {}, {}", v(a), v(b))?,
        mir::Instruction::ModUnsigned(a, b) => write!(f, "mod_unsigned {}, {}", v(a), v(b))?,
        mir::Instruction::ModFloat(a, b) => write!(f, "mod_float {}, {}", v(a), v(b))?,
        mir::Instruction::LessSigned(a, b) => write!(f, "less_signed {}, {}", v(a), v(b))?,
        mir::Instruction::LessUnsigned(a, b) => write!(f, "less_unsigned {}, {}", v(a), v(b))?,
        mir::Instruction::LessFloat(a, b) => write!(f, "less_float {}, {}", v(a), v(b))?,
        mir::Instruction::EqInt(a, b) => write!(f, "eq_int {}, {}", v(a), v(b))?,
        mir::Instruction::EqFloat(a, b) => write!(f, "eq_float {}, {}", v(a), v(b))?,
        mir::Instruction::BitwiseAnd(a, b) => write!(f, "bitwise_and {}, {}", v(a), v(b))?,
        mir::Instruction::BitwiseOr(a, b) => write!(f, "bitwise_or{}, {}", v(a), v(b))?,
        mir::Instruction::BitwiseXor(a, b) => write!(f, "bitwise_xor {}, {}", v(a), v(b))?,
        mir::Instruction::BitwiseNot(x) => write!(f, "bitwise_not {}", v(x))?,
        mir::Instruction::SignExtend(x) => write!(f, "sign_extend {}", v(x))?,
        mir::Instruction::ZeroExtend(x) => write!(f, "zero_extend {}", v(x))?,
        mir::Instruction::SignedToFloat(x) => write!(f, "signed_to_float {}", v(x))?,
        mir::Instruction::UnsignedToFloat(x) => write!(f, "unsigned_to_float {}", v(x))?,
        mir::Instruction::FloatToSigned(x) => write!(f, "float_to_signed {}", v(x))?,
        mir::Instruction::FloatToUnsigned(x) => write!(f, "float_to_unsigned {}", v(x))?,
        mir::Instruction::FloatPromote(x) => write!(f, "float_promote {}", v(x))?,
        mir::Instruction::FloatDemote(x) => write!(f, "float_demote {}", v(x))?,
        mir::Instruction::Truncate(x) => write!(f, "truncate {}", v(x))?,
        mir::Instruction::Deref(x) => write!(f, "deref {}", v(x))?,
        mir::Instruction::SizeOf(x) => write!(f, "size_of {x}")?,
        mir::Instruction::Extern(name) => write!(f, "extern \"{name}\"")?,
    }

    writeln!(f)
}

fn comma_separated(items: &[Value], mir: Option<&mir::Mir>) -> String {
    items.iter().map(|v| ValueDisplay { value: *v, mir }.to_string()).collect::<Vec<_>>().join(", ")
}

struct ValueDisplay<'local> {
    value: Value,
    mir: Option<&'local mir::Mir>,
}

impl<'local> Display for ValueDisplay<'local> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match &self.value {
            Value::Error => write!(f, "#error"),
            Value::Unit => write!(f, "()"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Char(c) => write!(f, "{c}"),
            Value::Integer(int) => write!(f, "{int}"),
            Value::Float(float) => write!(f, "{float}"),
            Value::InstructionResult(instruction_id) => write!(f, "v{}", instruction_id.0),
            Value::Parameter(block_id, i) => write!(f, "b{}_{}", block_id.0, i),
            Value::Definition(id) => {
                if let Some(name) = self.mir.as_ref().and_then(|mir| mir.get_name(*id)) {
                    write!(f, "{name}_")?;
                }
                write!(f, "{id}")
            },
        }
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
            Value::Definition(id) => write!(f, "{id}"),
        }
    }
}
