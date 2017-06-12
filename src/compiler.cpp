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
#include "target.h"
#include "yyparser.h"

using namespace llvm;


size_t getTupleSize(Node *tup){
    size_t size = 0;
    while(tup){
        tup = tup->next.get();
        size++;
    }
    return size;
}

/*
 *  Returns nth node from list.
 *  Does not check if list contains at least n nodes
 */
Node* getNthNode(Node *node, size_t n){
    for(; n > 0; n--)
        node = node->next.get();
    return node;
}

/*
 *  Compiles a statement list and returns its last statement.
 */
TypedValue* compileStmtList(Node *nList, Compiler *c){
    TypedValue *ret = nullptr;
    while(nList){
        ret = nList->compile(c);
        nList = nList->next.get();
    }
    return ret;
}


bool isUnsignedTypeTag(const TypeTag tt){
    return tt==TT_U8||tt==TT_U16||tt==TT_U32||tt==TT_U64||tt==TT_Usz;
}


TypedValue* IntLitNode::compile(Compiler *c){
    return new TypedValue(ConstantInt::get(*c->ctxt,
                            APInt(getBitWidthOfTypeTag(type), 
                            atol(val.c_str()), isUnsignedTypeTag(type))), mkAnonTypeNode(type));
}


const fltSemantics& typeTagToFltSemantics(TypeTag tokTy){
    switch(tokTy){
        case TT_F16: return APFloat::IEEEhalf();
        case TT_F32: return APFloat::IEEEsingle();
        case TT_F64: return APFloat::IEEEdouble();
        default:     return APFloat::IEEEdouble();
    }
}

TypedValue* FltLitNode::compile(Compiler *c){
    return new TypedValue(ConstantFP::get(*c->ctxt, APFloat(typeTagToFltSemantics(type), val.c_str())), mkAnonTypeNode(type));
}


TypedValue* BoolLitNode::compile(Compiler *c){
    return new TypedValue(ConstantInt::get(*c->ctxt, APInt(1, (bool)val, true)), mkAnonTypeNode(TT_Bool));
}


TypedValue* ModNode::compile(Compiler *c){
    return nullptr;
}


TypedValue* TypeNode::compile(Compiler *c){
    //check for enum value
    if(type == TT_Data || type == TT_TaggedUnion){
        auto *dataTy = c->lookupType(typeName);
        if(!dataTy) goto rettype;

        auto *unionDataTy = c->lookupType(dataTy->getParentUnionName());
        if(!unionDataTy) goto rettype;

        Value *tag = ConstantInt::get(*c->ctxt, APInt(8, unionDataTy->getTagVal(typeName), true));
        auto *ty = copy(unionDataTy->tyn);

        if(!unionDataTy->generics.empty()){
            for(auto &tn : unionDataTy->generics){
                TypeNode *v = mkTypeNodeWithExt(TT_Ptr, mkAnonTypeNode(TT_Void));
                c->stoTypeVar(tn->typeName, v);
            }
        }

        Type *unionTy = c->typeNodeToLlvmType(ty);
        Type *curTy = tag->getType();

        //allocate for the largest possible union member
        auto *alloca = c->builder.CreateAlloca(unionTy);

        //but make sure to bitcast it to the current member before storing an incorrect type
        Value *castTo = c->builder.CreateBitCast(alloca, curTy->getPointerTo());
        c->builder.CreateStore(tag, castTo);

        //load the initial alloca, not the bitcasted one
        Value *unionVal = c->builder.CreateLoad(alloca);
        ty->type = TT_TaggedUnion;
        return new TypedValue(unionVal, ty);
    }

rettype:
    //return the type as a value
    auto *cpy = copy(this);

    //The TypeNode* address is wrapped in an llvm int so that llvm::Value methods can be called
    //without crashing, even if their result is meaningless
    Value *v = c->builder.getInt64((unsigned long)cpy);
    return new TypedValue(v, mkAnonTypeNode(TT_Type));
}


void scanImports(Compiler *c, RootNode *r){
    for(auto &n : r->imports){
        try{
            n->compile(c);
        }catch(CtError *e){
            delete e;
        }
    }
}

TypedValue* compStrInterpolation(Compiler *c, StrLitNode *sln, int pos){
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
    setLexer(new Lexer(sln->loc.begin.filename, m, sln->loc.begin.line-1, sln->loc.begin.column + pos));
    yy::parser p{};
    int flag = p.parse();
    if(flag != PE_OK){ //parsing error, cannot procede
        fputs("Syntax error in string interpolation, aborting.\n", stderr);
        exit(flag);
    }

    RootNode *expr = parser::getRootNode();
    TypedValue *val = 0;
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

    if(!val) return 0;

    //if the expr is not already a string type, cast it to one
    if(val->type->typeName != "Str"){
        auto *str_ty = mkAnonTypeNode(TT_Data);
        str_ty->typeName = "Str";
        auto *fn = c->getCastFn(val->type.get(), str_ty);

        if(!fn){
            delete ls;
            delete rs;
            return c->compErr("Cannot cast " + typeNodeToColoredStr(val->type.get())
                + " to Str for string interpolation.", valNode->loc);
        }

        val = new TypedValue(c->builder.CreateCall(fn->val, val->val), str_ty);
    }

    //Finally, the interpolation is done.  Now just combine the three strings
    //get the ++_Str_Str function
    string appendFn = "++";
    string mangledAppendFn = "++_Str_Str";
    auto *fn = c->getFunction(appendFn, mangledAppendFn);
    if(!fn) return c->compErr("++ overload for Str and Str not found while performing Str interpolation.  The prelude may not be imported correctly.", sln->loc);

    //call the ++ function to combine the three strings
    auto *lstr = ls->compile(c);
    auto *appendL = c->builder.CreateCall(fn->val, vector<Value*>{lstr->val, val->val});

    auto *rstr = rs->compile(c);
    auto *appendR = c->builder.CreateCall(fn->val, vector<Value*>{appendL, rstr->val});

    //create the returning typenode
    auto *strty = mkAnonTypeNode(TT_Data);
    strty->typeName = "Str";

    delete lstr;
    delete rstr;
    delete val;
    return new TypedValue(appendR, strty);
}


TypedValue* StrLitNode::compile(Compiler *c){
    auto idx = val.find("${");

    if(idx != string::npos && (idx == 0 or val.find("\\${") != idx - 1))
        return compStrInterpolation(c, this, idx);

    TypeNode *strty = mkAnonTypeNode(TT_Data);
    strty->typeName = "Str";

    auto *ptr = c->builder.CreateGlobalStringPtr(val);

	//get the llvm Str data type from a fake type node in case we are compiling
	//the prelude and the Str data type isnt translated into an llvmty yet
    TypeNode *str_tn = mkDataTypeNode("Str");
    auto *tupleTy = cast<StructType>(c->typeNodeToLlvmType(str_tn));
    delete str_tn;
	
	vector<Constant*> strarr = {
		UndefValue::get(Type::getInt8PtrTy(*c->ctxt)),
		ConstantInt::get(*c->ctxt, APInt(32, val.length(), true))
	};

    auto *uninitStr = ConstantStruct::get(tupleTy, strarr);
    auto *str = c->builder.CreateInsertValue(uninitStr, ptr, 0);

    return new TypedValue(str, strty);
}

