#include "constraintfindingvisitor.h"
#include "unification.h"
#include "funcdecl.h"
#include "compiler.h"
#include "pattern.h"
#include "antype.h"
#include "module.h"
#include "trait.h"
#include "types.h"
#include "util.h"

using namespace std;

namespace ante {
    using namespace parser;

    UnificationList ConstraintFindingVisitor::getConstraints() const {
        return constraints;
    }

    void ConstraintFindingVisitor::addConstraint(AnType *a, AnType *b, LOC_TY &loc){
        constraints.emplace_back(a, b, loc);
    }

    void ConstraintFindingVisitor::addTypeClassConstraint(TraitImpl *constraint, LOC_TY &loc){
        constraints.emplace_back(constraint, loc);
    }

    template<typename T>
    inline void acceptAll(ConstraintFindingVisitor &v, std::vector<T> const& nodes){
        for(auto &n : nodes){
            TRY_TO(n->accept(v));
        }
    }

    /** Annotate all nodes with placeholder types */
    void ConstraintFindingVisitor::visit(RootNode *n){
        acceptAll(*this, n->imports);
        acceptAll(*this, n->types);
        acceptAll(*this, n->traits);
        acceptAll(*this, n->main);
    }

    void ConstraintFindingVisitor::visit(IntLitNode *n){}

    void ConstraintFindingVisitor::visit(FltLitNode *n){}

    void ConstraintFindingVisitor::visit(BoolLitNode *n){}

    void ConstraintFindingVisitor::visit(StrLitNode *n){}

    void ConstraintFindingVisitor::visit(CharLitNode *n){}

    void ConstraintFindingVisitor::visit(ArrayNode *n){
        auto arrty = try_cast<AnArrayType>(n->getType());
        if(!n->exprs.empty()){
            auto t1 = n->exprs[0]->getType();
            for(auto it = ++n->exprs.begin(); it != n->exprs.end(); it++){
                (*it)->accept(*this);
                addConstraint(t1, (*it)->getType(), (*it)->loc);
            }
            addConstraint(arrty->extTy, t1, n->loc);
        }else{
            addConstraint(arrty->extTy, AnType::getVoid(), n->loc);
        }
    }

    void ConstraintFindingVisitor::visit(TupleNode *n){
        for(auto &e : n->exprs)
            e->accept(*this);
    }

    void ConstraintFindingVisitor::visit(ModNode *n){
        if(n->expr)
            n->expr->accept(*this);
    }

    void ConstraintFindingVisitor::visit(TypeNode *n){}

    void ConstraintFindingVisitor::visit(TypeCastNode *n){
        n->rval->accept(*this);

        auto variant = try_cast<AnProductType>(n->typeExpr->getType());
        if(variant){
            TupleNode *tn = dynamic_cast<TupleNode*>(n->rval.get());

            size_t argc = tn ? tn->exprs.size() : 1;
            size_t offset = variant->parentUnionType ? 1 : 0;

            if(variant->fields.size() - offset != argc){
                auto lplural = variant->fields.size() == 1 + offset ? " argument, but " : " arguments, but ";
                auto rplural = argc == 1 ? " was given instead" : " were given instead";
                error(anTypeToColoredStr(variant) + " requires " + to_string(variant->fields.size()-offset)
                        + lplural + to_string(argc) + rplural, n->loc);
            }

            if(tn){
                for(size_t i = 0; i < argc; i++){
                    addConstraint(tn->exprs[i]->getType(), variant->fields[i+offset], tn->exprs[i]->loc);
                }
            }else{
                addConstraint(n->rval->getType(), variant->fields[offset], n->rval->loc);
            }
        }else{
            TraitDecl *decl = module->lookupTraitDecl("To");
            TraitImpl *impl = new TraitImpl(decl, {n->typeExpr->getType(), n->rval->getType()});
            addTypeClassConstraint(impl, n->loc);
        }
    }

    TraitImpl* getUnOpTraitType(Module *module, int op){
        TraitImpl *impl;
        switch(op){
            case '@': impl = module->freshTraitImpl("Deref"); break;
            case '-': impl = module->freshTraitImpl("Neg"); break;
            case Tok_Not: impl = module->freshTraitImpl("Not"); break;
            default:
                cerr << "getUnOpTraitType: unknown op '" << (char)op << "' (" << (int)op << ") given.  ";
                exit(1);
        }
        if(!impl){
            cerr << "getUnOpTraitType: numeric trait for op '" << (char)op << "' (" << (int)op << ") not found.  The stdlib may not have been imported properly.\n";
            exit(1);
        }
        return impl;
    }

