#include "constraintfindingvisitor.h"
#include "unification.h"
#include "antype.h"
#include "funcdecl.h"
#include "compiler.h"
#include "types.h"
#include "util.h"

using namespace std;

namespace ante {
    using namespace parser;

    UnificationList ConstraintFindingVisitor::getConstraints() const {
        return constraints;
    }

    template<typename T, typename F,
        typename U = typename std::decay<typename std::result_of<F&(typename std::vector<T>::const_reference)>::type>::type>
    std::vector<U> fnMap(std::vector<T> const& vec, F f){
        std::vector<U> result;
        result.reserve(vec.size());
        for(const auto& elem : vec){
            result.emplace_back(f(elem));
        }
        return result;
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
            auto tArgs = fnMap(pt->typeArgs, handleExt);
            return AnProductType::getOrCreateVariant(pt, pt->fields, tArgs);

        }else if(auto st = try_cast<AnSumType>(t)){
            auto tArgs = fnMap(st->typeArgs, handleExt);
            return AnSumType::getOrCreateVariant(st, st->tags, tArgs);

        }else if(auto tt = try_cast<AnTraitType>(t)){
            auto tArgs = fnMap(tt->typeArgs, handleExt);
            auto self = handleTypeClassConstraints(tt->selfType, loc);

            constraints.emplace_back(tt, loc);
            return self;

        }else if(auto fn = try_cast<AnFunctionType>(t)){
            auto params = fnMap(fn->extTys, handleExt);
            auto retty = handleTypeClassConstraints(fn->retTy, loc);
            auto tcc = fnMap(fn->typeClassConstraints, handleTcExt);
            return AnFunctionType::get(retty, params, tcc);

        }else if(auto tup = try_cast<AnAggregateType>(t)){
            return AnAggregateType::get(tup->typeTag, fnMap(tup->extTys, handleExt));
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
            tryTo([&]{ n->accept(v); });
        }
    }

    /** Annotate all nodes with placeholder types */
    void ConstraintFindingVisitor::visit(RootNode *n){
        acceptAll(*this, n->imports);
        acceptAll(*this, n->types);
        acceptAll(*this, n->traits);
        acceptAll(*this, n->extensions);
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

    AnType* getUnOpTraitType(int op){
        AnTraitType *parent;
        switch(op){
            case '@': parent = AnTraitType::get("Deref"); break;
            case '-': parent = AnTraitType::get("Neg"); break;
            case Tok_Not: parent = AnTraitType::get("Not"); break;
            default:
                cerr << "getUnOpTraitType: unknown op '" << (char)op << "' (" << (int)op << ") given.  ";
                exit(1);
        }
        if(!parent){
            cerr << "getUnOpTraitType: numeric trait for op '" << (char)op << "' (" << (int)op << ") not found.  The stdlib may not have been imported properly.\n";
            exit(1);
        }
        return AnTraitType::getOrCreateVariant(parent, nextTypeVar(), {});
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
            trait = getUnOpTraitType(n->op);
            addConstraint(n->getType(), trait, n->loc);
            addConstraint(n->rval->getType(), trait, n->loc);
            break;
        case Tok_Not:
            trait = getUnOpTraitType(n->op);
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

    AnType* getOpTraitType(int op){
        AnTraitType *parent;
        switch(op){
            case '+': parent = AnTraitType::get("Add"); break;
            case '-': parent = AnTraitType::get("Sub"); break;
            case '*': parent = AnTraitType::get("Mul"); break;
            case '/': parent = AnTraitType::get("Div"); break;
            case '%': parent = AnTraitType::get("Mod"); break;
            case '^': parent = AnTraitType::get("Pow"); break;
            case '<': parent = AnTraitType::get("Cmp"); break;
            case '>': parent = AnTraitType::get("Cmp"); break;
            case Tok_GrtrEq: parent = AnTraitType::get("Cmp"); break;
            case Tok_LesrEq: parent = AnTraitType::get("Cmp"); break;
            case '=': parent = AnTraitType::get("Eq"); break;
            case Tok_NotEq: parent = AnTraitType::get("Eq"); break;
            default:
                cerr << "getOpTraitType: unknown op '" << (char)op << "' (" << (int)op << ") given.  ";
                exit(1);
        }
        if(!parent){
            cerr << "getOpTraitType: numeric trait for op '" << (char)op << "' (" << (int)op << ") not found.  The stdlib may not have been imported properly.\n";
            exit(1);
        }
        return AnTraitType::getOrCreateVariant(parent, nextTypeVar(), {});
    }


    pair<AnTraitType*, AnTypeVarType*> getCollectionOpTraitType(int op){
        auto collectionTyVar = nextTypeVar();
        auto elemTy = nextTypeVar();

        if(op != '#' && op != Tok_In){
            cerr << "getCollectionOpTraitType: unknown op '" << (char)op << "' (" << (int)op << ") given.  ";
            exit(1);
        }

        string traitName = op == '#' ? "Extract" : "In";
        AnTraitType *parent = AnTraitType::get(traitName);

        if(!parent){
            cerr << "Cannot find the trait" << traitName << ". The prelude may not have been imported properly.\n";
            exit(1);
        }
        return {AnTraitType::getOrCreateVariant(parent, collectionTyVar, {elemTy}), elemTy};
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
            AnType *numTy = getOpTraitType(n->op);
            addConstraint(n->lval->getType(), numTy, n->loc);
            addConstraint(n->rval->getType(), numTy, n->loc);
            addConstraint(n->getType(), numTy, n->loc);
        }else if(n->op == '<' || n->op == '>' || n->op == Tok_GrtrEq || n->op == Tok_LesrEq){
            AnType *numTy = getOpTraitType(n->op);
            addConstraint(n->lval->getType(), numTy, n->loc);
            addConstraint(n->rval->getType(), numTy, n->loc);
            addConstraint(n->getType(), AnType::getBool(), n->loc);
        }else if(n->op == '#'){
            auto collectionTy_elemTy = getCollectionOpTraitType(n->op);

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
            auto collectionTy_elemTy = getCollectionOpTraitType(n->op);

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

    void ConstraintFindingVisitor::visit(NamedValNode *n){
        if(n->typeExpr)
            n->typeExpr->accept(*this);
    }

    void ConstraintFindingVisitor::visit(VarNode *n){}


    void ConstraintFindingVisitor::visit(VarAssignNode *n){
        n->expr->accept(*this);
        n->ref_expr->accept(*this);
    }

    void ConstraintFindingVisitor::visit(ExtNode *n){
        for(auto *m : *n->methods)
            m->accept(*this);
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

    void ConstraintFindingVisitor::visit(MatchNode *n){
        n->expr->accept(*this);
        for(auto &b : n->branches){
            b->accept(*this);
        }
    }

    void ConstraintFindingVisitor::visit(MatchBranchNode *n){
        //n->pattern->accept(*this);
        n->branch->accept(*this);
    }

    void ConstraintFindingVisitor::visit(FuncDeclNode *n){
        for(auto *p : *n->params){
            p->accept(*this);
        }

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
