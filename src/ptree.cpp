/*
 *      ptree.cpp
 *  Provide a C API to be used in parser.c generated from
 *  syntax.y which creates and links nodes to a parse tree.
 */
#include "parser.h"
#include "yyparser.h"


Node *rootNode = 0;


ArrayNode* setNext(Node *an, Node *nxt){
    auto *ret = static_cast<ArrayNode*>(an);
    ret->exprs.push_back(nxt);
    return ret;
}


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
    if(!expr) return new ArrayNode(loc, exprs);

    while(true){
        auto *seqNode = dynamic_cast<BinOpNode*>(expr);

        if(seqNode && seqNode->op == ';'){
            exprs.push_back(seqNode->lval.get());
            expr = seqNode->rval.get();
        }else{
            exprs.push_back(expr);
            return new ArrayNode(loc, exprs);
        }
    }
}

Node* mkTupleNode(yy::parser::location_type loc, Node *expr){
    vector<Node*> exprs;
    if(!expr) return new TupleNode(loc, exprs);
    
    while(true){
        auto *seqNode = dynamic_cast<BinOpNode*>(expr);

        if(seqNode && seqNode->op == ';'){
            exprs.push_back(seqNode->lval.get());
            expr = seqNode->rval.get();
        }else{
            exprs.push_back(expr);
            return new TupleNode(loc, exprs);
        }
    }
}

Node* mkModNode(yy::parser::location_type loc, TokenType mod){
    return new ModNode(loc, mod);
}

Node* mkTypeNode(yy::parser::location_type loc, TypeTag type, char* typeName, ArrayNode* extTy = nullptr){
    return new TypeNode(loc, type, typeName, extTy);
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

    auto *cpyExts = new ArrayNode(loc);
    TypeNode *cpy = new TypeNode(loc, n->type, n->typeName, cpyExts);

    if(n->type == TT_Tuple){
        for(auto *extTy : n->extTys->exprs){
            cpy->getExts().push_back(deepCopyTypeNode(static_cast<TypeNode*>(extTy)));
        }
    }else if(n->type == TT_Array || n->type == TT_Ptr){
        cpy->getExts().push_back(deepCopyTypeNode(static_cast<TypeNode*>(n->getExts()[0])));
    }
    return cpy;
}


/*
 *  This may create several NamedVal nodes depending on the
 *  number of VarNodes contained within varNodes.
 *  This is used for the shortcut when declaring multiple
 *  variables of the same type, e.g. i32 a b c
 */
ArrayNode* addNamedValNode(yy::parser::location_type loc, ArrayNode* params, ArrayNode* varNodes, Node* tExpr){
    //Note: there will always be at least one varNode
    const auto* ty = static_cast<TypeNode*>(tExpr);

    for(Node* e : varNodes->exprs){
        auto* vn = static_cast<VarNode*>(e);
        params->exprs.push_back(new NamedValNode(vn->loc, vn->name, deepCopyTypeNode(ty)));
    }
    delete varNodes;
    return params;
}

Node* mkVarNode(yy::parser::location_type loc, char* s){
    return new VarNode(loc, s);
}

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

Node* mkIfNode(yy::parser::location_type loc, Node* con, Node* then, Node* els){
    return new IfNode(loc, con, then, els);
}

Node* mkWhileNode(yy::parser::location_type loc, Node* con, Node* body){
    return new WhileNode(loc, con, body);
}

Node* mkFuncDeclNode(yy::parser::location_type loc, char* s, Node* mods, Node* tExpr, ArrayNode* p, Node* b){
    return new FuncDeclNode(loc, s, mods, tExpr, p, b);
}

Node* mkDataDeclNode(yy::parser::location_type loc, char* s, Node* b){
    return new DataDeclNode(loc, s, b, Compiler::getTupleSize(b));
}
