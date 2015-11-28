#include "lexer.h"
#include <cstring>

const char* tokDictionary[] = {
    "EndOfInput",
    "Identifier",

    //types
    "I8",
    "I16",
    "I32",
    "I64",
    "U8",
    "U16",
    "U32",
    "U64",
    "F32",
    "F64",
    "Bool",
    "Void",

    "Operator",
	"Eq",
    "NotEq",
	"AddEq",
	"SubEq",
    "MulEq",
    "DivEq",
	"GrtrEq",
	"LesrEq",
    "Or",
    "And",
    "True",
    "False",
	"IntLit",
	"FltLit",
	"StrLit",
    "StrCat",

    //keywords
    "Return",
	"If",
    "Elif",
	"Else",
	"For",
	"ForEach",
	"While",
    "Do",
    "In",
	"Continue",
	"Break",
    "Import",
    "Where",
    "Enum",
    "Struct",
    "Class",

    "Newline",
    "Indent",
    "Unindent",
};

map<string, Token> keywords = {
    {"i8",       {Tok_I8,       "", 0, 0}},
    {"i16",      {Tok_I16,      "", 0, 0}},
    {"i32",      {Tok_I32,      "", 0, 0}},
    {"i64",      {Tok_I64,      "", 0, 0}},
    {"u8",       {Tok_U8,       "", 0, 0}},
    {"u16",      {Tok_U16,      "", 0, 0}},
    {"u32",      {Tok_U32,      "", 0, 0}},
    {"u64",      {Tok_U64,      "", 0, 0}},
    {"f32",      {Tok_F32,      "", 0, 0}},
    {"f64",      {Tok_F64,      "", 0, 0}},
    {"bool",     {Tok_Bool,     "", 0, 0}},
    {"void",     {Tok_Void,     "", 0, 0}},
    
    {"or",       {Tok_Or,       "", 0, 0}},
    {"and",      {Tok_And,      "", 0, 0}},
    {"true",     {Tok_True,     "", 0, 0}},
    {"false",    {Tok_False,    "", 0, 0}},
    
    {"return",   {Tok_Return,   "", 0, 0}},
    {"if",       {Tok_If,       "", 0, 0}},
    {"elif",     {Tok_Elif,     "", 0, 0}},
    {"else",     {Tok_Else,     "", 0, 0}},
    {"for",      {Tok_For,      "", 0, 0}},
    {"foreach",  {Tok_ForEach,  "", 0, 0}},
    {"while",    {Tok_While,    "", 0, 0}},
    {"do",       {Tok_Do,       "", 0, 0}},
    {"in",       {Tok_In,       "", 0, 0}},
    {"continue", {Tok_Continue, "", 0, 0}},
    {"break",    {Tok_Break,    "", 0, 0}},
    {"import",   {Tok_Import,   "", 0, 0}},
    {"where",    {Tok_Where,    "", 0, 0}},
    {"enum",     {Tok_Enum,     "", 0, 0}},
    {"struct",   {Tok_Struct,   "", 0, 0}},
    {"class",    {Tok_Class,    "", 0, 0}},
};

Lexer::Lexer(void) : c{0}, n{0}
{
    incPos();
    incPos();
    scope = 0;
    cscope = 0;
}

Lexer::Lexer(const char* file): c{0}, n{0}
{
    in = new ifstream(file);
    incPos();
    incPos();
    scope = 0;
    cscope = 0;
}

Lexer::Lexer(ifstream **f): c{0}, n{0}
{
    in = *f;
    incPos();
    incPos();
    scope = 0;
    cscope = 0;
}
    
void Lexer::printTok(Token t)
{
    if(t.type == Tok_Ident || t.type == Tok_StrLit || t.type == Tok_IntLit || t.type == Tok_FltLit || t.type == Tok_Operator)
        cerr << t.lexeme << " (" << tokDictionary[t.type] << ")\n";
    else
        cerr << tokDictionary[t.type] << endl;
}

inline void Lexer::incPos(void)
{
    c = n;
    if(in->good())
        in->get(n);
    else
        n = 0;
}

void Lexer::incPos(int end)
{
    for(int i = 0; i < end; i++){
        c = n;
        if(in->good())
            in->get(n);
        else
            n = 0;
    }
}

Token Lexer::handleComment(void)
{
    if(c == '`'){
        do incPos(); while(c != '`' && c != EOF);
        incPos();
    }else{ // c == '~'
        while(c != '\n' && c != EOF) incPos();
    }
    return next();
}

Token Lexer::genAlphaNumTok()
{
    string s = "";
    while(IS_ALPHANUM(c)){
        s += c;
        incPos();
    }

    auto key = keywords.find(s.c_str());
    if(key != keywords.end()){
        return key->second;
    }else{
        return {Tok_Ident, s.c_str()};
    }
}

Token Lexer::genNumLitTok()
{
    string s = "";
    bool flt = false;

    while(IS_NUMERICAL(c) || (c == '.' && !flt && IS_NUMERICAL(n))){
        s += c;
        if(c == '.'){ 
            flt = true;
        }
        incPos();
    }
    return {flt? Tok_FltLit : Tok_IntLit, s.c_str()};
}

Token Lexer::genWsTok()
{
    if(c == '\n'){
        unsigned short newScope = 0;
        
        while(IS_WHITESPACE(c) && c != EOF){
            switch(c){
                case ' ': newScope++; break;
                case '\t': newScope += scStep; break;
                case '\n': newScope = 0; break;
                default: break;
            }
            incPos();
        }
        newScope /= scStep;

        if(newScope == scope){
            return {Tok_Newline, NULL};
        }
        scope = newScope;
        return next();
    }else{
        incPos();
        return next();
    }
}

Token Lexer::genStrLitTok()
{
    char delim = c;
    string s = "";
    incPos();
    while(c != delim && c != EOF){
        s += c;
        incPos();
    }
    incPos();
    return {Tok_StrLit, s.c_str()};
}

Token Lexer::next()
{
    if(cscope != scope){
        if(scope > cscope){
            cscope++;
            return {Tok_Indent, NULL};
        }else{
            cscope--;
            return {Tok_Unindent, NULL};
        }
    }

    if(IS_COMMENT(c))    return Lexer::handleComment();
    if(IS_NUMERICAL(c))  return Lexer::genNumLitTok();
    if(IS_ALPHANUM(c))   return Lexer::genAlphaNumTok();
    if(IS_WHITESPACE(c)) return Lexer::genWsTok();

    if(c == '"' || c == '\'') return Lexer::genStrLitTok();

    //substitute -> for an indent
    if PAIR('-', '>'){
        scope++;
        incPos(2);
        return next();
    }

    if(n == '='){
        switch(c){
            case '=': RETURN_PAIR(Tok_Eq);
            case '+': RETURN_PAIR(Tok_AddEq);
            case '-': RETURN_PAIR(Tok_SubEq);
            case '*': RETURN_PAIR(Tok_MulEq);
            case '/': RETURN_PAIR(Tok_DivEq);
            case '!': RETURN_PAIR(Tok_NotEq);
            case '>': RETURN_PAIR(Tok_GrtrEq);
            case '<': RETURN_PAIR(Tok_LesrEq);
        }
    }
    
    if PAIR('.', '.') RETURN_PAIR(Tok_StrCat);

    if(c == 0 || c == EOF){
        return {Tok_EndOfInput};
    }

    //If the character is nota, assume it is an operator and store
    //the character in the string for identification
    char* s = (char*)malloc(2);
    s[0] = c;
    s[1] = '\0';
    Token op = {Tok_Operator, s};
    incPos();
    return op;
}
