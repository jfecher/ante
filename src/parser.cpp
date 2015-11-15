#include "parser.h"


Parser::Parser(const char* file) : lexer(file)
{
    c = lexer.next();
    n = lexer.next();
}


ParseErr Parser::parse()
{
    return parseTopLevelStmt();
}

inline void Parser::incPos()
{
    c = n;
    n = lexer.next();
}

ParseErr Parser::parseErr(ParseErr e, string msg, bool showTok = true)
{
    cerr << "Syntax Error: ";
    fprintf(stderr, msg.c_str());
    if(showTok){
        if(c.type == Tok_Ident || c.type == Tok_StrLit || c.type == Tok_IntLit || c.type == Tok_FltLit)
            cerr << ", got " << c.lexeme << " (" << tokDictionary[c.type] << ")\n";
        else
            cerr << ", got " << tokDictionary[c.type] << endl;
    }
    return e;
}

bool Parser::accept(TokenType t)
{
    if(c.type == t){
        incPos();
        return true;
    }
    return false;
}

#define expect(t) if(!_expect(t)) return PE_EXPECTED
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

ParseErr Parser::parseTopLevelStmt()
{
    while(c.type != Tok_EndOfInput){
        ParseErr e = parseStmt();
        if(e != PE_OK)
            return e;
        accept(Tok_Newline); //accept?
    }
    return PE_OK;
}

ParseErr Parser::parseBlock()
{
    expect(Tok_Indent);
    while(c.type != Tok_EndOfInput && c.type != Tok_Unindent){
        ParseErr e = parseStmt();
        if(e != PE_OK)
            return e;
        accept(Tok_Newline);
    }
    expect(Tok_Unindent);
    return PE_OK;
}

//TODO: usertypes
bool Parser::isType(TokenType t)
{
    return t == Tok_I8 || t == Tok_I16 || t == Tok_I32 || t == Tok_I64
        || t == Tok_U8 || t == Tok_U16 || t == Tok_U32 || t == Tok_U64
        || t == Tok_F32 || t == Tok_F64 || t == Tok_Bool || t == Tok_Void;
}//Tok_Ident?

ParseErr Parser::parseStmt()
{
    switch(c.type){
        case Tok_If: return parseIfStmt();
        case Tok_Newline: accept(Tok_Newline); return parseStmt();
        case Tok_Class: return parseClass();
        case Tok_Return: accept(Tok_Return); return parseExpr();
        case Tok_Ident: return parseGenericVar();
        default: break;
    }

    if(isType(c.type)){
        return parseGenericDecl();
    }

    return c.type == Tok_EndOfInput? PE_OK : parseErr(PE_VAL_NOT_FOUND, "Invalid statement"); //end of file
}

ParseErr Parser::parseGenericVar()
{
    if(!parseVariable()) return PE_IDENT_NOT_FOUND;
    
    if(acceptOp('(')){//funcCall
        ParseErr e = parseExpr();
        if(e != PE_OK) return e;
        expectOp(')');
        return PE_OK;
    }

    //TODO: expand to += -= *= etc
    if(acceptOp('=')){//assignment
        return parseExpr();
    }
    return PE_OK;
}

ParseErr Parser::parseGenericDecl()
{
    incPos();//assume type is already found, and eat it
    if(!parseVariable()) return PE_IDENT_NOT_FOUND ;

    if(acceptOp(':')){//funcDef
        ParseErr e = parseTypeList();
        if(e!=PE_OK) return e;
        return parseBlock();
    }else if(acceptOp('=')){
        return parseExpr();
    }
    return PE_OK;
}

ParseErr Parser::parseClass()
{
    expect(Tok_Class);
    return parseBlock();
}

ParseErr Parser::parseIfStmt()
{
    expect(Tok_If);
    ParseErr e = parseExpr();
    if(e != PE_OK) return e;
    return parseBlock();
}

ParseErr Parser::parseTypeList()
{
    while(isType(c.type)){
        incPos();
        expect(Tok_Ident);
    }
    return PE_OK;
}

bool Parser::parseVariable()
{
    if(!_expect(Tok_Ident)) return false;

    while(acceptOp('.')){
        if(!_expect(Tok_Ident)) return false;
        if(acceptOp('[')){
            parseExpr();
            expectOp(']');
        }
    }
    if(acceptOp('[')){
        parseExpr();
        expectOp(']');
    }

    return true;
}

bool Parser::parseValue()
{
    switch(c.type){
        case Tok_IntLit:
        case Tok_FltLit:
        case Tok_StrLit:
        case Tok_True:
        case Tok_False:
            incPos();
            return true;
        case Tok_Ident:
            return parseVariable();
        case Tok_Operator:
            if(*c.lexeme != '(') return false;
            incPos();
            parseExpr();
            expectOp(')');
            return true;
        default: return false;
    }
}

bool Parser::parseOp()
{
    switch(c.type){
        case Tok_Operator:
            if(IS_TERMINATING_OP(*c.lexeme))
                return false;
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
            return true;
        default: return false;
    }
}

ParseErr Parser::parseExpr()
{
    if(!parseValue()){
        return parseErr(PE_VAL_NOT_FOUND, "Initial value not found in expression");
    }
    return parseRExpr();
}

ParseErr Parser::parseRExpr()
{
    if(parseOp()){
        if(!parseValue()) return parseErr(PE_VAL_NOT_FOUND, "Following value not found in expression");
        return parseRExpr();
    }
    return PE_OK;
}
