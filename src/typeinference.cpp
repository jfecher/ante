#include "constraintfindingvisitor.h"
#include "typeinference.h"
#include "unification.h"
#include "compiler.h"
#include "antype.h"
#include "module.h"
#include "types.h"
#include "trait.h"
#include "util.h"

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

        auto lastType = AnType::getUnit();
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
        auto strty = module->lookupType("Str");
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
            n->setType(AnType::getUnit());
        else
            n->setType(AnTupleType::get(types));
    }

    void TypeInferenceVisitor::visit(ModNode *n){
        if(n->expr)
            n->expr->accept(*this);
        n->setType(nextTypeVar());
    }

    /**
     * If the type T is a variant, return its parent Sum type
     * Otherwise, return Type T
     */
    void TypeInferenceVisitor::visit(TypeNode *n){
        auto repl = toAnType(n, module);
        auto variant = try_cast<AnProductType>(repl);
        if(variant && variant->parentUnionType){
            n->setType(copyWithNewTypeVars(variant->parentUnionType));
            return;
        }

        //Type 't
        auto type = try_cast<AnProductType>(module->lookupType("Type"));
        if(!type || type->typeArgs.size() != 1){
            ante::error("type `Type 't` in the prelude was redefined or removed sometime before translation of this type", n->loc);
        }

        auto tvar = type->typeArgs[0];
        auto type_n = applySubstitutions({{tvar, repl}}, type);
        n->setType(type_n);
    }

    void TypeInferenceVisitor::visit(TypeCastNode *n){
        auto ty = copyWithNewTypeVars(toAnType(n->typeExpr.get(), module));
        n->typeExpr->setType(ty);

        n->rval->accept(*this);

        auto variant = try_cast<AnProductType>(ty);
        if(variant && variant->parentUnionType){
            n->setType(variant->parentUnionType);
        }else{
            n->setType(ty);
        }
    }

    void TypeInferenceVisitor::visit(UnOpNode *n){
        n->rval->accept(*this);
        n->setType(nextTypeVar());
    }

    void TypeInferenceVisitor::visit(SeqNode *n){
        auto lastType = AnType::getUnit();
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
            auto argsTup = try_cast<AnTupleType>(args);
            vector<AnType*> params;
            params.reserve(argsTup->fields.size());
            for(size_t i = 0; i < argsTup->fields.size(); i++)
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
        if(n->getType()) return;

        // TODO: is this still necessary?
        if(n->op == '.'){
            if(dynamic_cast<TypeNode*>(n->lval.get())){
                n->rval->accept(*this);
                n->setType(n->rval->getType());
                return;
            }
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
        n->setType(AnType::getUnit());
    }

    void TypeInferenceVisitor::visit(IfNode *n){
        n->condition->accept(*this);
        n->thenN->accept(*this);
        if(n->elseN){
            n->elseN->accept(*this);
            n->setType(nextTypeVar());
        }else{
            n->setType(AnType::getUnit());
        }
    }

    void TypeInferenceVisitor::visit(NamedValNode *n){
        auto ty = toAnType((TypeNode*)n->typeExpr.get(), module);
        n->typeExpr->setType(ty);
        n->setType(ty);
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

        // n->ref_expr->setType(n->expr->getType());
        n->setType(AnType::getUnit());
    }

    vector<TraitImpl*> getAllTcConstraints(AnFunctionType *fn, UnificationList const& constraints,
            Substitutions const& substitutions){

        auto tcConstraints = fn->typeClassConstraints;
        for(auto &c : constraints){
            if(!c.isEqConstraint()){
                auto resolved = applySubstitutions(substitutions, c.asTypeClassConstraint());
                tcConstraints.push_back(resolved);
            }
        }
        return tcConstraints;
    }

    vector<TraitImpl*> toTraitTypeVec(std::unique_ptr<TypeNode> const& tn, Module *module){
        vector<TraitImpl*> ret;
        for(Node &n : *tn){
            ret.push_back(toTrait((TypeNode*)&n, module));
        }
        return ret;
    }

    vector<AnType*> setParamTypes(TypeInferenceVisitor &v, NamedValNode *params){
        return collect(*params, [&](const Node &n) {
            auto p = (NamedValNode*)&n;
            v.visit(p);
            return p->getType();
        });
    }


    void fillInFunctionParamsAndBodyTypes(TypeInferenceVisitor &v, FuncDeclNode *n){
        auto paramTypes = setParamTypes(v, n->params.get());

        auto typeClassConstraints = toTraitTypeVec(n->typeClassConstraints, v.module);
        AnType *retTy = n->returnType ? toAnType(n->returnType.get(), v.module) : nextTypeVar();
        n->setType(AnFunctionType::get(retTy, paramTypes, typeClassConstraints));

        if(n->child){
            n->child->accept(v);
        }
    }


    void TypeInferenceVisitor::visit(ExtNode *n){
        if(n->trait){
            for(Node &m : *n->methods){
                FuncDeclNode *fdn = dynamic_cast<FuncDeclNode*>(&m);
                if(fdn) fillInFunctionParamsAndBodyTypes(*this, fdn);
            }
            n->setType(AnType::getUnit());
            ConstraintFindingVisitor step2{module};
            step2.visit(n);
            auto constraints = step2.getConstraints();
            auto substitutions = unify(constraints);
            if(!substitutions.empty()){
                SubstitutingVisitor::substituteIntoAst(n, substitutions, module);
            }
        }else{
            for(Node &m : *n->methods){
                m.accept(*this);
            }
        }
    }

    void TypeInferenceVisitor::visit(JumpNode *n){
        n->expr->accept(*this);
        n->setType(AnType::getUnit());
    }

    void TypeInferenceVisitor::visit(WhileNode *n){
        n->condition->accept(*this);
        n->child->accept(*this);
        n->setType(AnType::getUnit());
    }

    void TypeInferenceVisitor::visit(ForNode *n){
        n->range->accept(*this);
        n->pattern->accept(*this);
        n->child->accept(*this);
        n->setType(AnType::getUnit());
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

    void checkTraitImpls(Module *m, AnFunctionType *f, LOC_TY loc){
        llvm::StringMap<const AnTypeVarType*> map;
        getAllContainedTypeVarsHelper(f->retTy, map);
        for(auto *paramTy : f->paramTys){
            getAllContainedTypeVarsHelper(paramTy, map);
        }

        for(TraitImpl *trait : f->typeClassConstraints){
            for(auto *ty : trait->typeArgs){
                if(hasTypeVarNotInMap(ty, map)){
                    std::cerr << "in fntype " << anTypeToColoredStr(f) << " trait "
                        << traitToColoredStr(trait) << " has " << anTypeToColoredStr(ty) << " not in function signature\n";
                }
            }
        }
    }


    void TypeInferenceVisitor::visit(FuncDeclNode *n){
        if(n->getType())
            return;

        fillInFunctionParamsAndBodyTypes(*this, n);

        // finish inference for functions early
        ConstraintFindingVisitor step2{this->module};
        tryTo([&]{
            n->accept(step2);
            auto constraints = step2.getConstraints();
            auto substitutions = unify(constraints);
            if(!substitutions.empty()){
                // apply typeclass constraints to function before substitution.
                // it may save some time for non-generic functions to apply them afterward separately.
                auto fnTy = try_cast<AnFunctionType>(n->getType());
                auto tcConstraints = getAllTcConstraints(fnTy, constraints, substitutions);
                auto newFnTy = AnFunctionType::get(fnTy->retTy, fnTy->paramTys, tcConstraints,
                        fnTy->typeTag == TT_MetaFunction);

                newFnTy = cleanTypeClassConstraints(newFnTy);
                n->setType(newFnTy);

                SubstitutingVisitor::substituteIntoAst(n, substitutions, this->module);
            }
        });
    }

    void TypeInferenceVisitor::visit(DataDeclNode *n){
        n->setType(AnType::getUnit());
    }


    void TypeInferenceVisitor::visit(TraitNode *n){
        n->setType(AnType::getUnit());
        for(Node &node : *n->child){
            node.accept(*this);

            auto fdn = dynamic_cast<FuncDeclNode*>(&node);
            if (fdn) {
                auto fdty = try_cast<AnFunctionType>(fdn->getType());
                auto traits = fdty->typeClassConstraints; // copy the vec so the old one isn't pushed to

                //TODO synchronize this fresh trait with the actual trait of the TraitNode from name resolution?
                traits.push_back(module->createTraitImplFromDecl(n->name));
                node.setType(AnFunctionType::get(fdty->retTy, fdty->paramTys, traits));
            }
        }
    }
}
