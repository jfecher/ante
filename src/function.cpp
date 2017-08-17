#include "function.h"
using namespace std;
using namespace llvm;

namespace ante {

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
    TypedValue* fn = getMangledFn(name, toTypeNodeVector(args));
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
vector<Type*> getParamTypes(Compiler *c, FuncDecl *fd, NamedValNode *nvn, size_t paramCount){
    vector<Type*> paramTys;
    paramTys.reserve(paramCount);

    for(size_t i = 0; i < paramCount && nvn; i++){

        TypeNode *paramTyNode = (TypeNode*)nvn->typeExpr.get();
        if(paramTyNode == (void*)1){ //self parameter
            //Self parameters originally have 0x1 as their TypeNodes, but
            //this should be replaced when FuncDeclNode::compile is called.
            //Throw an error if that check was somehow bypassed
            c->compErr("Stray self parameter", nvn->loc);
        }else if(paramTyNode){
            auto *type = c->typeNodeToLlvmType(paramTyNode);
            auto *correctedType = parameterize(type, paramTyNode);
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
        updateFn(fakeFnTv, fd, fdn->basename, fdn->name);

    //actually compile the function, and hold onto the last value
    TypedValue *v = fdn->child->compile(this);

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
        updateFn(ret, fd, fdn->basename, fdn->name);


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
        }else if(vn->name == "on_fn_decl"){
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

    vector<Type*> paramTys = getParamTypes(c, fd, paramsBegin, nParams);

    if(paramTys.size() > 0 && !paramTys.back()){ //varargs fn
        fdn->varargs = true;
        paramTys.pop_back();
    }

    if(!retNode){
        try{
            auto *ret = c->compLetBindingFn(fd, paramTys);
            c->builder.SetInsertPoint(caller);
            return ret;
        }catch(CtError *e){
            c->builder.SetInsertPoint(caller);
            throw e;
        }
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
    c->updateFn(ret, fd, fdn->basename, fdn->name);

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


NamedValNode* bindParams(unique_ptr<NamedValNode> &params, const vector<pair<string,unique_ptr<TypeNode>>> &bindings){
    NamedValNode *newParams = 0;
    NamedValNode *curParam  = 0;
    NamedValNode *oldParams = params.get();

    while(oldParams){
        auto *unboundTy = (TypeNode*)oldParams->typeExpr.get();

        auto *boundTy = copy(unboundTy);
        bindGenericToType(boundTy, bindings);

        auto *node = new NamedValNode(oldParams->loc, oldParams->name, boundTy);
        if(newParams){
            curParam->next.reset(node);
        }else{
            newParams = node;
        }

        curParam = node;
        oldParams = (NamedValNode*)oldParams->next.get();
    }
    return newParams;
}

FuncDecl* shallow_copy(FuncDecl* fd){
    auto *fdnCpy = new FuncDeclNode(fd->fdn);

    FuncDecl *cpy = new FuncDecl(fdnCpy, fd->scope, fd->module, nullptr);
    cpy->obj_bindings = fd->obj_bindings;
    cpy->obj = fd->obj;
    return cpy;
}


TypedValue* compTemplateFn(Compiler *c, FuncDecl *fd, TypeCheckResult &tc, TypeNode *args){
    fd = shallow_copy(fd);

    //Each binding from the typecheck results needs to be declared as a typevar in the
    //function's scope, but compFn sets this scope later on, so the needed bindings are
    //instead stored as fake obj bindings to be declared later in compFn
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
    auto boundParams = bindParams(fd->fdn->params, tc->bindings);

    //The unbound template owns the original params
    fd->fdn->params.release();
    fd->fdn->params.reset(boundParams);

    //test if bound variant is already compiled
    string mangled = mangle(fd->fdn->basename, fd->fdn->params.get());
    if(TypedValue *fn = c->getFunction(fd->fdn->basename, mangled))
        return fn;

    fd->fdn->name = mangled;

    //bind the return type if necessary
    TypeNode *retTy = 0;
    if(fd->fdn->type.get()){
        retTy = (TypeNode*)fd->fdn->type.release();
        auto *boundRetTy = copy(retTy);
        bindGenericToType(boundRetTy, tc->bindings);
        fd->fdn->type.reset(boundRetTy);
    }

    //compile the function normally (each typevar should now be
    //substituted with its checked type from the typecheck tc)
    return c->compFn(fd);
}

//Defined in compiler.cpp
void manageSelfParam(Compiler *c, FuncDeclNode *fdn);

/*
 *  Registers a function for later compilation
 */
TypedValue* FuncDeclNode::compile(Compiler *c){
    //check if the function is a named function.
    if(name.length() > 0){
        manageSelfParam(c, this);

        name = c->funcPrefix + name;
        basename = c->funcPrefix + basename;

        c->registerFunction(this);
        return c->getVoidLiteral();
    }else{
        //Otherwise, if it is a lambda function, compile it now and return it.
        FuncDecl *fd = new FuncDecl(this, c->scope, c->mergedCompUnits);
        auto *ret = c->compFn(fd);
        fd->fdn = 0;
        return ret;
    }
}

FuncDecl* getFuncDeclFromVec(vector<shared_ptr<FuncDecl>> &l, string &mangledName){
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



void Compiler::updateFn(TypedValue *f, FuncDecl *fd, string &name, string &mangledName){
    auto &list = mergedCompUnits->fnDecls[name];
    auto *vec_fd = getFuncDeclFromVec(list, mangledName);
    if(vec_fd){
        vec_fd->tv = new TypedValue(f->val, f->type);
    }else{
        fd->tv = new TypedValue(f->val, f->type);
        list.push_back(shared_ptr<FuncDecl>(fd));
    }
}


TypedValue* Compiler::getFunction(string& name, string& mangledName){
    auto& list = getFunctionList(name);
    if(list.empty()) return 0;

    auto *fd = getFuncDeclFromVec(list, mangledName);
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


TypeNode* toList(vector<TypeNode*> &nodes){
    TypeNode *begin = 0;
    TypeNode *cur;

    for(auto node : nodes){
        if(begin){
            cur->next.reset(copy(node));
            cur = (TypeNode*)cur->next.get();
        }else{
            begin = copy(node);
            cur = begin;
        }
    }

    return begin;
}

vector<pair<TypeCheckResult,FuncDecl*>> filterHighestMatches(vector<pair<TypeCheckResult,FuncDecl*>> &matches){
    unsigned int highestMatch = 0;
    vector<pair<TypeCheckResult,FuncDecl*>> highestMatches;

    for(auto &tcr : matches){
        if(!!tcr.first and tcr.first->matches >= highestMatch){
            if(tcr.first->matches > highestMatch){
                highestMatch = tcr.first->matches;
                highestMatches.clear();
            }
            highestMatches.push_back(tcr);
        }
    }
    return highestMatches;
}


vector<pair<TypeCheckResult,FuncDecl*>>
filterBestMatches(Compiler *c, vector<shared_ptr<FuncDecl>> candidates, vector<TypeNode*> args){
    vector<pair<TypeCheckResult,FuncDecl*>> results;
    results.reserve(candidates.size());

    for(auto& fd : candidates){
        auto fnty = unique_ptr<TypeNode>(createFnTyNode(fd->fdn->params.get(), mkAnonTypeNode(TT_Void)));
        auto *params = (TypeNode*)fnty->extTy->next.get();

        auto tc = c->typeEq(vectorize(params), args);
        results.push_back({tc, fd.get()});
    }

    return filterHighestMatches(results);
}


FuncDecl* Compiler::getMangledFuncDecl(string name, vector<TypeNode*> args){
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
    if(fd){ //exact match
        if(!fd->tv)
            fd->tv = compFnWithArgs(this, fd, args);

        return fd;
    }

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
TypedValue* compFnWithArgs(Compiler *c, FuncDecl *fd, vector<TypeNode*> args){
    //must check if this functions is generic first
    auto fnty = unique_ptr<TypeNode>(createFnTyNode(fd->fdn->params.get(), mkAnonTypeNode(TT_Void)));
    auto *params = (TypeNode*)fnty->extTy->next.get();
    auto tc = c->typeEq(vectorize(params), args);

    if(tc->res == TypeCheckResult::SuccessWithTypeVars)
        return compTemplateFn(c, fd, tc, toList(args));
    else if(!tc) //tc->res == TypeCheckResult::Failure
        return nullptr;
    else if(fd->tv)
        return fd->tv;
    else
        return c->compFn(fd);
}


TypedValue* Compiler::getMangledFn(string name, vector<TypeNode*> args){
    auto *fd = getMangledFuncDecl(name, args);
    if(!fd) return nullptr;

    return compFnWithArgs(this, fd, args);
}


vector<shared_ptr<FuncDecl>>& Compiler::getFunctionList(string& name) const{
    return mergedCompUnits->fnDecls[name];
}


TypeNode* tupleToList(TypeNode *tup){
    if(tup->type == TT_Tuple)
        tup = tup->extTy.get();
    else if(tup->type == TT_Void)
        tup = nullptr;

    return tup;
}


FuncDecl* Compiler::getCastFuncDecl(TypeNode *from_ty, TypeNode *to_ty){
    string fnBaseName = getCastFnBaseName(to_ty);
    TypeNode *argList = tupleToList(from_ty);
    return getMangledFuncDecl(fnBaseName, vectorize(argList));
}


TypedValue* Compiler::getCastFn(TypeNode *from_ty, TypeNode *to_ty, FuncDecl *fd){
    if(!fd)
        fd = getCastFuncDecl(from_ty, to_ty);

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

        //must check if this functions is generic first
        auto *args = tupleToList(from_ty);
        tv = compFnWithArgs(this, fd, vectorize(args));

        //TODO: if fd is a meta function that is a method of a generic object then the generic
        //      parameters of the object will be unbound here and untraceable when the function is
        //      lazily compiled at the callsite
        fd->obj = unbound_obj;
        fd->obj_bindings.clear();
        fd->tv = nullptr;
    }else{
        tv = fd->tv;
        if(!tv){
            auto *args = tupleToList(from_ty);
            tv = compFnWithArgs(this, fd, vectorize(args));
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

TypedValue* compMetaFunctionResult(Compiler *c, LOC_TY &loc, string &baseName, string &mangledName, vector<TypedValue*> &typedArgs);

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

    FuncDecl *fdRaw = new FuncDecl(fn, scope, mergedCompUnits);
    shared_ptr<FuncDecl> fd{fdRaw};
    fd->obj = compCtxt->obj;
    
    for(auto &hook : ctCtxt->on_fn_decl_hook){
        cout << "Iterating over hook " << hook->fdn->basename;
        Value *fd_val = builder.getInt64((unsigned long)fdRaw);
        vector<TypedValue*> args;
        args.push_back(new TypedValue(fd_val, mkDataTypeNode("FuncDecl")));
        compMetaFunctionResult(this, hook->fdn->loc, hook->fdn->basename, hook->fdn->name, args);
    }

    for(auto *m : *fn->modifiers){
        if(PreProcNode *ppn = dynamic_cast<PreProcNode*>(m)){
            VarNode *vn;
            if((vn = dynamic_cast<VarNode*>(ppn->expr.get())) and vn->name == "on_fn_decl"){
                ctCtxt->on_fn_decl_hook.push_back(fd);
            }
        }
    }

    compUnit->fnDecls[fn->basename].push_back(fd);
    mergedCompUnits->fnDecls[fn->basename].push_back(fd);
}

} //end of namespace ante
