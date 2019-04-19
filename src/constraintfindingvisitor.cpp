#include "constraintfindingvisitor.h"
#include "unification.h"
#include "funcdecl.h"
#include "compiler.h"
#include "pattern.h"
#include "antype.h"
#include "module.h"
#include "types.h"
#include "util.h"

using namespace std;

namespace ante {
    using namespace parser;

    UnificationList ConstraintFindingVisitor::getConstraints() const {
        return constraints;
    }

    AnType* ConstraintFindingVisitor::handleTypeClassConstraints(AnType *t, LOC_TY const& loc){
        if(!t->isGeneric)
            return t;

        auto handleExt = [&](AnType* ext){
            return handleTypeClassConstraints(ext, loc);
        };
        auto handleTcExt = [&](AnTraitType* ext){
            return (AnTraitType*)handleTypeClassConstraints(ext, loc);
        };

        if(auto ptr = try_cast<AnPtrType>(t)){
            return AnPtrType::get(handleTypeClassConstraints(ptr->extTy, loc));

        }else if(auto arr = try_cast<AnArrayType>(t)){
            return AnArrayType::get(handleTypeClassConstraints(arr->extTy, loc), arr->len);

        }else if(auto pt = try_cast<AnProductType>(t)){
            auto tArgs = applyToAll(pt->typeArgs, handleExt);
            return AnProductType::createVariant(pt, pt->fields, tArgs);

        }else if(auto st = try_cast<AnSumType>(t)){
            auto tArgs = applyToAll(st->typeArgs, handleExt);
            return AnSumType::createVariant(st, st->tags, tArgs);

        }else if(auto tt = try_cast<AnTraitType>(t)){
            auto tArgs = applyToAll(tt->typeArgs, handleExt);
            auto self = handleTypeClassConstraints(tt->selfType, loc);

            constraints.emplace_back(tt, loc);
            return self;

        }else if(auto fn = try_cast<AnFunctionType>(t)){
            auto params = applyToAll(fn->extTys, handleExt);
            auto retty = handleTypeClassConstraints(fn->retTy, loc);
            auto tcc = applyToAll(fn->typeClassConstraints, handleTcExt);
            return AnFunctionType::get(retty, params, tcc);

        }else if(auto tup = try_cast<AnAggregateType>(t)){
            return AnAggregateType::get(tup->typeTag, applyToAll(tup->extTys, handleExt));
        }else{
            return t;
        }
    }