    void ConstraintFindingVisitor::visit(UnOpNode *n){
        n->rval->accept(*this);
        auto tv = nextTypeVar();
        TraitImpl *trait;
        switch(n->op){
        case '@':
            addConstraint(n->rval->getType(), AnPtrType::get(tv), n->loc);
            addConstraint(n->getType(), tv, n->loc);
            break;
        case '&':
            addConstraint(n->getType(), AnPtrType::get(tv), n->loc);
            addConstraint(n->rval->getType(), tv, n->loc);
            break;
        case '-': //negation
            trait = getUnOpTraitType(module, n->op);
            addTypeClassConstraint(trait, n->loc);
            addConstraint(n->getType(), n->rval->getType(), n->loc);
            break;
        case Tok_Not:
            trait = getUnOpTraitType(module, n->op);
            addTypeClassConstraint(trait, n->loc);
            addConstraint(n->getType(), n->rval->getType(), n->loc);
            break;
        case Tok_New:
            addConstraint(n->getType(), AnPtrType::get(tv), n->loc);
            addConstraint(n->rval->getType(), tv, n->loc);
            break;
        }
    }

    void ConstraintFindingVisitor::visit(SeqNode *n){
        acceptAll(*this, n->sequence);
    }

    TraitImpl* getOpTraitType(Module *module, int op){
        TraitImpl *parent;
        switch(op){
            case '+': parent = module->freshTraitImpl("Add"); break;
            case '-': parent = module->freshTraitImpl("Sub"); break;
            case '*': parent = module->freshTraitImpl("Mul"); break;
            case '/': parent = module->freshTraitImpl("Div"); break;
            case '%': parent = module->freshTraitImpl("Mod"); break;
            case '^': parent = module->freshTraitImpl("Pow"); break;
            case '<': parent = module->freshTraitImpl("Cmp"); break;
            case '>': parent = module->freshTraitImpl("Cmp"); break;
            case Tok_GrtrEq: parent = module->freshTraitImpl("Cmp"); break;
            case Tok_LesrEq: parent = module->freshTraitImpl("Cmp"); break;
            case '=': parent = module->freshTraitImpl("Eq"); break;
            case Tok_NotEq: parent = module->freshTraitImpl("Eq"); break;
            default:
                cerr << "getOpTraitType: unknown op '" << (char)op << "' (" << (int)op << ") given.  ";
                exit(1);
        }
        if(!parent){
            cerr << "getOpTraitType: numeric trait for op '" << (char)op << "' (" << (int)op << ") not found.  The stdlib may not have been imported properly.\n";
            exit(1);
        }
        return parent;
    }


    pair<TraitImpl*, AnTypeVarType*> getCollectionOpTraitType(Module *module, int op){
        auto collectionTyVar = nextTypeVar();
        auto elemTy = nextTypeVar();

        if(op != '#' && op != Tok_In){
            cerr << "getCollectionOpTraitType: unknown op '" << (char)op << "' (" << (int)op << ") given.  ";
            exit(1);
        }

        string traitName = op == '#' ? "Extract" : "In";
        TraitDecl *parent = module->lookupTraitDecl(traitName);

        if(!parent){
            cerr << "Cannot find the trait" << traitName << ". The prelude may not have been imported properly.\n";
            exit(1);
        }
        return {new TraitImpl(parent, {collectionTyVar, elemTy}), elemTy};
    }


    TraitImpl* getRangeTraitType(Module *module){
        auto range = module->freshTraitImpl("Range");
        if(!range){
            cerr << "Cannot find the trait Range. The prelude may not have been imported properly.\n";
            exit(1);
        }
        return range;
    }


    bool invalidNumArguments(AnFunctionType *fnty, AnAggregateType *args){
        size_t argc = args->extTys.size();
        size_t paramc = fnty->extTys.size();
        if(argc == paramc)
            return false;

        if(argc == paramc + 1 && args->extTys.back()->typeTag == TT_Unit)
            return false;

        if(argc + 1 == paramc && fnty->extTys.back()->typeTag == TT_Unit)
            return false;

        if(fnty->isVarArgs() && argc >= paramc - 1)
            return false;

        return true;
    }


