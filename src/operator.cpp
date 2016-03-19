#include "parser.h"
#include "compiler.h"
#include "tokens.h"


TypedValue* Compiler::compAdd(TypedValue *l, TypedValue *r, BinOpNode *op){
    switch(l->type){
        case TT_I8:  case TT_U8:
        case TT_I16: case TT_U16:
        case TT_I32: case TT_U32:
        case TT_I64: case TT_U64:
        case TT_Ptr:
            return new TypedValue(builder.CreateAdd(l->val, r->val), l->type);
        case TT_F16:
        case TT_F32:
        case TT_F64:
            return new TypedValue(builder.CreateFAdd(l->val, r->val), l->type);

        default:
            return compErr("binary operator + is undefined for the type " + llvmTypeToStr(l->getType()) + " and " + llvmTypeToStr(r->getType()), op->row, op->col);
    }
}

TypedValue* Compiler::compSub(TypedValue *l, TypedValue *r, BinOpNode *op){
    switch(l->type){
        case TT_I8:  case TT_U8:
        case TT_I16: case TT_U16:
        case TT_I32: case TT_U32:
        case TT_I64: case TT_U64:
            return new TypedValue(builder.CreateSub(l->val, r->val), l->type);
        case TT_F16:
        case TT_F32:
        case TT_F64:
            return new TypedValue(builder.CreateFSub(l->val, r->val), l->type);

        default:
            return compErr("binary operator - is undefined for the type " + llvmTypeToStr(l->getType()) + " and " + llvmTypeToStr(r->getType()), op->row, op->col);
    }
}

TypedValue* Compiler::compMul(TypedValue *l, TypedValue *r, BinOpNode *op){
    switch(l->type){
        case TT_I8:  case TT_U8:
        case TT_I16: case TT_U16:
        case TT_I32: case TT_U32:
        case TT_I64: case TT_U64:
            return new TypedValue(builder.CreateMul(l->val, r->val), l->type);
        case TT_F16:
        case TT_F32:
        case TT_F64:
            return new TypedValue(builder.CreateFMul(l->val, r->val), l->type);

        default:
            return compErr("binary operator * is undefined for the type " + llvmTypeToStr(l->getType()) + " and " + llvmTypeToStr(r->getType()), op->row, op->col);
    }
}

TypedValue* Compiler::compDiv(TypedValue *l, TypedValue *r, BinOpNode *op){
    switch(l->type){
        case TT_I8:  
        case TT_I16: 
        case TT_I32: 
        case TT_I64: 
            return new TypedValue(builder.CreateSDiv(l->val, r->val), l->type);
        case TT_U8:
        case TT_U16:
        case TT_U32:
        case TT_U64:
            return new TypedValue(builder.CreateUDiv(l->val, r->val), l->type);
        case TT_F16:
        case TT_F32:
        case TT_F64:
            return new TypedValue(builder.CreateFDiv(l->val, r->val), l->type);

        default: 
            return compErr("binary operator / is undefined for the type " + llvmTypeToStr(l->getType()) + " and " + llvmTypeToStr(r->getType()), op->row, op->col);
    }
}

TypedValue* Compiler::compRem(TypedValue *l, TypedValue *r, BinOpNode *op){
    switch(l->type){
        case TT_I8: 
        case TT_I16:
        case TT_I32:
        case TT_I64:
            return new TypedValue(builder.CreateSRem(l->val, r->val), l->type);
        case TT_U8:
        case TT_U16:
        case TT_U32:
        case TT_U64:
            return new TypedValue(builder.CreateURem(l->val, r->val), l->type);
        case TT_F16:
        case TT_F32:
        case TT_F64:
            return new TypedValue(builder.CreateFRem(l->val, r->val), l->type);

        default:
            return compErr("binary operator % is undefined for the types " + llvmTypeToStr(l->getType()) + " and " + llvmTypeToStr(r->getType()), op->row, op->col);
    }
}

inline bool isIntTokTy(int ty){
    return ty==TT_I8||ty==TT_I16||ty==TT_I32||ty==TT_I64||
           ty==TT_U8||ty==TT_U16||ty==TT_U32||ty==TT_U64||
           ty==TT_Isz||ty==TT_Usz;
}

/*'
 *  Compiles the extract operator, [
 */
