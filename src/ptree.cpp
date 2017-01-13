/*
 *      ptree.cpp
 *  Provide a C API to be used in parser.c generated from
 *  syntax.y which creates and links nodes to a parse tree.
 */
#include "compiler.h"
#include "yyparser.h"
#include <stack>

stack<Node*> roots;

Node* ante::parser::getRootNode(){
    return roots.top();
}

/*
 *  Saves the root of a new block and returns it.
 */
Node* setRoot(Node* node){
    roots.push(node);
    return node;
}

Node* setElse(Node *ifn, Node *elseN){
    if(auto *n = dynamic_cast<IfNode*>(ifn)){
        n->elseN.reset(elseN);
    }else{
        auto *binop = dynamic_cast<BinOpNode*>(ifn);

        if(binop and (n = dynamic_cast<IfNode*>(binop->rval.get()))){
            while(auto *tmp = dynamic_cast<IfNode*>(n->elseN.get()))
                n = tmp;

            n->elseN.reset(elseN);
            return ifn;
        }else{
            ante::error("Missing matching if clause for else clause", ifn->loc);
        }
    }
    return ifn;
}

LOC_TY copyLoc(const LOC_TY &loc){
    return {yy::position(loc.begin.filename, loc.begin.line, loc.begin.column), 
            yy::position(loc.end.filename,   loc.end.line,   loc.end.column)};
}
    
//apply modifier to this type and all its extensions
TypeNode* TypeNode::addModifiers(ModNode *m){
    TypeNode *ext = extTy.get();

    //arrays have their size as their second extty so they
    //must be handled specially
    if(type == TT_Array){
        ext->addModifiers(m);
        ext = (TypeNode*)ext->next.get();
    }else{
        while(ext){
            ext->addModifiers(m);
            ext = (TypeNode*)ext->next.get();
        }
    }

    while(m){
        this->modifiers.push_back(m->mod);
        m = (ModNode*)m->next.get();
    }
    return this;
}

//add a single modifier to this type and all its extensions
TypeNode* TypeNode::addModifier(int m){
    TypeNode *ext = extTy.get();

    if(type == TT_Array){
        ext->addModifier(m);
        ext = (TypeNode*)ext->next.get();
    }else{
        while(ext){
            ext->addModifier(m);
            ext = (TypeNode*)ext->next.get();
        }
    }

    modifiers.push_back(m);
    return this;
}

void TypeNode::copyModifiersFrom(const TypeNode *tn){
    for(int m : tn->modifiers){
        addModifier(m);
    }
}
    
bool TypeNode::hasModifier(int m){
    return std::find(modifiers.cbegin(), modifiers.cend(), m) != modifiers.cend();
}

/*
 *  Pops and returns the root of the current block
 */
Node* getRoot(){
    Node* ret = roots.top();
    roots.pop();
    return ret;
}

Node* setNext(Node* cur, Node* nxt){
    cur->next.reset(nxt);
    nxt->prev = cur;
    return nxt;
}

Node* addMatch(Node *matchExpr, Node *newMatch){
    ((MatchNode*)matchExpr)->branches.push_back(
        unique_ptr<MatchBranchNode>((MatchBranchNode*)newMatch));
    return matchExpr;
}


Node* mkIntLitNode(LOC_TY loc, char* s){
    string str = s;
    TypeTag type = TT_I32;

    //check for type suffix
    int len = str.length();
    if(len > 2){
        if(len > 3 && (str[len -3] == 'u' || str[len - 3] == 'i')){
            char sign = str[len - 3];
            switch(str[len - 2]){
                case '1':
                    type = sign == 'i'? TT_I16 : TT_U16;
                    str = str.substr(0, len-3);
                    break;
                case '3':
                    type = sign == 'i'? TT_I32 : TT_U32;
                    str = str.substr(0, len-3);
                    break;
                case '6':
                    type = sign == 'i'? TT_I64 : TT_U64;
                    str = str.substr(0, len-3);
                    break;
                default:
                    break;
            }
        }else{
            char sign = str[len - 2];
            if(sign == 'u' || sign == 'i'){
                str = str.substr(0, len-2);
                type = sign == 'i'? TT_I8 : TT_U8;
            }
        }
    }

    return new IntLitNode(loc, str, type);
}

