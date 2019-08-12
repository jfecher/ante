#include "pattern.h"
#include "types.h"
#include "util.h"

using namespace std;
using namespace llvm;
using namespace ante::parser;

namespace ante {

    enum LiteralType {
        Int, Flt, Str
    };

    Function* getCurFunction(Compiler *c){
        return c->builder.GetInsertBlock()->getParent();
    }

    void match_literal(CompilingVisitor &cv, MatchNode *n, Node *pattern,
            BasicBlock *jmpOnFail, TypedValue &valToMatch, LiteralType literalType){

        pattern->accept(cv);

        Value *eq;
        if(literalType == Int){
            eq = cv.c->builder.CreateICmpEQ(cv.val.val, valToMatch.val);
        }else if(literalType == Flt){
            eq = cv.c->builder.CreateFCmpOEQ(cv.val.val, valToMatch.val);
        }else if(literalType == Str){
            eq = cv.c->callFn("==", {cv.val, valToMatch}).val;
        }else{
            ASSERT_UNREACHABLE();
        }

        BasicBlock *jmpOnSuccess = BasicBlock::Create(*cv.c->ctxt, "match", getCurFunction(cv.c));

        cv.c->builder.CreateCondBr(eq, jmpOnSuccess, jmpOnFail);
        cv.c->builder.SetInsertPoint(jmpOnSuccess);
    }

    /**
     * Match a catch-all var pattern that binds the
     * matched value to the given identifier
     */
    void match_var(CompilingVisitor &cv, MatchNode *n, VarNode *pattern,
            BasicBlock *jmpOnFail, TypedValue &valToMatch){

        //Do not bind to _ to enforce convention of _ to indicate an unused value
        if(pattern->name != "_"){
            pattern->decl->tval = valToMatch;
        }
    }

    /**
     * Match a tuple-destructure pattern
     */
    void match_tuple(CompilingVisitor &cv, MatchNode *n, TupleNode *t,
            BasicBlock *jmpOnFail, TypedValue &valToMatch){

        Type *tupTy = valToMatch.getType();

        if(!tupTy->isStructTy()){
            error("Cannot match tuple pattern against non-tuple type "
                    + anTypeToColoredStr(valToMatch.type), t->loc);
        }

        if(t->exprs.size() != tupTy->getNumContainedTypes()){
            error("Cannot match a tuple of size " + to_string(t->exprs.size()) +
                " to a pattern of size " + to_string(tupTy->getNumContainedTypes()), t->loc);
        }

        auto *aggTy = try_cast<AnAggregateType>(valToMatch.type);
        size_t elementNo = 0;

        for(auto &e : t->exprs){
            Value *elem = cv.c->builder.CreateExtractValue(valToMatch.val, elementNo);
            TypedValue elemTv{elem, (AnType*)valToMatch.type->addModifiersTo(aggTy->extTys[elementNo++])};

            handlePattern(cv, n, e.get(), jmpOnFail, elemTv);
        }
    }

    AnType* unionVariantToTupleTy(AnType *ty){
        if(ty->typeTag == TT_Data){
            AnProductType *dt = static_cast<AnProductType*>(ty);

            if(dt->fields.size() == 1){
                return dt->fields[0];
            }else{
                return AnType::getTupleOf(dt->fields);
            }
        }
        return ty;
    }

    Type* getUnionVariantType(Compiler *c, AnProductType *tagTy){
        AnType *anTagData = unionVariantToTupleTy(tagTy);
        Type *tagData = c->anTypeToLlvmType(anTagData);
        return tagData->isVoidTy() ?
            StructType::get(*c->ctxt, {c->builder.getInt8Ty()}, true) :
            StructType::get(*c->ctxt, {c->builder.getInt8Ty(), tagData}, true);
    }

    TypedValue unionDowncast(Compiler *c, TypedValue valToMatch, AnProductType *tagTy){
        auto alloca = addrOf(c, valToMatch);

        //bitcast valToMatch* to (tag, tagData)*
        auto *castTy = getUnionVariantType(c, tagTy);

        if(castTy->getStructNumElements() != 1){
            auto *cast = c->builder.CreateBitCast(alloca.val, castTy->getPointerTo());

            //extract tag_data from (tag, tagData)*
            auto *gep = c->builder.CreateStructGEP(castTy, cast, 1);
            auto *deref = c->builder.CreateLoad(gep);
            auto *type = (AnType*)valToMatch.type->addModifiersTo(unionVariantToTupleTy(tagTy));
            return {deref, type};
        }else{
            return c->getVoidLiteral();
        }
    }