    void issueInvalidArgCountError(AnFunctionType *fnty, AnAggregateType *args, LOC_TY &loc){
        bool isVA = fnty->isVarArgs();
        size_t paramc = fnty->extTys.size() - (isVA ? 1 : 0);
        size_t argc = args->extTys.size();

        string weregiven = argc == 1 ? " was given" : " were given";
        error("Function takes " + to_string(paramc) + (isVA ? "+" : "")
                + " argument" + plural(paramc) + " but " + to_string(argc)
                + weregiven, loc);
    }


    void ConstraintFindingVisitor::fnCallConstraints(BinOpNode *n){
        auto fnty = try_cast<AnFunctionType>(n->lval->getType());
        if(!fnty){
            auto args = try_cast<AnAggregateType>(n->rval->getType());
            if (!args) args = AnType::getTupleOf({n->rval->getType()});
            auto params = vecOf<AnType*>(args->extTys.size());

            for(size_t i = 0; i < args->extTys.size(); i++){
                auto param = nextTypeVar();
                addConstraint(args->extTys[i], param, n->loc);
                params.push_back(param);
            }
            auto retTy = nextTypeVar();
            addConstraint(n->getType(), retTy, n->loc);

            fnty = AnFunctionType::get(retTy, params, {});
            addConstraint(n->lval->getType(), fnty, n->loc);
        }else{
            auto args = try_cast<AnAggregateType>(n->rval->getType());
            if (!args) args = AnType::getTupleOf({ n->rval->getType() });

            if(invalidNumArguments(fnty, args)){
                issueInvalidArgCountError(fnty, args, n->lval->loc);
            }

            auto argtup = static_cast<parser::TupleNode*>(n->rval.get());

            if(!fnty->isVarArgs()){
                for(size_t i = 0; i < fnty->extTys.size(); i++){
                    addConstraint(args->extTys[i], fnty->extTys[i], argtup->exprs[i]->loc);
                }
            }else{
                size_t i = 0;
                for(; i < fnty->extTys.size() - 1; i++){
                    addConstraint(args->extTys[i], fnty->extTys[i], argtup->exprs[i]->loc);
                }

                // typecheck var args as a tuple of additional arguments, though they should always be
                // matched against a typevar anyway so these constraints should never fail.
                vector<AnType*> varargs;
                for(; i < args->extTys.size(); i++){
                    varargs.push_back(args->extTys[i]);
                }
                addConstraint(AnType::getTupleOf(varargs), fnty->extTys.back(), n->loc);
            }
            addConstraint(n->getType(), fnty->retTy, n->loc);
        }
    }


