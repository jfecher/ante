/*
 *      jitlinker.cpp
 * Provides some helper functions for linking
 * an llvm::Module without destroying the src
 * and only linking needed functions.
 */
#include "jitlinker.h"
namespace ante {

/*
 * Returns a new module containing shallow copies of the given module's
 * traits. Copies the FuncDecls and UserTypes one level deeper so that
 * when the FuncDecl is marked as compiled the change is not performed
 * across every Compiler instance that imported the function.
 */
shared_ptr<ante::Module> copyModuleFuncDecls(const shared_ptr<ante::Module> &mod){
    auto ret = make_shared<ante::Module>();
    ret->name = mod->name;
    ret->traits = mod->traits;

    //copy each userType except for the LLVMContext-specific llvmTypes field
    for(auto &pair : mod->userTypes){
        auto dt = pair.second;
        auto dt_cpy = make_shared<DataType>(dt->name, dt->fields, dt->tyn.get());
        dt_cpy->traitImpls = dt->traitImpls;
        dt_cpy->tags = dt->tags;
        dt_cpy->generics = dt->generics;
        dt_cpy->llvmTypes = dt->llvmTypes;
        ret->userTypes[pair.first] = dt_cpy;
    }

    for(auto &pair : mod->fnDecls){
        for(auto &fd : pair.second){
            auto fd_cpy = make_shared<FuncDecl>(fd->fdn, fd->scope, ret);
            fd_cpy->obj = fd->obj;
            fd_cpy->obj_bindings = fd->obj_bindings;
            ret->fnDecls[pair.first].push_back(fd_cpy);
        }
    }

    return ret;
}

vector<shared_ptr<ante::Module>>
copyModuleFuncDecls(const vector<shared_ptr<ante::Module>> &mods){
    vector<shared_ptr<ante::Module>> ret;
    for(auto &m : mods){
        ret.push_back(copyModuleFuncDecls(m));
    }
    return ret;
}


shared_ptr<unordered_map<string, shared_ptr<ante::Module>>>
copyModuleFuncDecls(const shared_ptr<unordered_map<string, shared_ptr<ante::Module>>> &varTable){
    auto ret = make_shared<unordered_map<string, shared_ptr<ante::Module>>>();
    for(auto &pair : *varTable){
        (*ret)[pair.first] = copyModuleFuncDecls(pair.second);
    }
    return ret;
}

void copyDecls(const Compiler *src, Compiler *dest){
    //dest->ctxt = src->ctxt;

    dest->compUnit = copyModuleFuncDecls(src->compUnit);
    dest->mergedCompUnits = copyModuleFuncDecls(src->mergedCompUnits);
    dest->imports = copyModuleFuncDecls(src->imports);
    dest->allCompiledModules = copyModuleFuncDecls(src->allCompiledModules);
}

/*
 *  Creates a copy of fdn with all compiler directives removed
 */
Node* stripCompilerDirectives(FuncDeclNode *fdn){
    Node *mods_begin = 0;
    Node *preprocs_begin = 0;

    Node *mods = 0;
    Node *preprocs = 0;

    //Go through all of the function's modifiers and separate it
    //into two lists.  One for compiler directives (preprocs) and
    //the other for normal modifiers
    Node *cur = fdn->modifiers.get();
    while(cur){
        if(dynamic_cast<PreProcNode*>(cur)){
            if(preprocs){
                preprocs->next.release();
                preprocs->next.reset(cur);
                preprocs = preprocs->next.get();
            }else{
                preprocs_begin = cur;
                preprocs = cur;
            }
        }else{
            if(mods){
                mods->next.release();
                mods->next.reset(cur);
                mods = mods->next.get();
            }else{
                mods_begin = cur;
                mods = cur;
            }
        }
        cur = cur->next.get();
    }

    //set the function's modifiers to the list containing just
    //the normal modifiers
    fdn->modifiers.release();
    fdn->modifiers.reset(mods_begin);
    return preprocs_begin;
}


Node* getLastNode(Node *n){
    Node *cur = n;
    while(cur){
        n = cur;
        cur = cur->next.get();
    }
    return n;
}

void appendModifiers(Node *n, unique_ptr<Node> &mods){
    Node *last = getLastNode(mods.get());
    if(last) last->next.reset(n);
    else mods.reset(n);
}


void declareTypes(Compiler *c){
    for(auto &p : c->mergedCompUnits->userTypes){
        string tyName = p.first;
        auto *dt = c->lookupType(tyName);
        if(!dt) continue;

        vector<Type*> fields;
        TypeNode *fieldNodes = dt->tyn.get();
        if(dt->tyn->type == TT_Tuple or dt->tyn->type == TT_TaggedUnion)
            fieldNodes = fieldNodes->extTy.get();

        while(fieldNodes){
            fields.push_back(c->typeNodeToLlvmType(fieldNodes));
            fieldNodes = (TypeNode*)fieldNodes->next.get();
        }

        StructType::create(*c->ctxt, fields, tyName);
    }
}


/*
 * Copies a function into a new module (named after the function)
 * and copies any functions that are needed by the copied function
 * into the new module as well.
 */
unique_ptr<Compiler> wrapFnInModule(Compiler *c, string &basename, string &mangledName){
    unique_ptr<Compiler> ccpy{new Compiler(c, c->ast.get(), mangledName)};

    copyDecls(c, ccpy.get());
    declareTypes(ccpy.get());

    //create an empty main function to avoid crashes with compFn when
    //trying to return to the caller function
    ccpy->createMainFn();
    //the ret comes separate
    ccpy->builder.CreateRet(ConstantInt::get(*ccpy->ctxt, APInt(32, 1)));

    auto *fn = ccpy->getFuncDecl(basename, mangledName);
    auto *cds = stripCompilerDirectives(fn->fdn);

    if(fn){
        ccpy->compFn(fn);
    }else{
        cerr << "Function '" << mangledName << "' not found.\n";
        c->errFlag = true;
        appendModifiers(cds, fn->fdn->modifiers);
        return 0;
    }

    //re-add the compiler directives
    appendModifiers(cds, fn->fdn->modifiers);
    return ccpy;
}

} // end of namespace ante
