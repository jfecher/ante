#include <llvm/IR/Verifier.h>          //for verifying basic structure of functions
#include <llvm/Support/FileSystem.h>   //for r/w when outputting bitcode
#include <llvm/Support/raw_ostream.h>  //for ostream when outputting bitcode
#include "llvm/Transforms/Scalar.h"    //for most passes
#include "llvm/Support/TargetRegistry.h"
#include "llvm/Target/TargetMachine.h"
#include "llvm/Linker/Linker.h"
#include "llvm/ExecutionEngine/SectionMemoryManager.h"
#include "llvm/ExecutionEngine/GenericValue.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "parser.h"
#include "compiler.h"
#include "types.h"
#include "repl.h"
#include "target.h"
#include "yyparser.h"

using namespace std;
using namespace llvm;
using namespace ante::parser;

namespace ante {

//Global containing every module/file compiled
//to avoid recompilation
llvm::StringMap<unique_ptr<Module>> allCompiledModules;

//each mergedCompUnits is static in lifetime
vector<unique_ptr<Module>> allMergedCompUnits;

//yy::locations stored in all Nodes contain a string* to
//a filename which must not be freed until all nodes are
//deleted, including the FuncDeclNodes within ante::Modules
//that all have a static lifetime
vector<unique_ptr<string>> fileNames;

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
    for(Node *n : *nList){
        ret = n->compile(c);
    }
    return ret;
}


/**
 * @return True if the TypeTag is an unsigned integer type
 */
bool isUnsignedTypeTag(const TypeTag tt){
    return tt==TT_U8||tt==TT_U16||tt==TT_U32||tt==TT_U64||tt==TT_Usz;
}


TypedValue IntLitNode::compile(Compiler *c){
    return TypedValue(ConstantInt::get(*c->ctxt,
                            APInt(getBitWidthOfTypeTag(type),
                            atol(val.c_str()), isUnsignedTypeTag(type))),
            AnType::getPrimitive(type));
}


const fltSemantics& typeTagToFltSemantics(TypeTag tokTy){
    switch(tokTy){
        case TT_F16: return APFloat::IEEEhalf();
        case TT_F32: return APFloat::IEEEsingle();
        case TT_F64: return APFloat::IEEEdouble();
        default:     return APFloat::IEEEdouble();
    }
}

TypedValue FltLitNode::compile(Compiler *c){
    return TypedValue(ConstantFP::get(*c->ctxt, APFloat(typeTagToFltSemantics(type), val.c_str())),
            AnType::getPrimitive(type));
}


TypedValue BoolLitNode::compile(Compiler *c){
    return TypedValue(ConstantInt::get(*c->ctxt, APInt(1, (bool)val, true)),
            AnType::getBool());
}


/**
 * @brief this is a stub.  ModNodes should be handled manually in DeclNode::compile methods
 */
TypedValue ModNode::compile(Compiler *c){
    return {};
}


/**
 * @brief Compiles a TypeNode
 *
 * @return The tag value if this node is a union tag, otherwise it returns
 *         a compile-time value of type Type
 */
TypedValue TypeNode::compile(Compiler *c){
    //check for enum value
    if(type == TT_Data || type == TT_TaggedUnion){
        auto *dataTy = AnDataType::get(typeName);
        if(!dataTy or dataTy->isStub()) goto rettype;

        auto *unionDataTy = dataTy->parentUnionType;
        if(!unionDataTy or dataTy->isStub()) goto rettype;

        size_t tagIndex = unionDataTy->getTagVal(typeName);
        Value *tag = ConstantInt::get(*c->ctxt, APInt(8, tagIndex, true));

        Type *unionTy = c->anTypeToLlvmType(unionDataTy, true);

        Type *curTy = tag->getType();

        //allocate for the largest possible union member
        auto *alloca = c->builder.CreateAlloca(unionTy);

        //but make sure to bitcast it to the current member before storing an incorrect type
        Value *castTo = c->builder.CreateBitCast(alloca, curTy->getPointerTo());
        c->builder.CreateStore(tag, castTo);

        //load the initial alloca, not the bitcasted one
        Value *unionVal = c->builder.CreateLoad(alloca);
        return TypedValue(unionVal, unionDataTy);
    }

rettype:
    //return the type as a value
    auto *ty = toAnType(c, this);

    //The TypeNode* address is wrapped in an llvm int so that llvm::Value methods can be called
    //without crashing, even if their result is meaningless
    Value *v = c->builder.getInt64((unsigned long)ty);
    return TypedValue(v, AnType::getPrimitive(TT_Type));
}


/**
 * @brief Compiles all top-level import expressions
 */
void scanImports(Compiler *c, RootNode *r){
    for(auto &n : r->imports){
        try{
            n->compile(c);
        }catch(CtError *e){
            delete e;
        }
    }
}

/**
 * @brief Compiles a Str literal that contains 1+ sites of string interpolation.
 * Concatenates
 *
 * @param sln The string literal to compile
 * @param pos The index of the first instance of ${ in the string
 *
 * @return The resulting concatenated Str
 */
TypedValue compStrInterpolation(Compiler *c, StrLitNode *sln, int pos){
    //get the left part of the string
    string l = sln->val.substr(0, pos);

    //make a new sub-location for it
    yy::location lloc = mkLoc(mkPos(sln->loc.begin.filename, sln->loc.begin.line, sln->loc.begin.column),
		                mkPos(sln->loc.end.filename,   sln->loc.end.line,   sln->loc.begin.column + pos-1));
    auto *ls = new StrLitNode(lloc, l);


    auto posEnd = sln->val.find("}", pos);
    if(posEnd == string::npos)
        return c->compErr("Interpolated string must have a closing bracket", sln->loc);

    //this is the ${...} part of the string without the ${ and }
    string m = sln->val.substr(pos+2, posEnd - (pos+2));

    string r = sln->val.substr(posEnd+1);
    yy::location rloc = mkLoc(mkPos(sln->loc.begin.filename, sln->loc.begin.line, sln->loc.begin.column + posEnd + 1),
							  mkPos(sln->loc.end.filename,   sln->loc.end.line,   sln->loc.end.column));
    auto *rs = new StrLitNode(rloc, r);

    //now that the string is separated, begin interpolation preparation

    //lex and parse
    auto *lex = new Lexer(sln->loc.begin.filename, m,
            sln->loc.begin.line-1, sln->loc.begin.column + pos);
    setLexer(lex);
    yy::parser p{};
    int flag = p.parse();
    if(flag != PE_OK){ //parsing error, cannot procede
        fputs("Syntax error in string interpolation, aborting.\n", stderr);
        exit(flag);
    }

    RootNode *expr = parser::getRootNode();
    TypedValue val;
    Node *valNode = 0;

    scanImports(c, expr);
    c->scanAllDecls(expr);

    //Compile main and hold onto the last value
    for(auto &n : expr->main){
        try{
            val = n->compile(c);
            valNode = n.get();
        }catch(CtError *e){
            delete e;
        }
    }

    if(!val) return val;

    //if the expr is not already a string type, cast it to one
    auto *strty = dyn_cast<AnDataType>(val.type);
    if(!strty or strty->name != "Str"){
		strty = AnDataType::get("Str");
        auto fn = c->getCastFn(val.type, strty);

        if(!fn){
            delete ls;
            delete rs;
            return c->compErr("Cannot cast " + anTypeToColoredStr(val.type)
                + " to Str for string interpolation.", valNode->loc);
        }

        val = TypedValue(c->builder.CreateCall(fn.val, val.val), strty);
    }

    //Finally, the interpolation is done.  Now just combine the three strings
    //get the ++_Str_Str function
    string appendFn = "++";
    string mangledAppendFn = "++_Str_Str";
    auto fn = c->getFunction(appendFn, mangledAppendFn);
    if(!fn) return c->compErr("++ overload for Str and Str not found while performing Str interpolation.  The prelude may not be imported correctly.", sln->loc);

    //call the ++ function to combine the three strings
    auto lstr = ls->compile(c);
    auto *appendL = c->builder.CreateCall(fn.val, vector<Value*>{lstr.val, val.val});

    auto rstr = rs->compile(c);
    auto *appendR = c->builder.CreateCall(fn.val, vector<Value*>{appendL, rstr.val});

    //create the returning typenode
    return TypedValue(appendR, strty);
}