    void ConstraintFindingVisitor::visit(BinOpNode *n){
        n->lval->accept(*this);
        n->rval->accept(*this);

        if(n->op == '('){
            auto fnty = try_cast<AnFunctionType>(n->lval->getType());
            if(!fnty){
                auto args = try_cast<AnAggregateType>(n->rval->getType());
                if (!args) args = AnType::getTupleOf({n->rval->getType()});
                auto params = vecOf<AnType*>(args->extTys.size());

                for(size_t i = 0; i < args->extTys.size(); i++){
                    auto param = nextTypeVar();
                    addConstraint(args->extTys[i], param, n->loc);
                    params.push_back(param);
                }
                auto retTy = nextTypeVar();
                addConstraint(n->getType(), retTy, n->loc);

                fnty = AnFunctionType::get(retTy, params, {});
                addConstraint(n->lval->getType(), fnty, n->loc);
            }else{
                auto args = try_cast<AnAggregateType>(n->rval->getType());
                if (!args) args = AnType::getTupleOf({ n->rval->getType() });

                if(invalidNumArguments(fnty, args)){
                    issueInvalidArgCountError(fnty, args, n->lval->loc);
                }

                auto argtup = static_cast<parser::TupleNode*>(n->rval.get());

                if(!fnty->isVarArgs()){
                    for(size_t i = 0; i < fnty->extTys.size(); i++){
                        addConstraint(args->extTys[i], fnty->extTys[i], argtup->exprs[i]->loc);
                    }
                }else{
                    size_t i = 0;
                    for(; i < fnty->extTys.size() - 1; i++){
                        addConstraint(args->extTys[i], fnty->extTys[i], argtup->exprs[i]->loc);
                    }

                    // typecheck var args as a tuple of additional arguments, though they should always be
                    // matched against a typevar anyway so these constraints should never fail.
                    vector<AnType*> varargs;
                    for(; i < args->extTys.size(); i++){
                        varargs.push_back(args->extTys[i]);
                    }
                    addConstraint(AnType::getTupleOf(varargs), fnty->extTys.back(), n->loc);
                }
                addConstraint(n->getType(), fnty->retTy, n->loc);
            }
        }else if(n->op == '+' || n->op == '-' || n->op == '*' || n->op == '/' || n->op == '%' || n->op == '^'){
            TraitImpl *num = getOpTraitType(module, n->op);
            addTypeClassConstraint(num, n->loc);
            addConstraint(n->lval->getType(), num->typeArgs[0], n->loc);
            addConstraint(n->rval->getType(), num->typeArgs[0], n->loc);
            addConstraint(n->getType(), num->typeArgs[0], n->loc);
        }else if(n->op == '<' || n->op == '>' || n->op == Tok_GrtrEq || n->op == Tok_LesrEq){
            TraitImpl *num = getOpTraitType(module, n->op);
            addTypeClassConstraint(num, n->loc);
            addConstraint(n->lval->getType(), num->typeArgs[0], n->loc);
            addConstraint(n->rval->getType(), num->typeArgs[0], n->loc);
            addConstraint(n->getType(), AnType::getBool(), n->loc);
        }else if(n->op == '#'){
            auto collection_elemTy = getCollectionOpTraitType(module, n->op);
            auto collection = collection_elemTy.first;

            addConstraint(n->lval->getType(), collection->typeArgs[0], n->loc);
            addConstraint(n->rval->getType(), AnType::getUsz(), n->loc);
            addConstraint(n->getType(), collection_elemTy.second, n->loc);
        }else if(n->op == Tok_Or || n->op == Tok_And){
            addConstraint(n->lval->getType(), AnType::getBool(), n->loc);
            addConstraint(n->rval->getType(), AnType::getBool(), n->loc);
            addConstraint(n->getType(), AnType::getBool(), n->loc);
        }else if(n->op == Tok_Is || n->op == Tok_Isnt || n->op == '=' || n->op == Tok_NotEq){
            addConstraint(n->lval->getType(), n->rval->getType(), n->loc);
            addConstraint(n->getType(), AnType::getBool(), n->loc);
        }else if(n->op == Tok_Range){
            auto range = getRangeTraitType(module);
            addTypeClassConstraint(range, n->loc);
            addConstraint(n->lval->getType(), range->typeArgs[0], n->loc);
            addConstraint(n->rval->getType(), range->typeArgs[1], n->loc);
        }else if(n->op == Tok_In){
            auto collection_elemTy = getCollectionOpTraitType(module, n->op);
            auto collection = collection_elemTy.first;

            addTypeClassConstraint(collection, n->loc);
            addConstraint(n->lval->getType(), collection_elemTy.second, n->loc);
            addConstraint(n->rval->getType(), collection->typeArgs[0], n->loc);
            addConstraint(n->getType(), AnType::getBool(), n->loc);
        }
    }

    void ConstraintFindingVisitor::visit(BlockNode *n){
        n->block->accept(*this);
    }

    void ConstraintFindingVisitor::visit(RetNode *n){
        n->expr->accept(*this);
    }

    void ConstraintFindingVisitor::visit(ImportNode *n){}

    void ConstraintFindingVisitor::visit(IfNode *n){
        n->condition->accept(*this);
        n->thenN->accept(*this);
        addConstraint(n->condition->getType(), AnType::getBool(), n->loc);
        if(n->elseN){
            n->elseN->accept(*this);
            addConstraint(n->thenN->getType(), n->elseN->getType(), n->loc);
            addConstraint(n->thenN->getType(), n->getType(), n->loc);
        }
    }

    void ConstraintFindingVisitor::visit(NamedValNode *n){}

    void ConstraintFindingVisitor::visit(VarNode *n){}


    void ConstraintFindingVisitor::visit(VarAssignNode *n){
        n->expr->accept(*this);
        n->ref_expr->accept(*this);
    }

