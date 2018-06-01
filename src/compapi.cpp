#include "types.h"
#include "argtuple.h"
#include "compapi.h"

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

    /** All compiler api functions must return a pointer to some
     * value, so void-returning functions return a nullptr */
    void* Ante_debug(Compiler *c, ArgTuple &tv){
        tv.print(c);
        return nullptr;
    }

    void* Ante_error(Compiler *c, ArgTuple &msg){
        auto *cstrTy = AnPtrType::get(AnType::getPrimitive(TT_C8));
        if(!c->typeEq(msg.getType(), cstrTy)){
            throw new CompilationError("First argument of Ante.store must be of type " +
                    anTypeToColoredStr(cstrTy) + " but a value of type "
                    + anTypeToColoredStr(msg.getType()) + " was given instead.");
        }

        auto *curfn = c->compCtxt->callStack.back()->fdn.get();
        yy::location fakeloc = mkLoc(mkPos(0,0,0), mkPos(0,0,0));
        c->compErr(msg.castTo<char*>(), curfn ? curfn->loc : fakeloc);
        return nullptr;
    }

    TypedValue* FuncDecl_getName(Compiler *c, ArgTuple &fd){
        FuncDecl *f = fd.castTo<FuncDecl*>();
        string &n = f->getName();

        yy::location lloc = mkLoc(mkPos(0,0,0), mkPos(0,0,0));
        auto *strlit = new StrLitNode(lloc, n);

        return new TypedValue(CompilingVisitor::compile(c, strlit));
    }

    TypedValue* Ante_sizeof(Compiler *c, ArgTuple &tv){
        auto size = tv.getType()->typeTag == TT_Type
                    ? tv.castTo<AnType*>()->getSizeInBits(c)
                    : tv.getType()->getSizeInBits(c);

        if(!size){
            cerr << size.getErr() << endl;
            size = 0;
        }

        Value *sizeVal = c->builder.getIntN(AN_USZ_SIZE, size.getVal() / 8);
        return new TypedValue(sizeVal, AnType::getUsz());
    }

    void* Ante_store(Compiler *c, ArgTuple &name, ArgTuple &gv){
        auto *cstrTy = AnPtrType::get(AnType::getPrimitive(TT_C8));
        if(!c->typeEq(name.getType(), cstrTy)){
            throw new CompilationError("First argument of Ante.store must be of type " +
                    anTypeToColoredStr(cstrTy) + " but a value of type "
                    + anTypeToColoredStr(name.getType()) + " was given instead.");
        }
        c->ctCtxt->ctStores[name.castTo<char*>()] = gv;
        return nullptr;
    }

    TypedValue* Ante_lookup(Compiler *c, ArgTuple &name){
        auto *cstrTy = AnPtrType::get(AnType::getPrimitive(TT_C8));
        if(!c->typeEq(name.getType(), cstrTy)){
            throw new CompilationError("Argument of Ante.lookup must be of type " +
                    anTypeToColoredStr(cstrTy) + " but a value of type "
                    + anTypeToColoredStr(name.getType()) + " was given instead.");
        }

        auto t = c->ctCtxt->ctStores.find(name.castTo<char*>());
        if(t != c->ctCtxt->ctStores.end()){
            return new TypedValue(t->second.asTypedValue(c));
        }else{
            std::cerr << "error: ctLookup: Cannot find var '" << name.castTo<char*>() << "'" << endl;
            throw new CtError();
        }
    }

    TypedValue* Ante_eval(Compiler *c, ArgTuple &evalArg){
        auto *cstrTy = AnPtrType::get(AnType::getPrimitive(TT_C8));
        if(!c->typeEq(evalArg.getType(), cstrTy)){
            throw new CompilationError("Argument of Ante.eval must be of type " +
                    anTypeToColoredStr(cstrTy) + " but a value of type "
                    + anTypeToColoredStr(evalArg.getType()) + " was given instead.");
        }

        string eval_str = evalArg.castTo<char*>();
        string file_name = "eval";

        auto *lex = new Lexer(&file_name, eval_str, 1u, 1u);
        setLexer(lex);
        yy::parser p{};
        int flag = p.parse();
        if(flag != PE_OK){ //parsing error, cannot procede
            fputs("Syntax error in call to Ante.eval, aborting.\n", stderr);
            return new TypedValue(c->getVoidLiteral());
        }

        RootNode *expr = parser::getRootNode();
        TypedValue val;

        scanImports(c, expr);
        c->scanAllDecls(expr);

        //Compile main and hold onto the last value
        for(auto &n : expr->main){
            try{
                val = CompilingVisitor::compile(c, n);
            }catch(CtError *e){
                delete e;
            }
        }
        return new TypedValue(val);
    }

    void* Ante_emit_ir(Compiler *c){
        if(c and c->module){
            c->module->print(llvm::errs(), nullptr);
        }else{
            cerr << "error: Ante.emit_ir: null module" << endl;
        }
        return nullptr;
    }

    void* Ante_forget(Compiler *c, ArgTuple &name){
        auto *cstrTy = AnPtrType::get(AnType::getPrimitive(TT_C8));
        if(!c->typeEq(name.getType(), cstrTy)){
            throw new CompilationError("Argument of Ante.forget must be of type " +
                    anTypeToColoredStr(cstrTy) + " but a value of type "
                    + anTypeToColoredStr(name.getType()) + " was given instead.");
        }
        c->mergedCompUnits->fnDecls[name.castTo<char*>()].clear();
        return nullptr;
    }
}

namespace ante {
    //compiler-api
    namespace capi {
        map<string, unique_ptr<CtFunc>> compapi;

        void init(){
            compapi.emplace("Ante_getAST",      new CtFunc((void*)Ante_getAST,      AnPtrType::get(AnDataType::get("Ante.Node"))));
            compapi.emplace("Ante_debug",       new CtFunc((void*)Ante_debug,       AnType::getVoid(), {AnTypeVarType::get("'t'")}));
            compapi.emplace("Ante_sizeof",      new CtFunc((void*)Ante_sizeof,      AnType::getU32(),  {AnTypeVarType::get("'t'")}));
            compapi.emplace("Ante_store",       new CtFunc((void*)Ante_store,       AnType::getVoid(), {AnPtrType::get(AnType::getPrimitive(TT_C8)), AnTypeVarType::get("'t'")}));
            compapi.emplace("Ante_lookup",      new CtFunc((void*)Ante_lookup,      AnTypeVarType::get("'Dyn"), {AnPtrType::get(AnType::getPrimitive(TT_C8))}));
            compapi.emplace("Ante_eval",        new CtFunc((void*)Ante_eval,        AnTypeVarType::get("'Dyn"), {AnPtrType::get(AnType::getPrimitive(TT_C8))}));
            compapi.emplace("Ante_error",       new CtFunc((void*)Ante_error,       AnType::getVoid(), {AnPtrType::get(AnType::getPrimitive(TT_C8))}));
            compapi.emplace("Ante_emit_ir",     new CtFunc((void*)Ante_emit_ir,     AnType::getVoid()));
            compapi.emplace("Ante_forget",      new CtFunc((void*)Ante_forget,      AnType::getVoid(), {AnPtrType::get(AnType::getPrimitive(TT_C8))}));
            compapi.emplace("FuncDecl_getName", new CtFunc((void*)FuncDecl_getName, AnDataType::get("Str"), {AnDataType::get("Ante.FuncDecl")}));
        }

        CtFunc* lookup(string const& fn){
            auto it = compapi.find(fn);
            return it != compapi.end() ?
                it->second.get() : nullptr;
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
