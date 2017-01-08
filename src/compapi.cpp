#include "compiler.h"

/* Provide a callable C API from ante */
extern "C" {

    Node* Ante_getAST(){
        return ante::parser::getRootNode();
    }

    void Ante_debug(TypedValue *tv){
        tv->dump();
    }

}

map<string, CtFunc*> compapi = {
    {"Ante_getAST", new CtFunc((void*)Ante_getAST, mkTypeNodeWithExt(TT_Ptr, mkDataTypeNode("Node")))},
    {"Ante_debug",  new CtFunc((void*)Ante_debug,  mkAnonTypeNode(TT_Void))}
};
    

CtFunc::CtFunc(void* f) : fn(f), params(), retty(mkAnonTypeNode(TT_Void)){}
CtFunc::CtFunc(void* f, TypeNode *retTy) : fn(f), params(), retty(retTy){}
CtFunc::CtFunc(void* f, TypeNode *retTy, vector<unique_ptr<TypeNode>> p) : fn(f), params(move(p)), retty(retTy){}

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
