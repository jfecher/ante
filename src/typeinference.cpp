#include "typeinference.h"
#include "antype.h"
#include "compiler.h"
#include "types.h"

#include "constraintfindingvisitor.h"
#include "unification.h"

using namespace std;

namespace ante {
    using namespace parser;

    /** Annotate all nodes with placeholder types */
    void TypeInferenceVisitor::visit(RootNode *n){
        for(auto &m : n->imports)
            m->accept(*this);
        for(auto &m : n->types)
            m->accept(*this);
        for(auto &m : n->traits)
            m->accept(*this);
        for(auto &m : n->extensions)
            m->accept(*this);
        for(auto &m : n->funcs)
            m->accept(*this);

        auto lastType = AnType::getVoid();
        for(auto &m : n->main){
            m->accept(*this);
            lastType = m->getType();
        }
        n->setType(lastType);
    }

    void TypeInferenceVisitor::visit(IntLitNode *n){
        n->setType(AnType::getPrimitive(n->typeTag));
    }

    void TypeInferenceVisitor::visit(FltLitNode *n){
        n->setType(AnType::getPrimitive(n->typeTag));
    }

    void TypeInferenceVisitor::visit(BoolLitNode *n){
        n->setType(AnType::getBool());
    }

    void TypeInferenceVisitor::visit(StrLitNode *n){
        auto strty = AnDataType::get("Str");
        if(!strty){
            strty = AnProductType::create("Str", {}, {});
        }
        n->setType(strty);
    }

    void TypeInferenceVisitor::visit(CharLitNode *n){
        n->setType(AnType::getPrimitive(TT_C8));
    }

    void TypeInferenceVisitor::visit(ArrayNode *n){
        for(auto &e : n->exprs)
            e->accept(*this);

        auto ty = AnArrayType::get(nextTypeVar(), n->exprs.size());
        n->setType(ty);
    }

    void TypeInferenceVisitor::visit(TupleNode *n){
        auto types = vecOf<AnType*>(n->exprs.size());
        for(auto &e : n->exprs){
            e->accept(*this);
            types.push_back(e->getType());
        }

        if(n->exprs.empty())
            n->setType(AnType::getVoid());
        else
            n->setType(AnType::getAggregate(TT_Tuple, types));
    }

    void TypeInferenceVisitor::visit(ModNode *n){
        if(n->expr)
            n->expr->accept(*this);
        n->setType(nextTypeVar());
    }

    void TypeInferenceVisitor::visit(TypeNode *n){
        //Type 't
        auto type = AnProductType::get("Type");
        if(!type || type->typeArgs.size() != 1){
            ante::error("type `Type 't` in the prelude was redefined or removed sometime before translation of this type", n->loc);
        }

        auto tvar = type->typeArgs[0];
        auto repl = toAnType(n);
        auto type_n = applySubstitutions({{tvar, repl}}, type);
        cout << anTypeToColoredStr(type) << " [" << anTypeToColoredStr(tvar) << " -> " << anTypeToColoredStr(repl) 
            << "] = " << anTypeToColoredStr(type_n) << endl;

        n->setType(type_n);
    }

    void TypeInferenceVisitor::visit(TypeCastNode *n){
        n->typeExpr->accept(*this);
        n->rval->accept(*this);
        n->setType(n->typeExpr->getType());
    }

    void TypeInferenceVisitor::visit(UnOpNode *n){
        n->rval->accept(*this);
        n->setType(nextTypeVar());
    }

    void TypeInferenceVisitor::visit(SeqNode *n){
        auto lastType = AnType::getVoid();
        for(auto &stmt : n->sequence){
            stmt->accept(*this);
            lastType = stmt->getType();
        }
        n->setType(lastType);
    }

    AnFunctionType* unknownFunctionType(Declaration *decl, AnType *args){
        if(decl->tval.type && !try_cast<AnTypeVarType>(decl->tval.type))
            return try_cast<AnFunctionType>(decl->tval.type);

        auto retTy = nextTypeVar();
        if(args->typeTag == TT_Tuple){
            auto argsTup = try_cast<AnAggregateType>(args);
            vector<AnType*> params;
            params.reserve(argsTup->extTys.size());
            for(size_t i = 0; i < argsTup->extTys.size(); i++)
                params.push_back(nextTypeVar());
            decl->tval.type = AnFunctionType::get(retTy, params, {});
        }else{
            auto param = nextTypeVar();
            decl->tval.type = AnFunctionType::get(retTy, {param}, {});
        }
        return try_cast<AnFunctionType>(decl->tval.type);
    }

    void TypeInferenceVisitor::visit(BinOpNode *n){
        // If we have a field access operator, we cannot try to
        // coerce the module name into a type
        if(n->op == '.' && dynamic_cast<TypeNode*>(n->lval.get())){
            return;
        }

        n->lval->accept(*this);
        n->rval->accept(*this);
        n->setType(nextTypeVar());
    }

    void TypeInferenceVisitor::visit(BlockNode *n){
        n->block->accept(*this);
        n->setType(n->block->getType());
    }

    void TypeInferenceVisitor::visit(RetNode *n){
        n->expr->accept(*this);
        n->setType(n->expr->getType());
    }

