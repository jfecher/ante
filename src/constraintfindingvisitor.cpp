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

    void ConstraintFindingVisitor::addConstraint(AnType *a, AnType *b, LOC_TY &loc, TypeError const& errMsg){
        constraints.emplace_back(a, b, loc, errMsg);
    }

    void ConstraintFindingVisitor::addTypeClassConstraint(TraitImpl *constraint, LOC_TY &loc){
        constraints.emplace_back(constraint, loc, "");
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
            for(size_t i = 1; i < n->exprs.size(); i++){
                n->exprs[i]->accept(*this);
                auto tn = n->exprs[i]->getType();
                addConstraint(t1, tn, n->exprs[i]->loc,
                        "Array element " + to_string(i+1) + " has type $2 which does not match the first element's type of $1");
            }
            addConstraint(arrty->extTy, t1, n->loc,
                    "Expected array's first element type $2 to match the overall array element type $1");
        }else{
            addConstraint(arrty->extTy, AnType::getUnit(), n->loc,
                    "Expected the empty array to be array of $2, but got array of $1");
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
                    auto tnty = tn->exprs[i]->getType();
                    auto vty = variant->fields[i+offset];
                    addConstraint(tnty, vty, tn->exprs[i]->loc,
                            "Expected field " + to_string(i+1) + " of type $1 to be typecasted to the corresponding field type $2 from " + variant->name);
                }
            }else{
                addConstraint(n->rval->getType(), variant->fields[offset], n->rval->loc,
                        "Cannot cast $1 to $2 when trying to cast to " + variant->name);
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
            addConstraint(n->rval->getType(), AnPtrType::get(tv), n->loc,
                    "Expected $1 to be a $2 since it is dereferenced");
            addConstraint(n->getType(), tv, n->loc,
                    "Expected the result of this dereference to be the pointer's element type $2 but got $1");
            break;
        case '&':
            addConstraint(n->getType(), AnPtrType::get(tv), n->loc,
                    "Expected result of & to be a $2 but got $1");
            addConstraint(n->rval->getType(), tv, n->loc, "Error: should never fail, line " + to_string(__LINE__));
            break;
        case '-': //negation
            trait = getUnOpTraitType(module, n->op);
            addTypeClassConstraint(trait, n->loc);
            addConstraint(trait->typeArgs[0], n->rval->getType(), n->loc, "Error: should never fail, line " + to_string(__LINE__));
            addConstraint(n->getType(), n->rval->getType(), n->loc, "Error: should never fail, line " + to_string(__LINE__));
            break;
        case Tok_Not:
            trait = getUnOpTraitType(module, n->op);
            addTypeClassConstraint(trait, n->loc);
            addConstraint(trait->typeArgs[0], n->rval->getType(), n->loc, "Error: should never fail, line " + to_string(__LINE__));
            addConstraint(n->getType(), n->rval->getType(), n->loc, "Error: should never fail, line " + to_string(__LINE__));
            break;
        case Tok_New:
            addConstraint(n->getType(), AnPtrType::get(tv), n->loc,
                    "Expected result of new to be a pointer type but got $1");
            addConstraint(n->rval->getType(), tv, n->loc, "Error: should never fail, line " + to_string(__LINE__));
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
            case Tok_EqEq: parent = module->freshTraitImpl("Eq"); break;
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


    bool invalidNumArguments(AnFunctionType *fnty, AnTupleType *args){
        size_t argc = args->fields.size();
        size_t paramc = fnty->paramTys.size();
        if(argc == paramc)
            return false;

        if(argc == paramc + 1 && args->fields.back()->typeTag == TT_Unit)
            return false;

        if(argc + 1 == paramc && fnty->paramTys.back()->typeTag == TT_Unit)
            return false;

        if(fnty->isVarArgs() && argc >= paramc - 1)
            return false;

        return true;
    }


    void issueInvalidArgCountError(AnFunctionType *fnty, AnTupleType *args, LOC_TY &loc){
        bool isVA = fnty->isVarArgs();
        size_t paramc = fnty->paramTys.size() - (isVA ? 1 : 0);
        size_t argc = args->fields.size();

        string weregiven = argc == 1 ? " was given" : " were given";
        error("Function takes " + to_string(paramc) + (isVA ? "+" : "")
                + " argument" + plural(paramc) + " but " + to_string(argc)
                + weregiven, loc);
    }

    bool ConstraintFindingVisitor::findFieldInTypeList(llvm::StringMap<TypeDecl> const& m, BinOpNode *op, VarNode *rval) {
        for(auto &p : m){
            if(auto *pt = try_cast<AnProductType>(p.second.type)){
                for(size_t i = 0; i < pt->fieldNames.size(); i++){
                    auto &field = pt->fieldNames[i];
                    if(field == rval->name){
                        auto ty = static_cast<AnProductType*>(copyWithNewTypeVars(pt));
                        addConstraint(op->lval->getType(), ty, op->loc,
                                "Expected lval of . operation to be of type $2 but got $1");
                        addConstraint(rval->getType(), ty->fields[i], op->loc,
                                "Expected field '" + rval->name + "' to be of type $2 but got $1");
                        addConstraint(op->getType(), ty->fields[i], op->loc,
                                "Expected result of field access to be the type of the field, $2 but got $1");
                        return true;
                    }
                }
            }
        }
        return false;
    }

    void ConstraintFindingVisitor::searchForField(BinOpNode *op) {
        if(dynamic_cast<TypeNode*>(op->lval.get())){
            // not a field access, just qualified name resolution
            return;
        }

        VarNode *vn = dynamic_cast<VarNode*>(op->rval.get());
        if(!vn){
            error("RHS of . operator must be an identifier", op->rval->loc);
        }

        if(findFieldInTypeList(module->userTypes, op, vn))
            return;

        for(Module *import : module->imports){
            if(findFieldInTypeList(import->userTypes, op, vn))
                return;
        }

        show(vn);
        error("No field named " + vn->name + " found for any type", vn->loc);
    }

    void ConstraintFindingVisitor::fnCallConstraints(BinOpNode *n){
        auto fnty = try_cast<AnFunctionType>(n->lval->getType());
        if(!fnty){
            auto args = try_cast<AnTupleType>(n->rval->getType());
            if (!args) args = AnTupleType::get({n->rval->getType()});
            auto params = vecOf<AnType*>(args->fields.size());

            for(size_t i = 0; i < args->fields.size(); i++){
                auto param = nextTypeVar();
                addConstraint(args->fields[i], param, n->loc, "Error: should never fail, line " + to_string(__LINE__));
                params.push_back(param);
            }
            auto retTy = nextTypeVar();
            addConstraint(n->getType(), retTy, n->loc, "Error: should never fail, line " + to_string(__LINE__));

            fnty = AnFunctionType::get(retTy, params, {});
            addConstraint(n->lval->getType(), fnty, n->lval->loc,
                    "Expected type of the function to be $2 from the arguments, but actual type is $1");
        }else{
            auto args = try_cast<AnTupleType>(n->rval->getType());
            if (!args) args = AnTupleType::get({ n->rval->getType() });

            if(invalidNumArguments(fnty, args)){
                issueInvalidArgCountError(fnty, args, n->lval->loc);
            }

            auto argtup = static_cast<parser::TupleNode*>(n->rval.get());

            if(!fnty->isVarArgs()){
                for(size_t i = 0; i < fnty->paramTys.size(); i++){
                    addConstraint(args->fields[i], fnty->paramTys[i], argtup->exprs[i]->loc,
                            "Expected parameter type of $2 but got argument of type $1");
                }
            }else{
                size_t i = 0;
                for(; i < fnty->paramTys.size() - 1; i++){
                    addConstraint(args->fields[i], fnty->paramTys[i], argtup->exprs[i]->loc,
                            "Expected parameter type of $2 but got argument of type $1");
                }

                // typecheck var args as a tuple of additional arguments, though they should always be
                // matched against a typevar anyway so these constraints should never fail.
                vector<AnType*> varargs;
                for(; i < args->fields.size(); i++){
                    varargs.push_back(args->fields[i]);
                }
                addConstraint(AnTupleType::get(varargs), fnty->paramTys.back(), n->loc,
                        "Error: should never fail, line " + to_string(__LINE__));
            }
            addConstraint(n->getType(), fnty->retTy, n->loc,
                    "Expected result of function call to match the function return type but got $1 and $2 respectively");
        }
    }


    void ConstraintFindingVisitor::visit(BinOpNode *n){
        n->lval->accept(*this);
        n->rval->accept(*this);

        if(n->op == '('){
            fnCallConstraints(n);
        }else if(n->op == '+' || n->op == '-' || n->op == '*' || n->op == '/' || n->op == '%' || n->op == '^'){
            TraitImpl *num = getOpTraitType(module, n->op);
            addTypeClassConstraint(num, n->loc);
            addConstraint(n->lval->getType(), num->typeArgs[0], n->loc,
                    "Operand types of '" + Lexer::getTokStr(n->op) + "' should match, but are $1 and $2 respectively");
            addConstraint(n->rval->getType(), num->typeArgs[0], n->loc,
                    "Operand types of '" + Lexer::getTokStr(n->op) + "' should match, but are $2 and $1 respectively");
            addConstraint(n->getType(), num->typeArgs[0], n->loc,
                    "Expected return type of operator to match the operand type $2, but found $1 instead");
        }else if(n->op == '<' || n->op == '>' || n->op == Tok_GrtrEq || n->op == Tok_LesrEq){
            TraitImpl *num = getOpTraitType(module, n->op);
            addTypeClassConstraint(num, n->loc);
            addConstraint(n->lval->getType(), num->typeArgs[0], n->loc,
                    "Operand types of '" + Lexer::getTokStr(n->op) + "' should match, but are $1 and $2 respectively");
            addConstraint(n->rval->getType(), num->typeArgs[0], n->loc,
                    "Operand types of '" + Lexer::getTokStr(n->op) + "' should match, but are $2 and $1 respectively");
            addConstraint(n->getType(), AnType::getBool(), n->loc,
                    "Expected return type of logical operator to be $2, but found $1 instead");
        }else if(n->op == '#'){
            auto collection_elemTy = getCollectionOpTraitType(module, n->op);
            auto collection = collection_elemTy.first;

            addConstraint(n->lval->getType(), collection->typeArgs[0], n->loc,
                    "Error: should never fail, line " + to_string(__LINE__));
            addConstraint(n->rval->getType(), AnType::getUsz(), n->loc,
                    "Expected index of subscript operator to be $2 but found $1 instead");
            addConstraint(n->getType(), collection_elemTy.second, n->loc,
                    "Error: should never fail, line " + to_string(__LINE__));
        }else if(n->op == Tok_Or || n->op == Tok_And){
            addConstraint(n->lval->getType(), AnType::getBool(), n->loc,
                    "Left operand of " + Lexer::getTokStr(n->op) + " should always be $2 but here is $1");
            addConstraint(n->rval->getType(), AnType::getBool(), n->loc,
                    "Right operand of " + Lexer::getTokStr(n->op) + " should always be $2 but here is $1");
            addConstraint(n->getType(), AnType::getBool(), n->loc,
                    "Return type of " + Lexer::getTokStr(n->op) + " should always be $2 but here is $1");
        }else if(n->op == Tok_Is || n->op == Tok_Isnt || n->op == Tok_EqEq || n->op == Tok_NotEq){
            addConstraint(n->lval->getType(), n->rval->getType(), n->loc,
                    "Operand types of '" + Lexer::getTokStr(n->op) + "' should match, but are $1 and $2 respectively");
            addConstraint(n->getType(), AnType::getBool(), n->loc,
                    "Return type of " + Lexer::getTokStr(n->op) + " should always be $2 but here is $1");
        }else if(n->op == Tok_Range){
            auto range = getRangeTraitType(module);
            addTypeClassConstraint(range, n->loc);
            addConstraint(n->lval->getType(), range->typeArgs[0], n->loc,
                    "Error: should never fail, line " + to_string(__LINE__));
            addConstraint(n->rval->getType(), range->typeArgs[1], n->loc,
                    "Error: should never fail, line " + to_string(__LINE__));
        }else if(n->op == Tok_In){
            auto collection_elemTy = getCollectionOpTraitType(module, n->op);
            auto collection = collection_elemTy.first;

            addTypeClassConstraint(collection, n->loc);
            addConstraint(n->lval->getType(), collection_elemTy.second, n->loc,
                    "Error: should never fail, line " + to_string(__LINE__));
            addConstraint(n->rval->getType(), collection->typeArgs[0], n->loc,
                    "Error: should never fail, line " + to_string(__LINE__));
            addConstraint(n->getType(), AnType::getBool(), n->loc,
                    "Return type of 'in' should always be $2 but here is $1");
        }else if(n->op == '.'){
            searchForField(n);
        }else if(n->op == Tok_As){
            // intentionally empty
        }else{
            ante::error("Internal compiler error, unrecognized op " + string(1, n->op) + " (" + to_string(n->op) + ")", n->loc);
        }
    }

    void ConstraintFindingVisitor::visit(BlockNode *n){
        n->block->accept(*this);
    }

    void ConstraintFindingVisitor::visit(RetNode *n){
        n->expr->accept(*this);
        addConstraint(n->getType(), functionReturnTypes.top(), n->loc,
                "The type of this explicit return, $1, does not match the type of the function's other returns or return type, $2");
    }

    void ConstraintFindingVisitor::visit(ImportNode *n){}

    void ConstraintFindingVisitor::visit(IfNode *n){
        n->condition->accept(*this);
        n->thenN->accept(*this);
        addConstraint(n->condition->getType(), AnType::getBool(), n->loc,
                "Expected if condition to be a $2 but got $1");
        if(n->elseN){
            n->elseN->accept(*this);
            addConstraint(n->thenN->getType(), n->elseN->getType(), n->loc,
                    "Expected type of then and else branches to match, but got $1 and $2 respectively");
            addConstraint(n->thenN->getType(), n->getType(), n->loc,
                    "Error: should never fail, line " + to_string(__LINE__));
        }
    }

    void ConstraintFindingVisitor::visit(NamedValNode *n){}

    void ConstraintFindingVisitor::visit(VarNode *n){
        auto fnty = try_cast<AnFunctionType>(n->getType());
        if(fnty && !fnty->typeClassConstraints.empty()){
            for(auto constraint : fnty->typeClassConstraints){
                addTypeClassConstraint(constraint, n->loc);
            }
        }
    }


    void ConstraintFindingVisitor::visit(VarAssignNode *n){
        n->expr->accept(*this);
        n->ref_expr->accept(*this);

        AnType *refty = n->expr->getType();
        if(n->hasModifier(Tok_Mut)){
            refty = (AnType*)refty->addModifier(Tok_Mut);
        }
        addConstraint(n->ref_expr->getType(), refty, n->loc,
                "Expected type of variable to match the type of its assignment expression, but got $1 and $2 respectively");
    }

    void ConstraintFindingVisitor::addConstraintsFromTCDecl(FuncDeclNode *fdn, TraitImpl *tr, FuncDeclNode *decl){
        TraitDecl *parent = tr->decl;
        if(parent->typeArgs.size() != tr->typeArgs.size()){
            error("Impl has " + to_string(tr->typeArgs.size()) + " typeargs, but there are "
                + to_string(parent->typeArgs.size()) + " typeargs in " + parent->name + "'s decl", fdn->loc);
        }

        for(size_t i = 0; i < parent->typeArgs.size(); i++){
            addConstraint(parent->typeArgs[i], tr->typeArgs[i], fdn->params->loc,
                    "Error: should never fail, line " + to_string(__LINE__)); //TODO: this line may fail (message may show)
        }

        NamedValNode *declParam = decl->params.get();
        NamedValNode *fdnParam = fdn->params.get();
        while(declParam){
            addConstraint(declParam->getType(), fdnParam->getType(), fdnParam->loc,
                    "Error: should never fail, line " + to_string(__LINE__)); //TODO: this line may fail (message may show)
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
                addConstraint(typeFamilyTypeVar, typeFamilyImpl, n->loc,
                    "Error: should never fail, line " + to_string(__LINE__)); //TODO: this line may fail (message may show)
            }
        }
    }

    void ConstraintFindingVisitor::visit(JumpNode *n){
        n->expr->accept(*this);
        addConstraint(n->expr->getType(), AnType::getI32(), n->loc,
                "Argument of break/continue should always be a $2 but here it is a $1 instead");
    }

    void ConstraintFindingVisitor::visit(WhileNode *n){
        n->condition->accept(*this);
        n->child->accept(*this);
        addConstraint(n->condition->getType(), AnType::getBool(), n->loc,
                "A while loop's condition should always be a $2 but here it is a $1 instead");
    }

    void ConstraintFindingVisitor::visit(ForNode *n){
        n->range->accept(*this);
        n->pattern->accept(*this);
        n->child->accept(*this);

        TraitDecl *decl = module->lookupTraitDecl("Iterable");
        TraitImpl *impl = new TraitImpl(decl, {n->range->getType()});
        n->iterableInstance = impl;
        addTypeClassConstraint(impl, n->loc);
    }

    void ConstraintFindingVisitor::handleTuplePattern(parser::MatchNode *n,
                parser::TupleNode *pat, AnType *expectedType, Pattern &patChecker){

        auto fieldTys = vecOf<AnType*>(pat->exprs.size());
        for(size_t i = 0; i < pat->exprs.size(); i++){
            fieldTys.push_back(nextTypeVar());
        }
        auto tupTy = AnTupleType::get(fieldTys);
        addConstraint(tupTy, expectedType, pat->loc,
                "Expected a $2 here from the tuple destructuring, but found a $1 instead");
        patChecker.overwrite(Pattern::fromTuple(fieldTys), pat->loc);

        for(size_t i = 0; i < pat->exprs.size(); i++){
            handlePattern(n, pat->exprs[i].get(), fieldTys[i], patChecker.getChild(i));
        }
    }

    void ConstraintFindingVisitor::handleUnionVariantPattern(parser::MatchNode *n,
                parser::TypeCastNode *pat, AnType *expectedType, Pattern &patChecker){

        addConstraint(pat->getType(), expectedType, pat->loc,
                "Expected a $2 here from the union variant destructuring, but found a $1 instead");
        auto sumType = try_cast<AnSumType>(pat->getType());
        auto variantType = try_cast<AnProductType>(pat->typeExpr->getType());

        patChecker.overwrite(Pattern::fromSumType(sumType), pat->loc);
        auto agg = variantType->getVariantWithoutTag();
        Pattern& child = patChecker.getChild(sumType->getTagVal(variantType->name));

        // auto-unwrap 1-element tuples to allow Some n instead of forcing Some (n,)
        bool shouldUnwrap = agg->fields.size() == 1;
        if(shouldUnwrap){
            handlePattern(n, pat->rval.get(), agg->fields[0], child.getChild(0));
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
            addConstraint(tn->getType(), expectedType, pattern->loc,
                "Expected a $2 here from the union variant pattern, but found a $1 instead");

        }else if(VarNode *vn = dynamic_cast<VarNode*>(pattern)){
            addConstraint(expectedType, vn->getType(), pattern->loc,
                "Expected the var pattern's type to be $1 from this match pattern but got $2 instead");
            patChecker.setMatched();

        }else if(IntLitNode *iln = dynamic_cast<IntLitNode*>(pattern)){
            auto ty = AnType::getPrimitive(iln->typeTag);
            patChecker.overwrite(Pattern::fromType(ty), iln->loc);
            addConstraint(ty, expectedType, pattern->loc,
                    "Expected this integer to be of type $2 from the match pattern, but got $1 instead");

        }else if(FltLitNode *fln = dynamic_cast<FltLitNode*>(pattern)){
            auto ty = AnType::getPrimitive(fln->typeTag);
            patChecker.overwrite(Pattern::fromType(ty), fln->loc);
            addConstraint(ty, expectedType, pattern->loc,
                    "Expected this float to be of type $2 from the match pattern, but got $1 instead");

        }else if(dynamic_cast<StrLitNode*>(pattern)){
            auto str = module->lookupType("Str");
            patChecker.overwrite(Pattern::fromType(str), pattern->loc);
            addConstraint(str, expectedType, pattern->loc,
                    "Expected this to be of type $2 from the match pattern, but got $1 instead");

        }else{
            error("Invalid pattern syntax", pattern->loc);
        }
    }

    void ConstraintFindingVisitor::visit(MatchNode *n){
        n->expr->accept(*this);
        AnType *firstBranchTy = nullptr;
        Pattern pattern = Pattern::getFillerPattern();

        size_t i = 1;
        for(auto &b : n->branches){
            handlePattern(n, b->pattern.get(), n->expr->getType(), pattern);
            b->branch->accept(*this);
            if(firstBranchTy){
                addConstraint(firstBranchTy, b->branch->getType(), b->branch->loc,
                        "Expected the type of match branch " + to_string(i) + " to match the type of the first branch, but got $2 and $1 respectively");
            }else{
                firstBranchTy = b->branch->getType();
            }
            i++;
        }
        if(firstBranchTy){
            addConstraint(firstBranchTy, n->getType(), n->loc,
                    "Error: should never fail, line " + to_string(__LINE__));
        }
        if(!pattern.irrefutable()){
            error("Match is not exhaustive, " + pattern.constructMissedCase() + " is not matched", n->loc);
        }
    }

    void ConstraintFindingVisitor::visit(MatchBranchNode *n){}

    void ConstraintFindingVisitor::visit(FuncDeclNode *n){
        if(n->child){
            auto fnty = try_cast<AnFunctionType>(n->getType());

            functionReturnTypes.push(fnty->retTy);
            n->child->accept(*this);
            functionReturnTypes.pop();

            if(fnty->retTy->typeTag != TT_Unit)
                addConstraint(fnty->retTy, n->child->getType(), n->loc,
                        "Expected function return type $1 to match the type of its last expression, $2");
        }
    }

    void ConstraintFindingVisitor::visit(DataDeclNode *n){}

    void ConstraintFindingVisitor::visit(TraitNode *n){}
}
