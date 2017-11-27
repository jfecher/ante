#include <llvm/ExecutionEngine/Interpreter.h>
#include <llvm/Linker/Linker.h>
#include "compiler.h"
#include "types.h"
#include "function.h"
#include "tokens.h"
#include "jitlinker.h"
#include "types.h"
#include "jit.h"
#include "argtuple.h"

using namespace std;
using namespace llvm;
using namespace ante::parser;

namespace ante {

TypedValue Compiler::compAdd(TypedValue &l, TypedValue &r, BinOpNode *op){
    switch(l.type->typeTag){
        case TT_I8:  case TT_U8:  case TT_C8:
        case TT_I16: case TT_U16:
        case TT_I32: case TT_U32:
        case TT_I64: case TT_U64:
        case TT_Isz: case TT_Usz:
        case TT_Ptr:
            return TypedValue(builder.CreateAdd(l.val, r.val), l.type);
        case TT_F16:
        case TT_F32:
        case TT_F64:
            return TypedValue(builder.CreateFAdd(l.val, r.val), l.type);

        default:
            return compErr("binary operator + is undefined for the type " + anTypeToColoredStr(l.type) + " and " + anTypeToColoredStr(r.type), op->loc);
    }
}

TypedValue Compiler::compSub(TypedValue &l, TypedValue &r, BinOpNode *op){
    switch(l.type->typeTag){
        case TT_I8:  case TT_U8:  case TT_C8:
        case TT_I16: case TT_U16:
        case TT_I32: case TT_U32:
        case TT_I64: case TT_U64:
        case TT_Isz: case TT_Usz:
        case TT_Ptr:
            return TypedValue(builder.CreateSub(l.val, r.val), l.type);
        case TT_F16:
        case TT_F32:
        case TT_F64:
            return TypedValue(builder.CreateFSub(l.val, r.val), l.type);

        default:
            return compErr("binary operator - is undefined for the type " + anTypeToColoredStr(l.type) + " and " + anTypeToColoredStr(r.type), op->loc);
    }
}

TypedValue Compiler::compMul(TypedValue &l, TypedValue &r, BinOpNode *op){
    switch(l.type->typeTag){
        case TT_I8:  case TT_U8:  case TT_C8:
        case TT_I16: case TT_U16:
        case TT_I32: case TT_U32:
        case TT_I64: case TT_U64:
        case TT_Isz: case TT_Usz:
            return TypedValue(builder.CreateMul(l.val, r.val), l.type);
        case TT_F16:
        case TT_F32:
        case TT_F64:
            return TypedValue(builder.CreateFMul(l.val, r.val), l.type);

        default:
            return compErr("binary operator * is undefined for the type " + anTypeToColoredStr(l.type) + " and " + anTypeToColoredStr(r.type), op->loc);
    }
}

TypedValue Compiler::compDiv(TypedValue &l, TypedValue &r, BinOpNode *op){
    switch(l.type->typeTag){
        case TT_I8:
        case TT_I16:
        case TT_I32:
        case TT_I64:
        case TT_Isz:
            return TypedValue(builder.CreateSDiv(l.val, r.val), l.type);
        case TT_U8: case TT_C8:
        case TT_U16:
        case TT_U32:
        case TT_U64:
        case TT_Usz:
            return TypedValue(builder.CreateUDiv(l.val, r.val), l.type);
        case TT_F16:
        case TT_F32:
        case TT_F64:
            return TypedValue(builder.CreateFDiv(l.val, r.val), l.type);

        default:
            return compErr("binary operator / is undefined for the type " + anTypeToColoredStr(l.type) + " and " + anTypeToColoredStr(r.type), op->loc);
    }
}

TypedValue Compiler::compRem(TypedValue &l, TypedValue &r, BinOpNode *op){
    switch(l.type->typeTag){
        case TT_I8:
        case TT_I16:
        case TT_I32:
        case TT_I64:
        case TT_Isz:
            return TypedValue(builder.CreateSRem(l.val, r.val), l.type);
        case TT_U8: case TT_C8:
        case TT_U16:
        case TT_U32:
        case TT_U64:
        case TT_Usz:
            return TypedValue(builder.CreateURem(l.val, r.val), l.type);
        case TT_F16:
        case TT_F32:
        case TT_F64:
            return TypedValue(builder.CreateFRem(l.val, r.val), l.type);

        default:
            return compErr("binary operator % is undefined for the types " + anTypeToColoredStr(l.type) + " and " + anTypeToColoredStr(r.type), op->loc);
    }
}

/*
 *  Compiles the extract operator, #
 */
TypedValue Compiler::compExtract(TypedValue &l, TypedValue &r, BinOpNode *op){
    if(!isIntTypeTag(r.type->typeTag)){
        return compErr("Index of operator '#' must be an integer expression, got expression of type " + anTypeToColoredStr(r.type), op->loc);
    }

    if(auto *arrty = dyn_cast<AnArrayType>(l.type)){
        //check for alloca
        if(LoadInst *li = dyn_cast<LoadInst>(l.val)){
            Value *arr = li->getPointerOperand();

            vector<Value*> indices;
            indices.push_back(ConstantInt::get(*ctxt, APInt(64, 0, true)));
            indices.push_back(r.val);
            return TypedValue(builder.CreateLoad(builder.CreateGEP(arr, indices)), arrty->extTy);
        }else{
            return TypedValue(builder.CreateExtractElement(l.val, r.val), arrty->extTy);
        }
    }else if(auto *ptrty = dyn_cast<AnPtrType>(l.type)){
        return TypedValue(builder.CreateLoad(builder.CreateGEP(l.val, r.val)), ptrty->extTy);

    }else if(l.type->typeTag == TT_Tuple || l.type->typeTag == TT_Data){
		auto indexval = dyn_cast<ConstantInt>(r.val);
        if(!indexval)
            return compErr("Tuple indices must always be known at compile time.", op->loc);

        auto index = indexval->getZExtValue();

        auto *aggty = (AnAggregateType*)l.type;

        if(index >= aggty->extTys.size())
            return compErr("Index of " + to_string(index) + " exceeds number of fields in " + anTypeToColoredStr(l.type), op->loc);

        AnType *indexTy = aggty->extTys[index];

        Value *tup = l.getType()->isPointerTy() ? builder.CreateLoad(l.val) : l.val;
        return TypedValue(builder.CreateExtractValue(tup, index), indexTy);
    }
    return compErr("Type " + anTypeToColoredStr(l.type) + " does not have elements to access", op->loc);
}


/*
 *  Compiles an insert statement for arrays or tuples.
 *  An insert statement would look similar to the following:
 *
 *  var tuple = ("one", 2.0, 4)
 *  tuple#2 = 3
 *
 *  This method works on lvals and returns a void value.
 */
TypedValue Compiler::compInsert(BinOpNode *op, Node *assignExpr){
    auto tmp = op->lval->compile(this);

    //if(!dynamic_cast<LoadInst*>(tmp->val))
    if(!tmp.type->hasModifier(Tok_Mut))
        return compErr("Variable must be mutable to insert values, but instead is an immutable " +
                anTypeToColoredStr(tmp.type), op->lval->loc);

    Value *var = static_cast<LoadInst*>(tmp.val)->getPointerOperand();

    auto index = op->rval->compile(this);
    auto newVal = assignExpr->compile(this);

    //see if insert operator # = is overloaded already
    string basefn = "#";
    string mangledfn = mangle(basefn, {tmp.type, AnType::getI32(), newVal.type});
    auto fn = getFunction(basefn, mangledfn);
    if(!!fn){
        vector<Value*> args = {var, index.val, newVal.val};
        auto *retty = ((AnAggregateType*)fn.type)->extTys[0];
        return TypedValue(builder.CreateCall(fn.val, args), retty);
    }

    switch(tmp.type->typeTag){
        case TT_Array: {
            auto *arrty = (AnArrayType*)tmp.type;
            if(!typeEq(arrty->extTy, newVal.type))
                return compErr("Cannot create store of types: "+anTypeToColoredStr(tmp.type)+" <- "
                        +anTypeToColoredStr(newVal.type), assignExpr->loc);

            Value *cast = builder.CreateBitCast(var, var->getType()->getPointerElementType()->getArrayElementType()->getPointerTo());
            Value *dest = builder.CreateInBoundsGEP(cast, index.val);
            builder.CreateStore(newVal.val, dest);
            return getVoidLiteral();
        }
        case TT_Ptr: {
            auto *ptrty = (AnPtrType*)tmp.type;
            if(!typeEq(ptrty->extTy, newVal.type))
                return compErr("Cannot create store of types: "+anTypeToColoredStr(tmp.type)+" <- "
                        +anTypeToColoredStr(newVal.type), assignExpr->loc);

            Value *dest = builder.CreateInBoundsGEP(/*tmp->getType()->getPointerElementType(),*/ tmp.val, index.val);
            builder.CreateStore(newVal.val, dest);
            return getVoidLiteral();
        }
        case TT_Tuple: case TT_Data: {
            ConstantInt *tupIndexVal = dyn_cast<ConstantInt>(index.val);
            if(!tupIndexVal){
                return compErr("Tuple indices must always be known at compile time", op->loc);
            }else{
                auto tupIndex = tupIndexVal->getZExtValue();
                auto *aggty = (AnAggregateType*)tmp.type;

                if(tupIndex >= aggty->extTys.size())
                    compErr("Index of " + to_string(tupIndex) + " exceeds the maximum index of the tuple, "
                            + to_string(aggty->extTys.size()-1), op->loc);

                //Type of element at tuple index tupIndex, for type checking
                auto *tupIndexTy = aggty->extTys[tupIndex];

                if(!typeEq(tupIndexTy, newVal.type)){
                    return compErr("Cannot assign expression of type " + anTypeToColoredStr(newVal.type)
                                + " to tuple index " + to_string(tupIndex) + " of type " + anTypeToColoredStr(tupIndexTy),
                                assignExpr->loc);
                }

                auto *ins = builder.CreateInsertValue(tmp.val, newVal.val, tupIndex);
                builder.CreateStore(ins, var);
                return getVoidLiteral();//new TypedValue(builder.CreateStore(insertedTup, var), mkAnonTypeNode(TT_Void));
            }
        }
        default:
            return compErr("Variable being indexed must be an Array or Tuple, but instead is a(n) " +
                    anTypeToColoredStr(tmp.type), op->loc); }
}


TypedValue createUnionVariantCast(Compiler *c, TypedValue &valToCast, string &tagName, AnDataType *dataTy, TypeCheckResult &tyeq){
    auto *unionDataTy = dataTy->parentUnionType;

    if(tyeq->res == TypeCheckResult::SuccessWithTypeVars){
        unionDataTy = (AnDataType*)bindGenericToType(c, unionDataTy, tyeq->bindings);
    }

    Type *variantTy = c->anTypeToLlvmType(valToCast.type);

    auto tagVal = unionDataTy->getTagVal(tagName);

    vector<Type*> unionTys;
    unionTys.push_back(Type::getInt8Ty(*c->ctxt));
    unionTys.push_back(variantTy);

    vector<Constant*> unionVals;
    unionVals.push_back(ConstantInt::get(*c->ctxt, APInt(8, tagVal, true))); //tag
    unionVals.push_back(UndefValue::get(variantTy));

    Type *unionTy = c->anTypeToLlvmType(unionDataTy);

    //create a struct of (u8 tag, <union member type>)
    auto *uninitUnion = ConstantStruct::get(StructType::get(*c->ctxt, unionTys, true), unionVals);
    auto* taggedUnion = c->builder.CreateInsertValue(uninitUnion, valToCast.val, 1);

    //allocate for the largest possible union member
    auto *alloca = c->builder.CreateAlloca(unionTy);

    //but bitcast it the the current member
    auto *castTo = c->builder.CreateBitCast(alloca, taggedUnion->getType()->getPointerTo());
    c->builder.CreateStore(taggedUnion, castTo);

    //load the original alloca, not the bitcasted one
    Value *unionVal = c->builder.CreateLoad(alloca);

    return TypedValue(unionVal, unionDataTy);
}


string getCastFnBaseName(AnType *t){
    if(auto *dt = dyn_cast<AnDataType>(t)){
        if(dt->unboundType)
            return anTypeToStrWithoutModifiers(dt->unboundType) + "_init";
    }
    return anTypeToStr(t) + "_init";
}


TypedValue compMetaFunctionResult(Compiler *c, LOC_TY &loc, string &baseName, string &mangledName, vector<TypedValue> &typedArgs);


struct ReinterpretCastResult {
    enum ReinterpretCastType {
        NoCast,
        ValToStruct,
        ValToUnion,
        ValToPrimitive
    } type;

