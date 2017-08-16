#ifndef COMPILER_H
#define COMPILER_H

#include <climits> //required by llvm when using clang
#include <llvm/IR/LegacyPassManager.h>
#include <llvm/IR/IRBuilder.h>
#include <llvm/IR/Module.h>
#include <llvm/IR/LLVMContext.h>
#include <llvm/ExecutionEngine/MCJIT.h>
#include <llvm/ExecutionEngine/ExecutionEngine.h>
#include <memory>
#include <unordered_map>
#include <list>
#include "parser.h"
#include "args.h"
#include "lazystr.h"

using namespace llvm;
using namespace std;

#define AN_MANGLED_SELF "_$self$"

namespace ante {

    extern TypeNode* copy(const TypeNode*);
    extern TypeNode* copy(const unique_ptr<TypeNode>&);

    /**
    * @brief A Value* and TypeNode* pair
    *
    * This is the main type used to represent a value in Ante
    */
    struct TypedValue {
        Value *val;
        unique_ptr<TypeNode> type;

        TypedValue(Value *v, TypeNode *ty) : val(v), type(ty){}

        /**
        * @brief Constructs a TypedValue
        *
        * @param v The Value to use
        * @param ty The TypeNode here is copied, not moved
        */
        TypedValue(Value *v, unique_ptr<TypeNode> &ty) : val(v), type(copy(ty)){}

        Type* getType() const{ return val->getType(); }
        /**
        * @brief Returns true if the type of this TypedValue contains the given modifier
        *
        * @param m The TokTy value of the modifier to search for
        */
        bool hasModifier(int m) const{ return type->hasModifier(m); }

        /**
        * @brief Prints type and value to stdout
        */
        void dump() const;
    };


    /**
    * @brief The result of a type check
    *
    * Can be one of three states: Failure, Success,
    * or SuccessWithTypeVars.
    *
    * SuccessWithTypeVars indicates the typecheck is only a
    * success if a typevar within is bound to a particular type.
    * For example the check of 't* and i32* would return this status.
    * Whenever SuccessWithTypeVars is set, the bindings field contains
    * the specific bindings that should be bound to the typevar term.
    */
    struct TypeCheckResult {
        enum Result { Failure, Success, SuccessWithTypeVars };

        //box internals for faster passing by value
        struct Internals {
            Result res;
            unsigned int matches;
            vector<pair<string,unique_ptr<TypeNode>>> bindings;

            Internals() : res(Success), matches(0), bindings(){}
        };

        Internals *box;

        TypeCheckResult successIf(bool b);
        TypeCheckResult successIf(Result r);
        TypeCheckResult success();
        TypeCheckResult successWithTypeVars();
        TypeCheckResult failure();

        bool failed();

        bool operator!(){return box->res == Failure;}
        Internals* operator->(){return box;}

        /**
        * @brief Searches for the suggested binding of a typevar
        *
        * @param s Name of the typevar to search for a binding for
        *
        * @return The binding if found, nullptr otherwise
        */
        TypeNode* getBindingFor(const string &s);
        TypeCheckResult() : box(new Internals()){}
        TypeCheckResult(const TypeCheckResult &r)  : box(r.box){}
        TypeCheckResult(TypeCheckResult &&r)  : box(r.box){}
    };


    /**
    * @brief Base for typeeq
    *
    * Unlike typeEq, typeEqBase does not require a Compiler instance, but will not
    * properly handle typevars and certain data types without one.
    *
    * Should only be used in rare situations where you do not have a Compiler instance.
    *
    * @param l Type to check
    * @param r Type to check against
    * @param tcr This parameter is passed recursively, pass a TypeCheckResult::Success
    * if at the beginning of the chain
    * @param c Optional parameter to lookup data type definitions and typevars
    *
    * @return The resulting TypeCheckResult
    */
    TypeCheckResult typeEqBase(const TypeNode *l, const TypeNode *r, TypeCheckResult tcr, const Compiler *c = 0);



    //declare ante::Module for FuncDecl
    struct Module;

    /**
    * @brief Contains information about a function that is not contained
    * within its FuncDeclNode.
    *
    * Holds the scope the function was compiled in, the value of the function
    * so it is not recompiled, the object type if it is a method along with any
    * generic parameters of the object, the module compiled in, and each return
    * instance for type checking.
    */
    struct FuncDecl {
        FuncDeclNode* fdn;
        unsigned int scope;
        TypedValue *tv;