TypedValue* CharLitNode::compile(Compiler *c){
    return new TypedValue(ConstantInt::get(*c->ctxt, APInt(8, val, true)), mkAnonTypeNode(TT_C8));
}


TypedValue* ArrayNode::compile(Compiler *c){
    vector<Constant*> arr;
    TypeNode *tyn = mkAnonTypeNode(TT_Array);

    int i = 1;
    for(auto& n : exprs){
        auto *tval = n->compile(c);

        arr.push_back((Constant*)tval->val);

        if(!tyn->extTy.get()){
            tyn->extTy.reset(tval->type.get());
        }else{
            if(!c->typeEq(tval->type.get(), tyn->extTy.get()))
                return c->compErr("Element " + to_string(i) + "'s type " + typeNodeToColoredStr(tval->type) +
                        " does not match the first element's type of " + typeNodeToColoredStr(tyn->extTy), n->loc);
        }
        i++;
    }

    tyn->extTy->next.reset(new IntLitNode(tyn->loc, to_string(exprs.size()), TT_U32));

    auto *ty = ArrayType::get(arr[0]->getType(), exprs.size());
    auto *val = ConstantArray::get(ty, arr);
    return new TypedValue(val, tyn);
}

/*
 *  Return a void literal.
 *
 *  Llvm does not have a void value to use, so an undef value is
 *  returned as the typedvalue's val instead.  The val itself is
 *  unimportant so long as it is both a no-op and is able to be
 *  properly dyn_casted (so a nullptr is out of question)
 */
TypedValue* Compiler::getVoidLiteral(){
    return new TypedValue(
            UndefValue::get(Type::getInt8Ty(*ctxt)),
            mkAnonTypeNode(TT_Void)
    );
}

TypedValue* TupleNode::compile(Compiler *c){
    vector<Constant*> elems;
    elems.reserve(exprs.size());

    vector<Type*> elemTys;
    elemTys.reserve(exprs.size());

    map<unsigned, Value*> pathogenVals;
    TypeNode *tyn = mkAnonTypeNode(TT_Tuple);

    TypeNode *cur = 0;

    //Compile every value in the tuple, and if it is not constant,
    //add it to pathogenVals
    for(unsigned i = 0; i < exprs.size(); i++){
        auto *tval = exprs[i]->compile(c);

        if(Constant *elem = dyn_cast<Constant>(tval->val)){
            elems.push_back(elem);
        }else{
            pathogenVals[i] = tval->val;
            elems.push_back(UndefValue::get(tval->getType()));
        }
        elemTys.push_back(tval->getType());

        if(cur){
            //cannot just do a swap here because unique_ptr<TypeNode> 
            //cannot swap with a unique_ptr<Node>
            cur->next.release();
            cur->next.reset(tval->type.get());
            tval->type.release();
            cur = (TypeNode*)cur->next.get();
        }else{
            tyn->extTy.reset(tval->type.get());
            cur = tyn->extTy.get();
        }
    }

    //Create the constant tuple with undef values in place for the non-constant values
    Value* tuple = ConstantStruct::get(StructType::get(*c->ctxt, elemTys), elems);

    //Insert each pathogen value into the tuple individually
    for(auto it = pathogenVals.cbegin(); it != pathogenVals.cend(); it++){
        tuple = c->builder.CreateInsertValue(tuple, it->second, it->first);
    }

    //A void value is represented by the empty tuple, ()
    if(exprs.size() == 0){
        tyn->type = TT_Void;
    }
   
    return new TypedValue(tuple, tyn);
}


vector<TypedValue*> TupleNode::unpack(Compiler *c){
    vector<TypedValue*> ret;
    for(auto& n : exprs){
        auto *tv = n->compile(c);

        if(tv && tv->type->type != TT_Void)
            ret.push_back(tv);
    }
    return ret;
}


/*
 *  When a retnode is compiled within a block, care must be taken to not
 *  forcibly insert the branch instruction afterwards as it leads to dead code.
 */
TypedValue* RetNode::compile(Compiler *c){
    TypedValue *ret = expr->compile(c);
    
    auto *retInst = ret->type->type == TT_Void ?
                 new TypedValue(c->builder.CreateRetVoid(), ret->type) :
                 new TypedValue(c->builder.CreateRet(ret->val), ret->type);

    auto *f = c->getCurrentFunction();
    f->returns.push_back({retInst, expr->loc});
    return retInst;
}


TypedValue* ImportNode::compile(Compiler *c){
    if(!dynamic_cast<StrLitNode*>(expr.get())) return 0;

    c->importFile(((StrLitNode*)expr.get())->val.c_str(), this);
    return c->getVoidLiteral();
}


TypedValue* WhileNode::compile(Compiler *c){
    Function *f = c->builder.GetInsertBlock()->getParent();
    BasicBlock *cond  = BasicBlock::Create(*c->ctxt, "while_cond", f);
    BasicBlock *begin = BasicBlock::Create(*c->ctxt, "while", f);
    BasicBlock *end   = BasicBlock::Create(*c->ctxt, "end_while", f);

    c->builder.CreateBr(cond);
    c->builder.SetInsertPoint(cond);
    auto *condval = condition->compile(c);

    c->builder.CreateCondBr(condval->val, begin, end);

    c->builder.SetInsertPoint(begin);

    c->compCtxt->breakLabels->push_back(end);
    c->compCtxt->continueLabels->push_back(cond);

    auto *val = child->compile(c); //compile the while loop's body

    c->compCtxt->breakLabels->pop_back();
    c->compCtxt->continueLabels->pop_back();

    if(!dyn_cast<ReturnInst>(val->val) and !dyn_cast<BranchInst>(val->val))
        c->builder.CreateBr(cond);

    c->builder.SetInsertPoint(end);
    return c->getVoidLiteral();
}


