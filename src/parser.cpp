#include "parser.h"


Parser::Parser(const char* file) : lexer{file}
{
    c = lexer.next();
    n = lexer.next();
}


ParseErr Parser::parse()
{
    
}

inline void Parser::incPos()
{
    c = n;
    n = lexer.next();
}

void Parser::parseErr(string msg, ...)
{
    va_list args;
    va_start(args, msg);
    cerr << "Syntax Error: ";



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
        parseErr("Expected %s, but got %s.\n", tokDictionary[t], tokDictionary[c.type]);
        return false;
    }
    return true;
}



/*
 *  topLevelStatement: classDecl
 *                   | funcDecl
 *                   | statement
 *                   ;
 */
ParseErr Parser::parseTopLevelStatement()
{
    while(c.type != Tok_EndOfInput){
        parseStatement();
        expect(Tok_Newline);
    }
}
