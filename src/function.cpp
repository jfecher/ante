#include "compiler.h"

/*
 * Transforms t into a parameter type if need be.
 * - returns pointers to tuple types
 * - returns pointers to array types
 */
Type* parameterize(Type *t, const TypeNode *tn){
    if(t->isArrayTy()) return t->getPointerTo();
    if(tn->hasModifier(Tok_Mut)) return t->getPointerTo();
    return t;
}

bool implicitPassByRef(TypeNode* t){
    return t->type == TT_Array or t->hasModifier(Tok_Mut);
}

/*
 * Translates a NamedValNode list to a vector
 * of the types it contains.  If the list contains
 * a varargs type (represented by the absence of a type)
 * then a nullptr is inserted for that parameter.
 */
vector<Type*> getParamTypes(Compiler *c, NamedValNode *nvn, size_t paramCount){
    vector<Type*> paramTys;
    paramTys.reserve(paramCount);

    for(size_t i = 0; i < paramCount && nvn; i++){

        TypeNode *paramTyNode = (TypeNode*)nvn->typeExpr.get();
        if(paramTyNode){
            auto *type = c->typeNodeToLlvmType(paramTyNode);
            auto *correctedType = parameterize(type, paramTyNode);
            paramTys.push_back(correctedType);
        }else
            paramTys.push_back(0); //terminating null = varargs function
        nvn = (NamedValNode*)nvn->next.get();
    }
    return paramTys;
}

/*
 *  Adds llvm attributes to an Argument based off the parameters type
 */