    void ConstraintFindingVisitor::addConstraintsFromTCDecl(FuncDeclNode *fdn, TraitImpl *tr, FuncDeclNode *decl){
        TraitDecl *parent = tr->decl;
        for(size_t i = 0; i < parent->typeArgs.size(); i++){
            addConstraint(parent->typeArgs[i], tr->typeArgs[i], fdn->params->loc);
        }

        NamedValNode *declParam = decl->params.get();
        NamedValNode *fdnParam = fdn->params.get();
        while(declParam){
            addConstraint(declParam->getType(), fdnParam->getType(), fdnParam->loc);
            declParam = (NamedValNode*)declParam->next.get();
            fdnParam = (NamedValNode*)fdnParam->next.get();
        }
    }

    FuncDeclNode* getDecl(string const& name, const TraitDecl *t){
        for(auto &fd : t->funcs){
            if(fd->getName() == name) return fd->getFDN();
        }
        return nullptr;
    }


    //TODO: Hold on to AnType of a DataDeclNode when it is first made
    //      or better sort out nameresolution for type family instances
    AnType* anTypeFromDataDecl(DataDeclNode *n, Module *m){
        AnProductType *data = AnProductType::create(n->name, {}, {});
        data->fields.reserve(n->fields);

        for(auto& n : *n->child){
            auto nvn = static_cast<NamedValNode*>(&n);
            TypeNode *tyn = (TypeNode*)nvn->typeExpr.get();
            auto ty = toAnType(tyn, m);
            data->fields.push_back(ty);
        }
        return data->getAliasedType();
    }


    AnType* findTypeFamilyImpl(ExtNode *en, string const& name, Module *m){
        for(auto &n : *en->methods){
            auto ddn = dynamic_cast<DataDeclNode*>(&n);
            if(ddn && ddn->name == name){
                return anTypeFromDataDecl(ddn, m);
            }
        }
        ASSERT_UNREACHABLE();
    }

    void ConstraintFindingVisitor::visit(ExtNode *n){
        if(n->trait){
            auto tr = n->traitType;
            for(Node &m : *n->methods){
                auto fdn = dynamic_cast<FuncDeclNode*>(&m);
                if(fdn){
                    auto *decl = getDecl(fdn->name, tr->decl);
                    fdn->setType(decl->getType());
                    visit(fdn);
                    // adding the constraints from the trait decl last gives better error messages
                    addConstraintsFromTCDecl(fdn, tr, decl);
                }
            }

            // Add the type family impls later otherwise it errors at type family definition instead
            // of in the function that is inconsistent with this definition
            for(auto &family : tr->decl->typeFamilies){
                auto typeFamilyTypeVar = AnTypeVarType::get("'" + family.name);
                auto typeFamilyImpl = findTypeFamilyImpl(tr->impl, family.name, module);
                addConstraint(typeFamilyTypeVar, typeFamilyImpl, n->loc);
            }
        }
    }

    void ConstraintFindingVisitor::visit(JumpNode *n){
        n->expr->accept(*this);
        addConstraint(n->expr->getType(), AnType::getI32(), n->loc);
    }

    void ConstraintFindingVisitor::visit(WhileNode *n){
        n->condition->accept(*this);
        n->child->accept(*this);
        addConstraint(n->condition->getType(), AnType::getBool(), n->loc);
    }

    void ConstraintFindingVisitor::visit(ForNode *n){
        n->range->accept(*this);
        n->pattern->accept(*this);
        n->child->accept(*this);
    }

    void ConstraintFindingVisitor::handleTuplePattern(parser::MatchNode *n,
                parser::TupleNode *pat, AnType *expectedType, Pattern &patChecker){

        auto fieldTys = vecOf<AnType*>(pat->exprs.size());
        for(size_t i = 0; i < pat->exprs.size(); i++){
            fieldTys.push_back(nextTypeVar());
        }
        auto tupTy = AnType::getTupleOf(fieldTys);
        addConstraint(tupTy, expectedType, pat->loc);
        patChecker.overwrite(Pattern::fromTuple(fieldTys), pat->loc);

        for(size_t i = 0; i < pat->exprs.size(); i++){
            handlePattern(n, pat->exprs[i].get(), fieldTys[i], patChecker.getChild(i));
        }
    }

