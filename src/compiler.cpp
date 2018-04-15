#include <llvm/IR/Verifier.h>          //for verifying basic structure of functions
#include <llvm/Support/FileSystem.h>   //for r/w when outputting bitcode
#include <llvm/Support/raw_ostream.h>  //for ostream when outputting bitcode

#if LLVM_VERSION_MAJOR >= 6
#include <llvm/Support/raw_os_ostream.h>
#endif

#include <llvm/Transforms/Scalar.h>    //for most passes
#include <llvm/IR/LegacyPassManager.h>
#include <llvm/Support/TargetRegistry.h>
#include <llvm/Target/TargetMachine.h>
#include <llvm/Linker/Linker.h>
#include <llvm/ExecutionEngine/SectionMemoryManager.h>
#include <llvm/ExecutionEngine/GenericValue.h>
#include <llvm/Transforms/IPO/AlwaysInliner.h>

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
        ret = CompilingVisitor::compile(c, n);
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
                    APInt(getBitWidthOfTypeTag(n->type),
                    atol(n->val.c_str()), isUnsignedTypeTag(n->type))),
            AnType::getPrimitive(n->type));
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
    val = TypedValue(ConstantFP::get(*c->ctxt, APFloat(typeTagToFltSemantics(n->type), n->val.c_str())),
            AnType::getPrimitive(n->type));
}


void CompilingVisitor::visit(BoolLitNode *n){
    val = TypedValue(ConstantInt::get(*c->ctxt, APInt(1, (bool)val, true)),
            AnType::getBool());
}


/**
 * @brief this is a stub.  ModNodes should be handled manually in visit(DeclNode) methods
 */
void CompilingVisitor::visit(ModNode *n){
    throw std::runtime_error("CompilingVisitor::visit(ModNode *n): stub");
}


/** returns true if this tag type does not have any associated types. */
bool isSimpleTag(AnDataType *dt){
    return dt->extTys.size() == 1
       and dt->extTys[0] == AnType::getVoid();
}


/**
 * @brief Compiles a TypeNode
 *
 * @return The tag value if this node is a union tag, otherwise it returns
 *         a compile-time value of type Type
 */
void CompilingVisitor::visit(TypeNode *n){
    //check for enum value
    if(n->type == TT_Data || n->type == TT_TaggedUnion){
        auto *dataTy = AnDataType::get(n->typeName);
        if(!dataTy or dataTy->isStub() or !isSimpleTag(dataTy)) goto rettype;

        auto *unionDataTy = dataTy->parentUnionType;
        if(!unionDataTy or unionDataTy->isStub()) goto rettype;

        size_t tagIndex = unionDataTy->getTagVal(n->typeName);
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
        val = TypedValue(unionVal, unionDataTy);
        return;
    }

rettype:
    //return the type as a value
    auto *ty = toAnType(c, n);

    //The TypeNode* address is wrapped in an llvm int so that llvm::Value methods can be called
    //without crashing, even if their result is meaningless
    Value *v = c->builder.getInt64((unsigned long)ty);
    val =TypedValue(v, AnType::getPrimitive(TT_Type));
}


/**
 * @brief Compiles all top-level import expressions
 */