        TypeNode *obj;

        /** @brief Any generic parameters the obj may have */
        vector<pair<string, TypeNode*>> obj_bindings;

        shared_ptr<Module> module;
        vector<pair<TypedValue*,LOC_TY>> returns;

        FuncDecl(FuncDeclNode *fn, unsigned int s, shared_ptr<Module> mod, TypedValue *f=0) : fdn(fn), scope(s), tv(f), module(mod), returns(){}
        ~FuncDecl(){ if(fdn) delete fdn; if(tv) delete tv; }
    };

    TypeNode* mkAnonTypeNode(TypeTag);

    /**
     * @brief A TypedValue of type TT_FunctionList is returned whenever there are
     * multiple equally-qualified functions found during a lookup.  A lookup without
     * the properly mangled argument types will also return this type so that the
     * desired function may be deduced later with the actual function call arguments.
     */
    struct FunctionCandidates : public TypedValue {
        vector<shared_ptr<FuncDecl>> candidates;
        TypedValue *obj;

        FunctionCandidates(LLVMContext *c, vector<shared_ptr<FuncDecl>> &ca, TypedValue *o) :
            TypedValue(UndefValue::get(Type::getInt8Ty(*c)),
            mkAnonTypeNode(TT_FunctionList)), candidates(ca), obj(o){}
    };

    /**
    * @brief An individual tag of a tagged union along with the types it corresponds to
    */
    struct UnionTag {
        string name;
        unique_ptr<TypeNode> tyn;
        unsigned short tag;

        UnionTag(string &n, TypeNode *ty, unsigned short t) : name(n), tyn(ty), tag(t){}
    };

    /**
    * @brief Holds the name of a trait and the functions needed to implement it
    */
    struct Trait {
        string name;
        vector<shared_ptr<FuncDecl>> funcs;
    };

    /**
    * @brief Contains information about a data type
    */
    struct DataType {
        string name;
        /** @brief Names of each field. */
        vector<string> fields;
        vector<shared_ptr<UnionTag>> tags;
        vector<shared_ptr<Trait>> traitImpls;

        /** @brief If this is a tagged union, tyn will contain each union member in its extTy */
        unique_ptr<TypeNode> tyn;
        vector<shared_ptr<TypeNode>> generics;

        /** @brief Types are lazily translated into their llvm::Type counterpart to better support
        * generics and prevent the need of forward-decls */
        map<string,Type*> llvmTypes;

        DataType(string n, const vector<string> &f, TypeNode *ty) : name(n), fields(f), tyn(ty){}
        ~DataType(){}

        /**
        * @param field Name of the field to search for
        *
        * @return The index of the field on success, -1 on failure
        */
        int getFieldIndex(string &field) const {
            for(unsigned int i = 0; i < fields.size(); i++)
                if(field == fields[i])
                    return i;
            return -1;
        }

        /**
        * @return True if this DataType is actually a tag type
        */
        bool isUnionTag() const {
            return fields.size() > 0 and fields[0][0] >= 'A' and fields[0][0] <= 'Z';
        }

        /**
        * @brief Gets the name of the parent union type
        *
        * Will fail if this DataType is not a union tag and contains no fields.
        * Use isUnionTag before calling this function if unsure.
        *
        * @return The name of the DataType containing this UnionTag
        */
        string getParentUnionName() const {
            return fields[0];
        }

        /**
        * @brief Returns the UnionTag value of a tag within the union type.
        *
        * This function assumes the tag is within the type. The 0 returned
        * on failure is indistinguishable from a tag of value 0 and will be
        * changed to an exception at a later date.
        *
        * @param name Name of the tag to search for
        *
        * @return the value of the tag found, or 0 on failure
        */
        unsigned short getTagVal(string &name){
            for(auto& tag : tags){
                if(tag->name == name){
                    return tag->tag;
                }
            }
            //TODO: throw exception
            return 0;
        }
    };

    struct Variable {
        string name;

        /**
        * @brief The value assigned to the variable
        */
        unique_ptr<TypedValue> tval;
        unsigned int scope;

        /** @brief Flag for managed pointers.  Currently unused */
        bool noFree;

        /**
        * @brief Set to true if this variable is an implicit pointer.
        * Used by mutable variables.
        */
        bool autoDeref;

        Value* getVal() const{
            return tval->val;
        }

