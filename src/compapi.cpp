#include "types.h"
#include "antevalue.h"
#include "compapi.h"

using namespace std;
using namespace llvm;
using namespace ante;
using namespace ante::parser;

/* Provide a callable C API from ante */
extern "C" {

    /** All compiler api functions must return a pointer to some
     * value, so void-returning functions return a nullptr */
    void* Ante_debug(Compiler *c, AnteValue &tv){
        tv.print(c);
        return nullptr;
    }

    void* Ante_error(Compiler *c, AnteValue &msg){
        auto *curfn = c->compCtxt->callStack.back()->getFDN();
        error(msg.castTo<char*>(), curfn ? curfn->loc : unknownLoc());
        return nullptr;
    }

    TypedValue* FuncDecl_getName(Compiler *c, AnteValue &fd){
        FuncDecl *f = fd.castTo<FuncDecl*>();
        string n = f->getName();

        yy::location lloc = mkLoc(mkPos(0,0,0), mkPos(0,0,0));
        auto *strlit = new StrLitNode(lloc, n);

        return new TypedValue(CompilingVisitor::compile(c, strlit));
    }

    TypedValue* Ante_sizeof(Compiler *c, AnteValue &tv){
        auto size = tv.getType()->getSizeInBits(c);

        if(!size){
            cerr << size.getErr() << endl;
            size = 0;
        }

        Value *sizeVal = c->builder.getIntN(AN_USZ_SIZE, size.getVal() / 8);
        return new TypedValue(sizeVal, AnType::getUsz());
    }

    TypedValue* Ante_typeof(Compiler *c, AnteValue &val){
        Value *addr = c->builder.getInt64((unsigned long) val.getType());
        return new TypedValue(addr, BasicModifier::get(AnPtrType::get(AnType::getUnit()), Tok_Ante));
    }

    void* Ante_emit_ir(Compiler *c){
        if(c && c->module){
            c->module->print(llvm::errs(), nullptr);
        }else{
            cerr << "error: Ante.emit_ir: null module" << endl;
        }
        return nullptr;
    }

    void* Ante_forget(Compiler *c, AnteValue &name){
        //TODO: re-add
        //c->mergedCompUnits->fnDecls[name.castTo<char*>()].clear();
        return nullptr;
    }
}

namespace ante {
    //compiler-api
    namespace capi {
        map<string, unique_ptr<CtFunc>> compapi;

        void init(){
            using U = std::unique_ptr<CtFunc>;
            compapi.emplace("debug",       U(new CtFunc((void*)Ante_debug,       AnType::getUnit(), {AnTypeVarType::get("'t'")})));
            compapi.emplace("sizeof",      U(new CtFunc((void*)Ante_sizeof,      AnType::getU32(),  {AnTypeVarType::get("'t'")})));
            compapi.emplace("typeof",      U(new CtFunc((void*)Ante_typeof,      AnPtrType::get(AnType::getUnit()), {AnTypeVarType::get("'t")})));
            compapi.emplace("error",       U(new CtFunc((void*)Ante_error,       AnType::getUnit(), {AnPtrType::get(AnType::getPrimitive(TT_C8))})));
            compapi.emplace("emit_ir",     U(new CtFunc((void*)Ante_emit_ir,     AnType::getUnit())));
            compapi.emplace("forget",      U(new CtFunc((void*)Ante_forget,      AnType::getUnit(), {AnPtrType::get(AnType::getPrimitive(TT_C8))})));
        }

        CtFunc* lookup(string const& fn){
            auto it = compapi.find(fn);
            return it != compapi.end() ?
                it->second.get() : nullptr;
        }

        CtFunc::CtFunc(void* f) : fn(f), params(), retty(AnType::getUnit()){}
        CtFunc::CtFunc(void* f, AnType *retTy) : fn(f), params(), retty(retTy){}
        CtFunc::CtFunc(void* f, AnType *retTy, vector<AnType*> p) : fn(f), params(p), retty(retTy){}

        //convert void* to TypedValue*(*)(Compiler*) and call it
        TypedValue* CtFunc::operator()(Compiler *c){
            TypedValue* (*resfn)(Compiler*) = 0;
            *reinterpret_cast<void**>(&resfn) = fn;
            return resfn(c);
        }

        TypedValue* CtFunc::operator()(Compiler *c, Arg tv){
            TypedValue* (*resfn)(Compiler*, Arg) = 0;
            *reinterpret_cast<void**>(&resfn) = fn;
            return resfn(c, tv);
        }

        TypedValue* CtFunc::operator()(Compiler *c, Arg tv1, Arg tv2){
            TypedValue* (*resfn)(Compiler*, Arg, Arg) = 0;
            *reinterpret_cast<void**>(&resfn) = fn;
            return resfn(c, tv1, tv2);
        }

        TypedValue* CtFunc::operator()(Compiler *c, Arg tv1, Arg tv2, Arg tv3){
            TypedValue* (*resfn)(Compiler*, Arg, Arg, Arg) = 0;
            *reinterpret_cast<void**>(&resfn) = fn;
            return resfn(c, tv1, tv2, tv3);
        }

        TypedValue* CtFunc::operator()(Compiler *c, Arg tv1, Arg tv2, Arg tv3, Arg tv4){
            TypedValue* (*resfn)(Compiler*, Arg, Arg, Arg, Arg) = 0;
            *reinterpret_cast<void**>(&resfn) = fn;
            return resfn(c, tv1, tv2, tv3, tv4);
        }

        TypedValue* CtFunc::operator()(Compiler *c, Arg tv1, Arg tv2, Arg tv3, Arg tv4, Arg tv5){
            TypedValue* (*resfn)(Compiler*, Arg, Arg, Arg, Arg, Arg) = 0;
            *reinterpret_cast<void**>(&resfn) = fn;
            return resfn(c, tv1, tv2, tv3, tv4, tv5);
        }

        TypedValue* CtFunc::operator()(Compiler *c, Arg tv1, Arg tv2, Arg tv3, Arg tv4, Arg tv5, Arg tv6){
            TypedValue* (*resfn)(Compiler*, Arg, Arg, Arg, Arg, Arg, Arg) = 0;
            *reinterpret_cast<void**>(&resfn) = fn;
            return resfn(c, tv1, tv2, tv3, tv4, tv5, tv6);
        }
    }
}