    void TypeInferenceVisitor::visit(ImportNode *n){
        n->setType(AnType::getVoid());
    }

    void TypeInferenceVisitor::visit(IfNode *n){
        n->condition->accept(*this);
        n->thenN->accept(*this);
        if(n->elseN){
            n->elseN->accept(*this);
            n->setType(nextTypeVar());
        }else{
            n->setType(AnType::getVoid());
        }
    }

    void TypeInferenceVisitor::visit(NamedValNode *n){
        if(n->typeExpr){
            auto ty = toAnType((TypeNode*)n->typeExpr.get());
            n->typeExpr->setType(ty);
            n->setType(ty);
        }else{ // type field is null if this is a variadic parameter ie the '...' in printf: Str a, ... -> i32
            auto tv = nextTypeVar();
            auto va = AnTypeVarType::get(tv->name + "...");
            n->setType(va);
        }
    }

    void TypeInferenceVisitor::visit(VarNode *n){
        auto *decl = n->decl;
        if(!decl->tval.type && decl->isFuncDecl()){
            decl->definition->accept(*this);
        }

        if(!decl->tval.type){
            auto tv = nextTypeVar();
            decl->tval.type = nextTypeVar();
            n->setType(tv);
        }else if(auto *fnty = try_cast<AnFunctionType>(decl->tval.type)){
            n->setType(copyWithNewTypeVars(fnty));
        }else{
            n->setType(decl->tval.type);
        }
    }


    void TypeInferenceVisitor::visit(VarAssignNode *n){
        n->expr->accept(*this);
        n->ref_expr->accept(*this);

        n->ref_expr->setType(n->expr->getType());
        if(n->modifiers.empty()){
            n->setType(AnType::getVoid());
        }else{
            n->setType(n->expr->getType());
        }
    }

    void TypeInferenceVisitor::visit(ExtNode *n){
        for(auto *m : *n->methods)
            m->accept(*this);
        n->setType(AnType::getVoid());
    }

    void TypeInferenceVisitor::visit(JumpNode *n){
        n->expr->accept(*this);
        n->setType(AnType::getVoid());
    }

    void TypeInferenceVisitor::visit(WhileNode *n){
        n->condition->accept(*this);
        n->child->accept(*this);
        n->setType(AnType::getVoid());
    }

    void TypeInferenceVisitor::visit(ForNode *n){
        n->range->accept(*this);
        n->pattern->accept(*this);
        n->child->accept(*this);
        n->setType(AnType::getVoid());
    }

    void TypeInferenceVisitor::visit(MatchNode *n){
        n->expr->accept(*this);
        for(auto &b : n->branches){
            b->accept(*this);
        }
        n->setType(nextTypeVar());
    }

    void TypeInferenceVisitor::visit(MatchBranchNode *n){
        n->pattern->accept(*this);
        n->branch->accept(*this);
        n->setType(n->branch->getType());
    }


    vector<AnTraitType*> getAllTcConstraints(AnFunctionType *fn, UnificationList const& constraints,
            Substitutions const& substitutions){

        auto tcConstraints = fn->typeClassConstraints;
        for(auto &c : constraints){
            if(!c.isEqConstraint()){
                auto resolved = applySubstitutions(substitutions, c.asTypeClassConstraint());
                tcConstraints.push_back((AnTraitType*)resolved);
            }
        }
        return tcConstraints;
    }


    vector<AnTraitType*> toTraitTypeVec(std::unique_ptr<TypeNode> &tn){
        vector<AnTraitType*> ret;
        for(Node *n : *tn){
            ret.push_back((AnTraitType*)toAnType((TypeNode*)n));
        }
        return ret;
    }


    void TypeInferenceVisitor::visit(FuncDeclNode *n){
        if(n->getType())
            return;

        vector<AnType*> paramTypes;
        for(auto *p : *n->params){
            p->accept(*this);
            paramTypes.push_back(p->getType());
        }

        auto typeClassConstraints = toTraitTypeVec(n->typeClassConstraints);
        if(n->returnType){
            n->setType(AnFunctionType::get(toAnType(n->returnType.get()), paramTypes, typeClassConstraints));
        }else{
            n->setType(AnFunctionType::get(nextTypeVar(), paramTypes, typeClassConstraints));
        }

        if(n->child)
            n->child->accept(*this);

        // finish inference for functions early
        ConstraintFindingVisitor step2;
        n->accept(step2);
        auto constraints = step2.getConstraints();
        auto substitutions = unify(constraints);
        SubstitutingVisitor::substituteIntoAst(n, substitutions);

        // apply typeclass constraints to function
        auto fnTy = try_cast<AnFunctionType>(n->getType());
        auto tcConstraints = getAllTcConstraints(fnTy, constraints, substitutions);
        auto newFnTy = AnFunctionType::get(fnTy->retTy, fnTy->extTys, tcConstraints,
                fnTy->typeTag == TT_MetaFunction);
        n->setType(newFnTy);
    }

    void TypeInferenceVisitor::visit(DataDeclNode *n){
        n->setType(AnType::getVoid());
    }

    void TypeInferenceVisitor::visit(TraitNode *n){
        n->setType(AnType::getVoid());
        for(auto *node : *n->child){
            node->accept(*this);
        }
    }
}
