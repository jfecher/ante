#include "function.h"

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


TypeNode* toTypeNodeList(vector<TypedValue*> &args){
    TypeNode *listBegin = 0;
    TypeNode *listCur = 0;
    
    for(auto *arg : args){
        if(listBegin){
            listCur->next.reset(copy(arg->type));
            listCur = (TypeNode*)listCur->next.get();
        }else{
            listBegin = copy(arg->type);
            listCur = listBegin;
        }
    }
    return listBegin;
}


TypedValue* Compiler::callFn(string name, vector<TypedValue*> args){
    TypedValue* fn = getMangledFunction(name, toTypeNodeVector(args));
    if(!fn) return 0;

    //vector of llvm::Value*s for the call to CreateCall at the end
    vector<Value*> vals;
    vals.reserve(args.size());

    //Loop through each arg, typecheck them, and build vals vector
    //TODO: re-arrange all args into one tuple so that typevars
    //      are matched correctly across parameters
    TypeNode *param = (TypeNode*)fn->type->extTy->next.get();
    for(auto *arg : args){
        arg = typeCheckWithImplicitCasts(this, arg, param);
        if(!arg) return 0;
        param = (TypeNode*)param->next.get();
        vals.push_back(arg->val);
    }

    return new TypedValue(builder.CreateCall(fn->val, vals), fn->type->extTy);
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
    auto *matchTy = retTy ? retTy : fd->returns[fd->returns.size()-1].first->type.get();

    for(auto pair : fd->returns){
        TypedValue *ret = pair.first;

        auto tcr = c->typeEq(matchTy, ret->type.get());
        if(!tcr){
            return (TypeNode*)c->compErr("Function " + fd->fdn->basename + " returned value of type " + 
                    typeNodeToColoredStr(ret->type) + " but was declared to return value of type " +
                    typeNodeToColoredStr(matchTy), pair.second);
        }

        if(tcr->res == TypeCheckResult::SuccessWithTypeVars){
            //TODO: copy type
            bindGenericToType(ret->type.get(), tcr->bindings);
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

    return copy(matchTy);
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
        
    //iterate through each parameter and add its value to the new scope.
    TypeNode *curTyn = 0;
    auto paramVec = vectorize(fdn->params.get());
    size_t i = 0;

    vector<Value*> preArgs;
    
    for(auto &arg : preFn->args()){
        NamedValNode *cParam = paramVec[i];
        TypeNode *paramTyNode = (TypeNode*)cParam->typeExpr.get();
        addArgAttrs(arg, paramTyNode);

        for(size_t j = 0; j < i; j++){
            if(cParam->name == paramVec[j]->name){
                return compErr("Parameter name '"+cParam->name+"' is repeated for parameters "+
                        to_string(j+1)+" and "+to_string(i+1), cParam->loc);
            }
        }
        stoVar(cParam->name, new Variable(cParam->name, new TypedValue(&arg, copy(paramTyNode)), this->scope,
                        /*nofree =*/ true, /*autoDeref = */implicitPassByRef(paramTyNode)));

        preArgs.push_back(&arg);

        if(curTyn){
            curTyn->next.reset(copy(paramTyNode));
            curTyn = (TypeNode*)curTyn->next.get();
        }else{
            fakeFnTyn->extTy->next.reset(copy(paramTyNode));
            curTyn = (TypeNode*)fakeFnTyn->extTy->next.get();
        }
        ++i;
        if(!(cParam = (NamedValNode*)cParam->next.get())) break;
    }
    
    //store a fake function var, in case this function is recursive
    auto *fakeFnTv = new TypedValue(preFn, fakeFnTyn);
    if(fdn->name.length() > 0)
        updateFn(fakeFnTv, fdn->basename, fdn->name);

    //actually compile the function, and hold onto the last value
    TypedValue *v = 0;
    try{
        v = fdn->child->compile(this);
    }catch(CtError *e){
        delete e;
        return 0;
    }

    //llvm requires explicit returns, so generate a return even if
    //the user did not in their function.
    if(!dyn_cast<ReturnInst>(v->val)){
        auto loc = getFinalLoc(fdn->child.get());

        if(v->type->type == TT_Void){
            builder.CreateRetVoid();
            fd->returns.push_back({getVoidLiteral(), loc});
        }else{
            builder.CreateRet(v->val);
            fd->returns.push_back({v, loc});
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
    TypeNode *newFnTyn = copy(fakeFnTyn);
    TypeNode *params = (TypeNode*)newFnTyn->extTy->next.release();

    retTy->next.reset(params);
    newFnTyn->extTy.reset(retTy);


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
    fnTy->extTy.reset(retTy ? copy(retTy) : mkAnonTypeNode(TT_Void));

    TypeNode *curTyn = fnTy->extTy.get();
    while(params && params->typeExpr.get()){
        curTyn->next.reset(copy((TypeNode*)params->typeExpr.get()));
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

    TypedValue *fn;
    if(VarNode *vn = dynamic_cast<VarNode*>(ppn->expr.get())){
        if(vn->name == "inline"){
            fn = c->compFn(fd);
            if(!fn) return 0;
            ((Function*)fn->val)->addFnAttr("always_inline");
        }else if(vn->name == "run"){
            fn = c->compFn(fd);
            if(!fn) return 0;
            
            auto *mod = c->module.release();

            c->module.reset(new llvm::Module(fdn->name, *c->ctxt));
            auto *recomp = c->compFn(fd);

            c->jitFunction((Function*)recomp->val);
            c->module.reset(mod);
        }else if(vn->name == "macro" or vn->name == "meta"){
            auto *ext = createFnTyNode(fdn->params.get(), (TypeNode*)fdn->type.get());
            ext->type = TT_MetaFunction;
            fn = new TypedValue(nullptr, ext);
        }else{
            return c->compErr("Unrecognized compiler directive '"+vn->name+"'", vn->loc);
        }
    
        //put back the preproc node modifier
        fdn->modifiers.release();
        fdn->modifiers.reset(ppn);
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
            auto *dt = c->lookupType(retNode);
            bindGenericToType(retNode, retNode->params, dt);
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

        auto paramVec = vectorize(paramsBegin);
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
            c->stoVar(cParam->name, new Variable(cParam->name, new TypedValue(&arg, copy(paramTyNode)),
                        c->scope, /*nofree = */true, /*autoDeref = */implicitPassByRef(paramTyNode)));
           
            i++;
            if(!(cParam = (NamedValNode*)cParam->next.get())) break;
        }

        //actually compile the function, and hold onto the last value
        TypedValue *v;
        try{
            v = fdn->child->compile(c);
        }catch(CtError *e){
            c->builder.SetInsertPoint(caller);
            throw e;
        }
        
        //push the final value as a return, explicit returns are already added in RetNode::compile
        if(retNode && !dyn_cast<ReturnInst>(v->val)){
            auto loc = getFinalLoc(fdn->child.get());

            if(retNode->type == TT_Void){
                c->builder.CreateRetVoid();
                fd->returns.push_back({c->getVoidLiteral(), loc});
            }else{
                c->builder.CreateRet(v->val);
                fd->returns.push_back({v, loc});
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


vector<TypeNode*> bindParams(NamedValNode *params, const vector<pair<string,unique_ptr<TypeNode>>> &bindings){
    vector<TypeNode*> oldParams;

    while(params){
        auto *unboundTy = (TypeNode*)params->typeExpr.release();
        oldParams.push_back(unboundTy);

        auto *boundTy = copy(unboundTy);
        bindGenericToType(boundTy, bindings);
        params->typeExpr.reset(boundTy);

        params = (NamedValNode*)params->next.get();
    }
    return oldParams;
}

void unbindParams(NamedValNode *params, vector<TypeNode*> replacements){
    size_t i = 0;
    while(params){
        params->typeExpr.reset(replacements[i]);

        params = (NamedValNode*)params->next.get();
        i++;
    }
}


TypedValue* compTemplateFn(Compiler *c, FuncDecl *fd, TypeCheckResult &tc, TypeNode *args){
    //Each binding from the typecheck results needs to be declared as a typevar in the
    //function's scope, but compFn sets this scope later on, so the needed bindings are
    //instead stored as fake obj bindings to be declared later in compFn
    size_t tmp_bindings_loc = fd->obj_bindings.size();
    for(auto& pair : tc->bindings){
        fd->obj_bindings.push_back({pair.first, pair.second.get()});
    }

    TypeNode *argscpy = copy(args);
    TypeNode *cur = argscpy;
    while((args = (TypeNode*)args->next.get())){
        cur->next.reset(copy((TypeNode*)args));
        cur = (TypeNode*)cur->next.get();
    }

    //swap out fn's generic params for the concrete arg types
    auto unboundParams = bindParams(fd->fdn->params.get(), tc->bindings);
    auto *retTy = (TypeNode*)fd->fdn->type.release();

    auto *boundRetTy = copy(retTy);
    bindGenericToType(boundRetTy, tc->bindings);
    fd->fdn->type.reset(boundRetTy);


    //compile the function normally (each typevar should now be
    //substituted with its checked type from the typecheck tc)
    TypedValue *res;
    try{
        res = c->compFn(fd);
    }catch(CtError *e){
        //cleanup, reset bindings
        while(fd->obj_bindings.size() > tmp_bindings_loc){
            fd->obj_bindings.pop_back();
        }
        fd->fdn->type.reset(retTy);
        unbindParams(fd->fdn->params.get(), unboundParams);
        throw e;
    }

    auto ls = typeNodeToColoredStr(res->type.get());

    //cleanup, reset bindings
    while(fd->obj_bindings.size() > tmp_bindings_loc){
        fd->obj_bindings.pop_back();
    }
    
    fd->fdn->type.reset(retTy);
    unbindParams(fd->fdn->params.get(), unboundParams);
    return res;
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
    for(auto& fd : l){
        if(fd->fdn->name == mangledName)
            return fd.get();
    }

    return 0;
}


void declareBindings(Compiler *c, vector<pair<string,TypeNode*>> &bindings){
    for(auto &p : bindings){
        c->stoTypeVar(p.first, p.second);
    }
}


//Provide a wrapper for function-compiling methods so that each
//function is compiled in its own isolated module
TypedValue* Compiler::compFn(FuncDecl *fd){
    compCtxt->callStack.push_back(fd);
    auto *continueLabels = compCtxt->continueLabels.release();
    auto *breakLabels = compCtxt->breakLabels.release();
    compCtxt->continueLabels = llvm::make_unique<vector<BasicBlock*>>();
    compCtxt->breakLabels = llvm::make_unique<vector<BasicBlock*>>();
    int callingFnScope = fnScope;
    
    enterNewScope();
    fnScope = scope;

    //Propogate type var bindings of the method obj into the function scope
    declareBindings(this, fd->obj_bindings);

    if(fd->module->name != compUnit->name){
        auto mcu = move(mergedCompUnits);

        mergedCompUnits = fd->module;
        auto *ret = compFnHelper(this, fd);
        mergedCompUnits = mcu;

        compCtxt->callStack.pop_back();
        compCtxt->continueLabels.reset(continueLabels);
        compCtxt->breakLabels.reset(breakLabels);
        fnScope = callingFnScope;
        exitScope();
        return ret;
    }else{
        auto *ret = compFnHelper(this, fd);
        compCtxt->callStack.pop_back();
        compCtxt->continueLabels.reset(continueLabels);
        compCtxt->breakLabels.reset(breakLabels);
        fnScope = callingFnScope;
        exitScope();
        return ret;
    }
}


FuncDecl* Compiler::getCurrentFunction() const{
    return compCtxt->callStack.back();
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
    //fd->tv = compFn(fd);
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


template<typename T>
vector<T*> vectorize(T *args){
    vector<T*> ret;
    while(args){
        ret.push_back(args);
        args = (T*)args->next.get();
    }
    return ret;
}


template<typename T>
T* toList(vector<T*> &nodes){
    T *begin = 0;
    T *cur;

    for(auto node : nodes){
        if(begin){
            cur->next.reset(copy(node));
            cur = (T*)cur->next.get();
        }else{
            begin = copy(node);
            cur = begin;
        }
    }
    return begin;
}


TypedValue* Compiler::getMangledFunction(string name, vector<TypeNode*> args){
    auto& fnlist = getFunctionList(name);
    if(fnlist.empty()) return 0;

    auto argc = args.size();

    auto candidates = filterByArgcAndScope(fnlist, argc, this->scope);
    if(candidates.empty()) return 0;

    //if there is only one function now, return it.  It will be typechecked later
    if(candidates.size() == 1){
        auto& fd = candidates.front();

        //must check if this functions is generic first
        auto fnty = unique_ptr<TypeNode>(createFnTyNode(fd->fdn->params.get(), mkAnonTypeNode(TT_Void)));
        auto *params = (TypeNode*)fnty->extTy->next.get();
        auto tc = typeEq(vectorize(params), args);

        if(tc->res == TypeCheckResult::SuccessWithTypeVars)
            return compTemplateFn(this, fd.get(), tc, args[0]);
        else if(!tc)
            return nullptr;
        else if(fd->tv)
            return fd->tv;
        else{
            //fd->tv = compFn(fd.get());
            return compFn(fd.get());
        }
    }

    //check for an exact match on the remaining candidates.
    string fnName = mangle(name, args);
    auto *fd = getFuncDeclFromList(candidates, fnName);
    if(fd){ //exact match
        if(!fd->tv){
            //fd->tv = compFn(fd);
            return compFn(fd);
        }else return fd->tv;
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

        auto tc = typeEq(vectorize(params), args);
        if(!!tc){
            return compTemplateFn(this, fd.get(), tc, toList(args));
        }
    }

    //TODO
    return 0;
}


list<shared_ptr<FuncDecl>>& Compiler::getFunctionList(string& name) const{
    return mergedCompUnits->fnDecls[name];
}


TypedValue* Compiler::getCastFn(TypeNode *from_ty, TypeNode *to_ty){
    string fnBaseName = (to_ty->params.empty() ? typeNodeToStr(to_ty) : to_ty->typeName) + "_init";
    string mangledName = mangle(fnBaseName, from_ty);

    //Search for the exact function, otherwise there would be implicit casts calling several implicit casts on a single parameter
    auto *fd = getFuncDecl(fnBaseName, mangledName);

    if(!fd) return nullptr;
    TypedValue *tv;

    if(!to_ty->params.empty() and !fd->obj->params.empty()){
        TypeNode *unbound_obj = fd->obj;
        fd->obj = to_ty;

        size_t argc = to_ty->params.size();
        if(argc != unbound_obj->params.size())
            return nullptr;

        size_t i = 0;
        for(auto& tn : unbound_obj->params){
            TypeNode *bound_ty = to_ty->params[i].get();

            fd->obj_bindings.push_back(pair<string,TypeNode*>(tn->typeName, bound_ty));
            i++;
        }

        tv = compFn(fd);

        //TODO: if fd is a meta function that is a method of a generic object then the generic
        //      parameters of the object will be unbound here and untraceable when the function is
        //      lazily compiled at the callsite
        fd->obj = unbound_obj;
        fd->obj_bindings.clear();
        fd->tv = nullptr;
    }else{
        tv = fd->tv;
        if(!tv)
            tv = compFn(fd);
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

    return getFuncDeclFromList(list, mangledName);
}

/*
 *  Adds a function to the list of declared, but not defined functions.  A declared function's
 *  FuncDeclNode can be added to be compiled only when it is later called.  Useful to prevent pollution
 *  of a module with unneeded library functions.
 */
inline void Compiler::registerFunction(FuncDeclNode *fn){
    //check for redeclaration
    auto *redecl = getFuncDecl(fn->basename, fn->name);
    
    if(redecl and redecl->fdn->name == fn->name){
        compErr("Function " + fn->name + " was redefined", fn->loc);
        return;
    }

    shared_ptr<FuncDecl> fd{new FuncDecl(fn, scope, mergedCompUnits)};
    fd->obj = compCtxt->obj;

    compUnit->fnDecls[fn->basename].push_front(fd);
    mergedCompUnits->fnDecls[fn->basename].push_front(fd);
}