TypedValue StrLitNode::compile(Compiler *c){
    auto idx = val.find("${");

    if(idx != string::npos and (idx == 0 or val.find("\\${") != idx - 1))
        return compStrInterpolation(c, this, idx);

    AnType *strty = AnDataType::get("Str");

    auto *ptr = c->builder.CreateGlobalStringPtr(val, "_strlit");

	//get the llvm Str data type from a fake type node in case we are compiling
	//the prelude and the Str data type isnt translated into an llvmty yet
    auto *tupleTy = cast<StructType>(c->anTypeToLlvmType(strty));

	vector<Constant*> strarr = {
		UndefValue::get(Type::getInt8PtrTy(*c->ctxt)),
		ConstantInt::get(*c->ctxt, APInt(AN_USZ_SIZE, val.length(), true))
	};

    auto *uninitStr = ConstantStruct::get(tupleTy, strarr);
    auto *str = c->builder.CreateInsertValue(uninitStr, ptr, 0);

    return TypedValue(str, strty);
}

TypedValue CharLitNode::compile(Compiler *c){
    return TypedValue(ConstantInt::get(*c->ctxt, APInt(8, val, true)), AnType::getPrimitive(TT_C8));
}


TypedValue ArrayNode::compile(Compiler *c){
    vector<Constant*> arr;
    AnType *elemTy = exprs.empty() ? AnType::getVoid() : nullptr;

    int i = 1;
    for(auto& n : exprs){
        auto tval = n->compile(c);

        arr.push_back((Constant*)tval.val);

        if(!elemTy){
            elemTy = tval.type;
        }else{
            if(!c->typeEq(tval.type, elemTy))
                return c->compErr("Element " + to_string(i) + "'s type " + anTypeToColoredStr(tval.type) +
                        " does not match the first element's type of " + anTypeToColoredStr(elemTy), n->loc);
        }
        i++;
    }

    auto *ty = ArrayType::get(arr[0]->getType(), exprs.size());
    auto *val = ConstantArray::get(ty, arr);
    return TypedValue(val, AnArrayType::get(elemTy, exprs.size()));
}

/**
 * @brief Creates and returns a literal of type void
 *
 * @return A void literal
 */
TypedValue Compiler::getVoidLiteral(){
    return TypedValue(UndefValue::get(Type::getInt8Ty(*ctxt)), AnType::getVoid());
}

TypedValue TupleNode::compile(Compiler *c){
    //A void value is represented by the empty tuple, ()
    if(exprs.empty())
        return c->getVoidLiteral();

    vector<Constant*> elems;
    elems.reserve(exprs.size());

    vector<Type*> elemTys;
    elemTys.reserve(exprs.size());

    vector<AnType*> anElemTys;
    anElemTys.reserve(exprs.size());

    map<unsigned, Value*> pathogenVals;

    //Compile every value in the tuple, and if it is not constant,
    //add it to pathogenVals
    for(unsigned i = 0; i < exprs.size(); i++){
        auto tval = exprs[i]->compile(c);

        if(Constant *elem = dyn_cast<Constant>(tval.val)){
            elems.push_back(elem);
        }else{
            pathogenVals[i] = tval.val;
            elems.push_back(UndefValue::get(tval.getType()));
        }
        elemTys.push_back(tval.getType());
        anElemTys.push_back(tval.type);
    }

    //Create the constant tuple with undef values in place for the non-constant values
    Value* tuple = ConstantStruct::get(StructType::get(*c->ctxt, elemTys), elems);

    //Insert each pathogen value into the tuple individually
    for(const auto &p : pathogenVals){
        tuple = c->builder.CreateInsertValue(tuple, p.second, p.first);
    }

    auto *tupTy = AnAggregateType::get(TT_Tuple, anElemTys);
    return TypedValue(tuple, tupTy);
}


/**
 * @brief Compiles a tuple's elements and returns them in a vector
 *
 * @return A vector of a tuple's elements
 */
vector<TypedValue> TupleNode::unpack(Compiler *c){
    vector<TypedValue> ret;
    for(auto& n : exprs){
        auto tv = n->compile(c);

        if(!!tv && tv.type->typeTag != TT_Void)
            ret.push_back(tv);
    }
    return ret;
}


/*
 *  When a retnode is compiled within a block, care must be taken to not
 *  forcibly insert the branch instruction afterwards as it leads to dead code.
 */
TypedValue RetNode::compile(Compiler *c){
    TypedValue ret = expr->compile(c);

    auto retInst = ret.type->typeTag == TT_Void ?
                 TypedValue(c->builder.CreateRetVoid(), ret.type) :
                 TypedValue(c->builder.CreateRet(ret.val), ret.type);

    auto *f = c->getCurrentFunction();
    f->returns.push_back({retInst, expr->loc});
    return retInst;
}


/*
 * TODO: implement for abitrary compile-time Str expressions
 */
TypedValue ImportNode::compile(Compiler *c){
    if(!dynamic_cast<StrLitNode*>(expr.get())) return {};

    c->importFile(((StrLitNode*)expr.get())->val.c_str(), this);
    return c->getVoidLiteral();
}


TypedValue WhileNode::compile(Compiler *c){
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

    TypedValue val;
    try{
        auto condval = condition->compile(c);

        c->builder.CreateCondBr(condval.val, begin, end);
        c->builder.SetInsertPoint(begin);

        val = child->compile(c);
    }catch(CtError *e){
        c->compCtxt->breakLabels->pop_back();
        c->compCtxt->continueLabels->pop_back();
        throw e;
    }

    c->compCtxt->breakLabels->pop_back();
    c->compCtxt->continueLabels->pop_back();

    if(!dyn_cast<ReturnInst>(val.val) and !dyn_cast<BranchInst>(val.val))
        c->builder.CreateBr(cond);

    c->builder.SetInsertPoint(end);
    return c->getVoidLiteral();
}


TypedValue ForNode::compile(Compiler *c){
    Function *f = c->builder.GetInsertBlock()->getParent();
    BasicBlock *cond  = BasicBlock::Create(*c->ctxt, "for_cond", f);
    BasicBlock *begin = BasicBlock::Create(*c->ctxt, "for", f);
    BasicBlock *incr = BasicBlock::Create(*c->ctxt, "for_incr", f);
    BasicBlock *end   = BasicBlock::Create(*c->ctxt, "end_for", f);


    auto rangev = range->compile(c);

    //check if the range expression is its own iterator and thus implements Iterator
    //If it does not, see if it implements Iterable by attempting to call into_iter on it
    auto *dt = dyn_cast<AnDataType>(rangev.type);
    if(!dt or !c->typeImplementsTrait(dt, "Iterator")){
        auto res = c->callFn("into_iter", {rangev});

        if(!res)
            return c->compErr("Range expression of type " + anTypeToColoredStr(rangev.type) + " needs to implement " +
                anTypeToColoredStr(AnDataType::get("Iterable")) + " or " + anTypeToColoredStr(AnDataType::get("Iterator")) +
                " to be used in a for loop", range->loc);

        rangev = res;
    }

    //by this point, rangev now properly stores the range information, so store it on the stack and insert calls to
    //unwrap, has_next, and next at the beginning, beginning, and end of the loop respectively.
    Value *alloca = c->builder.CreateAlloca(rangev.getType());
    c->builder.CreateStore(rangev.val, alloca);

    c->builder.CreateBr(cond);
    c->builder.SetInsertPoint(cond);

    //set var = unwrap range

    //candval = is_done range
    auto rangeVal = TypedValue(c->builder.CreateLoad(alloca), rangev.type);

    auto is_done = c->callFn("has_next", {rangeVal});
    if(!is_done) return c->compErr("Range expression of type " + anTypeToColoredStr(rangev.type) + " does not implement " +
            anTypeToColoredStr(AnDataType::get("Iterable")) + ", which it needs to be used in a for loop", range->loc);

    c->builder.CreateCondBr(is_done.val, begin, end);
    c->builder.SetInsertPoint(begin);

    //call unwrap at start of loop
    //make sure to update rangeVal
    rangeVal = TypedValue(c->builder.CreateLoad(alloca), rangev.type);
    auto uwrap = c->callFn("unwrap", {rangeVal});
    if(!uwrap) return c->compErr("Range expression of type " + anTypeToColoredStr(rangev.type) + " does not implement " +
            anTypeToColoredStr(AnDataType::get("Iterable")) + ", which it needs to be used in a for loop", range->loc);

    auto *uwrap_var = new Variable(var, uwrap, c->scope);
    c->stoVar(var, uwrap_var);


    //register the branches to break/continue to right before the body is compiled in case there was an error compiling the range
    c->compCtxt->breakLabels->push_back(end);
    c->compCtxt->continueLabels->push_back(incr);

    //compile the rest of the loop's body
    TypedValue val;
    try{
        val = child->compile(c);
    }catch(CtError *e){
        c->compCtxt->breakLabels->pop_back();;
        c->compCtxt->continueLabels->pop_back();
        throw e;
    }

    c->compCtxt->breakLabels->pop_back();;
    c->compCtxt->continueLabels->pop_back();

    if(!val) return val;
    if(!dyn_cast<ReturnInst>(val.val) and !dyn_cast<BranchInst>(val.val)){
        //set range = next range
        c->builder.CreateBr(incr);
        c->builder.SetInsertPoint(incr);

        TypedValue arg = {c->builder.CreateLoad(alloca), rangev.type};
        auto next = c->callFn("next", {arg});
        if(!next) return c->compErr("Range expression of type " + anTypeToColoredStr(rangev.type) + " does not implement " + anTypeToColoredStr(AnDataType::get("Iterable")) +
                ", which it needs to be used in a for loop", range->loc);

        c->builder.CreateStore(next.val, alloca);
        c->builder.CreateBr(cond);
    }

    c->builder.SetInsertPoint(end);
    return c->getVoidLiteral();
}


