#include "compiler.h"
#include "tokens.h"
#include "jitlinker.h"
#include <llvm/ExecutionEngine/GenericValue.h>
#include <llvm/ExecutionEngine/Interpreter.h>
#include <llvm/Linker/Linker.h>


TypedValue* Compiler::compAdd(TypedValue *l, TypedValue *r, BinOpNode *op){
    switch(l->type->type){
        case TT_I8:  case TT_U8:  case TT_C8:
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
        case TT_I8:  case TT_U8:  case TT_C8:
        case TT_I16: case TT_U16:
        case TT_I32: case TT_U32:
        case TT_I64: case TT_U64:
        case TT_Ptr:
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
        case TT_I8:  case TT_U8:  case TT_C8:
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
        case TT_U8: case TT_C8:
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
        case TT_U8: case TT_C8:
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

/*
 *  Compiles the extract operator, [
 */
TypedValue* Compiler::compExtract(TypedValue *l, TypedValue *r, BinOpNode *op){
    auto *lNxtTy = l->type->next.release();
    l->type->next.reset(r->type.get());

    //now look for the function
    auto *fn = getMangledFunction("#", l->type.get());
    l->type->next.release();
    l->type->next.reset(lNxtTy);

    //operator function found
    if(fn){
        //dont even bother type checking, assume the name mangling was performed correctly
        return new TypedValue(
                builder.CreateCall(fn->val, {l->val, r->val}),
                deepCopyTypeNode(fn->type->extTy.get())
        );
    }
    
    if(!isIntTypeTag(r->type->type)){
        return compErr("Index of operator '[' must be an integer expression, got expression of type " + typeNodeToStr(r->type.get()), op->loc);
    }

    if(l->type->type == TT_Array){
        //check for alloca
        if(LoadInst *li = dyn_cast<LoadInst>(l->val)){
            Value *arr = li->getPointerOperand();
            
            vector<Value*> indices;
            indices.push_back(ConstantInt::get(ctxt, APInt(64, 0, true)));
            indices.push_back(r->val);
            return new TypedValue(builder.CreateLoad(builder.CreateGEP(arr, indices)), l->type->extTy);
        }else{
            return new TypedValue(builder.CreateExtractElement(l->val, r->val), l->type->extTy.get());
        }
    }else if(l->type->type == TT_Ptr){
        return new TypedValue(builder.CreateLoad(builder.CreateGEP(l->val, r->val)), l->type->extTy);

    }else if(l->type->type == TT_Tuple || l->type->type == TT_Data){
		auto indexval = dyn_cast<ConstantInt>(r->val);
        if(!indexval)
            return compErr("Tuple indices must always be known at compile time.", op->loc);

        auto index = indexval->getZExtValue();

        //get the type from the index in question
        TypeNode* indexTyn = l->type->extTy.get();

        if(!indexTyn && l->type->type == TT_Data){
            auto *dataty = lookupType(l->type->typeName);
            if(!dataty)
                return compErr("Error when attempting to index variable of type " + typeNodeToStr(l->type.get()), op->loc);

            indexTyn = dataty->tyn->extTy.get();
        }

        for(unsigned i = 0; i < index; i++)
            indexTyn = (TypeNode*)indexTyn->next.get();

        Value *tup = llvmTypeToTypeTag(l->getType()) == TT_Ptr ? builder.CreateLoad(l->val) : l->val;
        return new TypedValue(builder.CreateExtractValue(tup, index), deepCopyTypeNode(indexTyn));
    }
    return compErr("Type " + llvmTypeToStr(l->getType()) + " does not have elements to access", op->loc);
}


/*
 *  Compiles an insert statement for arrays or tuples.
 *  An insert statement would look similar to the following (in ante syntax):
 *
 *  i32,i32,i32 tuple = (1, 2, 4)
 *  tuple#2 = 3
 *
 *  This method Works on lvals and returns a void value.
 */
TypedValue* Compiler::compInsert(BinOpNode *op, Node *assignExpr){
    auto *tmp = op->lval->compile(this);
    if(!tmp) return 0;

    //if(!dynamic_cast<LoadInst*>(tmp->val))
    if(!tmp->hasModifier(Tok_Mut))
        return compErr("Variable must be mutable to insert values, but instead is an immutable " +
                typeNodeToStr(tmp->type.get()), op->lval->loc);

    Value *var = static_cast<LoadInst*>(tmp->val)->getPointerOperand();
    if(!var) return 0;


    auto *index = op->rval->compile(this);
    auto *newVal = assignExpr->compile(this);
    if(!var || !index || !newVal) return 0;

    switch(tmp->type->type){
        case TT_Array: {
            if(*tmp->type->extTy != *newVal->type)
                return compErr("Cannot create store of types: "+typeNodeToStr(tmp->type.get())+" <- "
                        +typeNodeToStr(newVal->type.get()), assignExpr->loc);

            builder.CreateInsertElement(tmp->val, newVal->val, index->val);
            return getVoidLiteral();
        }
        case TT_Ptr: {
            if(*tmp->type->extTy.get() != *newVal->type.get())
                return compErr("Cannot create store of types: "+typeNodeToStr(tmp->type.get())+" <- "
                        +typeNodeToStr(newVal->type.get()), assignExpr->loc);

            Value *dest = builder.CreateInBoundsGEP(tmp->getType()->getPointerElementType(), tmp->val, index->val);
            builder.CreateStore(newVal->val, dest);
            return getVoidLiteral();
        }
        case TT_Tuple: case TT_Data: {
            ConstantInt *tupIndexVal = dyn_cast<ConstantInt>(index->val);
            if(!tupIndexVal){
                return compErr("Tuple indices must always be known at compile time.", op->loc);
            }else{
                auto tupIndex = tupIndexVal->getZExtValue();

                //Type of element at tuple index tupIndex, for type checking
                Type* tupIndexTy = tmp->val->getType()->getStructElementType(tupIndex);
                Type* exprTy = newVal->getType();

                if(!llvmTypeEq(tupIndexTy, exprTy)){
                    return compErr("Cannot assign expression of type " + llvmTypeToStr(exprTy)
                                + " to tuple index " + to_string(tupIndex) + " of type " + llvmTypeToStr(tupIndexTy),
                                assignExpr->loc);
                }

                auto *ins = builder.CreateInsertValue(tmp->val, newVal->val, tupIndex);
                builder.CreateStore(ins, var);
                return getVoidLiteral();//new TypedValue(builder.CreateStore(insertedTup, var), mkAnonTypeNode(TT_Void));
            }
        }
        default:
            return compErr("Variable being indexed must be an Array or Tuple, but instead is a(n) " +
                    typeNodeToStr(tmp->type.get()), op->loc); }
}

/*
 *  Creates a cast instruction appropriate for valToCast's type to castTy.
 */
TypedValue* createCast(Compiler *c, Type *castTy, TypeNode *tyn, TypedValue *valToCast){
    //first, see if the user created their own cast function
    if(TypedValue *fn = c->getCastFn(valToCast->type.get(), tyn)){
        vector<Value*> args;
        if(valToCast->type->type != TT_Void) args.push_back(valToCast->val);
        auto *call = c->builder.CreateCall(fn->val, args);
        return new TypedValue(call, fn->type->extTy);
    }

    //otherwise, fallback on known conversions
    if(isIntTypeTag(valToCast->type->type)){
        // int -> int  (maybe unsigned)
        if(isIntTypeTag(tyn->type)){
            return new TypedValue(c->builder.CreateIntCast(valToCast->val, castTy, isUnsignedTypeTag(tyn->type)), deepCopyTypeNode(tyn));

        // int -> float
        }else if(isFPTypeTag(tyn->type)){
            if(isUnsignedTypeTag(valToCast->type->type)){
                return new TypedValue(c->builder.CreateUIToFP(valToCast->val, castTy), deepCopyTypeNode(tyn));
            }else{
                return new TypedValue(c->builder.CreateSIToFP(valToCast->val, castTy), deepCopyTypeNode(tyn));
            }
        
        // int -> ptr
        }else if(tyn->type == TT_Ptr){
            return new TypedValue(c->builder.CreatePtrToInt(valToCast->val, castTy), deepCopyTypeNode(tyn));
        }
    }else if(isFPTypeTag(valToCast->type->type)){
        // float -> int  (maybe unsigned)
        if(isIntTypeTag(tyn->type)){
            if(isUnsignedTypeTag(tyn->type)){
                return new TypedValue(c->builder.CreateFPToUI(valToCast->val, castTy), deepCopyTypeNode(tyn));
            }else{
                return new TypedValue(c->builder.CreateFPToSI(valToCast->val, castTy), deepCopyTypeNode(tyn));
            }

        // float -> float
        }else if(isFPTypeTag(tyn->type)){
            return new TypedValue(c->builder.CreateFPCast(valToCast->val, castTy), deepCopyTypeNode(tyn));
        }

    }else if(valToCast->type->type == TT_Ptr){
        // ptr -> ptr
        if(tyn->type == TT_Ptr){
            return new TypedValue(c->builder.CreatePointerCast(valToCast->val, castTy), deepCopyTypeNode(tyn));

        // ptr -> int
        }else if(isIntTypeTag(tyn->type)){
            return new TypedValue(c->builder.CreatePtrToInt(valToCast->val, castTy), deepCopyTypeNode(tyn));
        }
    }

    //if all automatic checks fail, test for structural equality in a datatype cast!
    //This would apply for the following scenario (and all structurally equivalent types)
    //
    //type Int = i32
    //let example = Int 3
    //              ^^^^^
    auto *dataTy = c->lookupType(tyn->typeName);
    if(dataTy && *valToCast->type.get() == *dataTy->tyn.get()){
        auto *tycpy = deepCopyTypeNode(valToCast->type.get());

        //check if this is a tagged union (sum type)
        if(dataTy->isUnionTag()){
            auto *unionDataTy = c->lookupType(dataTy->getParentUnionName());
            tycpy->typeName = dataTy->getParentUnionName();
            tycpy->type = TT_TaggedUnion;

            auto t = unionDataTy->getTagVal(tyn->typeName);
            Type *variantTy = c->typeNodeToLlvmType(valToCast->type.get());

            vector<Type*> unionTys;
            unionTys.push_back(Type::getInt8Ty(c->ctxt));
            unionTys.push_back(variantTy);

            vector<Constant*> unionVals;
            unionVals.push_back(ConstantInt::get(c->ctxt, APInt(8, t, true))); //tag
            unionVals.push_back(UndefValue::get(variantTy));


            Type *unionTy = c->typeNodeToLlvmType(unionDataTy->tyn.get());

            //create a struct of (u8 tag, <union member type>)
            auto *uninitUnion = ConstantStruct::get(StructType::get(c->ctxt, unionTys), unionVals);
            auto* taggedUnion = c->builder.CreateInsertValue(uninitUnion, valToCast->val, 1);

            //allocate for the largest possible union member
            auto *alloca = c->builder.CreateAlloca(unionTy);

            //but bitcast it the the current member
            auto *castTo = c->builder.CreateBitCast(alloca, taggedUnion->getType()->getPointerTo());
            c->builder.CreateStore(taggedUnion, castTo);

            //load the original alloca, not the bitcasted one
            Value *unionVal = c->builder.CreateLoad(alloca);
            return new TypedValue(unionVal, tycpy);
        }

        tycpy->typeName = tyn->typeName;
        tycpy->type = TT_Data;
        return new TypedValue(valToCast->val, tycpy);
    //test for the reverse case, something like:  i32 example
    //where example is of type Int
    }else if(valToCast->type->typeName.size() > 0 && (dataTy = c->lookupType(valToCast->type->typeName))){
        if(*dataTy->tyn.get() == *tyn){
            auto *tycpy = deepCopyTypeNode(valToCast->type.get());
            tycpy->typeName = "";
            tycpy->type = tyn->type;
            return new TypedValue(valToCast->val, tycpy);
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
        if(*rtval->type == *typeExpr)
            return c->compErr("Typecast to same type", loc);
        
        return c->compErr("Invalid type cast " + typeNodeToStr(rtval->type.get()) + 
                " -> " + typeNodeToStr(typeExpr.get()), loc);
    }else{
        return tval;
    }
}

TypedValue* compIf(Compiler *c, IfNode *ifn, BasicBlock *mergebb, vector<pair<TypedValue*,BasicBlock*>> &branches){
    auto *cond = ifn->condition->compile(c);
    if(!cond) return 0;
    
    Function *f = c->builder.GetInsertBlock()->getParent();
    auto &blocks = f->getBasicBlockList();

    auto *thenbb = BasicBlock::Create(c->ctxt, "then");
   
    //only create the else block if this ifNode actually has an else clause
    BasicBlock *elsebb;
    
    if(ifn->elseN){
        if(dynamic_cast<IfNode*>(ifn->elseN.get())){
            elsebb = BasicBlock::Create(c->ctxt, "else");
            c->builder.CreateCondBr(cond->val, thenbb, elsebb);
    
            blocks.push_back(thenbb);
    
            c->builder.SetInsertPoint(thenbb);
            auto *thenVal = ifn->thenN->compile(c);
            auto *thenretbb = c->builder.GetInsertBlock();
            c->builder.CreateBr(mergebb);
           
            //save the 'then' value for the PhiNode after all the elifs
            branches.push_back({thenVal, thenretbb});

            blocks.push_back(elsebb);
            c->builder.SetInsertPoint(elsebb);
            return compIf(c, (IfNode*)ifn->elseN.get(), mergebb, branches);
        }else{
            elsebb = BasicBlock::Create(c->ctxt, "else");
            c->builder.CreateCondBr(cond->val, thenbb, elsebb);

            blocks.push_back(thenbb);
            blocks.push_back(elsebb);
            blocks.push_back(mergebb);
        }
    }else{
        c->builder.CreateCondBr(cond->val, thenbb, mergebb);
        blocks.push_back(thenbb);
        blocks.push_back(mergebb);
    }

    c->builder.SetInsertPoint(thenbb);
    auto *thenVal = ifn->thenN->compile(c);
    if(!thenVal) return 0;
    auto *thenretbb = c->builder.GetInsertBlock(); //bb containing final ret of then branch.


    if(!dyn_cast<ReturnInst>(thenVal->val))
        c->builder.CreateBr(mergebb);

    if(ifn->elseN){
        //save the final 'then' value for the upcoming PhiNode
        branches.push_back({thenVal, thenretbb});

        c->builder.SetInsertPoint(elsebb);
        auto *elseVal = ifn->elseN->compile(c);
        auto *elseretbb = c->builder.GetInsertBlock();

        if(!elseVal) return 0;
        if(!dyn_cast<ReturnInst>(elseVal->val))
            c->builder.CreateBr(mergebb);
        
        //save the final else
        branches.push_back({elseVal, elseretbb});

        if(!thenVal) return 0;


        if(*thenVal->type.get() != *elseVal->type.get() && !dyn_cast<ReturnInst>(thenVal->val) && !dyn_cast<ReturnInst>(elseVal->val))
            return c->compErr("If condition's then expr's type " + typeNodeToStr(thenVal->type.get()) +
                            " does not match the else expr's type " + typeNodeToStr(elseVal->type.get()), ifn->loc);


        c->builder.SetInsertPoint(mergebb);

        //finally, create the ret value of this if expr, unless it is of void type
        if(thenVal->type->type != TT_Void){
            auto *phi = c->builder.CreatePHI(thenVal->getType(), branches.size());
            for(auto &pair : branches)
                if(!dyn_cast<ReturnInst>(pair.first->val))
                    phi->addIncoming(pair.first->val, pair.second);

            return new TypedValue(phi, thenVal->type);
        }else{
            return c->getVoidLiteral();
        }
    }else{
        c->builder.SetInsertPoint(mergebb);
        return c->getVoidLiteral();
    }
}

TypedValue* IfNode::compile(Compiler *c){
    auto branches = vector<pair<TypedValue*,BasicBlock*>>();
    auto *mergebb = BasicBlock::Create(c->ctxt, "endif");
    return compIf(c, this, mergebb, branches);
}

TypedValue* Compiler::compMemberAccess(Node *ln, VarNode *field, BinOpNode *binop){
    if(!ln) return 0;

    if(auto *tn = dynamic_cast<TypeNode*>(ln)){
        //since ln is a typenode, this is a static field/method access, eg Math.rand
        string valName = typeNodeToStr(tn) + "_" + field->name;

        auto l = getFunctionList(valName);

        if(l.size() == 1){
            auto *fd = l.front();
            if(!fd->tv)
                fd->tv = compFn(fd->fdn, fd->scope);

            return fd->tv;
        }else if(l.size() > 1)
            return compErr("Multiple static methods of the same name with different parameters are currently unimplemented.  In the mean time, you can use global functions.", field->loc);

        return compErr("No static method called '" + field->name + "' was found in type " + 
                typeNodeToStr(tn), binop->loc);
    }else{
        //ln is not a typenode, so this is not a static method call
        Value *val;
        TypeNode *tyn;

        //prevent l from being used after this scope; only val and tyn should be used as only they
        //are updated with the automatic pointer dereferences.
        { 
            auto *l = ln->compile(this);
            if(!l) return 0;

            val = l->val;
            tyn = l->type.get();
        }

        //the . operator automatically dereferences pointers, so update val and tyn accordingly.
        while(tyn->type == TT_Ptr){
            val = builder.CreateLoad(val);
            tyn = tyn->extTy.get();
        }

        //check to see if this is a field index
        if(tyn->type == TT_Data || tyn->type == TT_Tuple){
            auto dataTy = lookupType(typeNodeToStr(tyn));

            if(dataTy){
                auto index = dataTy->getFieldIndex(field->name);

                if(index != -1){
                    TypeNode *indexTy = dataTy->tyn->extTy.get();

                    for(int i = 0; i < index; i++)
                        indexTy = (TypeNode*)indexTy->next.get();

                    //The data type when looking up (usually) does not have any modifiers,
                    //so apply any potential modifers from the parent to this
                    indexTy->copyModifiersFrom(tyn);
                    return new TypedValue(builder.CreateExtractValue(val, index), deepCopyTypeNode(indexTy));
                }
            }
        }

        //not a field, so look for a method.
        //TODO: perhaps create a calling convention function
        string funcName = typeNodeToStr(tyn) + "_" + field->name;
        auto l = getFunctionList(funcName);

        if(l.size() == 1){
            auto *fd = l.front();
            if(!fd->tv)
                fd->tv = compFn(fd->fdn, fd->scope);

            TypedValue *obj = new TypedValue(val, deepCopyTypeNode(tyn));
            auto *method_fn = new TypedValue(fd->tv->val, fd->tv->type);
            return new MethodVal(obj, method_fn);
        }else if(l.size() > 1)
            return compErr("Multiple methods of the same name with different parameters are currently unimplemented.  In the mean time, you can use global functions.", field->loc);
        else
            return compErr("Method/Field " + field->name + " not found in type " + typeNodeToStr(tyn), binop->loc);
    }
}


template<typename T>
void push_front(vector<T*> *vec, T *val){
    auto size = vec->size();
    if(size != 0)
        vec->push_back((*vec)[size-1]);

    auto it = vec->rbegin();
    for(; it != vec->rend();){
        *it = *(++it);
    }

    if(size != 0)
        (*vec)[0] = val;
    else
        vec->push_back(val);
}


TypeNode* typedValsToTypeNodes(vector<TypedValue*> &tvs){
    if(tvs.empty())
        return 0;

    TypeNode *first = 0;
    TypeNode *cur = 0;

    for(auto *tv : tvs){
        if(!first){
            first = deepCopyTypeNode(tv->type.get());
            cur = first;
        }else{
            cur->next.release();
            cur->next.reset(deepCopyTypeNode(tv->type.get()));
            cur = (TypeNode*)cur->next.get();
        }
    }

    return first;
}

//ante function to convert between IEEE half and IEEE single
//since c++ does not support an IEEE half value
//extern "C" float f16ToF32_f16(GenericValue v);

/*
 *  Converts an llvm GenericValue to a TypedValue
 */
TypedValue* genericValueToTypedValue(Compiler *c, GenericValue gv, TypeNode *tn){
    auto *copytn = deepCopyTypeNode(tn);
    switch(tn->type){
        case TT_I8:              return new TypedValue(c->builder.getInt8( *gv.IntVal.getRawData()),    copytn);
        case TT_I16:             return new TypedValue(c->builder.getInt16(*gv.IntVal.getRawData()),    copytn);
        case TT_I32:             return new TypedValue(c->builder.getInt32(*gv.IntVal.getRawData()),    copytn);
        case TT_I64:             return new TypedValue(c->builder.getInt64(*gv.IntVal.getRawData()),    copytn);
        case TT_U8:              return new TypedValue(c->builder.getInt8( *gv.IntVal.getRawData()),    copytn);
        case TT_U16:             return new TypedValue(c->builder.getInt16(*gv.IntVal.getRawData()),    copytn);
        case TT_U32:             return new TypedValue(c->builder.getInt32(*gv.IntVal.getRawData()),    copytn);
        case TT_U64:             return new TypedValue(c->builder.getInt64(*gv.IntVal.getRawData()),    copytn);
        case TT_Isz:             return new TypedValue(c->builder.getInt64(*gv.IntVal.getRawData()),    copytn);
        case TT_Usz:             return new TypedValue(c->builder.getInt64(*gv.IntVal.getRawData()),    copytn);
        case TT_C8:              return new TypedValue(c->builder.getInt8( *gv.IntVal.getRawData()),    copytn);
        case TT_C32:             return new TypedValue(c->builder.getInt32(*gv.IntVal.getRawData()),    copytn);
        case TT_F16:             return new TypedValue(ConstantFP::get(c->ctxt, APFloat(gv.FloatVal)),  copytn);
        case TT_F32:             return new TypedValue(ConstantFP::get(c->ctxt, APFloat(gv.FloatVal)),  copytn);
        case TT_F64:             return new TypedValue(ConstantFP::get(c->ctxt, APFloat(gv.DoubleVal)), copytn);
        case TT_Bool:            return new TypedValue(c->builder.getInt1(*gv.IntVal.getRawData()),     copytn);
        case TT_Tuple:           break;
        case TT_Array:           break;
        case TT_Ptr: {
            auto *cint = c->builder.getInt64((unsigned long) gv.PointerVal);
            auto *ty = c->typeNodeToLlvmType(tn);
            return new TypedValue(c->builder.CreateIntToPtr(cint, ty), copytn);
        }case TT_Data:            break;    
        case TT_TypeVar:         break;
        case TT_Function:        break;
        case TT_Method:
        case TT_TaggedUnion:
        case TT_MetaFunction:
                                 break;
        case TT_Void:
            return c->getVoidLiteral();
    }
    
    c->errFlag = true;
    cerr << "genericValueToTypedValue: Unknown TypeTag " << typeTagToStr(tn->type) << endl;
    return 0;
}

/*
 *  Converts a TypedValue to an llvm GenericValue
 *  - Assumes the Value* within the TypedValue is a Constant*
 */
GenericValue typedValueToGenericValue(Compiler *c, TypedValue *tv){
    return GenericValue(nullptr);
}


vector<GenericValue> typedValuesToGenericValues(Compiler *c, vector<TypedValue*> &typedArgs, LOC_TY loc, string fnname){
    vector<GenericValue> ret;
    ret.reserve(typedArgs.size());

    for(size_t i = 0; i < typedArgs.size(); i++){
        auto *tv = typedArgs[i];

        if(!dyn_cast<Constant>(tv->val)){
            c->compErr("Parameter " + to_string(i+1) + " of metafunction " + fnname + " is not a compile time constant", loc);
            return ret;
        }
        ret.push_back(typedValueToGenericValue(c, tv));
    }
    return ret;
}


extern map<string, CtFunc*> compapi;
/*
 *  Compile a compile-time function/macro which should not return a function call, just a compile-time constant.
 *  Ex: A call to Ante.getAST() would be a meta function as it wouldn't make sense to get the parse tree
 *      during runtime
 *
 *  - Assumes arguments are already type-checked
 */
TypedValue* compMetaFunctionResult(Compiler *c, Node *lnode, TypedValue *l, vector<TypedValue*> typedArgs){
    string fnName = l->val->getName().str();
    
    CtFunc* fn;
    if((fn = compapi[fnName])){
        void *res;

        if(fnName == "Ante_debug")
            res = (*fn)(typedArgs[0]);
        else
            res = (*fn)();

        auto gv = GenericValue(res);
        auto *conv = genericValueToTypedValue(c, gv, l->type->extTy.get());

        auto *llvmfn = static_cast<Function*>(l->val);
        if(llvmfn->getParent())
            llvmfn->removeFromParent();

        return conv;
    }else{
        LLVMInitializeNativeTarget();
        LLVMInitializeNativeAsmPrinter();

        auto* mod = wrapFnInModule(c, (Function*)l->val);

        if(!mod) return 0;

        auto* eBuilder = new EngineBuilder(unique_ptr<Module>(mod));

        string err;

        //set use interpreter; for some reason both MCJIT and its ORC replacement corrupt/free the memory
        //of c->varTable in some way in four instances: two in the call to jit->finalizeObject() and two
        //in the destructor of jit
        LLVMLinkInInterpreter();
        auto *jit = eBuilder->setErrorStr(&err).setEngineKind(EngineKind::Interpreter).create();

        if(err.length() > 0){
            cerr << err << endl;
            return 0;
        }

        string baseName = l->val->getName().str();
        auto args = typedValuesToGenericValues(c, typedArgs, lnode->loc, baseName);

        auto *fn = jit->FindFunctionNamed(fnName.c_str());
        auto genret = jit->runFunction(fn, args);

        //static_cast<Function*>(l->val)->removeFromParent();
        auto *ret = genericValueToTypedValue(c, genret, l->type->extTy.get());
        return ret;
    }
}


TypedValue* compFnCall(Compiler *c, Node *l, Node *r){
    //used to type-check each parameter later
    vector<TypedValue*> typedArgs;
    vector<Value*> args;

    //add all remaining arguments
    if(auto *tup = dynamic_cast<TupleNode*>(r)){
        typedArgs = tup->unpack(c);
        for(TypedValue *v : typedArgs){
            if(!v) return 0;
            args.push_back(v->val);
        }
    }else{ //single parameter being applied
        auto *param = r->compile(c);
        if(!param) return 0;
        if(param->type->type != TT_Void){
            typedArgs.push_back(param);
            args.push_back(param->val);
        }
    }


    //try to compile the function now that the parameters are compiled.
    TypedValue *tvf = 0;

    //First, check if the lval is a symple VarNode (identifier) and then attempt to
    //inference a method call for it (inference as in if the <type>. syntax is omitted)
    if(VarNode *vn = dynamic_cast<VarNode*>(l)){
        //try to see if arg 1's type contains a method of the same name
        auto *params = typedValsToTypeNodes(typedArgs);
       
        //try to do module inference
        if(!typedArgs.empty()){
            string fnName = typeNodeToStr(typedArgs[0]->type.get()) + "_" + vn->name;
            tvf = c->getMangledFunction(fnName, params);
        }

        //if the above fails, do regular name mangling only
        if(!tvf) tvf = c->getMangledFunction(vn->name, params);
        delete params;
    }

    //if it is not a varnode/no method is found, then compile it normally
    if(!tvf) tvf = l->compile(c);

    //if there was an error, return
    if(!tvf || !tvf->val) return 0;

    //make sure the l val compiles to a function
    if(tvf->type->type != TT_Function && tvf->type->type != TT_Method && tvf->type->type != TT_MetaFunction)
        return c->compErr("Called value is not a function or method, it is a(n) " + 
                llvmTypeToStr(tvf->getType()), l->loc);

    //now that we assured it is a function, unwrap it
    Function *f = (Function*)tvf->val;
   
    //if tvf is a method, add its host object as the first argument
    if(tvf->type->type == TT_Method){
        TypedValue *obj = ((MethodVal*) tvf)->obj;
        push_front(&args, obj->val);
        push_front(&typedArgs, obj);
    }

    if(f->arg_size() != args.size() && !f->isVarArg()){
        //check if an empty tuple (a void value) is being applied to a zero argument function before continuing
        //if not checked, it will count it as an argument instead of the absence of any
        //NOTE: this has the possibly unwanted side effect of allowing 't->void function applications to be used
        //      as parameters for functions requiring 0 parameters, although this does not affect the behaviour of either.
        if(f->arg_size() != 0 || typedArgs[0]->type->type != TT_Void){
            if(args.size() == 1)
                return c->compErr("Called function was given 1 argument but was declared to take " 
                        + to_string(f->arg_size()), r->loc);
            else
                return c->compErr("Called function was given " + to_string(args.size()) + 
                        " arguments but was declared to take " + to_string(f->arg_size()), r->loc);
        }
    }

    /* unpack the tuple of arguments into a vector containing each value */
    int i = 1;
    TypeNode *paramTy = (TypeNode*)tvf->type->extTy->next.get();

    //type check each parameter
    for(auto tArg : typedArgs){
        if(!paramTy) break;

        //NOTE: llvmTypeEq tests by structural equality, TypeNode::operator== checks nominal equality
        if(*tArg->type != *paramTy){
            //param types not equal; check for implicit conversion
            if(isNumericTypeTag(tArg->type->type) && isNumericTypeTag(paramTy->type)){
                auto *widen = c->implicitlyWidenNum(tArg, paramTy->type);
                if(widen != tArg){
                    args[i-1] = widen->val;
                    paramTy = (TypeNode*)paramTy->next.get();
                    i++;
                    delete widen;
                    continue;
                }
            }

            //check for an implicit Cast function
            string castFn = typeNodeToStr(paramTy);

            if(auto *fn = c->getMangledFunction(castFn, tArg->type.get())){
                //the function's parameter is not type checked as it is assumed it was mangled correctly.
                
                //optimize case of Str -> c8* implicit cast
                if(tArg->type->typeName == "Str" && castFn == "c8*")
                    args[i-1] = c->builder.CreateExtractValue(args[i-1], 0);
                else
                    args[i-1] = c->builder.CreateCall(fn->val, tArg->val);

            }else return c->compErr("Argument " + to_string(i) + " of function is a(n) " + typeNodeToStr(tArg->type.get())
                    + " but was declared to be a(n) " + typeNodeToStr(paramTy), r->loc);
        }
        paramTy = (TypeNode*)paramTy->next.get();
        i++;
    }

    if(tvf->type->type == TT_MetaFunction){
        return compMetaFunctionResult(c, l, tvf, typedArgs);
    }

    //use tvf->val as arg, NOT f
    //
    //both a C-style cast and dyn-cast to functions fail if f is a function-pointer
    auto *call = c->builder.CreateCall(tvf->val, args);

    return new TypedValue(call, tvf->type->extTy);
}

TypedValue* Compiler::compLogicalOr(Node *lexpr, Node *rexpr, BinOpNode *op){
    Function *f = builder.GetInsertBlock()->getParent();
    auto &blocks = f->getBasicBlockList();

    auto *lhs = lexpr->compile(this);

    auto *curbbl = builder.GetInsertBlock();
    auto *orbb = BasicBlock::Create(ctxt, "or");
    auto *mergebb = BasicBlock::Create(ctxt, "merge");

    builder.CreateCondBr(lhs->val, mergebb, orbb);
    blocks.push_back(orbb);
    blocks.push_back(mergebb);


    builder.SetInsertPoint(orbb);
    auto *rhs = rexpr->compile(this);
    
    //the block must be re-gotten in case the expression contains if-exprs, while nodes,
    //or other exprs that change the current block
    auto *curbbr = builder.GetInsertBlock();
    builder.CreateBr(mergebb);
    
    if(rhs->type->type != TT_Bool)
        return compErr("The 'or' operator's rval must be of type bool, but instead is of type "+typeNodeToStr(rhs->type.get()), op->rval->loc);

    builder.SetInsertPoint(mergebb);
    auto *phi = builder.CreatePHI(rhs->getType(), 2);
   
    //short circuit, returning true if return from the first label
    phi->addIncoming(ConstantInt::get(ctxt, APInt(1, true, true)), curbbl);
    phi->addIncoming(rhs->val, curbbr);

    return new TypedValue(phi, rhs->type);
    
}

TypedValue* Compiler::compLogicalAnd(Node *lexpr, Node *rexpr, BinOpNode *op){
    Function *f = builder.GetInsertBlock()->getParent();
    auto &blocks = f->getBasicBlockList();

    auto *lhs = lexpr->compile(this);

    auto *curbbl = builder.GetInsertBlock();
    auto *andbb = BasicBlock::Create(ctxt, "and");
    auto *mergebb = BasicBlock::Create(ctxt, "merge");

    builder.CreateCondBr(lhs->val, andbb, mergebb);
    blocks.push_back(andbb);
    blocks.push_back(mergebb);


    builder.SetInsertPoint(andbb);
    auto *rhs = rexpr->compile(this);

    //the block must be re-gotten in case the expression contains if-exprs, while nodes,
    //or other exprs that change the current block
    auto *curbbr = builder.GetInsertBlock();
    builder.CreateBr(mergebb);

    if(rhs->type->type != TT_Bool)
        return compErr("The 'and' operator's rval must be of type bool, but instead is of type "+typeNodeToStr(rhs->type.get()), op->rval->loc);

    builder.SetInsertPoint(mergebb);
    auto *phi = builder.CreatePHI(rhs->getType(), 2);
   
    //short circuit, returning false if return from the first label
    phi->addIncoming(ConstantInt::get(ctxt, APInt(1, false, true)), curbbl);
    phi->addIncoming(rhs->val, curbbr);

    return new TypedValue(phi, rhs->type);
}


TypedValue* Compiler::opImplementedForTypes(int op, TypeNode *l, TypeNode *r){
    if(isNumericTypeTag(l->type) && isNumericTypeTag(r->type)){
        switch(op){
            case '+': case '-': case '*': case '/': case '%': return (TypedValue*)1;
        }
    }

    string ls = typeNodeToStr(l);
    string rs = typeNodeToStr(r);
    string baseName = Lexer::getTokStr(op);
    string fullName = baseName + "_" + ls + "_" + rs;
    
    return getFunction(baseName, fullName);
}

TypedValue* handlePrimitiveNumericOp(BinOpNode *bop, Compiler *c, TypedValue *lhs, TypedValue *rhs){
    switch(bop->op){
        case '+': return c->compAdd(lhs, rhs, bop);
        case '-': return c->compSub(lhs, rhs, bop);
        case '*': return c->compMul(lhs, rhs, bop);
        case '/': return c->compDiv(lhs, rhs, bop);
        case '%': return c->compRem(lhs, rhs, bop);
        case '<':
                    if(isFPTypeTag(lhs->type->type))
                        return new TypedValue(c->builder.CreateFCmpOLT(lhs->val, rhs->val), mkAnonTypeNode(TT_Bool));
                    else if(isUnsignedTypeTag(lhs->type->type))
                        return new TypedValue(c->builder.CreateICmpULT(lhs->val, rhs->val), mkAnonTypeNode(TT_Bool));
                    else
                        return new TypedValue(c->builder.CreateICmpSLT(lhs->val, rhs->val), mkAnonTypeNode(TT_Bool));
        case '>':
                    if(isFPTypeTag(lhs->type->type))
                        return new TypedValue(c->builder.CreateFCmpOGT(lhs->val, rhs->val), mkAnonTypeNode(TT_Bool));
                    else if(isUnsignedTypeTag(lhs->type->type))
                        return new TypedValue(c->builder.CreateICmpUGT(lhs->val, rhs->val), mkAnonTypeNode(TT_Bool));
                    else
                        return new TypedValue(c->builder.CreateICmpSGT(lhs->val, rhs->val), mkAnonTypeNode(TT_Bool));
        case '^': return new TypedValue(c->builder.CreateXor(lhs->val, rhs->val), lhs->type);
        case Tok_Eq:
                    if(isFPTypeTag(lhs->type->type))
                        return new TypedValue(c->builder.CreateFCmpOEQ(lhs->val, rhs->val), mkAnonTypeNode(TT_Bool));
                    else
                        return new TypedValue(c->builder.CreateICmpEQ(lhs->val, rhs->val), mkAnonTypeNode(TT_Bool));
        case Tok_NotEq:
                    if(isFPTypeTag(lhs->type->type))
                        return new TypedValue(c->builder.CreateFCmpONE(lhs->val, rhs->val), mkAnonTypeNode(TT_Bool));
                    else
                        return new TypedValue(c->builder.CreateICmpNE(lhs->val, rhs->val), mkAnonTypeNode(TT_Bool));
        case Tok_LesrEq:
                    if(isFPTypeTag(lhs->type->type))
                        return new TypedValue(c->builder.CreateFCmpOLE(lhs->val, rhs->val), mkAnonTypeNode(TT_Bool));
                    else if(isUnsignedTypeTag(lhs->type->type))
                        return new TypedValue(c->builder.CreateICmpULE(lhs->val, rhs->val), mkAnonTypeNode(TT_Bool));
                    else
                        return new TypedValue(c->builder.CreateICmpSLE(lhs->val, rhs->val), mkAnonTypeNode(TT_Bool));
        case Tok_GrtrEq:
                    if(isFPTypeTag(lhs->type->type))
                        return new TypedValue(c->builder.CreateFCmpOGE(lhs->val, rhs->val), mkAnonTypeNode(TT_Bool));
                    else if(isUnsignedTypeTag(lhs->type->type))
                        return new TypedValue(c->builder.CreateICmpUGE(lhs->val, rhs->val), mkAnonTypeNode(TT_Bool));
                    else
                        return new TypedValue(c->builder.CreateICmpSGE(lhs->val, rhs->val), mkAnonTypeNode(TT_Bool));
        default:
            return c->compErr("Operator " + Lexer::getTokStr(bop->op) + " is not overloaded for types "
                   + typeNodeToStr(lhs->type.get()) + " and " + typeNodeToStr(rhs->type.get()), bop->loc);
    }
}

/*
 *  Compiles an operation along with its lhs and rhs
 */
TypedValue* BinOpNode::compile(Compiler *c){
    switch(op){
        case '.': return c->compMemberAccess(lval.get(), (VarNode*)rval.get(), this);
        case '(': return compFnCall(c, lval.get(), rval.get());
        case Tok_And: return c->compLogicalAnd(lval.get(), rval.get(), this);
        case Tok_Or: return c->compLogicalOr(lval.get(), rval.get(), this);
    }


    TypedValue *lhs = lval->compile(c);
    TypedValue *rhs = rval->compile(c);
    if(!lhs || !rhs) return 0;
    
    if(op == ';') return rhs;
    if(op == '#') return c->compExtract(lhs, rhs, this);


    //Check if both Values are numeric, and if so, check if their types match.
    //If not, do an implicit conversion (usually a widening) to match them.
    c->handleImplicitConversion(&lhs, &rhs);
            

    //first, if both operands are primitive numeric types, use the default ops
    if(isNumericTypeTag(lhs->type->type) && isNumericTypeTag(rhs->type->type)){
        return handlePrimitiveNumericOp(this, c, lhs, rhs);

    //and bools/ptrs are only compatible with == and !=
    }else if((lhs->type->type == TT_Bool and rhs->type->type == TT_Bool) or
             (lhs->type->type == TT_Ptr  and rhs->type->type == TT_Ptr)){
        
        switch(op){
            case Tok_Eq: return new TypedValue(c->builder.CreateICmpEQ(lhs->val, rhs->val), mkAnonTypeNode(TT_Bool));
            case Tok_NotEq: return new TypedValue(c->builder.CreateICmpNE(lhs->val, rhs->val), mkAnonTypeNode(TT_Bool));
        }
    }

    //otherwise check if the operator is overloaded
    //
    //first create the parameter tuple by setting lty.next = rty
    auto *lNxtTy = lhs->type->next.get();
    lhs->type->next.release();
    lhs->type->next.reset(rhs->type.get());

    //now look for the function
    auto *fn = c->getMangledFunction(Lexer::getTokStr(op), lhs->type.get());

    //and make swap lTys old next value back
    lhs->type->next.release();
    lhs->type->next.reset(lNxtTy);

    //operator function found
    if(fn){
        //dont even bother type checking, assume the name mangling was performed correctly
        return new TypedValue(
                c->builder.CreateCall(fn->val, {lhs->val, rhs->val}),
                deepCopyTypeNode(fn->type->extTy.get())
        );
    }

    return c->compErr("Operator " + Lexer::getTokStr(op) + " is not overloaded for types "
            + typeNodeToStr(lhs->type.get()) + " and " + typeNodeToStr(rhs->type.get()), loc);
}


TypedValue* UnOpNode::compile(Compiler *c){
    TypedValue *rhs = rval->compile(c);
    if(!rhs) return 0;

    switch(op){
        case '@': //pointer dereference
            if(rhs->type->type != TT_Ptr){
                return c->compErr("Cannot dereference non-pointer type " + llvmTypeToStr(rhs->getType()), loc);
            }
            
            return new TypedValue(c->builder.CreateLoad(rhs->val), rhs->type->extTy);
        case '&': {//address-of
            auto *oldTy = deepCopyTypeNode(rhs->type.get());
            auto *ptrTy = mkAnonTypeNode(TT_Ptr);
            ptrTy->extTy.reset(oldTy);

            if(LoadInst* li = dyn_cast<LoadInst>(rhs->val)){
                return new TypedValue(li->getPointerOperand(), ptrTy);
            }else{
                //if it is not stack-allocated already, allocate it on the stack
                auto *alloca = c->builder.CreateAlloca(rhs->getType());
                c->builder.CreateStore(rhs->val, alloca);
                return new TypedValue(alloca, ptrTy);
            }}
        case '-': //negation
            return new TypedValue(c->builder.CreateNeg(rhs->val), rhs->type);
        case Tok_Not:
            if(rhs->type->type != TT_Bool)
                return c->compErr("Unary not operator not overloaded for type " + typeNodeToStr(rhs->type.get()), loc);

            return new TypedValue(c->builder.CreateNot(rhs->val), rhs->type);
        case Tok_New:
            //the 'new' keyword in ante creates a reference to any existing value

            if(rhs->getType()->isSized()){
                string mallocFnName = "malloc";
                Function* mallocFn = (Function*)c->getFunction(mallocFnName, mallocFnName)->val;

                unsigned size = rhs->type->getSizeInBits(c) / 8;

                Value *sizeVal = ConstantInt::get(c->ctxt, APInt(32, size, true));

                Value *voidPtr = c->builder.CreateCall(mallocFn, sizeVal);
                Type *ptrTy = rhs->getType()->getPointerTo();
                Value *typedPtr = c->builder.CreatePointerCast(voidPtr, ptrTy);

                //finally store rhs into the malloc'd slot
                c->builder.CreateStore(rhs->val, typedPtr);

                TypeNode *tyn = mkAnonTypeNode(TT_Ptr);
                tyn->extTy.reset(deepCopyTypeNode(rhs->type.get()));

                auto *ret = new TypedValue(typedPtr, tyn);


                //Create an upper-case name so it cannot be referenced normally
                string tmpAllocName = "_New" + to_string((unsigned long)ret);
                c->stoVar(tmpAllocName, new Variable(tmpAllocName, ret, c->scope, false /*always free*/));

                return ret;
            }
    }
    
    return c->compErr("Unknown unary operator " + Lexer::getTokStr(op), loc);
}
