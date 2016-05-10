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

inline bool isIntTypeTag(const TypeTag ty){
    return ty==TT_I8||ty==TT_I16||ty==TT_I32||ty==TT_I64||
           ty==TT_U8||ty==TT_U16||ty==TT_U32||ty==TT_U64||
           ty==TT_Isz||ty==TT_Usz;
}

inline bool isFPTypeTag(const TypeTag tt){
    return tt==TT_F16||tt==TT_F32||tt==TT_F64;
}

/*'
 *  Compiles the extract operator, [
 */
TypedValue* Compiler::compExtract(TypedValue *l, TypedValue *r, BinOpNode *op){
    if(!isIntTypeTag(r->type)){
        return compErr("Index of operator '[' must be an integer expression, got expression of type " + Lexer::getTokStr(r->type), op->row, op->col);
    }

    if(l->type == TT_Array){
        //check for alloca
        Value *arr = l->val;
        if(dynamic_cast<AllocaInst*>(l->val)){
            arr = builder.CreateLoad(l->val);
        }
        Type *elemTy = arr->getType()->getPointerElementType();
        return new TypedValue(builder.CreateExtractElement(arr, r->val), llvmTypeToTypeTag(elemTy));
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
            return new TypedValue(field, llvmTypeToTypeTag(field->getType()));
        }
        return compErr("Type " + llvmTypeToStr(l->getType()) + " does not have elements to access", op->row, op->col);
    }else{
        return compErr("Type " + llvmTypeToStr(l->getType()) + " does not have elements to access", op->row, op->col);
    }
}


/*
 *  Compiles an insert statement for arrays or tuples.
 *  An insert statement would look similar to the following (in ante syntax):
 *
 *  i32,i32,i32 tuple = (1, 2, 4)
 *  tuple[2] = 3
 *
 *  This method Works on lvals and returns the value of the the CreateStore
 *  method when storing the newly inserted tuple.
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
            return new TypedValue(builder.CreateStore(builder.CreateInsertElement(loadVal, newVal->val, index->val), var->val), TT_Void);
            //return new TypedValue(builder.CreateInsertElement(loadVal, newVal->val, index->val), TT_Void);
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
 *  Creates a cast instruction appropriate for valToCast's type to castTy.
 */
Value* createCast(Compiler *c, Type *castTy, TypeTag castTyTag, TypedValue *valToCast){
    if(isIntTypeTag(valToCast->type)){
        // int -> int  (maybe unsigned)
        if(isIntTypeTag(castTyTag)){
            return c->builder.CreateIntCast(valToCast->val, castTy, isUnsignedTypeTag(castTyTag));

        // int -> float
        }else if(isFPTypeTag(castTyTag)){
            if(isUnsignedTypeTag(castTyTag)){
                return c->builder.CreateUIToFP(valToCast->val, castTy);
            }else{
                return c->builder.CreateSIToFP(valToCast->val, castTy);
            }
        
        // int -> ptr
        }else if(castTyTag == TT_Ptr){
            return c->builder.CreatePtrToInt(valToCast->val, castTy);
        }
    }else if(isFPTypeTag(valToCast->type)){
        // float -> int  (maybe unsigned)
        if(isIntTypeTag(castTyTag)){
            if(isUnsignedTypeTag(castTyTag)){
                return c->builder.CreateFPToUI(valToCast->val, castTy);
            }else{
                return c->builder.CreateFPToSI(valToCast->val, castTy);
            }

        // float -> float
        }else if(isFPTypeTag(castTyTag)){
            return c->builder.CreateFPCast(valToCast->val, castTy);
        }

    }else if(valToCast->type == TT_Ptr || valToCast->type == TT_StrLit){
        // ptr -> ptr
        if(castTyTag == TT_Ptr){
            return c->builder.CreatePointerCast(valToCast->val, castTy);

        // ptr -> int
        }else if(isIntTypeTag(castTyTag)){
            return c->builder.CreatePtrToInt(valToCast->val, castTy);
        }
    }
    return nullptr;
}

TypedValue* TypeCastNode::compile(Compiler *c){
    Type *castTy = c->typeNodeToLlvmType(typeExpr.get());
    auto *rtval = rval->compile(c);
    if(!castTy || !rtval) return 0;

    if(llvmTypeEq(castTy, rtval->getType())) return rtval;
    auto* val = createCast(c, castTy, typeExpr->type, rtval);
    
    if(!val){
        return c->compErr("Invalid type cast " + llvmTypeToStr(rtval->getType()) + " -> " + llvmTypeToStr(castTy), row, col);
    }else{
        return new TypedValue(val, typeExpr->type);
    }
}

