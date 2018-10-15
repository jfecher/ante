#include "function.h"
#include "compapi.h"
#include "scopeguard.h"

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

/*
 * Return true if the given function type
 * is a user-declared ante function or a
 * compiler-api function (TT_MetaFunction)
 */
bool isCompileTimeFunction(AnType *fty){
    return fty->typeTag == TT_MetaFunction or
        (fty->typeTag == TT_Function && fty->hasModifier(Tok_Ante));
}

bool isCompileTimeFunction(TypedValue &tv){
    return isCompileTimeFunction(tv.type);
}

vector<AnType*> toTypeVector(vector<TypedValue> const& tvs){
    auto ret = vecOf<AnType*>(tvs.size());
    for(const auto tv : tvs)
        ret.push_back(tv.type);
    return ret;
}


TypedValue Compiler::callFn(string name, vector<TypedValue> args){
    auto typeVec = toTypeVector(args);
    TypedValue fn = getMangledFn(name, typeVec);
    if(!fn) return fn;

    //vector of llvm::Value*s for the call to CreateCall at the end
    auto vals = vecOf<Value*>(args.size());

    //Loop through each arg, typecheck them, and build vals vector
    //TODO: re-arrange all args into one tuple so that typevars
    //      are matched correctly across parameters
    auto *fnty = try_cast<AnFunctionType>(fn.type);
    for(size_t i = 0; i < args.size(); i++){
        auto arg = args[i];
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
    auto *nvn = fd->getFDN()->params.get();
    while(nvn){
        TypeNode *paramTyNode = (TypeNode*)nvn->typeExpr.get();
        if(paramTyNode == (void*)1){ //self parameter
            //Self parameters originally have 0x1 as their TypeNodes, but
            //this should be replaced when visit(FuncDeclNode*) is called.
            //Throw an error if that check was somehow bypassed
            c->compErr("Stray self parameter", nvn->loc);
        }else if(paramTyNode){
            auto *antype = toAnType(paramTyNode);
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
    if(paramTyNode->typeTag == TT_Function){
        arg.addAttr(Attribute::AttrKind::NoCapture);

        //TODO: re-add
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

LOC_TY getFinalLoc(Node *n){
    auto *bop = dynamic_cast<BinOpNode*>(n);

    if(!bop){
        if(BlockNode* bn = dynamic_cast<BlockNode*>(n)){
            n = bn->block.get();
            bop = dynamic_cast<BinOpNode*>(n);
        }
    }

    return (bop && bop->op == ';') ? bop->rval->loc : n->loc;
}


//swap the bodies of the two functions and delete the former.
void moveFunctionBody(Function *src, Function *dest){
    dest->getBasicBlockList().splice(dest->begin(), src->getBasicBlockList());
    src->getBasicBlockList().clearAndLeakNodesUnsafely();
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
TypedValue compFnWithModifiers(Compiler *c, FuncDecl *fd, ModNode *mod){
    //remove the preproc node at the front of the modifier list so that the call to
    //compFn does not call this function in an infinite loop
    auto *fdn = fd->getFDN();
    fdn->modifiers.back().release();
    fdn->modifiers.pop_back();

    TypedValue fn;
    if(mod->isCompilerDirective()){
        if(VarNode *vn = dynamic_cast<VarNode*>(mod->directive.get())){
            if(vn->name == "inline"){
                fn = c->compFn(fd);
                if(!fn) return fn;
                ((Function*)fn.val)->addFnAttr(Attribute::AttrKind::AlwaysInline);
            }else if(vn->name == "run"){
                fn = c->compFn(fd);
                if(!fn) return fn;

                auto *mod = c->module.release();

                c->module.reset(new llvm::Module(fd->getMangledName(), *c->ctxt));
                auto recomp = c->compFn(fd);

                c->jitFunction((Function*)recomp.val);
                c->module.reset(mod);
            }else if(vn->name == "on_fn_decl"){
                auto *rettn = (TypeNode*)fdn->returnType.get();
                auto *fnty = AnFunctionType::get(toAnType(rettn), fdn->params.get(), true);
                fn = TypedValue(nullptr, fnty);
            }else{
                fdn->modifiers.emplace_back(mod);
                return c->compErr("Unrecognized compiler directive '"+vn->name+"'", vn->loc);
            }

            return fn;
        }else{
            fdn->modifiers.emplace_back(mod);
            return c->compErr("Unrecognized compiler directive", mod->loc);
        }
    // ppn is a normal modifier
    }else{
        if(mod->mod == Tok_Ante){
            if(c->isJIT){
                if(capi::lookup(fd->getName())){
                    fn = fd->tval.val ? fd->tval : c->compFn(fd);
                    //Tag as TT_MetaFunction
                    auto *oldTy = try_cast<AnFunctionType>(fn.type);
                    fn.type = AnFunctionType::get(oldTy->retTy, oldTy->extTys, true);
                    fd->tval = fn;
                }else{
                    fn = c->compFn(fd);
                    fn.type = (AnType*)fn.type->addModifier(Tok_Ante);
                    fd->tval.type = fn.type;
                }
            }else{
                auto *rettn = (TypeNode*)fdn->returnType.get();
                AnType *fnty;
                if(capi::lookup(fd->getName())){
                    fnty = AnFunctionType::get(toAnType(rettn), fdn->params.get(), true);
                }else{
                    fnty = AnFunctionType::get(toAnType(rettn), fdn->params.get(), false);
                    fnty = (AnType*)fnty->addModifier(Tok_Ante);
                }
                fn = TypedValue(nullptr, fnty);
                fd->tval.type = fnty;
            }
        }else{
            fn = c->compFn(fd);
        }
        fdn->modifiers.emplace_back(mod);
        return fn;
    }
}


TypedValue compFnHelper(Compiler *c, FuncDecl *fd){
    BasicBlock *caller = c->builder.GetInsertBlock();
    auto *fdn = fd->getFDN();

    if(!fdn->modifiers.empty()){
        auto ret = compFnWithModifiers(c, fd, fdn->modifiers.back().get());
        c->builder.SetInsertPoint(caller);
        return ret;
    }

    //Get and translate the function's return type to an llvm::Type*
    TypeNode *retNode = (TypeNode*)fdn->returnType.get();

    vector<Type*> paramTys = getParamTypes(c, fd);

    if(paramTys.size() > 0 && !paramTys.back()){ //varargs fn
        fdn->varargs = true;
        paramTys.pop_back();
    }

    AnFunctionType *fnTy = try_cast<AnFunctionType>(fdn->getType());
    AnType *anRetTy = fnTy->retTy;

    //llvm return type and function type corresponding to the AnTypes above
    Type *retTy = c->anTypeToLlvmType(anRetTy);

    FunctionType *ft = FunctionType::get(retTy, paramTys, fdn->varargs);
    Function *f = Function::Create(ft, Function::ExternalLinkage, fd->getMangledName(), c->module.get());
    f->addFnAttr(Attribute::AttrKind::NoUnwind);
    addAllArgAttrs(f, fdn->params.get());


    TypedValue ret{f, fnTy};
    c->updateFn(ret, fd, fdn->name, fd->getMangledName());

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
                    : toAnType(paramTyNode);

            cParam->decls[0]->tval = {&arg, paramTy};
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

            if(retNode->typeTag == TT_Void){
                c->builder.CreateRetVoid();
                fd->returns.push_back({c->getVoidLiteral(), loc});
            }else{
                v.val = c->builder.CreateRet(v.val);
                fd->returns.push_back({v, loc});
            }
        }
    }

    c->builder.SetInsertPoint(caller);
    return ret;
}


TypedValue FuncDecl::getOrCompileFn(Compiler *c){
    if(this->tval) return tval;
    c->compFn(this);
    return tval;
}


FuncDecl* shallow_copy(FuncDecl* fd, string &mangledName){
    FuncDecl *cpy = new FuncDecl(fd->getFDN(), mangledName, fd->module);
    cpy->obj = fd->obj;
    return cpy;
}

//Defined in compiler.cpp
string manageSelfParam(Compiler *c, FuncDeclNode *fdn, string &mangledName);


/**
 * Returns true if the given function name is a declaration
 * and not a definition
 */
bool isDecl(string &name){
    return !name.empty() && name.back() == ';';
}


void CompilingVisitor::visit(FuncDeclNode *n){}


FuncDecl* getFuncDeclFromVec(vector<shared_ptr<FuncDecl>> &l, string const& mangledName){
    for(auto& fd : l){
        if(fd->getMangledName() == mangledName)
            return fd.get();
    }
    return 0;
}


//Provide a wrapper for function-compiling methods so that each
//function is compiled in its own isolated module
TypedValue Compiler::compFn(FuncDecl *fd){
    compCtxt->callStack.push_back(fd);
    auto *continueLabels = compCtxt->continueLabels.release();
    auto *breakLabels = compCtxt->breakLabels.release();
    compCtxt->continueLabels = llvm::make_unique<vector<BasicBlock*>>();
    compCtxt->breakLabels = llvm::make_unique<vector<BasicBlock*>>();

    DEFER(
        compCtxt->callStack.pop_back();
        compCtxt->continueLabels.reset(continueLabels);
        compCtxt->breakLabels.reset(breakLabels);
    );

    TMP_SET(this->fnScope, this->scope);
    return compFnHelper(this, fd);
}


FuncDecl* Compiler::getCurrentFunction() const{
    return compCtxt->callStack.back();
}


void Compiler::updateFn(TypedValue &f, FuncDecl *fd, string const& name, string const& mangledName){
    //TODO: remove entirely
}


TypedValue Compiler::getFunction(string const& name, string const& mangledName){
    auto list = getFunctionList(name);
    if(list.empty()) return {};

    auto *fd = getFuncDeclFromVec(list, mangledName);
    if(!fd) return {};

    if(fd->tval) return fd->tval;

    //Function has been declared but not defined, so define it.
    //fd->tv = compFn(fd);
    return compFn(fd);
}

/*
 * Returns all FuncDecls from a list that have argc number of parameters
 * and can be accessed in the current scope.
 */
vector<shared_ptr<FuncDecl>> filterByArgc(vector<shared_ptr<FuncDecl>> &l, size_t argc, unsigned int scope){
    vector<shared_ptr<FuncDecl>> ret;
    for(auto& fd : l){
        if(getTupleSize(fd->getFDN()->params.get()) == argc){
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

FuncDecl* Compiler::getMangledFuncDecl(string name, vector<AnType*> &args){
    auto fnlist = getFunctionList(name);
    if(fnlist.empty()) return 0;

    auto candidates = filterByArgc(fnlist, args.size(), scope);
    if(candidates.empty()) return 0;

    //if there is only one function now, return it.  It will be typechecked later
    if(candidates.size() == 1)
        return candidates.front().get();

    //check for an exact match on the remaining candidates.
    string fnName = mangle(name, args);
    auto *fd = getFuncDeclFromVec(candidates, fnName);
    if(fd) //exact match
        return fd;

    throw CtError();
    //TODO: possibly return all functions considered for better error checking
    return nullptr;
}


/*
 * Compile a possibly-generic function with given arg types
 */
TypedValue compFnWithArgs(Compiler *c, FuncDecl *fd, vector<AnType*> args){
    if(fd->tval.val)
        return fd->tval;
    else
        return c->compFn(fd);
}


TypedValue Compiler::getMangledFn(string name, vector<AnType*> &args){
    auto *fd = getMangledFuncDecl(name, args);
    if(!fd) return {};

    return compFnWithArgs(this, fd, args);
}


vector<shared_ptr<FuncDecl>> Compiler::getFunctionList(string const& name) const{
    //TODO: remove entirely
    // return mergedCompUnits->fnDecls[name];
    return {};
}

/*
 * Returns the FuncDecl* of a given name/basename pair
 * returns nullptr if specified function is not found
 */
FuncDecl* Compiler::getFuncDecl(string baseName, string mangledName){
    auto list = getFunctionList(baseName);
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
    //TODO: remove entirely
    auto *redecl = getFuncDecl(fn->name, mangledName);

    if(redecl && redecl->getMangledName() == mangledName){
        compErr("Function " + fn->name + " was redefined", fn->loc);
        return;
    }

    //FuncDecl *fdRaw = new FuncDecl(fn, mangledName, scope, mergedCompUnits);
    //shared_ptr<FuncDecl> fd{fdRaw};
    //fd->obj = compCtxt->obj;

    // compUnit->fnDecls[fn->name].push_back(fd);
    // mergedCompUnits->fnDecls[fn->name].push_back(fd);
}

} //end of namespace ante