TypedValue JumpNode::compile(Compiler *c){
    auto e = expr->compile(c);
    auto *ci = dyn_cast<ConstantInt>(e.val);
    if(!ci)
        return c->compErr("Expression must evaluate to a constant integer\n", expr->loc);

    if(!isUnsignedTypeTag(e.type->typeTag) and ci->getSExtValue() < 0)
        return c->compErr("Cannot jump out of a negative number (" + to_string(ci->getSExtValue()) +  ") of loops", expr->loc);

    //we can now safely get the zero-extended value of ci since even if it is signed, it is not negative
    auto jumpCount = ci->getZExtValue();

    //NOTE: continueLabels->size() == breakLabels->size() always
    auto loopCount = c->compCtxt->breakLabels->size();

    if(loopCount == 0)
        return c->compErr("There are no loops to jump out of", this->loc);


    if(jumpCount == 0)
        return c->compErr("Cannot jump out of 0 loops", expr->loc);


    if(jumpCount > loopCount)
        return c->compErr("Cannot jump out of " + to_string(jumpCount) + " loops when there are only " +
                to_string(c->compCtxt->breakLabels->size()) + " loop(s) nested", expr->loc);

    //actually create the branch instruction
    BranchInst *br = jumpType == Tok_Continue ?
        c->builder.CreateBr( c->compCtxt->continueLabels->at(loopCount - jumpCount) ) :
        c->builder.CreateBr( c->compCtxt->breakLabels->at(loopCount - jumpCount) );

    //Although returning a void, use the br as the value so loops know the last instruction was a br and not to insert another
    return TypedValue(br, AnType::getVoid());
}


//create a new scope if the user indents
TypedValue BlockNode::compile(Compiler *c){
    c->enterNewScope();
    TypedValue ret = block->compile(c);
    c->exitScope();
    return ret;
}


/**
 *  @brief This is a stub.  Compilation of parameters is handled within Compiler::compFn
 */
TypedValue NamedValNode::compile(Compiler *c)
{ return {}; }


/**
 * @brief Performs a lookup for an identifier and returns its value if found
 *
 * @return The value of the variable
 */
TypedValue VarNode::compile(Compiler *c){
    auto *var = c->lookup(name);

    if(var){
        if(var->autoDeref){
            auto *load = c->builder.CreateLoad(var->getVal(), name);
            return TypedValue(load, var->tval.type);
        }else{
            return TypedValue(var->tval.val, var->tval.type);
        }
    }else{
        //if this is a function, then there must be only one function of the same name, otherwise the reference is ambiguous
        auto& fnlist = c->getFunctionList(name);

        if(fnlist.size() == 1){
            auto& fd = *fnlist.begin();
            if(!fd->tv)
                fd->tv = c->compFn(fd.get());

            if(!fd or !fd->tv)
                return {};

            return TypedValue(fd->tv.val, fd->tv.type);
        }else if(fnlist.empty()){
            return c->compErr("Variable or function '" + name + "' has not been declared.", this->loc);
        }else{
            return FunctionCandidates::getAsTypedValue(c->ctxt.get(), fnlist, {});
        }
    }
}


TypedValue LetBindingNode::compile(Compiler *c){
    TypedValue val = expr->compile(c);
    if(val.type->typeTag == TT_Void)
        return c->compErr("Cannot assign a "+anTypeToColoredStr(AnType::getVoid())+
                " value to a variable", expr->loc);

    TypeNode *tyNode;
    if((tyNode = (TypeNode*)typeExpr.get())){
        auto *anty = toAnType(c, tyNode);
        if(!llvmTypeEq(val.val->getType(), c->anTypeToLlvmType(anty))){
            return c->compErr("Incompatible types in explicit binding.", expr->loc);
        }
    }

    bool isGlobal = false;

    //add the modifiers to the typedvalue
    for(Node *n : *modifiers){
        int m = ((ModNode*)n)->mod;
        val.type = val.type->addModifier((TokenType)m);
        if(m == Tok_Global) isGlobal = true;
    }

    if(isGlobal){
        auto *ty = c->anTypeToLlvmType(val.type);
        auto *global = new GlobalVariable(*c->module, ty, false, GlobalValue::PrivateLinkage, UndefValue::get(ty), this->name);
        c->builder.CreateStore(val.val, global);
        val.val = global;
    }

    if(val.getType()->isArrayTy() and not isGlobal){
        Value *alloca = c->builder.CreateAlloca(val.getType(), nullptr, name.c_str());
        c->builder.CreateStore(val.val, alloca);
        val.val = alloca;
        isGlobal = true;
    }

    c->stoVar(name, new Variable(name, val, c->scope, true, isGlobal));
    return val;
}

/**
 * @brief Helper function to compile a VarDeclNode with no specified type.
 *        Matches the type of the variable with the init expression's type.
 *
 * @param node The declaration expression
 *
 * @return The newly-declared variable with an inferred type
 */
TypedValue compVarDeclWithInferredType(VarDeclNode *node, Compiler *c){
    TypedValue val = node->expr->compile(c);
    if(val.type->typeTag == TT_Void)
        return c->compErr("Cannot assign a "+anTypeToColoredStr(AnType::getVoid())+
                " value to a variable", node->expr->loc);

    bool isGlobal = false;

    //Add all of the declared modifiers to the typedval
    for(Node *n : *node->modifiers){
        int m = ((ModNode*)n)->mod;
        val.type = val.type->addModifier((TokenType)m);
        if(m == Tok_Global) isGlobal = true;
    }

    //set the value as mutable
    if(!val.type->hasModifier(Tok_Mut)){
        val.type = val.type->addModifier(Tok_Mut);
    }

    //location to store var
    Value *ptr = isGlobal ?
            (Value*) new GlobalVariable(*c->module, val.getType(), false, GlobalValue::PrivateLinkage, UndefValue::get(val.getType()), node->name) :
            c->builder.CreateAlloca(val.getType(), nullptr, node->name.c_str());

    TypedValue alloca = TypedValue(ptr, val.type);

    bool nofree = true;//val->type->type != TT_Ptr || dynamic_cast<Constant*>(val->val);
    c->stoVar(node->name, new Variable(node->name, alloca, c->scope, nofree, true));

    return TypedValue(c->builder.CreateStore(val.val, alloca.val), val.type);
}