TypedValue* compMemberAccess(Compiler *c, TypedValue *l, VarNode *field, BinOpNode *binop){
    if(!l) return 0;

    if(l->type == TT_Data){
        auto dataTy = c->lookupType(l->getType()->getStructName());
        auto index = dataTy->getFieldIndex(field->name);

        if(index != -1)
            return c->compErr("Method/Field '" + field->name + "' is not present within the " + llvmTypeToStr(l->getType()) + " datatype.", field->row, field->col);

        return new TypedValue(c->builder.CreateExtractValue(l->val, index), llvmTypeToTypeTag(l->getType()->getStructElementType(index)));
    }
    //not a field, so look for a method.
    string funcName = llvmTypeToStr(l->getType()) + "_" + field->name;

    if(auto *f = c->getFunction(funcName))
        return new TypedValue(f, TT_Function);

    return c->compErr("Method/Field " + funcName + " not found in type " + llvmTypeToStr(l->getType()), binop->row, binop->col);
}


TypedValue* compFnCall(Compiler *c, Node *l, Node *r){
    /* Check given argument count matches declared argument count. */
    TypedValue *tvf = l->compile(c);
    if(!tvf || !tvf->val) return 0;
    if(tvf->type != TT_Function)
        return c->compErr("Called value is not a function or method.", l->row, l->col);

    //now that we assured it is a function, unwrap it
    Function *f = (Function*) tvf->val;

    auto args = ((TupleNode*)r)->unpack(c);

    if(f->arg_size() != args.size() && !f->isVarArg()){
        if(args.size() == 1)
            return c->compErr("Called method was given 1 argument but was declared to take " + to_string(f->arg_size()), r->row, r->col);
        else
            return c->compErr("Called method was given " + to_string(args.size()) + " arguments but was declared to take " + to_string(f->arg_size()), r->row, r->col);
    }

    /* unpack the tuple of arguments into a vector containing each value */
    int i = 0;
    for(auto &param : f->args()){//type check each parameter
        if(!llvmTypeEq(args[i++]->getType(), param.getType())){
            return c->compErr("Argument " + to_string(i) + " of method is a(n) " + llvmTypeToStr(args[i-1]->getType())
                    + " but was declared to be a(n) " + llvmTypeToStr(param.getType()), r->row, r->col);
        }
    }

    return new TypedValue(c->builder.CreateCall(f, args), llvmTypeToTypeTag(f->getReturnType()));
    
}


/*
 *  Compiles an operation along with its lhs and rhs
 */
TypedValue* BinOpNode::compile(Compiler *c){
    if(op == Tok_Where){
        rval->compile(c); //rval should always be a LetBindingNode
        return lval->compile(c);
    }else if(op == '.'){
        return compMemberAccess(c, lval->compile(c), (VarNode*)rval.get(), this);
    }else if(op == '('){
        return compFnCall(c, lval.get(), rval.get());
    }

    TypedValue *lhs = lval->compile(c);
    TypedValue *rhs = rval->compile(c);
    if(!lhs || !rhs) return 0;

    //Check if both Values are numeric, and if so, check if their types match.
    //If not, do an implicit conversion (usually a widening) to match them.
    c->handleImplicitConversion(&lhs, &rhs);

    switch(op){
        case '+': return c->compAdd(lhs, rhs, this);
        case '-': return c->compSub(lhs, rhs, this);
        case '*': return c->compMul(lhs, rhs, this);
        case '/': return c->compDiv(lhs, rhs, this);
        case '%': return c->compRem(lhs, rhs, this);
        case '[': return c->compExtract(lhs, rhs, this);
        case ';': return rhs;
        case Tok_Let: return rhs;
        case '<': return new TypedValue(c->builder.CreateICmpULT(lhs->val, rhs->val), lhs->type);
        case '>': return new TypedValue(c->builder.CreateICmpUGT(lhs->val, rhs->val), lhs->type);
        case '^': return new TypedValue(c->builder.CreateXor(lhs->val, rhs->val), lhs->type);
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
        case '@': //pointer dereference
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
