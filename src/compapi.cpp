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

    void Ante_debug(TypedValue &tv){
        tv.dump();
    }

    void Ante_ctError(Compiler *c, TypedValue &msgTv){
        char *msg = (char*)typedValueToGenericValue(c, msgTv).PointerVal;
        auto *curfn = c->compCtxt->callStack.back()->fdn.get();
        yy::location fakeloc = mkLoc(mkPos(0,0,0), mkPos(0,0,0));
        c->compErr(msg, curfn ? curfn->loc : fakeloc);
    }

    TypedValue* FuncDecl_getName(Compiler *c, TypedValue &fd){
        FuncDecl *f = (FuncDecl*)((ConstantInt*)fd.val)->getZExtValue();
        string &n = f->getName();

        yy::location lloc = mkLoc(mkPos(0,0,0), mkPos(0,0,0));
        auto *strlit = new StrLitNode(lloc, n);

        return new TypedValue(strlit->compile(c));
    }

    size_t Ante_sizeof(Compiler *c, TypedValue &tv){
        if(tv.type->typeTag == TT_Type){
            return extractTypeValue(tv)->getSizeInBits(c) / 8;
        }else{
            return tv.type->getSizeInBits(c) / 8;
        }
    }

    void Ante_ctStore(Compiler *c, TypedValue &nameTv, TypedValue &gv){
        char *name = (char*)typedValueToGenericValue(c, nameTv).PointerVal;
        c->ctCtxt->ctStores[name] = gv;
    }

    TypedValue* Ante_ctLookup(Compiler *c, TypedValue &nameTv){
        char *name = (char*)typedValueToGenericValue(c, nameTv).PointerVal;
        try{
            return new TypedValue(c->ctCtxt->ctStores.at(name));
        }catch(out_of_range r){
            cerr << "error: ctLookup: Cannot find var '" << name << "'\n";
            throw new CtError();
        }
    }
}

namespace ante {
    map<string, unique_ptr<CtFunc>> compapi;

    void init_compapi(){
        compapi.emplace("Ante_getAST",      new CtFunc((void*)Ante_getAST,      AnPtrType::get(AnDataType::get("Node"))));
        compapi.emplace("Ante_debug",       new CtFunc((void*)Ante_debug,       AnType::getVoid(), {AnTypeVarType::get("'t'")}));
        compapi.emplace("Ante_sizeof",      new CtFunc((void*)Ante_sizeof,      AnType::getU32(),  {AnTypeVarType::get("'t'")}));
        compapi.emplace("Ante_ctStore",     new CtFunc((void*)Ante_ctStore,     AnType::getVoid(), {AnPtrType::get(AnType::getPrimitive(TT_C8)), AnTypeVarType::get("'t'")}));
        compapi.emplace("Ante_ctLookup",    new CtFunc((void*)Ante_ctLookup,    AnTypeVarType::get("'t'"), {AnPtrType::get(AnType::getPrimitive(TT_C8))}));
        compapi.emplace("Ante_ctError",     new CtFunc((void*)Ante_ctError,     AnType::getVoid(), {AnPtrType::get(AnType::getPrimitive(TT_C8))}));
        compapi.emplace("FuncDecl_getName", new CtFunc((void*)FuncDecl_getName, AnDataType::get("Str"), {AnDataType::get("FuncDecl")}));
    }

    CtFunc::CtFunc(void* f) : fn(f), params(), retty(AnType::getVoid()){}
    CtFunc::CtFunc(void* f, AnType *retTy) : fn(f), params(), retty(retTy){}
    CtFunc::CtFunc(void* f, AnType *retTy, vector<AnType*> p) : fn(f), params(p), retty(retTy){}

    //convert void* to void*() and call it
    void* CtFunc::operator()(){
        void* (*resfn)() = 0;
        *reinterpret_cast<void**>(&resfn) = fn;
        return resfn();
    }

    void* CtFunc::operator()(TypedValue &tv){
        void* (*resfn)(TypedValue&) = 0;
        *reinterpret_cast<void**>(&resfn) = fn;
        return resfn(tv);
    }

    void* CtFunc::operator()(Compiler *c, TypedValue &tv){
        void* (*resfn)(Compiler*, TypedValue&) = 0;
        *reinterpret_cast<void**>(&resfn) = fn;
        return resfn(c, tv);
    }

    void* CtFunc::operator()(TypedValue &tv1, TypedValue &tv2){
        void* (*resfn)(TypedValue&, TypedValue&) = 0;
        *reinterpret_cast<void**>(&resfn) = fn;
        return resfn(tv1, tv2);
    }

    void* CtFunc::operator()(Compiler *c, TypedValue &tv1, TypedValue &tv2){
        void* (*resfn)(Compiler*, TypedValue&, TypedValue&) = 0;
        *reinterpret_cast<void**>(&resfn) = fn;
        return resfn(c, tv1, tv2);
    }
}
