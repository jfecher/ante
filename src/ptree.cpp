/*
 *      ptree.cpp
 *  Provide a C API to be used in parser.c generated from
 *  syntax.y which creates and links nodes to a parse tree.
 */
#include "parser.h"
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

/*
 *  Sets the else of an ifnode to a given ifnode representing
 *  either an else or an elif.

Node* setElse(IfNode *c, IfNode *elif){
    c->elseN.reset(elif);
    return elif;
}*/

Node* mkIntLitNode(yy::parser::location_type loc, char* s){
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

Node* mkFltLitNode(yy::parser::location_type loc, char* s){
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

Node* mkStrLitNode(yy::parser::location_type loc, char* s){
    return new StrLitNode(loc, s);
}

Node* mkBoolLitNode(yy::parser::location_type loc, char b){
    return new BoolLitNode(loc, b);
}

Node* mkArrayNode(yy::parser::location_type loc, Node *expr){
    vector<Node*> exprs;
    while(expr){
        exprs.push_back(expr);
        expr = expr->next.get();
    }
    return new ArrayNode(loc, exprs);
}

Node* mkTupleNode(yy::parser::location_type loc, Node *expr){
    vector<Node*> exprs;
    while(expr){
        exprs.push_back(expr);
        expr = expr->next.get();
    }
    return new TupleNode(loc, exprs);
}

Node* mkModNode(yy::parser::location_type loc, TokenType mod){
    return new ModNode(loc, mod);
}

Node* mkTypeNode(yy::parser::location_type loc, TypeTag type, char* typeName, Node* extTy = nullptr){
    return new TypeNode(loc, type, typeName, static_cast<TypeNode*>(extTy));
}

Node* mkTypeCastNode(yy::parser::location_type loc, Node *l, Node *r){
    return new TypeCastNode(loc, static_cast<TypeNode*>(l), r);
}

Node* mkUnOpNode(yy::parser::location_type loc, int op, Node* r){
    return new UnOpNode(loc, op, r);
}

Node* mkBinOpNode(yy::parser::location_type loc, int op, Node* l, Node* r){
    return new BinOpNode(loc, op, l, r);
}

Node* mkRetNode(yy::parser::location_type loc, Node* expr){
    return new RetNode(loc, expr);
}

//helper function to deep-copy TypeNodes.  Used in mkNamedValNode
TypeNode* deepCopyTypeNode(const TypeNode *n){
    yy::location loc = {{n->loc.begin.filename, n->loc.begin.line, n->loc.begin.column}, 
                        {n->loc.end.filename, n->loc.end.line, n->loc.end.column}};
    TypeNode *cpy = new TypeNode(loc, n->type, n->typeName, nullptr);

    if(n->type == TT_Tuple){
        TypeNode *nxt = n->extTy.get();
        TypeNode *ext = nxt? deepCopyTypeNode(nxt) : 0;
        cpy->extTy.reset(ext);

        while((nxt = static_cast<TypeNode*>(nxt->next.get()))){
            ext->next.reset(deepCopyTypeNode(nxt));
            ext = static_cast<TypeNode*>(ext->next.get());
        }
    }else if(n->type == TT_Array || n->type == TT_Ptr){
        cpy->extTy.reset(deepCopyTypeNode(n->extTy.get()));
    }
    return cpy;
}


/*
 *  This may create several NamedVal nodes depending on the
 *  number of VarNodes contained within varNodes.
 *  This is used for the shortcut when declaring multiple
 *  variables of the same type, e.g. i32 a b c
 */
Node* mkNamedValNode(yy::parser::location_type loc, Node* varNodes, Node* tExpr){
    //Note: there will always be at least one varNode
    const TypeNode* ty = (TypeNode*)tExpr;
    VarNode* vn = (VarNode*)varNodes;
    Node *ret = new NamedValNode(loc, vn->name, tExpr);
    Node *nxt = ret;

    while((vn = (VarNode*)vn->next.get())){
        TypeNode *tyNode = deepCopyTypeNode(ty);
        nxt->next.reset(new NamedValNode(vn->loc, vn->name, tyNode));
        nxt->next->prev = nxt;
        nxt = nxt->next.get();
    }
    delete varNodes;
    return ret;
}

Node* mkFuncCallNode(yy::parser::location_type loc, char* s, Node* p){
    return new FuncCallNode(loc, s, (TupleNode*)p);
}

Node* mkVarNode(yy::parser::location_type loc, char* s){
    return new VarNode(loc, s);
}

/*Node* mkRefVarNode(yy::parser::location_type loc, char* s){
    return new RefVarNode(loc, s);
}*/

Node* mkImportNode(yy::parser::location_type loc, Node* expr){
    return new ImportNode(loc, expr);
}

Node* mkLetBindingNode(yy::parser::location_type loc, char* s, Node* mods, Node* tExpr, Node* expr){
    return new LetBindingNode(loc, s, mods, tExpr, expr);
}

Node* mkVarDeclNode(yy::parser::location_type loc, char* s, Node* mods, Node* tExpr, Node* expr){
    return new VarDeclNode(loc, s, mods, tExpr, expr);
}

Node* mkVarAssignNode(yy::parser::location_type loc, Node* var, Node* expr, bool freeLval = true){
    return new VarAssignNode(loc, var, expr, freeLval);
}

Node* mkExtNode(yy::parser::location_type loc, Node* ty, Node* methods){
    return new ExtNode(loc, (TypeNode*)ty, methods);
}

/*
ParentNode* mkIfNode(yy::parser::location_type loc, Node* con, Node* body, Node* els = nullptr){
    return new IfNode(loc, con, body, (IfNode*)els);
}*/

Node* mkExprIfNode(yy::parser::location_type loc, Node* con, Node* then, Node* els){
    return new ExprIfNode(loc, con, then, els);
}

ParentNode* mkWhileNode(yy::parser::location_type loc, Node* con, Node* body){
    return new WhileNode(loc, con, body);
}

ParentNode* mkFuncDeclNode(yy::parser::location_type loc, char* s, Node* mods, Node* tExpr, Node* p, Node* b){
    return new FuncDeclNode(loc, s, mods, tExpr, p, b);
}

ParentNode* mkDataDeclNode(yy::parser::location_type loc, char* s, Node* b){
    return new DataDeclNode(loc, s, b, Compiler::getTupleSize(b));
}