    void ConstraintFindingVisitor::handleUnionVariantPattern(parser::MatchNode *n,
                parser::TypeCastNode *pat, AnType *expectedType, Pattern &patChecker){

        addConstraint(pat->getType(), expectedType, pat->loc);
        auto sumType = try_cast<AnSumType>(pat->getType());
        auto variantType = try_cast<AnProductType>(pat->typeExpr->getType());

        patChecker.overwrite(Pattern::fromSumType(sumType), pat->loc);
        auto agg = variantType->getVariantWithoutTag();
        Pattern& child = patChecker.getChild(sumType->getTagVal(variantType->name));

        // auto-unwrap 1-element tuples to allow Some n instead of forcing Some (n,)
        bool shouldUnwrap = agg->extTys.size() == 1;
        if(shouldUnwrap){
            handlePattern(n, pat->rval.get(), agg->extTys[0], child.getChild(0));
        }else{
            handlePattern(n, pat->rval.get(), agg, child);
        }
    }

    void ConstraintFindingVisitor::handlePattern(MatchNode *n, Node *pattern, AnType *expectedType, Pattern &patChecker){
        if(TupleNode *tn = dynamic_cast<TupleNode*>(pattern)){
            handleTuplePattern(n, tn, expectedType, patChecker);

        }else if(TypeCastNode *tcn = dynamic_cast<TypeCastNode*>(pattern)){
            handleUnionVariantPattern(n, tcn, expectedType, patChecker);

        }else if(TypeNode *tn = dynamic_cast<TypeNode*>(pattern)){
            auto sumType = try_cast<AnSumType>(tn->getType());
            patChecker.overwrite(Pattern::fromSumType(sumType), tcn->loc);
            auto idx = sumType->getTagVal(tn->typeName);
            patChecker.getChild(idx).setMatched();
            addConstraint(tn->getType(), expectedType, pattern->loc);

        }else if(VarNode *vn = dynamic_cast<VarNode*>(pattern)){
            addConstraint(expectedType, vn->getType(), pattern->loc);
            patChecker.setMatched();

        }else if(IntLitNode *iln = dynamic_cast<IntLitNode*>(pattern)){
            auto ty = AnType::getPrimitive(iln->typeTag);
            patChecker.overwrite(Pattern::fromType(ty), iln->loc);
            addConstraint(ty, expectedType, pattern->loc);

        }else if(FltLitNode *fln = dynamic_cast<FltLitNode*>(pattern)){
            auto ty = AnType::getPrimitive(fln->typeTag);
            patChecker.overwrite(Pattern::fromType(ty), fln->loc);
            addConstraint(ty, expectedType, pattern->loc);

        }else if(dynamic_cast<StrLitNode*>(pattern)){
            auto str = module->lookupType("Str");
            patChecker.overwrite(Pattern::fromType(str), pattern->loc);
            addConstraint(str, expectedType, pattern->loc);

        }else{
            error("Invalid pattern syntax", pattern->loc);
        }
    }

    void ConstraintFindingVisitor::visit(MatchNode *n){
        n->expr->accept(*this);
        AnType *firstBranchTy = nullptr;
        Pattern pattern = Pattern::getFillerPattern();

        for(auto &b : n->branches){
            handlePattern(n, b->pattern.get(), n->expr->getType(), pattern);
            b->branch->accept(*this);
            if(firstBranchTy){
                addConstraint(firstBranchTy, b->branch->getType(), b->branch->loc);
            }else{
                firstBranchTy = b->branch->getType();
            }
        }
        if(firstBranchTy){
            addConstraint(firstBranchTy, n->getType(), n->loc);
        }
        if(!pattern.irrefutable()){
            error("Match is not exhaustive, " + pattern.constructMissedCase() + " is not matched", n->loc);
        }
    }

    void ConstraintFindingVisitor::visit(MatchBranchNode *n){}

    void ConstraintFindingVisitor::visit(FuncDeclNode *n){
        if(n->child){
            n->child->accept(*this);

            auto fnty = try_cast<AnFunctionType>(n->getType());
            if(fnty->retTy->typeTag != TT_Unit)
                addConstraint(fnty->retTy, n->child->getType(), n->loc);
        }
    }

    void ConstraintFindingVisitor::visit(DataDeclNode *n){}

    void ConstraintFindingVisitor::visit(TraitNode *n){}
}