    TypeCheckResult typeCheck;
    AnDataType *dataTy;
};


vector<AnType*> toArgTuple(AnType *ty){
    if(ty->typeTag == TT_Tuple){
        return ((AnAggregateType*)ty)->extTys;
    }else if(ty->typeTag == TT_Void){
        return {};
    }else{
        return {ty};
    }
}

/*
 * Check if a reinterpret cast can be performed and return some
 * information about the type of cast so that no double lookup
 * is needed
 */
ReinterpretCastResult checkForReinterpretCast(Compiler *c, AnType *castTy, TypedValue &valToCast){
    auto *dataTy = dyn_cast<AnDataType>(castTy);

    if(dataTy){
        auto argTup = toArgTuple(valToCast.type);
        auto tc = c->typeEq(dataTy->extTys, argTup);

        if(!!tc){
            if(dataTy->isUnionTag())
                return {ReinterpretCastResult::ValToUnion, tc, dataTy};
            else
                return {ReinterpretCastResult::ValToStruct, tc, dataTy};
        }
    }

    if(auto *valDt = dyn_cast<AnDataType>(valToCast.type)){
        auto argTup = toArgTuple(castTy);

        auto tc = c->typeEq(valDt->extTys, argTup);
        if(!!tc){
            return {ReinterpretCastResult::ValToPrimitive, tc, dataTy};
        }
    }

    return {ReinterpretCastResult::NoCast, {}, nullptr};
}




TypedValue doReinterpretCast(Compiler *c, AnType *castTy, TypedValue &valToCast, ReinterpretCastResult &rcr){
    if(rcr.type == ReinterpretCastResult::NoCast){
        return {};

    }else if(rcr.type == ReinterpretCastResult::ValToPrimitive){
        return TypedValue(valToCast.val, castTy);

    }else{ //ValToUnion or ValToStruct
        bool isUnion = rcr.type == ReinterpretCastResult::ValToUnion;
        auto *to_tyn = rcr.dataTy;

        string tag;
        if(((AnDataType*)castTy)->unboundType)
            tag = ((AnDataType*)((AnDataType*)castTy)->unboundType)->name;
        else
            tag = ((AnDataType*)castTy)->name;
        //to_tyn->typeName = castTy->typeName;
        //to_tyn->type = isUnion ? TT_TaggedUnion : TT_Data;

        if(rcr.typeCheck->res == TypeCheckResult::SuccessWithTypeVars){
            to_tyn = (AnDataType*)bindGenericToType(c, to_tyn, rcr.typeCheck->bindings);
        }

        if(isUnion) return createUnionVariantCast(c, valToCast, tag, rcr.dataTy, rcr.typeCheck);
        else        return TypedValue(valToCast.val, to_tyn);
    }
}

TypedValue doReinterpretCast(Compiler *c, AnType *castTy, TypedValue &valToCast){
    auto rcr = checkForReinterpretCast(c, castTy, valToCast);
    return doReinterpretCast(c, castTy, valToCast, rcr);
}

bool preferCastOverFunction(Compiler *c, TypedValue &valToCast, ReinterpretCastResult &res, FuncDecl *fd){
    FuncDecl *curFn = c->getCurrentFunction();
    if(curFn->fdn and curFn->mangledName == fd->mangledName)
        return true;

    auto *fnTy = AnFunctionType::get(c, AnType::getVoid(), fd->fdn->params.get());
    auto args = toArgTuple(valToCast.type);

    auto tc = c->typeEq(fnTy->extTys, args);
    return tc->matches >= res.typeCheck->matches;
}


/*
 *  Creates a cast instruction appropriate for valToCast's type to castTy.
 */
TypedValue createCast(Compiler *c, AnType *castTy, TypedValue &valToCast, LOC_TY &loc){
    //first, see if the user created their own cast function
    //
    //NOTE: using getCastFuncDecl lets us not compile the function until after
    //      we have determined it is the best cast available (otherwise whenever
    //      a cast fn tries to call its default init we would have an infinite loop)
    if(FuncDecl *fd = c->getCastFuncDecl(valToCast.type, castTy)){
        vector<Value*> args;
        if(valToCast.type->typeTag != TT_Void) args.push_back(valToCast.val);

        //Check if a cast matches the valToCast closer than the function args do
        auto castResult = checkForReinterpretCast(c, castTy, valToCast);
        if(castResult.type != ReinterpretCastResult::NoCast){
            if(preferCastOverFunction(c, valToCast, castResult, fd))
                return doReinterpretCast(c, castTy, valToCast, castResult);
        }

        //Compile the function now that we know to use it over a cast
        auto fn = c->getCastFn(valToCast.type, castTy, fd);
        if(!!fn){
            if(fn.type->typeTag == TT_MetaFunction){
                string baseName = getCastFnBaseName(castTy);
                string mangledName = mangle(baseName, {valToCast.type});
                vector<TypedValue> args = {valToCast};
                return compMetaFunctionResult(c, loc, baseName, mangledName, args);
            }

            auto *call = c->builder.CreateCall(fn.val, args);
            return TypedValue(call, fn.type->getFunctionReturnType());
        }
    }

    //otherwise, fallback on known conversions
    if(isIntTypeTag(castTy->typeTag)){
        Type *llvmCastTy = c->anTypeToLlvmType(castTy);

        // int -> int  (maybe unsigned)
        if(isIntTypeTag(valToCast.type->typeTag)){
            return TypedValue(c->builder.CreateIntCast(valToCast.val, llvmCastTy,
                        isUnsignedTypeTag(castTy->typeTag)), castTy);

        // float -> int
        }else if(isFPTypeTag(valToCast.type->typeTag)){
            if(isUnsignedTypeTag(castTy->typeTag)){
                return TypedValue(c->builder.CreateFPToUI(valToCast.val, llvmCastTy), castTy);
            }else{
                return TypedValue(c->builder.CreateFPToSI(valToCast.val, llvmCastTy), castTy);
            }

        // ptr -> int
        }else if(valToCast.type->typeTag == TT_Ptr){
            return TypedValue(c->builder.CreatePtrToInt(valToCast.val, llvmCastTy), castTy);
        }
    }else if(isFPTypeTag(castTy->typeTag)){
        Type *llvmCastTy = c->anTypeToLlvmType(castTy);

        // int -> float
        if(isIntTypeTag(valToCast.type->typeTag)){
            if(isUnsignedTypeTag(valToCast.type->typeTag)){
                return TypedValue(c->builder.CreateUIToFP(valToCast.val, llvmCastTy), castTy);
            }else{
                return TypedValue(c->builder.CreateSIToFP(valToCast.val, llvmCastTy), castTy);
            }

        // float -> float
        }else if(isFPTypeTag(valToCast.type->typeTag)){
            return TypedValue(c->builder.CreateFPCast(valToCast.val, llvmCastTy), castTy);
        }

    }else if(castTy->typeTag == TT_Ptr){
        Type *llvmCastTy = c->anTypeToLlvmType(castTy);

        // ptr -> ptr
        if(valToCast.type->typeTag == TT_Ptr){
            return TypedValue(c->builder.CreatePointerCast(valToCast.val, llvmCastTy), castTy);

		// int -> ptr
        }else if(isIntTypeTag(valToCast.type->typeTag)){
            return TypedValue(c->builder.CreateIntToPtr(valToCast.val, llvmCastTy), castTy);
        }
    }

    //NOTE: doReinterpretCast only casts if a valid cast is found,
    //      if no valid cast is found nullptr is returned
    return doReinterpretCast(c, castTy, valToCast);
}

TypedValue TypeCastNode::compile(Compiler *c){
    auto rtval = rval->compile(c);

    auto *ty = toAnType(c, typeExpr.get());
    if(ty->isGeneric){
        TypeCheckResult tc;
        //if(auto *dt = dyn_cast<AnDataType>(ty)){
        //    if(dt->isUnionTag()){
        //        tc = c->typeEq(dt->extTys[0], rtval.type);
        //    }
        //}

        if(tc->res != TypeCheckResult::SuccessWithTypeVars)
            tc = c->typeEq(ty, rtval.type);

        ty = bindGenericToType(c, ty, tc->bindings);

        //if(ty->isGeneric)
        //    c->compErr("Cannot cast to a generic type " + anTypeToColoredStr(ty), typeExpr->loc);
    }

    auto tval = createCast(c, ty, rtval, loc);

    if(!tval){
        //if(!!c->typeEq(rtval->type.get(), ty))
        //    c->compErr("Typecast to same type", loc, ErrorType::Warning);

        return c->compErr("Invalid type cast " + anTypeToColoredStr(rtval.type) +
                " -> " + anTypeToColoredStr(ty), loc);
    }
    return tval;
}

TypedValue compIf(Compiler *c, IfNode *ifn, BasicBlock *mergebb, vector<pair<TypedValue,BasicBlock*>> &branches){
    auto cond = ifn->condition->compile(c);

    if(cond.type->typeTag != TT_Bool)
        return c->compErr("If condition must be of type " + anTypeToColoredStr(AnType::getBool()) +
                    " but an expression of type " + anTypeToColoredStr(cond.type) + " was given", ifn->condition->loc);

    Function *f = c->builder.GetInsertBlock()->getParent();
    auto &blocks = f->getBasicBlockList();

    auto *thenbb = BasicBlock::Create(*c->ctxt, "then");

    //only create the else block if this ifNode actually has an else clause
    BasicBlock *elsebb = 0;

    if(ifn->elseN){
        if(dynamic_cast<IfNode*>(ifn->elseN.get())){
            elsebb = BasicBlock::Create(*c->ctxt, "elif");
            c->builder.CreateCondBr(cond.val, thenbb, elsebb);

            blocks.push_back(thenbb);
            c->builder.SetInsertPoint(thenbb);
            auto thenVal = ifn->thenN->compile(c);

            //If a break, continue, or return was encountered then this branch doesn't merge to the endif
            if(!dyn_cast<ReturnInst>(thenVal.val) and !dyn_cast<BranchInst>(thenVal.val)){
                auto *thenretbb = c->builder.GetInsertBlock();
                c->builder.CreateBr(mergebb);

                //save the 'then' value for the PhiNode after all the elifs
                branches.push_back({thenVal, thenretbb});

                blocks.push_back(elsebb);
            }

            c->builder.SetInsertPoint(elsebb);
            return compIf(c, (IfNode*)ifn->elseN.get(), mergebb, branches);
        }else{
            elsebb = BasicBlock::Create(*c->ctxt, "else");
            c->builder.CreateCondBr(cond.val, thenbb, elsebb);

            blocks.push_back(thenbb);
            blocks.push_back(elsebb);
            blocks.push_back(mergebb);
        }
    }else{
        c->builder.CreateCondBr(cond.val, thenbb, mergebb);
        blocks.push_back(thenbb);
        blocks.push_back(mergebb);
    }

    c->builder.SetInsertPoint(thenbb);
    auto thenVal = ifn->thenN->compile(c);
    if(!thenVal) return thenVal;
    auto *thenretbb = c->builder.GetInsertBlock(); //bb containing final ret of then branch.


    if(!dyn_cast<ReturnInst>(thenVal.val) and !dyn_cast<BranchInst>(thenVal.val))
        c->builder.CreateBr(mergebb);

    if(ifn->elseN){
        //save the final 'then' value for the upcoming PhiNode
        branches.push_back({thenVal, thenretbb});

        c->builder.SetInsertPoint(elsebb);
        auto elseVal = ifn->elseN->compile(c);
        auto *elseretbb = c->builder.GetInsertBlock();

        if(!elseVal) return {};

        //save the final else
        if(!dyn_cast<ReturnInst>(elseVal.val) and !dyn_cast<BranchInst>(elseVal.val))
            branches.push_back({elseVal, elseretbb});

        if(!thenVal) return {};

        auto eq = c->typeEq(thenVal.type, elseVal.type);
        if(!eq and !dyn_cast<ReturnInst>(thenVal.val) and !dyn_cast<ReturnInst>(elseVal.val) and
                   !dyn_cast<BranchInst>(thenVal.val) and !dyn_cast<BranchInst>(elseVal.val)){

            /*
            bool tEmpty = thenVal->type->isGeneric;
            bool eEmpty = elseVal->type->isGeneric;

            if(tEmpty and not eEmpty){
                auto *dt = c->lookupType(elseVal->type.get());
                bindGenericToType(thenVal->type.get(), elseVal->type->params, dt);
                thenVal->val->mutateType(c->typeNodeToLlvmType(thenVal->type.get()));

                if(LoadInst *li = dyn_cast<LoadInst>(thenVal->val)){
                    auto *alloca = li->getPointerOperand();
                    auto *cast = c->builder.CreateBitCast(alloca, c->typeNodeToLlvmType(elseVal->type.get())->getPointerTo());
                    thenVal->val = c->builder.CreateLoad(cast);
                }
            }else if(eEmpty and not tEmpty){
                auto *dt = c->lookupType(thenVal->type.get());
                bindGenericToType(elseVal->type.get(), thenVal->type->params, dt);
                elseVal->val->mutateType(c->typeNodeToLlvmType(elseVal->type.get()));

                if(LoadInst *ri = dyn_cast<LoadInst>(elseVal->val)){
                    auto *alloca = ri->getPointerOperand();
                    auto *cast = c->builder.CreateBitCast(alloca, c->typeNodeToLlvmType(thenVal->type.get())->getPointerTo());
                    elseVal->val = c->builder.CreateLoad(cast);
                }
            }else{
            */
            return c->compErr("If condition's then expr's type " + anTypeToColoredStr(thenVal.type) +
                        " does not match the else expr's type " + anTypeToColoredStr(elseVal.type), ifn->loc);
        }

        if(eq->res == TypeCheckResult::SuccessWithTypeVars){
            bool tEmpty = thenVal.type->isGeneric;
            bool eEmpty = elseVal.type->isGeneric;

            TypedValue generic;
            TypedValue concrete;

            if(tEmpty and !eEmpty){
                generic = thenVal;
                concrete = elseVal;
            }else if(eEmpty and !tEmpty){
                generic = elseVal;
                concrete = thenVal;
            }else{
                return c->compErr("If condition's then expr's type " + anTypeToColoredStr(thenVal.type) +
                            " does not match the else expr's type " + anTypeToColoredStr(elseVal.type), ifn->loc);
            }

            generic.type = bindGenericToType(c, generic.type, eq->bindings);

            //TODO: find a way to handle this more gracefully
            generic.val->mutateType(c->anTypeToLlvmType(generic.type));

            auto *ri = dyn_cast<ReturnInst>(generic.val);

            if(LoadInst *li = dyn_cast<LoadInst>(ri ? ri->getReturnValue() : generic.val)){
                auto *alloca = li->getPointerOperand();

                auto *ins = ri ? ri->getParent() : c->builder.GetInsertBlock();
                c->builder.SetInsertPoint(ins);

                auto *cast = c->builder.CreateBitCast(alloca, c->anTypeToLlvmType(generic.type)->getPointerTo());
                auto *fixed_ret = c->builder.CreateLoad(cast);
                generic.val = fixed_ret;
                if(ri) ri->eraseFromParent();
            }
        }

        if(!dyn_cast<ReturnInst>(elseVal.val) and !dyn_cast<BranchInst>(elseVal.val))
            c->builder.CreateBr(mergebb);

        c->builder.SetInsertPoint(mergebb);

        //finally, create the ret value of this if expr, unless it is of void type
        if(thenVal.type->typeTag != TT_Void){
            auto *phi = c->builder.CreatePHI(thenVal.getType(), branches.size());

            for(auto &pair : branches)
                if(!dyn_cast<ReturnInst>(pair.first.val)){
                    phi->addIncoming(pair.first.val, pair.second);
                }

            return TypedValue(phi, thenVal.type);
        }else{
            return c->getVoidLiteral();
        }
    }else{
        c->builder.SetInsertPoint(mergebb);
        return c->getVoidLiteral();
    }
}

TypedValue IfNode::compile(Compiler *c){
    auto branches = vector<pair<TypedValue,BasicBlock*>>();
    auto *mergebb = BasicBlock::Create(*c->ctxt, "endif");
    return compIf(c, this, mergebb, branches);
}

string _anTypeToStr(const AnType *t, AnModifier *m);

string toModuleName(AnType *t){
    if(AnDataType *dt = dyn_cast<AnDataType>(t)){
        return dt->name;
    }else{
        return _anTypeToStr(t, t->mods);
    }
}

/*
 *  Compiles the member access operator, .  eg. struct.field
 */
TypedValue Compiler::compMemberAccess(Node *ln, VarNode *field, BinOpNode *binop){
    if(!ln) throw new CtError();

    if(auto *tn = dynamic_cast<TypeNode*>(ln)){
        //since ln is a typenode, this is a static field/method access, eg Math.rand
        string valName = typeNodeToStr(tn) + "_" + field->name;

        auto& l = getFunctionList(valName);

        if(!l.empty())
            return FunctionCandidates::getAsTypedValue(ctxt.get(), l, {});

        return compErr("No static method called '" + field->name + "' was found in type " +
                anTypeToColoredStr(toAnType(this, tn)), binop->loc);
    }else{
        //ln is not a typenode, so this is not a static method call
        Value *val;
        AnType *ltyn;
        AnType *tyn;

        //prevent l from being used after this scope; only val and tyn should be used as only they
        //are updated with the automatic pointer dereferences.
        {
            auto l = ln->compile(this);
            if(!l) return {};

            val = l.val;
            tyn = ltyn = l.type;
        }

        //the . operator automatically dereferences pointers, so update val and tyn accordingly.
        while(tyn->typeTag == TT_Ptr){
            val = builder.CreateLoad(val);
            tyn = ((AnPtrType*)tyn)->extTy;
        }

        //if pointer derefs took place, tyn could have lost its modifiers, so make sure they are copied back
        //
        //TODO: manage AnModifierType* or remove it entirely
        //
        //if(ltyn->typeTag == TT_Ptr and tyn->modifiers.empty() and !ltyn->modifiers.empty()){
        //    tyn->copyModifiersFrom(ltyn);
        //}

        //check to see if this is a field index
        if(auto *dataTy = dyn_cast<AnDataType>(tyn)){
            auto index = dataTy->getFieldIndex(field->name);

            if(index != -1){
                AnType *retTy = dataTy->extTys[index];

                if(dataTy->isStub()){
                    updateLlvmTypeBinding(this, dataTy, false);
                }

                //The data type when looking up (usually) does not have any modifiers,
                //so apply any potential modifers from the parent to this
                if(!retTy->mods and ltyn->mods){
                    retTy = retTy->setModifier(tyn->mods);
                }

                //If dataTy is a single value tuple then val may not be a tuple at all. In this
                //case, val should be returned without being extracted from a nonexistant tuple
                if(index == 0 and !val->getType()->isStructTy())
                    return TypedValue(val, retTy);

                auto ev = builder.CreateExtractValue(val, index);
                auto ret = TypedValue(ev, retTy);
                return ret;
            }
        }

        //not a field, so look for a method.
        //TODO: perhaps create a calling convention function
        string funcName = toModuleName(tyn) + "_" + field->name;
        auto& l = getFunctionList(funcName);



        if(!l.empty()){
            TypedValue obj = {val, tyn};
            return FunctionCandidates::getAsTypedValue(ctxt.get(), l, obj);
        }else{
            return compErr("Method/Field " + field->name + " not found in type " + anTypeToColoredStr(tyn), binop->loc);
        }
    }
}


template<typename T>
void push_front(vector<T> &vec, T val){
    vector<T> cpy;
    cpy.reserve(vec.size() + 1);
    cpy.push_back(val);

    for(auto &v : vec)
        cpy.push_back(v);

    vec = cpy;
}


vector<AnType*> toAnTypeVector(vector<TypedValue> &tvs){
    vector<AnType*> ret;
    for(const auto &tv : tvs){
        ret.push_back(tv.type);
    }
    return ret;
}


string getName(Node *n){
    if(VarNode *vn = dynamic_cast<VarNode*>(n))
        return vn->name;
    else if(BinOpNode *op = dynamic_cast<BinOpNode*>(n))
        return getName(op->lval.get()) + "_" + getName(op->rval.get());
    else if(TypeNode *tn = dynamic_cast<TypeNode*>(n))
        return tn->params.empty() ? typeNodeToStr(tn) : tn->typeName;
    else
        return "";
}

#ifdef _WIN32
void* lookupCFn(string name){
    static map<string,void*> fnMap = {
        {"printf",  (void*)printf},
        {"puts",    (void*)puts},
        {"putchar", (void*)putchar},
        {"getchar", (void*)getchar},
        {"exit",    (void*)exit},
        {"malloc",  (void*)malloc},
        {"realloc", (void*)realloc},
        {"free",    (void*)free},
        {"memcpy",  (void*)memcpy},
        {"system",  (void*)system},
        {"strlen",  (void*)strlen},
        {"fopen",   (void*)fopen},
        {"fclose",  (void*)fclose},
        {"fputs",   (void*)fputs},
        {"fputc",   (void*)fputc},
        {"fgetc",   (void*)fgetc},
        {"fgets",   (void*)fgets},
        {"ungetc",  (void*)ungetc},
        {"fgetpos", (void*)fgetpos},
        {"ftell",   (void*)ftell},
        {"fsetpos", (void*)fsetpos},
        {"fseek",   (void*)fseek},
        {"feof",    (void*)feof},
        {"ferror",  (void*)ferror}
    };

    return fnMap[name];
}
#endif


TypedValue createMallocAndStore(Compiler *c, TypedValue &val){
    string mallocFnName = "malloc";
    Function* mallocFn = (Function*)c->getFunction(mallocFnName, mallocFnName).val;

    unsigned size = val.type->getSizeInBits(c) / 8;

    Value *sizeVal = ConstantInt::get(*c->ctxt, APInt(AN_USZ_SIZE, size, true));

    Value *voidPtr = c->builder.CreateCall(mallocFn, sizeVal);
    Type *ptrTy = val.getType()->getPointerTo();
    Value *typedPtr = c->builder.CreatePointerCast(voidPtr, ptrTy);

    //finally store val1 into the malloc'd slot
    c->builder.CreateStore(val.val, typedPtr);

    auto *tyn = AnPtrType::get(val.type);
    return TypedValue(typedPtr, tyn);
}



vector<Value*> unwrapVoidPtrArgs(Compiler *c, Value *anteCallArg, FuncDecl *fd){
    vector<Value*> ret;

    auto *fnTy = cast<Function>(fd->tv.val)->getFunctionType();
    Type *argTupTy = StructType::get(*c->ctxt, fnTy->params(), true);
    Type *argsTy = argTupTy->getPointerTo();

    Value *cast = c->builder.CreateBitCast(anteCallArg, argsTy);

    for(size_t i = 0; i < argTupTy->getNumContainedTypes(); i++){
        vector<Value*> indices = {
            c->builder.getInt32(0),
            c->builder.getInt32(i)
        };
        Value *gep = c->builder.CreateGEP(cast, indices);
        ret.push_back(c->builder.CreateLoad(gep));
    }

    return ret;
}


/**
 * Creates a function AnteCall that unpacks the given arguments from a void*,
 * and returns the result of a call to the given FuncDecl with those arguments.
 *
 * AnteCall has the type 't*->'u* where 't is a tuple of fd's parameter types (to
 * be unpacked within AnteCall) and 'u is the return type of fd.
 */
void createDriverFunction(Compiler *c, FuncDecl *fd, vector<TypedValue> &typedArgs){
    Type *voidPtrTy = Type::getInt8Ty(*c->ctxt)->getPointerTo();
    FunctionType *fnTy = FunctionType::get(voidPtrTy, voidPtrTy, false);

    //preFn is the predecessor to fn because we do not yet know its return type, so its body must be compiled,
    //then the type must be checked and the new function with correct return type created, and their bodies swapped.
    Function *fn = Function::Create(fnTy, Function::ExternalLinkage, "AnteCall", c->module.get());
    BasicBlock *entry = BasicBlock::Create(*c->ctxt, "entry", fn);
    c->builder.SetInsertPoint(entry);

    auto *fnArg1 = fn->arg_begin();
    auto args = unwrapVoidPtrArgs(c, fnArg1, fd);

    Value *call = c->builder.CreateCall(fd->tv.val, args);
    AnType *retTy = fd->tv.type->getFunctionReturnType();
    if(retTy->typeTag == TT_Void){
        c->builder.CreateRetVoid();
    }else{
        auto callTv = TypedValue(call, fd->tv.type->getFunctionReturnType());

        auto store = createMallocAndStore(c, callTv);
        c->builder.CreateRet(store.val);
    }
}


extern map<string, unique_ptr<CtFunc>> compapi;

/*
 *  Compile a compile-time function/macro which should not return a function call, just a compile-time constant.
 *  Ex: A call to Ante.getAST() would be a meta function as it wouldn't make sense to get the parse tree
 *      during runtime
 *
 *  - Assumes arguments are already type-checked
 */
TypedValue compMetaFunctionResult(Compiler *c, LOC_TY &loc, string &baseName, string &mangledName, vector<TypedValue> &typedArgs){
    CtFunc* fn;
    if((fn = compapi[baseName].get())){
        TypedValue *res;

        //TODO organize CtFunc's by param count + type instead of a hard-coded name check
        if(baseName == "Ante_debug"){
            if(typedArgs.size() != 1)
                return c->compErr("Called function was given " + to_string(typedArgs.size()) +
                        " arguments but was declared to take 1", loc);

            res = (*fn)(c, typedArgs[0]);
        }else if(baseName == "Ante_sizeof"){
            if(typedArgs.size() != 1)
                return c->compErr("Called function was given " + to_string(typedArgs.size()) +
                        " arguments but was declared to take 1", loc);

            res = (*fn)(c, typedArgs[0]);
        }else if(baseName == "Ante_store"){
            if(typedArgs.size() != 2)
                return c->compErr("Called function was given " + to_string(typedArgs.size()) +
                        " argument(s) but was declared to take 2", loc);

            res = (*fn)(c, typedArgs[0], typedArgs[1]);
        }else if(baseName == "Ante_lookup" or baseName == "Ante_error" or baseName == "FuncDecl_getName"){
            if(typedArgs.size() != 1)
                return c->compErr("Called function was given " + to_string(typedArgs.size()) +
                        " arguments but was declared to take 1", loc);

            res = (*fn)(c, typedArgs[0]);
        }else if(baseName == "Ante_forget"){
            if(typedArgs.size() != 1)
                return c->compErr("Called function was given " + to_string(typedArgs.size()) +
                        " arguments but was declared to take 1", loc);

            res = (*fn)(c, typedArgs[0]);
        }else if(baseName == "Ante_emitIR"){
            if(typedArgs.size() != 0)
                return c->compErr("Called function was given " + to_string(typedArgs.size()) +
                        " argument(s) but was declared to take 0", loc);

            res = (*fn)(c);
        }else{
            res = (*fn)(c);
        }

        if(res){
            TypedValue ret = *res;
            delete res;
            return ret;
        }else{
            return c->getVoidLiteral();
        }
    }else{
        auto mod_compiler = wrapFnInModule(c, baseName, mangledName);
        mod_compiler->ast.release();

        if(!mod_compiler or mod_compiler->errFlag){
            c->errFlag = true;
            throw new CtError();
        }


        //jit->DisableSymbolSearching();
        //for(auto &f : mod->getFunctionList()){
        //    if(f.isDeclaration()){
        //        try{
        //            auto fAddr = lookupCFn(f.getName().str());
        //            jit->addGlobalMapping(&f, fAddr);
        //        }catch(out_of_range r){
        //            c->compErr("Cannot link to unknown external function "+f.getName().str()+ " in compile-time module", loc);
        //        }
        //    }
        //}

        auto *fd = mod_compiler->getFuncDecl(baseName, mangledName);
        createDriverFunction(mod_compiler.get(), fd, typedArgs);

        JIT* jit = new JIT();
        jit->addModule(move(mod_compiler->module));

        auto fn = (void*(*)(void*))jit->getSymbolAddress("AnteCall");
        if(fn){
            auto arg = ArgTuple(c, typedArgs);

            auto res = fn(arg.asRawData());
            auto *retTy = fd->tv.type->getFunctionReturnType();
            return ArgTuple(c, res, retTy).asTypedValue();
        }else{
            cerr << "(null)" << endl;
            return c->getVoidLiteral();
        }
    }
}


bool isInvalidParamType(Type *t){
    return t->isArrayTy();
}

//Computes the address of operator &
TypedValue addrOf(Compiler *c, TypedValue &tv){
    auto *ptrTy = AnPtrType::get(tv.type);

    if(LoadInst* li = dyn_cast<LoadInst>(tv.val)){
        return TypedValue(li->getPointerOperand(), ptrTy);
    }else{
        //if it is not stack-allocated already, allocate it on the stack
        auto *alloca = c->builder.CreateAlloca(tv.getType());
        c->builder.CreateStore(tv.val, alloca);
        return TypedValue(alloca, ptrTy);
    }
}


TypedValue tryImplicitCast(Compiler *c, TypedValue &arg, AnType *castTy){
    if(isNumericTypeTag(arg.type->typeTag) and isNumericTypeTag(castTy->typeTag)){
        auto widen = c->implicitlyWidenNum(arg, castTy->typeTag);
        if(widen.val != arg.val){
            return widen;
        }
    }

    //check for an implicit Cast function
    TypedValue fn;

    if(!!(fn = c->getCastFn(arg.type, castTy))){
        AnFunctionType *fty = (AnFunctionType*)fn.type;
        if(!!c->typeEq({arg.type}, fty->extTys)){

            //optimize case of Str -> c8* implicit cast
            if(fn.val->getName() == "c8*_init_Str"){
                Value *str = arg.val;
                if(str->getType()->isPointerTy())
                    str = c->builder.CreateLoad(str);

                return TypedValue(c->builder.CreateExtractValue(str, 0),
                       AnPtrType::get(AnType::getPrimitive(TT_C8)));
            }else{
                return TypedValue(c->builder.CreateCall(fn.val, arg.val),
                       fn.type->getFunctionReturnType());
            }
        }
    }
    return {};
}


TypedValue deduceFunction(Compiler *c, FunctionCandidates *fc, vector<TypedValue> &args, LOC_TY &loc){
    if(!!fc->obj) push_front(args, fc->obj);

    auto argTys = toAnTypeVector(args);

    auto matches = filterBestMatches(c, fc->candidates, argTys);

    if(matches.size() == 1){
        return compFnWithArgs(c, matches[0].second, argTys);

    }else if(matches.empty()){
        try {
            lazy_printer msg = "No matching candidates for call to "+fc->candidates[0]->getName();
            if(!argTys.empty())
                msg = msg + " with args " + anTypeToColoredStr(AnAggregateType::get(TT_Tuple, argTys));

            c->compErr(msg, loc);
        }catch(CtError *e){
            for(auto &fd : fc->candidates){
                auto *fnty = fd->type ? fd->type
                    : AnFunctionType::get(c, AnType::getVoid(), fd->fdn->params.get());
                auto *params = AnAggregateType::get(TT_Tuple, fnty->extTys);

                c->compErr("Candidate function with params "+anTypeToColoredStr(params), fd->fdn->loc, ErrorType::Note);
            }
            throw e;
        }
    }else{
        try {
            lazy_printer msg = "Multiple equally-matching candidates found for call to "+fc->candidates[0]->getName();
            if(!argTys.empty())
                msg = msg + " with args " + anTypeToColoredStr(AnAggregateType::get(TT_Tuple, argTys));

            c->compErr(msg, loc);
        }catch(CtError *e){
            for(auto &p : matches){
                auto *fnty = p.second->type ? p.second->type
                    : AnFunctionType::get(c, AnType::getVoid(), p.second->fdn->params.get());
                auto *params = AnAggregateType::get(TT_Tuple, fnty->extTys);

                c->compErr("Candidate function with params "+anTypeToColoredStr(params), p.second->fdn->loc, ErrorType::Note);
            }
            throw e;
        }
    }
    return {};
}


TypedValue searchForFunction(Compiler *c, Node *l, vector<TypedValue> &typedArgs){
    if(VarNode *vn = dynamic_cast<VarNode*>(l)){
        //Check if there is a var in local scope first
        auto *var = c->lookup(vn->name);
        if(var){
            return var->autoDeref ?
                TypedValue(c->builder.CreateLoad(var->getVal(), vn->name), var->tval.type):
                TypedValue(var->tval.val, var->tval.type);
        }

        auto params = toTypeVector(typedArgs);

        //try to do module inference
        if(!typedArgs.empty()){
            string fnName = anTypeToStrWithoutModifiers(typedArgs[0].type) + "_" + vn->name;
            TypedValue tvf = c->getMangledFn(fnName, params);
            if(!!tvf) return tvf;
        }


        auto f = c->getMangledFn(vn->name, params);
        if(!!f) return f;
    }

    //if it is not a varnode/no method is found, then compile it normally
    return l->compile(c);
}


TypedValue compFnCall(Compiler *c, Node *l, Node *r){
    //used to type-check each parameter later
    vector<TypedValue> typedArgs;
    vector<Value*> args;

    //add all remaining arguments
    if(auto *tup = dynamic_cast<TupleNode*>(r)){
        typedArgs = tup->unpack(c);

        for(TypedValue v : typedArgs){
            auto arg = v;
            if(isInvalidParamType(arg.getType()))
                arg = addrOf(c, arg);

            args.push_back(arg.val);
        }
    }else{ //single parameter being applied
        auto param = r->compile(c);
        if(!param) return param;

        if(param.type->typeTag != TT_Void){
            auto arg = param;
            if(isInvalidParamType(arg.getType()))
                arg = addrOf(c, arg);

            typedArgs.push_back(arg);
            args.push_back(arg.val);
        }
    }

    //try to compile the function now that the parameters are compiled.
    TypedValue tvf = searchForFunction(c, l, typedArgs);

    //Compiling "normally" above may result in a list of functions returned due to the
    //lack of information on argument types, so handle that now
    bool is_method = false;
    if(tvf.type->typeTag == TT_FunctionList){
        auto *funcs = (FunctionCandidates*)tvf.val;
        tvf = deduceFunction(c, funcs, typedArgs, l->loc);
        if(!!funcs->obj){
            push_front(args, funcs->obj.val);
            is_method = true;
        }
        delete funcs;
    }

    if(!tvf)
        return {};

    if(tvf.type->typeTag != TT_Function && tvf.type->typeTag != TT_MetaFunction)
        return c->compErr("Called value is not a function or method, it is a(n) " +
                anTypeToColoredStr(tvf.type), l->loc);


    //now that we assured it is a function, unwrap it
    Function *f = (Function*)tvf.val;
    AnAggregateType *fty = (AnAggregateType*)tvf.type;

    size_t argc = fty->extTys.size();

    cout << "fty = ";
    fty->dump();
    cout << ", argc = " << argc << endl;

    if(argc != args.size() and (!f or !f->isVarArg())){
        //check if an empty tuple (a void value) is being applied to a zero argument function before continuing
        //if not checked, it will count it as an argument instead of the absence of any
        //NOTE: this has the possibly unwanted side effect of allowing 't->void function applications to be used
        //      as parameters for functions requiring 0 parameters, although this does not affect the behaviour of either.
        if(argc != 0 || typedArgs[0].type->typeTag != TT_Void){
            if(args.size() == 1)
                return c->compErr("Called function was given 1 argument but was declared to take "
                        + to_string(argc), r->loc);
            else
                return c->compErr("Called function was given " + to_string(args.size()) +
                        " arguments but was declared to take " + to_string(argc), r->loc);
        }
    }

    //type check each parameter
    for(size_t i = 0; i < argc; i++){
        TypedValue tArg = typedArgs[i];
        AnType *paramTy = fty->extTys[i];

        if(!paramTy) break;

        //Mutable parameters are implicitely passed by reference
        //
        //Note that by getting the address of tArg (and not args[i-1])
        //any previous implicit references (like from the passing of an array type)
        //are not applied so no implicit references to references accidentally occur
        if(paramTy->hasModifier(Tok_Mut)){
            args[i] = addrOf(c, tArg).val;
        }

        auto typecheck = c->typeEq(tArg.type, paramTy);
        if(!typecheck){
            TypedValue cast = tryImplicitCast(c, tArg, paramTy);

            if(!!cast){
                args[i] = cast.val;
                typedArgs[i] = cast;
            }else{
                TupleNode *tn = dynamic_cast<TupleNode*>(r);

                //If there is no arg tuple then this function was applied with <| or |>
                if(!tn){
                    return c->compErr("Argument " + to_string(i+1) + " of function is a(n) " + anTypeToColoredStr(tArg.type)
                        + " but was declared to be a(n) " + anTypeToColoredStr(paramTy) + " and there is no known implicit cast", r->loc);
                }

                size_t index = i - (is_method ? 1 : 0);
                Node* locNode = tn->exprs[index].get();
                if(!locNode) return {};

                return c->compErr("Argument " + to_string(i+1) + " of function is a(n) " + anTypeToColoredStr(tArg.type)
                    + " but was declared to be a(n) " + anTypeToColoredStr(paramTy) + " and there is no known implicit cast", locNode->loc);
            }

		//If the types passed type check but still dont match exactly there was probably a void* involved
		//In that case, create a bit cast to the ptr type of the parameter
        }else if(tvf.val and args[i]->getType() != tvf.getType()->getPointerElementType()->getFunctionParamType(i) and paramTy->typeTag == TT_Ptr){
			args[i] = c->builder.CreateBitCast(args[i], tvf.getType()->getPointerElementType()->getFunctionParamType(i));
		}
    }

    //if tvf is a ![macro] or similar MetaFunction, then compile it in a separate
    //module and JIT it instead of creating a call instruction
    if(tvf.type->typeTag == TT_MetaFunction){
        string baseName = getName(l);
        auto *fnty = (AnFunctionType*)tvf.type;
        string mangledName = mangle(baseName, fnty->extTys);
        return compMetaFunctionResult(c, l->loc, baseName, mangledName, typedArgs);
    }

    //use tvf->val as arg, NOT f, (if tvf->val is a function-type parameter then f cannot be called)
    //
    //both a C-style cast and dyn-cast to functions fail if f is a function-pointer
    auto *call = c->builder.CreateCall(tvf.val, args);

    return TypedValue(call, tvf.type->getFunctionReturnType());
}

TypedValue Compiler::compLogicalOr(Node *lexpr, Node *rexpr, BinOpNode *op){
    Function *f = builder.GetInsertBlock()->getParent();
    auto &blocks = f->getBasicBlockList();

    auto lhs = lexpr->compile(this);

    auto *curbbl = builder.GetInsertBlock();
    auto *orbb = BasicBlock::Create(*ctxt, "or");
    auto *mergebb = BasicBlock::Create(*ctxt, "merge");

    builder.CreateCondBr(lhs.val, mergebb, orbb);
    blocks.push_back(orbb);
    blocks.push_back(mergebb);


    builder.SetInsertPoint(orbb);
    auto rhs = rexpr->compile(this);

    //the block must be re-gotten in case the expression contains if-exprs, while nodes,
    //or other exprs that change the current block
    auto *curbbr = builder.GetInsertBlock();
    builder.CreateBr(mergebb);

    if(rhs.type->typeTag != TT_Bool)
        return compErr("The 'or' operator's rval must be of type bool, but instead is of type "+anTypeToColoredStr(rhs.type), op->rval->loc);

    builder.SetInsertPoint(mergebb);
    auto *phi = builder.CreatePHI(rhs.getType(), 2);

    //short circuit, returning true if return from the first label
    phi->addIncoming(ConstantInt::get(*ctxt, APInt(1, true, true)), curbbl);
    phi->addIncoming(rhs.val, curbbr);

    return TypedValue(phi, rhs.type);

}

TypedValue Compiler::compLogicalAnd(Node *lexpr, Node *rexpr, BinOpNode *op){
    Function *f = builder.GetInsertBlock()->getParent();
    auto &blocks = f->getBasicBlockList();

    auto lhs = lexpr->compile(this);

    auto *curbbl = builder.GetInsertBlock();
    auto *andbb = BasicBlock::Create(*ctxt, "and");
    auto *mergebb = BasicBlock::Create(*ctxt, "merge");

    builder.CreateCondBr(lhs.val, andbb, mergebb);
    blocks.push_back(andbb);
    blocks.push_back(mergebb);


    builder.SetInsertPoint(andbb);
    auto rhs = rexpr->compile(this);

    //the block must be re-gotten in case the expression contains if-exprs, while nodes,
    //or other exprs that change the current block
    auto *curbbr = builder.GetInsertBlock();
    builder.CreateBr(mergebb);

    if(rhs.type->typeTag != TT_Bool)
        return compErr("The 'and' operator's rval must be of type bool, but instead is of type "+anTypeToColoredStr(rhs.type), op->rval->loc);

    builder.SetInsertPoint(mergebb);
    auto *phi = builder.CreatePHI(rhs.getType(), 2);

    //short circuit, returning false if return from the first label
    phi->addIncoming(ConstantInt::get(*ctxt, APInt(1, false, true)), curbbl);
    phi->addIncoming(rhs.val, curbbr);

    return TypedValue(phi, rhs.type);
}


TypedValue handlePrimitiveNumericOp(BinOpNode *bop, Compiler *c, TypedValue &lhs, TypedValue &rhs){
    switch(bop->op){
        case '+': return c->compAdd(lhs, rhs, bop);
        case '-': return c->compSub(lhs, rhs, bop);
        case '*': return c->compMul(lhs, rhs, bop);
        case '/': return c->compDiv(lhs, rhs, bop);
        case '%': return c->compRem(lhs, rhs, bop);
        case '<':
                    if(isFPTypeTag(lhs.type->typeTag))
                        return TypedValue(c->builder.CreateFCmpOLT(lhs.val, rhs.val), AnType::getBool());
                    else if(isUnsignedTypeTag(lhs.type->typeTag))
                        return TypedValue(c->builder.CreateICmpULT(lhs.val, rhs.val), AnType::getBool());
                    else
                        return TypedValue(c->builder.CreateICmpSLT(lhs.val, rhs.val), AnType::getBool());
        case '>':
                    if(isFPTypeTag(lhs.type->typeTag))
                        return TypedValue(c->builder.CreateFCmpOGT(lhs.val, rhs.val), AnType::getBool());
                    else if(isUnsignedTypeTag(lhs.type->typeTag))
                        return TypedValue(c->builder.CreateICmpUGT(lhs.val, rhs.val), AnType::getBool());
                    else
                        return TypedValue(c->builder.CreateICmpSGT(lhs.val, rhs.val), AnType::getBool());
        case '^':
                    return TypedValue(c->builder.CreateXor(lhs.val, rhs.val), lhs.type);
        case Tok_Eq:
        case Tok_Is:
                    if(isFPTypeTag(lhs.type->typeTag))
                        return TypedValue(c->builder.CreateFCmpOEQ(lhs.val, rhs.val), AnType::getBool());
                    else
                        return TypedValue(c->builder.CreateICmpEQ(lhs.val, rhs.val), AnType::getBool());
        case Tok_NotEq:
                    if(isFPTypeTag(lhs.type->typeTag))
                        return TypedValue(c->builder.CreateFCmpONE(lhs.val, rhs.val), AnType::getBool());
                    else
                        return TypedValue(c->builder.CreateICmpNE(lhs.val, rhs.val), AnType::getBool());
        case Tok_LesrEq:
                    if(isFPTypeTag(lhs.type->typeTag))
                        return TypedValue(c->builder.CreateFCmpOLE(lhs.val, rhs.val), AnType::getBool());
                    else if(isUnsignedTypeTag(lhs.type->typeTag))
                        return TypedValue(c->builder.CreateICmpULE(lhs.val, rhs.val), AnType::getBool());
                    else
                        return TypedValue(c->builder.CreateICmpSLE(lhs.val, rhs.val), AnType::getBool());
        case Tok_GrtrEq:
                    if(isFPTypeTag(lhs.type->typeTag))
                        return TypedValue(c->builder.CreateFCmpOGE(lhs.val, rhs.val), AnType::getBool());
                    else if(isUnsignedTypeTag(lhs.type->typeTag))
                        return TypedValue(c->builder.CreateICmpUGE(lhs.val, rhs.val), AnType::getBool());
                    else
                        return TypedValue(c->builder.CreateICmpSGE(lhs.val, rhs.val), AnType::getBool());
        default:
            return c->compErr("Operator " + Lexer::getTokStr(bop->op) + " is not overloaded for types "
                   + anTypeToColoredStr(lhs.type) + " and " + anTypeToColoredStr(rhs.type), bop->loc);
    }
}

/*
 *  Checks the type of a value (usually a function argument) against a type
 *  and attempts to look for and use an implicit conversion if one is found.
 */
TypedValue typeCheckWithImplicitCasts(Compiler *c, TypedValue &arg, AnType *ty){
    auto tc = c->typeEq(arg.type, ty);
    if(!!tc) return arg;

    return tryImplicitCast(c, arg, ty);
}


TypedValue checkForOperatorOverload(Compiler *c, TypedValue &lhs, int op, TypedValue rhs){
    string basefn = Lexer::getTokStr(op);
    string mangledfn = mangle(basefn, {lhs.type, rhs.type});

    //now look for the function
    vector<AnType*> argtys = {lhs.type, rhs.type};
    auto fn = c->getMangledFn(basefn, argtys);
    if(!fn) return fn;

    auto *fnty = (AnFunctionType*)fn.type;

    AnType *param1 = fnty->extTys[0];
    AnType *param2 = fnty->extTys[1];

    lhs = typeCheckWithImplicitCasts(c, lhs, param1);
    rhs = typeCheckWithImplicitCasts(c, rhs, param2);

    if(implicitPassByRef(param1)) lhs = addrOf(c, lhs);
    if(implicitPassByRef(param2)) rhs = addrOf(c, rhs);

    vector<Value*> argVals = {lhs.val, rhs.val};
    return TypedValue(c->builder.CreateCall(fn.val, argVals), fnty->getFunctionReturnType());
}


TypedValue SeqNode::compile(Compiler *c){
    TypedValue ret;
    size_t i = 1;

    for(auto &n : sequence){
        try{
            ret = n->compile(c);
            if(dynamic_cast<FuncDeclNode*>(n.get()))
                n.release();
        }catch(CtError *e){
            //Unless the final value throws, delete the error
            if(i == sequence.size()) throw e;
            else delete e;
        }
        i++;
    }

    return ret;
}


/*
 *  Compiles an operation along with its lhs and rhs
 */
TypedValue BinOpNode::compile(Compiler *c){
    switch(op){
        case '.': return c->compMemberAccess(lval.get(), (VarNode*)rval.get(), this);
        case '(': return compFnCall(c, lval.get(), rval.get());
        case Tok_And: return c->compLogicalAnd(lval.get(), rval.get(), this);
        case Tok_Or: return c->compLogicalOr(lval.get(), rval.get(), this);
    }

    TypedValue lhs = lval->compile(c);
    TypedValue rhs = rval->compile(c);

    TypedValue res;
    if(!!(res = checkForOperatorOverload(c, lhs, op, rhs))){
        return res;
    }

    if(op == '#') return c->compExtract(lhs, rhs, this);


    //Check if both Values are numeric, and if so, check if their types match.
    //If not, do an implicit conversion (usually a widening) to match them.
    c->handleImplicitConversion(&lhs, &rhs);


    //first, if both operands are primitive numeric types, use the default ops
    if(isNumericTypeTag(lhs.type->typeTag) && isNumericTypeTag(rhs.type->typeTag)){
        return handlePrimitiveNumericOp(this, c, lhs, rhs);

    //and bools/ptrs are only compatible with == and !=
    }else if((lhs.type->typeTag == TT_Bool and rhs.type->typeTag == TT_Bool) or
             (lhs.type->typeTag == TT_Ptr  and rhs.type->typeTag == TT_Ptr)){

        //== is no longer implemented for pointers by default
        if(op == Tok_Eq and lhs.type->typeTag == TT_Bool and rhs.type->typeTag == TT_Bool)
            return TypedValue(c->builder.CreateICmpEQ(lhs.val, rhs.val), AnType::getBool());

        switch(op){
            case Tok_Is:    return TypedValue(c->builder.CreateICmpEQ(lhs.val, rhs.val), AnType::getBool());
            case Tok_NotEq: return TypedValue(c->builder.CreateICmpNE(lhs.val, rhs.val), AnType::getBool());
        }
    }

    if(op == '+' or op == '-'){
        if((lhs.type->typeTag == TT_Ptr or isNumericTypeTag(lhs.type->typeTag)) and
           (rhs.type->typeTag == TT_Ptr or isNumericTypeTag(rhs.type->typeTag)))
            return handlePrimitiveNumericOp(this, c, lhs, rhs);
    }

    return c->compErr("Operator " + Lexer::getTokStr(op) + " is not overloaded for types "
            + anTypeToColoredStr(lhs.type) + " and " + anTypeToColoredStr(rhs.type), loc);
}


TypedValue UnOpNode::compile(Compiler *c){
    TypedValue rhs = rval->compile(c);

    switch(op){
        case '@': //pointer dereference
            if(rhs.type->typeTag != TT_Ptr){
                return c->compErr("Cannot dereference non-pointer type " + anTypeToColoredStr(rhs.type), loc);
            }

            return TypedValue(c->builder.CreateLoad(rhs.val), ((AnPtrType*)rhs.type)->extTy);
        case '&': //address-of
            return addrOf(c, rhs);
        case '-': //negation
            return TypedValue(c->builder.CreateNeg(rhs.val), rhs.type);
        case Tok_Not:
            if(rhs.type->typeTag != TT_Bool)
                return c->compErr("Unary not operator not overloaded for type " + anTypeToColoredStr(rhs.type), loc);

            return TypedValue(c->builder.CreateNot(rhs.val), rhs.type);
        case Tok_New:
            //the 'new' keyword in ante creates a reference to any existing value
            return createMallocAndStore(c, rhs);
    }

    return c->compErr("Unknown unary operator " + Lexer::getTokStr(op), loc);
}

} // end of namespace ante
