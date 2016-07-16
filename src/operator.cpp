#include "compiler.h"
#include "tokens.h"


TypedValue* Compiler::compAdd(TypedValue *l, TypedValue *r, BinOpNode *op){
    switch(l->type->type){
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
            return compErr("binary operator + is undefined for the type " + typeNodeToStr(l->type.get()) + " and " + typeNodeToStr(r->type.get()), op->loc);
    }
}

TypedValue* Compiler::compSub(TypedValue *l, TypedValue *r, BinOpNode *op){
    switch(l->type->type){
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
            return compErr("binary operator - is undefined for the type " + llvmTypeToStr(l->getType()) + " and " + llvmTypeToStr(r->getType()), op->loc);
    }
}

TypedValue* Compiler::compMul(TypedValue *l, TypedValue *r, BinOpNode *op){
    switch(l->type->type){
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
            return compErr("binary operator * is undefined for the type " + llvmTypeToStr(l->getType()) + " and " + llvmTypeToStr(r->getType()), op->loc);
    }
}

TypedValue* Compiler::compDiv(TypedValue *l, TypedValue *r, BinOpNode *op){
    switch(l->type->type){
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
            return compErr("binary operator / is undefined for the type " + llvmTypeToStr(l->getType()) + " and " + llvmTypeToStr(r->getType()), op->loc);
    }
}