TypedValue* Compiler::compExtract(TypedValue *l, TypedValue *r, BinOpNode *op){
    if(!isIntTokTy(r->type)){
        return compErr("Index of operator '[' must be an integer expression, got expression of type " + Lexer::getTokStr(r->type), op->row, op->col);
    }

    if(l->type == TT_Array){
        Constant *lc = (Constant*)l->val;
        Constant *rc = (Constant*)r->val;
        return new TypedValue(lc->getAggregateElement(rc), llvmTypeToTypeTag(l->getType()->getArrayElementType()));
    }else if(l->type == TT_Tuple){
        if(!dynamic_cast<ConstantInt*>(r->val))
            return compErr("Tuple indices must always be known at compile time.", op->row, op->col) - 1;

        auto index = ((ConstantInt*)r->val)->getZExtValue();

        //if the entire tuple is known at compile-time, then the element can be directly retrieved.
        if(auto *lc = dynamic_cast<Constant*>(l->val)){
            return new TypedValue(lc->getAggregateElement(index), llvmTypeToTypeTag(l->getType()->getStructElementType(index)));
        }else{
            return new TypedValue(builder.CreateExtractValue(l->val, index), llvmTypeToTypeTag(l->getType()->getStructElementType(index)));
        }
    }else if(l->type == TT_Ptr){ //assume RefVal
        Value *v = builder.CreateLoad(l->val);
        if(llvmTypeToTypeTag(v->getType()) == TT_Tuple){
            if(!dynamic_cast<ConstantInt*>(r->val))
                return compErr("Pathogen values cannot be used as tuple indices.", op->row, op->col);
            auto index = ((ConstantInt*)r->val)->getZExtValue();
            
            Value *field = builder.CreateExtractValue(v, index);
            return new TypedValue(field, TT_Ptr);
        }
        return compErr("Type " + Lexer::getTokStr(l->type) + " does not have elements to access", op->row, op->col);
    }else{
        return compErr("Type " + Lexer::getTokStr(l->type) + " does not have elements to access", op->row, op->col);
    }
}


/*
 *  Compiles an insert statement for arrays or tuples.
 *  An insert statement would look similar to the following (in ante syntax):
 *
 *  i32,i32,i32 tuple = (1, 2, 4)
 *  tuple[2] = 3
 *
 *  This method Works on lvals and returns the new array/tuple with
 *  the inserted element for storage by VarAssignNode::compile.
 */
TypedValue* Compiler::compInsert(BinOpNode *op, Node *assignExpr){
    //currently, the parser only accepts a single RefVarNode as the lval.
    //this line will have to be changed if it were to become more lenient.
    auto *vn = (RefVarNode*)op->lval.get();

    /* AllocaInst of var for storage*/
    auto *var = vn->compile(this);

    /* Load the Value from the AllocaInst of var */
    auto *loadVal = builder.CreateLoad(var->val);

    auto *index = op->rval->compile(this);
    auto *newVal = assignExpr->compile(this);
    if(!var || !index || !newVal) return 0;

    switch(llvmTypeToTypeTag(loadVal->getType())){
        case TT_Array:
            return compErr("Array insert element is not yet implemented!", op->row, op->col);
        case TT_Tuple:
            if(!dynamic_cast<ConstantInt*>(index->val)){
                return compErr("Tuple indices must always be known at compile time.", op->row, op->col) - 1;
            }else{
                auto tupIndex = ((ConstantInt*)index->val)->getZExtValue();

                //Type of element at tuple index tupIndex, for type checking
                Type* tupIndexTy = loadVal->getType()->getStructElementType(tupIndex);
                Type* exprTy = newVal->getType();

                if(!llvmTypeEq(tupIndexTy, exprTy)){
                    return compErr("Cannot assign expression of type " + llvmTypeToStr(exprTy)
                                + " to tuple index " + to_string(tupIndex) + " of type " + llvmTypeToStr(tupIndexTy),
                                assignExpr->row, assignExpr->col);
                }

                Value *insertedTup = builder.CreateInsertValue(loadVal, newVal->val, tupIndex);
                return new TypedValue(builder.CreateStore(insertedTup, var->val), TT_Void);
            }
        default:
            return compErr("Variable being indexed must be an Array or Tuple, but instead is a(n) " +
                    llvmTypeToStr(loadVal->getType()), op->row, op->col);
    }
}


/*
 *  Compiles an operation along with its lhs and rhs
 */
TypedValue* BinOpNode::compile(Compiler *c){
    if(op == Tok_Where){
        rval->compile(c); //rval should always be a LetBindingNode
        return lval->compile(c);
    }

    TypedValue *lhs = lval->compile(c);
    TypedValue *rhs = rval->compile(c);
    if(!lhs || !rhs) return 0;

    //Check if both Values are integers, and if so, check if their bit width's match.
    //If not, the smaller is extended to the larger's type.
    if(isIntTokTy(lhs->type) && isIntTokTy(rhs->type)){
        c->checkIntSize(&lhs, &rhs);
    }

    switch(op){
        case '+': return c->compAdd(lhs, rhs, this);
        case '-': return c->compSub(lhs, rhs, this);
        case '*': return c->compMul(lhs, rhs, this);
        case '/': return c->compDiv(lhs, rhs, this);
        case '%': return c->compRem(lhs, rhs, this);
        case '[': return c->compExtract(lhs, rhs, this);
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


TypedValue* UnOpNode::compile(Compiler *c){
    TypedValue *rhs = rval->compile(c);
    if(!rhs) return 0;

    switch(op){
        case '*': //pointer dereference
            if(rhs->type != TT_Ptr){
                return c->compErr("Cannot dereference non-pointer type " + llvmTypeToStr(rhs->getType()), this->row, this->col);
            }
            
            return new TypedValue(c->builder.CreateLoad(rhs->val), llvmTypeToTypeTag(rhs->getType()->getPointerElementType()));
        case '&': //address-of
            break; //TODO
        case '-': //negation
            return new TypedValue(c->builder.CreateNeg(rhs->val), rhs->type);
    }
    
    return c->compErr("Unknown unary operator " + Lexer::getTokStr(op), this->row, this->col);
}
