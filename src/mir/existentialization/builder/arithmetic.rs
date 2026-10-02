//! Arithmetic on sizes, folded when both sides are known. Anything left for runtime goes in the
//! prologue, reusing an earlier identical result.

use crate::mir::{
    Instruction, IntConstant, Type, Value,
    existentialization::{builder::FunctionBuilder, types::usz},
};

/// A `Usz` or `U64` known either at compile time or only at runtime
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Op {
    Const(u64),
    Value(Value),
}

impl FunctionBuilder<'_> {
    pub(super) fn binary(
        &mut self, a: Op, b: Op, fold: fn(u64, u64) -> u64, instruction: fn(Value, Value) -> Instruction, typ: Type,
    ) -> Op {
        match (a, b) {
            (Op::Const(a), Op::Const(b)) => Op::Const(fold(a, b)),
            _ => {
                let (a, b) = (word_value(a, &typ), word_value(b, &typ));
                let instruction = instruction(a, b);
                Op::Value(self.arithmetic_in_prologue(instruction, a, b, typ))
            },
        }
    }

    /// Emit pure arithmetic into the prologue, reusing an earlier identical result
    fn arithmetic_in_prologue(&mut self, instruction: Instruction, a: Value, b: Value, typ: Type) -> Value {
        let key = (std::mem::discriminant(&instruction), a, b);
        if let Some(result) = self.cache.arithmetic.get(&key) {
            return *result;
        }
        let result = self.emit_prologue(instruction, typ);
        self.cache.arithmetic.insert(key, result);
        result
    }

    pub(crate) fn add(&mut self, a: Op, b: Op) -> Op {
        match (a, b) {
            (Op::Const(0), other) | (other, Op::Const(0)) => other,
            _ => self.binary(a, b, u64::wrapping_add, Instruction::AddInt, usz()),
        }
    }

    pub(super) fn mul(&mut self, a: Op, b: Op) -> Op {
        match (a, b) {
            (Op::Const(1), other) | (other, Op::Const(1)) => other,
            _ => self.binary(a, b, u64::wrapping_mul, Instruction::MulInt, usz()),
        }
    }

    pub(super) fn sub(&mut self, a: Op, b: Op) -> Op {
        self.binary(a, b, u64::wrapping_sub, Instruction::SubInt, usz())
    }

    fn and(&mut self, a: Op, b: Op) -> Op {
        match (a, b) {
            (Op::Const(0), _) | (_, Op::Const(0)) => Op::Const(0),
            _ => self.binary(a, b, |a, b| a & b, Instruction::BitwiseAnd, usz()),
        }
    }

    pub(super) fn or(&mut self, a: Op, b: Op) -> Op {
        match (a, b) {
            (Op::Const(0), other) | (other, Op::Const(0)) => other,
            _ if a == b => a,
            _ => self.binary(a, b, |a, b| a | b, Instruction::BitwiseOr, usz()),
        }
    }

    fn xor(&mut self, a: Op, b: Op) -> Op {
        self.binary(a, b, |a, b| a ^ b, Instruction::BitwiseXor, usz())
    }

    fn not(&mut self, a: Op) -> Op {
        match a {
            Op::Const(n) => Op::Const(!n),
            Op::Value(value) => {
                Op::Value(self.arithmetic_in_prologue(Instruction::BitwiseNot(value), value, value, usz()))
            },
        }
    }

    /// `x` rounded up to a multiple of the alignment `mask + 1`
    pub(super) fn align_up(&mut self, x: Op, mask: Op) -> Op {
        match (x, mask) {
            (Op::Const(0), _) | (_, Op::Const(0)) => x,
            _ => {
                let sum = self.add(x, mask);
                let inverse = self.not(mask);
                self.and(sum, inverse)
            },
        }
    }

    /// All ones if `a < b`, otherwise zero
    pub(super) fn less_mask(&mut self, a: Op, b: Op) -> Op {
        match (a, b) {
            (Op::Const(a), Op::Const(b)) => Op::Const(if a < b { u64::MAX } else { 0 }),
            _ => {
                let (a, b) = (usz_value(a), usz_value(b));
                let less = self.arithmetic_in_prologue(Instruction::LessUnsigned(a, b), a, b, Type::BOOL);
                let less = self.arithmetic_in_prologue(Instruction::ZeroExtend(less), less, less, usz());
                self.sub(Op::Const(0), Op::Value(less))
            },
        }
    }

    /// `then_` if `mask` is all ones, `else_` if it is zero
    pub(super) fn select(&mut self, mask: Op, then_: Op, else_: Op) -> Op {
        match mask {
            Op::Const(0) => else_,
            Op::Const(_) => then_,
            _ if then_ == else_ => then_,
            _ => {
                let difference = self.xor(then_, else_);
                let masked = self.and(difference, mask);
                self.xor(else_, masked)
            },
        }
    }

    pub(crate) fn min(&mut self, a: Op, b: Op) -> Op {
        let mask = self.less_mask(a, b);
        self.select(mask, a, b)
    }
}

pub(crate) fn usz_value(op: Op) -> Value {
    match op {
        Op::Const(n) => Value::Integer(IntConstant::Usz(n as usize)),
        Op::Value(value) => value,
    }
}

pub(super) fn u64_value(op: Op) -> Value {
    match op {
        Op::Const(n) => Value::Integer(IntConstant::U64(n)),
        Op::Value(value) => value,
    }
}

/// A word of a type info or type info table, whose type is either `usz` or `u64`
pub(super) fn word_value(word: Op, typ: &Type) -> Value {
    if *typ == usz() { usz_value(word) } else { u64_value(word) }
}
