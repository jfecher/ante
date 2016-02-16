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
struct BinOpNode;
struct FuncDeclNode;
struct FuncCallNode;
struct StrLitNode;
struct IntLitNode;


/*
 *  Used for storage of additional information, such as signedness,
 *  not represented by llvm::Type
 */
struct TypedValue {
    Value *val;
    int type;

    TypedValue(Value *v, int ty) : val(v), type(ty){}
};

namespace ante{
    struct Compiler{
        unique_ptr<legacy::FunctionPassManager> passManager;
        unique_ptr<Module> module;
        unique_ptr<Node> ast;
        IRBuilder<> builder;

        //Stack of maps of variables mapped to their identifier.
        //Maps are seperated according to their scope.
        stack<std::map<string, TypedValue*>> varTable;

        //Map of declared, but non-defined functions
        map<string, FuncDeclNode*> fnDecls;

        bool errFlag, compiled;
        string fileName;
        
        Compiler(char *fileName);
        ~Compiler();

        void compile();
        void compileNative();
        void compilePrelude();
        void emitIR();
        void enterNewScope();
        void exitScope();
        
        TypedValue* compAdd(TypedValue *l, TypedValue *r, BinOpNode *op);
        TypedValue* compSub(TypedValue *l, TypedValue *r, BinOpNode *op);
        TypedValue* compMul(TypedValue *l, TypedValue *r, BinOpNode *op);
        TypedValue* compDiv(TypedValue *l, TypedValue *r, BinOpNode *op);
        TypedValue* compRem(TypedValue *l, TypedValue *r, BinOpNode *op);
        
        TypedValue* compErr(string msg, unsigned int row, unsigned int col);

        Function* compFn(FuncDeclNode *fn);
        void registerFunction(FuncDeclNode *func);

        TypedValue* lookup(string var);
        void stoVar(string var, TypedValue *val);

        bool isSigned(Node *n);
        void checkIntSize(TypedValue **lhs, TypedValue **rhs);
        
        static Type* tokTypeToLlvmType(int tokTy, string typeName);
        static int llvmTypeToTokType(Type *t);

        static char getBitWidthOfTokTy(int tokTy);
        static bool isUnsignedTokTy(int tokTy);
        
        static int compileIRtoObj(Module *m, string inFile, string outFile);
        static int linkObj(string inFiles, string outFile);
    };
}


#endif
