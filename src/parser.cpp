#include "parser.h"


Parser::Parser(const char* file) : lexer(file)
{
    c = lexer.next();
    n = lexer.next();
}


ParseErr Parser::parse()
{
    buildParseTree();
    return errFlag;
}

inline void Parser::incPos()
{
    c = n;
    n = lexer.next();
}

void Parser::parseErr(ParseErr e, string msg, bool showTok = true)
{
    cerr << "Syntax Error: ";
    fprintf(stderr, msg.c_str());
    if(showTok){
        if(c.type == Tok_Ident || c.type == Tok_StrLit || c.type == Tok_IntLit || c.type == Tok_FltLit)
            cerr << ", got " << c.lexeme << " (" << tokDictionary[c.type] << ")\n";
        else
            cerr << ", got " << tokDictionary[c.type] << endl;
    }
    errFlag = e;
}

bool Parser::accept(TokenType t)
{
    if(c.type == t){
        incPos();
        return true;
    }
    return false;
}

#define expect(t) if(!_expect(t)) {errFlag = PE_EXPECTED; return NULL;}
bool Parser::_expect(TokenType t)
{
    if(!accept(t)){
        string s = "Expected ";
        s += tokDictionary[t];
        parseErr(PE_EXPECTED, s);
        return false;
    }
    return true;
}

bool Parser::acceptOp(char op){
    if(c.type == Tok_Operator && *c.lexeme == op){
        incPos();
        return true;
    }
    return false;
}

bool Parser::expectOp(char op){
    if(!acceptOp(op)){
        string s = "Expected ";
        s += op;
        parseErr(PE_EXPECTED, s);
        return false;
    }
    return true;
}

Node* Parser::buildParseTree()
{
    root = new ClassDeclNode("__main__", NULL); //TODO: replace __main__ with file name
    branch = root;

    while(c.type != Tok_EndOfInput){
        Node *n = parseStmt();
        if(errFlag != PE_OK)
            return NULL;
        accept(Tok_Newline);

        branch->next = n;
        n->next = NULL;
        branch = n;
    }
    return root;
}

Node* Parser::parseBlock()
{
    Node* bRoot = NULL;
    Node* bBranch = NULL;

    expect(Tok_Indent);
    while(c.type != Tok_EndOfInput && c.type != Tok_Unindent){
        Node* n = parseStmt();
        if(errFlag != PE_OK) return NULL;
        accept(Tok_Newline);

        if(bRoot == NULL){
            bRoot = n;
            bBranch = n;
        }else{
            bBranch->next = n;
            n->next = NULL;
            bBranch = n;
        }
    }
    expect(Tok_Unindent);
    return bRoot;
}

//TODO: usertypes
bool Parser::isType(TokenType t)
{
    return t == Tok_I8 || t == Tok_I16 || t == Tok_I32 || t == Tok_I64
        || t == Tok_U8 || t == Tok_U16 || t == Tok_U32 || t == Tok_U64
        || t == Tok_F32 || t == Tok_F64 || t == Tok_Bool || t == Tok_Void;
}//Tok_Ident?

Node* Parser::parseStmt()
{
    switch(c.type){
        case Tok_If: return parseIfStmt();
        case Tok_Newline: accept(Tok_Newline); return parseStmt();
        case Tok_Class: return parseClass();
        case Tok_Return: accept(Tok_Return); return parseExpr();//TODO: dedicated parseRetStmt
        case Tok_Ident: return parseGenericVar();
        default: break;
    }

    if(isType(c.type)){
        return parseGenericDecl();
    }

    if(c.type != Tok_EndOfInput){
        parseErr(PE_INVALID_STMT, "Invalid statement starting with ");
    }
    return NULL;
}

