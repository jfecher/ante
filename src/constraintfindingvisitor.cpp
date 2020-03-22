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

    void ConstraintFindingVisitor::addConstraint(AnType *a, AnType *b, LOC_TY &loc, lazy_printer const& errMsg){
        TypeError err{errMsg, loc};
        constraints.emplace_back(a, b, err);
    }

    void ConstraintFindingVisitor::addTypeClassConstraint(TraitImpl *constraint, LOC_TY &loc){
        TypeError err{"", loc};
        constraints.emplace_back(constraint, err);
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
        for(auto &arg : n->args){
            arg->accept(*this);
        }

        auto dt = try_cast<AnDataType>(n->typeExpr->getType());
        if(dt){
            AnType *t = dt;
            vector<AnType*> fields = dt->getBoundFieldTypes();

            if(dt->decl->isUnionType){
                size_t variantIndex = dt->decl->getTagIndex(n->typeExpr->typeName);
                t = fields[variantIndex];
                fields = cast<AnTupleType>(t)->fields;
            }

            size_t argc = n->args.size();
            if(fields.size() != argc){
                auto lplural = fields.size() == 1 ? " argument, but " : " arguments, but ";
                auto rplural = argc == 1 ? " was given instead" : " were given instead";
                error(anTypeToColoredStr(t) + " requires " + to_string(fields.size())
                        + lplural + to_string(argc) + rplural, n->loc);
            }

            for(size_t i = 0; i < argc; i++){
                auto tnty = n->args[i]->getType();
                auto vty = fields[i];
                addConstraint(tnty, vty, n->args[i]->loc, "Expected field " + to_string(i+1)
                        + " of type $1 to be typecasted to the corresponding field type $2 from " + n->typeExpr->typeName);
            }
        }else{
            showError(anTypeToColoredStr(n->typeExpr->getType()) + " can't be constructed (with this syntax) because it is not a record", n->typeExpr->loc);
            error("You can use the 'cast' function for casting one type to another", n->typeExpr->loc, ErrorType::Note);
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

    void ConstraintFindingVisitor::searchForField(BinOpNode *op) {
        if(dynamic_cast<TypeNode*>(op->lval.get())){
            // not a field access, just qualified name resolution
            return;
        }

        VarNode *vn = dynamic_cast<VarNode*>(op->rval.get());
        if(!vn){
            auto tupleIndex = dynamic_cast<IntLitNode*>(op->rval.get());
            if(tupleIndex){
                size_t idx = atoi(tupleIndex->val.c_str());
                auto fields = vecOf<AnType*>(idx + 2);
                for(size_t i = 0; i <= idx; ++i){
                    fields.push_back(nextTypeVar());
                }
                addConstraint(op->getType(), fields.back(), op->loc,
                        "Expected result of tuple member access of index " + to_string(idx) + " to be $2 but got $1 instead");

                auto rho = nextTypeVar();
                rho = AnTypeVarType::get(rho->name + "...");
                fields.push_back(rho);
                addConstraint(op->lval->getType(), AnTupleType::get(fields), op->loc,
                        "Expected lhs of . to be a tuple resembling $2 but found $1 instead");
            }else{
                error("RHS of . operator must be an identifier or natural number", op->rval->loc);
            }
            return;
        }

        return; // TODO: row-polymorphism for struct fields
        // error("No field named " + vn->name + " found for any type", vn->loc);
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
            addConstraint(n->lval->getType(), fnty, n->loc,
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
            auto trait = module->freshTraitImpl("Extract"); // Extract 'col 'index -> 'elem

            addConstraint(n->lval->getType(), trait->typeArgs[0], n->loc,
                    "Error: should never fail, line " + to_string(__LINE__));
            addConstraint(n->rval->getType(), trait->typeArgs[1], n->loc,
                    "Expected index of subscript operator to be $2 but found $1 instead");
            addConstraint(n->getType(), trait->fundeps[0], n->loc,
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
            auto trait = module->freshTraitImpl("In");

            addTypeClassConstraint(trait, n->loc);
            addConstraint(n->lval->getType(), trait->typeArgs[0], n->loc,
                    "Error: should never fail, line " + to_string(__LINE__));
            addConstraint(n->rval->getType(), trait->typeArgs[1], n->loc,
                    "Error: should never fail, line " + to_string(__LINE__));
            addConstraint(n->getType(), AnType::getBool(), n->loc,
                    "Return type of 'in' should always be $2 but here is $1");
        }else if(n->op == '.'){
            searchForField(n);
        }else if(n->op == Tok_As){
            // intentionally empty
            TraitImpl *impl = module->freshTraitImpl("Cast");
            addTypeClassConstraint(impl, n->loc);
            addConstraint(n->rval->getType(), impl->typeArgs.back(), n->loc,
                    "Cannot cast to $1, variable is inferred to have type $2");
            addConstraint(n->lval->getType(), impl->typeArgs.front(), n->loc,
                    "Value is annotated to be of type $2, but it is of type $1");
            addConstraint(n->getType(), impl->typeArgs.back(), n->loc,
                    "Return value of 'as' operator should match the type used for casting, but found $1 and $2 respectively");
        }else if(n->op == Tok_Append){
            TraitImpl *impl = module->freshTraitImpl("Append");
            addTypeClassConstraint(impl, n->loc);
            addConstraint(impl->typeArgs[0], n->lval->getType(), n->loc,
                    "Error: should never fail, line " + to_string(__LINE__));
            addConstraint(n->lval->getType(), n->rval->getType(), n->loc,
                    "Operand types of '" + Lexer::getTokStr(n->op) + "' should match, but are $1 and $2 respectively");
            addConstraint(n->getType(), n->lval->getType(), n->loc,
                    "Return type of " + Lexer::getTokStr(n->op) + " should always match the first argument's type but here is $1");
        }else if(n->op == ':'){
            addConstraint(n->lval->getType(), n->rval->getType(), n->loc,
                    "Value is annotated to be of type $2, but it is of type $1");
            addConstraint(n->lval->getType(), n->getType(), n->loc,
                    "Return value of ':' operator should match the type of its left operand, but instead found $2 and $1 respectively");
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

    void ConstraintFindingVisitor::addConstraintsFromTCDecl(FuncDeclNode *fdn, TraitImpl *tr, FuncDeclNode *decl, LOC_TY &implLoc){
        TraitDecl *parent = tr->decl;

        for(size_t i = 0; i < parent->typeArgs.size(); i++){
            addConstraint(parent->typeArgs[i], tr->typeArgs[i], fdn->params->loc,
                    "Error: should never fail, line " + to_string(__LINE__)); //TODO: this line may fail (message may show)
        }

        for(size_t i = 0; i < tr->fundeps.size(); i++){
            addConstraint(parent->fundeps[i], tr->fundeps[i], fdn->params->loc,
                    "Error: should never fail, line " + to_string(__LINE__)); //TODO: this line may fail (message may show)
        }

        NamedValNode *declParam = decl->params.get();
        NamedValNode *fdnParam = fdn->params.get();
        while(declParam){
            addConstraint(declParam->getType(), fdnParam->getType(), fdnParam->loc,
                    "Expected type of parameter to be $1 from the impl arguments, but its type as used in the function is $2");
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


    void ConstraintFindingVisitor::visit(ExtNode *n){
        if(n->trait){
            auto tr = n->traitType;
            for(Node &m : *n->methods){
                auto fdn = dynamic_cast<FuncDeclNode*>(&m);
                if(fdn){
                    auto *decl = getDecl(fdn->name, tr->decl);
                    fdn->setType(decl->getType());
                    addConstraintsFromTCDecl(fdn, tr, decl, n->loc);
                    visit(fdn);
                }
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

        // Iterable 'i -> 'it 'e
        TraitImpl *iterable = module->freshTraitImpl("Iterable");
        n->iterableInstance = iterable;
        addTypeClassConstraint(iterable, n->loc);
        addConstraint(iterable->typeArgs[0], n->range->getType(), n->loc,
                "");
        addConstraint(iterable->fundeps[1], n->pattern->getType(), n->loc,
                "");
    }

    void ConstraintFindingVisitor::handleTuplePattern(parser::MatchNode *n,
                parser::TupleNode *pat, AnType *expectedType, Pattern &patChecker){

        auto fieldTys = vecOf<AnType*>(pat->exprs.size());
        for(size_t i = 0; i < pat->exprs.size(); i++){
            fieldTys.push_back(nextTypeVar());
        }
        auto tupTy = AnTupleType::get(fieldTys);
        addConstraint(tupTy, expectedType, pat->loc,
                "Expected a $1 here from the tuple destructuring, but found a $2 instead");
        patChecker.overwrite(Pattern::fromTuple(fieldTys), pat->loc);

        for(size_t i = 0; i < pat->exprs.size(); i++){
            handlePattern(n, pat->exprs[i].get(), fieldTys[i], patChecker.getChild(i));
        }
    }

    void ConstraintFindingVisitor::handleUnionVariantPattern(parser::MatchNode *n,
                parser::TypeCastNode *pat, AnType *expectedType, Pattern &patChecker){

        addConstraint(pat->getType(), expectedType, pat->loc,
                "Expected a $1 here from the union variant destructuring, but found a $2 instead");
        auto sumType = try_cast<AnDataType>(pat->getType());

        patChecker.overwrite(Pattern::fromSumType(sumType), pat->loc);
        string const& variantName = pat->typeExpr->typeName;
        size_t variantIndex = sumType->decl->getTagIndex(variantName);
        Pattern& child = patChecker.getChild(variantIndex);
        auto variantType = sumType->getVariantType(variantIndex);;

        for(size_t i = 0; i < pat->args.size(); i++){
            handlePattern(n, pat->args[i].get(), variantType->fields[i], child.getChild(i));
        }
    }

    void ConstraintFindingVisitor::handlePattern(MatchNode *n, Node *pattern, AnType *expectedType, Pattern &patChecker){
        if(TupleNode *tn = dynamic_cast<TupleNode*>(pattern)){
            handleTuplePattern(n, tn, expectedType, patChecker);

        }else if(TypeCastNode *tcn = dynamic_cast<TypeCastNode*>(pattern)){
            handleUnionVariantPattern(n, tcn, expectedType, patChecker);

        }else if(TypeNode *tn = dynamic_cast<TypeNode*>(pattern)){
            auto sumType = try_cast<AnDataType>(tn->getType());
            patChecker.overwrite(Pattern::fromSumType(sumType), tn->loc);
            auto idx = sumType->decl->getTagIndex(tn->typeName);
            patChecker.getChild(idx).setMatched();
            addConstraint(tn->getType(), expectedType, pattern->loc,
                "Expected a $1 here from the union variant pattern, but found a $2 instead");

        }else if(VarNode *vn = dynamic_cast<VarNode*>(pattern)){
            addConstraint(expectedType, vn->getType(), pattern->loc,
                "Expected the var pattern's type to be $2 from this match pattern but got $1 instead");
            patChecker.setMatched();

        }else if(IntLitNode *iln = dynamic_cast<IntLitNode*>(pattern)){
            auto ty = AnType::getPrimitive(iln->typeTag);
            patChecker.overwrite(Pattern::fromType(ty), iln->loc);
            addConstraint(ty, expectedType, pattern->loc,
                    "Expected this integer to be of type $1 from the match pattern, but got $2 instead");

        }else if(FltLitNode *fln = dynamic_cast<FltLitNode*>(pattern)){
            auto ty = AnType::getPrimitive(fln->typeTag);
            patChecker.overwrite(Pattern::fromType(ty), fln->loc);
            addConstraint(ty, expectedType, pattern->loc,
                    "Expected this float to be of type $1 from the match pattern, but got $2 instead");

        }else if(dynamic_cast<StrLitNode*>(pattern)){
            auto str = module->lookupType("Str");
            patChecker.overwrite(Pattern::fromType(str), pattern->loc);
            addConstraint(str, expectedType, pattern->loc,
                    "Expected this to be of type $1 from the match pattern, but got $2 instead");

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