TypedValue* ForNode::compile(Compiler *c){
    Function *f = c->builder.GetInsertBlock()->getParent();
    BasicBlock *cond  = BasicBlock::Create(*c->ctxt, "for_cond", f);
    BasicBlock *begin = BasicBlock::Create(*c->ctxt, "for", f);
    BasicBlock *incr = BasicBlock::Create(*c->ctxt, "for_incr", f);
    BasicBlock *end   = BasicBlock::Create(*c->ctxt, "end_for", f);


    auto *rangev = range->compile(c);

    //check if the range expression is its own iterator and thus implements Iterator
    //If it does not, see if it implements Iterable by attempting to call into_iter on it
    auto *dt = c->lookupType(typeNodeToStr(rangev->type.get()));
    if(!dt or (dt and !c->typeImplementsTrait(dt, "Iterator"))){
        auto *res = c->callFn("into_iter", {rangev});

        if(!res)
            return c->compErr("Range expression of type " + typeNodeToColoredStr(rangev->type) + " needs to implement " +
                typeNodeToColoredStr(mkDataTypeNode("Iterable")) + " or " + typeNodeToColoredStr(mkDataTypeNode("Iterator")) +
                " to be used in a for loop", range->loc);

        rangev = res;
    }

    //by this point, rangev now properly stores the range information, so store it on the stack and insert calls to
    //unwrap, has_next, and next at the beginning, beginning, and end of the loop respectively.
    Value *alloca = c->builder.CreateAlloca(rangev->getType());
    c->builder.CreateStore(rangev->val, alloca);

    c->builder.CreateBr(cond);
    c->builder.SetInsertPoint(cond);

    //set var = unwrap range

    //candval = is_done range
    auto *rangeVal = new TypedValue(c->builder.CreateLoad(alloca), rangev->type.get());
    auto *is_done = c->callFn("has_next", {rangeVal});
    if(!is_done) return c->compErr("Range expression of type " + typeNodeToColoredStr(rangev->type) + " does not implement " +
            typeNodeToColoredStr(mkDataTypeNode("Iterable")) + ", which it needs to be used in a for loop", range->loc);

    c->builder.CreateCondBr(is_done->val, begin, end);

    c->builder.SetInsertPoint(begin);

    //call unwrap at start of loop
    //make sure to update rangeVal
    rangeVal = new TypedValue(c->builder.CreateLoad(alloca), rangev->type.get());
    auto *uwrap = c->callFn("unwrap", {rangeVal});
    if(!uwrap) return c->compErr("Range expression of type " + typeNodeToColoredStr(rangev->type) + " does not implement " +
            typeNodeToColoredStr(mkDataTypeNode("Iterable")) + ", which it needs to be used in a for loop", range->loc);

    auto *uwrap_var = new Variable(var, uwrap, c->scope);
    c->stoVar(var, uwrap_var);
 
    
    //register the branches to break/continue to right before the body is compiled in case there was an error compiling the range
    c->compCtxt->breakLabels->push_back(end);
    c->compCtxt->continueLabels->push_back(incr);

    //compile the rest of the loop's body
    auto *val = child->compile(c);
    
    c->compCtxt->breakLabels->pop_back();;
    c->compCtxt->continueLabels->pop_back();

    if(!val) return 0;
    if(!dyn_cast<ReturnInst>(val->val) and !dyn_cast<BranchInst>(val->val)){
        //set range = next range
        c->builder.CreateBr(incr);
        c->builder.SetInsertPoint(incr);

        auto *next = c->callFn("next", {new TypedValue(c->builder.CreateLoad(alloca), rangev->type.get())});
        if(!next) return c->compErr("Range expression of type " + typeNodeToColoredStr(rangev->type) + " does not implement " + typeNodeToColoredStr(mkDataTypeNode("Iterable")) +
                ", which it needs to be used in a for loop", range->loc);
    
        c->builder.CreateStore(next->val, alloca);
        c->builder.CreateBr(cond);
    }

    c->builder.SetInsertPoint(end);
    return c->getVoidLiteral();
}


TypedValue* JumpNode::compile(Compiler *c){
    auto *e = expr->compile(c);
    auto *ci = dyn_cast<ConstantInt>(e->val);
    if(!ci)
        return c->compErr("Expression must evaluate to a constant integer\n", expr->loc);

    if(!isUnsignedTypeTag(e->type->type) and ci->getSExtValue() < 0)
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
    return new TypedValue(br, mkAnonTypeNode(TT_Void));
}


//create a new scope if the user indents
TypedValue* BlockNode::compile(Compiler *c){
    c->enterNewScope();
    TypedValue *ret = block->compile(c);
    c->exitScope();
    return ret;
}


//Since parameters are managed in Compiler::compfn, this need not do anything
TypedValue* NamedValNode::compile(Compiler *c)
{ return nullptr; }


/*
 *  Loads a variable from the stack
 */
TypedValue* VarNode::compile(Compiler *c){
    auto *var = c->lookup(name);

    if(var){
        return var->autoDeref ?
            new TypedValue(c->builder.CreateLoad(var->getVal(), name), var->tval->type):
            new TypedValue(var->tval->val, var->tval->type); //deep copy type
    }else{
        //if this is a function, then there must be only one function of the same name, otherwise the reference is ambiguous
        auto& fnlist = c->getFunctionList(name);

        if(fnlist.size() == 1){
            auto& fd = *fnlist.begin();
            if(!fd->tv)
                fd->tv = c->compFn(fd.get());

            if(!fd or !fd->tv)
                return 0;

            return new TypedValue(fd->tv->val, fd->tv->type);
        }else if(fnlist.empty()){
            return c->compErr("Variable or function '" + name + "' has not been declared.", this->loc);
        }else{
            return c->compErr("Too many candidates for function '" + name + "' to reduce to a single instance", this->loc);
        }
    }
}


TypedValue* LetBindingNode::compile(Compiler *c){
    TypedValue *val = expr->compile(c);
    if(val->type->type == TT_Void)
        return c->compErr("Cannot assign a "+typeNodeToColoredStr(mkAnonTypeNode(TT_Void))+
                " value to a variable", expr->loc);

    TypeNode *tyNode;
    if((tyNode = (TypeNode*)typeExpr.get())){
        if(!llvmTypeEq(val->val->getType(), c->typeNodeToLlvmType(tyNode))){
            return c->compErr("Incompatible types in explicit binding.", expr->loc);
        }
    }

    bool isGlobal = false;

    //add the modifiers to the typedvalue
    Node *mod = modifiers.get();
    while(mod){
        int m = ((ModNode*)mod)->mod;
        val->type->addModifier(m);
        if(m == Tok_Global) isGlobal = true;
        mod = mod->next.get();
    }

    if(isGlobal){
        auto *ty = c->typeNodeToLlvmType(val->type.get());
        auto *global = new GlobalVariable(*c->module, ty, false, GlobalValue::PrivateLinkage, UndefValue::get(ty), this->name);
        c->builder.CreateStore(val->val, global);
        val->val = global;
    }

    if(val->getType()->isArrayTy() and not isGlobal){
        Value *alloca = c->builder.CreateAlloca(val->getType(), nullptr, name.c_str());
        c->builder.CreateStore(val->val, alloca);
        val->val = alloca;
        isGlobal = true;
    }

    c->stoVar(name, new Variable(name, val, c->scope, true, isGlobal));
    return val;
}

TypedValue* compVarDeclWithInferredType(VarDeclNode *node, Compiler *c){
    TypedValue *val = node->expr->compile(c);
    if(val->type->type == TT_Void)
        return c->compErr("Cannot assign a "+typeNodeToColoredStr(mkAnonTypeNode(TT_Void))+
                " value to a variable", node->expr->loc);

    bool isGlobal = false;
    //Add all of the declared modifiers to the typedval
    Node *mod = node->modifiers.get();
    while(mod){
        int m = ((ModNode*)mod)->mod;
        val->type->addModifier(m);
        if(m == Tok_Global) isGlobal = true;
        mod = mod->next.get();
    }

    //set the value as mutable
    if(!val->type->hasModifier(Tok_Mut))
        val->type->addModifier(Tok_Mut);
    
    //location to store var
    Value *ptr = isGlobal ?
            (Value*) new GlobalVariable(*c->module, val->getType(), false, GlobalValue::PrivateLinkage, UndefValue::get(val->getType()), node->name) :
            c->builder.CreateAlloca(val->getType(), nullptr, node->name.c_str());

    TypedValue *alloca = new TypedValue(ptr, val->type);

    bool nofree = true;//val->type->type != TT_Ptr || dynamic_cast<Constant*>(val->val);
    c->stoVar(node->name, new Variable(node->name, alloca, c->scope, nofree, true));

    return new TypedValue(c->builder.CreateStore(val->val, alloca->val), val->type);
}