    /**
     * Match a union variant pattern, eg. Some x or None
     * @param pattern The type to match against, eg. Some
     * @param bindExpr The optional expr to bind params to, eg. x
     */
    void match_variant(CompilingVisitor &cv, MatchNode *n, TypeNode *pattern,
            Node *bindExpr, BasicBlock *jmpOnFail, TypedValue &valToMatch){

        Compiler *c = cv.c;

        // TODO: Fix with new type information
        auto *tagTy = static_cast<AnProductType*>(pattern->getType());
        auto *parentTy = static_cast<AnSumType*>(pattern->getType()); //wrong

        ConstantInt *ci = ConstantInt::get(*c->ctxt,
                APInt(8, parentTy->getTagVal(pattern->typeName), true));

        //Extract tag value and check for equality
        Value *eq;
        if(valToMatch.getType()->isStructTy()){
            Value *tagVal = c->builder.CreateExtractValue(valToMatch.val, 0);
            eq = c->builder.CreateICmpEQ(tagVal, ci);
        }else if(valToMatch.getType()->isIntegerTy()){
            eq = c->builder.CreateICmpEQ(valToMatch.val, ci);
        }else{
            //all tagged unions are either just their tag (enum) or a tag and value.
            ASSERT_UNREACHABLE();
        }

        BasicBlock *jmpOnSuccess = BasicBlock::Create(*cv.c->ctxt, "match", getCurFunction(cv.c));
        c->builder.CreateCondBr(eq, jmpOnSuccess, jmpOnFail);
        c->builder.SetInsertPoint(jmpOnSuccess);

        //bind any identifiers and match remaining pattern
        if(bindExpr){
            TypedValue variant;
            if(valToMatch.getType()->isStructTy()){
                variant = unionDowncast(c, valToMatch, tagTy);
            }else if(valToMatch.getType()->isIntegerTy()){
                variant = c->getVoidLiteral();
            }else{
                //all tagged unions are either just their tag (enum) or a tag and value.
                ASSERT_UNREACHABLE();
            }
            handlePattern(cv, n, bindExpr, jmpOnFail, variant);
        }
    }

    void handlePattern(CompilingVisitor &cv, MatchNode *n, Node *pattern,
            BasicBlock *jmpOnFail, TypedValue valToMatch){

        if(TupleNode *tn = dynamic_cast<TupleNode*>(pattern)){
            match_tuple(cv, n, tn, jmpOnFail, valToMatch);

        }else if(TypeCastNode *tcn = dynamic_cast<TypeCastNode*>(pattern)){
            match_variant(cv, n, tcn->typeExpr.get(), tcn->rval.get(), jmpOnFail, valToMatch);

        }else if(TypeNode *tn = dynamic_cast<TypeNode*>(pattern)){
            match_variant(cv, n, tn, nullptr, jmpOnFail, valToMatch);

        }else if(VarNode *vn = dynamic_cast<VarNode*>(pattern)){
            match_var(cv, n, vn, jmpOnFail, valToMatch);

        }else if(dynamic_cast<IntLitNode*>(pattern)){
            match_literal(cv, n, pattern, jmpOnFail, valToMatch, Int);

        }else if(dynamic_cast<FltLitNode*>(pattern)){
            match_literal(cv, n, pattern, jmpOnFail, valToMatch, Flt);

        }else if(dynamic_cast<StrLitNode*>(pattern)){
            match_literal(cv, n, pattern, jmpOnFail, valToMatch, Str);

        }else{
            error("Unknown pattern", pattern->loc);
        }
    }