TypedValue VarDeclNode::compile(Compiler *c){
    //check for redeclaration, but only on topmost scope
    auto redeclare = c->varTable.back()->find(this->name);
    if(redeclare != c->varTable.back()->end()){
        return c->compErr("Variable " + name + " was redeclared.", this->loc);
    }

    //check for an inferred type
    if(!typeExpr.get())
        return compVarDeclWithInferredType(this, c);

    if(((TypeNode*)typeExpr.get())->type == TT_Void)
        return c->compErr("Cannot create a variable of type "+
                anTypeToColoredStr(AnType::getVoid()), typeExpr->loc);


    //the type held by this node will be deleted when the parse tree is, so copy
    //this one so it is not double freed
    AnType *anTy = toAnType(c, (TypeNode*)typeExpr.get());

    Type *ty = c->anTypeToLlvmType(anTy);

    bool isGlobal = false;

    //Add all of the declared modifiers to the typedval
    for(Node *n : *modifiers){
        int m = ((ModNode*)n)->mod;
        anTy = anTy->addModifier((TokenType)m);
        if(m == Tok_Global) isGlobal = true;
    }

    if(!anTy->hasModifier(Tok_Mut))
        anTy = anTy->addModifier(Tok_Mut);

    //location to store var
    Value *loc = isGlobal ?
        (Value*) new GlobalVariable(*c->module, ty, false, GlobalValue::PrivateLinkage, UndefValue::get(ty), name) :
        c->builder.CreateAlloca(ty, nullptr, name.c_str());

    TypedValue alloca = TypedValue(loc, anTy);

    Variable *var = new Variable(name, alloca, c->scope, true, true);
    c->stoVar(name, var);
    if(expr.get()){
        TypedValue val = expr->compile(c);
        if(val.type->typeTag == TT_Void)
            return c->compErr("Cannot assign a "+anTypeToColoredStr(AnType::getVoid())+
                    " value to a variable", expr->loc);

        AnType *exprTy = val.type->addModifier(Tok_Mut);
        var->noFree = true;//var->getType() != TT_Ptr || dynamic_cast<Constant*>(val->val);

        //Make sure the assigned value matches the variable's type
        auto *allocaTy = (AnPtrType*)alloca.type;
        if(!c->typeEq(allocaTy->extTy, exprTy)){
            return c->compErr("Cannot assign expression of type " + anTypeToColoredStr(val.type)
                        + " to a variable of type " + anTypeToColoredStr(allocaTy->extTy), expr->loc);
        }

        //transfer ownership of val->type
        return TypedValue(c->builder.CreateStore(val.val, alloca.val), exprTy);
    }else{
        return alloca;
    }
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
        return c->compErr("Cannot insert value into static module '" +
                anTypeToColoredStr(toAnType(c, tn)), tn->loc);


    Value *val;
    AnType *tyn;
    AnType *ltyn;

    //prevent l from being used after this scope; only val and tyn should be used as only they
    //are updated with the automatic pointer dereferences.
    {
        auto l = bop->lval->compile(c);

        val = l.val;
        tyn = ltyn = l.type;

        if(!tyn->hasModifier(Tok_Mut))
            return c->compErr("Variable must be mutable to be assigned to, but instead is an immutable " +
                    anTypeToColoredStr(tyn), bop->loc);
    }

    //the . operator automatically dereferences pointers, so update val and tyn accordingly.
    while(auto *ptr = dyn_cast<AnPtrType>(tyn)){
        val = c->builder.CreateLoad(val);
        tyn = ptr->extTy;
    }

    //this is the variable that will store the changes after the later insertion
    Value *var = static_cast<LoadInst*>(val)->getPointerOperand();

    //check to see if this is a field index
    if(auto dataTy = dyn_cast<AnDataType>(tyn)){
        auto index = dataTy->getFieldIndex(field->name);

        if(index != -1){
            AnType *indexTy = dataTy->extTys[index];

            auto newval = expr->compile(c);

            //see if insert operator # = is overloaded already
            string op = "#";
            string mangledfn = mangle(op, {tyn, AnType::getI32(), newval.type});
            auto fn = c->getFunction(op, mangledfn);
            if(!!fn)
                return TypedValue(c->builder.CreateCall(fn.val, vector<Value*>{
                            var, c->builder.getInt32(index), newval.val}),
                        fn.type->getFunctionReturnType());

            //if not, proceed with normal operations
            if(!c->typeEq(indexTy, newval.type))
                return c->compErr("Cannot assign expression of type " + anTypeToColoredStr(newval.type) +
                        " to a variable of type " + anTypeToColoredStr(indexTy), expr->loc);

            Value *nv = newval.val;
            Type *nt = val->getType()->getStructElementType(index);

            //Type check may succeed if a void* is being inserted into any ptr slot,
            //but llvm will still complain so we create a bit cast to appease it
            if(nv->getType() != nt and newval.type->typeTag == TT_Ptr) {
                nv = c->builder.CreateBitCast(nv, nt);
            }

            auto *ins = c->builder.CreateInsertValue(val, nv, index);

            c->builder.CreateStore(ins, var);
            return c->getVoidLiteral();
        }
    }

    return c->compErr("Method/Field " + field->name + " not found in type " + anTypeToColoredStr(tyn), bop->loc);
}

/**
 * @brief Compiles an assign expression of an already-declared variable
 *
 * @return A void literal
 */
TypedValue VarAssignNode::compile(Compiler *c){
    //If this is an insert value (where the lval resembles var[index] = ...)
    //then this must be instead compiled with compInsert, otherwise the [ operator
    //would retrieve the value at the index instead of the reference for storage.
    if(BinOpNode *bop = dynamic_cast<BinOpNode*>(ref_expr)){
        if(bop->op == '#')
            return c->compInsert(bop, expr.get());
        else if(bop->op == '.')
            return compFieldInsert(c, bop, expr.get());
    }

    //otherwise, this is just a normal assign to a variable
    TypedValue tmp = ref_expr->compile(c);

    //if(!dynamic_cast<LoadInst*>(tmp->val))
    if(!tmp.type->hasModifier(Tok_Mut))
        return c->compErr("Variable must be mutable to be assigned to, but instead is an immutable " +
                anTypeToColoredStr(tmp.type), ref_expr->loc);

    Value *dest = ((LoadInst*)tmp.val)->getPointerOperand();

    //compile the expression to store
    TypedValue assignExpr = expr->compile(c);

    //lvalue must compile to a pointer for storage, usually an alloca value
    if(!PointerType::isLoadableOrStorableType(tmp.getType())){
        return c->compErr("Attempted assign without a memory address, with type "
                + anTypeToColoredStr(tmp.type), ref_expr->loc);
    }

    //and finally, make sure the assigned value matches the variable's type
    if(!c->typeEq(tmp.type, assignExpr.type)){
        return c->compErr("Cannot assign expression of type " + anTypeToColoredStr(assignExpr.type)
                    + " to a variable of type " + anTypeToColoredStr(tmp.type), expr->loc);
    }

    //now actually create the store
    c->builder.CreateStore(assignExpr.val, dest);

    //all assignments return a void value
    return c->getVoidLiteral();
}


/**
 * @brief Mangles a function name
 *
 * @param base The unmangled function name
 * @param params The type of each parameter of the function
 *
 * @return The mangled version of the function name
 */
string mangle(string &base, vector<AnType*> params){
    string name = base;
    for(auto *tv : params){
        if(tv->typeTag != TT_Void)
            name += "_" + anTypeToStrWithoutModifiers(tv);
    }
    return name;
}

string mangle(string &base, shared_ptr<NamedValNode> &paramTys){
    string name = base;
    NamedValNode *cur = paramTys.get();
    while(cur){
        auto *tn = (TypeNode*)cur->typeExpr.get();

        if(!tn)
            name += "...";
        else if(tn == (void*)1)
            name += AN_MANGLED_SELF;
        else if(tn->type != TT_Void)
            name += "_" + typeNodeToStr(tn);

        cur = (NamedValNode*)cur->next.get();
    }
    return name;
}

string mangle(string &base, TypeNode *paramTys){
    string name = base;
    while(paramTys){
        if(paramTys->type != TT_Void)
            name += "_" + typeNodeToStr(paramTys);
        paramTys = (TypeNode*)paramTys->next.get();
    }
    return name;
}

string mangle(string &base, TypeNode *p1, TypeNode *p2){
    string name = base;
    string param1 = "_" + typeNodeToStr(p1);
    string param2 = "_" + typeNodeToStr(p2);
    name += param1 + param2;
    return name;
}

