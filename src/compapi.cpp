#include "compiler.h"
#include "types.h"
#include "jitlinker.h"

using namespace ante;
using namespace std;
using namespace llvm;

/* Provide a callable C API from ante */
extern "C" {

    Node* Ante_getAST(){
        return parser::getRootNode();
    }

    void Ante_debug(TypedValue *tv){
        tv->dump();
    }

    void Ante_ctError(Compiler *c, TypedValue *msgTv){
        char *msg = (char*)typedValueToGenericValue(c, msgTv).PointerVal;
        auto *curfn = c->compCtxt->callStack.back()->fdn;
        yy::location fakeloc = mkLoc(mkPos(0,0,0), mkPos(0,0,0));
        c->compErr(msg, curfn ? curfn->loc : fakeloc);
    }

    TypedValue* FuncDecl_getName(Compiler *c, TypedValue *fd){
        FuncDecl *f = (FuncDecl*)((ConstantInt*)fd->val)->getZExtValue();
        string &n = f->fdn->basename;

        yy::location lloc = mkLoc(mkPos(0,0,0), mkPos(0,0,0));
        auto *strlit = new StrLitNode(lloc, n);

        return strlit->compile(c);
    }

    size_t Ante_sizeof(Compiler *c, TypedValue *tv){
        if(tv->type->type == TT_Type){
            return extractTypeValue(tv)->getSizeInBits(c) / 8;
        }else{
            return tv->type->getSizeInBits(c) / 8;
        }
    }

    void Ante_ctStore(Compiler *c, TypedValue *nameTv, TypedValue *gv){
        char *name = (char*)typedValueToGenericValue(c, nameTv).PointerVal;
        c->ctCtxt->ctStores[name] = gv;
    }

    TypedValue* Ante_ctLookup(Compiler *c, TypedValue *nameTv){
        char *name = (char*)typedValueToGenericValue(c, nameTv).PointerVal;
        try{
            return c->ctCtxt->ctStores.at(name);
        }catch(out_of_range r){
            cerr << "error: ctLookup: Cannot find var '" << name << "'\n";
            throw new CtError();
        }
    }
}

namespace ante {
    map<string, CtFunc*> compapi = {
        {"Ante_getAST",      new CtFunc((void*)Ante_getAST,      mkTypeNodeWithExt(TT_Ptr, mkDataTypeNode("Node")))},
        {"Ante_debug",       new CtFunc((void*)Ante_debug,       mkAnonTypeNode(TT_Void), {mkAnonTypeNode(TT_TypeVar)})},
        {"Ante_sizeof",      new CtFunc((void*)Ante_sizeof,      mkAnonTypeNode(TT_U32), {mkAnonTypeNode(TT_TypeVar)})},
        {"Ante_ctStore",     new CtFunc((void*)Ante_ctStore,     mkAnonTypeNode(TT_Void), {mkTypeNodeWithExt(TT_Ptr, mkAnonTypeNode(TT_C8)), mkAnonTypeNode(TT_TypeVar)})},
        {"Ante_ctLookup",    new CtFunc((void*)Ante_ctLookup,    mkAnonTypeNode(TT_TypeVar), {mkTypeNodeWithExt(TT_Ptr, mkAnonTypeNode(TT_C8))})},
        {"Ante_ctError",     new CtFunc((void*)Ante_ctError,     mkAnonTypeNode(TT_Void), {mkTypeNodeWithExt(TT_Ptr, mkAnonTypeNode(TT_C8))})},
        {"FuncDecl_getName", new CtFunc((void*)FuncDecl_getName, mkDataTypeNode("Str"), {mkDataTypeNode("FuncDecl")})}
    };


    CtFunc::CtFunc(void* f) : fn(f), params(), retty(mkAnonTypeNode(TT_Void)){}
    CtFunc::CtFunc(void* f, TypeNode *retTy) : fn(f), params(), retty(retTy){}
    CtFunc::CtFunc(void* f, TypeNode *retTy, vector<TypeNode*> p) : fn(f), params(p), retty(retTy){}

    //convert void* to void*() and call it
    void* CtFunc::operator()(){
        void* (*resfn)() = 0;
        *reinterpret_cast<void**>(&resfn) = fn;
        return resfn();
    }

    void* CtFunc::operator()(TypedValue *tv){
        void* (*resfn)(TypedValue*) = 0;
        *reinterpret_cast<void**>(&resfn) = fn;
        return resfn(tv);
    }

    void* CtFunc::operator()(Compiler *c, TypedValue *tv){
        void* (*resfn)(Compiler*, TypedValue*) = 0;
        *reinterpret_cast<void**>(&resfn) = fn;
        return resfn(c, tv);
    }

    void* CtFunc::operator()(TypedValue *tv1, TypedValue *tv2){
        void* (*resfn)(TypedValue*, TypedValue*) = 0;
        *reinterpret_cast<void**>(&resfn) = fn;
        return resfn(tv1, tv2);
    }

    void* CtFunc::operator()(Compiler *c, TypedValue *tv1, TypedValue *tv2){
        void* (*resfn)(Compiler*, TypedValue*, TypedValue*) = 0;
        *reinterpret_cast<void**>(&resfn) = fn;
        return resfn(c, tv1, tv2);
    }
}