Node* Parser::parseGenericVar()
{
    string identifier = c.lexeme;
    expect(Tok_Ident);
    
    if(acceptOp('(')){//funcCall
        FuncCallNode *n = new FuncCallNode(identifier, parseExpr());
        if(errFlag != PE_OK) return n;
        expectOp(')');
        return n;
    }

    //TODO: expand to += -= *= etc
    if(acceptOp('=')){//assignment
        return parseExpr();
    }
    return NULL;//TODO
}

Node* Parser::parseGenericDecl()
{
    Token type = c;
    incPos();//assume type is already found, and eat it
    
    string name = c.lexeme;
    if(!parseVariable())
        return NULL;

    if(acceptOp(':')){//funcDef
        NamedValNode *params = parseTypeList();
        if(errFlag != PE_OK) return NULL;
        Node *body = parseBlock();
        return new FuncDeclNode(name, type, params, body);
    }else if(acceptOp('=')){
        return new VarDeclNode(name, type, parseExpr());
    }

    //declaration without default value
    return new VarDeclNode(name, type, NULL);
}

Node* Parser::parseClass()
{
    expect(Tok_Class);
    return parseBlock();
}

Node* Parser::parseIfStmt()
{
    expect(Tok_If);
    Node *conditional = parseExpr();
    if(errFlag != PE_OK) return NULL;
    Node *body = parseBlock();
    return new IfNode(conditional, body);
}

NamedValNode* Parser::parseTypeList()
{
    NamedValNode *tlRoot = NULL;
    NamedValNode *tlBranch = NULL;

    while(isType(c.type)){
        Token type = c;
        incPos();
        string name = c.lexeme;
        expect(Tok_Ident);

        if(tlRoot == NULL){
            tlRoot = new NamedValNode(name, type);
            tlBranch = tlRoot;
        }else{
            tlBranch->next = new NamedValNode(name, type);
            tlBranch = (NamedValNode*)tlBranch->next;
            tlBranch->next = NULL;
        }
    }
    return tlRoot;
}

//TODO: parseLExpr
Node* Parser::parseVariable()
{
    string s = c.lexeme;
    if(!_expect(Tok_Ident)) return NULL;
    return new VarNode(s);
}

Node* Parser::parseValue()
{
    string s = c.lexeme;
    Node* ret;
    switch(c.type){
        case Tok_IntLit: 
            incPos();
            return new IntLitNode(s);
        case Tok_FltLit: 
            incPos();
            return new FltLitNode(s);
        case Tok_StrLit: 
            incPos();
            return new StrLitNode(s);
        case Tok_True:   
            incPos();
            return new BoolLitNode(true);
        case Tok_False:  
            incPos();
            return new BoolLitNode(false);
        case Tok_Ident:
            return parseVariable();
        case Tok_Operator:
            if(*c.lexeme != '(') return NULL;
            incPos();
            ret = parseExpr();
            expectOp(')');
            return ret;
        default: 
            return NULL;
    }
}

Node* Parser::parseOp()
{
    Token op = c;
    switch(c.type){
        case Tok_Operator:
            if(IS_TERMINATING_OP(*c.lexeme))
                return NULL;
        case Tok_Eq:
        case Tok_AddEq:
        case Tok_SubEq:
        case Tok_MulEq:
        case Tok_DivEq:
        case Tok_NotEq:
        case Tok_GrtrEq:
        case Tok_LesrEq:
        case Tok_StrCat:
            incPos();
            return new BinOpNode(c, NULL, NULL);
        default: 
            return NULL;
    }
}

Node* Parser::parseExpr()
{
    Node *val = parseValue();
    if(val == NULL){
        parseErr(PE_VAL_NOT_FOUND, "Initial value not found in expression");
        return NULL;
    }
    return parseRExpr();
}

Node* Parser::parseRExpr()
{
    if(parseOp()){
        if(!parseValue()){
            parseErr(PE_VAL_NOT_FOUND, "Following value not found in expression");
            return NULL;
        }
        return parseRExpr();
    }
    return NULL;//PE_OKAY
}
