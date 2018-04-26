#include "compiler.h"
#include "types.h"

using namespace std;
using namespace llvm;
using namespace ante::parser;

namespace ante {
    void handleTypeCastPattern(Compiler *c, TypedValue lval, TypeCastNode *tn, AnDataType *tagTy, AnDataType *parentTy){
        //If this is a generic type cast like Some 't, the 't must be bound to a concrete type first

        //This is a pattern of the match _ with expr, so if that is mutable this should be too
        //tagTy = (AnDataType*)tagTy->setModifier(lval.type->mods);
        //AnType *tagtycpy = tagTy/*->extTys[0]*/;

        auto tcr = c->typeEq(parentTy, lval.type);

        if(tcr->res == TypeCheckResult::SuccessWithTypeVars)
            tagTy = (AnDataType*)bindGenericToType(c, tagTy, tcr->bindings);
        else if(tcr->res == TypeCheckResult::Failure)
            c->compErr("Cannot bind pattern of type " + anTypeToColoredStr(parentTy) +
                    " to matched value of type " + anTypeToColoredStr(lval.type), tn->rval->loc);

        //cast it from (<tag type>, <largest union member type>) to (<tag type>, <this union member's type>)
        auto *tupTy = StructType::get(*c->ctxt, {Type::getInt8Ty(*c->ctxt), c->anTypeToLlvmType(tagTy)}, true);

        auto alloca = addrOf(c, lval);

        //bit cast the alloca to a pointer to the largest type of the parent union
        //auto *cast = c->builder.CreateBitCast(alloca.val, c->anTypeToLlvmType(parentTy)->getPointerTo());
        auto cast = alloca.val;

        //Cast in the form of: Some n
        if(VarNode *v = dynamic_cast<VarNode*>(tn->rval.get())){
            auto *tup = c->builder.CreateLoad(cast);
            auto extract = TypedValue(c->builder.CreateExtractValue(tup, 1), tagTy->extTys[0]);

            c->stoVar(v->name, new Variable(v->name, extract, c->scope));

        //Destructure multiple: Triple(x, y, z)
        }else if(TupleNode *t = dynamic_cast<TupleNode*>(tn->rval.get())){
            auto *taggedValTy = tupTy->getStructElementType(1);
            if(!tupTy->isStructTy()){
                c->compErr("Cannot match tuple pattern against non-tuple type " + anTypeToColoredStr(tagTy), t->loc);
            }

            if(t->exprs.size() != taggedValTy->getNumContainedTypes()){
                c->compErr("Cannot match a tuple of size " + to_string(t->exprs.size()) +
                    " to a pattern of size " + to_string(taggedValTy->getNumContainedTypes()), t->loc);
            }

            auto *aggTy = (AnAggregateType*)tagTy;
            size_t elementNo = 0;

            for(auto &e : t->exprs){
                VarNode *v;
                if(!(v = dynamic_cast<VarNode*>(e.get()))){
                    c->compErr("Unknown pattern, expected identifier", e->loc);
                }

                auto *zero = c->builder.getInt32(0);
                auto *ptr = c->builder.CreateGEP(cast, {zero, c->builder.getInt32(1)});
                ptr = c->builder.CreateGEP(ptr, {zero, c->builder.getInt32(elementNo)});

                AnType *curTy = aggTy->extTys[elementNo];
                auto elem = TypedValue(c->builder.CreateLoad(ptr), curTy);
                c->stoVar(v->name, new Variable(v->name, elem, c->scope));
                elementNo++;
            }

        }else{
            c->compErr("Cannot match unknown pattern", tn->rval->loc);
        }
    }


