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

void Parser::parseErr(string msg, bool showTok = false)
{
    cerr << "Syntax Error: ";
    fprintf(stderr, msg.c_str());
    if(showTok)
        cerr << ", Got " << c.lexeme;
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
        parseErr(s, true);
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
        parseErr(s, true);
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
        case Tok_Ident: return parseGenericVar();
        default: break;
    }

    if(isType(c.type)){
        return parseGenericDecl();
    }
    return PE_VAL_NOT_FOUND; //end of file
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
        //TODO: parse parameters
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
    if(!parseExpr()) return PE_VAL_NOT_FOUND;
    return parseBlock();
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
            incPos();
            return true;
        case Tok_Ident:
            return parseVariable();
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
            incPos();
            return true;
        default: return false;
    }
}

ParseErr Parser::parseExpr()
{
    if(!parseValue()) return PE_VAL_NOT_FOUND;
    return parseRExpr();
}

ParseErr Parser::parseRExpr()
{
    if(parseOp()){
        if(!parseValue()) return PE_VAL_NOT_FOUND;
        return parseRExpr();
    }
    return PE_OK;
}