void scanImports(Compiler *c, RootNode *r){
    for(auto &n : r->imports){
        try{
            CompilingVisitor::compile(c, n.get());
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
            val = CompilingVisitor::compile(c, n);
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
        auto fd = c->getCastFuncDecl(val.type, strty);
        auto fnty = AnFunctionType::get(c, AnType::getVoid(), fd->fdn->params.get());

        if(!fd or !c->typeEq(fnty->extTys, {val.type})){
            delete ls;
            delete rs;
            return c->compErr("Cannot cast " + anTypeToColoredStr(val.type)
                + " to Str for string interpolation.", valNode->loc);
        }

        if(c->isJIT){
            val = compileAndCallAnteFunction(c, fd->getName(), fd->mangledName, {val});
        }else{
            auto fn = c->getCastFn(val.type, strty, fd);
            val = TypedValue(c->builder.CreateCall(fn.val, val.val), strty);
        }
    }

    //Finally, the interpolation is done.  Now just combine the three strings
    //get the ++_Str_Str function
    string appendFn = "++";
    string mangledAppendFn = "++_Str_Str";
    auto lstr = CompilingVisitor::compile(c, ls);
    auto rstr = CompilingVisitor::compile(c, rs);

    if(c->isJIT){
        auto appendL = compileAndCallAnteFunction(c, appendFn, mangledAppendFn, {lstr, val});
        return compileAndCallAnteFunction(c, appendFn, mangledAppendFn, {appendL, rstr});
    }

    auto fn = c->getFunction(appendFn, mangledAppendFn);
    if(!fn) return c->compErr("++ overload for Str and Str not found while performing Str interpolation."
            "  The prelude may not be imported correctly.", sln->loc);

    //call the ++ function to combine the three strings
    auto *appendL = c->builder.CreateCall(fn.val, vector<Value*>{lstr.val, val.val});
    auto *appendR = c->builder.CreateCall(fn.val, vector<Value*>{appendL, rstr.val});

    //create the returning typenode
    return TypedValue(appendR, strty);
}


void CompilingVisitor::visit(StrLitNode *n){
    auto idx = n->val.find("${");

    if(idx != string::npos and (idx == 0 or n->val.find("\\${") != idx - 1)){
        this->val = compStrInterpolation(c, n, idx);
        return;
    }

    AnType *strty = AnDataType::get("Str");

    auto *ptr = c->builder.CreateGlobalStringPtr(n->val, "_strlit");

	//get the llvm Str data type from a fake type node in case we are compiling
	//the prelude and the Str data type isnt translated into an llvmty yet
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
    AnType *elemTy = n->exprs.empty() ? AnType::getVoid() : nullptr;

    int i = 1;
    for(auto& n : n->exprs){
        auto tval = CompilingVisitor::compile(c, n);

        arr.push_back((Constant*)tval.val);

        if(!elemTy){
            elemTy = tval.type;
        }else{
            if(!c->typeEq(tval.type, elemTy))
                c->compErr("Element " + to_string(i) + "'s type " + anTypeToColoredStr(tval.type) +
                    " does not match the first element's type of " + anTypeToColoredStr(elemTy), n->loc);
        }
        i++;
    }

    if(n->exprs.empty()){
        auto *ty = ArrayType::get(Type::getInt8Ty(*c->ctxt)->getPointerTo(), 0);
        auto *carr = ConstantArray::get(ty, arr);
        this->val = TypedValue(carr, AnArrayType::get(elemTy, 0));
    }else{
        auto *ty = ArrayType::get(arr[0]->getType(), n->exprs.size());
        auto *carr = ConstantArray::get(ty, arr);
        this->val = TypedValue(carr, AnArrayType::get(elemTy, n->exprs.size()));
    }
}

/**
 * @brief Creates and returns a literal of type void
 *
 * @return A void literal
 */
TypedValue Compiler::getVoidLiteral(){
    return TypedValue(UndefValue::get(Type::getInt8Ty(*ctxt)), AnType::getVoid());
}

void CompilingVisitor::visit(TupleNode *n){
    //A void value is represented by the empty tuple, ()
    if(n->exprs.empty()){
        this->val = c->getVoidLiteral();
        return;
    }

    vector<Constant*> elems;
    elems.reserve(n->exprs.size());

    vector<Type*> elemTys;
    elemTys.reserve(n->exprs.size());

    vector<AnType*> anElemTys;
    anElemTys.reserve(n->exprs.size());

    map<unsigned, Value*> pathogenVals;

    //Compile every value in the tuple, and if it is not constant,
    //add it to pathogenVals
    for(unsigned i = 0; i < n->exprs.size(); i++){
        auto tval = CompilingVisitor::compile(c, n->exprs[i]);

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

        if(tv && tv.type->typeTag != TT_Void)
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
    TypedValue ret = val;

    auto retInst = ret.type->typeTag == TT_Void ?
                 TypedValue(c->builder.CreateRetVoid(), ret.type) :
                 TypedValue(c->builder.CreateRet(ret.val), ret.type);

    auto *f = c->getCurrentFunction();
    f->returns.push_back({retInst, n->expr->loc});
    this->val = retInst;
}

/** add ".an" if string does not end with it already */
string addAnSuffix(string const& s){
    if(s.empty() || (s.length() >= 3 && s.substr(s.length()-3) == ".an")){
        return s;
    }else{
        return s + ".an";
    }
}

/**
 * Return a copy of the given string with the first character in lowercase.
 */
string lowercaseFirstLetter(string const& s){
    if(s.empty()) return "";
    return char(tolower(s[0])) + s.substr(1);
}

/**
 * Convert an import expression to a filepath string.
 * Converts most tokens as given, but lowercases the first letter of types
 * as these modules are expected to meet the convention of capital module
 * name referring to a lowercase filename.  If this is not desired, string
 * literals can be used instead.
 */
string moduleExprToStr(Node *expr){
    if(BinOpNode *bn = dynamic_cast<BinOpNode*>(expr)){
        if(bn->op != '.') return "";

        return moduleExprToStr(bn->lval.get()) + "/" + moduleExprToStr(bn->rval.get());
    }else if(TypeNode *tn = dynamic_cast<TypeNode*>(expr)){
        if(tn->type != TT_Data || !tn->params.empty()) return "";

        return lowercaseFirstLetter(tn->typeName);
    }else if(VarNode *va = dynamic_cast<VarNode*>(expr)){
        return va->name;
    }else if(StrLitNode *sln = dynamic_cast<StrLitNode*>(expr)){
        return sln->val;
    }else{
        return "";
    }
}

/**
 * Converts an import expression to a filepath string.
 * See moduleExprToStr for details.
 */
string importExprToStr(Node *expr){
    if(StrLitNode *sln = dynamic_cast<StrLitNode*>(expr)){
        return sln->val;
    }else{
        return addAnSuffix(moduleExprToStr(expr));
    }
}


/*
 * TODO: implement for abitrary compile-time Str expressions
 */
void CompilingVisitor::visit(ImportNode *n){
    string path = importExprToStr(n->expr.get());
    if(path.empty()){
        c->compErr("No viable overload for import for malformed expression", n->loc);
    }

    c->importFile(path.c_str(), n->loc);
    val =c->getVoidLiteral();
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
    this->val = c->getVoidLiteral();
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
    auto *dt = dyn_cast<AnDataType>(rangev.type);
    if(!dt or !c->typeImplementsTrait(dt, "Iterator")){
        auto res = c->callFn("into_iter", {rangev});

        if(!res)
            c->compErr("Range expression of type " + anTypeToColoredStr(rangev.type) + " needs to implement " +
                anTypeToColoredStr(AnDataType::get("Iterable")) + " or " + anTypeToColoredStr(AnDataType::get("Iterator")) +
                " to be used in a for loop", n->range->loc);

        rangev = res;
    }

    //by this point, rangev now properly stores the range information,
    //so store it on the stack and insert calls to unwrap, has_next,
    //and next at the beginning, beginning, and end of the loop respectively.
    Value *alloca = c->builder.CreateAlloca(rangev.getType());
    c->builder.CreateStore(rangev.val, alloca);

    c->builder.CreateBr(cond);
    c->builder.SetInsertPoint(cond);

    //set var = unwrap range

    //candval = is_done range
    auto rangeVal = TypedValue(c->builder.CreateLoad(alloca), rangev.type);

    auto is_done = c->callFn("has_next", {rangeVal});
    if(!is_done) c->compErr("Range expression of type " + anTypeToColoredStr(rangev.type) + " does not implement " +
            anTypeToColoredStr(AnDataType::get("Iterable")) + ", which it needs to be used in a for loop", n->range->loc);

    c->builder.CreateCondBr(is_done.val, begin, end);
    c->builder.SetInsertPoint(begin);

    //call unwrap at start of loop
    //make sure to update rangeVal
    rangeVal = TypedValue(c->builder.CreateLoad(alloca), rangev.type);
    auto uwrap = c->callFn("unwrap", {rangeVal});
    if(!uwrap) c->compErr("Range expression of type " + anTypeToColoredStr(rangev.type) + " does not implement " +
            anTypeToColoredStr(AnDataType::get("Iterable")) + ", which it needs to be used in a for loop", n->range->loc);

    auto *uwrap_var = new Variable(n->var, uwrap, c->scope);
    c->stoVar(n->var, uwrap_var);


    //register the branches to break/continue to right before the body
    //is compiled in case there was an error compiling the range
    c->compCtxt->breakLabels->push_back(end);
    c->compCtxt->continueLabels->push_back(incr);

    //compile the rest of the loop's body
    try{
        n->child->accept(*this);
    }catch(CtError *e){
        c->compCtxt->breakLabels->pop_back();;
        c->compCtxt->continueLabels->pop_back();
        throw e;
    }

    c->compCtxt->breakLabels->pop_back();;
    c->compCtxt->continueLabels->pop_back();

    if(!val) return;
    if(!dyn_cast<ReturnInst>(val.val) and !dyn_cast<BranchInst>(val.val)){
        //set range = next range
        c->builder.CreateBr(incr);
        c->builder.SetInsertPoint(incr);

        TypedValue arg = {c->builder.CreateLoad(alloca), rangev.type};
        auto next = c->callFn("next", {arg});
        if(!next) c->compErr("Range expression of type " + anTypeToColoredStr(rangev.type) + " does not implement " + anTypeToColoredStr(AnDataType::get("Iterable")) +
                ", which it needs to be used in a for loop", n->range->loc);

        c->builder.CreateStore(next.val, alloca);
        c->builder.CreateBr(cond);
    }

    c->builder.SetInsertPoint(end);
    this->val = c->getVoidLiteral();
}


void CompilingVisitor::visit(JumpNode *n){
    n->expr->accept(*this);

    auto *ci = dyn_cast<ConstantInt>(val.val);
    if(!ci)
        c->compErr("Expression must evaluate to a constant integer\n", n->expr->loc);

    if(!isUnsignedTypeTag(val.type->typeTag) and ci->getSExtValue() < 0)
        c->compErr("Cannot jump out of a negative number (" + to_string(ci->getSExtValue()) +  ") of loops", n->expr->loc);

    //we can now safely get the zero-extended value of ci since even if it is signed, it is not negative
    auto jumpCount = ci->getZExtValue();

    //NOTE: continueLabels->size() == breakLabels->size() always
    auto loopCount = c->compCtxt->breakLabels->size();

    if(loopCount == 0)
        c->compErr("There are no loops to jump out of", n->loc);


    if(jumpCount == 0)
        c->compErr("Cannot jump out of 0 loops", n->expr->loc);


    if(jumpCount > loopCount)
        c->compErr("Cannot jump out of " + to_string(jumpCount) + " loops when there are only " +
                to_string(c->compCtxt->breakLabels->size()) + " loop(s) nested", n->expr->loc);

    //actually create the branch instruction
    BranchInst *br = n->jumpType == Tok_Continue ?
        c->builder.CreateBr( c->compCtxt->continueLabels->at(loopCount - jumpCount) ) :
        c->builder.CreateBr( c->compCtxt->breakLabels->at(loopCount - jumpCount) );

    //Although returning a void, use the br as the value so loops know the last instruction was a br and not to insert another
    this->val = TypedValue(br, AnType::getVoid());
}


//create a new scope if the user indents
void CompilingVisitor::visit(BlockNode *n){
    c->enterNewScope();
    n->block->accept(*this);
    c->exitScope();
}


/**
 *  @brief This is a stub.  Compilation of parameters is handled within Compiler::compFn
 */
void CompilingVisitor::visit(NamedValNode *n)
{
    //STUB
}


/**
 * @brief Performs a lookup for an identifier and returns its value if found
 *
 * @return The value of the variable
 */
void CompilingVisitor::visit(VarNode *n){
    auto *var = c->lookup(n->name);

    if(var){
        if(var->autoDeref){
            auto *load = c->builder.CreateLoad(var->getVal(), n->name);
            this->val = TypedValue(load, var->tval.type);
        }else{
            this->val = TypedValue(var->tval.val, var->tval.type);
        }
    }else{
        //if this is a function, then there must be only one function of the same name, otherwise the reference is ambiguous
        auto& fnlist = c->getFunctionList(n->name);

        if(fnlist.size() == 1){
            auto& fd = *fnlist.begin();
            if(!fd->tv)
                fd->tv = c->compFn(fd.get());

            if(!fd or !fd->tv){
                c->errFlag = true;
                return;
            }

            this->val = TypedValue(fd->tv.val, fd->tv.type);
        }else if(fnlist.empty()){
            c->compErr("Variable or function '" + n->name + "' has not been declared.", n->loc);
        }else{
            this->val = FunctionCandidates::getAsTypedValue(c->ctxt.get(), fnlist, {});
        }
    }
}


void CompilingVisitor::visit(LetBindingNode *n){
    n->expr->accept(*this);

    if(val.type->typeTag == TT_Void)
        c->compErr("Cannot assign a "+anTypeToColoredStr(AnType::getVoid())+
                " value to a variable", n->expr->loc);

    TypeNode *tyNode;
    if((tyNode = (TypeNode*)n->typeExpr.get())){
        auto *anty = toAnType(c, tyNode);
        if(!llvmTypeEq(val.val->getType(), c->anTypeToLlvmType(anty))){
            c->compErr("Incompatible types in explicit binding.", n->expr->loc);
        }
    }

    bool isGlobal = false;

    //add the modifiers to the typedvalue
    for(Node *n : *n->modifiers){
        int m = ((ModNode*)n)->mod;
        val.type = val.type->addModifier((TokenType)m);
        if(m == Tok_Global) isGlobal = true;
    }

    if(isGlobal){
        auto *ty = c->anTypeToLlvmType(val.type);
        auto *global = new GlobalVariable(*c->module, ty, false, GlobalValue::PrivateLinkage, UndefValue::get(ty), n->name);
        c->builder.CreateStore(val.val, global);
        val.val = global;
    }

    if(val.getType()->isArrayTy() and not isGlobal){
        Value *alloca = c->builder.CreateAlloca(val.getType(), nullptr, n->name.c_str());
        c->builder.CreateStore(val.val, alloca);
        val.val = alloca;
        isGlobal = true;
    }

    c->stoVar(n->name, new Variable(n->name, val, c->scope, true, isGlobal));
    //return val;
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
    TypedValue val = CompilingVisitor::compile(c, node->expr);
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

void CompilingVisitor::visit(VarDeclNode *n){
    //check for redeclaration, but only on topmost scope
    auto redeclare = c->varTable.back()->find(n->name);
    if(redeclare != c->varTable.back()->end()){
        c->compErr("Variable " + n->name + " was redeclared.", n->loc);
    }

    //check for an inferred type
    if(!n->typeExpr.get()){
        this->val = compVarDeclWithInferredType(n, c);
        return;
    }

    if(((TypeNode*)n->typeExpr.get())->type == TT_Void)
        c->compErr("Cannot create a variable of type "+
                anTypeToColoredStr(AnType::getVoid()), n->typeExpr->loc);


    //the type held by this node will be deleted when the parse tree is, so copy
    //this one so it is not double freed
    AnType *anTy = toAnType(c, (TypeNode*)n->typeExpr.get());

    Type *ty = c->anTypeToLlvmType(anTy);

    bool isGlobal = false;

    //Add all of the declared modifiers to the typedval
    for(Node *n : *n->modifiers){
        int m = ((ModNode*)n)->mod;
        anTy = anTy->addModifier((TokenType)m);
        if(m == Tok_Global) isGlobal = true;
    }

    if(!anTy->hasModifier(Tok_Mut))
        anTy = anTy->addModifier(Tok_Mut);

    //location to store var
    Value *loc = isGlobal ?
        (Value*) new GlobalVariable(*c->module, ty, false, GlobalValue::PrivateLinkage, UndefValue::get(ty), n->name) :
        c->builder.CreateAlloca(ty, nullptr, n->name.c_str());

    TypedValue alloca = TypedValue(loc, anTy);

    Variable *var = new Variable(n->name, alloca, c->scope, true, true);
    c->stoVar(n->name, var);
    if(n->expr.get()){
        n->expr->accept(*this);
        if(val.type->typeTag == TT_Void)
            c->compErr("Cannot assign a "+anTypeToColoredStr(AnType::getVoid())+
                    " value to a variable", n->expr->loc);

        AnType *exprTy = val.type->addModifier(Tok_Mut);
        var->noFree = true;//var->getType() != TT_Ptr || dynamic_cast<Constant*>(val->val);

        //Make sure the assigned value matches the variable's type
        auto *allocaTy = (AnPtrType*)alloca.type;
        if(!c->typeEq(allocaTy->extTy, exprTy)){
            c->compErr("Cannot assign expression of type " + anTypeToColoredStr(val.type)
                        + " to a variable of type " + anTypeToColoredStr(allocaTy->extTy), n->expr->loc);
        }

        //transfer ownership of val->type
        this->val = TypedValue(c->builder.CreateStore(val.val, alloca.val), exprTy);
    }else{
        this->val = alloca;
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
        auto l = CompilingVisitor::compile(c, bop->lval);

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

            auto newval = CompilingVisitor::compile(c, expr);

            //see if insert operator # = is overloaded already
            string op = "#";
            string mangledfn = mangle(op, {tyn, AnType::getI32(), newval.type});
            auto fn = c->getFunction(op, mangledfn);
            if(fn)
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

    //otherwise, this is just a normal assign to a variable
    n->ref_expr->accept(*this);

    //if(!dynamic_cast<LoadInst*>(val->val))
    if(!val.type->hasModifier(Tok_Mut))
        c->compErr("Variable must be mutable to be assigned to, but instead is an immutable " +
                anTypeToColoredStr(val.type), n->ref_expr->loc);

    Value *dest = ((LoadInst*)val.val)->getPointerOperand();

    //compile the expression to store
    TypedValue assignExpr = CompilingVisitor::compile(c, n->expr);

    //lvalue must compile to a pointer for storage, usually an alloca value
    if(!PointerType::isLoadableOrStorableType(val.getType())){
        c->compErr("Attempted assign without a memory address, with type "
                + anTypeToColoredStr(val.type), n->ref_expr->loc);
    }

    //and finally, make sure the assigned value matches the variable's type
    if(!c->typeEq(val.type, assignExpr.type)){
        c->compErr("Cannot assign expression of type " + anTypeToColoredStr(assignExpr.type)
                    + " to a variable of type " + anTypeToColoredStr(val.type), n->expr->loc);
    }

    //now actually create the store
    c->builder.CreateStore(assignExpr.val, dest);

    //all assignments return a void value
    this->val = c->getVoidLiteral();
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
        if(tv->typeTag != TT_Void)
            name += "_" + anTypeToStrWithoutModifiers(tv);
    }
    return name;
}

string mangle(FuncDecl *fd, vector<AnType*> const& params){
    string name = fd->fdn->name;
    for(auto *tv : params)
        if(tv->typeTag != TT_Void)
            name += "_" + anTypeToStrWithoutModifiers(tv);
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
        else if(tn->type != TT_Void)
            name += "_" + typeNodeToStr(tn);

        cur = (NamedValNode*)cur->next.get();
    }
    return name;
}

string mangle(string const& base, TypeNode *paramTys){
    string name = base;
    while(paramTys){
        if(paramTys->type != TT_Void)
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


void CompilingVisitor::visit(ExtNode *n){
    if(n->traits.get()){
        //this ExtNode is an implementation of a trait
        string typestr = typeNodeToStr(n->typeExpr.get());
        AnDataType *dt;

        if(n->typeExpr->typeName.empty()){ //primitive type being extended
            dt = AnDataType::get(typestr);
            if(!dt or dt->isStub()){ //if primitive type has not been extended before, make it a DataType to store in
                dt = AnDataType::create(typestr, {toAnType(c, n->typeExpr.get())}, false, {});
                c->stoType(dt, typestr);
            }
        }else{
            dt = AnDataType::get(typestr);
            if(!dt or dt->isStub())
                c->compErr("Cannot implement traits for undeclared type " +
                        typeNodeToColoredStr(n->typeExpr.get()), n->typeExpr->loc);
        }

        //create a vector of the traits that must be implemented
        TypeNode *curTrait = n->traits.get();
        vector<Trait*> traits;
        while(curTrait){
            string traitstr = typeNodeToStr(curTrait);
            auto *trait = c->lookupTrait(traitstr);
            if(!trait)
                c->compErr("Trait " + typeNodeToColoredStr(curTrait)
                        + " is undeclared", curTrait->loc);

            traits.push_back(trait);
            curTrait = (TypeNode*)curTrait->next.get();
        }

        //go through each trait and compile the methods for it
        auto *funcs = n->methods.release();
        for(auto& trait : traits){
            auto *traitImpl = new Trait();
            traitImpl->name = trait->name;

            for(auto& fd_proto : trait->funcs){
                auto *fdn = findFDN(funcs, fd_proto->getName());

                if(!fdn)
                    c->compErr(typeNodeToColoredStr(n->typeExpr.get()) + " must implement " + fd_proto->getName() +
                        " to implement " + anTypeToColoredStr(AnDataType::get(trait->name)), fd_proto->fdn->loc);

                string mangledName = c->funcPrefix + mangle(fdn->name, fdn->params);
                fdn->name = c->funcPrefix + fdn->name;

                //If there is a self param it would be mangled incorrectly above as mangle does not have
                //access to what type 'self' references, so fix that here.
                auto *oldTn = c->compCtxt->objTn;
                c->compCtxt->objTn = n->typeExpr.get();
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
        auto params = move(n->typeExpr->params);
        c->funcPrefix = typeNodeToStr(n->typeExpr.get()) + "_";
        n->typeExpr->params = move(params);

        auto prevObj = c->compCtxt->obj;
        auto prevObjTn = c->compCtxt->objTn;

        c->compCtxt->obj = toAnType(c, n->typeExpr.get());
        c->compCtxt->objTn = n->typeExpr.get();

        compileStmtList(n->methods.release(), c);

        c->funcPrefix = oldPrefix;
        c->compCtxt->obj = prevObj;
        c->compCtxt->objTn = prevObjTn;
    }
    this->val = c->getVoidLiteral();
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
            exts.push_back(tagTy);
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
    data->isAlias = n->isAlias;

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

void CompilingVisitor::visit(DataDeclNode *n){
    //{   //new scope to ensure dt isn't used after this check
    //    auto *dt = AnDataType::get(this->name);
    //    if(dt and !dt->isStub()) return c->compErr("Type " + name + " was redefined", loc);
    //}

    auto *nvn = (NamedValNode*)n->child.get();
    if(((TypeNode*) nvn->typeExpr.get())->type == TT_TaggedUnion){
        this->val = compTaggedUnion(c, n);
        return;
    }

    //Create the DataType as a stub first, have its contents be recursive
    //just to cause an error if something tries to use the stub
    AnDataType *data = AnDataType::create(n->name, {}, false, toVec(c, n->generics));

    if(data->llvmType)
        data->llvmType = nullptr;

    c->stoType(data, n->name);

    vector<string> fieldNames;
    vector<AnType*> fieldTypes;

    fieldNames.reserve(n->fields);
    fieldTypes.reserve(n->fields);

    while(nvn){
        TypeNode *tyn = (TypeNode*)nvn->typeExpr.get();
        auto ty = toAnType(c, tyn);

        validateType(c, ty, n);

        fieldTypes.push_back(ty);
        fieldNames.push_back(nvn->name);

        nvn = (NamedValNode*)nvn->next.get();
    }

    data->fields = fieldNames;
    data->extTys = fieldTypes;
    data->isAlias = n->isAlias;

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
    this->val = c->getVoidLiteral();
}

void DataDeclNode::declare(Compiler *c){
    AnDataType::create(name, {}, false, toVec(c, generics));
}


void CompilingVisitor::visit(TraitNode *n){
    auto *trait = new Trait();
    trait->name = n->name;

    auto *curfn = n->child.release();
    while(curfn){
        auto *fn = (FuncDeclNode*)curfn;
        string mangledName = c->funcPrefix + mangle(fn->name, fn->params);
        fn->name = c->funcPrefix + fn->name;

        shared_ptr<FuncDeclNode> spfdn{fn};
        shared_ptr<FuncDecl> fd{new FuncDecl(spfdn, mangledName, c->scope, c->mergedCompUnits)};

        //create trait type as a generic void* container
        vector<AnType*> ext;
        ext.push_back(AnPtrType::get(AnType::getVoid()));
        fd->obj = AnDataType::getOrCreate(n->name, ext, false);

        trait->funcs.push_back(fd);
        curfn = curfn->next.get();
    }

    auto traitPtr = shared_ptr<Trait>(trait);
    c->compUnit->traits[n->name] = traitPtr;
    c->mergedCompUnits->traits[n->name] = traitPtr;

    this->val = c->getVoidLiteral();
}


/**
 * @brief Compiles the global expression importing global vars.  This compiles
 *        the statement-like version of a GlobalNode.  The modifier-like version
 *        is handled along with other modifiers during a variable's declaration.
 *
 * @return The value of the last global brought into scope
 */
void CompilingVisitor::visit(GlobalNode *n){
    TypedValue ret;
    for(auto &varName : n->vars){
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
            c->compErr("Variable '" + varName->name + "' has not been declared.", varName->loc);

        if(!var->tval.type->hasModifier(Tok_Global))
            c->compErr("Variable " + varName->name + " must be global to be imported.", varName->loc);

        var->scope = c->scope;
        c->stoVar(varName->name, new Variable(*var));
        ret = var->tval;
    }

    this->val = TypedValue(c->builder.CreateLoad(ret.val), ret.type);
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


void CompilingVisitor::visit(MatchNode *n){
    n->expr->accept(*this);
    auto valToMatch = this->val;

    if(valToMatch.type->typeTag != TT_TaggedUnion && valToMatch.type->typeTag != TT_Data){
        c->compErr("Cannot match expression of type " + anTypeToColoredStr(valToMatch.type) +
                ".  Match expressions must be a tagged union type", n->expr->loc);
    }

    //the tag is always the zero-th index except for in certain optimization cases and if
    //the tagged union has no tagged values and is equivalent to an enum in C-like languages.
    Value *switchVal = llvmTypeToTypeTag(valToMatch.getType()) == TT_Tuple ?
            c->builder.CreateExtractValue(valToMatch.val, 0)
            : valToMatch.val;

    Function *f = c->builder.GetInsertBlock()->getParent();
    auto *matchbb = c->builder.GetInsertBlock();

    auto *end = BasicBlock::Create(*c->ctxt, "end_match");
    auto *match = c->builder.CreateSwitch(switchVal, end, n->branches.size());
    vector<pair<BasicBlock*,TypedValue>> merges;

    for(auto& mbn : n->branches){
        ConstantInt *ci = nullptr;
        auto *br = BasicBlock::Create(*c->ctxt, "br", f);
        c->builder.SetInsertPoint(br);
        c->enterNewScope();

        //TypeCast-esque pattern:  Some n
        if(TypeCastNode *tn = dynamic_cast<TypeCastNode*>(mbn->pattern.get())){
            auto *tagTy = AnDataType::get(tn->typeExpr->typeName);
            if(!tagTy or tagTy->isStub())
                c->compErr("Union tag " + typeNodeToColoredStr(tn->typeExpr.get()) + " was not yet declared.", tn->typeExpr->loc);

            if(!tagTy->isUnionTag())
                c->compErr(typeNodeToColoredStr(tn->typeExpr.get()) + " must be a union tag to be used in a pattern", tn->typeExpr->loc);

            auto *parentTy = tagTy->parentUnionType;
            ci = ConstantInt::get(*c->ctxt, APInt(8, parentTy->getTagVal(tn->typeExpr->typeName), true));

            tagTy = (AnDataType*)bindGenericToType(c, tagTy, ((AnDataType*)valToMatch.type)->boundGenerics);
            tagTy = tagTy->setModifier(valToMatch.type->mods);
            handleTypeCastPattern(c, valToMatch, tn, tagTy, parentTy);

        //single type pattern:  None
        }else if(TypeNode *tn = dynamic_cast<TypeNode*>(mbn->pattern.get())){
            auto *tagTy = AnDataType::get(tn->typeName);
            if(!tagTy or tagTy->isStub())
                c->compErr("Union tag " + typeNodeToColoredStr(tn) + " was not yet declared.", tn->loc);

            if(!tagTy->isUnionTag())
                c->compErr(typeNodeToColoredStr(tn) + " must be a union tag to be used in a pattern", tn->loc);

            auto *parentTy = tagTy->parentUnionType;
            ci = ConstantInt::get(*c->ctxt, APInt(8, parentTy->getTagVal(tn->typeName), true));

        //variable/match-all pattern: _
        }else if(VarNode *vn = dynamic_cast<VarNode*>(mbn->pattern.get())){
            auto tn = TypedValue(valToMatch.val, valToMatch.type);
            match->setDefaultDest(br);
            c->stoVar(vn->name, new Variable(vn->name, tn, c->scope));
        }else{
            c->compErr("Pattern matching non-tagged union types is not yet implemented", mbn->pattern->loc);
        }

        mbn->branch->accept(*this);
        c->exitScope();

        if(!dyn_cast<ReturnInst>(val.val) and !dyn_cast<BranchInst>(val.val))
            c->builder.CreateBr(end);

        merges.push_back(pair<BasicBlock*,TypedValue>(c->builder.GetInsertBlock(), val));

        if(ci)
            match->addCase(ci, br);
    }

    f->getBasicBlockList().push_back(end);
    c->builder.SetInsertPoint(end);

    //merges can be empty if each branch has an early return
    if(merges.empty() or merges[0].second.type->typeTag == TT_Void){
        this->val = c->getVoidLiteral();
        return;
    }

    int i = 1;
    auto *phi = c->builder.CreatePHI(merges[0].second.getType(), n->branches.size());
    for(auto &pair : merges){

        //add each branch to the phi node if it does not return early
        if(!dyn_cast<ReturnInst>(pair.second.val)){

            //match the types of those branches that will merge
            if(!c->typeEq(pair.second.type, merges[0].second.type))
                c->compErr("Branch "+to_string(i)+"'s return type " + anTypeToColoredStr(pair.second.type) +
                        " != " + anTypeToColoredStr(merges[0].second.type) + ", the first branch's return type", n->loc);
            else
                phi->addIncoming(pair.second.val, pair.first);
        }
        i++;
    }
    phi->addIncoming(UndefValue::get(merges[0].second.getType()), matchbb);
    this->val = TypedValue(phi, merges[0].second.type);
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

inline bool fileExists(const string &fName){
    if(FILE *f = fopen(fName.c_str(), "r")){
        fclose(f);
        return true;
    }
    return false;
}

/**
 * Returns the first path to a given filename
 * matched within the relative root directories.
 * If no file is found then the empty string is returned.
 */
string findFile(Compiler *c, string const& fName){
    for(auto &root : c->relativeRoots){
        string f = root + addAnSuffix(fName);

        if(fileExists(f)){
            return f;
        }
    }
    return "";
}


void Compiler::importFile(string const& fName, LOC_TY &loc){
    //f = fName with full directory
    string f = findFile(this, fName);
    auto it = allCompiledModules.find(f);

    if(it != allCompiledModules.end()){
        //module already compiled
        auto *import = it->getValue().get();
        string fmodName = removeFileExt(fName);

        for(auto &mod : imports){
            if(mod->name == fmodName){
                compErr("Module " + string(fName) + " has already been imported", loc, ErrorType::Warning);
            }
        }

        imports.push_back(import);
        mergedCompUnits->import(import);
    }else{
        if(f.empty()){
            compErr("No file named '" + string(fName) + "' was found.", loc);
        }

        //module not found; create new Compiler instance to compile it
        auto c = unique_ptr<Compiler>(new Compiler(f.c_str(), true, ctxt));
        c->ctxt = ctxt;
        c->module.reset(module.get());
        c->compile();

        if(c->errFlag){
            compErr("Error when importing '" + string(fName) + "'", loc);
        }

        c->module.release();
        imports.push_back(c->compUnit);
        mergedCompUnits->import(c->compUnit);
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
        auto fakeLoc = mkLoc(mkPos(0, 0, 0), mkPos(0, 0, 0));
        importFile("prelude.an", fakeLoc);
    }
}

string& Compiler::getModuleName() const {
    return compUnit->name;
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
void compileAll(Compiler *c, vector<T> &vec){
    CompilingVisitor v{c};
    for(auto &elem : vec){
        try{
            elem->accept(v);
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
    FunctionType *ft = isLib?
        FunctionType::get(Type::getInt32Ty(*ctxt), {}, false):
        FunctionType::get(Type::getInt32Ty(*ctxt), {argcty, argvty}, false);

    //Actually create the function in module m
    string fnName = isLib ? getModuleName() + "_init_module" : "main";
    Function *main = Function::Create(ft, Function::ExternalLinkage, fnName, module.get());

    //Create the entry point for the function
    BasicBlock *bb = BasicBlock::Create(*ctxt, "entry", main);
    builder.SetInsertPoint(bb);

    AnFunctionType *main_fn_ty;

    if(!isLib){
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

        main_fn_ty = AnFunctionType::get(AnType::getU8(), {argcAnty, argvAnty});
    }else{
        main_fn_ty = AnFunctionType::get(AnType::getU8(), {});
    }

    auto main_tv = TypedValue(main, main_fn_ty);
    auto fakeLoc = mkLoc(mkPos(0, 0, 0), mkPos(0, 0, 0));
    auto *fakeFdn = new FuncDeclNode(fakeLoc, fnName, nullptr, nullptr, nullptr, nullptr);
    shared_ptr<FuncDeclNode> fakeSp{fakeFdn};
    auto *main_var = new FuncDecl(fakeSp, fnName, scope, mergedCompUnits, main_tv);

    //TODO: merge this code with Compiler::registerFunction
    shared_ptr<FuncDecl> fd{main_var};
    compUnit->fnDecls[fnName].push_back(fd);
    mergedCompUnits->fnDecls[fnName].push_back(fd);

    compCtxt->callStack.push_back(main_var);
    return main;
}


void CompilingVisitor::visit(RootNode *n){
    scanImports(c, n);
    c->scanAllDecls(n);

    //Compile the rest of the program
    for(auto &node : n->main){
        try{
            node->accept(*this);
        }catch(CtError *e){
            delete e;
        }
    }

    if(n->main.empty())
        this->val = c->getVoidLiteral();
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
inline void addPasses(legacy::PassManager &pm, char optLvl){
    if(optLvl > 0){
        if(optLvl >= 3){
            pm.add(createLoopStrengthReducePass());
            pm.add(createLoopUnrollPass());
            pm.add(createMergedLoadStoreMotionPass());
            pm.add(createMemCpyOptPass());
            pm.add(createSpeculativeExecutionPass());
        }
        //pm.add(createAlwaysInlinerLegacyPass());
        pm.add(createDeadStoreEliminationPass());
        pm.add(createDeadCodeEliminationPass());
        pm.add(createCFGSimplificationPass());
        pm.add(createTailCallEliminationPass());
        pm.add(createInstructionSimplifierPass());

#if LLVM_VERSION_MAJOR < 5
        pm.add(createLoadCombinePass());
#endif
        pm.add(createLoopLoadEliminationPass());
        pm.add(createReassociatePass());
        pm.add(createPromoteMemoryToRegisterPass());

        //Instruction Combining Pass seems to break nested for loops
        //pm.add(createInstructionCombiningPass());
    }
}



void Compiler::compile(){
    if(compiled){
        cerr << "Module " << module->getName().str() << " is already compiled, cannot recompile.\n";
        return;
    }

    //create implicit main function and import the prelude
    createMainFn();
    compilePrelude();

    CompilingVisitor::compile(this, ast.get());

    //always return 0
    builder.CreateRet(ConstantInt::get(*ctxt, APInt(32, 0)));

    if(!errFlag and !isLib){
        legacy::PassManager pm;
        addPasses(pm, optLvl);
        pm.run(*module);
    }

    //flag this module as compiled.
    compiled = true;

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
#if LLVM_VERSION_MAJOR >= 6
        llvm::raw_os_ostream os{std::cout};
		TargetRegistry::printRegisteredTargetsForVersion(os);
#else
		TargetRegistry::printRegisteredTargetsForVersion();
#endif
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

#if LLVM_VERSION_MAJOR >= 6
    TargetMachine *tm = target->createTargetMachine(triple, cpu, features, op, Reloc::Model::Static,
            None, CodeGenOpt::Level::Aggressive);
#else
    TargetMachine *tm = target->createTargetMachine(triple, cpu, features, op, Reloc::Model::Static,
            CodeModel::Default, CodeGenOpt::Level::Aggressive);
#endif

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

    if(type->typeTag == TT_Void)
        puts("void ()");
    else if(type->typeTag == TT_Type)
        cout << anTypeToStr(extractTypeValue(*this)) << endl;
    else if(type->typeTag == TT_FunctionList){
        auto *fl = (FunctionCandidates*)val;
        cout << "(" << fl->candidates.size() << " function" << (fl->candidates.size() == 1 ? ")\n" : "s)\n");

        for(auto &c : fl->candidates){
            cout << endl << c->getName() << " (" << c->mangledName << "): \n";
            if(c->tv){
                c->tv.dump();
            }else{
                cout << "(not yet compiled)\n\n";
            }
            cout << "Parse tree:\n";
            PrintingVisitor pv;
            c->fdn->accept(pv);
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
    if(varTable.empty()) return;

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


Variable* Compiler::lookup(string const& var) const{
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


void Compiler::stoTypeVar(string const& name, AnType *ty){
    Value *addr = builder.getInt64((unsigned long)ty);
    TypedValue tv = TypedValue(addr, AnType::getPrimitive(TT_Type));
    Variable *var = new Variable(name, tv, scope);
    stoVar(name, var);
}

AnType* Compiler::lookupTypeVar(string const& name) const{
    auto tvar = lookup(name);
    if(!tvar) return nullptr;

    return extractTypeValue(tvar->tval);
}


AnDataType* Compiler::lookupType(string const& tyname) const{
    auto& ut = mergedCompUnits->userTypes;
    auto it = ut.find(tyname);
    if(it != ut.end())
        return it->getValue();
    return nullptr;
}

Trait* Compiler::lookupTrait(string const& tyname) const{
    auto& ts = mergedCompUnits->traits;
    auto it = ts.find(tyname);
    if(it != ts.end())
        return it->getValue().get();
    return nullptr;
}


inline void Compiler::stoType(AnDataType *dt, string const& typeName){
    //shared_ptr<AnDataType> dt{ty};
    compUnit->userTypes[typeName] = dt;
    mergedCompUnits->userTypes[typeName] = dt;
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
 * Converts a given filename (with its file
 * extension already removed) to a module name.
 *
 * - Replaces directory separators with '.'
 * - Capitalizes first letters of words
 * - Ignores non alphanumeric characters
 */
string toModuleName(string &s){
    string mod = "";
    bool capitalize = true;

    for(auto &c : s){
        if(capitalize and ((c >= 'a' and c <= 'z') or (c >= 'A' and c <= 'Z'))){
            if(c >= 'a' and c <= 'z'){
                mod += c + 'A' - 'a';
            }else{
                mod += c;
            }
            capitalize = false;
        }else{
#ifdef _WIN32
            if(c == '\\'){
#else
            if(c == '/'){
#endif
                if(&c != s.c_str()){
                    capitalize = true;
                    mod += '.';
                }
            }else if(c == '_'){
                capitalize = true;
            }else if(IS_ALPHANUM(c)){
                mod += c;
            }
        }
    }
    return mod;
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

    relativeRoots = {AN_EXEC_STR, AN_LIB_DIR};

    auto fileNameWithoutExt = removeFileExt(fileName);
    auto modName = toModuleName(fileNameWithoutExt);
    compUnit->name = modName;
    mergedCompUnits->name = modName;

    //Add this module to the cache to ensure it is not compiled twice
    allMergedCompUnits.emplace_back(mergedCompUnits);
    allCompiledModules.try_emplace(fileName, compUnit);

    outFile = fileNameWithoutExt;
	if (outFile.empty())
		outFile = "a.out";

    module.reset(new llvm::Module(outFile, *ctxt));

    enterNewScope();
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
    allCompiledModules.try_emplace(fileName, compUnit);
    relativeRoots = {AN_EXEC_STR, AN_LIB_DIR};

    compUnit->name = modName;
    mergedCompUnits->name = modName;

    ast.reset(new RootNode(root->loc));
    ast->main.push_back(unique_ptr<Node>(root));

    module.reset(new llvm::Module(outFile, *ctxt));

    enterNewScope();
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
}

} //end of namespace ante