        TypeTag getType() const{
            return tval->type->type;
        }

        /**
        * @return True if this is a managed pointer
        */
        bool isFreeable() const{
            return tval->type? tval->type->type == TT_Ptr && !noFree : false;
        }

        /**
        * @brief Variable constructor
        *
        * @param n Name of variable
        * @param tv Value of variable
        * @param s Scope of variable
        * @param nofr True if the variable should not be free'd
        * @param autoDr True if the variable should be autotomatically dereferenced
        */
        Variable(string n, TypedValue *tv, unsigned int s, bool nofr=true, bool autoDr=false) : name(n), tval(tv), scope(s), noFree(nofr), autoDeref(autoDr){}
    };

    /**
    * @brief Holds a c++ function
    *
    * Used to represent compiler API functions and call them
    * with compile-time constants as arguments
    */
    struct CtFunc {
        void *fn;
        vector<TypeNode*> params;
        unique_ptr<TypeNode> retty;

        size_t numParams() const { return params.size(); }
        bool typeCheck(vector<TypeNode*> &args);
        bool typeCheck(vector<TypedValue*> &args);
        CtFunc(void* fn);
        CtFunc(void* fn, TypeNode *retTy);
        CtFunc(void* fn, TypeNode *retTy, vector<TypeNode*> params);

        ~CtFunc(){ for(auto *tv : params) delete tv; }

        void* operator()();
        void* operator()(TypedValue *tv);
        void* operator()(Compiler *c, TypedValue *tv);
        void* operator()(TypedValue *p1, TypedValue *p2);
        void* operator()(Compiler *c, TypedValue *tv1, TypedValue *tv2);
    };


    struct Compiler;

    /**
     * @brief An Ante Module
     */
    struct Module {
        string name;

        /**
         * @brief Each declared function in the module
         */
        unordered_map<string, vector<shared_ptr<FuncDecl>>> fnDecls;

        /**
         * @brief Each declared DataType in the module
         */
        unordered_map<string, shared_ptr<DataType>> userTypes;

        /**
         * @brief Map of all declared traits; not including their implementations for a given type
         * Each DataType is reponsible for holding its own trait implementations
         */
        unordered_map<string, shared_ptr<Trait>> traits;

        /**
        * @brief Merges two modules
        *
        * @param m module to merge into this
        */
        void import(shared_ptr<Module> m);
    };

    /**
     * @brief Contains state information on the module being compiled
     */
    struct CompilerCtxt {
        //Stack of each called function
        vector<FuncDecl*> callStack;

        //Method object type
        TypeNode *obj;

        map<string, TypeNode*> obj_bindings;

        //the continue and break labels of each for/while loop to jump out of
        //the pointer is swapped/nullified when a function is called to prevent
        //cross-function jumps
        unique_ptr<vector<BasicBlock*>> continueLabels;
        unique_ptr<vector<BasicBlock*>> breakLabels;

        CompilerCtxt() : callStack(), obj(0), continueLabels(new vector<BasicBlock*>()), breakLabels(new vector<BasicBlock*>()){}
    };

    /**
     * @brief Contains compile-time information for user hooks and ctStores.
     */
    struct CompilerCtCtxt {
        /** @brief Compile-time values stored using Ante.ctStore  */
        map<string, TypedValue*> ctStores;

        /** @brief functions to run whenever a function is declared. */
        vector<shared_ptr<FuncDecl>> on_fn_declared_hook;
    };

    /**
     * @brief An Ante compiler responsible for a single module
     */
    struct Compiler {
        shared_ptr<LLVMContext> ctxt;
        unique_ptr<ExecutionEngine> jit;
        unique_ptr<legacy::FunctionPassManager> passManager;
        unique_ptr<llvm::Module> module;
        unique_ptr<RootNode> ast;
        IRBuilder<> builder;

        /** @brief functions and type definitions of current module */
        shared_ptr<Module> compUnit;

        /** @brief all functions and type definitions visible to current module */
        shared_ptr<Module> mergedCompUnits;

        /** @brief all imported modules */
        vector<shared_ptr<Module>> imports;

        /**
         * @brief every single compiled module, even ones invisible to the current
         * compilation unit.  Prevents recompilation of modules
         */
        shared_ptr<unordered_map<string, shared_ptr<Module>>> allCompiledModules;

