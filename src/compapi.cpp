#include "compiler.h"
#include "types.h"
#include "jitlinker.h"
#include "argtuple.h"

using namespace std;
using namespace llvm;
using namespace ante;
using namespace ante::parser;

/* Provide a callable C API from ante */
extern "C" {

    TypedValue* Ante_getAST(Compiler *c){
        auto *root = parser::getRootNode();
        Value *addr = c->builder.getIntN(AN_USZ_SIZE, (size_t)root);
        
        auto *anType = AnPtrType::get(AnDataType::get("Ante.Node"));
        auto *llvmType = c->anTypeToLlvmType(anType);

        Value *ptr = c->builder.CreateIntToPtr(addr, llvmType);
        return new TypedValue(ptr, anType);
    }

    /** All api functions must return a pointer to some value,
     * so void-returning functions return a void* nullptr by convention */
    void* Ante_debug(Compiler *c, TypedValue &tv){
        tv.dump();
        return nullptr;
    }

    void* Ante_error(Compiler *c, TypedValue &msgTv){
        char *msg = *(char**)ArgTuple(c, msgTv).asRawData();
        auto *curfn = c->compCtxt->callStack.back()->fdn.get();
        yy::location fakeloc = mkLoc(mkPos(0,0,0), mkPos(0,0,0));
        c->compErr(msg, curfn ? curfn->loc : fakeloc);
        return nullptr;
    }

    TypedValue* FuncDecl_getName(Compiler *c, TypedValue &fd){
        FuncDecl *f = (FuncDecl*)((ConstantInt*)fd.val)->getZExtValue();
        string &n = f->getName();

        yy::location lloc = mkLoc(mkPos(0,0,0), mkPos(0,0,0));
        auto *strlit = new StrLitNode(lloc, n);

        return new TypedValue(strlit->compile(c));
    }

    TypedValue* Ante_sizeof(Compiler *c, TypedValue &tv){
        auto size = tv.type->typeTag == TT_Type
                    ? extractTypeValue(tv)->getSizeInBits(c)
                    : tv.type->getSizeInBits(c);

        if(!size){
            cerr << size.getErr() << endl;
            size = 0;
        }

        Value *sizeVal = c->builder.getIntN(AN_USZ_SIZE, size.getVal() / 8);
        return new TypedValue(sizeVal, AnType::getUsz());
    }

    void* Ante_store(Compiler *c, TypedValue &nameTv, TypedValue &gv){
        char *name = *(char**)ArgTuple(c, nameTv).asRawData();
        c->ctCtxt->ctStores[name] = gv;
        return nullptr;
    }

    TypedValue* Ante_lookup(Compiler *c, TypedValue &nameTv){
        char *name = *(char**)ArgTuple(c, nameTv).asRawData();

        auto t = c->ctCtxt->ctStores.lookup(name);
        if(!!t){
            return new TypedValue(t);
        }else{
            cerr << "error: ctLookup: Cannot find var '" << name << "'" << endl;
            throw new CtError();
        }
    }

    void* Ante_emitIR(Compiler *c){
        if(c and c->module){
            c->module->print(llvm::errs(), nullptr);
        }else{
            cerr << "error: Ante.emitIR: null module" << endl;
        }
        return nullptr;
    }

    void* Ante_forget(Compiler *c, TypedValue &msgTv){
        char *msg = *(char**)ArgTuple(c, msgTv).asRawData();
        c->mergedCompUnits->fnDecls[msg].clear();
        return nullptr;
    }
}

namespace ante {
    map<string, unique_ptr<CtFunc>> compapi;

    void init_compapi(){
        compapi.emplace("Ante_getAST",      new CtFunc((void*)Ante_getAST,      AnPtrType::get(AnDataType::get("Ante.Node"))));
        compapi.emplace("Ante_debug",       new CtFunc((void*)Ante_debug,       AnType::getVoid(), {AnTypeVarType::get("'t'")}));
        compapi.emplace("Ante_sizeof",      new CtFunc((void*)Ante_sizeof,      AnType::getU32(),  {AnTypeVarType::get("'t'")}));
        compapi.emplace("Ante_store",       new CtFunc((void*)Ante_store,       AnType::getVoid(), {AnPtrType::get(AnType::getPrimitive(TT_C8)), AnTypeVarType::get("'t'")}));
        compapi.emplace("Ante_lookup",      new CtFunc((void*)Ante_lookup,      AnTypeVarType::get("'t'"), {AnPtrType::get(AnType::getPrimitive(TT_C8))}));
        compapi.emplace("Ante_error",       new CtFunc((void*)Ante_error,       AnType::getVoid(), {AnPtrType::get(AnType::getPrimitive(TT_C8))}));
        compapi.emplace("Ante_emitIR",      new CtFunc((void*)Ante_emitIR,      AnType::getVoid()));
        compapi.emplace("Ante_forget",      new CtFunc((void*)Ante_forget,      AnType::getVoid(), {AnPtrType::get(AnType::getPrimitive(TT_C8))}));
        compapi.emplace("FuncDecl_getName", new CtFunc((void*)FuncDecl_getName, AnDataType::get("Str"), {AnDataType::get("Ante.FuncDecl")}));
    }

    CtFunc::CtFunc(void* f) : fn(f), params(), retty(AnType::getVoid()){}
    CtFunc::CtFunc(void* f, AnType *retTy) : fn(f), params(), retty(retTy){}
    CtFunc::CtFunc(void* f, AnType *retTy, vector<AnType*> p) : fn(f), params(p), retty(retTy){}

    //convert void* to TypedValue*(*)(Compiler*) and call it
    TypedValue* CtFunc::operator()(Compiler *c){
        TypedValue* (*resfn)(Compiler*) = 0;
        *reinterpret_cast<void**>(&resfn) = fn;
        return resfn(c);
    }

    TypedValue* CtFunc::operator()(Compiler *c, TypedValue const& tv){
        TypedValue* (*resfn)(Compiler*, TypedValue const&) = 0;
        *reinterpret_cast<void**>(&resfn) = fn;
        return resfn(c, tv);
    }

    TypedValue* CtFunc::operator()(Compiler *c, TypedValue const& tv1, TypedValue const& tv2){
        TypedValue* (*resfn)(Compiler*, TypedValue const&, TypedValue const&) = 0;
        *reinterpret_cast<void**>(&resfn) = fn;
        return resfn(c, tv1, tv2);
    }
}
