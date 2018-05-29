#include <llvm/ExecutionEngine/Interpreter.h>
#include <llvm/Transforms/Utils/Cloning.h>
#include "compiler.h"
#include "types.h"
#include "function.h"
#include "tokens.h"
#include "jitlinker.h"
#include "types.h"
#include "jit.h"
#include "argtuple.h"
#include "compapi.h"

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

    if(auto *arrty = try_cast<AnArrayType>(l.type)){
        //check for alloca
        Value *arr = dyn_cast<LoadInst>(l.val) ?
                cast<LoadInst>(l.val)->getPointerOperand() :
                addrOf(this, l).val;

        vector<Value*> indices;
        indices.push_back(ConstantInt::get(*ctxt, APInt(64, 0, true)));
        indices.push_back(r.val);
        return TypedValue(builder.CreateLoad(builder.CreateGEP(arr, indices)), arrty->extTy);

    }else if(auto *ptrty = try_cast<AnPtrType>(l.type)){
        return TypedValue(builder.CreateLoad(builder.CreateGEP(l.val, r.val)), ptrty->extTy);

    }else if(l.type->typeTag == TT_Tuple || l.type->typeTag == TT_Data){
		auto indexval = dyn_cast<ConstantInt>(r.val);
        if(!indexval)
            return compErr("Tuple indices must always be known at compile time.", op->loc);

        auto index = indexval->getZExtValue();

        auto *aggty = try_cast<AnAggregateType>(l.type);

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
    auto tmp = CompilingVisitor::compile(this, op->lval);

    //if(!dynamic_cast<LoadInst*>(tmp->val))
    if(!tmp.type->hasModifier(Tok_Mut))
        return compErr("Variable must be mutable to insert values, but instead is an immutable " +
                anTypeToColoredStr(tmp.type), op->lval->loc);

    Value *var = static_cast<LoadInst*>(tmp.val)->getPointerOperand();

    auto index =  CompilingVisitor::compile(this, op->rval);
    auto newVal = CompilingVisitor::compile(this, assignExpr);

    //see if insert operator # = is overloaded already
    string basefn = "#";
    vector<AnType*> args = {tmp.type, AnType::getI32(), newVal.type};
    auto fn = getMangledFn(basefn, args);
    if(fn){
        vector<Value*> args = {var, index.val, newVal.val};
        auto *retty = try_cast<AnAggregateType>(fn.type)->extTys[0];
        auto *call = builder.CreateCall(fn.val, args);
        return TypedValue(call, retty);
    }

    switch(tmp.type->typeTag){
        case TT_Array: {
            auto *arrty = try_cast<AnArrayType>(tmp.type);
            if(!typeEq(arrty->extTy, newVal.type))
                return compErr("Cannot create store of types: "+anTypeToColoredStr(tmp.type)+" <- "
                        +anTypeToColoredStr(newVal.type), assignExpr->loc);

            Value *cast = builder.CreateBitCast(var, var->getType()->getPointerElementType()->getArrayElementType()->getPointerTo());
            Value *dest = builder.CreateInBoundsGEP(cast, index.val);
            builder.CreateStore(newVal.val, dest);
            return getVoidLiteral();
        }
        case TT_Ptr: {
            auto *ptrty = try_cast<AnPtrType>(tmp.type);
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
                auto *aggty = try_cast<AnAggregateType>(tmp.type);

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
        unionDataTy = try_cast<AnDataType>(bindGenericToType(c, unionDataTy, tyeq->bindings));
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
    if(auto *dt = try_cast<AnDataType>(t)){
        return dt->name + "_init";
    }
    return anTypeToStr(t) + "_init";
}

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
        return try_cast<AnAggregateType>(ty)->extTys;
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
    auto *dataTy = try_cast<AnDataType>(castTy);

    if(dataTy){
        auto argTup = toArgTuple(valToCast.type);
        auto tc = c->typeEq(dataTy->extTys, argTup);

        if(tc){
            if(dataTy->isUnionTag())
                return {ReinterpretCastResult::ValToUnion, tc, dataTy};
            else
                return {ReinterpretCastResult::ValToStruct, tc, dataTy};
        }
    }

    if(auto *valDt = try_cast<AnDataType>(valToCast.type)){
        auto argTup = toArgTuple(castTy);

        auto tc = c->typeEq(valDt->extTys, argTup);
        if(tc){
            return {ReinterpretCastResult::ValToPrimitive, tc, dataTy};
        }
    }

    return {ReinterpretCastResult::NoCast, {}, nullptr};
}


/**
 *  Reinterpret a value as a tuple value when casting to a tuple type.
 *
 *  This function handles instances when a casted type is equal to the
 *  casting type's rhs in its definition.
 *
 *  For example, given the definition type T = U and a variable u: U
 *  the cast T u will be managed by this function with from = u and to = T
 */
TypedValue reinterpretTuple(Compiler *c, Value *from, AnType *to){
    auto *structTy = c->anTypeToLlvmType(to);
    Value *rstruct = UndefValue::get(structTy);

    if(structTy->getStructElementType(0) == from->getType()){
        rstruct = c->builder.CreateInsertValue(rstruct, from, 0);
        return TypedValue(rstruct, to);
    }

    auto nElems = rstruct->getType()->getStructNumElements();
    for(size_t i = 0; i < nElems; i++){
        auto *elem = c->builder.CreateExtractValue(from, i);
        rstruct = c->builder.CreateInsertValue(rstruct, elem, i);
    }

    return TypedValue(rstruct, to);
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
        if(AnDataType *unbound = try_cast<AnDataType>(castTy)->unboundType)
            tag = try_cast<AnDataType>(unbound)->name;
        else
            tag = try_cast<AnDataType>(castTy)->name;
        //to_tyn->typeName = castTy->typeName;
        //to_tyn->type = isUnion ? TT_TaggedUnion : TT_Data;

        if(rcr.typeCheck->res == TypeCheckResult::SuccessWithTypeVars){
            to_tyn = try_cast<AnDataType>(bindGenericToType(c, to_tyn, rcr.typeCheck->bindings));
        }

        if(isUnion) return createUnionVariantCast(c, valToCast, tag, rcr.dataTy, rcr.typeCheck);
        else return reinterpretTuple(c, valToCast.val, to_tyn);
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
        if(fn){
            if(isCompileTimeFunction(fn)){
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
                        !isUnsignedTypeTag(valToCast.type->typeTag)), castTy);

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

void CompilingVisitor::visit(TypeCastNode *n){
    n->rval->accept(*this);
    auto rtval = this->val;

    auto *ty = toAnType(c, n->typeExpr.get());

    this->val = createCast(c, ty, rtval, n->loc);

    if(!val){
        c->compErr("Invalid type cast " + anTypeToColoredStr(rtval.type) +
                " -> " + anTypeToColoredStr(ty), n->loc);
    }
}

TypedValue compIf(Compiler *c, IfNode *ifn, BasicBlock *mergebb, vector<pair<TypedValue,BasicBlock*>> &branches){
    auto cond = CompilingVisitor::compile(c, ifn->condition);

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
            auto thenVal = CompilingVisitor::compile(c, ifn->thenN);

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
    auto thenVal = CompilingVisitor::compile(c, ifn->thenN);
    if(!thenVal) return thenVal;
    auto *thenretbb = c->builder.GetInsertBlock(); //bb containing final ret of then branch.


    if(!dyn_cast<ReturnInst>(thenVal.val) and !dyn_cast<BranchInst>(thenVal.val))
        c->builder.CreateBr(mergebb);

    if(ifn->elseN){
        //save the final 'then' value for the upcoming PhiNode
        branches.push_back({thenVal, thenretbb});

        c->builder.SetInsertPoint(elsebb);
        auto elseVal = CompilingVisitor::compile(c, ifn->elseN);
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

void CompilingVisitor::visit(IfNode *n){
    auto branches = vector<pair<TypedValue,BasicBlock*>>();
    auto *mergebb = BasicBlock::Create(*c->ctxt, "endif");
    this->val = compIf(c, n, mergebb, branches);
}

string toModuleName(const AnType *t){
    if(auto *dt = try_cast<AnDataType>(t)){
        return dt->name;
    }else if(t->isModifierType()){
        return toModuleName(static_cast<const AnModifier*>(t)->extTy);
    }else{
        return anTypeToStr(t);
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
            auto l = CompilingVisitor::compile(this, ln);
            if(!l) return {};

            val = l.val;
            tyn = ltyn = l.type;
        }

        //the . operator automatically dereferences pointers, so update val and tyn accordingly.
        while(tyn->typeTag == TT_Ptr){
            val = builder.CreateLoad(val);
            tyn = try_cast<AnPtrType>(tyn)->extTy;
        }

        //check to see if this is a field index
        if(auto *dataTy = try_cast<AnDataType>(tyn)){
            auto index = dataTy->getFieldIndex(field->name);

            if(index != -1){
                AnType *retTy = dataTy->extTys[index];

                if(dataTy->isStub()){
                    updateLlvmTypeBinding(this, dataTy, false);
                }

                retTy = (AnType*)tyn->addModifiersTo(retTy);

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
    auto cpy = vecOf<T>(vec.size() + 1);
    cpy.push_back(val);

    for(auto &v : vec)
        cpy.push_back(v);

    vec = cpy;
}


vector<AnType*> toAnTypeVector(vector<TypedValue> &tvs){
    auto ret = vecOf<AnType*>(tvs.size());
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

    auto size_result = val.type->getSizeInBits(c);
    if(!size_result){
        cerr << size_result.getErr() << endl;
        size_result = 0;
    }
    auto size = size_result.getVal() / 8;

    Value *sizeVal = ConstantInt::get(*c->ctxt, APInt(AN_USZ_SIZE, size, true));

    Value *voidPtr = c->builder.CreateCall(mallocFn, sizeVal);
    Type *ptrTy = val.getType()->getPointerTo();
    Value *typedPtr = c->builder.CreatePointerCast(voidPtr, ptrTy);

    //finally store val1 into the malloc'd slot
    c->builder.CreateStore(val.val, typedPtr);

    auto *tyn = AnPtrType::get(val.type);
    return TypedValue(typedPtr, tyn);
}


/*
 * Unwrap the single i8* argument given to AnteCall into a vector of each value the
 * function it should call requires.
 */
vector<Value*> unwrapVoidPtrArgs(Compiler *c, Value *anteCallArg, vector<TypedValue> const& typedArgs, FuncDecl *fd){
    vector<Value*> ret;
    bool varargs = cast<Function>(fd->tv.val)->isVarArg();

    auto *fnTy = cast<Function>(fd->tv.val)->getFunctionType();
    if(fnTy->getNumParams() == 0 and !varargs) return ret;

    size_t argc = fnTy->getNumParams();
    for(size_t i = 0; i < argc or (varargs and i < typedArgs.size()); i++){
        llvm::Type *castTy = varargs ?
            typedArgs[i].getType()->getPointerTo() :
            fnTy->getParamType(i)->getPointerTo();

        Value *cast = c->builder.CreateBitCast(anteCallArg, castTy);
        ret.push_back(c->builder.CreateLoad(cast));

        if(i != argc - 1 or (varargs and i != typedArgs.size() - 1))
            anteCallArg = c->builder.CreateInBoundsGEP(cast, c->builder.getInt64(1));
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
void createDriverFunction(Compiler *c, FuncDecl *fd, vector<TypedValue> const& typedArgs){
    Type *voidPtrTy = Type::getInt8Ty(*c->ctxt)->getPointerTo();
    FunctionType *fnTy = FunctionType::get(voidPtrTy, voidPtrTy, false);

    //preFn is the predecessor to fn because we do not yet know its return type, so its body must be compiled,
    //then the type must be checked and the new function with correct return type created, and their bodies swapped.
    Function *fn = Function::Create(fnTy, Function::ExternalLinkage, "AnteCall", c->module.get());
    BasicBlock *entry = BasicBlock::Create(*c->ctxt, "entry", fn);
    c->builder.SetInsertPoint(entry);

    auto *fnArg1 = fn->arg_begin();
    auto args = unwrapVoidPtrArgs(c, fnArg1, typedArgs, fd);

    Value *call = c->builder.CreateCall(fd->tv.val, args);
    AnType *retTy = fd->tv.type->getFunctionReturnType();
    if(retTy->typeTag == TT_Void){
        c->builder.CreateRetVoid();
    }else{
        auto callTv = TypedValue(call, fd->tv.type->getFunctionReturnType());

        auto store = createMallocAndStore(c, callTv);
        auto ret = c->builder.CreateBitCast(store.val, voidPtrTy);
        c->builder.CreateRet(ret);
    }
}

void p(llvm::Module *m){
    m->print(dbgs(), nullptr);
    puts("");
}

void p(unique_ptr<llvm::Module> &m){
    p(m.get());
    puts("");
}

void p(Value *v){
    v->print(dbgs());
    puts("");
}

void p(Type *t){
    t->print(dbgs());
    puts("");
}

TypedValue compileAndCallAnteFunction(Compiler *c, string const& baseName,
        string const& mangledName, vector<TypedValue> const& typedArgs){

    auto mod_compiler = wrapFnInModule(c, baseName, mangledName, typedArgs);
    mod_compiler->ast.release();

    if(!mod_compiler or mod_compiler->errFlag){
        c->errFlag = true;
        cerr << "Error encountered while JITing " << baseName << ", aborting.\n";
        throw new CtError();
    }

    auto *fd = mod_compiler->getFuncDecl(baseName, mangledName);

    //error here!
    if(!fd) fd = mod_compiler->getFuncDecl(baseName, baseName);

    if(!fd){
        c->errFlag = true;
        cerr << "Error encountered while getting JITed FuncDecl of " << baseName << ", aborting.\n";
        throw new CtError();
    }

    createDriverFunction(mod_compiler.get(), fd, typedArgs);
    std::error_code ec;

    JIT* jit = new JIT();
    jit->addModule(move(mod_compiler->module));

    auto fn = (void*(*)(void*))jit->getSymbolAddress("AnteCall");
    if(fn){
        auto arg = ArgTuple(c, typedArgs);

        auto res = fn(arg.asRawData());
        auto *retTy = fd->tv.type->getFunctionReturnType();
        return ArgTuple(res, retTy).asTypedValue(c);
    }else{
        cerr << "(null)" << endl;
        return c->getVoidLiteral();
    }
}

/*
 *  Compile a compile-time function/macro which should not return a function call,
 *  just a compile-time constant.
 *
 *  Ex: A call to Ante.getAST() would be a meta function as it wouldn't make sense
 *      to get the parse tree during runtime.
 *
 *  - Assumes arguments are already type-checked
 */
TypedValue compMetaFunctionResult(Compiler *c, LOC_TY const& loc, string const& baseName,
        string const& mangledName, vector<TypedValue> const& ta){

    capi::CtFunc* fn = capi::lookup(baseName);

    //fn not found, this is a user-defined ante function
    if(!fn)
        return compileAndCallAnteFunction(c, baseName, mangledName, ta);

    if(ta.size() != fn->params.size())
        return c->compErr("Called function was given " + to_string(ta.size()) +
                " argument(s) but was declared to take " + to_string(fn->params.size()), loc);

    TypedValue *res;
    switch(fn->params.size()){
        case 0: res = (*fn)(c); break;
        case 1: res = (*fn)(c, ArgTuple(c, ta[0])); break;
        case 2: res = (*fn)(c, ArgTuple(c, ta[0]), ArgTuple(c, ta[1])); break;
        case 3: res = (*fn)(c, ArgTuple(c, ta[0]), ArgTuple(c, ta[1]), ArgTuple(c, ta[2])); break;
        case 4: res = (*fn)(c, ArgTuple(c, ta[0]), ArgTuple(c, ta[1]), ArgTuple(c, ta[2]), ArgTuple(c, ta[3])); break;
        case 5: res = (*fn)(c, ArgTuple(c, ta[0]), ArgTuple(c, ta[1]), ArgTuple(c, ta[2]), ArgTuple(c, ta[3]), ArgTuple(c, ta[4])); break;
        case 6: res = (*fn)(c, ArgTuple(c, ta[0]), ArgTuple(c, ta[1]), ArgTuple(c, ta[2]), ArgTuple(c, ta[3]), ArgTuple(c, ta[4]), ArgTuple(c, ta[5])); break;
        default:
            cerr << "CtFuncs with more than 6 parameters are unimplemented." << endl;
            return {};
    }

    if(res){
        TypedValue ret = *res;
        delete res;
        return ret;
    }else{
        return c->getVoidLiteral();
    }
}


bool isInvalidParamType(Type *t){
    return t->isArrayTy();
}


//Computes the address of operator &
//
//Returns a TypedValue that is a reference to the given tv.
//If the given tv is not mutable and does not have an existing
//reference one is created on the stack.
TypedValue addrOf(Compiler *c, TypedValue &tv){
    auto *ptrTy = AnPtrType::get(tv.type);

    if(LoadInst* li = dyn_cast<LoadInst>(tv.val)){
        return TypedValue(li->getPointerOperand(), ptrTy);
    }else if(ExtractValueInst *evi = dyn_cast<ExtractValueInst>(tv.val)){
        Value *agg = evi->getAggregateOperand();
        size_t index = evi->getIndices()[0];
        if(LoadInst *li = dyn_cast<LoadInst>(agg)){
            return TypedValue(c->builder.CreateStructGEP(agg->getType(),
                        li->getPointerOperand(), index), ptrTy);
        }
    }
    //if it is not stack-allocated already, allocate it on the stack
    auto *alloca = c->builder.CreateAlloca(tv.getType());
    c->builder.CreateStore(tv.val, alloca);
    return TypedValue(alloca, ptrTy);
}


TypedValue tryImplicitCast(Compiler *c, TypedValue &arg, AnType *castTy){
    if(isNumericTypeTag(arg.type->typeTag) and isNumericTypeTag(castTy->typeTag)){
        auto widen = c->implicitlyWidenNum(arg, castTy->typeTag);
        if(widen.val != arg.val){
            return widen;
        }
    }

    //check for an implicit Cast function
    if(TypedValue fn = c->getCastFn(arg.type, castTy)){
        AnFunctionType *fty = try_cast<AnFunctionType>(fn.type);
        if(c->typeEq({arg.type}, fty->extTys)){
            vector<Value*> args{arg.val};
            auto *call = c->builder.CreateCall(fn.val, args);
            return TypedValue(call, fn.type->getFunctionReturnType());
        }
    }
    return {};
}


void showNoMatchingCandidateError(Compiler *c, vector<shared_ptr<FuncDecl>> &candidates,
        vector<AnType*> &argTys, LOC_TY &loc){

    try {
        lazy_printer msg = "No matching candidates for call to " + candidates[0]->getName();
        if(!argTys.empty())
            msg = msg + " with args " + anTypeToColoredStr(AnAggregateType::get(TT_Tuple, argTys));

        c->compErr(msg, loc);
    }catch(CtError *e){
        for(auto &fd : candidates){
            auto *fnty = fd->type ? fd->type
                : AnFunctionType::get(c, AnType::getVoid(), fd->fdn->params.get());
            auto *params = AnAggregateType::get(TT_Tuple, fnty->extTys);

            c->compErr("Candidate function with params "+anTypeToColoredStr(params),
                    fd->fdn->loc, ErrorType::Note);
        }
        throw e;
    }
}


void showMultipleEquallyMatchingCandidatesError(Compiler *c, vector<shared_ptr<FuncDecl>> &candidates,
        vector<AnType*> argTys, FunctionListTCResults &matches, LOC_TY &loc){

    try {
        lazy_printer msg = "Multiple equally-matching candidates found for call to " + candidates[0]->getName();
        if(!argTys.empty())
            msg = msg + " with args " + anTypeToColoredStr(AnAggregateType::get(TT_Tuple, argTys));

        c->compErr(msg, loc);
    }catch(CtError *e){
        for(auto &p : matches){
            auto *fnty = p.second->type ? p.second->type
                : AnFunctionType::get(c, AnType::getVoid(), p.second->fdn->params.get());
            auto *params = AnAggregateType::get(TT_Tuple, fnty->extTys);

            c->compErr("Candidate function with params "+anTypeToColoredStr(params),
                    p.second->fdn->loc, ErrorType::Note);
        }
        throw e;
    }
}


TypedValue deduceFunction(Compiler *c, FunctionCandidates *fc, vector<TypedValue> &args, LOC_TY &loc){
    if(!!fc->obj) push_front(args, fc->obj);

    auto argTys = toAnTypeVector(args);

    if(fc->candidates.size() == 1){
        auto fnty = AnFunctionType::get(c, AnType::getVoid(), fc->candidates[0]->fdn->params.get());
        if(fnty->isGeneric){
            return compFnWithArgs(c, fc->candidates[0].get(), argTys);
        }else{
            return c->compFn(fc->candidates[0].get());
        }
    }

    auto matches = filterBestMatches(c, fc->candidates, argTys);

    if(matches.size() == 1){
        return compFnWithArgs(c, matches[0].second, argTys);

    }else if(matches.empty()){
        showNoMatchingCandidateError(c, fc->candidates, argTys, loc);
    }else{
        showMultipleEquallyMatchingCandidatesError(c, fc->candidates, argTys, matches, loc);
    }
    return {};
}


Value* Compiler::tupleOf(vector<Value*> const& elems, bool packed){
    vector<int> nonConstIndices;
    auto constVals = vecOf<Constant*>(elems.size());

    for(size_t i = 0; i < elems.size(); i++){
        if(Constant *con = dyn_cast<Constant>(elems[i])){
            constVals.push_back(con);
        }else{
            constVals.push_back(UndefValue::get(elems[i]->getType()));
            nonConstIndices.push_back(i);
        }
    }

    Value* tuple = ConstantStruct::getAnon(constVals, packed);

    for(int i : nonConstIndices){
        tuple = builder.CreateInsertValue(tuple, elems[i], i);
    }
    return tuple;
}


Value* Compiler::ptrTo(void* val){
    auto *cint = builder.getIntN(AN_USZ_SIZE, (size_t)val);
    Type *ptrTy = Type::getInt8Ty(*ctxt)->getPointerTo();
    return builder.CreateIntToPtr(cint, ptrTy);
}


vector<Value*> adaptArgsToCompilerAPIFn(Compiler *c, vector<Value*> &args, vector<TypedValue> &typedArgs){
    auto ret = vecOf<Value*>(args.size() + 1);

    //Compiler API functions take an implicit Compiler* parameter
    Value *cArg = c->ptrTo(c);
    ret.push_back(cArg);

    int i = 0;
    for(auto *val : args){
        auto valref = c->builder.CreateAlloca(val->getType());
        c->builder.CreateStore(val, valref);
        auto valTy = c->ptrTo(typedArgs[i++].type);

        auto arg = c->tupleOf({valref, valTy}, true);
        auto argref = c->builder.CreateAlloca(arg->getType());
        c->builder.CreateStore(arg, argref);
        ret.push_back(argref);
    }
    return ret;
}


TypedValue searchForFunction(Compiler *c, Node *l, vector<TypedValue> const& typedArgs){
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
            string fnName = toModuleName(typedArgs[0].type) + "_" + vn->name;

            TypedValue tvf = c->getMangledFn(fnName, params);
            if(tvf) return tvf;
        }


        auto f = c->getMangledFn(vn->name, params);
        if(f) return f;
    }

    //if it is not a varnode/no method is found, then compile it normally
    return CompilingVisitor::compile(c, l);
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
        auto param = CompilingVisitor::compile(c, r);
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

    if(!tvf){
        c->errFlag = true;
        return {};
    }

    if(!try_cast<AnFunctionType>(tvf.type))
        return c->compErr("Called value is not a function or method, it is a(n) " +
                anTypeToColoredStr(tvf.type), l->loc);


    //now that we assured it is a function, unwrap it
    Function *f = (Function*)tvf.val;
    AnAggregateType *fty = try_cast<AnAggregateType>(tvf.type);

    size_t argc = fty->extTys.size();
    if(argc != args.size() and (!f or !f->isVarArg())){
        //check if an empty tuple (a void value) is being applied to a zero argument function before
        //continuing if not checked, it will count it as an argument instead of the absence of any
        //NOTE: this has the possibly unwanted side effect of allowing 't->void function applications
        //      to be used as parameters for functions requiring 0 parameters, although this does
        //      not affect the behaviour of the call.
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

            if(cast){
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
                if(!locNode){
                    c->errFlag = true;
                    return {};
                }

                return c->compErr("Argument " + to_string(i+1) + " of function is a(n) " + anTypeToColoredStr(tArg.type)
                    + " but was declared to be a(n) " + anTypeToColoredStr(paramTy) + " and there is no known implicit cast", locNode->loc);
            }

		//If the types passed type check but still dont match exactly there was probably a void* involved
		//In that case, create a bit cast to the ptr type of the parameter
        }else if(tvf.val and args[i]->getType() != tvf.getType()->getPointerElementType()->getFunctionParamType(i) and paramTy->typeTag == TT_Ptr){
			args[i] = c->builder.CreateBitCast(args[i], tvf.getType()->getPointerElementType()->getFunctionParamType(i));
		}
    }

    //if tvf is a ante function or similar MetaFunction, then compile it in a separate
    //module and JIT it instead of creating a call instruction
    if(isCompileTimeFunction(tvf)){
        if(c->isJIT and tvf.type->typeTag == TT_MetaFunction){
            args = adaptArgsToCompilerAPIFn(c, args, typedArgs);
        }else{
            string baseName = getName(l);
            auto *fnty = try_cast<AnFunctionType>(tvf.type);
            string mangledName = mangle(baseName, fnty->extTys);
            return compMetaFunctionResult(c, l->loc, baseName, mangledName, typedArgs);
        }
    }

    //Create the call to tvf.val, not f as if tvf is a function pointer,
    //passing it as f will fail.
    auto *call = c->builder.CreateCall(tvf.val, args);
    return TypedValue(call, tvf.type->getFunctionReturnType());
}

TypedValue Compiler::compLogicalOr(Node *lexpr, Node *rexpr, BinOpNode *op){
    Function *f = builder.GetInsertBlock()->getParent();
    auto &blocks = f->getBasicBlockList();

    auto lhs = CompilingVisitor::compile(this, lexpr);
    if(lhs.type->typeTag != TT_Bool)
        return compErr("The 'or' operator's lval must be of type bool, but instead is of type "
                + anTypeToColoredStr(lhs.type), op->lval->loc);

    auto *curbbl = builder.GetInsertBlock();
    auto *orbb = BasicBlock::Create(*ctxt, "or");
    auto *mergebb = BasicBlock::Create(*ctxt, "merge");

    builder.CreateCondBr(lhs.val, mergebb, orbb);
    blocks.push_back(orbb);
    blocks.push_back(mergebb);


    builder.SetInsertPoint(orbb);
    auto rhs = CompilingVisitor::compile(this, rexpr);

    //the block must be re-gotten in case the expression contains if-exprs, while nodes,
    //or other exprs that change the current block
    auto *curbbr = builder.GetInsertBlock();
    builder.CreateBr(mergebb);

    if(rhs.type->typeTag != TT_Bool)
        return compErr("The 'or' operator's rval must be of type bool, but instead is of type "
                + anTypeToColoredStr(rhs.type), op->rval->loc);

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

    auto lhs = CompilingVisitor::compile(this, lexpr);
    if(lhs.type->typeTag != TT_Bool)
        return compErr("The 'and' operator's lval must be of type bool, but instead is of type "
                + anTypeToColoredStr(lhs.type), op->lval->loc);

    auto *curbbl = builder.GetInsertBlock();
    auto *andbb = BasicBlock::Create(*ctxt, "and");
    auto *mergebb = BasicBlock::Create(*ctxt, "merge");

    builder.CreateCondBr(lhs.val, andbb, mergebb);
    blocks.push_back(andbb);
    blocks.push_back(mergebb);


    builder.SetInsertPoint(andbb);
    auto rhs = CompilingVisitor::compile(this, rexpr);

    //the block must be re-gotten in case the expression contains if-exprs, while nodes,
    //or other exprs that change the current block
    auto *curbbr = builder.GetInsertBlock();
    builder.CreateBr(mergebb);

    if(rhs.type->typeTag != TT_Bool)
        return compErr("The 'and' operator's rval must be of type bool, but instead is of type "
                + anTypeToColoredStr(rhs.type), op->rval->loc);

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
    if(tc) return arg;

    return tryImplicitCast(c, arg, ty);
}


TypedValue checkForOperatorOverload(Compiler *c, TypedValue &lhs, int op, TypedValue rhs){
    string basefn = Lexer::getTokStr(op);
    string mangledfn = mangle(basefn, {lhs.type, rhs.type});

    //now look for the function
    vector<AnType*> argtys = {lhs.type, rhs.type};
    auto fn = c->getMangledFn(basefn, argtys);
    if(!fn) return fn;

    auto *fnty = try_cast<AnFunctionType>(fn.type);

    AnType *param1 = fnty->extTys[0];
    AnType *param2 = fnty->extTys[1];

    lhs = typeCheckWithImplicitCasts(c, lhs, param1);
    rhs = typeCheckWithImplicitCasts(c, rhs, param2);

    if(implicitPassByRef(param1)) lhs = addrOf(c, lhs);
    if(implicitPassByRef(param2)) rhs = addrOf(c, rhs);

    vector<Value*> argVals = {lhs.val, rhs.val};
    auto call = c->builder.CreateCall(fn.val, argVals);
    return TypedValue(call, fnty->getFunctionReturnType());
}


void CompilingVisitor::visit(SeqNode *n){
    size_t i = 1;
    for(auto &node : n->sequence){
        try{
            node->accept(*this);
            if(dynamic_cast<FuncDeclNode*>(node.get()))
                node.release();
        }catch(CtError *e){
            //Unless the final value throws, delete the error
            if(i == n->sequence.size()) throw e;
            else delete e;
        }
        i++;
    }
}


TypedValue handlePointerOffset(BinOpNode *n, Compiler *c, TypedValue &lhs, TypedValue &rhs){
    Value *ptr;
    Value *idx;
    AnType *ptrTy;

    if(lhs.type->typeTag == TT_Ptr and rhs.type->typeTag != TT_Ptr){
        ptr = lhs.val;
        idx = rhs.val;
        ptrTy = lhs.type;
    }else if(lhs.type->typeTag != TT_Ptr and rhs.type->typeTag == TT_Ptr){
        ptr = rhs.val;
        idx = lhs.val;
        ptrTy = rhs.type;
    }else{
        c->compErr("Operands for pointer addition must be a pointer and an integer", n->loc);
    }

    if(n->op == '+'){
        return {c->builder.CreateInBoundsGEP(ptr, idx), ptrTy};
    }else if(n->op == '-'){
        idx = c->builder.CreateNeg(idx);
        return {c->builder.CreateInBoundsGEP(ptr, idx), ptrTy};
    }else{
        c->compErr("Operator " + to_string(n->op) + " is not a primitive pointer operator", n->loc);
    }
    return {}; //unreachable
}


/*
 *  Compiles an operation along with its lhs and rhs
 */
void CompilingVisitor::visit(BinOpNode *n){
    if(n->op == '.'){
        this->val = c->compMemberAccess(n->lval.get(), (VarNode*)n->rval.get(), n);
        return;
    }else if(n->op == '('){
        this->val = compFnCall(c, n->lval.get(), n->rval.get());
        return;
    }else if(n->op == Tok_And){
        this->val = c->compLogicalAnd(n->lval.get(), n->rval.get(), n);
        return;
    }else if(n->op == Tok_Or){
        this->val = c->compLogicalOr(n->lval.get(), n->rval.get(), n);
        return;
    }

    TypedValue lhs = CompilingVisitor::compile(c, n->lval);
    TypedValue rhs = CompilingVisitor::compile(c, n->rval);

    TypedValue res;
    if((res = checkForOperatorOverload(c, lhs, n->op, rhs))){
        this->val =res;
        return;
    }

    if(n->op == '#'){
        this->val = c->compExtract(lhs, rhs, n);
        return;
    }


    //Check if both Values are numeric, and if so, check if their types match.
    //If not, do an implicit conversion (usually a widening) to match them.
    c->handleImplicitConversion(&lhs, &rhs);


    //first, if both operands are primitive numeric types, use the default ops
    if(isNumericTypeTag(lhs.type->typeTag) && isNumericTypeTag(rhs.type->typeTag)){
        this->val = handlePrimitiveNumericOp(n, c, lhs, rhs);
        return;

    //and bools/ptrs are only compatible with == and !=
    }else if((lhs.type->typeTag == TT_Bool and rhs.type->typeTag == TT_Bool) or
             (lhs.type->typeTag == TT_Ptr  and rhs.type->typeTag == TT_Ptr)){

        //== is no longer implemented for pointers by default
        if(n->op == Tok_Eq and lhs.type->typeTag == TT_Bool and rhs.type->typeTag == TT_Bool){
            this->val = TypedValue(c->builder.CreateICmpEQ(lhs.val, rhs.val), AnType::getBool());
            return;
        }

        if(n->op == Tok_Is){
            this->val = TypedValue(c->builder.CreateICmpEQ(lhs.val, rhs.val), AnType::getBool());
            return;
        }else if(n->op == Tok_NotEq){
            this->val = TypedValue(c->builder.CreateICmpNE(lhs.val, rhs.val), AnType::getBool());
            return;
        }
    }

    if(n->op == '+' or n->op == '-'){
        if((lhs.type->typeTag == TT_Ptr or isNumericTypeTag(lhs.type->typeTag)) and
           (rhs.type->typeTag == TT_Ptr or isNumericTypeTag(rhs.type->typeTag))){
            this->val = handlePointerOffset(n, c, lhs, rhs);
            return;
        }
    }

    c->compErr("Operator " + Lexer::getTokStr(n->op) + " is not overloaded for types "
            + anTypeToColoredStr(lhs.type) + " and " + anTypeToColoredStr(rhs.type), n->loc);
}


void CompilingVisitor::visit(UnOpNode *n){
    n->rval->accept(*this);

    switch(n->op){
        case '@': //pointer dereference
            if(val.type->typeTag != TT_Ptr){
                c->compErr("Cannot dereference non-pointer type " + anTypeToColoredStr(val.type), n->loc);
            }

            this->val = TypedValue(c->builder.CreateLoad(val.val), try_cast<AnPtrType>(val.type)->extTy);
            return;
        case '&': //address-of
            this->val = addrOf(c, val);
            return;
        case '-': //negation
            if(!isNumericTypeTag(val.type->typeTag))
                c->compErr("Cannot negate non-numeric type " + anTypeToColoredStr(val.type), n->loc);

            this->val = TypedValue(c->builder.CreateNeg(val.val), val.type);
            return;
        case Tok_Not:
            if(val.type->typeTag != TT_Bool)
                c->compErr("Unary not operator not overloaded for type " + anTypeToColoredStr(val.type), n->loc);

            this->val = TypedValue(c->builder.CreateNot(val.val), val.type);
            return;
        case Tok_New:
            //the 'new' keyword in ante creates a reference to any existing value
            this->val = createMallocAndStore(c, val);
            return;
    }

    c->compErr("Unknown unary operator " + Lexer::getTokStr(n->op), n->loc);
}

} // end of namespace ante