        /**
         * @brief Stack of variables mapped to their identifier.
         * Maps are seperated according to their scope.
         */
        vector<unique_ptr<unordered_map<string, Variable*>>> varTable;

        unique_ptr<CompilerCtxt> compCtxt;

        shared_ptr<CompilerCtCtxt> ctCtxt;

        bool errFlag, compiled, isLib;
        string fileName, outFile, funcPrefix;
        unsigned int scope, optLvl, fnScope;

        /**
        * @brief The main constructor for Compiler
        *
        * @param fileName Name of the file being compiled
        * @param lib Set to true if this module should be compiled as a library
        * @param ctxt The LLVMContext possibly shared with another Compiler
        */
        Compiler(const char *fileName, bool lib=false, shared_ptr<LLVMContext> ctxt = nullptr);

        /**
        * @brief Constructor for a Compiler compiling a sub-module within the current file.  Currently only
        * used for string interpolation.
        *
        * @param root The node to set as the root node (does not need to be a RootNode already)
        * @param modName Name of the module being compiled
        * @param fName Name of the file being compiled
        * @param lib Set to true if this module should be compiled as a library
        * @param ctxt The LLVMContext shared from the parent Compiler
        */
        Compiler(Compiler *c, Node *root, string modName, bool lib=false);
        ~Compiler();

        /** @brief Fully compiles a module into llvm bytecode */
        void compile();

        /** @brief Compiles a native binary */
        void compileNative();

        /**
        * @brief Compiles a module to an object file
        *
        * @param outName name of the file to output
        *
        * @return 0 on success
        */
        int  compileObj(string &outName);

        /**
        * @brief Imports the prelude module unless the current module is the prelude
        */
        void compilePrelude();

        /**
        * @brief Creates the main function of a main module or creates the library_init
        * function of a lib module.
        *
        * @return The llvm::Function* of the created function.
        */
        Function* createMainFn();

        /** @brief Starts the REPL */
        void eval();

        /** @brief Dumps current contents of module to stdout */
        void emitIR();

        /** @brief Creates and enters a new scope */
        void enterNewScope();

        /** @brief Exits a scope and performs any necessary cleanup */
        void exitScope();

        /**
        * @brief Sweeps through parse tree registering all functions, type
        * declarations, and traits.
        */
        void scanAllDecls(RootNode *n = 0);

        /**
        * @brief Sets appropriate flags and executes operations specified by
        *        the command line arguments
        *
        * @param args The command line arguments
        */
        void processArgs(CompilerArgs *args);

        //binop functions
        /**
         * @brief Emits an add instruction
         *
         * Operator overloads are not taken into account and should be handled beforehand.
         * l and r must be the same type.
         *
         * @param op The + Node used for error reporting
         *
         * @return The resulting add instruction
         */
        TypedValue* compAdd(TypedValue *l, TypedValue *r, BinOpNode *op);
        TypedValue* compSub(TypedValue *l, TypedValue *r, BinOpNode *op);
        TypedValue* compMul(TypedValue *l, TypedValue *r, BinOpNode *op);
        TypedValue* compDiv(TypedValue *l, TypedValue *r, BinOpNode *op);
        TypedValue* compRem(TypedValue *l, TypedValue *r, BinOpNode *op);

        /**
         * @brief Compiles an extract operation such as array#index
         *
         * Operator overloads are not taken into account and should be handled beforehand.
         *
         * @param l The container to extract from
         * @param r The index to extract
         * @param op The # operator used for error reporting
         *
         * @return The result of the extraction
         */
        TypedValue* compExtract(TypedValue *l, TypedValue *r, BinOpNode *op);

        /**
         * @brief Compiles an insert operation such as array#index = 2
         *
         * @param insertOp The # operator containing the lhs of the assignment
         * @param assignExpr The rhs of the assignment
         *
         * @return A void literal
         */
        TypedValue* compInsert(BinOpNode *insertOp, Node *assignExpr);

        /**
         * @brief Compiles a named member access such as str.len
         *
         * @param ln The value or type/module being accessed
         * @param field Name of the desired field/method
         * @param binop Location of the . operator for error reporting
         *
         * @return The extracted field or method
         */
        TypedValue* compMemberAccess(Node *ln, VarNode *field, BinOpNode *binop);
        TypedValue* compLogicalOr(Node *l, Node *r, BinOpNode *op);
        TypedValue* compLogicalAnd(Node *l, Node *r, BinOpNode *op);