    void ConstraintFindingVisitor::addConstraint(AnType *a, AnType *b, LOC_TY &loc){
        a = handleTypeClassConstraints(a, loc);
        b = handleTypeClassConstraints(b, loc);
        constraints.emplace_back(a, b, loc);
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
        }
    }

    AnType* getUnOpTraitType(Module *module, int op){
        AnType *parent;
        switch(op){
            case '@': parent = module->lookupType("Deref"); break;
            case '-': parent = module->lookupType("Neg"); break;
            case Tok_Not: parent = module->lookupType("Not"); break;
            default:
                cerr << "getUnOpTraitType: unknown op '" << (char)op << "' (" << (int)op << ") given.  ";
                exit(1);
        }
        if(!parent){
            cerr << "getUnOpTraitType: numeric trait for op '" << (char)op << "' (" << (int)op << ") not found.  The stdlib may not have been imported properly.\n";
            exit(1);
        }
        return AnTraitType::createVariant(static_cast<AnTraitType*>(parent), nextTypeVar(), {});
    }

    void ConstraintFindingVisitor::visit(UnOpNode *n){
        n->rval->accept(*this);
        auto tv = nextTypeVar();
        AnType *trait;
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
            addConstraint(n->getType(), trait, n->loc);
            addConstraint(n->rval->getType(), trait, n->loc);
            break;
        case Tok_Not:
            trait = getUnOpTraitType(module, n->op);
            addConstraint(n->rval->getType(), trait, n->loc);
            addConstraint(n->getType(), trait, n->loc);
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

    AnType* getOpTraitType(Module *module, int op){
        AnType *parent;
        switch(op){
            case '+': parent = module->lookupType("Add"); break;
            case '-': parent = module->lookupType("Sub"); break;
            case '*': parent = module->lookupType("Mul"); break;
            case '/': parent = module->lookupType("Div"); break;
            case '%': parent = module->lookupType("Mod"); break;
            case '^': parent = module->lookupType("Pow"); break;
            case '<': parent = module->lookupType("Cmp"); break;
            case '>': parent = module->lookupType("Cmp"); break;
            case Tok_GrtrEq: parent = module->lookupType("Cmp"); break;
            case Tok_LesrEq: parent = module->lookupType("Cmp"); break;
            case '=': parent = module->lookupType("Eq"); break;
            case Tok_NotEq: parent = module->lookupType("Eq"); break;
            default:
                cerr << "getOpTraitType: unknown op '" << (char)op << "' (" << (int)op << ") given.  ";
                exit(1);
        }
        if(!parent){
            cerr << "getOpTraitType: numeric trait for op '" << (char)op << "' (" << (int)op << ") not found.  The stdlib may not have been imported properly.\n";
            exit(1);
        }
        return AnTraitType::createVariant(static_cast<AnTraitType*>(parent), nextTypeVar(), {});
    }


    pair<AnTraitType*, AnTypeVarType*> getCollectionOpTraitType(Module *module, int op){
        auto collectionTyVar = nextTypeVar();
        auto elemTy = nextTypeVar();

        if(op != '#' && op != Tok_In){
            cerr << "getCollectionOpTraitType: unknown op '" << (char)op << "' (" << (int)op << ") given.  ";
            exit(1);
        }

        string traitName = op == '#' ? "Extract" : "In";
        AnTraitType *parent = try_cast<AnTraitType>(module->lookupType(traitName));

        if(!parent){
            cerr << "Cannot find the trait" << traitName << ". The prelude may not have been imported properly.\n";
            exit(1);
        }
        return {AnTraitType::createVariant(parent, collectionTyVar, {elemTy}), elemTy};
    }


    void ConstraintFindingVisitor::visit(BinOpNode *n){
        n->lval->accept(*this);
        n->rval->accept(*this);

        if(n->op == '('){
            auto fnty = try_cast<AnFunctionType>(n->lval->getType());
            if(!fnty){
                auto args = try_cast<AnAggregateType>(n->rval->getType());
                if (!args) args = AnAggregateType::get(TT_Tuple, {n->rval->getType()});
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
                if (!args) args = AnAggregateType::get(TT_Tuple, { n->rval->getType() });

                size_t paramc = fnty->extTys.size();
                size_t argc = args->extTys.size();

                if(argc != paramc && !fnty->isVarArgs()){
                    // If this is not a single () being applied to a no-parameter function
                    if(!(argc == 1 && paramc == 0 && args->extTys[0]->typeTag == TT_Void)){
                        string weregiven = argc == 1 ? " was given" : " were given";
                        error("Function takes " + to_string(paramc)
                                + " argument(s) but " + to_string(argc)
                                + weregiven, n->lval->loc);
                    }
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
                    addConstraint(AnAggregateType::get(TT_Tuple, varargs), fnty->extTys.back(), n->loc);
                }
                addConstraint(n->getType(), fnty->retTy, n->loc);
            }
        }else if(n->op == '+' || n->op == '-' || n->op == '*' || n->op == '/' || n->op == '%' || n->op == '^'){
            AnType *numTy = getOpTraitType(module, n->op);
            addConstraint(n->lval->getType(), numTy, n->loc);
            addConstraint(n->rval->getType(), numTy, n->loc);
            addConstraint(n->getType(), numTy, n->loc);
        }else if(n->op == '<' || n->op == '>' || n->op == Tok_GrtrEq || n->op == Tok_LesrEq){
            AnType *numTy = getOpTraitType(module, n->op);
            addConstraint(n->lval->getType(), numTy, n->loc);
            addConstraint(n->rval->getType(), numTy, n->loc);
            addConstraint(n->getType(), AnType::getBool(), n->loc);
        }else if(n->op == '#'){
            auto collectionTy_elemTy = getCollectionOpTraitType(module, n->op);

            addConstraint(n->lval->getType(), collectionTy_elemTy.first, n->loc);
            addConstraint(n->rval->getType(), AnType::getUsz(), n->loc);
            addConstraint(n->getType(), collectionTy_elemTy.second, n->loc);
        }else if(n->op == Tok_Or || n->op == Tok_And){
            addConstraint(n->lval->getType(), AnType::getBool(), n->loc);
            addConstraint(n->rval->getType(), AnType::getBool(), n->loc);
            addConstraint(n->getType(), AnType::getBool(), n->loc);
        }else if(n->op == Tok_Is || n->op == Tok_Isnt || n->op == '=' || n->op == Tok_NotEq){
            addConstraint(n->lval->getType(), n->rval->getType(), n->loc);
            addConstraint(n->getType(), AnType::getBool(), n->loc);
        }else if(n->op == Tok_Range){
            addConstraint(n->lval->getType(), AnType::getI32(), n->loc);
            addConstraint(n->rval->getType(), AnType::getI32(), n->loc);
        }else if(n->op == Tok_In){
            auto collectionTy_elemTy = getCollectionOpTraitType(module, n->op);

            addConstraint(n->lval->getType(), collectionTy_elemTy.second, n->loc);
            addConstraint(n->rval->getType(), collectionTy_elemTy.first, n->loc);
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

    void ConstraintFindingVisitor::addConstraintsFromTCDecl(FuncDeclNode *fdn, AnTraitType *tr, FuncDeclNode *decl){
        auto parent = try_cast<AnTraitType>(tr->unboundType);
        addConstraint(parent->selfType, tr->selfType, fdn->params->loc);
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

    FuncDeclNode* getDecl(string const& name, const Trait *t){
        for(auto &fd : t->funcs){
            if(fd->getName() == name) return fd->getFDN();
        }
        return nullptr;
    }

    void ConstraintFindingVisitor::visit(ExtNode *n){
        if(n->trait){
            auto tr = try_cast<AnTraitType>(toAnType(n->trait.get(), module));
            for(Node &m : *n->methods){
                auto fdn = dynamic_cast<FuncDeclNode*>(&m);
                if(fdn){
                    auto *decl = getDecl(fdn->name, tr->trait);
                    fdn->setType(decl->getType());
                    visit(fdn);
                    // adding the constraints from the trait decl last gives better error messages
                    addConstraintsFromTCDecl(fdn, tr, decl);
                }
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
        auto tupTy = AnAggregateType::get(TT_Tuple, fieldTys);
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
            if(fnty->retTy->typeTag != TT_Void)
                addConstraint(fnty->retTy, n->child->getType(), n->loc);
        }
    }

    void ConstraintFindingVisitor::visit(DataDeclNode *n){}

    void ConstraintFindingVisitor::visit(TraitNode *n){}
}