TypedValue* Compiler::compRem(TypedValue *l, TypedValue *r, BinOpNode *op){
    switch(l->type->type){
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
            return compErr("binary operator % is undefined for the types " + llvmTypeToStr(l->getType()) + " and " + llvmTypeToStr(r->getType()), op->loc);
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
    if(!isIntTypeTag(r->type->type)){
        return compErr("Index of operator '[' must be an integer expression, got expression of type " + typeNodeToStr(r->type.get()), op->loc);
    }

    if(l->type->type == TT_Array){
        //check for alloca
        Value *arr = l->val;
        if(dynamic_cast<AllocaInst*>(l->val)){
            arr = builder.CreateLoad(l->val);
        }
        return new TypedValue(builder.CreateExtractElement(arr, r->val), l->type->extTy.get());
    }else if(l->type->type == TT_Tuple || l->type->type == TT_Data){
        if(!dynamic_cast<ConstantInt*>(r->val))
            return compErr("Tuple indices must always be known at compile time.", op->loc);

        auto index = ((ConstantInt*)r->val)->getZExtValue();

        //get the type from the index in question
        TypeNode* indexTyn = l->type->extTy.get();
        for(unsigned i = 0; i < index; i++)
            indexTyn = (TypeNode*)indexTyn->next.get();

        //if the entire tuple is known at compile-time, then the element can be directly retrieved.
        //
        //TODO: possibly remove this check, an extract should be optimized away if it is a constant anyway
        if(auto *lc = dynamic_cast<Constant*>(l->val)){
            return new TypedValue(lc->getAggregateElement(index), indexTyn);
        }else{
            return new TypedValue(builder.CreateExtractValue(l->val, index), indexTyn);
        }
    }else if(l->type->type == TT_Ptr){ //assume RefVal
        Value *v = builder.CreateLoad(l->val);
        if(llvmTypeToTypeTag(v->getType()) == TT_Tuple){
            if(!dynamic_cast<ConstantInt*>(r->val))
                return compErr("Pathogen values cannot be used as tuple indices.", op->loc);
            auto index = ((ConstantInt*)r->val)->getZExtValue();
       
            //get the type of the index in question
            TypeNode* indexTyn = l->type->extTy.get();
            for(unsigned i = 0; i < index; i++)
                indexTyn = (TypeNode*)indexTyn->next.get();
            
            Value *field = builder.CreateExtractValue(v, index);
            return new TypedValue(field, indexTyn);
        }
        return compErr("Type " + llvmTypeToStr(l->getType()) + " does not have elements to access", op->loc);
    }else{
        return compErr("Type " + llvmTypeToStr(l->getType()) + " does not have elements to access", op->loc);
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
    auto *tmp = op->lval->compile(this);

    if(!dynamic_cast<LoadInst*>(tmp->val))
        return compErr("Variable must be mutable to insert values, but instead is an immutable " +
                llvmTypeToStr(tmp->getType()), op->lval->loc);

    Value *var = static_cast<LoadInst*>(tmp->val)->getPointerOperand();
    if(!var) return 0;


    auto *index = op->rval->compile(this);
    auto *newVal = assignExpr->compile(this);
    if(!var || !index || !newVal) return 0;

    switch(llvmTypeToTypeTag(tmp->getType())){
        case TT_Array:
            return new TypedValue(builder.CreateStore(builder.CreateInsertElement(tmp->val, newVal->val, index->val), var), mkAnonTypeNode(TT_Void));
            //return new TypedValue(builder.CreateInsertElement(loadVal, newVal->val, index->val), TT_Void);
        case TT_Tuple:
            if(!dynamic_cast<ConstantInt*>(index->val)){
                return compErr("Tuple indices must always be known at compile time.", op->loc);
            }else{
                auto tupIndex = ((ConstantInt*)index->val)->getZExtValue();

                //Type of element at tuple index tupIndex, for type checking
                Type* tupIndexTy = tmp->val->getType()->getStructElementType(tupIndex);
                Type* exprTy = newVal->getType();

                if(!llvmTypeEq(tupIndexTy, exprTy)){
                    return compErr("Cannot assign expression of type " + llvmTypeToStr(exprTy)
                                + " to tuple index " + to_string(tupIndex) + " of type " + llvmTypeToStr(tupIndexTy),
                                assignExpr->loc);
                }

                Value *insertedTup = builder.CreateInsertValue(tmp->val, newVal->val, tupIndex);
                return new TypedValue(builder.CreateStore(insertedTup, var), mkAnonTypeNode(TT_Void));
            }
        default:
            return compErr("Variable being indexed must be an Array or Tuple, but instead is a(n) " +
                    llvmTypeToStr(tmp->val->getType()), op->loc);
    }
}

/*
 *  Creates a cast instruction appropriate for valToCast's type to castTy.
 */
TypedValue* createCast(Compiler *c, Type *castTy, TypeNode *tyn, TypedValue *valToCast){
    if(isIntTypeTag(valToCast->type->type)){
        // int -> int  (maybe unsigned)
        if(isIntTypeTag(tyn->type)){
            return new TypedValue(c->builder.CreateIntCast(valToCast->val, castTy, isUnsignedTypeTag(tyn->type)), tyn);

        // int -> float
        }else if(isFPTypeTag(tyn->type)){
            if(isUnsignedTypeTag(valToCast->type->type)){
                return new TypedValue(c->builder.CreateUIToFP(valToCast->val, castTy), tyn);
            }else{
                return new TypedValue(c->builder.CreateSIToFP(valToCast->val, castTy), tyn);
            }
        
        // int -> ptr
        }else if(tyn->type == TT_Ptr){
            return new TypedValue(c->builder.CreatePtrToInt(valToCast->val, castTy), tyn);
        }
    }else if(isFPTypeTag(valToCast->type->type)){
        // float -> int  (maybe unsigned)
        if(isIntTypeTag(tyn->type)){
            if(isUnsignedTypeTag(tyn->type)){
                return new TypedValue(c->builder.CreateFPToUI(valToCast->val, castTy), tyn);
            }else{
                return new TypedValue(c->builder.CreateFPToSI(valToCast->val, castTy), tyn);
            }

        // float -> float
        }else if(isFPTypeTag(tyn->type)){
            return new TypedValue(c->builder.CreateFPCast(valToCast->val, castTy), tyn);
        }

    }else if(valToCast->type->type == TT_Ptr || valToCast->type->type == TT_StrLit){
        // ptr -> ptr
        if(tyn->type == TT_Ptr){
            return new TypedValue(c->builder.CreatePointerCast(valToCast->val, castTy), tyn);

        // ptr -> int
        }else if(isIntTypeTag(tyn->type)){
            return new TypedValue(c->builder.CreatePtrToInt(valToCast->val, castTy), tyn);
        }
    }else if(tyn->type == TT_Data && valToCast->type->type == TT_Tuple){
        if(llvmTypeEq(castTy, valToCast->getType())){
            valToCast->type->typeName = tyn->typeName;
            valToCast->type->type = TT_Data;
            return valToCast;
        }
    }
    return nullptr;
}

TypedValue* TypeCastNode::compile(Compiler *c){
    Type *castTy = c->typeNodeToLlvmType(typeExpr.get());
    auto *rtval = rval->compile(c);
    if(!castTy || !rtval) return 0;

    auto* tval = createCast(c, castTy, typeExpr.get(), rtval);

    if(!tval){
        return c->compErr("Invalid type cast " + typeNodeToStr(rtval->type.get()) + 
                " -> " + typeNodeToStr(typeExpr.get()), loc);
    }else{
        return tval;
    }
}

TypedValue* IfNode::compile(Compiler *c){
    
    auto *cond = condition->compile(c);
    if(!cond) return 0;
    
    Function *f = c->builder.GetInsertBlock()->getParent();
    auto &blocks = f->getBasicBlockList();

    auto *thenbb = BasicBlock::Create(getGlobalContext(), "then");
    auto *mergbb = BasicBlock::Create(getGlobalContext(), "endif");
   
    //only create the else block if this ifNode actually has an else clause
    BasicBlock *elsebb;
    
    if(elseN){
        elsebb = BasicBlock::Create(getGlobalContext(), "else");
        c->builder.CreateCondBr(cond->val, thenbb, elsebb);
   
        blocks.push_back(thenbb);
        blocks.push_back(elsebb);
        blocks.push_back(mergbb);
    }else{
        c->builder.CreateCondBr(cond->val, thenbb, mergbb);
        blocks.push_back(thenbb);
        blocks.push_back(mergbb);
    }

    c->builder.SetInsertPoint(thenbb);
    auto *thenVal = thenN->compile(c);
    c->builder.CreateBr(mergbb);

    if(elseN){
        c->builder.SetInsertPoint(elsebb);
        auto *elseVal = elseN->compile(c);
        c->builder.CreateBr(mergbb);

        if(!thenVal || !elseVal) return 0;


        if(!llvmTypeEq(thenVal->getType(), elseVal->getType())){
            return c->compErr("If condition's then expr's type " + llvmTypeToStr(thenVal->getType()) +
                            " does not match the else expr's type " + llvmTypeToStr(elseVal->getType()), loc);
        }


        c->builder.SetInsertPoint(mergbb);

        if(thenVal->type->type != TT_Void){
            auto *phi = c->builder.CreatePHI(thenVal->getType(), 2);
            phi->addIncoming(thenVal->val, thenbb);
            phi->addIncoming(elseVal->val, elsebb);

            return new TypedValue(phi, thenVal->type);
        }else{
            return c->getVoidLiteral();
        }
    }else{
        c->builder.SetInsertPoint(mergbb);
        return c->getVoidLiteral();
    }
}

TypedValue* compMemberAccess(Compiler *c, Node *ln, VarNode *field, BinOpNode *binop){
    if(!ln) return 0;

    if(dynamic_cast<TypeNode*>(ln)){
        //since ln is a typenode, this is a static field/method access, eg Math.rand
        Type* lty = c->typeNodeToLlvmType((TypeNode*)ln);

        string valName = llvmTypeToStr(lty) + "_" + field->name;

        if(auto *f = c->getFunction(valName))
            return f;

        return c->compErr("No static method or field called " + field->name + " was found in type " + 
                llvmTypeToStr(lty), binop->loc);
    }else{
        //ln is not a typenode, this is not a static method call
        auto *l = ln->compile(c);
        if(!l) return 0;

        //the . operator should automatically dereference pointers
        while(l->type->type == TT_Ptr){
            l->val = c->builder.CreateLoad(l->val);
            l->type.reset(l->type->extTy.get());
        }

        if(l->type->type == TT_Data || l->type->type == TT_Tuple){
            auto dataTy = c->lookupType(typeNodeToStr(l->type.get()));

            if(dataTy){
                auto index = dataTy->getFieldIndex(field->name);

                if(index != -1){
                    TypeNode *indexTy = l->type->extTy.get();

                    for(int i = 0; i < index; i++){
                        indexTy = (TypeNode*)indexTy->next.get();
                    }

                    return new TypedValue(c->builder.CreateExtractValue(l->val, index), deepCopyTypeNode(indexTy));
                }
            }
        }

        //not a field, so look for a method.
        //TODO: perhaps create a calling convention function
        string funcName = typeNodeToStr(l->type.get()) + "_" + field->name;

        if(auto *f = c->getFunction(funcName))
            return new MethodVal(l->val, f);

        return c->compErr("Method/Field " + field->name + " not found in type " + 
                llvmTypeToStr(l->getType()), binop->loc);
    }
}


TypedValue* compFnCall(Compiler *c, Node *l, Node *r){
    /* Check given argument count matches declared argument count. */
    TypedValue *tvf = l->compile(c);
    if(!tvf || !tvf->val) return 0;
    if(tvf->type->type != TT_Function && tvf->type->type != TT_Method)
        return c->compErr("Called value is not a function or method, it is a(n) " + 
                llvmTypeToStr(tvf->getType()), l->loc);

    //now that we assured it is a function, unwrap it
    Function *f = (Function*) tvf->val;

    //if tvf is a method, add its host object as the first argument
    vector<Value*> args;
    if(tvf->type->type == TT_Method){
        Value *obj = ((MethodVal*) tvf)->obj;
        args.push_back(obj);
    }

    //add all remaining arguments
    if(auto *tup = dynamic_cast<TupleNode*>(r)){
        for(TypedValue *v : tup->unpack(c)){
            if(!v) return 0;
            args.push_back(v->val);
        }
    }else{ //single parameter being applied
        auto *param = r->compile(c);
        if(!param) return 0;
        args.push_back(param->val);
    }


    if(f->arg_size() != args.size() && !f->isVarArg()){
        if(args.size() == 1)
            return c->compErr("Called function was given 1 argument but was declared to take " 
                    + to_string(f->arg_size()), r->loc);
        else
            return c->compErr("Called function was given " + to_string(args.size()) + 
                    " arguments but was declared to take " + to_string(f->arg_size()), r->loc);
    }

    /* unpack the tuple of arguments into a vector containing each value */
    int i = 0;
    for(auto &param : f->args()){//type check each parameter
        if(!llvmTypeEq(args[i++]->getType(), param.getType())){
            return c->compErr("Argument " + to_string(i) + " of function is a(n) " + llvmTypeToStr(args[i-1]->getType())
                    + " but was declared to be a(n) " + llvmTypeToStr(param.getType()), r->loc);
        }
    }

    
    return new TypedValue(c->builder.CreateCall(f, args), tvf->type->extTy.get());
}


/*
 *  Compiles an operation along with its lhs and rhs
 */
TypedValue* BinOpNode::compile(Compiler *c){
    if(op == '.')
        return compMemberAccess(c, lval.get(), (VarNode*)rval.get(), this);
    else if(op == '(')
        return compFnCall(c, lval.get(), rval.get());


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

    return c->compErr("Unknown operator " + Lexer::getTokStr(op), loc);
}


TypedValue* UnOpNode::compile(Compiler *c){
    TypedValue *rhs = rval->compile(c);
    if(!rhs) return 0;

    switch(op){
        case '@': //pointer dereference
            if(rhs->type->type != TT_Ptr){
                return c->compErr("Cannot dereference non-pointer type " + llvmTypeToStr(rhs->getType()), loc);
            }
            
            return new TypedValue(c->builder.CreateLoad(rhs->val), rhs->type->extTy.get());
        case '&': //address-of
            break; //TODO
        case '-': //negation
            return new TypedValue(c->builder.CreateNeg(rhs->val), rhs->type);
        case Tok_New:
            if(rhs->getType()->isSized()){
                string mallocFnName = "malloc";
                Function* mallocFn = (Function*)c->getFunction(mallocFnName)->val;

                auto size = rhs->getType()->getPrimitiveSizeInBits();
                Value *sizeVal = ConstantInt::get(getGlobalContext(), APInt(32, size, true));
                
                Value *voidPtr = c->builder.CreateCall(mallocFn, sizeVal);
                Type *ptrTy = rhs->getType()->getPointerTo();
                Value *typedPtr = c->builder.CreatePointerCast(voidPtr, ptrTy);

                //finally store rhs into the malloc'd slot
                c->builder.CreateStore(rhs->val, typedPtr);

                TypeNode *tyn = mkAnonTypeNode(TT_Ptr);
                tyn->extTy.reset(rhs->type.get());
                return new TypedValue(typedPtr, tyn);
            }
    }
    
    return c->compErr("Unknown unary operator " + Lexer::getTokStr(op), loc);
}
