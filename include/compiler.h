#ifndef COMPILER_H
#define COMPILER_H

#include <climits> //required by llvm is using clang
#include <llvm/IR/LegacyPassManager.h>
#include <llvm/IR/IRBuilder.h>
#include <llvm/IR/Module.h>
#include <memory>
#include <stack>
#include <map>

using namespace llvm;
using namespace std;

/* Forward-declarations of Nodes defined in parser.h */
struct Node;
struct VarNode;
struct TypeNode;
struct BinOpNode;
struct StrLitNode;
struct IntLitNode;
struct FuncDeclNode;
struct FuncCallNode;
struct DataDeclNode;


/*
 *  Used for storage of additional information, such as signedness,
 *  not represented by llvm::Type
 */
struct TypedValue {
    Value *val;
    TypeTag type;

    TypedValue(Value *v, TypeTag ty) : val(v), type(ty){}
    Type* getType() const{ return val->getType(); }
};


struct DataType {
    vector<string> fields;
    Type* type;

    DataType(vector<string> &f, Type *ty) : fields(f), type(ty){}

    int getFieldIndex(string &field){
        for(int i = 0; i < fields.length(); i++)
            if(field == fields[i])
                return i;
        return -1;
    }
};


struct Variable {
    string name;
    TypedValue *tval;
    unsigned int scope;
    bool noFree;

    Value* getVal() const{
        return tval->val;
    }
   
    TypeTag getType() const{
        return tval->type;
    }

    bool isFreeable() const{
        return tval->type == TT_Ptr && !noFree;
    }

    Variable(string n, TypedValue *tv, unsigned int s, bool nofr=true) : name(n), tval(tv), scope(s), noFree(nofr){}
};


namespace ante{
    struct Compiler{
        unique_ptr<legacy::FunctionPassManager> passManager;
        unique_ptr<Module> module;
        unique_ptr<Node> ast;
        IRBuilder<> builder;

        //Stack of maps of variables mapped to their identifier.
        //Maps are seperated according to their scope.
        stack<std::map<string, Variable*>> varTable;

        //Map of declared, but non-defined functions
        map<string, FuncDeclNode*> fnDecls;

        //Map of declared usertypes
        map<string, DataType*> userTypes;

        bool errFlag, compiled;
        string fileName;
        unsigned int scope;
        
        Compiler(char *fileName);
        ~Compiler();

        void compile();
        void compileNative();
        void compilePrelude();
        void emitIR();
        void enterNewScope();
        void exitScope();

        //binop functions
        TypedValue* compAdd(TypedValue *l, TypedValue *r, BinOpNode *op);
        TypedValue* compSub(TypedValue *l, TypedValue *r, BinOpNode *op);
        TypedValue* compMul(TypedValue *l, TypedValue *r, BinOpNode *op);
        TypedValue* compDiv(TypedValue *l, TypedValue *r, BinOpNode *op);
        TypedValue* compRem(TypedValue *l, TypedValue *r, BinOpNode *op);
        TypedValue* compExtract(TypedValue *l, TypedValue *r, BinOpNode *op);
        TypedValue* compInsert(BinOpNode *insertOp, Node *assignExpr);
        
        TypedValue* compErr(string msg, unsigned int row, unsigned int col);

        Function* getFunction(string& name);
        TypedValue* compLetBindingFn(FuncDeclNode *fdn, size_t nParams, vector<Type*> &paramTys, Type *retTy);
        Function* compFn(FuncDeclNode *fn);
        void registerFunction(FuncDeclNode *func);

        unsigned int getScope() const;
        Variable* lookup(string var) const;
        void stoVar(string var, Variable *val);
        DataType* lookupType(string tyname) const;
        void stoType(DataType *ty, string &typeName);

        Type* typeNodeToLlvmType(TypeNode *tyNode);
        
        void handleImplicitConversion(TypedValue **lhs, TypedValue **rhs);
        void implicitlyCastIntToInt(TypedValue **lhs, TypedValue **rhs);
        void implicitlyCastFltToFlt(TypedValue **lhs, TypedValue **rhs);
        void implicitlyCastIntToFlt(TypedValue **tval, Type *ty);


        static size_t getTupleSize(Node *tup);
        
        static int compileIRtoObj(Module *m, string inFile, string outFile);
        static int linkObj(string inFiles, string outFile);
    };
}

//conversions
Type* typeTagToLlvmType(TypeTag tagTy, string typeName);
TypeTag llvmTypeToTypeTag(Type *t);
string llvmTypeToStr(Type *ty);
string typeTagToStr(TypeTag ty);
bool llvmTypeEq(Type *l, Type *r);

char getBitWidthOfTypeTag(const TypeTag tagTy);
bool isUnsignedTypeTag(const TypeTag tagTy);
#endif