    void CompilingVisitor::visit(MatchNode *n){
        n->expr->accept(*this);
        auto valToMatch = this->val;

        Function *f = c->builder.GetInsertBlock()->getParent();

        vector<pair<BasicBlock*,TypedValue>> merges;
        merges.reserve(n->branches.size());

        BasicBlock *endmatch = BasicBlock::Create(*c->ctxt, "end_match", f);
        BasicBlock *finalEndPat = nullptr;

        for(auto& mbn : n->branches){
            BasicBlock *endpat = &mbn == &n->branches.back() ?
                endmatch : BasicBlock::Create(*c->ctxt, "end_pattern", f);

            handlePattern(*this, n, mbn->pattern.get(), endpat, valToMatch);
            mbn->branch->accept(*this);
            merges.push_back({c->builder.GetInsertBlock(), this->val});

            //dont jump to after the match if the branch already returned from the function
            if(!dyn_cast<ReturnInst>(this->val.val))
                c->builder.CreateBr(endmatch);

            c->builder.SetInsertPoint(endpat); //set insert point to next branch
            finalEndPat = endpat == endmatch ? finalEndPat : endpat;
        }

        // Cannot prove to LLVM match is exhaustive so an uninitialized value must be
        // "returned" each time from the branch where all matches fail.
        if(finalEndPat){
            TypedValue retOnFailAll = {UndefValue::get(this->val.getType()), val.type};
            merges.push_back({finalEndPat, retOnFailAll});
        }

        //merges can be empty if each branch has an early return
        if(merges.empty() or merges[0].second.type->typeTag == TT_Unit){
            this->val = c->getVoidLiteral();
            return;
        }

        int i = 1;
        auto *phi = c->builder.CreatePHI(merges[0].second.getType(), n->branches.size());
        for(auto &pair : merges){
            //add each branch to the phi node if it does not return early
            if(!dyn_cast<ReturnInst>(pair.second.val)){
                phi->addIncoming(pair.second.val, pair.first);
            }
            i++;
        }
        //phi->addIncoming(UndefValue::get(merges[0].second.getType()), matchbb);
        this->val = TypedValue(phi, merges[0].second.type);
    }


    Pattern Pattern::getFillerPattern(){
        return Pattern{TT_TypeVar};
    }

    Pattern Pattern::fromSumType(const AnSumType *t){
        Pattern p{TT_Data};
        p.name = t->name;
        for(AnProductType *variant : t->tags){
            Pattern pat = Pattern::fromType(variant->getVariantWithoutTag());
            pat.name = variant->name;
            p.children.push_back(pat);
        }
        return p;
    }

    Pattern Pattern::fromTuple(std::vector<AnType*> const& types){
        Pattern p{TT_Tuple};
        for(AnType *t : types){
            p.children.push_back(Pattern::fromType(t));
        }
        return p;
    }

    Pattern Pattern::fromType(const AnType *t){
        auto st = try_cast<AnSumType>(t);
        if(st) return Pattern::fromSumType(st);

        auto pt = try_cast<AnProductType>(t);
        if(pt) {
          auto pat = Pattern::fromTuple(pt->fields);
          pat.name = pt->name;
          return pat;
        }

        auto ag = try_cast<AnAggregateType>(t);
        if(ag) return Pattern::fromTuple(ag->extTys);

        auto tv = try_cast<AnTypeVarType>(t);
        if(tv) return Pattern::getFillerPattern();

        return {t->typeTag};
    }

    void Pattern::overwrite(Pattern const& other, LOC_TY &loc){
        if(type == other.type && (name == other.name || other.name.empty()))
            return;

        if(type == TT_TypeVar){
            this->type = other.type;
            this->name = other.name;
            this->children = other.children;
        }else{
            lazy_str typeA{name.empty() ? typeTagToStr(type) : name, AN_TYPE_COLOR};
            lazy_str typeB{other.name.empty() ? typeTagToStr(other.type) : other.name, AN_TYPE_COLOR};
            error("Conflicting types in pattern, inferenced is " +
                typeA + ", but found here is " + typeB, loc);
        }
    }

    void Pattern::setMatched(){
        matched = true;
    }

    bool Pattern::irrefutable() const {
        if(matched) return matched;
        if(children.empty()) return false;

        for(auto &child : children){
            if(!child.irrefutable())
              return false;
        }
        return true;
    }

    Pattern& Pattern::getChild(size_t idx) {
        return children[idx];
    }

    lazy_printer Pattern::constructMissedCase() const {
        if(irrefutable()){
            std::cerr << "error in Pattern::constructMissedCase: No missed cases in an irrefutable pattern.\n";
            exit(2);
        }

        if(type == TT_Data){
            for(auto &p : children){
                if(!p.irrefutable()){
                    return p.constructMissedCase();
                }
            }
            ASSERT_UNREACHABLE();
        }

        if(type == TT_Tuple){
            lazy_printer args = "(";
            for(auto &p : children){
                if(!p.irrefutable()){
                    args = args + p.constructMissedCase();
                }else{
                    args = args + '_';
                }
                if(&p != &children.back())
                    args = args + ", ";
            }
            args = args + ')';
            lazy_str ret{name, AN_TYPE_COLOR};
            return ret + (!name.empty() && children.empty() ? "" : args);
        }

        if(type == TT_TypeVar){
            return "_";
        }

        return lazy_str("0_" + typeTagToStr(type), AN_CONSTANT_COLOR) + "";
    }
}