        /**
         * @brief Reports a message and highlights the relevant source lines.
         *
         * @param t Type of message to report, either Error, Warning, or Note
         */
        TypedValue* compErr(lazy_printer msg, const yy::location& loc, ErrorType t = ErrorType::Error);

        /**
        * @brief JIT compiles a function with no arguments and calls it afterward
        *
        * @param f the function to JIT
        */
        void jitFunction(Function *fnName);

        /**
        * @brief Imports a given ante file to the current module
        * inputted file must exist and be a valid ante source file.
        *
        * @param fName Name of file to import
        * @param The node containing where the file was imported from.
        *        Usually the ImportNode importing the file.  Used for
        *        error reporting.
        */
        void importFile(const char *name, Node* locNode = 0);

        /** @brief Sets the tv of the FuncDecl specified to the value of f */
        void updateFn(TypedValue *f, FuncDecl *fd, string &name, string &mangledName);
        FuncDecl* getCurrentFunction() const;

        /** @brief Returns the exact function specified if found or nullptr if not */
        TypedValue* getFunction(string& name, string& mangledName);

        /** @brief Returns a vector of all functions with the specified baseName */
        vector<shared_ptr<FuncDecl>>& getFunctionList(string& name) const;

        /** @brief Returns the exact FuncDecl specified if found or nullptr if not */
        FuncDecl* getFuncDecl(string bn, string mangledName);

        /** @brief Emits and returns a function call */
        TypedValue* callFn(string fnBaseName, vector<TypedValue*> args);

        /**
         * @brief Retrieves the function specified
         *
         * Automatically binds generic functions and
         * Performs argument deduction if necessary
         *
         * @param name Basename of function to search for
         * @param args Types of each argument in case multiple functions are found
         *
         * @return The specified function or nullptr
         */
        TypedValue* getMangledFn(string name, vector<TypeNode*> args);

        /**
         * @brief Returns the init method of a type
         *
         * @param from_ty Tuple of argument types
         * @param to_ty Type to cast to
         * @param fd Optional FuncDecl of cast function to use if already found
         *
         * @return The compiled cast function or nullptr if not found
         */
        TypedValue* getCastFn(TypeNode *from_ty, TypeNode *to_ty, FuncDecl *fd = 0);

        /**
         * @brief Retrieves the FuncDecl specified
         *
         * @param name Basename of function to search for
         * @param args Argument types if multiple functions are found
         *
         * @return The FuncDecl if found or nullptr if not
         */
        FuncDecl* getMangledFuncDecl(string name, vector<TypeNode*> args);
        FuncDecl* getCastFuncDecl(TypeNode *from_ty, TypeNode *to_ty);

        /** @brief Compiles a function with inferred return type */
        TypedValue* compLetBindingFn(FuncDecl *fdn, vector<Type*> &paramTys);

        /**
         * @brief Compiles any non-generic function
         *
         * Generic functions (indicated by a typecheck returning
         * TypeCheckResult::SuccessWithTypeVars) should be compiled
         * with compTemplateFn which calls this function internally.
         */
        TypedValue* compFn(FuncDecl *fn);
        void registerFunction(FuncDeclNode *func);

        unsigned int getScope() const;

        /**
        * @brief Performs a lookup for a variable
        *
        * @param var Name of the variable to lookup
        *
        * @return The Variable* if found, otherwise nullptr
        */
        Variable* lookup(string var) const;

        /**
        * @brief Stores a variable in the current scope
        *
        * @param var Name of the variable to store
        * @param val Variable to store
        */
        void stoVar(string var, Variable *val);

        /**
        * @brief Performs a lookup for the specified DataType
        *
        * @param tyname Name of the type to lookup
        *
        * @return The DataType* if found, otherwise nullptr
        */
        DataType* lookupType(string tyname) const;

        /**
        * @brief Performs a lookup a type's full definition
        *
        * @return The DataType* if found, otherwise nullptr
        */
        DataType* lookupType(const TypeNode *tn) const;

        /**
        * @brief Performs a lookup for the specified trait
        *
        * @param tyname Name of the trait to lookup
        *
        * @return The Trait* if found, otherwise nullptr
        */
        Trait* lookupTrait(string tyname) const;
        bool typeImplementsTrait(DataType* dt, string traitName) const;

