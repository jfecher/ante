#include <llvm/IR/Verifier.h>          //for verifying basic structure of functions
#include <llvm/Support/FileSystem.h>   //for r/w when outputting bitcode
#include <llvm/Support/raw_ostream.h>  //for ostream when outputting bitcode
#include <llvm/Passes/PassBuilder.h>
#include <llvm/Support/raw_os_ostream.h>
#include <llvm/Transforms/Scalar.h>    //for most passes
#include <llvm/Transforms/IPO.h>
#include <llvm/IR/LegacyPassManager.h>
#include <llvm/Support/TargetRegistry.h>
#include <llvm/Target/TargetMachine.h>
#include <llvm/Linker/Linker.h>
#include <llvm/ExecutionEngine/SectionMemoryManager.h>
#include <llvm/ExecutionEngine/GenericValue.h>
#include <llvm/Transforms/IPO/AlwaysInliner.h>
#include <llvm/Transforms/IPO/PassManagerBuilder.h>

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <chrono>

#include "parser.h"
#include "compiler.h"
#include "function.h"
#include "types.h"
#include "trait.h"
#include "repl.h"
#include "uniontag.h"
#include "target.h"
#include "nameresolution.h"
#include "typeinference.h"
#include "util.h"

using namespace std;
using namespace llvm;
using namespace ante::parser;

