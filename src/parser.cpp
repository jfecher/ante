#include "parser.h"


Parser::Parser(const char* file) : lexer(file)
{
    errFlag = PE_OK;
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
    lexer.printTok(n);
}

void Parser::parseErr(ParseErr e, string msg, bool showTok = true)
{
    cerr << "Syntax Error: " << msg;
    if(showTok)
        lexer.printTok(c);
    else
        cerr << endl;

    if(errFlag != PE_OK) errFlag = e;
    cout << "errFlag: " << e;
    exit(5);
}

bool Parser::accept(TokenType t)
{
    if(c.type == t){
        incPos();
        return true;
    }
    return false;
}

bool Parser::expect(TokenType t)
{
    if(!accept(t)){
        string s = "Expected ";
        s += tokDictionary[t];
        s += ", but got ";
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
        s += ", but got ";
        parseErr(PE_EXPECTED, s);
        return false;
    }
    return true;
}

void Parser::buildParseTree()
{
    while(c.type != Tok_EndOfInput){
        Node* n = parseStmt();
        accept(Tok_Newline);
        parseTree.push_back(n);
        cout << "\n\nerrFlag = " << errFlag << endl;
    }
}

void Parser::printParseTree()
{
    for(vector<Node*>::iterator it = parseTree.begin(); it != parseTree.end(); ++it){
        (*it)->print();
        cout << " thus was node." << endl;
    }
}

vector<Node*> Parser::parseBlock()
{
    vector<Node*> block;
    expect(Tok_Indent);
    while(c.type != Tok_EndOfInput && c.type != Tok_Unindent){
        Node* n = parseStmt();
        accept(Tok_Newline);
        block.push_back(n);
    }
    expect(Tok_Unindent);
    return block;
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
        expectOp(')');
        return n;
    }

    //TODO: expand to += -= *= etc
    if(acceptOp('=')){//assignment
        return new VarAssignNode(identifier, parseExpr());
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
        vector<NamedValNode*> params = parseTypeList();
        return new FuncDeclNode(name, type, params, parseBlock());
    }else if(acceptOp('=')){
        return new VarDeclNode(name, type, parseExpr());
    }

    //declaration without default value
    return new VarDeclNode(name, type, NULL);
}

ClassDeclNode* Parser::parseClass()
{
    expect(Tok_Class);
    string identifier = c.lexeme;
    expect(Tok_Ident);
    return new ClassDeclNode(identifier, parseBlock());
}

IfNode* Parser::parseIfStmt()
{
    expect(Tok_If);
    Node *conditional = parseExpr();
    return new IfNode(conditional, parseBlock());
}

vector<NamedValNode*> Parser::parseTypeList()
{
    vector<NamedValNode*> typeList;

    while(isType(c.type)){
        Token type = c;
        incPos();
        string name = c.lexeme;
        if(!expect(Tok_Ident)) return typeList;
        typeList.push_back(new NamedValNode(name, type));
    }
    return typeList;
}

//TODO: parseLExpr
Node* Parser::parseVariable()
{
    string s = c.lexeme;
    if(!expect(Tok_Ident)) return NULL;
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
            return new BinOpNode(op, NULL, NULL);
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