Node* mkFltLitNode(LOC_TY loc, char* s){
    string str = s;
    int len = str.length();
    TypeTag type = TT_F64;

    if(len > 3 && str[len - 3] == 'f'){
        char fltSize = str[len - 2];
        if(fltSize == '1'){ //16 bit IEEE half
            type = TT_F16;
            str = str.substr(0, len-3);
        }else if(fltSize == '3'){ //32 bit IEEE single
            type = TT_F32;
            str = str.substr(0, len-3);
        }else if(fltSize == '6'){ //64 bit IEEE double
            type = TT_F64;
            str = str.substr(0, len-3);
        }
    }

    return new FltLitNode(loc, str, type);
}

Node* mkStrLitNode(LOC_TY loc, char* s){
    return new StrLitNode(loc, s);
}

Node* mkCharLitNode(LOC_TY loc, char* s){
    return new CharLitNode(loc, s[0]);
}

Node* mkBoolLitNode(LOC_TY loc, char b){
    return new BoolLitNode(loc, b);
}

Node* mkArrayNode(LOC_TY loc, Node *expr){
    vector<unique_ptr<Node>> exprs;
    while(expr){
        exprs.push_back(unique_ptr<Node>(expr));
        auto *nxt = expr->next.get();
        expr->next.release();
        expr = nxt;
    }
    return new ArrayNode(loc, exprs);
}

Node* mkTupleNode(LOC_TY loc, Node *expr){
    vector<unique_ptr<Node>> exprs;
    while(expr){
        exprs.push_back(unique_ptr<Node>(expr));
        auto *nxt = expr->next.get();
        expr->next.release();
        expr = nxt;
    }
    return new TupleNode(loc, exprs);
}

Node* mkModNode(LOC_TY loc, TokenType mod){
    return new ModNode(loc, mod);
}

Node* mkPreProcNode(LOC_TY loc, Node* expr){
    return new PreProcNode(loc, expr);
}

Node* mkTypeNode(LOC_TY loc, TypeTag type, char* typeName, Node* extTy = nullptr){
    if(type == TT_Array){
        //2nd type ext is size of the array when making Array types, ensure it is an intlit
        auto *size = dynamic_cast<IntLitNode*>(extTy->next.get());

        if(!size){
            ante::error("Size of array must be an integer literal", extTy->next->loc);
            exit(1);
        }
    }
    return new TypeNode(loc, type, typeName, static_cast<TypeNode*>(extTy));
}

Node* mkTypeCastNode(LOC_TY loc, Node *l, Node *r){
    return new TypeCastNode(loc, static_cast<TypeNode*>(l), r);
}

Node* mkUnOpNode(LOC_TY loc, int op, Node* r){
    return new UnOpNode(loc, op, r);
}

Node* mkBinOpNode(LOC_TY loc, int op, Node* l, Node* r){
    return new BinOpNode(loc, op, l, r);
}

Node* mkBlockNode(LOC_TY loc, Node *b){
    return new BlockNode(loc, b);
}

Node* mkRetNode(LOC_TY loc, Node* expr){
    return new RetNode(loc, expr);
}


//returns true if type->extTy should be defined
bool typeHasExtData(TypeTag t){
    return t == TT_Tuple or t == TT_Array or t == TT_Ptr or t == TT_Data or t == TT_Function
        or t == TT_Method or t == TT_TaggedUnion or t == TT_MetaFunction;
    
}