namespace ante {

/**
 * @param tup The head of the list
 *
 * @return The length of the Node list
 */
size_t getTupleSize(Node *tup){
    size_t size = 0;
    while(tup){
        size++;
        tup = tup->next.get();
    }

    return size;
}

/**
 * @brief Does not check if list contains at least n nodes
 *
 * @param node The head of the list
 * @param n Index of the node to return
 *
 * @return The nth node from the list
 */
Node* getNthNode(Node *node, size_t n){
    for(; n > 0; n--)
        node = node->next.get();
    return node;
}

/**
 * @brief Compiles a list of expressions
 *
 * @param nList The list to compile
 *
 * @return The value of the last expression
 */
TypedValue compileStmtList(Node *nList, Compiler *c){
    TypedValue ret;
    for(Node &n : *nList){
        ret = CompilingVisitor::compile(c, &n);
    }
    return ret;
}


/**
 * @return True if the TypeTag is an unsigned integer type
 */
bool isUnsignedTypeTag(const TypeTag tt){
    return tt==TT_U8||tt==TT_U16||tt==TT_U32||tt==TT_U64||tt==TT_Usz;
}


void CompilingVisitor::visit(IntLitNode *n){
    val = TypedValue(ConstantInt::get(*c->ctxt,
                    APInt(getBitWidthOfTypeTag(n->typeTag),
                    atol(n->val.c_str()), isUnsignedTypeTag(n->typeTag))),
            AnType::getPrimitive(n->typeTag));
}


const fltSemantics& typeTagToFltSemantics(TypeTag tokTy){
    switch(tokTy){
        case TT_F16: return APFloat::IEEEhalf();
        case TT_F32: return APFloat::IEEEsingle();
        case TT_F64: return APFloat::IEEEdouble();
        default:     return APFloat::IEEEdouble();
    }
}

void CompilingVisitor::visit(FltLitNode *n){
    val = TypedValue(ConstantFP::get(*c->ctxt, APFloat(typeTagToFltSemantics(n->typeTag), n->val.c_str())),
            AnType::getPrimitive(n->typeTag));
}


void CompilingVisitor::visit(BoolLitNode *n){
    val = TypedValue(ConstantInt::get(*c->ctxt, APInt(1, (bool)n->val, true)),
            AnType::getBool());
}


/**
 * @brief Compiles a TypeNode
 *
 * @return The tag value if this node is a union tag, otherwise it returns
 *         a compile-time value of type Type
 */
void CompilingVisitor::visit(TypeNode *n){
    //check for enum value
    auto t = try_cast<AnSumType>(n->getType());
    if(t && t->name != "Type"){
        size_t tagIndex = t->getTagVal(n->typeName);
        Value *tag = ConstantInt::get(*c->ctxt, APInt(8, tagIndex, true));

        Type *unionTy = c->anTypeToLlvmType(t);
        Type *curTy = tag->getType();

        //allocate for the largest possible union member
        auto *alloca = c->builder.CreateAlloca(unionTy);

        //but make sure to bitcast it to the current member before storing an incorrect type
        Value *castTo = c->builder.CreateBitCast(alloca, curTy->getPointerTo());
        c->builder.CreateStore(tag, castTo);

        //load the initial alloca, not the bitcasted one
        Value *unionVal = c->builder.CreateLoad(alloca);
        val = TypedValue(unionVal, t);
    }else{
        ASSERT_UNREACHABLE("Cannot compile first-class types as values");
        // auto dt = try_cast<AnDataType>(n->getType());

        // //return the type as a value
        // auto *ty = dt->typeArgs[0];

        // //The TypeNode* address is wrapped in an llvm int so that llvm::Value methods can be called
        // //without crashing, even if their result is meaningless
        // Value *v = c->builder.getInt64((unsigned long)ty);
        // val = TypedValue(v, t);
    }
}

void CompilingVisitor::visit(StrLitNode *n){
    AnType *strty = c->compUnit->lookupType("Str");

    auto *ptr = c->builder.CreateGlobalStringPtr(n->val, "_strlit");

    //get the llvm Str data type from a fake type node in case we are compiling
    //the prelude && the Str data type isnt translated into an llvmty yet
    auto *tupleTy = cast<StructType>(c->anTypeToLlvmType(strty));

    vector<Constant*> strarr = {
        UndefValue::get(Type::getInt8PtrTy(*c->ctxt)),
        ConstantInt::get(*c->ctxt, APInt(AN_USZ_SIZE, n->val.length(), true))
    };

    auto *uninitStr = ConstantStruct::get(tupleTy, strarr);
    auto *str = c->builder.CreateInsertValue(uninitStr, ptr, 0);

    this->val = TypedValue(str, strty);
}

void CompilingVisitor::visit(CharLitNode *n){
    this->val = TypedValue(ConstantInt::get(*c->ctxt, APInt(8, n->val, true)), AnType::getPrimitive(TT_C8));
}


void CompilingVisitor::visit(ArrayNode *n){
    vector<Constant*> arr;

    for(auto& elem : n->exprs){
        auto tval = CompilingVisitor::compile(c, elem);
        arr.push_back((Constant*)tval.val);
    }

    if(n->exprs.empty()){
        auto *ty = ArrayType::get(Type::getInt8Ty(*c->ctxt)->getPointerTo(), 0);
        auto *carr = ConstantArray::get(ty, arr);
        this->val = TypedValue(carr, n->getType());
    }else{
        auto *ty = ArrayType::get(arr[0]->getType(), n->exprs.size());
        auto *carr = ConstantArray::get(ty, arr);
        this->val = TypedValue(carr, n->getType());
    }
}

/**
 * @brief Creates and returns a literal of type unit
 *
 * Note that care must be taken not to accidentally pass this value
 * to functions and elide it from tuple types such as (i32, unit, i8).
 * Which are represented in memory as (i32, i8).
 *
 * @return A unit literal
 */
TypedValue Compiler::getUnitLiteral(){
    return TypedValue(UndefValue::get(Type::getVoidTy(*ctxt)), AnType::getUnit());
}

void CompilingVisitor::visit(TupleNode *n){
    //A void value is represented by the empty tuple, ()
    if(n->exprs.empty()){
        this->val = c->getUnitLiteral();
        return;
    }

    auto elemTys = vecOf<AnType*>(n->exprs.size());
    auto vals = vecOf<Value*>(n->exprs.size());

    for(auto &expr : n->exprs){
        expr->accept(*this);
        vals.push_back(this->val.val);
        elemTys.push_back(this->val.type);
    }

    Value* tuple = c->tupleOf(vals, false);

    auto *tupTy = AnTupleType::get(elemTys);
    this->val = TypedValue(tuple, tupTy);
}


/**
 * @brief Compiles a tuple's elements and returns them in a vector
 *
 * @return A vector of a tuple's elements
 */
vector<TypedValue> TupleNode::unpack(Compiler *c){
    vector<TypedValue> ret;
    for(auto& n : exprs){
        auto tv = CompilingVisitor::compile(c, n);

        if(tv)
            ret.push_back(tv);
    }
    return ret;
}


/*
 *  When a retnode is compiled within a block, care must be taken to not
 *  forcibly insert the branch instruction afterwards as it leads to dead code.
 */
void CompilingVisitor::visit(RetNode *n){
    n->expr->accept(*this);

    val = val.type->typeTag == TT_Unit ?
        TypedValue(c->builder.CreateRetVoid(), val.type) :
        TypedValue(c->builder.CreateRet(val.val), val.type);
}



/*
 * TODO: implement for abitrary compile-time Str expressions
 */
void CompilingVisitor::visit(ImportNode*){
    val = c->getUnitLiteral();
}


void CompilingVisitor::visit(WhileNode *n){
    Function *f = c->builder.GetInsertBlock()->getParent();
    BasicBlock *cond  = BasicBlock::Create(*c->ctxt, "while_cond", f);
    BasicBlock *begin = BasicBlock::Create(*c->ctxt, "while", f);
    BasicBlock *end   = BasicBlock::Create(*c->ctxt, "end_while", f);

    c->builder.CreateBr(cond);
    c->builder.SetInsertPoint(cond);

    //Allow break/continue to occur in the while condition
    //portion of the loop to allow them in the "body" of
    //while ... do () loops
    c->compCtxt->breakLabels->push_back(end);
    c->compCtxt->continueLabels->push_back(cond);

    try{
        n->condition->accept(*this);

        c->builder.CreateCondBr(val.val, begin, end);
        c->builder.SetInsertPoint(begin);

        n->child->accept(*this);
    }catch(CtError const& e){
        c->compCtxt->breakLabels->pop_back();
        c->compCtxt->continueLabels->pop_back();
        throw e;
    }

    c->compCtxt->breakLabels->pop_back();
    c->compCtxt->continueLabels->pop_back();

    if(!dyn_cast<ReturnInst>(val.val) && !dyn_cast<BranchInst>(val.val))
        c->builder.CreateBr(cond);

    c->builder.SetInsertPoint(end);
    this->val = c->getUnitLiteral();
}

TypedValue compForLoopTraitFn(Compiler *c, string const& fnName, TraitImpl *impl, AnType *argTy, LOC_TY &loc);

TypedValue callForLoopTraitFn(Compiler *c, string const& fnName, TypedValue const& arg, LOC_TY &loc){
    TraitImpl *impl = fnName == "into_iter" ?
        c->compUnit->lookupTraitImpl("Iterable", {arg.type}):
        c->compUnit->lookupTraitImpl("Iterator", {arg.type});

    TypedValue fn = compForLoopTraitFn(c, fnName, impl, arg.type, loc);

    array<Value*, 1> args{arg.val};
    Value *call = c->builder.CreateCall(fn.val, args);
    return {call, fn.type->getFunctionReturnType()};
}


void CompilingVisitor::visit(ForNode *n){
    Function *f = c->builder.GetInsertBlock()->getParent();
    BasicBlock *cond  = BasicBlock::Create(*c->ctxt, "for_cond", f);
    BasicBlock *begin = BasicBlock::Create(*c->ctxt, "for", f);
    BasicBlock *incr = BasicBlock::Create(*c->ctxt, "for_incr", f);
    BasicBlock *end   = BasicBlock::Create(*c->ctxt, "end_for", f);

    auto rangev = CompilingVisitor::compile(c, n->range);

    //check if the range expression is its own iterator and thus implements Iterator
    //If it does not, see if it implements Iterable by attempting to call into_iter on it
    rangev = callForLoopTraitFn(c, "into_iter", rangev, n->range->loc);
    if(!rangev)
        error("Range expression of type " + anTypeToColoredStr(rangev.type) + " needs to implement " +
            lazy_str("Iterable", AN_TYPE_COLOR) + " and " + lazy_str("Iterator", AN_TYPE_COLOR) +
            " to be used in a for loop", n->range->loc);

    //by this point, rangev now properly stores the range information,
    //so store it on the stack and insert calls to unwrap, has_next,
    //and next at the beginning, beginning, and end of the loop respectively.
    Value *alloca = c->builder.CreateAlloca(rangev.getType());
    c->builder.CreateStore(rangev.val, alloca);

    c->builder.CreateBr(cond);
    c->builder.SetInsertPoint(cond);

    //set var = cur_elem range
    //candval = is_done range
    auto rangeVal = TypedValue(c->builder.CreateLoad(alloca), rangev.type);

    TypedValue is_done = callForLoopTraitFn(c, "has_next", rangeVal, n->range->loc);
    if(!is_done) error("Range expression of type " + anTypeToColoredStr(rangev.type) + " does not implement " +
            lazy_str("Iterable", AN_TYPE_COLOR) + ", which it needs to be used in a for loop", n->range->loc);

    c->builder.CreateCondBr(is_done.val, begin, end);
    c->builder.SetInsertPoint(begin);

    //call unwrap at start of loop
    //make sure to update rangeVal
    rangeVal = TypedValue(c->builder.CreateLoad(alloca), rangev.type);
    TypedValue uwrap = callForLoopTraitFn(c, "cur_elem", rangeVal, n->range->loc);
    if(!uwrap) error("Range expression of type " + anTypeToColoredStr(rangev.type) + " does not implement " +
            lazy_str("Iterable", AN_TYPE_COLOR) + ", which it needs to be used in a for loop", n->range->loc);

    TypeError err{"A for-loop's binding pattern should match the return type of the iterator's unwrap function, but found " +
            anTypeToColoredStr(n->pattern->getType()) + " and " + anTypeToColoredStr(uwrap.type) + " respectively", n->pattern->loc};

    auto subs = unify({{n->pattern->getType(), uwrap.type, err}});
    c->compCtxt->insertMonomorphisationMappings(subs);

    auto vn = dynamic_cast<VarNode*>(n->pattern.get());
    if(vn){
        vn->decl->tval = uwrap;
    }

    //TODO: handle arbitrary patterns
    // auto *decl = n->pattern->decls[0];
    // decl->tval = uwrap;

    //register the branches to break/continue to right before the body
    //is compiled in case there was an error compiling the range
    c->compCtxt->breakLabels->push_back(end);
    c->compCtxt->continueLabels->push_back(incr);

    //compile the rest of the loop's body
    try{
        n->child->accept(*this);
    }catch(CtError const& e){
        c->compCtxt->breakLabels->pop_back();
        c->compCtxt->continueLabels->pop_back();
        throw e;
    }

    c->compCtxt->breakLabels->pop_back();
    c->compCtxt->continueLabels->pop_back();

    if(!val) return;
    if(!dyn_cast<ReturnInst>(val.val) && !dyn_cast<BranchInst>(val.val)){
        //set range = next range
        c->builder.CreateBr(incr);
        c->builder.SetInsertPoint(incr);

        TypedValue arg = {c->builder.CreateLoad(alloca), rangev.type};
        TypedValue next = callForLoopTraitFn(c, "advance", arg, n->range->loc);
        if(!next) error("Range expression of type " + anTypeToColoredStr(rangev.type) + " does not implement "
                + lazy_str("Iterable", AN_TYPE_COLOR) + ", which it needs to be used in a for loop", n->range->loc);

        c->builder.CreateStore(next.val, alloca);
        c->builder.CreateBr(cond);
    }

    c->builder.SetInsertPoint(end);
    this->val = c->getUnitLiteral();
}


void CompilingVisitor::visit(JumpNode *n){
    n->expr->accept(*this);

    auto *ci = dyn_cast<ConstantInt>(val.val);
    if(!ci)
        error("Expression must evaluate to a constant integer\n", n->expr->loc);

    if(!isUnsignedTypeTag(val.type->typeTag) && ci->getSExtValue() < 0)
        error("Cannot jump out of a negative number (" + to_string(ci->getSExtValue()) +  ") of loops", n->expr->loc);

    //we can now safely get the zero-extended value of ci since even if it is signed, it is not negative
    auto jumpCount = ci->getZExtValue();

    //NOTE: continueLabels->size() == breakLabels->size() always
    auto loopCount = c->compCtxt->breakLabels->size();

    if(loopCount == 0)
        error("There are no loops to jump out of", n->loc);


    if(jumpCount == 0)
        error("Cannot jump out of 0 loops", n->expr->loc);


    if(jumpCount > loopCount)
        error("Cannot jump out of " + to_string(jumpCount) + " loops when there are only " +
                to_string(c->compCtxt->breakLabels->size()) + " loop(s) nested", n->expr->loc);

    //actually create the branch instruction
    BranchInst *br = n->jumpType == Tok_Continue ?
        c->builder.CreateBr( c->compCtxt->continueLabels->at(loopCount - jumpCount) ) :
        c->builder.CreateBr( c->compCtxt->breakLabels->at(loopCount - jumpCount) );

    //Although returning a void, use the br as the value so loops know the last instruction was a br and not to insert another
    this->val = TypedValue(br, AnType::getUnit());
}


//create a new scope if the user indents
void CompilingVisitor::visit(BlockNode *n){
    n->block->accept(*this);
}


/**
 *  @brief This is a stub.  Compilation of parameters is handled within Compiler::compFn
 */
void CompilingVisitor::visit(NamedValNode*)
{
    //STUB
}

/**
 * @brief Performs a lookup for an identifier and returns its value if found
 *
 * @return The value of the variable
 */
void CompilingVisitor::visit(VarNode *n){
    if(!n->decl->tval.val && n != n->decl->definition){
        n->decl->definition->accept(*this);
    }
    val = n->decl->tval;
    if(n->decl->tval.type->hasModifier(Tok_Mut) && val.val->getType()->isPointerTy()){
        val.val = c->builder.CreateLoad(val.val, n->name);
    }
}

/**
 * @brief Helper function to compile a variable declaration with no specified type.
 *        Matches the type of the variable with the init expression's type.
 *
 * @param node The declaration expression
 *
 * @return The newly-declared variable with an inferred type
 */
void compMutBinding(VarAssignNode *node, CompilingVisitor &cv){
    Compiler *c = cv.c;
    if(!dynamic_cast<VarNode*>(node->ref_expr))
        error("Unknown pattern for l-expr", node->expr->loc);

    auto *decl = static_cast<VarNode*>(node->ref_expr)->decl;

    node->expr->accept(cv);
    TypedValue &val = cv.val;

    for(auto &n : node->modifiers){
        TokenType m = (TokenType)n->mod;
        val.type = (AnType*)val.type->addModifier(m);
    }

    if(cv.c->isJIT)
        val.type = (AnType*)val.type->addModifier(Tok_Ante);

    //set the value as mutable if not already.
    val.type = (AnType*)val.type->addModifier(Tok_Mut);

    //location to store var
    Value *ptr = decl->isGlobal() ?
            (Value*) new GlobalVariable(*c->module, val.getType(), false,
                    GlobalValue::PrivateLinkage, UndefValue::get(val.getType()), decl->name) :
            c->builder.CreateAlloca(val.getType(), nullptr, decl->name.c_str());

    TypedValue alloca{ptr, val.type};
    decl->tval = alloca;

    c->builder.CreateStore(val.val, alloca.val);
    cv.val = c->getUnitLiteral();
}


void compLetBinding(VarAssignNode *node, CompilingVisitor &cv){
    Compiler *c = cv.c;
    if(!dynamic_cast<VarNode*>(node->ref_expr))
        error("Unknown pattern for l-expr", node->expr->loc);

    auto *decl = static_cast<VarNode*>(node->ref_expr)->decl;

    TypedValue val = CompilingVisitor::compile(c, node->expr);

    for(auto &n : node->modifiers){
        TokenType m = (TokenType)n->mod;
        val.type = (AnType*)val.type->addModifier(m);
    }

    if(cv.c->isJIT)
        val.type = (AnType*)val.type->addModifier(Tok_Ante);

    if(decl->isGlobal()){
        //location to store var
        Value *ptr = new GlobalVariable(*c->module, val.getType(), false,
                        GlobalValue::PrivateLinkage, UndefValue::get(val.getType()), decl->name);

        TypedValue alloca{ptr, val.type};
        decl->tval = alloca;
        cv.val = {c->builder.CreateStore(val.val, alloca.val), val.type};
    }else{
        decl->tval = val;
        cv.val = val;
    }
}


void CompilingVisitor::visit(ModNode *n){
    cerr << "Warning: " << Lexer::getTokStr(n->mod) << " unimplemented in expr:\n";
    PrintingVisitor::print(n);
    n->expr->accept(*this);
}


/**
 * @brief Compiles an insertion operand into a named field. eg. str#len = 2
 *
 * @param bop The field extract that is the lhs of the insertion expression
 * @param expr The rhs of the insertion expression
 *
 * @return A void literal
 */
TypedValue compFieldInsert(Compiler *c, BinOpNode *bop, Node *expr){
    VarNode *field = static_cast<VarNode*>(bop->rval.get());

    //A . operator can also have a type/module as its lval, but its
    //impossible to insert into a non-value so fail if the lvalue is one
    if(auto *tn = dynamic_cast<TypeNode*>(bop->lval.get()))
        error("Cannot insert value into static module '" +
                anTypeToColoredStr(toAnType(tn, c->compUnit)), tn->loc);


    Value *val;
    AnType *tyn;

    //prevent l from being used after this scope; only val and tyn should be used as only they
    //are updated with the automatic pointer dereferences.
    {
        CompilingVisitor cv{c};
        auto l = compileRefExpr(cv, bop->lval.get(), expr);

        val = l.val;
        tyn = l.type;
    }

    //the . operator automatically dereferences pointers, so update val and tyn accordingly.
    while(auto *ptr = try_cast<AnPtrType>(tyn)){
        val = c->builder.CreateLoad(val);
        tyn = ptr->extTy;
    }

    //this is the variable that will store the changes after the later insertion
    Value *var = static_cast<LoadInst*>(val)->getPointerOperand();

    //check to see if this is a field index
    if(auto dataTy = try_cast<AnProductType>(tyn)){
        auto index = dataTy->getFieldIndex(field->name);

        if(index != -1){
            auto newval = CompilingVisitor::compile(c, expr);

            Value *nv = newval.val;
            Type *nt = val->getType()->getStructElementType(index);

            //Type check may succeed if a void* is being inserted into any ptr slot,
            //but llvm will still complain so we create a bit cast to appease it
            if(nv->getType() != nt && newval.type->typeTag == TT_Ptr) {
                nv = c->builder.CreateBitCast(nv, nt);
            }

            auto *ins = c->builder.CreateInsertValue(val, nv, index);

            c->builder.CreateStore(ins, var);
            return c->getUnitLiteral();
        }
    }

    error("Method/Field " + field->name + " not found in type " + anTypeToColoredStr(tyn), bop->loc);
    return {};
}

/**
 * @brief Keeps track of assignments to variables
 *
 * This will fail if the assignment is not in the form: ident := expr
 */
TypedValue compileRefExpr(CompilingVisitor &cv, Node *refExpr, Node *assignExpr){
    refExpr->accept(cv);
    auto li = dyn_cast<LoadInst>(cv.val.val);

    if(li){
        return {li->getPointerOperand(), AnPtrType::get(cv.val.type)};
    }else if(cv.val.getType()->isPointerTy()){
        auto ptrty = try_cast<AnPtrType>(cv.val.type);
        return {cv.c->builder.CreateLoad(cv.val.val), ptrty};
    }

    show(refExpr);
    show(assignExpr);
    cv.val.dump();
    ASSERT_UNREACHABLE("Tried to mutate an immutable variable");
}

/**
 * @brief Compiles an assign expression of an already-declared variable
 *
 * @return A void literal
 */
void CompilingVisitor::visit(VarAssignNode *n){
    //If this is an insert value (where the lval resembles var[index] = ...)
    //then this must be instead compiled with compInsert, otherwise the [ operator
    //would retrieve the value at the index instead of the reference for storage.
    if(BinOpNode *bop = dynamic_cast<BinOpNode*>(n->ref_expr)){
        if(bop->op == '#'){
            this->val = c->compInsert(bop, n->expr.get());
            return;
        }else if(bop->op == '.'){
            this->val = compFieldInsert(c, bop, n->expr.get());
            return;
        }
    }

    if(n->hasModifier(Tok_Mut)){
        compMutBinding(n, *this);
        return;
    }else if(!n->modifiers.empty()){
        compLetBinding(n, *this);
        return;
    }

    //otherwise, this is just a normal assign to a variable
    this->val = compileRefExpr(*this, n->ref_expr, n->expr.get());
    Value *dest = val.val;

    //compile the expression to store
    TypedValue assignExpr = CompilingVisitor::compile(c, n->expr);

    //lvalue must compile to a pointer for storage, usually an alloca value
    if(!PointerType::isLoadableOrStorableType(val.getType())){
        error("Attempted assign without a memory address, with type "
                + anTypeToColoredStr(val.type), n->ref_expr->loc);
    }

    //now actually create the store
    c->builder.CreateStore(assignExpr.val, dest);

    //all assignments return a void value
    this->val = c->getUnitLiteral();
}


/**
 * @brief Mangles a function name
 *
 * @param base The unmangled function name
 * @param params The type of each parameter of the function
 *
 * @return The mangled version of the function name
 */
string mangle(string const& base, vector<AnType*> const& params){
    string name = base;
    for(auto *tv : params){
        if(tv->typeTag != TT_Unit)
            name += "_" + anTypeToStr(tv);
    }
    return name;
}

string mangle(FuncDecl *fd, vector<AnType*> const& params){
    if(fd->isDecl())
        return fd->getName();

    string name = fd->getName();
    for(auto *tv : params)
        if(tv->typeTag != TT_Unit)
            name += "_" + anTypeToStr(tv);
    return name;
}

string mangle(string const& base, shared_ptr<NamedValNode> const& paramTys){
    string name = base;
    NamedValNode *cur = paramTys.get();
    while(cur){
        auto *tn = (TypeNode*)cur->typeExpr.get();

        if(!tn)
            name += "...";
        else if(tn == (void*)1)
            name += AN_MANGLED_SELF;
        else if(tn->typeTag != TT_Unit)
            name += "_" + typeNodeToStr(tn);

        cur = (NamedValNode*)cur->next.get();
    }
    return name;
}

string mangle(string const& base, TypeNode *paramTys){
    string name = base;
    while(paramTys){
        if(paramTys->typeTag != TT_Unit)
            name += "_" + typeNodeToStr(paramTys);
        paramTys = (TypeNode*)paramTys->next.get();
    }
    return name;
}

string mangle(string const& base, TypeNode *p1, TypeNode *p2){
    string name = base;
    string param1 = "_" + typeNodeToStr(p1);
    string param2 = "_" + typeNodeToStr(p2);
    name += param1 + param2;
    return name;
}

string mangle(string const& base, TypeNode *p1, TypeNode *p2, TypeNode *p3){
    string name = base;
    string param1 = "_" + typeNodeToStr(p1);
    string param2 = "_" + typeNodeToStr(p2);
    string param3 = "_" + typeNodeToStr(p3);
    name += param1 + param2 + param3;
    return name;
}

/**
 * @brief Given a list of FuncDeclNodes, returns the function whose name
 *        matches basename, or returns nullptr if not found.
 *
 * @param list A list containing only FuncDeclNodes
 * @param basename The basename of the function to search for
 *
 * @return The FuncDeclNode sharing the basename or nullptr if no matching
 *         functions were found.
 */
FuncDeclNode* findFDN(Node *list, string const& basename){
    for(Node &n : *list){
        auto *fdn = (FuncDeclNode*)&n;

        if(fdn->name == basename){
            return fdn;
        }
    }
    return nullptr;
}


void CompilingVisitor::visit(ExtNode*){
    this->val = c->getUnitLiteral();
}

void CompilingVisitor::visit(DataDeclNode *n){
    //updateLlvmTypeBinding(c, data, true);
    this->val = c->getUnitLiteral();
}


void CompilingVisitor::visit(TraitNode *n){
    this->val = c->getUnitLiteral();
}


/**
 * @brief This is a stub until patterns are properly implemented
 *
 * @return A void literal
 */
void CompilingVisitor::visit(MatchBranchNode *n){
    //STUB
}

/**
 * @brief Removes all text after the final . in a string
 *
 * @return The string with the file extension removed
 */
string removeFileExt(string file){
    auto index = file.find_last_of('.');
    return index == string::npos ? file : file.substr(0, index);
}

string Compiler::getModuleName() const {
    return removeFileExt(this->fileName);
}


template<typename T>
void compileAll(Compiler *c, vector<T> &vec){
    CompilingVisitor v{c};
    for(auto &elem : vec){
        try{
            elem->accept(v);
        }catch(CtError const& err){}
    }
}


bool Compiler::scanAllDecls(RootNode *root){
    NameResolutionVisitor v{getModuleName()};
    this->compUnit = v.compUnit;
    root->accept(v);
    if(!errorCount())
        TypeInferenceVisitor::infer(root, compUnit);
    return errorCount();
}

void Compiler::eval(){
    //setup compiler
    // createMainFn();
    startRepl(this);
}

Function* Compiler::createMainFn(){
    Type* argcty = Type::getInt32Ty(*ctxt);
    Type* argvty = Type::getInt8Ty(*ctxt)->getPointerTo()->getPointerTo();

    //get or create the function type for the main method: (i32, c8**)->i32
    array<Type*, 2> args{argcty, argvty};
    FunctionType *ft = isLib?
        FunctionType::get(Type::getInt32Ty(*ctxt), {}, false):
        FunctionType::get(Type::getInt32Ty(*ctxt), args, false);

    //Actually create the function in module m
    string fnName = isLib ? "init_module" : "main";
    Function *main = Function::Create(ft, Function::ExternalLinkage, fnName, module.get());

    //Create the entry point for the function
    BasicBlock *bb = BasicBlock::Create(*ctxt, "entry", main);
    builder.SetInsertPoint(bb);

    if(!isLib){
        //create argc and argv global vars
        auto *argc = new GlobalVariable(*module, argcty, false, GlobalValue::PrivateLinkage, builder.getInt32(0), "argc");
        auto *argv = new GlobalVariable(*module, argvty, false, GlobalValue::PrivateLinkage, ConstantPointerNull::get(dyn_cast<PointerType>(argvty)), "argv");
        auto args = main->arg_begin();

        builder.CreateStore(&*args, argc);
        builder.CreateStore(&*++args, argv);
    }

    return main;
}


void CompilingVisitor::visit(RootNode *n){
    if(c->scanAllDecls(n))
        return;

    //Compile the rest of the program
    for(auto &node : n->main){
        try{
            if(node)
                node->accept(*this);
        }catch(CtError const& e){}
    }

    if(n->main.empty())
        this->val = c->getUnitLiteral();
}

/**
 * @brief Fills the given PassManager with passes appropriate
 * for the given optLvl.
 *
 * @param pm The PassManager to add passes to
 * @param optLvl The optimization level in the range 0..3.
 * Determines which passes should be added.  With 3 being
 * all passes.
 */
void addPasses(llvm::Module *m, char optLvl){
    using namespace std::chrono;
    auto start = high_resolution_clock::now();
    if(optLvl > 0){
        llvm::verifyModule(*m, &dbgs());

        llvm::legacy::PassManager pm;
        llvm::PassManagerBuilder pmb;
        pmb.OptLevel = optLvl;
        pmb.populateModulePassManager(pm);
        pm.run(*m);
    }
    auto end = high_resolution_clock::now();
    if(showTimingInformation())
        cout << "Llvm Optimizations: " << duration_cast<milliseconds>(end - start).count() << "ms\n";
}



void Compiler::compile(){
    if(compiled){
        cerr << "Module " << module->getName().str() << " is already compiled, cannot recompile.\n";
        return;
    }

    using namespace std::chrono;
    auto start = high_resolution_clock::now();

    try {
        //create implicit main function and import the prelude
        createMainFn();

        CompilingVisitor::compile(this, ast);

        //always return 0
        builder.CreateRet(ConstantInt::get(*ctxt, APInt(32, 0)));


        auto end = high_resolution_clock::now();
        if(showTimingInformation())
            std::cout << "Compiling: " << duration_cast<milliseconds>(end - start).count() << "ms\n";

        if(!errorCount() && !isLib){
            addPasses(module.get(), optLvl);
        }

        //flag this module as compiled.
        compiled = true;
    }catch(CtError const& e){
        if(!errorCount()){
            ASSERT_UNREACHABLE("Top-level exception caught without an error");
        }
    }

    if(errorCount()){
        fputs("Compilation aborted.\n", stderr);
        exit(1);
    }
}


void Compiler::compileNative(){
    if(!compiled) compile();

    //this file will become the obj file before linking
    string objFile = outFile + ".o";

    if(!compileIRtoObj(module.get(), objFile)){
        linkObj(objFile, outFile);
        remove(objFile.c_str());
    }
}

int Compiler::compileObj(string &outName){
    if(!compiled) compile();

    string modName = getModuleName();
    string objFile = outName.length() > 0 ? outName : modName + ".o";

    return compileIRtoObj(module.get(), objFile);
}


const Target* getTarget(){
    LLVMInitializeNativeTarget();
    LLVMInitializeNativeAsmPrinter();
    string err = "";

    string triple = Triple(AN_NATIVE_ARCH, AN_NATIVE_VENDOR, AN_NATIVE_OS).getTriple();
    const Target* target = TargetRegistry::lookupTarget(triple, err);

    if(!err.empty()){
        cerr << err << endl;
        cerr << "Selected triple: " << AN_NATIVE_ARCH ", " AN_NATIVE_VENDOR ", " AN_NATIVE_OS << endl;
        cerr << "\nRegistered targets:\n";
        llvm::raw_os_ostream os{std::cout};
        TargetRegistry::printRegisteredTargetsForVersion(os);
        exit(1);
    }

    return target;
}

TargetMachine* getTargetMachine(){
    auto *target = getTarget();

    string cpu = "";
    string features = "";
    string triple = Triple(AN_NATIVE_ARCH, AN_NATIVE_VENDOR, AN_NATIVE_OS).getTriple();
    TargetOptions op;

    TargetMachine *tm = target->createTargetMachine(triple, cpu, features, op, Reloc::Model::PIC_,
            None, CodeGenOpt::Level::Aggressive);

    if(!tm){
        cerr << "Error when initializing TargetMachine.\n";
        exit(1);
    }

    return tm;
}


void Compiler::jitFunction(Function *f){
    if(!jit.get()){
        auto* eBuilder = new EngineBuilder(unique_ptr<llvm::Module>(module.get()));

        string err;

        jit.reset(eBuilder->setErrorStr(&err).setEngineKind(EngineKind::JIT).create());
        if(err.length() > 0) cerr << err << endl;
    }

    compileIRtoObj(module.get(), (".tmp_" + f->getName()).str());
    jit->addModule(move(module));
    jit->finalizeObject();
    remove((".tmp_" + f->getName()).str().c_str());

    auto* fn = jit->getPointerToFunction(f);

    if(fn)
        reinterpret_cast<void(*)()>(fn)();
}


int Compiler::compileIRtoObj(llvm::Module *mod, string outFile){
    using namespace std::chrono;
    auto start = high_resolution_clock::now();

    auto *tm = getTargetMachine();

    char **err = nullptr;
    char *filename = (char*)outFile.c_str();
    int res = LLVMTargetMachineEmitToFile(
        (LLVMTargetMachineRef)tm,
        (LLVMModuleRef)mod,
        filename,
        (LLVMCodeGenFileType)llvm::TargetMachine::CGFT_ObjectFile, err);

    delete tm;
    auto end = high_resolution_clock::now();
    if(showTimingInformation())
        std::cout << "Writing .ll: " << duration_cast<milliseconds>(end - start).count() << "ms\n";
    return res;
}


int Compiler::linkObj(string inFiles, string outFile){
    using namespace std::chrono;
    auto start = high_resolution_clock::now();

    string cmd = AN_LINKER " " + inFiles + " -o " + outFile;
    int ret = system(cmd.c_str());

    auto end = high_resolution_clock::now();
    if(showTimingInformation())
        cout << "Linking: " << duration_cast<milliseconds>(end - start).count() << "ms\n";
    return ret;
}


void Compiler::emitIR(){
    if(!compiled) compile();

    std::error_code ec;
    auto&& fd = raw_fd_ostream(outFile + ".ll", ec, llvm::sys::fs::OpenFlags::F_Text);
    module->print(fd, nullptr);
}


/**
 * @brief Prints type and value of TypeNode to stdout
 */
void TypedValue::dump() const{
    if(!type){
        cout << "(null)\n";
        return;
    }

    cout << "type:\t" << anTypeToStr(type) << endl
         << "val:\t" << flush;

    if(type->typeTag == TT_Unit)
        puts("void ()");
    else if(type->typeTag == TT_Type)
        cout << anTypeToStr(extractTypeValue(*this)) << endl;
    else if(type->typeTag == TT_MetaFunction)
        cout << "(compiler API function)\n";
    else{
        if(val){
            val->print(llvm::dbgs(), false);
            llvm::dbgs() << '\n';
        }else{
            cout << "(null)\n";
        }
    }
}


/*
 * Helper function to create an llvm integer literal
 * with the address of a pointer as its value
 */
Value* mkPtrInt(Compiler *c, void *addr){
    return c->builder.getInt64((unsigned long)addr);
}

/**
 * Returns the directory prefix of a filename.
 * If there is none, an empty string is returned.
 * Eg. dirprefix("path/to/file") == "path/to/"
 */
string dirprefix(string &f){
    auto c = f.find_last_of("/");
    if(c != string::npos){
        return f.substr(0, c+1);
    }else{
        return "";
    }
}


/**
 * @brief The main constructor for Compiler
 *
 * @param _fileName Name of the file being compiled
 * @param lib Set to true if this module should be compiled as a library
 * @param llvmCtxt The llvmCtxt possibly shared with another module
 */
Compiler::Compiler(const char *_fileName, bool lib, shared_ptr<LLVMContext> llvmCtxt) :
        ctxt(llvmCtxt ? llvmCtxt : shared_ptr<LLVMContext>(new LLVMContext())),
        builder(*ctxt),
        compUnit(nullptr),
        compCtxt(new CompilerCtxt()),
        ctCtxt(new CompilerCtCtxt()),
        compiled(false),
        isLib(lib),
        isJIT(false),
        fileName(_fileName? _fileName : "(stdin)"),
        funcPrefix(""),
        scope(0), optLvl(2), fnScope(1){

    if(_fileName){
        string* fileName_cpy = new string(fileName);
        setLexer(new Lexer(fileName_cpy));
        yy::parser p{};
        int flag = p.parse();
        if(flag != PE_OK){ //parsing error, cannot procede
            //print out remaining errors
            int tok;
            yy::location loc;
            while((tok = yylexer->next(&loc)) != Tok_Newline && tok != 0);
            while(p.parse() != PE_OK && yylexer->peek() != 0);

            fputs("Syntax error, aborting.\n", stderr);
            exit(flag);
        }

        RootNode* root = parser::getRootNode();
        this->ast = root;
    }

    //Add this module to the cache to ensure it is not compiled twice
    outFile = getModuleName();
    if (outFile.empty())
        outFile = "a.out";

    module.reset(new llvm::Module(outFile, *ctxt));
}

/**
 * @brief Constructor for a Compiler compiling a sub-module within the current file.  Currently only
 * used for string interpolation.
 *
 * @param root The node to set as the root node (does not need to be a RootNode already)
 * @param modName Name of the module being compiled
 * @param fName Name of the file being compiled
 * @param lib Set to true if this module should be compiled as a library
 * @param llvmCtxt The llvmCtxt shared from the parent Module
 */
Compiler::Compiler(Compiler *c, Node *root, string modName, bool lib) :
        ctxt(c->ctxt),
        builder(*ctxt),
        compUnit(nullptr),
        compCtxt(new CompilerCtxt()),
        ctCtxt(c->ctCtxt),
        compiled(false),
        isLib(lib),
        isJIT(false),
        fileName(c->fileName),
        outFile(modName),
        funcPrefix(""),
        scope(0), optLvl(2), fnScope(1){

    module.reset(new llvm::Module(outFile, *ctxt));
    this->ast = (RootNode*)root;
}

// TODO: Have a Context or Config class that is passed around
//       to each visitor instead of having globals
bool showTimingInformationGlobal = false;
bool showTimingInformation() {
    return showTimingInformationGlobal;
}

void Compiler::processArgs(CompilerArgs *args){
    string out = "";
    bool shouldGenerateExecutable = true;
    showTimingInformationGlobal = args->hasArg(Args::Time);

    if(auto *arg = args->getArg(Args::OutputName)){
        outFile = arg->arg;
        out = outFile;
    }

    if(auto *arg = args->getArg(Args::OptLvl)){
        if(arg->arg == "0") optLvl = 0;
        else if(arg->arg == "1") optLvl = 1;
        else if(arg->arg == "2") optLvl = 2;
        else if(arg->arg == "3") optLvl = 3;
        else{ cerr << "Unrecognized OptLvl " << arg->arg << endl; return; }
    }


    //make sure even non-called functions are included in the binary
    //if the -lib flag is set
    if(args->hasArg(Args::Lib)){
        isLib = true;
        if(!compiled) compile();

        for(auto &f : ast->funcs)
            CompilingVisitor::compile(this, f);
    }

    if(args->hasArg(Args::Check)){
        if(!compiled) compile();
        shouldGenerateExecutable = false;
    }

    if(args->hasArg(Args::EmitLLVM)){
        emitIR();
        shouldGenerateExecutable = false;
    }

    if(args->hasArg(Args::Parse))
        shouldGenerateExecutable = false;

    if(args->hasArg(Args::CompileToObj)){
        compileObj(out);
        shouldGenerateExecutable = false;
    }

    if(args->hasArg(Args::CompileAndRun))
        shouldGenerateExecutable = true;

    if(shouldGenerateExecutable){
        compileNative();

        if(!errorCount() && args->hasArg(Args::CompileAndRun)){
            int res = system((AN_EXEC_STR + outFile).c_str());
            if(res) return; //silence unused return result warning
        }
    }
}

Compiler::~Compiler(){
    if(yylexer){
        delete yylexer;
        yylexer = 0;
    }
}

} //end of namespace ante
