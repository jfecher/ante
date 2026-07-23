use inc_complete::DbGet;

use crate::{
    incremental::{GetItem, GetItemRaw, TypeCheck},
    mir::{AtomicOrdering, AtomicRmwOp, Instruction, Value, builder::Context},
    name_resolution::{Origin, builtin::Builtin},
    parser::{
        cst::{self, Argument},
        ids::ExprId,
    },
    type_inference::types::Type as TCType,
};

impl<'local, Db> Context<'local, Db>
where
    Db: DbGet<TypeCheck> + DbGet<GetItem> + DbGet<GetItemRaw>,
{
    pub(super) fn try_lower_intrinsic(&mut self, call: &cst::Call, call_id: ExprId) -> Option<Value> {
        // Function must be the `intrinsic` defined only within the stdlib
        let cst::Expr::Variable(path) = &self.context()[call.function] else { return None };
        if !matches!(self.context().path_origin(*path), Some(Origin::Builtin(Builtin::Intrinsic))) {
            return None;
        }

        // Assumes the stdlib is well-formed: a string literal tag followed by 0+ other arguments.
        let cst::Expr::Literal(cst::Literal::String(intrinsic)) = &self.context()[call.arguments[0].expr] else {
            panic!("Malformed call to `intrinsic`")
        };

        let args = &call.arguments[1..];
        let result_type = self.convert_type(&self.types.result.maps.expr_types[&call_id], None);

        let push_1arg_ins = |this: &mut Self, f: fn(Value) -> Instruction| {
            assert_eq!(args.len(), 1);
            let arg = this.expression(args[0].expr);
            this.push_instruction(f(arg), result_type.clone())
        };

        let push_2arg_ins = |this: &mut Self, f: fn(Value, Value) -> Instruction| {
            assert_eq!(args.len(), 2);
            let arg1 = this.expression(args[0].expr);
            let arg2 = this.expression(args[1].expr);
            this.push_instruction(f(arg1, arg2), result_type.clone())
        };

        // TODO: Support more AtomicOrderings. We only use SeqCst
        let push_atomic_rmw = |this: &mut Self, op: AtomicRmwOp| {
            assert_eq!(args.len(), 2);
            let pointer = this.expression(args[0].expr);
            let value = this.expression(args[1].expr);
            let ins = Instruction::AtomicRmw { op, pointer, value, ordering: AtomicOrdering::SeqCst };
            this.push_instruction(ins, result_type.clone())
        };

        Some(match intrinsic.as_ref() {
            "AddInt" => push_2arg_ins(self, Instruction::AddInt),
            "OverflowingAddInt" => push_2arg_ins(self, Instruction::OverflowingAddInt),
            "AddFloat" => push_2arg_ins(self, Instruction::AddFloat),

            "SubInt" => push_2arg_ins(self, Instruction::SubInt),
            "OverflowingSubInt" => push_2arg_ins(self, Instruction::OverflowingSubInt),
            "SubFloat" => push_2arg_ins(self, Instruction::SubFloat),

            "MulInt" => push_2arg_ins(self, Instruction::MulInt),
            "OverflowingMulInt" => push_2arg_ins(self, Instruction::OverflowingMulInt),
            "MulFloat" => push_2arg_ins(self, Instruction::MulFloat),

            "DivSigned" => push_2arg_ins(self, Instruction::DivSigned),
            "DivUnsigned" => push_2arg_ins(self, Instruction::DivUnsigned),
            "DivFloat" => push_2arg_ins(self, Instruction::DivFloat),

            "ModSigned" => push_2arg_ins(self, Instruction::ModSigned),
            "ModUnsigned" => push_2arg_ins(self, Instruction::ModUnsigned),
            "ModFloat" => push_2arg_ins(self, Instruction::ModFloat),

            "LessSigned" => push_2arg_ins(self, Instruction::LessSigned),
            "LessUnsigned" => push_2arg_ins(self, Instruction::LessUnsigned),
            "LessFloat" => push_2arg_ins(self, Instruction::LessFloat),

            "EqInt" => push_2arg_ins(self, Instruction::EqInt),
            "EqFloat" => push_2arg_ins(self, Instruction::EqFloat),

            "SignExtend" => push_1arg_ins(self, Instruction::SignExtend),
            "ZeroExtend" => push_1arg_ins(self, Instruction::ZeroExtend),

            "SignedToFloat" => push_1arg_ins(self, Instruction::SignedToFloat),
            "UnsignedToFloat" => push_1arg_ins(self, Instruction::UnsignedToFloat),
            "FloatToSigned" => push_1arg_ins(self, Instruction::FloatToSigned),
            "FloatToUnsigned" => push_1arg_ins(self, Instruction::FloatToUnsigned),
            "FloatPromote" => push_1arg_ins(self, Instruction::FloatPromote),
            "FloatDemote" => push_1arg_ins(self, Instruction::FloatDemote),

            "BitwiseAnd" => push_2arg_ins(self, Instruction::BitwiseAnd),
            "BitwiseOr" => push_2arg_ins(self, Instruction::BitwiseOr),
            "BitwiseXor" => push_2arg_ins(self, Instruction::BitwiseXor),
            "BitwiseNot" => push_1arg_ins(self, Instruction::BitwiseNot),

            "Truncate" => push_1arg_ins(self, Instruction::Truncate),

            "Deref" => push_1arg_ins(self, Instruction::Deref),
            "Transmute" => push_1arg_ins(self, Instruction::Transmute),

            "SizeOf" => {
                // TODO: Why don't we panic on error here again? Can it be removed?
                // The argument has type `Type t`, we need to extract `t` from it.
                // The Mir builder must still be resiliant to type errors
                let t = self.get_t_from_type_t(args).unwrap_or(super::Type::ERROR);
                self.push_instruction(Instruction::SizeOf(t), result_type.clone())
            },
            "ArrayLen" => {
                // The argument has type `Type (Array n t)`. We need the inner type so
                // monomorphization can specialize the array's length to a constant.
                let t = self.get_t_from_type_t(args).unwrap_or(super::Type::ERROR);
                self.push_instruction(Instruction::ArrayLen(t), result_type.clone())
            },
            "AtomicLoad" => {
                assert_eq!(args.len(), 1);
                let pointer = self.expression(args[0].expr);
                let ins = Instruction::AtomicLoad { pointer, ordering: AtomicOrdering::SeqCst };
                self.push_instruction(ins, result_type.clone())
            },
            "AtomicStore" => {
                assert_eq!(args.len(), 2);
                let pointer = self.expression(args[0].expr);
                let value = self.expression(args[1].expr);
                let ins = Instruction::AtomicStore { pointer, value, ordering: AtomicOrdering::SeqCst };
                self.push_instruction(ins, result_type.clone())
            },
            "AtomicSwap" => push_atomic_rmw(self, AtomicRmwOp::Xchg),
            "AtomicFetchAdd" => push_atomic_rmw(self, AtomicRmwOp::Add),
            "AtomicFetchSub" => push_atomic_rmw(self, AtomicRmwOp::Sub),
            "AtomicFetchAnd" => push_atomic_rmw(self, AtomicRmwOp::And),
            "AtomicFetchOr" => push_atomic_rmw(self, AtomicRmwOp::Or),
            "AtomicFetchXor" => push_atomic_rmw(self, AtomicRmwOp::Xor),
            "AtomicCompareAndSwap" => {
                assert_eq!(args.len(), 3);
                let pointer = self.expression(args[0].expr);
                let expected = self.expression(args[1].expr);
                let desired = self.expression(args[2].expr);
                let ins = Instruction::AtomicCmpxchg {
                    pointer,
                    expected,
                    desired,
                    success: AtomicOrdering::SeqCst,
                    failure: AtomicOrdering::SeqCst,
                };
                self.push_instruction(ins, result_type.clone())
            },
            other => panic!("Unknown intrinsic `{other}`"),
        })
    }

    /// Given arguments where the first (and only) argument has the type `Type t`, return `t`
    fn get_t_from_type_t(&self, args: &[Argument]) -> Option<super::Type> {
        if args.len() != 1 {
            return None;
        }

        let type_t = self.types.result.maps.expr_types[&args[0].expr].follow(&self.types.bindings);
        match &type_t {
            TCType::Application(_, args) if args.len() == 1 => Some(self.convert_type(&args[0], None)),
            _ => None,
        }
    }
}