void addArgAttrs(llvm::Argument &arg, TypeNode *paramTyNode){
    if(paramTyNode->type == TT_Function)
        arg.addAttr(Attribute::AttrKind::NoCapture);
    
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
TypeNode* validateReturns(Compiler *c, FuncDecl *fd, TypeNode *retTy = 0){
    auto *matchTy = retTy ? retTy : fd->returns[fd->returns.size()-1]->type.get();

    for(auto *ret : fd->returns){
        auto tcr = c->typeEq(matchTy, ret->type.get());
        if(!tcr){
            return (TypeNode*)c->compErr("Function " + fd->fdn->basename + " returned value of type " + 
                    typeNodeToColoredStr(ret->type) + " but was declared to return value of type " +
                    typeNodeToColoredStr(matchTy), fd->fdn->loc);
        }

        if(tcr.res == TypeCheckResult::SuccessWithTypeVars){
            //TODO: copy type
            bindGenericToType(ret->type.get(), matchTy->params);
            ret->val->mutateType(c->typeNodeToLlvmType(ret->type.get()));

            auto *ri = dyn_cast<ReturnInst>(ret->val);

            if(LoadInst *li = dyn_cast<LoadInst>(ri ? ri->getReturnValue() : ret->val)){
                auto *alloca = li->getPointerOperand();

                auto *ins = ri ? ri->getParent() : c->builder.GetInsertBlock();
                c->builder.SetInsertPoint(ins);

                auto *cast = c->builder.CreateBitCast(alloca, c->typeNodeToLlvmType(matchTy)->getPointerTo());
                auto *fixed_ret = c->builder.CreateLoad(cast);
                c->builder.CreateRet(fixed_ret);
                if(ri) ri->eraseFromParent();
            }
        }
    }

    return deepCopyTypeNode(matchTy);
}



TypedValue* Compiler::compLetBindingFn(FuncDecl *fd, vector<Type*> &paramTys){
    auto *fdn = fd->fdn;
    FunctionType *preFnTy = FunctionType::get(Type::getVoidTy(*ctxt), paramTys, fdn->varargs);

    //preFn is the predecessor to fn because we do not yet know its return type, so its body must be compiled,
    //then the type must be checked and the new function with correct return type created, and their bodies swapped.
    Function *preFn = Function::Create(preFnTy, Function::ExternalLinkage, "__lambda_pre__", module.get());

    //Create the entry point for the function
    BasicBlock *entry = BasicBlock::Create(*ctxt, "entry", preFn);
    builder.SetInsertPoint(entry);
 
    TypeNode *fakeFnTyn = mkAnonTypeNode(TT_Function);
    TypeNode *fakeRetTy = mkAnonTypeNode(TT_Void);
    fakeFnTyn->extTy.reset(fakeRetTy);
        
    //tell the compiler to create a new scope on the stack.
    enterNewScope();

    //iterate through each parameter and add its value to the new scope.
    TypeNode *curTyn = 0;
    NamedValNode *cParam = fdn->params.get();
    vector<Value*> preArgs;
    
    for(auto &arg : preFn->args()){
        TypeNode *paramTyNode = (TypeNode*)cParam->typeExpr.get();
        addArgAttrs(arg, paramTyNode);

        stoVar(cParam->name, new Variable(cParam->name, new TypedValue(&arg, deepCopyTypeNode(paramTyNode)), this->scope,
                        /*nofree =*/ true, /*autoDeref = */implicitPassByRef(paramTyNode)));

        preArgs.push_back(&arg);

        if(curTyn){
            curTyn->next.reset(paramTyNode);
            curTyn = (TypeNode*)curTyn->next.get();
        }else{
            fakeFnTyn->extTy->next.reset(deepCopyTypeNode(paramTyNode));
            curTyn = (TypeNode*)fakeFnTyn->extTy->next.get();
        }
        if(!(cParam = (NamedValNode*)cParam->next.get())) break;
    }
    
    //store a fake function var, in case this function is recursive
    auto *fakeFnTv = new TypedValue(preFn, fakeFnTyn);
    if(fdn->name.length() > 0)
        updateFn(fakeFnTv, fdn->basename, fdn->name);

    //actually compile the function, and hold onto the last value
    TypedValue *v = fdn->child->compile(this);
    if(!v) return 0;

    //End of the function, discard the function's scope.
    exitScope();

    //llvm requires explicit returns, so generate a return even if
    //the user did not in their function.
    if(!dyn_cast<ReturnInst>(v->val)){
        if(v->type->type == TT_Void){
            builder.CreateRetVoid();
            fd->returns.push_back(v);
        }else{
            builder.CreateRet(v->val);
            fd->returns.push_back(v);
        }
    }
    
    TypeNode *retTy;
    if(!(retTy = validateReturns(this, fd)))
        return 0;

    //create the actual function's type, along with the function itself.
    FunctionType *ft = FunctionType::get(typeNodeToLlvmType(retTy), paramTys, fdn->varargs);
    Function *f = Function::Create(ft, Function::ExternalLinkage,
            fdn->name.length() > 0 ? fdn->name : "__lambda__", module.get());
  
    //now that we have the real function, replace the old one with it

    //prepend the ret type to the function's type node node extension list.
    //(A typenode represents functions by having the first extTy as the ret type,
    //and the (optional) next types in the list as the parameter types)
    TypeNode *newFnTyn = deepCopyTypeNode(fakeFnTyn);
    TypeNode *params = (TypeNode*)newFnTyn->extTy->next.release();

    retTy->next.reset(params);
    newFnTyn->extTy.reset(retTy);


    //finally, swap the bodies of the two functions and delete the former.
    //f->getBasicBlockList().push_back(&preFn->getBasicBlockList().front());
    f->getBasicBlockList().splice(f->begin(), preFn->getBasicBlockList());
    preFn->getBasicBlockList().clearAndLeakNodesUnsafely();

    //swap all instances of preFn's parameters with f's parameters
    int i = 0;
    for(auto &arg : f->args()){
        preArgs[i++]->replaceAllUsesWith(&arg);
    }
    
    preFn->replaceAllUsesWith(f);
    preFn->removeFromParent();

    auto *ret = new TypedValue(f, newFnTyn);

    //only store the function if it has a name (and thus is not a lambda function)
    if(fdn->name.length() > 0)
        updateFn(ret, fdn->basename, fdn->name);


    delete fakeFnTv;
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
 *  Translates a list of NamedValNodes to a list of TypeNodes
 *  that are deep copies of each named val node's type
 */
TypeNode* createFnTyNode(NamedValNode *params, TypeNode *retTy){
    TypeNode *fnTy = mkAnonTypeNode(TT_Function);
    fnTy->extTy.reset(retTy ? deepCopyTypeNode(retTy) : mkAnonTypeNode(TT_Void));

    TypeNode *curTyn = fnTy->extTy.get();
    while(params && params->typeExpr.get()){
        curTyn->next.reset(deepCopyTypeNode((TypeNode*)params->typeExpr.get()));
        curTyn = (TypeNode*)curTyn->next.get();
        params = (NamedValNode*)params->next.get();
    }
    return fnTy;
}


/*
 *  Handles a compiler directive (eg. ![inline]) then compiles the function fdn
 *  with either compFn or compLetBindingFn.
 */
TypedValue* compCompilerDirectiveFn(Compiler *c, FuncDecl *fd, PreProcNode *ppn){
    //remove the preproc node at the front of the modifier list so that the call to
    //compFn does not call this function in an infinite loop
    auto *fdn = fd->fdn;

    fdn->modifiers.release();
    fdn->modifiers.reset(ppn->next.get());
    auto *fn = c->compFn(fd);
    if(!fn) return 0;

    //put back the preproc node modifier
    fdn->modifiers.release();
    fdn->modifiers.reset(ppn);

    if(VarNode *vn = dynamic_cast<VarNode*>(ppn->expr.get())){
        if(vn->name == "inline"){
            ((Function*)fn->val)->addFnAttr("always_inline");
        }else if(vn->name == "run"){
            auto *mod = c->module.get();
            c->module.release();

            c->module.reset(new llvm::Module(fdn->name, *c->ctxt));
            auto *recomp = c->compFn(fd);

            c->jitFunction((Function*)recomp->val);
            c->module.reset(mod);
        }else if(vn->name == "macro"){
            fn->type->type = TT_MetaFunction;
        }else if(vn->name == "meta"){
            fn->type->type = TT_MetaFunction;
        }else{
            return c->compErr("Unrecognized compiler directive '"+vn->name+"'", vn->loc);
        }

        return fn;
    }else{
        return c->compErr("Unrecognized compiler directive", ppn->loc);
    }
}


TypedValue* compFnHelper(Compiler *c, FuncDecl *fd){
    BasicBlock *caller = c->builder.GetInsertBlock();
    auto *fdn = fd->fdn;

    if(PreProcNode *ppn = dynamic_cast<PreProcNode*>(fdn->modifiers.get())){
        auto *ret = compCompilerDirectiveFn(c, fd, ppn);
        c->builder.SetInsertPoint(caller);
        return ret;
    }


    //Get and translate the function's return type to an llvm::Type*
    TypeNode *retNode = (TypeNode*)fdn->type.get();

    //Count the number of parameters
    NamedValNode *paramsBegin = fdn->params.get();
    size_t nParams = getTupleSize(paramsBegin);

    vector<Type*> paramTys = getParamTypes(c, paramsBegin, nParams);

    if(paramTys.size() > 0 && !paramTys.back()){ //varargs fn
        fdn->varargs = true;
        paramTys.pop_back();
    }
    
    if(!retNode){
        auto *ret = c->compLetBindingFn(fd, paramTys);
        c->builder.SetInsertPoint(caller);
        return ret;
    }else{
        if(!retNode->params.empty()){
            c->expand(retNode);
            bindGenericToType(retNode, retNode->params);
        }
    }


    //create the function's actual type node for the tval later
    TypeNode *fnTy = createFnTyNode(fdn->params.get(), retNode);


    Type *retTy = c->typeNodeToLlvmType(retNode);

    FunctionType *ft = FunctionType::get(retTy, paramTys, fdn->varargs);
    Function *f = Function::Create(ft, Function::ExternalLinkage, fdn->name, c->module.get());
    f->addFnAttr("nounwind");
    addAllArgAttrs(f, paramsBegin);


    auto* ret = new TypedValue(f, fnTy);
    //stoVar(fdn->name, new Variable(fdn->name, ret, scope));
    c->updateFn(ret, fdn->basename, fdn->name);

    //The above handles everything for a function declaration
    //If the function is a definition, then the body will be compiled here.
    if(fdn->child){
        //Create the entry point for the function
        BasicBlock *bb = BasicBlock::Create(*c->ctxt, "entry", f);
        c->builder.SetInsertPoint(bb);

        //tell the compiler to create a new scope on the stack.
        c->enterNewScope();

        NamedValNode *cParam = paramsBegin;

        //iterate through each parameter and add its value to the new scope.
        for(auto &arg : f->args()){
            TypeNode *paramTyNode = (TypeNode*)cParam->typeExpr.get();

            c->stoVar(cParam->name, new Variable(cParam->name, new TypedValue(&arg, deepCopyTypeNode(paramTyNode)),
                        c->scope, /*nofree = */true, /*autoDeref = */implicitPassByRef(paramTyNode)));
            
            if(!(cParam = (NamedValNode*)cParam->next.get())) break;
        }

        //actually compile the function, and hold onto the last value
        TypedValue *v = fdn->child->compile(c);
        if(!v){
            c->builder.SetInsertPoint(caller);
            return 0;
        }
        
        //End of the function, discard the function's scope.
        c->exitScope();
  
        //push the final value as a return, explicit returns are already added in RetNode::compile
        if(retNode && !dyn_cast<ReturnInst>(v->val)){
            if(retNode->type == TT_Void){
                c->builder.CreateRetVoid();
                fd->returns.push_back(c->getVoidLiteral());
            }else{
                c->builder.CreateRet(v->val);
                fd->returns.push_back(v);
            }
        }

        //dont optimize if the return type is invalid.  LLVM would most likely crash
        TypeNode *retty;
        if(!(retty = validateReturns(c, fd, retNode))){
            c->builder.SetInsertPoint(caller);
            return 0;
        }

        //optimize!
        if(!c->errFlag)
            c->passManager->run(*f);
        
        delete retty;
    }

    c->builder.SetInsertPoint(caller);
    return ret;
}
            

TypeNode* replaceParams(NamedValNode *params, TypeNode *args){
    TypeNode *oldParams = 0;

    while(params and args){
        if(oldParams ){
            oldParams->next.release();
            oldParams->next.reset(params->typeExpr.release());
        }else
            oldParams = (TypeNode*)params->typeExpr.release();

        params->typeExpr.reset(args);

        params = (NamedValNode*)params->next.get();
        args = (TypeNode*)args->next.get();
    }
    return oldParams;
}


TypedValue* compTemplateFn(Compiler *c, FuncDecl *fd, TypeCheckResult &tc, TypeNode *args){
    c->enterNewScope();

    //apply each binding from the typecheck results to a type variables in this scope
    for(auto& pair : tc.bindings){
        auto *type_var = new TypedValue(nullptr, pair.second.release());
        c->stoVar(pair.first, new Variable(pair.first, type_var, c->scope));
    }

    TypeNode *argscpy = deepCopyTypeNode(args);
    TypeNode *cur = argscpy;
    while((args = (TypeNode*)args->next.get())){
        cur->next.reset(deepCopyTypeNode((TypeNode*)args));
        cur = (TypeNode*)cur->next.get();
    }

    //swap out fn's generic params for the concrete arg types
    auto *params = replaceParams(fd->fdn->params.get(), argscpy);

    //compile the function normally (each typevar should now be
    //substituted with its checked type from the typecheck tc)
    auto *res = c->compFn(fd);
    
    replaceParams(fd->fdn->params.get(), params);
    c->exitScope();
    return res;
}

//Provide a wrapper for function-compiling methods so that each
//function is compiled in its own isolated module
TypedValue* Compiler::compFn(FuncDecl *fd){
    callStack.push_back(fd);

    if(fd->module->name != compUnit->name){
        auto mcu = move(mergedCompUnits);

        mergedCompUnits = fd->module;
        auto *ret = compFnHelper(this, fd);
        mergedCompUnits = mcu;

        callStack.pop_back();
        return ret;
    }else{
        auto *ret = compFnHelper(this, fd);
        callStack.pop_back();
        return ret;
    }
}


/*
 *  Registers a function for later compilation
 */
TypedValue* FuncDeclNode::compile(Compiler *c){
    //check if the function is a named function.
    if(name.length() > 0){
        //if it is not, register it to be lazily compiled later (when it is called)
        name = c->funcPrefix + name;
        basename = c->funcPrefix + basename;
        c->registerFunction(this);
        //and return a void value
        return c->getVoidLiteral();
    }else{
        //Otherwise, if it is a lambda function, compile it now and return it.
        FuncDecl fd(this, c->scope, c->mergedCompUnits);
        auto *ret = c->compFn(&fd);
        fd.fdn = 0;
        return ret;
    }
}

FuncDecl* getFuncDeclFromList(list<shared_ptr<FuncDecl>> &l, string &mangledName){
    for(auto& fd : l)
        if(fd->fdn->name == mangledName)
            return fd.get();

    return 0;
}


FuncDecl* Compiler::getCurrentFunction() const{
    return callStack.back();
}



void Compiler::updateFn(TypedValue *f, string &name, string &mangledName){
    auto &list = mergedCompUnits->fnDecls[name];
    auto *fd = getFuncDeclFromList(list, mangledName);

    //TODO: free me first
    //
    //NOTE: fd here is shared between compUnit and mergedCompUnit modules
    //      so one update will update across each module
    //
    //copy the type
    fd->tv = new TypedValue(f->val, f->type);
}


TypedValue* Compiler::getFunction(string& name, string& mangledName){
    auto& list = getFunctionList(name);
    if(list.empty()) return 0;

    auto *fd = getFuncDeclFromList(list, mangledName);
    if(!fd) return 0;

    if(fd->tv) return fd->tv;

    //Function has been declared but not defined, so define it.
    return compFn(fd);
}

/*
 * Returns all FuncDecls from a list that have argc number of parameters
 * and can be accessed in the current scope.
 */
list<shared_ptr<FuncDecl>> filterByArgcAndScope(list<shared_ptr<FuncDecl>> &l, size_t argc, unsigned int scope){
    list<shared_ptr<FuncDecl>> ret;
    for(auto& fd : l){
        if(fd->scope <= scope && getTupleSize(fd->fdn->params.get()) == argc){
            ret.push_back(fd);
        }
    }
    return ret;
}


TypedValue* Compiler::getMangledFunction(string name, TypeNode *args){
    auto& fnlist = getFunctionList(name);
    if(fnlist.empty()) return 0;

    auto argc = getTupleSize(args);

    auto candidates = filterByArgcAndScope(fnlist, argc, this->scope);
    if(candidates.empty()) return 0;

    //if there is only one function now, return it.  It will be typechecked later
    if(candidates.size() == 1){
        auto& fd = candidates.front();

        //must check if this functions is generic first
        auto fnty = unique_ptr<TypeNode>(createFnTyNode(fd->fdn->params.get(), mkAnonTypeNode(TT_Void)));
        auto *params = (TypeNode*)fnty->extTy->next.get();
        auto tc = typeEq(params, args);
        
        if(tc.res == TypeCheckResult::SuccessWithTypeVars)
            return compTemplateFn(this, fd.get(), tc, args);
        else if(fd->tv)
            return fd->tv;
        else
            return compFn(fd.get());
    }

    //check for an exact match on the remaining candidates.
    string fnName = mangle(name, args);
    auto *fd = getFuncDeclFromList(candidates, fnName);
    if(fd){ //exact match
        return compFn(fd);
    }

    //Otherwise, determine which function to use by which needs the least
    //amount of implicit conversions.
    //first, perform a typecheck.  If it succeeds then the function had a generic/trait parameter
    //
    //NOTE: the current implementation will return the first generic function that matches, not necessarily
    //      the most specific one.
    for(auto& fd : candidates){
        auto fnty = unique_ptr<TypeNode>(createFnTyNode(fd->fdn->params.get(), mkAnonTypeNode(TT_Void)));
        auto *params = (TypeNode*)fnty->extTy->next.get();

        auto tc = typeEq(params, args);
        if(!!tc){
            return compTemplateFn(this, fd.get(), tc, args);
        }
    }

    //TODO
    return 0;
}


list<shared_ptr<FuncDecl>>& Compiler::getFunctionList(string& name) const{
    return mergedCompUnits->fnDecls[name];
}


/*
 *  Adds a function to the list of declared, but not defined functions.  A declared function's
 *  FuncDeclNode can be added to be compiled only when it is later called.  Useful to prevent pollution
 *  of a module with unneeded library functions.
 */
inline void Compiler::registerFunction(FuncDeclNode *fn){
    shared_ptr<FuncDecl> fd{new FuncDecl(fn, scope, mergedCompUnits)};

    compUnit->fnDecls[fn->basename].push_front(fd);
    mergedCompUnits->fnDecls[fn->basename].push_front(fd);
}