//helper function to deep-copy TypeNodes.  Used in mkNamedValNode
TypeNode* deepCopyTypeNode(const TypeNode *n){
    if(!n) return 0;

    auto loc = copyLoc(n->loc);
    TypeNode *cpy = new TypeNode(loc, n->type, n->typeName, nullptr);

    //arrays can have an IntLit in their extTy so handle them specially
    if(n->type == TT_Array){
        cpy->extTy.reset(deepCopyTypeNode(n->extTy.get()));

        auto *len = (IntLitNode*)n->extTy->next.get();
        if(len){
            auto loc_cpy = copyLoc(len->loc);
            auto *len_cpy = new IntLitNode(loc_cpy, len->val, len->type);
            cpy->extTy->next.reset(len_cpy);
        }
    }else if(n->extTy.get()){
        TypeNode *nxt = n->extTy.get();
        if(!nxt) return cpy;

        TypeNode *ext = deepCopyTypeNode(nxt);
        cpy->extTy.reset(ext);

        while((nxt = static_cast<TypeNode*>(nxt->next.get()))){
            ext->next.reset(deepCopyTypeNode(nxt));
            ext = static_cast<TypeNode*>(ext->next.get());
        }
    }

    //finally, do a shallow copy for the modifiers
    //this becomes a deep copy since this method is called recursively for each extTy
    for(int m : n->modifiers)
        cpy->modifiers.push_back(m);

    return cpy;
}


/*
 *  This may create several NamedVal nodes depending on the
 *  number of VarNodes contained within varNodes.
 *  This is used for the shortcut when declaring multiple
 *  variables of the same type, e.g. i32 a b c
 */
Node* mkNamedValNode(LOC_TY loc, Node* varNodes, Node* tExpr, Node* prev){
    //Note: there will always be at least one varNode
    const TypeNode* ty = (TypeNode*)tExpr;
    VarNode* vn = (VarNode*)varNodes;
    Node *first = new NamedValNode(loc, vn->name, tExpr);
    Node *nxt = first;

    if(!prev) setRoot(first);
    else setNext(prev, first);

    while((vn = (VarNode*)vn->next.get())){
        TypeNode *tyNode = deepCopyTypeNode(ty);
        LOC_TY loccpy = copyLoc(vn->loc);

        nxt->next.reset(new NamedValNode(loccpy, vn->name, tyNode));
        nxt->next->prev = nxt;
        nxt = nxt->next.get();
    }
    delete varNodes;
    return nxt;
}

Node* mkVarNode(LOC_TY loc, char* s){
    return new VarNode(loc, s);
}

Node* mkImportNode(LOC_TY loc, Node* expr){
    return new ImportNode(loc, expr);
}

Node* mkLetBindingNode(LOC_TY loc, char* s, Node* mods, Node* tExpr, Node* expr){
    return new LetBindingNode(loc, s, mods, tExpr, expr);
}

Node* mkVarDeclNode(LOC_TY loc, char* s, Node* mods, Node* tExpr, Node* expr){
    return new VarDeclNode(loc, s, mods, tExpr, expr);
}

Node* mkVarAssignNode(LOC_TY loc, Node* var, Node* expr, bool freeLval = true){
    return new VarAssignNode(loc, var, expr, freeLval);
}

Node* mkExtNode(LOC_TY loc, Node* ty, Node* methods, Node* traits){
    return new ExtNode(loc, (TypeNode*)ty, methods, (TypeNode*)traits);
}

Node* mkIfNode(LOC_TY loc, Node* con, Node* then, Node* els){
    return new IfNode(loc, con, then, els);
}

Node* mkWhileNode(LOC_TY loc, Node* con, Node* body){
    return new WhileNode(loc, con, body);
}

Node* mkForNode(LOC_TY loc, Node* var, Node* range, Node* body){
    return new ForNode(loc, (char*)var, range, body);
}

Node* mkFuncDeclNode(LOC_TY loc, Node* s, Node *bn, Node* mods, Node* tExpr, Node* p, Node* b){
    return new FuncDeclNode(loc, (char*)s, (char*)bn, mods, tExpr, p, b);
}

Node* mkDataDeclNode(LOC_TY loc, char* s, Node* b){
    return new DataDeclNode(loc, s, b, getTupleSize(b));
}

Node* mkMatchNode(LOC_TY loc, Node* expr, Node* branch){
    vector<unique_ptr<MatchBranchNode>> branches;
    branch->next.release();
    branches.push_back(unique_ptr<MatchBranchNode>((MatchBranchNode*)branch));
    return new MatchNode(loc, expr, branches);
}

Node* mkMatchBranchNode(LOC_TY loc, Node* pattern, Node* branch){
    return new MatchBranchNode(loc, pattern, branch);
}

Node* mkTraitNode(LOC_TY loc, char* s, Node* fns){
    return new TraitNode(loc, s, fns);
}
