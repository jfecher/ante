#include "function.h"
#include "argtuple.h"
#include "jitlinker.h"

using namespace std;
using namespace llvm;
using namespace ante::parser;

namespace ante {

/*
 * Transforms t into a parameter type if need be.
 * - wraps array types in pointers
 * - wraps mutable types in pointers
 */
Type* parameterize(Compiler *c, AnType *t){
    if(t->typeTag == TT_Array) return c->anTypeToLlvmType(t)->getPointerTo();
    if(t->hasModifier(Tok_Mut)) return c->anTypeToLlvmType(t)->getPointerTo();
    return c->anTypeToLlvmType(t);
}

bool implicitPassByRef(AnType* t){
    return t->typeTag == TT_Array or t->hasModifier(Tok_Mut);
}


vector<AnType*> toTypeVector(vector<TypedValue> const& tvs){
    vector<AnType*> ret;
    ret.reserve(tvs.size());
    for(const auto tv : tvs)
        ret.push_back(tv.type);
    return ret;
}


TypedValue Compiler::callFn(string name, vector<TypedValue> args){
    auto typeVec = toTypeVector(args);
    TypedValue fn = getMangledFn(name, typeVec);
    if(!fn) return fn;

    //vector of llvm::Value*s for the call to CreateCall at the end
    vector<Value*> vals;
    vals.reserve(args.size());

    //Loop through each arg, typecheck them, and build vals vector
    //TODO: re-arrange all args into one tuple so that typevars
    //      are matched correctly across parameters
    auto *fnty = (AnFunctionType*)fn.type;
    for(size_t i = 0; i < args.size(); i++){
        auto arg = typeCheckWithImplicitCasts(this, args[i], fnty->extTys[i]);
        if(!arg) return arg;
        if(fnty->extTys[i]->hasModifier(Tok_Mut)){
            arg = addrOf(this, arg);
        }
        vals.push_back(arg.val);
    }

    return TypedValue(builder.CreateCall(fn.val, vals), fnty->retTy);
}



/*
 * Translates a NamedValNode list to a vector
 * of the types it contains.  If the list contains
 * a varargs type (represented by the absence of a type)
 * then a nullptr is inserted for that parameter.
 */
vector<Type*> getParamTypes(Compiler *c, FuncDecl *fd){
    vector<Type*> paramTys;

    if(fd->type){
        paramTys.reserve(fd->type->extTys.size());
        for(auto *paramTy : fd->type->extTys){
            auto *llvmty = parameterize(c, paramTy);
            paramTys.push_back(llvmty);
        }
        return paramTys;
    }

    paramTys.reserve(4);
    auto *nvn = fd->fdn->params.get();
    while(nvn){
        TypeNode *paramTyNode = (TypeNode*)nvn->typeExpr.get();
        if(paramTyNode == (void*)1){ //self parameter
            //Self parameters originally have 0x1 as their TypeNodes, but
            //this should be replaced when FuncDeclNode::compile is called.
            //Throw an error if that check was somehow bypassed
            c->compErr("Stray self parameter", nvn->loc);
        }else if(paramTyNode){
            auto *antype = toAnType(c, paramTyNode);
            auto *correctedType = parameterize(c, antype);
            paramTys.push_back(correctedType);
        }else{
            paramTys.push_back(0); //terminating null = varargs function
        }
        nvn = (NamedValNode*)nvn->next.get();
    }
    return paramTys;
}

/*
 *  Adds llvm attributes to an Argument based off the parameters type
 */
void addArgAttrs(llvm::Argument &arg, TypeNode *paramTyNode){
    if(paramTyNode->type == TT_Function){
        arg.addAttr(Attribute::AttrKind::NoCapture);

        if(!paramTyNode->hasModifier(Tok_Mut)){
            arg.addAttr(Attribute::AttrKind::ReadOnly);
        }
    }
}

/*
 *  Same as addArgAttrs, but for every parameter
 */
void addAllArgAttrs(Function *f, NamedValNode *params){
    for(auto &arg : f->args()){
        TypeNode *paramTyNode = (TypeNode*)params->typeExpr.get();

        addArgAttrs(arg, paramTyNode);

        if(!(params = (NamedValNode*)params->next.get())) break;
    }
}

/*
 * Type checks each return value
 *
 * returns the return type or nullptr if it could not be matched
 */
AnType* validateReturns(Compiler *c, FuncDecl *fd, TypeNode *retTy = 0){
    auto *matchTy = retTy ? toAnType(c, retTy) : fd->returns[fd->returns.size()-1].first.type;

    for(auto pair : fd->returns){
        TypedValue &ret = pair.first;

        auto tcr = c->typeEq(matchTy, ret.type);
        if(!tcr){
            c->compErr("Function " + fd->getName() + " returned value of type " +
                 anTypeToColoredStr(ret.type) + " but was declared to return value of type " +
                 anTypeToColoredStr(matchTy), pair.second);
        }

        if(tcr->res == TypeCheckResult::SuccessWithTypeVars){
            //TODO: copy type
            bindGenericToType(c, ret.type, tcr->bindings);
            ret.val->mutateType(c->anTypeToLlvmType(ret.type));

            auto *ri = dyn_cast<ReturnInst>(ret.val);

            if(LoadInst *li = dyn_cast<LoadInst>(ri ? ri->getReturnValue() : ret.val)){
                auto *alloca = li->getPointerOperand();

                auto *ins = ri ? ri->getParent() : c->builder.GetInsertBlock();
                c->builder.SetInsertPoint(ins);

                auto *cast = c->builder.CreateBitCast(alloca, c->anTypeToLlvmType(matchTy)->getPointerTo());
                auto *fixed_ret = c->builder.CreateLoad(cast);
                c->builder.CreateRet(fixed_ret);
                if(ri) ri->eraseFromParent();
            }
        }
    }

    return matchTy;
}


LOC_TY getFinalLoc(Node *n){
    auto *bop = dynamic_cast<BinOpNode*>(n);

    if(!bop){
        if(BlockNode* bn = dynamic_cast<BlockNode*>(n)){
            n = bn->block.get();
            bop = dynamic_cast<BinOpNode*>(n);
        }
    }

    return (bop and bop->op == ';') ? bop->rval->loc : n->loc;
}


TypedValue Compiler::compLetBindingFn(FuncDecl *fd, vector<Type*> &paramTys){
    auto *fdn = fd->fdn.get();
    FunctionType *preFnTy = FunctionType::get(Type::getVoidTy(*ctxt), paramTys, fdn->varargs);

    //preFn is the predecessor to fn because we do not yet know its return type, so its body must be compiled,
    //then the type must be checked and the new function with correct return type created, and their bodies swapped.
    Function *preFn = Function::Create(preFnTy, Function::ExternalLinkage, "__lambda_pre__", module.get());

    //Create the entry point for the function
    BasicBlock *entry = BasicBlock::Create(*ctxt, "entry", preFn);
    builder.SetInsertPoint(entry);

    //iterate through each parameter and add its value to the new scope.
    auto paramVec = vectorize(fdn->params.get());
    size_t i = 0;

    vector<Value*> preArgs;
    vector<AnType*> paramAnTys;

    for(auto &arg : preFn->args()){
        NamedValNode *cParam = paramVec[i];
        TypeNode *paramTyNode = (TypeNode*)cParam->typeExpr.get();
        addArgAttrs(arg, paramTyNode);

        //Self parameters originally have 0x1 as their TypeNodes, but
        //this should be replaced when FuncDeclNode::compile is called.
        //Throw an error if that check was somehow bypassed
        if(paramTyNode == (void*)1){
            compErr("Stray self parameter", cParam->loc);
            //paramTyNode = fd->obj;
        }

        for(size_t j = 0; j < i; j++){
            if(cParam->name == paramVec[j]->name){
                return compErr("Parameter name '"+cParam->name+"' is repeated for parameters "+
                        to_string(j+1)+" and "+to_string(i+1), cParam->loc);
            }
        }

        //If this function's type is specified beforehand (from a generic binding),
        //use the param types it specifies, otherwise translate the types now.
        AnType *paramTy = fd->type ?
                fd->type->extTys[i]
                : toAnType(this, paramTyNode);

        //If we are JIT compiling this function we want the args to be perfectly forwarded.
        //They must be passed directly to work with certain compiler-api functions.  For example,
        //it is important that Ante.store does not store a llvm::Argument
        TypedValue tArg = isJIT ?
            ctCtxt->args[i] : TypedValue(&arg, paramTy);

        stoVar(cParam->name, new Variable(cParam->name, tArg, this->scope,
                        /*nofree =*/ true, /*autoDeref = */implicitPassByRef(paramTy)));

        preArgs.push_back(&arg);
        paramAnTys.push_back(paramTy);

        ++i;
    }

    //store a fake function var, in case this function is recursive
    auto *fakeFnTy = AnFunctionType::get(AnType::getVoid(), paramAnTys);
    auto fakeFnTv = TypedValue(preFn, fakeFnTy);
    if(fdn->name.length() > 0)
        updateFn(fakeFnTv, fd, fdn->name, fd->mangledName);

    //actually compile the function, and hold onto the last value
    TypedValue v = CompilingVisitor::compile(this, fdn->child);

    //llvm requires explicit returns, so generate a return even if
    //the user did not in their function.
    if(!dyn_cast<ReturnInst>(v.val)){
        auto loc = getFinalLoc(fdn->child.get());

        if(v.type->typeTag == TT_Void){
            builder.CreateRetVoid();
            fd->returns.push_back({getVoidLiteral(), loc});
        }else{
            v.val = builder.CreateRet(v.val);
            fd->returns.push_back({v, loc});
        }
    }

    AnType *retTy;
    if(!(retTy = validateReturns(this, fd)))
        return {};

    //create the actual function's type, along with the function itself.
    FunctionType *ft = FunctionType::get(anTypeToLlvmType(retTy), paramTys, fdn->varargs);
    Function *f = Function::Create(ft, Function::ExternalLinkage,
            fdn->name.length() > 0 ? fd->mangledName : "__lambda__", module.get());

    //now that we have the real function, replace the old one with it
    auto *newFnTyn = AnFunctionType::get(retTy, paramAnTys);

    //finally, swap the bodies of the two functions and delete the former.
    //f->getBasicBlockList().push_back(&preFn->getBasicBlockList().front());
    f->getBasicBlockList().splice(f->begin(), preFn->getBasicBlockList());
    preFn->getBasicBlockList().clearAndLeakNodesUnsafely();

    //swap all instances of preFn's parameters with f's parameters
    i = 0;
    for(auto &arg : f->args()){
        preArgs[i++]->replaceAllUsesWith(&arg);
    }

    //preFn->replaceAllUsesWith(f);
    //preFn->removeFromParent();
    preFn->eraseFromParent();

    TypedValue ret = {f, newFnTyn};

    //only store the function if it has a name (and thus is not a lambda function)
    if(fdn->name.length() > 0)
        updateFn(ret, fd, fdn->name, fd->mangledName);

    return ret;
}


vector<llvm::Argument*> buildArguments(FunctionType *ft){
    vector<llvm::Argument*> args;
    for(unsigned i = 0, e = ft->getNumParams(); i != e; i++){
        assert(!ft->getParamType(i)->isVoidTy() && "Cannot have void typed arguments!");
        args.push_back(new llvm::Argument(ft->getParamType(i)));
    }
    return args;
}


/*
 *  Handles the modifiers or compiler directives (eg. ![inline]) then
 *  compiles the function fdn with either compFn or compLetBindingFn.
 */
TypedValue compFnWithModifiers(Compiler *c, FuncDecl *fd, ModNode *ppn){
    //remove the preproc node at the front of the modifier list so that the call to
    //compFn does not call this function in an infinite loop
    auto *fdn = fd->fdn.get();
    auto mod_cpy = fdn->modifiers;
    fdn->modifiers.reset((ModNode*)ppn->next.get());

    TypedValue fn;
    if(ppn->isCompilerDirective()){
        if(VarNode *vn = dynamic_cast<VarNode*>(ppn->expr.get())){
            if(vn->name == "inline"){
                fn = c->compFn(fd);
                if(!fn) return fn;
                ((Function*)fn.val)->addFnAttr(Attribute::AttrKind::AlwaysInline);
            }else if(vn->name == "run"){
                fn = c->compFn(fd);
                if(!fn) return fn;

                auto *mod = c->module.release();

                c->module.reset(new llvm::Module(fd->mangledName, *c->ctxt));
                auto recomp = c->compFn(fd);

                c->jitFunction((Function*)recomp.val);
                c->module.reset(mod);
            }else if(vn->name == "on_fn_decl"){
                auto *rettn = (TypeNode*)fdn->type.get();
                auto *fnty = AnFunctionType::get(c, toAnType(c, rettn), fdn->params.get(), true);
                fn = TypedValue(nullptr, fnty);
            }else{
                return c->compErr("Unrecognized compiler directive '"+vn->name+"'", vn->loc);
            }

            //put back the preproc node modifier
            fdn->modifiers = mod_cpy;
            return fn;
        }else{
            return c->compErr("Unrecognized compiler directive", ppn->loc);
        }
    // ppn is a normal modifier
    }else{
        if(ppn->mod == Tok_Ante){
            bool fnInCompAPI = compapi[fd->getName()].get();
            if(c->isJIT && !fnInCompAPI){
                fn = c->compFn(fd);
            }else{
                auto *rettn = (TypeNode*)fd->fdn->type.get();
                auto *fnty = AnFunctionType::get(c, toAnType(c, rettn), fd->fdn->params.get(), true);
                fn = TypedValue(nullptr, fnty);
            }
        }else{
            fn = c->compFn(fd);
        }
        fdn->modifiers = mod_cpy;
        return fn;
    }
}


TypedValue compFnHelper(Compiler *c, FuncDecl *fd){
    BasicBlock *caller = c->builder.GetInsertBlock();
    auto *fdn = fd->fdn.get();

    if(ModNode *ppn = fdn->modifiers.get()){
        auto ret = compFnWithModifiers(c, fd, ppn);
        c->builder.SetInsertPoint(caller);
        return ret;
    }

    //Get and translate the function's return type to an llvm::Type*
    TypeNode *retNode = (TypeNode*)fdn->type.get();

    vector<Type*> paramTys = getParamTypes(c, fd);

    if(paramTys.size() > 0 && !paramTys.back()){ //varargs fn
        fdn->varargs = true;
        paramTys.pop_back();
    }

    if(!retNode){
        try{
            auto ret = c->compLetBindingFn(fd, paramTys);
            c->builder.SetInsertPoint(caller);
            return ret;
        }catch(CtError *e){
            c->builder.SetInsertPoint(caller);
            throw e;
        }
    }

    AnType *anRetTy;
    AnType *fnTy;

    //If the function type was set beforehand (likely due to a generic binding)
    //Then just retrieve type information from there
    if(fd->type){
        fnTy = fd->type;
        anRetTy = fnTy->getFunctionReturnType();
    }else{
        anRetTy = toAnType(c, retNode);
        fnTy = AnFunctionType::get(c, anRetTy, fdn->params.get());
    }

    //llvm return type and function type corresponding to the AnTypes above
    Type *retTy = c->anTypeToLlvmType(anRetTy);

    FunctionType *ft = FunctionType::get(retTy, paramTys, fdn->varargs);
    Function *f = Function::Create(ft, Function::ExternalLinkage, fd->mangledName, c->module.get());
    f->addFnAttr(Attribute::AttrKind::NoUnwind);
    addAllArgAttrs(f, fdn->params.get());


    auto ret = TypedValue(f, fnTy);
    //stoVar(fdn->name, new Variable(fdn->name, ret, scope));
    c->updateFn(ret, fd, fdn->name, fd->mangledName);

    //The above handles everything for a function declaration
    //If the function is a definition, then the body will be compiled here.
    if(fdn->child){
        //Create the entry point for the function
        BasicBlock *bb = BasicBlock::Create(*c->ctxt, "entry", f);
        c->builder.SetInsertPoint(bb);

        auto paramVec = vectorize(fdn->params.get());
        size_t i = 0;

        //iterate through each parameter and add its value to the new scope.
        for(auto &arg : f->args()){
            NamedValNode *cParam = paramVec[i];
            TypeNode *paramTyNode = (TypeNode*)cParam->typeExpr.get();

            for(size_t j = 0; j < i; j++){
                if(cParam->name == paramVec[j]->name){
                    return c->compErr("Parameter name '"+cParam->name+"' is repeated for parameters "+
                            to_string(j+1)+" and "+to_string(i+1), cParam->loc);
                }
            }

            //Again, if the function type was manually specified from a generic type
            //binding then use that as the param type, otherwise assume it is a concrete type
            AnType *paramTy = fd->type ?
                    fd->type->extTys[i]
                    : toAnType(c, paramTyNode);

            //If we are JIT compiling this function we want the args to be perfectly forwarded.
            //They must be passed directly to work with certain compiler-api functions.  For example,
            //it is important that Ante.store does not store a llvm::Argument
            TypedValue tArg = c->isJIT ?
                c->ctCtxt->args[i] : TypedValue(&arg, paramTy);

            c->stoVar(cParam->name, new Variable(cParam->name, tArg, c->scope,
                    /*nofree = */true, /*autoDeref = */implicitPassByRef(paramTy)));

            i++;
        }

        //actually compile the function, and hold onto the last value
        TypedValue v;
        try{
            v = CompilingVisitor::compile(c, fdn->child);
        }catch(CtError *e){
            c->builder.SetInsertPoint(caller);
            throw e;
        }

        //push the final value as a return, explicit returns are already added in RetNode::compile
        if(retNode && !dyn_cast<ReturnInst>(v.val)){
            auto loc = getFinalLoc(fdn->child.get());

            if(retNode->type == TT_Void){
                c->builder.CreateRetVoid();
                fd->returns.push_back({c->getVoidLiteral(), loc});
            }else{
                v.val = c->builder.CreateRet(v.val);
                fd->returns.push_back({v, loc});
            }
        }

        //dont optimize if the return type is invalid.  LLVM will most likely crash
        AnType *retty;
        if(!(retty = validateReturns(c, fd, retNode))){
            c->builder.SetInsertPoint(caller);
            return {};
        }
    }

    c->builder.SetInsertPoint(caller);
    return ret;
}


FuncDecl* shallow_copy(FuncDecl* fd, string &mangledName){
    FuncDecl *cpy = new FuncDecl(fd->fdn, mangledName, fd->scope, fd->module);
    cpy->obj_bindings = fd->obj_bindings;
    cpy->obj = fd->obj;
    return cpy;
}


TypedValue compTemplateFn(Compiler *c, FuncDecl *fd, TypeCheckResult &tc, vector<AnType*> &args){
    //test if bound variant is already compiled
    string mangled = mangle(fd, args);

    FuncDecl* fdRedef;
    if((fdRedef = c->getFuncDecl(fd->getName(), mangled))){
        if(fdRedef->mangledName == mangled && fdRedef->tv){
            return fdRedef->tv;
        }
    }

    fd = shallow_copy(fd, mangled);

    //Each binding from the typecheck results needs to be declared as a typevar in the
    //function's scope, but compFn sets this scope later on, so the needed bindings are
    //instead stored as fake obj bindings to be declared later in compFn
    for(auto& pair : tc->bindings){
        fd->obj_bindings.push_back({pair.first, pair.second});
    }

    //Default return type in case this function has an inferred return type;
    AnType *anRetTy = AnType::getVoid();

    //bind the return type if necessary
    if(TypeNode* retTy = (TypeNode*)fd->fdn->type.get()){
        anRetTy = bindGenericToType(c, toAnType(c, retTy), fd->obj_bindings);
    }

    auto *fty = AnFunctionType::get(anRetTy, args);
    fd->type = fty;

    //compile the function normally (each typevar should now be
    //substituted with its checked type from the typecheck tc)
    return c->compFn(fd);
}

//Defined in compiler.cpp
string manageSelfParam(Compiler *c, FuncDeclNode *fdn, string &mangledName);


/**
 * Returns true if the given function name is a declaration
 * and not a definition
 */
bool isDecl(string &name){
    return name.back() == ';';
}

/*
 *  Registers a function for later compilation
 */
void CompilingVisitor::visit(FuncDeclNode *n){
    //check if the function is a named function.
    if(n->name.length() > 0){
        string mangledName;
        if(isDecl(n->name)){
            n->name = c->funcPrefix + n->name.substr(0, n->name.length() - 1);
            mangledName = n->name;
        }else{
            mangledName = c->funcPrefix + mangle(n->name, n->params);
            mangledName = manageSelfParam(c, n, mangledName);
            n->name = c->funcPrefix + n->name;
        }

        c->registerFunction(n, mangledName);
        this->val = c->getVoidLiteral();
    }else{
        //Otherwise, if it is a lambda function, compile it now and return it.
        string no_name;
        shared_ptr<FuncDeclNode> lambda{n};
        FuncDecl *fd = new FuncDecl(lambda, no_name, c->scope, c->mergedCompUnits);
        this->val = c->compFn(fd);

        //prevent this function from being called by name
        fd->mangledName = "";
    }
}

FuncDecl* getFuncDeclFromVec(vector<shared_ptr<FuncDecl>> &l, string const& mangledName){
    for(auto& fd : l){
        if(fd->mangledName == mangledName)
            return fd.get();
    }
    return 0;
}


void declareBindings(Compiler *c, vector<pair<string,AnType*>> &bindings){
    for(auto &p : bindings){
        if(!p.second->isGeneric)
            c->stoTypeVar(p.first, p.second);
    }
}


//Provide a wrapper for function-compiling methods so that each
//function is compiled in its own isolated module
TypedValue Compiler::compFn(FuncDecl *fd){
    compCtxt->callStack.push_back(fd);
    auto *continueLabels = compCtxt->continueLabels.release();
    auto *breakLabels = compCtxt->breakLabels.release();
    compCtxt->continueLabels = llvm::make_unique<vector<BasicBlock*>>();
    compCtxt->breakLabels = llvm::make_unique<vector<BasicBlock*>>();
    size_t callingFnScope = fnScope;

    enterNewScope();
    fnScope = scope;

    //Propogate type var bindings of the method obj into the function scope
    declareBindings(this, fd->obj_bindings);
    TypedValue ret;

    try{
        ret = compFnHelper(this, fd);
    }catch(CtError *e){
        compCtxt->callStack.pop_back();
        compCtxt->continueLabels.reset(continueLabels);
        compCtxt->breakLabels.reset(breakLabels);

        while(scope > callingFnScope)
            exitScope();

        fnScope = callingFnScope;

        throw e;
    }

    compCtxt->callStack.pop_back();
    compCtxt->continueLabels.reset(continueLabels);
    compCtxt->breakLabels.reset(breakLabels);
    fnScope = callingFnScope;
    exitScope();
    return ret;
}


FuncDecl* Compiler::getCurrentFunction() const{
    return compCtxt->callStack.back();
}



void Compiler::updateFn(TypedValue &f, FuncDecl *fd, string &name, string &mangledName){
    auto &list = mergedCompUnits->fnDecls[name];
    auto *vec_fd = getFuncDeclFromVec(list, mangledName);
    if(vec_fd){
        vec_fd->tv = f;
    }else{
        fd->tv = f;
        list.push_back(shared_ptr<FuncDecl>(fd));
    }
}


TypedValue Compiler::getFunction(string const& name, string const& mangledName){
    auto& list = getFunctionList(name);
    if(list.empty()) return {};

    auto *fd = getFuncDeclFromVec(list, mangledName);
    if(!fd) return {};

    if(fd->tv) return fd->tv;

    //Function has been declared but not defined, so define it.
    //fd->tv = compFn(fd);
    return compFn(fd);
}

/*
 * Returns all FuncDecls from a list that have argc number of parameters
 * and can be accessed in the current scope.
 */
vector<shared_ptr<FuncDecl>> filterByArgcAndScope(vector<shared_ptr<FuncDecl>> &l, size_t argc, unsigned int scope){
    vector<shared_ptr<FuncDecl>> ret;
    for(auto& fd : l){
        if(fd->scope <= scope && getTupleSize(fd->fdn->params.get()) == argc){
            ret.push_back(fd);
        }
    }
    return ret;
}


template<typename T>
vector<T*> vectorize(T *args){
    vector<T*> ret;
    while(args){
        ret.push_back(args);
        args = (T*)args->next.get();
    }
    return ret;
}

/**
 * Return a new vector containing only the given pairs with the
 * highest amount of matches.  In the case there are multiple equally,
 * matching candidates, results are further filtered by the minimum amount
 * of type bindings needed.  If there are still multiple equally matching
 * candidates, they are all returned.
 */
vector<pair<TypeCheckResult,FuncDecl*>> filterHighestMatches(vector<pair<TypeCheckResult,FuncDecl*>> &matches){
    unsigned int highestMatch = 0;
    unsigned int reqBindings = 0;
    vector<pair<TypeCheckResult,FuncDecl*>> highestMatches;

    for(auto &tcr : matches){
        if(tcr.first and tcr.first->matches >= highestMatch){
            if(tcr.first->matches > highestMatch){
                highestMatch = tcr.first->matches;
                reqBindings = tcr.first->bindings.size();
                highestMatches.clear();
            }else if(tcr.first->bindings.size() < reqBindings){
                highestMatch = tcr.first->matches;
                reqBindings = tcr.first->bindings.size();
                highestMatches.clear();
            }
            highestMatches.push_back(tcr);
        }
    }
    return highestMatches;
}


/**
 * Return only the candidates that best match the given argument types
 * in a type check.  This results in the candidates with the most structural
 * matches (eg. prefer  't* over 't if arg is a pointer) and the least amount
 * of required type bindings (eg. prefer i32* over 't* if arg is i32*).
 */
vector<pair<TypeCheckResult,FuncDecl*>>
filterBestMatches(Compiler *c, vector<shared_ptr<FuncDecl>> &candidates, vector<AnType*> args){
    vector<pair<TypeCheckResult,FuncDecl*>> results;
    results.reserve(candidates.size());

    for(auto fd : candidates){
        auto *fnty = fd->type ? fd->type
            : AnFunctionType::get(c, AnType::getVoid(), fd->fdn->params.get());
        auto tc = c->typeEq(fnty->extTys, args);
        results.emplace_back(tc, fd.get());
    }

    return filterHighestMatches(results);
}


FuncDecl* Compiler::getMangledFuncDecl(string name, vector<AnType*> &args){
    auto& fnlist = getFunctionList(name);
    if(fnlist.empty()) return 0;

    auto argc = args.size();

    auto candidates = filterByArgcAndScope(fnlist, argc, scope);
    if(candidates.empty()) return 0;

    //if there is only one function now, return it.  It will be typechecked later
    if(candidates.size() == 1)
        return candidates.front().get();

    //check for an exact match on the remaining candidates.
    string fnName = mangle(name, args);
    auto *fd = getFuncDeclFromVec(candidates, fnName);
    if(fd) //exact match
        return fd;

    auto matches = filterBestMatches(this, candidates, args);

    //TODO: return typecheck infromation so it need not typecheck again in Compiler::getMangledFn
    if(matches.size() == 1)
        return matches[0].second;

    //TODO: possibly return all functions considered for better error checking
    return nullptr;
}


/*
 * Compile a possibly-generic function with given arg types
 */
TypedValue compFnWithArgs(Compiler *c, FuncDecl *fd, vector<AnType*> args){
    //must check if this functions is generic first
    auto fnty = AnFunctionType::get(c, AnType::getVoid(), fd->fdn->params.get());
    auto tc = c->typeEq(fnty->extTys, args);

    if(tc->res == TypeCheckResult::SuccessWithTypeVars)
        return compTemplateFn(c, fd, tc, args);
    else if(!tc) //tc->res == TypeCheckResult::Failure
        return {};
    else if(fd->tv)
        return fd->tv;
    else
        return c->compFn(fd);
}


TypedValue Compiler::getMangledFn(string name, vector<AnType*> &args){
    auto *fd = getMangledFuncDecl(name, args);
    if(!fd) return {};

    return compFnWithArgs(this, fd, args);
}


vector<shared_ptr<FuncDecl>>& Compiler::getFunctionList(string const& name) const{
    return mergedCompUnits->fnDecls[name];
}


vector<AnType*> toArgTuple(AnType *ty);


FuncDecl* Compiler::getCastFuncDecl(AnType *from_ty, AnType *to_ty){
    string fnBaseName = getCastFnBaseName(to_ty);
    auto args = toArgTuple(from_ty);
    return getMangledFuncDecl(fnBaseName, args);
}


TypedValue Compiler::getCastFn(AnType *from_ty, AnType *to_ty, FuncDecl *fd){
    if(!fd)
        fd = getCastFuncDecl(from_ty, to_ty);

    if(!fd) return {};
    TypedValue tv;

    auto *to_ty_dt = dyn_cast<AnDataType>(to_ty);
    if(to_ty_dt and to_ty_dt->isVariant()){
        AnType *unbound_obj = fd->obj;
        fd->obj = to_ty;
        fd->obj_bindings = to_ty_dt->boundGenerics;

        //must check if this function is generic first
        auto args = toArgTuple(from_ty);
        if(!fd->obj_bindings.empty()){
            //force a call to compTemplateFunction as the object itself is generic
            //and must be bound even if the function has no parameters to match.
            //This is common in a constructor for an empty container, eg. Vec<i32>()
            auto fnty = AnFunctionType::get(this, AnType::getVoid(), fd->fdn->params.get());
            auto tc = typeEq(fnty->extTys, args);
            tv = compTemplateFn(this, fd, tc, args);
        }else{
            tv = compFnWithArgs(this, fd, args);
        }

        //TODO: if fd is a meta function that is a method of a generic object then the generic
        //      parameters of the object will be unbound here and untraceable when the function is
        //      lazily compiled at the callsite
        fd->obj = unbound_obj;
        fd->obj_bindings.clear();
        fd->tv = {};
    }else{
        tv = fd->tv;
        if(!tv){
            auto args = toArgTuple(from_ty);
            tv = compFnWithArgs(this, fd, args);
        }
    }
    return tv;
}


/*
 * Returns the FuncDecl* of a given name/basename pair
 * returns nullptr if specified function is not found
 */
FuncDecl* Compiler::getFuncDecl(string baseName, string mangledName){
    auto& list = getFunctionList(baseName);
    if(list.empty()) return 0;

    return getFuncDeclFromVec(list, mangledName);
}

/*
 *  Adds a function to the list of declared, but not defined functions.  A declared function's
 *  FuncDeclNode can be added to be compiled only when it is later called.  Useful to prevent pollution
 *  of a module with unneeded library functions.
 */
void Compiler::registerFunction(FuncDeclNode *fn, string &mangledName){
    //check for redeclaration
    auto *redecl = getFuncDecl(fn->name, mangledName);

    if(redecl and redecl->mangledName == mangledName){
        compErr("Function " + fn->name + " was redefined", fn->loc);
        return;
    }

    shared_ptr<FuncDeclNode> spToFn{fn};
    FuncDecl *fdRaw = new FuncDecl(spToFn, mangledName, scope, mergedCompUnits);
    shared_ptr<FuncDecl> fd{fdRaw};
    fd->obj = compCtxt->obj;

    for(auto &hook : ctCtxt->on_fn_decl_hook){
        Value *fd_val = builder.getInt64((unsigned long)fdRaw);
        vector<TypedValue> args;
        args.emplace_back(fd_val, AnDataType::get("FuncDecl"));
        compMetaFunctionResult(this, hook->fdn->loc, hook->getName(), hook->mangledName, args);
    }

    for(auto *mod : *fn->modifiers){
        auto *m = (ModNode*)mod;
        if(m->isCompilerDirective()){
            VarNode *vn;
            if((vn = dynamic_cast<VarNode*>(m->expr.get())) and vn->name == "on_fn_decl"){
                ctCtxt->on_fn_decl_hook.push_back(fd);
            }
        }
    }

    compUnit->fnDecls[fn->name].push_back(fd);
    mergedCompUnits->fnDecls[fn->name].push_back(fd);
}

} //end of namespace ante
