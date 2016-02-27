#include "parser.h"
#include "compiler.h"
#include "tokens.h"

/*
 *  Converts an operation type to its string equivalent for
 *  helpful error messages.
 */
string opType2Str(int opTy){
    switch(opTy){
        case '[': return "Array";
        case '(': return "Function";
        case '*': return "Pointer";
        default:  return Lexer::getTokStr(opTy);
    }
}

TypedValue* Compiler::compAdd(TypedValue *l, TypedValue *r, BinOpNode *op){
    switch(l->type){
        case Tok_I8:  case Tok_U8:
        case Tok_I16: case Tok_U16:
        case Tok_I32: case Tok_U32:
        case Tok_I64: case Tok_U64:
        case '*':
            return new TypedValue(builder.CreateAdd(l->val, r->val), l->type);
        case Tok_F16:
        case Tok_F32:
        case Tok_F64:
            return new TypedValue(builder.CreateFAdd(l->val, r->val), l->type);

        default:
            return compErr("binary operator + is undefined for the type " + opType2Str(l->type) + " and " + opType2Str(r->type), op->row, op->col);
    }
}

TypedValue* Compiler::compSub(TypedValue *l, TypedValue *r, BinOpNode *op){
    switch(l->type){
        case Tok_I8:  case Tok_U8:
        case Tok_I16: case Tok_U16:
        case Tok_I32: case Tok_U32:
        case Tok_I64: case Tok_U64:
            return new TypedValue(builder.CreateSub(l->val, r->val), l->type);
        case Tok_F16:
        case Tok_F32:
        case Tok_F64:
            return new TypedValue(builder.CreateFSub(l->val, r->val), l->type);

        default:
            return compErr("binary operator - is undefined for the type " + opType2Str(l->type) + " and " + opType2Str(r->type), op->row, op->col);
    }
}

TypedValue* Compiler::compMul(TypedValue *l, TypedValue *r, BinOpNode *op){
    switch(l->type){
        case Tok_I8:  case Tok_U8:
        case Tok_I16: case Tok_U16:
        case Tok_I32: case Tok_U32:
        case Tok_I64: case Tok_U64:
            return new TypedValue(builder.CreateMul(l->val, r->val), l->type);
        case Tok_F16:
        case Tok_F32:
        case Tok_F64:
            return new TypedValue(builder.CreateFMul(l->val, r->val), l->type);

        default:
            return compErr("binary operator * is undefined for the type " + opType2Str(l->type) + " and " + opType2Str(r->type), op->row, op->col);
    }
}

TypedValue* Compiler::compDiv(TypedValue *l, TypedValue *r, BinOpNode *op){
    switch(l->type){
        case Tok_I8:  
        case Tok_I16: 
        case Tok_I32: 
        case Tok_I64: 
            return new TypedValue(builder.CreateSDiv(l->val, r->val), l->type);
        case Tok_U8:
        case Tok_U16:
        case Tok_U32:
        case Tok_U64:
            return new TypedValue(builder.CreateUDiv(l->val, r->val), l->type);
        case Tok_F16:
        case Tok_F32:
        case Tok_F64:
            return new TypedValue(builder.CreateFDiv(l->val, r->val), l->type);

        default: 
            return compErr("binary operator / is undefined for the type " + opType2Str(l->type) + " and " + opType2Str(r->type), op->row, op->col);
    }
}

TypedValue* Compiler::compRem(TypedValue *l, TypedValue *r, BinOpNode *op){
    switch(l->type){
        case Tok_I8: 
        case Tok_I16:
        case Tok_I32:
        case Tok_I64:
            return new TypedValue(builder.CreateSRem(l->val, r->val), l->type);
        case Tok_U8:
        case Tok_U16:
        case Tok_U32:
        case Tok_U64:
            return new TypedValue(builder.CreateURem(l->val, r->val), l->type);
        case Tok_F16:
        case Tok_F32:
        case Tok_F64:
            return new TypedValue(builder.CreateFRem(l->val, r->val), l->type);

        default:
            return compErr("binary operator % is undefined for the types " + opType2Str(l->type) + " and " + opType2Str(r->type), op->row, op->col);
    }
}


inline bool isIntTokTy(int ty){
    return ty==Tok_I8||ty==Tok_I16||ty==Tok_I32||ty==Tok_I64||
           ty==Tok_U8||ty==Tok_U16||ty==Tok_U32||ty==Tok_U64||
           ty==Tok_Isz||ty==Tok_Usz;
}


/*
 *  Compiles an operation along with its lhs and rhs
 *
 *  TODO: more type checking
 */
TypedValue* BinOpNode::compile(Compiler *c, Module *m){
    TypedValue *lhs = lval->compile(c, m);
    TypedValue *rhs = rval->compile(c, m);
    if(!lhs || !rhs) return 0;

    //Check if both Values are integers, and if so, check if their bit width's match.
    //If not, the smaller is set to the larger's type.
    if(isIntTokTy(lhs->type) && isIntTokTy(rhs->type)){
        c->checkIntSize(&lhs, &rhs);
    }

    switch(op){
        case '+': return c->compAdd(lhs, rhs, this);
        case '-': return c->compSub(lhs, rhs, this);
        case '*': return c->compMul(lhs, rhs, this);
        case '/': return c->compDiv(lhs, rhs, this);
        case '%': return c->compRem(lhs, rhs, this);
        case '<': return new TypedValue(c->builder.CreateICmpULT(lhs->val, rhs->val), lhs->type);
        case '>': return new TypedValue(c->builder.CreateICmpUGT(lhs->val, rhs->val), lhs->type);
        case '^': return new TypedValue(c->builder.CreateXor(lhs->val, rhs->val), lhs->type);
        case '.': break;
        case Tok_Eq: return new TypedValue(c->builder.CreateICmpEQ(lhs->val, rhs->val), lhs->type);
        case Tok_NotEq: return new TypedValue(c->builder.CreateICmpNE(lhs->val, rhs->val), lhs->type);
        case Tok_LesrEq: return new TypedValue(c->builder.CreateICmpULE(lhs->val, rhs->val), lhs->type);
        case Tok_GrtrEq: return new TypedValue(c->builder.CreateICmpUGE(lhs->val, rhs->val), lhs->type);
        case Tok_Or: break;
        case Tok_And: break;
    }

    return c->compErr("Unknown operator " + Lexer::getTokStr(op), this->row, this->col);
}


TypedValue* UnOpNode::compile(Compiler *c, Module *m){
    TypedValue *rhs = rval->compile(c, m);
    if(!rhs) return 0;

    switch(op){
        case '@': //pointer dereference
            if(rhs->type != '*'){
                return c->compErr("Cannot dereference non-pointer type " + Lexer::getTokStr(rhs->type), this->row, this->col);
            }
            
            return new TypedValue(c->builder.CreateLoad(rhs->val), Compiler::llvmTypeToTokType(rhs->val->getType()->getPointerElementType()));
        case '&': //address-of
            break; //TODO
        case '-': //negation
            break;
    }
    
    return c->compErr("Unknown unary operator " + Lexer::getTokStr(op), this->row, this->col);
}