TypedValue* VarDeclNode::compile(Compiler *c){
    //check for redeclaration, but only on topmost scope
    Variable *redeclare;
    try{
        redeclare = c->varTable.back()->at(this->name);
    }catch(out_of_range r){
        redeclare = 0;
    }

    if(redeclare)
        return c->compErr("Variable " + name + " was redeclared.", this->loc);

    //check for an inferred type
    TypeNode *tyNode = (TypeNode*)typeExpr.get();
    if(!tyNode) return compVarDeclWithInferredType(this, c);

    if(tyNode->type == TT_Void)
        return c->compErr("Cannot create a variable of type "+
                typeNodeToColoredStr(mkAnonTypeNode(TT_Void)), tyNode->loc);


    //the type held by this node will be deleted when the parse tree is, so copy
    //this one so it is not double freed
    tyNode = copy(tyNode);

    Type *ty = c->typeNodeToLlvmType(tyNode);

    bool isGlobal = false;
    //Add all of the declared modifiers to the typedval
    Node *mod = modifiers.get();
    while(mod){
        int m = ((ModNode*)mod)->mod;
        tyNode->addModifier(m);
        if(m == Tok_Global) isGlobal = true;
        mod = mod->next.get();
    }

    if(!tyNode->hasModifier(Tok_Mut))
        tyNode->addModifier(Tok_Mut);

    //location to store var
    Value *loc = isGlobal ?
        (Value*) new GlobalVariable(*c->module, ty, false, GlobalValue::PrivateLinkage, UndefValue::get(ty), name) :
        c->builder.CreateAlloca(ty, nullptr, name.c_str());

    TypedValue *alloca = new TypedValue(loc, tyNode);

    Variable *var = new Variable(name, alloca, c->scope, true, true);
    c->stoVar(name, var);
    if(expr.get()){
        TypedValue *val = expr->compile(c);
        if(val->type->type == TT_Void)
            return c->compErr("Cannot assign a "+typeNodeToColoredStr(mkAnonTypeNode(TT_Void))+
                    " value to a variable", expr->loc);

        TypeNode *exprTy = copy(val->type);
        exprTy->addModifier(Tok_Mut);
        var->noFree = true;//var->getType() != TT_Ptr || dynamic_cast<Constant*>(val->val);

        //Make sure the assigned value matches the variable's type
        if(!c->typeEq(alloca->type->extTy.get(), exprTy)){
            return c->compErr("Cannot assign expression of type " + typeNodeToColoredStr(val->type)
                        + " to a variable of type " + typeNodeToColoredStr(alloca->type->extTy), expr->loc);
        }

        //transfer ownership of val->type
        return new TypedValue(c->builder.CreateStore(val->val, alloca->val), exprTy);
    }else{
        return alloca;
    }
}

/*
 *  Simple wrapper function for compInsert to insert into a named field
 *  instead of an index
 */
TypedValue* compFieldInsert(Compiler *c, BinOpNode *bop, Node *expr){
    VarNode *field = static_cast<VarNode*>(bop->rval.get());

    //A . operator can also have a type/module as its lval, but its
    //impossible to insert into a non-value so fail if the lvalue is one
    if(auto *tn = dynamic_cast<TypeNode*>(bop->lval.get()))
        return c->compErr("Cannot insert value into static module '" + typeNodeToColoredStr(tn), tn->loc);

   
    Value *val;
    TypeNode *tyn;
    TypeNode *ltyn;

    //prevent l from being used after this scope; only val and tyn should be used as only they
    //are updated with the automatic pointer dereferences.
    { 
        auto *l = bop->lval->compile(c);

        val = l->val;
        tyn = ltyn = l->type.get();
       
        if(!tyn->hasModifier(Tok_Mut))
            return c->compErr("Variable must be mutable to be assigned to, but instead is an immutable " +
                    typeNodeToColoredStr(tyn), bop->loc);
    }

    //the . operator automatically dereferences pointers, so update val and tyn accordingly.
    while(tyn->type == TT_Ptr){
        val = c->builder.CreateLoad(val);
        tyn = tyn->extTy.get();
    }
    
    //if pointer derefs took place, tyn could have lost its modifiers, so make sure they are copied back
    if(ltyn->type == TT_Ptr and tyn->modifiers.empty())
        tyn->copyModifiersFrom(ltyn);

    //this is the variable that will store the changes after the later insertion
    Value *var = static_cast<LoadInst*>(val)->getPointerOperand();

    //check to see if this is a field index
    if(tyn->type == TT_Data || tyn->type == TT_Tuple){
        auto dataTy = c->lookupType(typeNodeToStr(tyn));

        if(dataTy){
            auto index = dataTy->getFieldIndex(field->name);

            if(index != -1){
                TypeNode *indexTy = (TypeNode*)getNthNode(dataTy->tyn->extTy.get(), index);

                auto *newval = expr->compile(c);

                //see if insert operator # = is overloaded already
                string op = "#";
                string mangledfn = mangle(op, tyn, mkAnonTypeNode(TT_I32), newval->type.get());
                auto *fn = c->getFunction(op, mangledfn);
                if(fn){
                    return new TypedValue(c->builder.CreateCall(fn->val, vector<Value*>{var, c->builder.getInt32(index), newval->val}), fn->type->extTy);
                }

                //if not, proceed with normal operations
                if(!c->typeEq(indexTy, newval->type.get()))
                    return c->compErr("Cannot assign expression of type " + typeNodeToColoredStr(newval->type.get()) +
                           " to a variable of type " + typeNodeToColoredStr(indexTy), expr->loc);

				Value *nv = newval->val;
				Type *nt = val->getType()->getStructElementType(index);

				//Type check may succeed if a void* is being inserted into any ptr slot,
				//but llvm will still complain so we create a bit cast to appease it
				if (nv->getType() != nt and newval->type->type == TT_Ptr) {
					nv = c->builder.CreateBitCast(nv, nt);
				}

                auto *ins = c->builder.CreateInsertValue(val, nv, index);

                c->builder.CreateStore(ins, var);
                return c->getVoidLiteral();
            }
        }
    }

    return c->compErr("Method/Field " + field->name + " not found in type " + typeNodeToColoredStr(tyn), bop->loc);
}

TypedValue* VarAssignNode::compile(Compiler *c){
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
    TypedValue *tmp = ref_expr->compile(c);

    //if(!dynamic_cast<LoadInst*>(tmp->val))
    if(!tmp->hasModifier(Tok_Mut))
        return c->compErr("Variable must be mutable to be assigned to, but instead is an immutable " +
                typeNodeToColoredStr(tmp->type), ref_expr->loc);
    
    Value *dest = ((LoadInst*)tmp->val)->getPointerOperand();
    
    //compile the expression to store
    TypedValue *assignExpr = expr->compile(c);

    //lvalue must compile to a pointer for storage, usually an alloca value
    if(!PointerType::isLoadableOrStorableType(tmp->getType())){
        return c->compErr("Attempted assign without a memory address, with type "
                + typeNodeToColoredStr(tmp->type), ref_expr->loc);
    }

    //and finally, make sure the assigned value matches the variable's type
    if(!c->typeEq(tmp->type.get(), assignExpr->type.get())){
        tmp->dump();
        assignExpr->dump();
        return c->compErr("Cannot assign expression of type " + typeNodeToColoredStr(assignExpr->type)
                    + " to a variable of type " + typeNodeToColoredStr(tmp->type), expr->loc);
    }

    //now actually create the store
    c->builder.CreateStore(assignExpr->val, dest);

    //all assignments return a void value
    return c->getVoidLiteral();
}