    void CompilingVisitor::visit(MatchNode *n){
        n->expr->accept(*this);
        auto valToMatch = this->val;

        if(valToMatch.type->typeTag != TT_TaggedUnion && valToMatch.type->typeTag != TT_Data){
            c->compErr("Cannot match expression of type " + anTypeToColoredStr(valToMatch.type) +
                    ".  Match expressions must be a tagged union type", n->expr->loc);
        }

        //the tag is always the zero-th index except for in certain optimization cases and if
        //the tagged union has no tagged values and is equivalent to an enum in C-like languages.
        Value *switchVal = llvmTypeToTypeTag(valToMatch.getType()) == TT_Tuple ?
                c->builder.CreateExtractValue(valToMatch.val, 0)
                : valToMatch.val;

        Function *f = c->builder.GetInsertBlock()->getParent();
        auto *matchbb = c->builder.GetInsertBlock();

        auto *end = BasicBlock::Create(*c->ctxt, "end_match");
        auto *match = c->builder.CreateSwitch(switchVal, end, n->branches.size());
        vector<pair<BasicBlock*,TypedValue>> merges;

        for(auto& mbn : n->branches){
            ConstantInt *ci = nullptr;
            auto *br = BasicBlock::Create(*c->ctxt, "br", f);
            c->builder.SetInsertPoint(br);
            c->enterNewScope();

            //TypeCast-esque pattern:  Some n
            if(TypeCastNode *tn = dynamic_cast<TypeCastNode*>(mbn->pattern.get())){
                auto *tagTy = AnDataType::get(tn->typeExpr->typeName);
                if(!tagTy or tagTy->isStub())
                    c->compErr("Union tag " + typeNodeToColoredStr(tn->typeExpr.get()) + " was not yet declared.", tn->typeExpr->loc);

                if(!tagTy->isUnionTag())
                    c->compErr(typeNodeToColoredStr(tn->typeExpr.get()) + " must be a union tag to be used in a pattern", tn->typeExpr->loc);

                auto *parentTy = tagTy->parentUnionType;
                ci = ConstantInt::get(*c->ctxt, APInt(8, parentTy->getTagVal(tn->typeExpr->typeName), true));

                tagTy = (AnDataType*)bindGenericToType(c, tagTy, ((AnDataType*)valToMatch.type)->boundGenerics);
                tagTy = tagTy->setModifier(valToMatch.type->mods);
                handleTypeCastPattern(c, valToMatch, tn, tagTy, parentTy);

            //single type pattern:  None
            }else if(TypeNode *tn = dynamic_cast<TypeNode*>(mbn->pattern.get())){
                auto *tagTy = AnDataType::get(tn->typeName);
                if(!tagTy or tagTy->isStub())
                    c->compErr("Union tag " + typeNodeToColoredStr(tn) + " was not yet declared.", tn->loc);

                if(!tagTy->isUnionTag())
                    c->compErr(typeNodeToColoredStr(tn) + " must be a union tag to be used in a pattern", tn->loc);

                auto *parentTy = tagTy->parentUnionType;
                ci = ConstantInt::get(*c->ctxt, APInt(8, parentTy->getTagVal(tn->typeName), true));

            //variable/match-all pattern: _
            }else if(VarNode *vn = dynamic_cast<VarNode*>(mbn->pattern.get())){
                auto tn = TypedValue(valToMatch.val, valToMatch.type);
                match->setDefaultDest(br);
                c->stoVar(vn->name, new Variable(vn->name, tn, c->scope));
            }else{
                c->compErr("Pattern matching non-tagged union types is not yet implemented", mbn->pattern->loc);
            }

            mbn->branch->accept(*this);
            c->exitScope();

            if(!dyn_cast<ReturnInst>(val.val) and !dyn_cast<BranchInst>(val.val))
                c->builder.CreateBr(end);

            merges.push_back(pair<BasicBlock*,TypedValue>(c->builder.GetInsertBlock(), val));

            if(ci)
                match->addCase(ci, br);
        }

        f->getBasicBlockList().push_back(end);
        c->builder.SetInsertPoint(end);

        //merges can be empty if each branch has an early return
        if(merges.empty() or merges[0].second.type->typeTag == TT_Void){
            this->val = c->getVoidLiteral();
            return;
        }

        int i = 1;
        auto *phi = c->builder.CreatePHI(merges[0].second.getType(), n->branches.size());
        for(auto &pair : merges){

            //add each branch to the phi node if it does not return early
            if(!dyn_cast<ReturnInst>(pair.second.val)){

                //match the types of those branches that will merge
                if(!c->typeEq(pair.second.type, merges[0].second.type))
                    c->compErr("Branch "+to_string(i)+"'s return type " + anTypeToColoredStr(pair.second.type) +
                            " != " + anTypeToColoredStr(merges[0].second.type) + ", the first branch's return type", n->loc);
                else
                    phi->addIncoming(pair.second.val, pair.first);
            }
            i++;
        }
        phi->addIncoming(UndefValue::get(merges[0].second.getType()), matchbb);
        this->val = TypedValue(phi, merges[0].second.type);
    }
}
