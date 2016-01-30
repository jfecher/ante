#include "parser.h"
//#include "compiler.h"
#include "tokens.h"

using namespace ante;

int type2TokType(Type *t)
{
    if(t->isIntegerTy(8)) return Tok_I8;
    if(t->isIntegerTy(16)) return Tok_I16;
    if(t->isIntegerTy(32)) return Tok_I32;
    if(t->isIntegerTy(64)) return Tok_I64;
    if(t->isHalfTy()) return Tok_F16;
    if(t->isFloatTy()) return Tok_F32;
    if(t->isDoubleTy()) return Tok_F64;
    
    if(t->isArrayTy()) return '[';
    if(t->isStructTy()) return Tok_Data;
    if(t->isPointerTy()) return '*';
    if(t->isFunctionTy()) return '(';

    return Tok_Void;
}

/*
 *  Converts an operation type to its string equivalent for
 *  helpful error messages.
 */
string opType2Str(int opTy)
{
    switch(opTy){
        case '[': return "Array";
        case '(': return "Function";
        case '*': return "Pointer";
        default:  return lexer::getTokStr(opTy);
    }
}

Value* Compiler::compAdd(Type *t, Value *l, Value *r)
{
    int tt = type2TokType(t);

    switch(tt){
        case Tok_I8:
        case Tok_I16:
        case Tok_I32:
        case Tok_I64:
            return builder.CreateAdd(l, r);
        case Tok_F16:
        case Tok_F32:
        case Tok_F64:
            return builder.CreateFAdd(l, r);

        default:
            break;
            //return compErr("binary operator + is undefined for the type ", opType2Str(tt));
    }
}

Value* Compiler::compSub(Type *t, Value *l, Value *r)
{
    int tt = type2TokType(t);
    switch(tt){
        case Tok_I8:
        case Tok_I16:
        case Tok_I32:
        case Tok_I64:
            return builder.CreateSub(l, r);
        case Tok_F16:
        case Tok_F32:
        case Tok_F64:
            return builder.CreateFSub(l, r);

        default:
            break;
            //return compErr("binary operator - is undefined for the type ", opType2Str(tt));
    }
}

Value* Compiler::compMul(Type *t, Value *l, Value *r)
{
    int tt = type2TokType(t);
    switch(tt){
        case Tok_I8:
        case Tok_I16:
        case Tok_I32:
        case Tok_I64:
            return builder.CreateMul(l, r);
        case Tok_F16:
        case Tok_F32:
        case Tok_F64:
            return builder.CreateFMul(l, r);

        default:
            break;
        //      return compErr("binary operator * is undefined for the type ", opType2Str(tt));
    }
}

Value* Compiler::compDiv(Type *t, Value *l, Value *r)
{
    int tt = type2TokType(t);
    switch(tt){
        case Tok_I8:
        case Tok_I16:
        case Tok_I32:
        case Tok_I64:
            return builder.CreateSDiv(l, r);
        case Tok_F16:
        case Tok_F32:
        case Tok_F64:
            return builder.CreateFDiv(l, r);

        default: 
            break;
      //      return compErr("binary operator / is undefined for the type ", opType2Str(tt));
    }
}

Value* Compiler::compRem(Type *t, Value *l, Value *r)
{
    int tt = type2TokType(t);
    switch(tt){
        case Tok_I8:
        case Tok_I16:
        case Tok_I32:
        case Tok_I64:
            return builder.CreateSRem(l, r);
        case Tok_F16:
        case Tok_F32:
        case Tok_F64:
            return builder.CreateFRem(l, r);

        default:
            break;
     //       return compErr("binary operator % is undefined for the type ", opType2Str(tt));
    }
}

/*
 *  Compiles an operation along with its lhs and rhs
 *
 *  TODO: type checking
 *  TODO: CreateExactUDiv for when it is known there is no remainder
 *  TODO: CreateFcmpOEQ vs CreateFCmpUEQ
 */
Value* BinOpNode::compile(Compiler *c, Module *m)
{
    Value *lhs = lval->compile(c, m);
    Value *rhs = rval->compile(c, m);

    Type *lt = lhs->getType();
    //Type *rt = lhs->getType();

    switch(op){
        case '+': return c->compAdd(lt, lhs, rhs);
        case '-': return c->compSub(lt, lhs, rhs);
        case '*': return c->compMul(lt, lhs, rhs);
        case '/': return c->compDiv(lt, lhs, rhs);
        case '%': return c->compRem(lt, lhs, rhs);
        case '<': return c->builder.CreateICmpULT(lhs, rhs);
        case '>': return c->builder.CreateICmpUGT(lhs, rhs);
        case '^': return c->builder.CreateXor(lhs, rhs);
        case '.': break;
        case Tok_Eq: return c->builder.CreateICmpEQ(lhs, rhs);
        case Tok_NotEq: return c->builder.CreateICmpNE(lhs, rhs);
        case Tok_LesrEq: return c->builder.CreateICmpULE(lhs, rhs);
        case Tok_GrtrEq: return c->builder.CreateICmpUGE(lhs, rhs);
        case Tok_Or: break;
        case Tok_And: break;
    }

    //return c->compErr("Unknown operator ", lexer::getTokStr(op));
}