string mangle(string &base, TypeNode *p1, TypeNode *p2, TypeNode *p3){
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
FuncDeclNode* findFDN(Node *list, string& basename){
    for(Node *n : *list){
        auto *fdn = (FuncDeclNode*)n;

        if(fdn->name == basename){
            return fdn;
        }
    }
    return nullptr;
}


string manageSelfParam(Compiler *c, FuncDeclNode *fdn, string &mangledName){
    auto self_loc = mangledName.find(AN_MANGLED_SELF);
    if(self_loc != string::npos){
        if(!c->compCtxt->objTn)
            c->compErr("Function must be a method to have a self parameter", fdn->params->loc);

        mangledName.replace(self_loc, strlen(AN_MANGLED_SELF), "_" + typeNodeToStr(c->compCtxt->objTn));
        fdn->params->typeExpr.release();
        fdn->params->typeExpr.reset(c->compCtxt->objTn);
    }
    return mangledName;
}


TypedValue ExtNode::compile(Compiler *c){
    if(traits.get()){
        //this ExtNode is an implementation of a trait
        string typestr = typeNodeToStr(typeExpr.get());
        AnDataType *dt;

        if(typeExpr->typeName.empty()){ //primitive type being extended
            dt = AnDataType::get(typestr);
            if(!dt or dt->isStub()){ //if primitive type has not been extended before, make it a DataType to store in
                dt = AnDataType::create(typestr, {toAnType(c, typeExpr.get())}, false, {});
                c->stoType(dt, typestr);
            }
        }else{
            dt = AnDataType::get(typestr);
            if(!dt or dt->isStub())
                return c->compErr("Cannot implement traits for undeclared type " +
                        typeNodeToColoredStr(typeExpr.get()), typeExpr->loc);
        }

        //create a vector of the traits that must be implemented
        TypeNode *curTrait = this->traits.get();
        vector<Trait*> traits;
        while(curTrait){
            string traitstr = typeNodeToStr(curTrait);
            auto *trait = c->lookupTrait(traitstr);
            if(!trait)
                return c->compErr("Trait " + typeNodeToColoredStr(curTrait)
                        + " is undeclared", curTrait->loc);

            traits.push_back(trait);
            curTrait = (TypeNode*)curTrait->next.get();
        }

        //go through each trait and compile the methods for it
        auto *funcs = methods.release();
        for(auto& trait : traits){
            auto *traitImpl = new Trait();
            traitImpl->name = trait->name;

            for(auto& fd_proto : trait->funcs){
                auto *fdn = findFDN(funcs, fd_proto->getName());

                if(!fdn)
                    return c->compErr(typeNodeToColoredStr(typeExpr.get()) + " must implement " + fd_proto->getName() +
                            " to implement " + anTypeToColoredStr(AnDataType::get(trait->name)), fd_proto->fdn->loc);

                string mangledName = c->funcPrefix + mangle(fdn->name, fdn->params);
                fdn->name = c->funcPrefix + fdn->name;

                //If there is a self param it would be mangled incorrectly above as mangle does not have
                //access to what type 'self' references, so fix that here.
                auto *oldTn = c->compCtxt->objTn;
                c->compCtxt->objTn = typeExpr.get();
                mangledName = manageSelfParam(c, fdn, mangledName);
                c->compCtxt->objTn = oldTn;

                shared_ptr<FuncDeclNode> spfdn{fdn};
                shared_ptr<FuncDecl> fd{new FuncDecl(spfdn, mangledName, c->scope, c->mergedCompUnits)};
                traitImpl->funcs.emplace_back(fd);

                c->compUnit->fnDecls[fdn->name].emplace_back(fd);
                c->mergedCompUnits->fnDecls[fdn->name].emplace_back(fd);
            }

            //trait is fully implemented, add it to the DataType
            dt->traitImpls.emplace_back(traitImpl);
        }
    }else{
        //this ExtNode is not a trait implementation, so just compile all functions normally
        string oldPrefix = c->funcPrefix;

        //Temporarily move away any type params so we get Vec.remove not Vec<'t>.remove as the fn name
        auto params = move(typeExpr->params);
        c->funcPrefix = typeNodeToStr(typeExpr.get()) + "_";
        typeExpr->params = move(params);

        auto prevObj = c->compCtxt->obj;
        auto prevObjTn = c->compCtxt->objTn;

        c->compCtxt->obj = toAnType(c, typeExpr.get());
        c->compCtxt->objTn = typeExpr.get();

        compileStmtList(methods.release(), c);

        c->funcPrefix = oldPrefix;
        c->compCtxt->obj = prevObj;
        c->compCtxt->objTn = prevObjTn;
    }
    return c->getVoidLiteral();
}

/**
 * @return True if a DataType implements the specified trait
 */
bool Compiler::typeImplementsTrait(AnDataType* dt, string traitName) const{
    for(auto& tr : dt->traitImpls)
        if(tr->name == traitName)
            return true;
    return false;
}

vector<AnTypeVarType*> toVec(Compiler *c, const vector<unique_ptr<TypeNode>> &generics){
    vector<AnTypeVarType*> ret;
    ret.reserve(generics.size());
    for(auto &tn : generics){
        ret.push_back((AnTypeVarType*)toAnType(c, tn.get()));
    }
    return ret;
}

void addGenerics(vector<AnTypeVarType*> &dest, vector<AnType*> &src);

/**
 * @brief A helper function to compile tagged union declarations
 *
 * @return A void literal
 */
TypedValue compTaggedUnion(Compiler *c, DataDeclNode *n){
    vector<string> fieldNames;
    fieldNames.reserve(n->fields);

    auto *nvn = (NamedValNode*)n->child.get();

    string &union_name = n->name;

    vector<shared_ptr<UnionTag>> tags;

    vector<AnType*> unionTypes;
    AnDataType *data = AnDataType::create(union_name, {}, true, toVec(c, n->generics));

    while(nvn){
        TypeNode *tyn = (TypeNode*)nvn->typeExpr.get();
        AnType *tagTy = tyn->extTy ? toAnType(c, tyn->extTy.get()) : AnType::getVoid();

        vector<AnType*> exts;
        if(tagTy->typeTag == TT_Tuple){
            exts = ((AnAggregateType*)tagTy)->extTys;
        }else{
            exts.emplace_back(tagTy);
        }

        //Each union member's type is a tuple of the tag (a u8 value), and the user-defined value
        auto *tup = AnAggregateType::get(TT_Tuple, {AnType::getU8(), tagTy});

        //Store the tag as a UnionTag and a AnDataType
        AnDataType *tagdt = AnDataType::create(nvn->name, exts, false, toVec(c, n->generics));
        tagdt->fields.emplace_back(union_name);
        tagdt->parentUnionType = data;
        tagdt->isGeneric = isGeneric(exts);

        //Store tag vals as a UnionTag
        UnionTag *tag = new UnionTag(nvn->name, tagdt, data, tags.size());
        tags.emplace_back(tag);

        unionTypes.push_back(tup);

        validateType(c, tagTy, n);
        c->stoType(tagdt, nvn->name);

        nvn = (NamedValNode*)nvn->next.get();
    }

    data->typeTag = TT_TaggedUnion;
    data->extTys = unionTypes;
    data->fields = fieldNames;

    data->tags = tags;

    for(auto &v : data->variants){
        v->extTys = data->extTys;
        v->isGeneric = data->isGeneric;
        v->typeTag = data->typeTag;
        v->tags = tags;
        v->unboundType = data;
        *v = *(AnDataType*)bindGenericToType(c, v, v->boundGenerics);
        if(v->parentUnionType)
            v->parentUnionType = (AnDataType*)bindGenericToType(c, v->parentUnionType, v->parentUnionType->boundGenerics);
        addGenerics(v->generics, v->extTys);
    }

    c->stoType(data, union_name);
    return c->getVoidLiteral();
}

TypedValue DataDeclNode::compile(Compiler *c){
    //{   //new scope to ensure dt isn't used after this check
    //    auto *dt = AnDataType::get(this->name);
    //    if(dt and !dt->isStub()) return c->compErr("Type " + name + " was redefined", loc);
    //}

    auto *nvn = (NamedValNode*)child.get();
    if(((TypeNode*) nvn->typeExpr.get())->type == TT_TaggedUnion){
        return compTaggedUnion(c, this);
    }

    //Create the DataType as a stub first, have its contents be recursive
    //just to cause an error if something tries to use the stub
    AnDataType *data = AnDataType::create(name, {}, false, toVec(c, generics));

    if(data->llvmType)
        data->llvmType = nullptr;

    c->stoType(data, name);

    vector<string> fieldNames;
    vector<AnType*> fieldTypes;

    fieldNames.reserve(fields);
    fieldTypes.reserve(fields);

    while(nvn){
        TypeNode *tyn = (TypeNode*)nvn->typeExpr.get();
        auto ty = toAnType(c, tyn);

        validateType(c, ty, this);

        fieldTypes.push_back(ty);
        fieldNames.push_back(nvn->name);

        nvn = (NamedValNode*)nvn->next.get();
    }

    data->fields = fieldNames;
    data->extTys = fieldTypes;

    for(auto &v : data->variants){
        v->extTys = data->extTys;
        v->isGeneric = data->isGeneric;
        v->typeTag = data->typeTag;
        v->fields = data->fields;
        v->unboundType = data;
        *v = *(AnDataType*)bindGenericToType(c, v, v->boundGenerics);
        if(v->parentUnionType)
            v->parentUnionType = (AnDataType*)bindGenericToType(c, v->parentUnionType, v->parentUnionType->boundGenerics);
        addGenerics(v->generics, v->extTys);
    }

    //updateLlvmTypeBinding(c, data, true);
    return c->getVoidLiteral();
}

void DataDeclNode::declare(Compiler *c){
    AnDataType::create(name, {}, false, toVec(c, generics));
}


TypedValue TraitNode::compile(Compiler *c){
    auto *trait = new Trait();
    trait->name = name;

    auto *curfn = child.release();
    while(curfn){
        auto *fn = (FuncDeclNode*)curfn;
        string mangledName = c->funcPrefix + mangle(fn->name, fn->params);
        fn->name = c->funcPrefix + fn->name;

        shared_ptr<FuncDeclNode> spfdn{fn};
        shared_ptr<FuncDecl> fd{new FuncDecl(spfdn, mangledName, c->scope, c->mergedCompUnits)};

        //create trait type as a generic void* container
        vector<AnType*> ext;
        ext.push_back(AnPtrType::get(AnType::getVoid()));
        fd->obj = AnDataType::getOrCreate(name, ext, false);

        trait->funcs.push_back(fd);
        curfn = curfn->next.get();
    }

    auto traitPtr = shared_ptr<Trait>(trait);
    c->compUnit->traits[name] = traitPtr;
    c->mergedCompUnits->traits[name] = traitPtr;

    return c->getVoidLiteral();
}


/**
 * @brief Compiles the global expression importing global vars.  This compiles
 *        the statement-like version of a GlobalNode.  The modifier-like version
 *        is handled along with other modifiers during a variable's declaration.
 *
 * @return The value of the last global brought into scope
 */
TypedValue GlobalNode::compile(Compiler *c){
    TypedValue ret;
    for(auto &varName : vars){
        Variable *var;
        for(auto i = c->varTable.size(); i >= 1; --i){
            auto it = c->varTable[i-1]->find(varName->name);
            if(it != c->varTable[i-1]->end()){
                var = it->getValue().get();
            }else{
                var = nullptr;
            }
        }

        if(!var)
            return c->compErr("Variable '" + varName->name + "' has not been declared.", varName->loc);

        if(!var->tval.type->hasModifier(Tok_Global))
            return c->compErr("Variable " + varName->name + " must be global to be imported.", varName->loc);

        var->scope = c->scope;
        c->stoVar(varName->name, new Variable(*var));
        ret = var->tval;
    }

    return TypedValue(c->builder.CreateLoad(ret.val), ret.type);
}


void handleTypeCastPattern(Compiler *c, TypedValue lval, TypeCastNode *tn, AnDataType *tagTy, AnDataType *parentTy){
    //If this is a generic type cast like Some 't, the 't must be bound to a concrete type first

    //This is a pattern of the match _ with expr, so if that is mutable this should be too
    //tagTy = (AnDataType*)tagTy->setModifier(lval.type->mods);
    //AnType *tagtycpy = tagTy/*->extTys[0]*/;

    auto tcr = c->typeEq(parentTy, lval.type);

    if(tcr->res == TypeCheckResult::SuccessWithTypeVars)
        tagTy = (AnDataType*)bindGenericToType(c, tagTy, tcr->bindings);
    else if(tcr->res == TypeCheckResult::Failure)
        c->compErr("Cannot bind pattern of type " + anTypeToColoredStr(parentTy) +
                " to matched value of type " + anTypeToColoredStr(lval.type), tn->rval->loc);

    //cast it from (<tag type>, <largest union member type>) to (<tag type>, <this union member's type>)
    auto *tupTy = StructType::get(*c->ctxt, {Type::getInt8Ty(*c->ctxt), c->anTypeToLlvmType(tagTy)}, true);

    auto alloca = addrOf(c, lval);

    //bit cast the alloca to a pointer to the largest type of the parent union
    //auto *cast = c->builder.CreateBitCast(alloca.val, c->anTypeToLlvmType(parentTy)->getPointerTo());
    auto cast = alloca.val;

    //Cast in the form of: Some n
    if(VarNode *v = dynamic_cast<VarNode*>(tn->rval.get())){
        auto *tup = c->builder.CreateLoad(cast);
        auto extract = TypedValue(c->builder.CreateExtractValue(tup, 1), tagTy->extTys[0]);

        c->stoVar(v->name, new Variable(v->name, extract, c->scope));

    //Destructure multiple: Triple(x, y, z)
    }else if(TupleNode *t = dynamic_cast<TupleNode*>(tn->rval.get())){
        auto *taggedValTy = tupTy->getStructElementType(1);
        if(!tupTy->isStructTy()){
            c->compErr("Cannot match tuple pattern against non-tuple type " + anTypeToColoredStr(tagTy), t->loc);
        }

        if(t->exprs.size() != taggedValTy->getNumContainedTypes()){
            c->compErr("Cannot match a tuple of size " + to_string(t->exprs.size()) +
                   " to a pattern of size " + to_string(taggedValTy->getNumContainedTypes()), t->loc);
        }

        auto *aggTy = (AnAggregateType*)tagTy;
        size_t elementNo = 0;

        for(auto &e : t->exprs){
            VarNode *v;
            if(!(v = dynamic_cast<VarNode*>(e.get()))){
                c->compErr("Unknown pattern, expected identifier", e->loc);
            }

            auto *zero = c->builder.getInt32(0);
            auto *ptr = c->builder.CreateGEP(cast, {zero, c->builder.getInt32(1)});
            ptr = c->builder.CreateGEP(ptr, {zero, c->builder.getInt32(elementNo)});

            AnType *curTy = aggTy->extTys[elementNo];
            auto elem = TypedValue(c->builder.CreateLoad(ptr), curTy);
            c->stoVar(v->name, new Variable(v->name, elem, c->scope));
            elementNo++;
        }

    }else{
        c->compErr("Cannot match unknown pattern", tn->rval->loc);
    }
}


TypedValue MatchNode::compile(Compiler *c){
    auto lval = expr->compile(c);

    if(lval.type->typeTag != TT_TaggedUnion && lval.type->typeTag != TT_Data){
        return c->compErr("Cannot match expression of type " + anTypeToColoredStr(lval.type) +
                ".  Match expressions must be a tagged union type", expr->loc);
    }

    //the tag is always the zero-th index except for in certain optimization cases and if
    //the tagged union has no tagged values and is equivalent to an enum in C-like languages.
    Value *switchVal = llvmTypeToTypeTag(lval.getType()) == TT_Tuple ?
            c->builder.CreateExtractValue(lval.val, 0)
            : lval.val;

    Function *f = c->builder.GetInsertBlock()->getParent();
    auto *matchbb = c->builder.GetInsertBlock();

    auto *end = BasicBlock::Create(*c->ctxt, "end_match");
    auto *match = c->builder.CreateSwitch(switchVal, end, branches.size());
    vector<pair<BasicBlock*,TypedValue>> merges;

    for(auto& mbn : branches){
        ConstantInt *ci = nullptr;
        auto *br = BasicBlock::Create(*c->ctxt, "br", f);
        c->builder.SetInsertPoint(br);
        c->enterNewScope();

        //TypeCast-esque pattern:  Some n
        if(TypeCastNode *tn = dynamic_cast<TypeCastNode*>(mbn->pattern.get())){
            auto *tagTy = AnDataType::get(tn->typeExpr->typeName);
            if(!tagTy or tagTy->isStub())
                return c->compErr("Union tag " + typeNodeToColoredStr(tn->typeExpr.get()) + " was not yet declared.", tn->typeExpr->loc);

            if(!tagTy->isUnionTag())
                return c->compErr(typeNodeToColoredStr(tn->typeExpr.get()) + " must be a union tag to be used in a pattern", tn->typeExpr->loc);

            auto *parentTy = tagTy->parentUnionType;
            ci = ConstantInt::get(*c->ctxt, APInt(8, parentTy->getTagVal(tn->typeExpr->typeName), true));

            tagTy = (AnDataType*)bindGenericToType(c, tagTy, ((AnDataType*)lval.type)->boundGenerics);
            tagTy = tagTy->setModifier(lval.type->mods);
            handleTypeCastPattern(c, lval, tn, tagTy, parentTy);

        //single type pattern:  None
        }else if(TypeNode *tn = dynamic_cast<TypeNode*>(mbn->pattern.get())){
            auto *tagTy = AnDataType::get(tn->typeName);
            if(!tagTy or tagTy->isStub())
                return c->compErr("Union tag " + typeNodeToColoredStr(tn) + " was not yet declared.", tn->loc);

            if(!tagTy->isUnionTag())
                return c->compErr(typeNodeToColoredStr(tn) + " must be a union tag to be used in a pattern", tn->loc);

            auto *parentTy = tagTy->parentUnionType;
            ci = ConstantInt::get(*c->ctxt, APInt(8, parentTy->getTagVal(tn->typeName), true));

        //variable/match-all pattern: _
        }else if(VarNode *vn = dynamic_cast<VarNode*>(mbn->pattern.get())){
            auto tn = TypedValue(lval.val, lval.type);
            match->setDefaultDest(br);
            c->stoVar(vn->name, new Variable(vn->name, tn, c->scope));
        }else{
            return c->compErr("Pattern matching non-tagged union types is not yet implemented", mbn->pattern->loc);
        }

        auto then = mbn->branch->compile(c);
        c->exitScope();

        if(!dyn_cast<ReturnInst>(then.val) and !dyn_cast<BranchInst>(then.val))
            c->builder.CreateBr(end);

        merges.push_back(pair<BasicBlock*,TypedValue>(c->builder.GetInsertBlock(), then));

        if(ci)
            match->addCase(ci, br);
    }

    f->getBasicBlockList().push_back(end);
    c->builder.SetInsertPoint(end);

    //merges can be empty if each branch has an early return
    if(merges.empty() or merges[0].second.type->typeTag == TT_Void)
        return c->getVoidLiteral();

    int i = 1;
    auto *phi = c->builder.CreatePHI(merges[0].second.getType(), branches.size());
    for(auto &pair : merges){

        //add each branch to the phi node if it does not return early
        if(!dyn_cast<ReturnInst>(pair.second.val)){

            //match the types of those branches that will merge
            if(!c->typeEq(pair.second.type, merges[0].second.type))
                return c->compErr("Branch "+to_string(i)+"'s return type " + anTypeToColoredStr(pair.second.type) +
                            " != " + anTypeToColoredStr(merges[0].second.type) + ", the first branch's return type", this->loc);
            else
                phi->addIncoming(pair.second.val, pair.first);
        }
        i++;
    }
    phi->addIncoming(UndefValue::get(merges[0].second.getType()), matchbb);
    return TypedValue(phi, merges[0].second.type);
}


/**
 * @brief This is a stub until patterns are properly implemented
 *
 * @return A void literal
 */
TypedValue MatchBranchNode::compile(Compiler *c){
    return c->getVoidLiteral();
}


/**
 * @brief Merges two modules
 *
 * @param mod module to merge into this
 */
void ante::Module::import(ante::Module *mod){
    for(auto& pair : mod->fnDecls)
        for(auto& fd : pair.second)
            fnDecls[pair.first()].push_back(fd);

    for(auto& pair : mod->userTypes)
        userTypes[pair.first()] = pair.second;

    for(auto& pair : mod->traits)
        traits[pair.first()] = pair.second;
}


void Compiler::importFile(const char *fName, Node *locNode){
    auto it = allCompiledModules.find(fName);

    if(it != allCompiledModules.end()){
        auto *import = it->getValue().get();
        string fmodName = removeFileExt(fName);

        for(auto &mod : imports){
            if(mod->name == fmodName){
                compErr("module " + string(fName) + " has already been imported", locNode->loc, ErrorType::Warning);
                return;
            }
        }

        //module is already compiled; just copy the ptr to imports
        imports.push_back(import);
        mergedCompUnits->import(import);
    }else{
        //module not found; create new Compiler instance to compile it
        auto c = unique_ptr<Compiler>(new Compiler(fName, true, ctxt));
        c->ctxt = ctxt;
        c->compilePrelude();
        c->scanAllDecls();

        if(c->errFlag){
            cout << "Error when importing " << fName << endl;
            errFlag = true;
            return;
        }

        imports.push_back(c->compUnit);
        mergedCompUnits->import(c->compUnit);

        allCompiledModules.try_emplace(fName, c->compUnit);
    }
}


/**
 * @brief Creates and returns an anonymous TypeNode (one with
 *        no location in the source file)
 *
 * @param t Value for the TypeNode's type field
 *
 * @return The newly created TypeNode
 */
TypeNode* mkAnonTypeNode(TypeTag t){
    auto fakeLoc = mkLoc(mkPos(0, 0, 0), mkPos(0, 0, 0));
    return new TypeNode(fakeLoc, t, "", nullptr);
}

/**
 * @brief Creates and returns an anonymous TypeNode
 *
 * @param tt Value for the TypeNode's type field
 * @param ext Value for the TypeNodes's extTy field
 *
 * @return The newly created TypeNode
 */
TypeNode* mkTypeNodeWithExt(TypeTag tt, TypeNode *ext){
    auto *p = mkAnonTypeNode(tt);
    p->extTy.reset(ext);
    return p;
}

/**
 * @brief Creates and returns an anonymous TypeNode of type TT_Data
 *
 * @param tyname The name of the DataType referenced
 *
 * @return The newly created TypeNode
 */
TypeNode* mkDataTypeNode(string tyname){
    auto *d = mkAnonTypeNode(TT_Data);
    d->typeName = tyname;
    return d;
}


void Compiler::compilePrelude(){
    if(fileName != AN_LIB_DIR "prelude.an"){
        importFile(AN_LIB_DIR "prelude.an");
    }
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


template<typename T>
void compileAll(Compiler *c, vector<T> &v){
    for(auto &elem : v){
        try{
            elem->compile(c);
        }catch(CtError *err){
            delete err;
        }
    }
}


void Compiler::scanAllDecls(RootNode *root){
    auto *n = root ? root : ast.get();
	
    for (auto& f : n->types) {
		try {
			f->declare(this);
		}catch (CtError *e) {
			delete e;
		}
	}

    compileAll(this, n->types);
    compileAll(this, n->traits);
    compileAll(this, n->extensions);
	compileAll(this, n->funcs);
}

void Compiler::eval(){
    //setup compiler
    createMainFn();
    compilePrelude();

    startRepl(this);
}

Function* Compiler::createMainFn(){
    Type* argcty = Type::getInt32Ty(*ctxt);
    Type* argvty = Type::getInt8Ty(*ctxt)->getPointerTo()->getPointerTo();

    //get or create the function type for the main method: (i32, c8**)->i32
    FunctionType *ft = FunctionType::get(Type::getInt32Ty(*ctxt), {argcty, argvty}, false);

    //Actually create the function in module m
    string fnName = isLib ? "init_" + removeFileExt(fileName) : "main";
    Function *main = Function::Create(ft, Function::ExternalLinkage, fnName, module.get());

    //Create the entry point for the function
    BasicBlock *bb = BasicBlock::Create(*ctxt, "entry", main);
    builder.SetInsertPoint(bb);

    //create argc and argv global vars
    auto *argc = new GlobalVariable(*module, argcty, false, GlobalValue::PrivateLinkage, builder.getInt32(0), "argc");
    auto *argv = new GlobalVariable(*module, argvty, false, GlobalValue::PrivateLinkage, ConstantPointerNull::get(dyn_cast<PointerType>(argvty)), "argv");

#if LLVM_VERSION_MAJOR < 5
    auto args = main->getArgumentList().begin();
#else
    auto args = main->arg_begin();
#endif
    builder.CreateStore(&*args, argc);
    builder.CreateStore(&*++args, argv);

    auto *global_mod = AnModifier::get({Tok_Global});
    AnType *argcAnty = AnType::getPrimitive(TT_I32, global_mod);
    AnType *argvAnty = AnPtrType::get(AnPtrType::get(AnType::getPrimitive(TT_C8)), global_mod);

    stoVar("argc", new Variable("argc", TypedValue(builder.CreateLoad(argc), argcAnty), 1));
    stoVar("argv", new Variable("argv", TypedValue(builder.CreateLoad(argv), argvAnty), 1));

    //add main to call stack
    auto *main_fn_ty = AnFunctionType::get(AnType::getU8(), {argcAnty, argvAnty});

    auto main_tv = TypedValue(main, main_fn_ty);
    shared_ptr<FuncDeclNode> fakeSp;
    auto *main_var = new FuncDecl(fakeSp, fnName, scope, mergedCompUnits, main_tv);
    compCtxt->callStack.push_back(main_var);
    return main;
}


TypedValue RootNode::compile(Compiler *c){
    scanImports(c, this);
    c->scanAllDecls(this);

    //Compile the rest of the program
    TypedValue ret;
    for(auto &n : main){
        try{
            ret = n->compile(c);
        }catch(CtError *e){
            delete e;
        }
    }

    return !!ret ? ret : c->getVoidLiteral();
}


void Compiler::compile(){
    if(compiled){
        cerr << "Module " << module->getName().str() << " is already compiled, cannot recompile.\n";
        return;
    }

    //create implicit main function and import the prelude
    auto *mainFn = createMainFn();
    compilePrelude();

    ast->compile(this);

    //always return 0
    builder.CreateRet(ConstantInt::get(*ctxt, APInt(32, 0)));
    if(!errFlag)
        passManager->run(*mainFn);

    //flag this module as compiled.
    compiled = true;

    //show other modules this is compiled
    allCompiledModules.try_emplace(fileName, compUnit);

    if(errFlag){
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

    string modName = removeFileExt(fileName);
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
		cout << "\nRegistered targets:" << endl;
		TargetRegistry::printRegisteredTargetsForVersion();
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

    TargetMachine *tm = target->createTargetMachine(triple, cpu, features, op, Reloc::Model::Static,
            CodeModel::Default, CodeGenOpt::Level::Aggressive);

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
    auto *tm = getTargetMachine();

    std::error_code errCode;
    raw_fd_ostream out{outFile, errCode, sys::fs::OpenFlags::F_RW};

	char **err = nullptr;
	char *filename = (char*)outFile.c_str();
	int res = LLVMTargetMachineEmitToFile(
		(LLVMTargetMachineRef)tm,
		(LLVMModuleRef)mod,
		filename,
		(LLVMCodeGenFileType)llvm::TargetMachine::CGFT_ObjectFile, err);

    //legacy::PassManager pm;
    //int res = tm->addPassesToEmitFile(pm, out, llvm::TargetMachine::CGFT_ObjectFile);
    //pm.run(*mod);

	if (out.has_error())
		cerr << "Error when compiling to object: " << errCode << endl;

    delete tm;
    return res;
}


int Compiler::linkObj(string inFiles, string outFile){
    string cmd = AN_LINKER " " + inFiles + " -static -o " + outFile;
    return system(cmd.c_str());
}


void Compiler::emitIR(){
    if(!compiled) compile();
    if(errFlag) puts("Partially compiled module: \n");
    module->print(llvm::errs(), nullptr);
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

    if(type->typeTag == TT_Void)
        puts("void ()");
    else if(type->typeTag == TT_Type)
        cout << anTypeToStr(extractTypeValue(*this)) << endl;
    else if(type->typeTag == TT_FunctionList){
        auto *fl = (FunctionCandidates*)val;
        cout << "(" << fl->candidates.size() << " function" << (fl->candidates.size() == 1 ? ")\n" : "s)\n");

        for(auto &c : fl->candidates){
            cout << endl << c->getName() << " (" << c->mangledName << "): \n";
            if(!!c->tv){
                c->tv.dump();
            }else{
                cout << "(not yet compiled)\n\n";
            }
            cout << "Parse tree:\n";
            c->fdn->print();
            cout << endl;
        }
    }else if(type->typeTag == TT_MetaFunction){
        cout << "(ante function)\n";
    }else{
        val->print(llvm::dbgs(), false);
        llvm::dbgs() << '\n';
    }
}


void Compiler::enterNewScope(){
    scope++;
    auto *vtable = new llvm::StringMap<unique_ptr<Variable>>();
    varTable.emplace_back(vtable);
}


bool Variable::isFreeable() const{
    return !noFree;
}

void Compiler::exitScope(){
    //iterate through all known variables, check for pointers at the end of
    //their lifetime, and insert calls to free for any that are found
    auto vtable = varTable.back().get();

    for(auto &pair : *vtable){
        if(pair.second->isFreeable() && pair.second->scope == this->scope){
            string freeFnName = "free";
            Function* freeFn = (Function*)getFunction(freeFnName, freeFnName).val;

            auto *inst = dyn_cast<AllocaInst>(pair.second->getVal());
            auto *val = inst? builder.CreateLoad(inst) : pair.second->getVal();

            //cast the freed value to i32* as that is what free accepts
            Type *vPtr = freeFn->getFunctionType()->getFunctionParamType(0);
            val = builder.CreatePointerCast(val, vPtr);
            builder.CreateCall(freeFn, val);
        }
    }

    scope--;
    varTable.pop_back();
}


Variable* Compiler::lookup(string var) const{
    for(auto i = varTable.size(); i >= fnScope; --i){
        auto& vt = varTable[i-1];
        auto it = vt->find(var);
        if(it != vt->end())
            return it->getValue().get();
    }
    return nullptr;
}


void Compiler::stoVar(string var, Variable *val){
    (*varTable[val->scope-1])[var].reset(val);
    //varTable[val->scope-1]->emplace(var, val);
}


/*
 * Helper function to create an llvm integer literal
 * with the address of a pointer as its value
 */
Value* mkPtrInt(Compiler *c, void *addr){
    return c->builder.getInt64((unsigned long)addr);
}


void Compiler::stoTypeVar(string &name, AnType *ty){
    Value *addr = builder.getInt64((unsigned long)ty);
    TypedValue tv = TypedValue(addr, AnType::getPrimitive(TT_Type));
    Variable *var = new Variable(name, tv, scope);
    stoVar(name, var);
}


AnDataType* Compiler::lookupType(string tyname) const{
    auto& ut = mergedCompUnits->userTypes;
    auto it = ut.find(tyname);
    if(it != ut.end())
        return it->getValue();
    return nullptr;
}

Trait* Compiler::lookupTrait(string tyname) const{
    auto& ts = mergedCompUnits->traits;
    auto it = ts.find(tyname);
    if(it != ts.end())
        return it->getValue().get();
    return nullptr;
}


inline void Compiler::stoType(AnDataType *dt, string &typeName){
    //shared_ptr<AnDataType> dt{ty};
    compUnit->userTypes[typeName] = dt;
    mergedCompUnits->userTypes[typeName] = dt;
}

/**
 * @brief Creates a pass manager and fills it with passes.
 *
 * @param m Module to create the psas manager for
 * @param optLvl The optimization level in the range 0..3.
 * Determines which passes should be added.
 *
 * @return The newly-created pass manager
 */
legacy::FunctionPassManager* mkPassManager(llvm::Module *m, char optLvl){
    auto *pm = new legacy::FunctionPassManager(m);
    if(optLvl > 0){
        if(optLvl >= 3){
            pm->add(createLoopStrengthReducePass());
            pm->add(createLoopUnrollPass());
            pm->add(createMergedLoadStoreMotionPass());
            pm->add(createMemCpyOptPass());
            pm->add(createSpeculativeExecutionPass());
        }
        pm->add(createDeadStoreEliminationPass());
        pm->add(createDeadCodeEliminationPass());
        pm->add(createCFGSimplificationPass());
        pm->add(createTailCallEliminationPass());
        pm->add(createInstructionSimplifierPass());

#if LLVM_VERSION_MAJOR < 5
        pm->add(createLoadCombinePass());
#endif
        pm->add(createLoopLoadEliminationPass());
        pm->add(createReassociatePass());
        pm->add(createPromoteMemoryToRegisterPass());

        //Instruction Combining Pass seems to break nested for loops
        //pm->add(createInstructionCombiningPass());
    }
    pm->doInitialization();
    return pm;
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
        compUnit(new ante::Module()),
        mergedCompUnits(new ante::Module()),
        compCtxt(new CompilerCtxt()),
        ctCtxt(new CompilerCtCtxt()),
        errFlag(false),
        compiled(false),
        isLib(lib),
        isJIT(false),
        fileName(_fileName? _fileName : "(stdin)"),
        funcPrefix(""),
        scope(0), optLvl(2), fnScope(1){

    //The lexer stores the fileName in the loc field of all Nodes. The fileName is copied
    //to let Node's outlive the Compiler they were made in, ensuring they work with imports.
    if(_fileName){
        string* fileName_cpy = new string(fileName);
        fileNames.emplace_back(fileName_cpy);
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

        ast.reset(parser::getRootNode());
    }

    allMergedCompUnits.emplace_back(mergedCompUnits);

    auto modName = removeFileExt(fileName);
    compUnit->name = modName;
    mergedCompUnits->name = modName;

    outFile = modName;
	if (outFile.empty())
		outFile = "a.out";

    module.reset(new llvm::Module(outFile, *ctxt));

    enterNewScope();

    //add passes to passmanager.
    passManager.reset(mkPassManager(module.get(), optLvl));
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
        compUnit(new ante::Module()),
        mergedCompUnits(new ante::Module()),
        compCtxt(new CompilerCtxt()),
        ctCtxt(c->ctCtxt),
        errFlag(false),
        compiled(false),
        isLib(lib),
        isJIT(false),
        fileName(c->fileName),
        outFile(modName),
        funcPrefix(""),
        scope(0), optLvl(2), fnScope(1){

    allMergedCompUnits.emplace_back(mergedCompUnits);

    compUnit->name = modName;
    mergedCompUnits->name = modName;

    ast.reset(new RootNode(root->loc));
    ast->main.push_back(unique_ptr<Node>(root));

    module.reset(new llvm::Module(outFile, *ctxt));

    enterNewScope();

    //add passes to passmanager.
    passManager.reset(mkPassManager(module.get(), 3));
}


void Compiler::processArgs(CompilerArgs *args){
    string out = "";
    bool shouldGenerateExecutable = true;

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

        passManager.reset(mkPassManager(module.get(), optLvl));
    }


    //make sure even non-called functions are included in the binary
    //if the -lib flag is set
    if(args->hasArg(Args::Lib)){
        isLib = true;
        if(!compiled) compile();

        for(auto& pair : compUnit->fnDecls){
            for(auto& fd : pair.second){
                if(!fd->tv)
                    compFn(fd.get());
            }
        }
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

        if(!errFlag && args->hasArg(Args::CompileAndRun)){
            int res = system((AN_EXEC_STR + outFile).c_str());
            if(res) return; //silence unused return result warning
        }
    }
}

Compiler::~Compiler(){
    exitScope();
    if(yylexer){
        delete yylexer;
        yylexer = 0;
    }

    if(compCtxt and compCtxt->callStack.size() >= 1){
        delete compCtxt->callStack[0];
    }

	//passManager.release();
	//module.release();
}

} //end of namespace ante