        /**
        * @brief Stores a new DataType
        *
        * @param ty The DataType to store
        * @param typeName The name of the DataType
        */
        void stoType(DataType *ty, string &typeName);

        /**
        * @brief Stores a TypeVar in the current scope
        *
        * @param name Name of the typevar to store (including the preceeding ')
        * @param ty The type to store
        */
        void stoTypeVar(string &name, TypeNode *ty);

        /**
         * @brief Searches through tn and replaces any typevars inside with
         * their definition from a lookup if found.
         *
         * Care must be taken so that the resulting TypeNode not escape the
         * scope of the typevars in the lookup.  Thus, this function should
         * not be used for TypeNodes that may be reused at lower scopes.
         */
        void searchAndReplaceBoundTypeVars(TypeNode* tn) const;

        Type* typeNodeToLlvmType(const TypeNode *tyNode);

        /** @brief Performs a type check against l and r */
        TypeCheckResult typeEq(const TypeNode *l, const TypeNode *r) const;

        /**
         * @brief Performs a type check against l and r
         *
         * Used for function parameters and similar situations where typevars
         * across multiple type checks need to be consistent.  Eg. a function
         * of type ('t, 't)->void should not match the arguments i32 and u64.
         * Performing a typecheck on each argument separately would give a different
         * bound value for 't.  Using this function would result in the appropriate
         * TypeCheckResult::Failure
         */
        TypeCheckResult typeEq(vector<TypeNode*> l, vector<TypeNode*> r) const;

        /**
         * @brief Expands any TT_Data -typed nodes inside of tn to contain their
         * constituent types.  Does not expand any nodes pointed to by a pointer
         * type to avoid infinite recursion.
         */
        void expand(TypeNode *tn);

        /**
         * @brief Performs an implicit widening
         *
         * @param num Integer to widen
         * @param castTy Type to widen to
         *
         * @return The widened integer
         */
        TypedValue* implicitlyWidenNum(TypedValue *num, TypeTag castTy);

        /** @brief Mutates numerical arguments to match types if possible */
        void handleImplicitConversion(TypedValue **lhs, TypedValue **rhs);

        /** @brief Mutates integer arguments to match types if not already */
        void implicitlyCastIntToInt(TypedValue **lhs, TypedValue **rhs);

        /** @brief Mutates floating-point arguments to match types if not already */
        void implicitlyCastFltToFlt(TypedValue **lhs, TypedValue **rhs);

        /** @brief Mutates an integer to a float */
        void implicitlyCastIntToFlt(TypedValue **tval, Type *ty);

        /**
        * @brief Compiles a module into an obj file to be used for linking.
        *
        * @param mod The already-compiled module
        * @param outFile Name of the file to output
        *
        * @return 0 on success
        */
        int compileIRtoObj(llvm::Module *mod, string outFile);

        TypedValue* getVoidLiteral();

        /**
        * @brief Invokes the linker specified by AN_LINKER (in target.h) to
        *        link each object file
        *
        * @param inFiles String containing each obj file to link separated with spaces
        * @param outFile Name of the file to output
        *
        * @return 0 on success
        */
        static int linkObj(string inFiles, string outFile);
    };

    TypedValue* addrOf(Compiler *c, TypedValue* tv);

    /**
    * @brief Retrieves the Nth node of a list
    *
    * Does not check if list contains at least n nodes
    *
    * @param node The head of the list
    * @param n Index of the node to return
    *
    * @return The nth node from the list
    */
    Node* getNthNode(Node *node, size_t n);

    /** @brief Counts the amount of Nodes in the list */
    size_t getTupleSize(Node *tup);

    /** @brief Converts the Node list argument into a vector */
    template<typename T> vector<T*> vectorize(T *args);

    /** @brief Converts the vector argument into a TypeNode list */
    TypeNode* toList(vector<TypeNode*> &nodes);

    /** @brief Extracts the type of each arg into a TypeNode vector */
    vector<TypeNode*> toTypeNodeVector(vector<TypedValue*> &tvs);

    string mangle(string &base, vector<TypeNode*> params);
    string mangle(string &base, NamedValNode *paramTys);
    string mangle(string &base, TypeNode *paramTys);
    string mangle(string &base, TypeNode *p1, TypeNode *p2);
    string mangle(string &base, TypeNode *p1, TypeNode *p2, TypeNode *p3);

    string removeFileExt(string file);

}

#endif