TypedValue* PreProcNode::compile(Compiler *c){
    return c->getVoidLiteral();
}


string mangle(string &base, vector<TypeNode*> params){
    string name = base;
    for(auto *tv : params){
        if(tv->type != TT_Void)
            name += "_" + typeNodeToStr(tv);
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

//provide common mangle shortcuts.  Useful for checking
//for operator overloads without needing to remove each Node->next
//of each TypeNode of the op arguments
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

//Given a list of FuncDeclNodes, returns the function whose name
//matches basename, or returns nullptr if not found.
FuncDeclNode* findFDN(Node *n, string& basename){
    while(n){
        auto *fdn = (FuncDeclNode*)n;
        
        if(fdn->basename == basename){
            return fdn;
        }

        n = n->next.get();
    }
    return nullptr;
}

TypedValue* ExtNode::compile(Compiler *c){
    if(traits.get()){
        //this ExtNode is an implementation of a trait
        string typestr = typeNodeToStr(typeExpr.get());
        DataType *dt;

        if(typeExpr->typeName.empty()){ //primitive type being extended
            dt = c->lookupType(typestr);
            if(!dt){ //if primitive type has not been extended before, make it a DataType to store in
                dt = new DataType(typestr, {/*no fields*/}, copy(typeExpr));
                c->stoType(dt, typestr);
            }
        }else{
            dt = c->lookupType(typestr);
            if(!dt)
                return c->compErr("Cannot implement traits for undeclared type " + typeNodeToColoredStr(typeExpr), typeExpr->loc);
        }

        //create a vector of the traits that must be implemented
        TypeNode *curTrait = this->traits.get();
        vector<Trait*> traits;
        while(curTrait){
            string traitstr = typeNodeToStr(curTrait);
            auto *trait = c->lookupTrait(traitstr);
            if(!trait)
                return c->compErr("Trait " + typeNodeToColoredStr(curTrait) + " is undeclared", curTrait->loc);

            traits.push_back(trait);
            curTrait = (TypeNode*)curTrait->next.get();
        }

        //go through each trait and compile the methods for it
        auto *funcs = methods.get();
        for(auto& trait : traits){
            auto *traitImpl = new Trait();
            traitImpl->name = trait->name;

            for(auto& fd_proto : trait->funcs){
                auto *fdn = findFDN(funcs, fd_proto->fdn->basename);

                if(!fdn)
                    return c->compErr(typeNodeToColoredStr(typeExpr) + " must implement " + fd_proto->fdn->basename +
                            " to implement " + typeNodeToColoredStr(mkDataTypeNode(trait->name)), fd_proto->fdn->loc);

                fdn->name = c->funcPrefix + fdn->name;
                fdn->basename = c->funcPrefix + fdn->basename;
                
                shared_ptr<FuncDecl> fd{new FuncDecl(fdn, c->scope, c->compUnit)};
                traitImpl->funcs.push_back(fd);
    
                c->compUnit->fnDecls[fdn->basename].push_front(fd);
                c->mergedCompUnits->fnDecls[fdn->basename].push_front(fd);
            }

            //trait is fully implemented, add it to the DataType
            dt->traitImpls.push_back(shared_ptr<Trait>(traitImpl));
        }
    }else{
        //this ExtNode is not a trait implementation, so just compile all functions normally
        string oldPrefix = c->funcPrefix;

        //Temporarily move away any type params so we get Vec.remove not Vec<'t>.remove as the fn name
        auto params = move(typeExpr->params);
        c->funcPrefix = typeNodeToStr(typeExpr.get()) + "_";
        typeExpr->params = move(params);

        auto prevObj = c->compCtxt->obj;
        c->compCtxt->obj = typeExpr.get();

        compileStmtList(methods.release(), c);

        c->funcPrefix = oldPrefix;
        c->compCtxt->obj = prevObj;
    }
    return c->getVoidLiteral();
}
        
bool Compiler::typeImplementsTrait(DataType* dt, string traitName) const{
    for(auto& tr : dt->traitImpls)
        if(tr->name == traitName)
            return true;
    return false;
}


TypedValue* compTaggedUnion(Compiler *c, DataDeclNode *n){
    vector<string> fieldNames;
    fieldNames.reserve(n->fields);

    auto *nvn = (NamedValNode*)n->child.get();

    vector<string> union_name;
    union_name.push_back(n->name);

    vector<shared_ptr<UnionTag>> tags;
    
    //hotfix for generic types: bind each generic to void and retranslate when a concrete type is given
    //TODO: implement fully lazy translation
    TypeNode *voidTy = mkAnonTypeNode(TT_Void);    
    if(!n->generics.empty()){
        for(auto &tn : n->generics){
            c->stoTypeVar(tn->typeName, voidTy);
        }
    }

    TypeNode *unionTy = mkAnonTypeNode(TT_TaggedUnion);
    TypeNode *curExt = 0;

    while(nvn){
        TypeNode *tyn = (TypeNode*)nvn->typeExpr.get();
        UnionTag *tag = new UnionTag(nvn->name, copy(tyn->extTy), tags.size());

        tags.push_back(shared_ptr<UnionTag>(tag));

        //Each union member's type is a tuple of the tag, a u8 value, and the user-defined value
        TypeNode *tagTy = tyn->extTy.get();
        TypeNode *variant = mkAnonTypeNode(TT_U8);
        variant->next.reset(tagTy);
        TypeNode *tup = mkTypeNodeWithExt(TT_Tuple, variant);

        DataType *data = new DataType(nvn->name, union_name, copy(tagTy));
        c->stoType(data, nvn->name);

        TypeNode *tupCpy = copy(tup);
        if(curExt){
            curExt->next.reset(tupCpy);
        }else{
            unionTy->extTy.reset(tupCpy);
        }
        curExt = tupCpy;
        variant->next.release();
        delete tup;

        try{
            if(tagTy) tagTy->getSizeInBits(c, &n->name);
        }catch(IncompleteTypeError *e){
            delete e;
            return c->compErr("Cannot define a recursive type", tyn->loc);
        }catch(TypeVarError *e){
            delete e;
            return c->compErr("Typevar not bound in expression", tyn->loc);
        }

        nvn = (NamedValNode*)nvn->next.get();
    }

    unionTy->typeName = n->name;
    DataType *data = new DataType(n->name, fieldNames, unionTy);
    for(auto &g : n->generics)
        data->generics.push_back(shared_ptr<TypeNode>(g.release()));
    
    data->tags.swap(tags); 

    c->stoType(data, n->name);
    return c->getVoidLiteral();
}


TypedValue* DataDeclNode::compile(Compiler *c){
    auto *dt = c->lookupType(this->name);
    if(dt) return c->compErr("Type " + name + " was redefined", loc);
    
    auto *nvn = (NamedValNode*)child.get();
    if(((TypeNode*) nvn->typeExpr.get())->type == TT_TaggedUnion){
        return compTaggedUnion(c, this);
    }


    vector<string> fieldNames;
    vector<Type*> fieldTypes;

    fieldNames.reserve(fields);
    fieldTypes.reserve(fields);

    TypeNode *first = 0;
    TypeNode *nxt = 0;

    //hotfix for generic types: bind each generic to void and retranslate when a concrete type is given
    //TODO: implement fully lazy translation
    TypeNode *voidTy = mkAnonTypeNode(TT_Void);    
    if(!generics.empty()){
        for(auto &tn : generics){
            c->stoTypeVar(tn->typeName, voidTy);
        }
    }

   
    while(nvn){
        TypeNode *tyn = (TypeNode*)nvn->typeExpr.get();
  
        try{
            tyn->getSizeInBits(c, &name);
        }catch(IncompleteTypeError *e){
            delete e;
            return c->compErr("Cannot define a recursive type", tyn->loc);
        }catch(TypeVarError *e){
            delete e;
            return c->compErr("Typevar not bound in expression", tyn->loc);
        }

        if(first){
            nxt->next.reset(copy(tyn));
            nxt = (TypeNode*)nxt->next.get();
        }else{
            first = copy(tyn);
            nxt = first;
        }

        fieldNames.push_back(nvn->name);
        fieldTypes.push_back(c->typeNodeToLlvmType(tyn));

        nvn = (NamedValNode*)nvn->next.get();
    }

    //the type is a tuple if it has multiple params,
    //otherwise it is just a normal type
    TypeNode *dataTyn = first->next.get()
                      ? mkTypeNodeWithExt(TT_Tuple, first)
                      : first;

    DataType *data = new DataType(name, fieldNames, dataTyn);
    for(auto &g : generics)
        data->generics.push_back(shared_ptr<TypeNode>(g.release()));

    c->stoType(data, name);
    return c->getVoidLiteral();
}


TypedValue* TraitNode::compile(Compiler *c){
    auto *trait = new Trait();
    trait->name = name;
    
    auto *curfn = child.release();
    while(curfn){
        auto *fn = (FuncDeclNode*)curfn;
        fn->name = c->funcPrefix + fn->name;
        fn->basename = c->funcPrefix + fn->basename;
        
        shared_ptr<FuncDecl> fd{new FuncDecl(fn, c->scope, c->compUnit)};
        trait->funcs.push_back(fd);
        curfn = curfn->next.get();
    }

    auto traitPtr = shared_ptr<Trait>(trait);
    c->compUnit->traits[name] = traitPtr;
    c->mergedCompUnits->traits[name] = traitPtr;

    return c->getVoidLiteral();
}


TypedValue* GlobalNode::compile(Compiler *c){
    TypedValue *ret = 0;
    for(auto &varName : vars){
        auto oldFnScope = c->fnScope;
        c->fnScope = 1;
        
        Variable *var;
        for(auto i = c->varTable.size(); i >= c->fnScope; --i){
            try{
                var = c->varTable[i-1]->at(varName->name);
            }catch(out_of_range r){
                var = nullptr;
            }
        }

        c->fnScope = oldFnScope;

        if(!var)
            return c->compErr("Variable '" + varName->name + "' has not been declared.", varName->loc);

        if(!var->tval->type->hasModifier(Tok_Global))
            return c->compErr("Variable " + varName->name + " must be global to be imported.", varName->loc);

        var->scope = c->scope;
        c->stoVar(varName->name, var);
        ret = var->tval.get();
    }

    return new TypedValue(c->builder.CreateLoad(ret->val), ret->type);
}


TypedValue* MatchNode::compile(Compiler *c){
    auto *lval = expr->compile(c);

    if(lval->type->type != TT_TaggedUnion && lval->type->type != TT_Data){
        return c->compErr("Cannot match expression of type " + typeNodeToColoredStr(lval->type) +
                ".  Match expressions must be a tagged union type", expr->loc);
    }


    //the tag is always the zero-th index except for in certain optimization cases and if
    //the tagged union has no tagged values and is equivalent to an enum in C-like languages.
    Value *switchVal = llvmTypeToTypeTag(lval->getType()) == TT_Tuple ?
            c->builder.CreateExtractValue(lval->val, 0)
            : lval->val;

    Function *f = c->builder.GetInsertBlock()->getParent();
    auto *matchbb = c->builder.GetInsertBlock();

    auto *end = BasicBlock::Create(*c->ctxt, "end_match");
    auto *match = c->builder.CreateSwitch(switchVal, end, branches.size());
    vector<pair<BasicBlock*,TypedValue*>> merges;

    for(auto& mbn : branches){
        ConstantInt *ci = nullptr;
        auto *br = BasicBlock::Create(*c->ctxt, "br", f);
        c->builder.SetInsertPoint(br);
        c->enterNewScope();

        //TypeCast-esque pattern:  Some n
        if(TypeCastNode *tn = dynamic_cast<TypeCastNode*>(mbn->pattern.get())){
            auto *tagTy = c->lookupType(tn->typeExpr->typeName);
            if(!tagTy)
                return c->compErr("Union tag " + typeNodeToColoredStr(tn->typeExpr) + " was not yet declared.", tn->typeExpr->loc);

            if(!tagTy->isUnionTag())
                return c->compErr(typeNodeToColoredStr(tn->typeExpr) + " must be a union tag to be used in a pattern", tn->typeExpr->loc);

            auto *parentTy = c->lookupType(tagTy->getParentUnionName());
            ci = ConstantInt::get(*c->ctxt, APInt(8, parentTy->getTagVal(tn->typeExpr->typeName), true));


            if(VarNode *v = dynamic_cast<VarNode*>(tn->rval.get())){
                auto *alloca = c->builder.CreateAlloca(lval->getType());
                c->builder.CreateStore(lval->val, alloca);

                //If this is a generic type cast like Some 't, the 't must be bound to a concrete type first
                auto *tagtycpy = copy(tagTy->tyn);
                
                auto *structty = mkTypeNodeWithExt(lval->type->type, mkAnonTypeNode(TT_U8));
                structty->typeName = lval->type->typeName;
                structty->extTy->next.reset(tagtycpy);

                bindGenericToType(structty, lval->type->params);

                auto tcr = c->typeEq(structty, lval->type.get());
                if(tcr.res == TypeCheckResult::SuccessWithTypeVars)
                    bindGenericToType(tagtycpy, tcr.bindings);
                else if(tcr.res == TypeCheckResult::Failure)
                    return c->compErr("Cannot bind pattern of type " + typeNodeToColoredStr(structty) +
                            " to matched value of type " + typeNodeToColoredStr(lval->type), tn->rval->loc);

                structty->extTy->next.release();
                delete structty;

                //cast it from (<tag type>, <largest union member type>) to (<tag type>, <this union member's type>)
                auto *tupTy = StructType::get(*c->ctxt, {Type::getInt8Ty(*c->ctxt), c->typeNodeToLlvmType(tagtycpy)});

                auto *cast = c->builder.CreateBitCast(alloca, tupTy->getPointerTo());
                auto *tup = c->builder.CreateLoad(cast);
                auto *extract = new TypedValue(c->builder.CreateExtractValue(tup, 1), tagtycpy);
                c->stoVar(v->name, new Variable(v->name, extract, c->scope));
            }else{
                return c->compErr("pattern typecast's rval is not a identifier", tn->rval->loc);
            }

        //single type pattern:  None
        }else if(TypeNode *tn = dynamic_cast<TypeNode*>(mbn->pattern.get())){
            auto *tagTy = c->lookupType(tn->typeName);
            if(!tagTy)
                return c->compErr("Union tag " + typeNodeToColoredStr(tn) + " was not yet declared.", tn->loc);
       
            if(!tagTy->isUnionTag())
                return c->compErr(typeNodeToColoredStr(tn) + " must be a union tag to be used in a pattern", tn->loc);

            auto *parentTy = c->lookupType(tagTy->getParentUnionName());
            ci = ConstantInt::get(*c->ctxt, APInt(8, parentTy->getTagVal(tn->typeName), true));

        //variable/match-all pattern: _
        }else if(VarNode *vn = dynamic_cast<VarNode*>(mbn->pattern.get())){
            auto *tn = new TypedValue(lval->val, lval->type);
            match->setDefaultDest(br);
            c->stoVar(vn->name, new Variable(vn->name, tn, c->scope));
        }else{
            return c->compErr("Pattern matching non-tagged union types is not yet implemented", mbn->pattern->loc);
        }

        auto *then = mbn->branch->compile(c);
        c->exitScope();
        c->builder.CreateBr(end);
        merges.push_back(pair<BasicBlock*,TypedValue*>(c->builder.GetInsertBlock(), then));
      
        //if ci is still null, then the else-like branch was just compiled, and does not correlate to a singular tag
        if(ci)
            match->addCase(ci, br);
    }

    f->getBasicBlockList().push_back(end);
    c->builder.SetInsertPoint(end);

    if(!merges[0].second) return 0;

    if(merges[0].second->type->type != TT_Void){
        int i = 1;
        auto *phi = c->builder.CreatePHI(merges[0].second->getType(), branches.size());
        for(auto &pair : merges){

            //add each branch to the phi node if it does not return early
            if(!dyn_cast<ReturnInst>(pair.second->val)){

                //match the types of those branches that will merge
                if(!c->typeEq(pair.second->type.get(), merges[0].second->type.get()))
                    return c->compErr("Branch "+to_string(i)+"'s return type " + typeNodeToColoredStr(pair.second->type) +
                              " != " + typeNodeToColoredStr(merges[0].second->type) + ", the first branch's return type", this->loc);
                else
                    phi->addIncoming(pair.second->val, pair.first);
            }
            i++;
        }
        phi->addIncoming(UndefValue::get(merges[0].second->getType()), matchbb);
        return new TypedValue(phi, merges[0].second->type);
    }else{
        return c->getVoidLiteral();
    }
}


TypedValue* MatchBranchNode::compile(Compiler *c){
    return c->getVoidLiteral();
}


void ante::Module::import(shared_ptr<ante::Module> mod){
    for(auto& pair : mod->fnDecls)
        for(auto& fd : pair.second)
            fnDecls[pair.first].push_back(fd);

    for(auto& pair : mod->userTypes)
        userTypes[pair.first] = pair.second;
    
    for(auto& pair : mod->traits)
        traits[pair.first] = pair.second;
}

/*
 * imports a given ante file to the current module
 * inputted file must exist and be a valid ante source file.
 */
void Compiler::importFile(const char *fName, Node *locNode){
    try{
        auto& module = allCompiledModules->at(fName);
        string fmodName = removeFileExt(fName);

        for(auto &mod : imports){
            if(mod->name == fmodName){
                compErr("module " + string(fName) + " has already been imported", locNode->loc, ErrorType::Warning);
                return;
            }
        }

        //module is already compiled; just copy the ptr to imports
        imports.push_back(module);
        mergedCompUnits->import(module);

    }catch(out_of_range r){
        //module not found; create new Compiler instance to compile it
        Compiler *c = new Compiler(fName, true, ctxt);
        c->allCompiledModules = allCompiledModules;
        c->compilePrelude();
        c->scanAllDecls();

        if(c->errFlag){
            cout << "Error when importing " << fName << endl;
            errFlag = true;
            return;
        }

        imports.push_back(c->compUnit);
        mergedCompUnits->import(c->compUnit);

        (*allCompiledModules)[fName] = c->compUnit;
        //delete c;
    }
}


TypeNode* mkAnonTypeNode(TypeTag t){
    auto fakeLoc = mkLoc(mkPos(0, 0, 0), mkPos(0, 0, 0));
    return new TypeNode(fakeLoc, t, "", nullptr);
}

TypeNode* mkTypeNodeWithExt(TypeTag tt, TypeNode *ext){
    auto *p = mkAnonTypeNode(tt);
    p->extTy.reset(ext);
    return p;
}

TypeNode* mkDataTypeNode(string tyname){
    auto *d = mkAnonTypeNode(TT_Data);
    d->typeName = tyname;
    return d;
}


/*
 *  Declares functions to be included in every module without need of an import.
 *  These are registered but not compiled until they are called so that they
 *  do not pollute the module with unused definitions.
 */
void Compiler::compilePrelude(){
    if(fileName != AN_LIB_DIR "prelude.an"){
        importFile(AN_LIB_DIR "prelude.an");
    }
}


/*
 *  Removes .an from a source file to get its module name
 */
string removeFileExt(string file){
    auto index = file.find_last_of('.');
    return index == string::npos ? file : file.substr(0, index);
}


/*
 * Creates a placeholder node that will not generate any code
 * if its compile method is called.
 *
 * Used for filling in gaps after parse tree modifications
 */
Node* mkPlaceholderNode(){
    auto* empty = new string("");

    auto fakeLoc = mkLoc(mkPos(empty, 0, 0), mkPos(empty, 0, 0));
    
    return new IntLitNode(fakeLoc, "0", TT_U8);
}

/*
 *  Sweeps through entire parse tree registering all function and data
 *  declarations.  Removes compiled functions.
 */
void Compiler::scanAllDecls(RootNode *root){
    auto *n = root ? root : ast.get();

	for (auto& f : n->types) {
		try {
			f->compile(this);
		}catch (CtError *e) {
			delete e;
		}
	}

	for (auto& f : n->funcs) {
		try {
			f->compile(this);
		}catch (CtError *e) {
			delete e;
		}
	}

	for (auto& f : n->traits) {
		try {
			f->compile(this);
		}catch (CtError *e) {
			delete e;
		}
	}

	for (auto& f : n->extensions) {
		try {
			f->compile(this);
		}catch (CtError *e) {
			delete e;
		}
	}
}

//evaluates and prints a single-expression module
//Used in REPL
void Compiler::eval(){
    string cmd = "";
    cout << "Ante REPL v0.0.1\nType 'exit' to exit.\n";
   
    //setup compiler
    createMainFn();
    compilePrelude();

    cout << ": " << flush;
    getline(cin, cmd);
    while(cmd != "exit"){
        //lex and parse the new string
        setLexer(new Lexer(nullptr, cmd, /*line*/1, /*col*/1));
        yy::parser p{};
        int flag = p.parse();
        if(flag == PE_OK){
            RootNode *expr = parser::getRootNode();

            //Compile each expression and hold onto the last value
            TypedValue *val = expr->compile(this);

            //print val if it's not an error
            if(val)
                val->val->dump();
        }
        cout << ": " << flush;
        getline(cin, cmd);
    }
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

    auto args = main->getArgumentList().begin();
    builder.CreateStore(&*args, argc);
    builder.CreateStore(&*++args, argv);

    stoVar("argc", new Variable("argc", new TypedValue(builder.CreateLoad(argc), mkAnonTypeNode(TT_I32)), 1));
    stoVar("argv", new Variable("argv", new TypedValue(builder.CreateLoad(argv), mkTypeNodeWithExt(TT_Ptr, mkTypeNodeWithExt(TT_Ptr, mkAnonTypeNode(TT_C8)))), 1));

    //add main to call stack
    auto *main_params = mkAnonTypeNode(TT_U32);
    main_params->next.reset(mkTypeNodeWithExt(TT_Ptr, mkTypeNodeWithExt(TT_Ptr, mkAnonTypeNode(TT_C8))));

    auto *main_tv = new TypedValue(main, mkTypeNodeWithExt(TT_Function, main_params));
    auto *main_var = new FuncDecl(0, scope, mergedCompUnits, main_tv);
    compCtxt->callStack.push_back(main_var);
    return main;
}


TypedValue* RootNode::compile(Compiler *c){
    scanImports(c, this);
    c->scanAllDecls(this);

    //Compile the rest of the program
    TypedValue *ret = 0;
    for(auto &n : main){
        try{
            ret = n->compile(c);
        }catch(CtError *e){
            delete e;
        }
    }

    return ret ? ret : c->getVoidLiteral();
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
    (*allCompiledModules)[fileName] = compUnit;

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

//returns 0 on success
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

/*
 *  Compiles a module into a .o file to be used for linking.
 */
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

    //delete tm;
    return res;
}

/*
 *  Invoke linker to linke module
 */
int Compiler::linkObj(string inFiles, string outFile){
    string cmd = AN_LINKER " " + inFiles + " -o " + outFile;
    return system(cmd.c_str());
}


/*
 *  Dumps current contents of module to stdout
 */
void Compiler::emitIR(){
    if(!compiled) compile();
    if(errFlag) puts("Partially compiled module: \n");
    module->dump();
}
    

string typeNodeToStrWithModifiers(TypeNode *tn){
    string ret = "";
    for(int m : tn->modifiers){
        ret += Lexer::getTokStr(m) + " ";
    }
    return ret + typeNodeToStr(tn);
}

/*
 *  Prints type and value of TypeNode to stdout
 */
void TypedValue::dump() const{
    cout << "type:\t" << typeNodeToStrWithModifiers(type.get()) << endl
         << "val:\t" << flush;
    
    if(type->type == TT_Void)
        puts("void ()");
    else if(type->type == TT_Type)
        cout << typeNodeToStr((TypeNode*)((ConstantInt*)val)->getZExtValue()) << endl;
    else
        val->dump();
}


void Compiler::enterNewScope(){
    scope++;
    auto *vtable = new map<string, Variable*>();
    varTable.push_back(unique_ptr<map<string, Variable*>>(vtable));
}


void Compiler::exitScope(){
    //iterate through all known variables, check for pointers at the end of
    //their lifetime, and insert calls to free for any that are found
    auto vtable = varTable.back().get();

    for(auto it = vtable->cbegin(); it != vtable->cend(); it++){
        if(it->second->isFreeable() && it->second->scope == this->scope){
            string freeFnName = "free";
            Function* freeFn = (Function*)getFunction(freeFnName, freeFnName)->val;
            
            auto *inst = dyn_cast<AllocaInst>(it->second->getVal());
            auto *val = inst? builder.CreateLoad(inst) : it->second->getVal();

            //change the pointer's type to void so it is not freed again
            it->second->tval->type->type = TT_Void;

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
        try{
            return varTable[i-1]->at(var);
        }catch(out_of_range r){}
    }

    return nullptr;
}


void Compiler::stoVar(string var, Variable *val){
    if(var[0] == '\''){
        auto ty = (TypeNode*)dyn_cast<ConstantInt>(val->tval->val)->getZExtValue();
        cout << "Storing var " << var << " on scope " << scope << " with fnScope " << fnScope << " and value " << typeNodeToStr(ty) << endl;
    }

    (*varTable[val->scope-1])[var] = val;
}


void Compiler::stoTypeVar(string &name, TypeNode *ty){
    Value *addr = builder.getInt64((unsigned long)ty);
    TypedValue *tv = new TypedValue(addr, mkAnonTypeNode(TT_Type));
    Variable *var = new Variable(name, tv, scope);
    stoVar(name, var);
}


DataType* Compiler::lookupType(string tyname) const{
    try{
        return mergedCompUnits->userTypes.at(tyname).get();
    }catch(out_of_range r){
        return nullptr;
    }
}

Trait* Compiler::lookupTrait(string tyname) const{
    try{
        return mergedCompUnits->traits.at(tyname).get();
    }catch(out_of_range r){
        return nullptr;
    }
}


inline void Compiler::stoType(DataType *ty, string &typeName){
    shared_ptr<DataType> dt{ty};

    compUnit->userTypes[typeName] = dt;
    mergedCompUnits->userTypes[typeName] = dt;
}

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
        pm->add(createLoadCombinePass());
        pm->add(createLoopLoadEliminationPass());
        pm->add(createReassociatePass());
        pm->add(createPromoteMemoryToRegisterPass());

        //Instruction Combining Pass seems to break nested for loops
        //pm->add(createInstructionCombiningPass());
    }
    pm->doInitialization();
    return pm;
}

char* copy(const char* str){
    size_t len = strlen(str);
    char* cpy = new char[len+1];
    strncpy(cpy, str, len);
    cpy[len] = '\0';
    return cpy;
}

Compiler::Compiler(const char *_fileName, bool lib, shared_ptr<LLVMContext> llvmCtxt) :
        ctxt(llvmCtxt ? llvmCtxt : shared_ptr<LLVMContext>(new LLVMContext())),
        builder(*ctxt), 
        compUnit(new ante::Module()),
        mergedCompUnits(new ante::Module()),
        allCompiledModules(new map<string,shared_ptr<ante::Module>>()),
        compCtxt(new CompilerCtxt()),
        errFlag(false),
        compiled(false),
        isLib(lib),
        fileName(_fileName? _fileName : "(stdin)"),
        funcPrefix(""),
        scope(0), optLvl(2), fnScope(1){

    //The lexer stores the fileName in the loc field of all Nodes. The fileName is copied
    //to let Node's outlive the Compiler they were made in, ensuring they work with imports.
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
        
        ast.reset(parser::getRootNode());
    }

    auto modName = removeFileExt(fileName);
    compUnit->name = modName;
    mergedCompUnits->name = modName;

    outFile = modName;
	if (outFile.empty())
		outFile = "a";

    module.reset(new llvm::Module(outFile, *ctxt));

    enterNewScope();

    //add passes to passmanager.
    passManager.reset(mkPassManager(module.get(), optLvl));
}

Compiler::Compiler(Node *root, string modName, string &fName, bool lib, shared_ptr<LLVMContext> llvmCtxt) :
        ctxt(llvmCtxt ? llvmCtxt : shared_ptr<LLVMContext>(new LLVMContext())),
        builder(*ctxt),
        compUnit(new ante::Module()),
        mergedCompUnits(new ante::Module()),
        allCompiledModules(new map<string,shared_ptr<ante::Module>>()),
        compCtxt(new CompilerCtxt()),
        errFlag(false),
        compiled(false),
        isLib(lib),
        fileName(fName),
        outFile(modName),
        funcPrefix(""),
        scope(0), optLvl(2), fnScope(1){

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

    if(compCtxt)
        compCtxt->callStack.pop_back();

	passManager.release();
	module.release();
}
