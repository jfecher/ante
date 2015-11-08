#include "lexer.h"

const char* tokDictionary[] = {
    "Tok_EndOfInput",
    "Tok_Ident",

    //types
    "Tok_I8",
    "Tok_I16",
    "Tok_I32",
    "Tok_I64",
    "Tok_U8",
    "Tok_U16",
    "Tok_U32",
    "Tok_U64",
    "Tok_F32",
    "Tok_F64",
    "Tok_Bool",
    "Tok_Void",

	"Tok_Eq",
    "Tok_NotEq",
	"Tok_AddEq",
	"Tok_SubEq",
    "Tok_MulEq",
    "Tok_DivEq",
	"Tok_GrtrEq",
	"Tok_LesrEq",
    "Tok_Add",
    "Tok_Sub",
    "Tok_Mul",
    "Tok_Div",
    "Tok_Or",
    "Tok_And",
    "Tok_True",
    "Tok_False",
	"Tok_IntLit",
	"Tok_FltLit",
	"Tok_StrLit",
    "Tok_StrCat",

    "Tok_ParenOpen",
    "Tok_ParenClose",
    "Tok_BraceOpen",
    "Tok_BraceClose",
    "Tok_BracketOpen",
    "Tok_BracketClose",

    //keywords
    "Tok_Return",
	"Tok_If",
    "Tok_Elif",
	"Tok_Else",
	"Tok_For",
	"Tok_ForEach",
	"Tok_While",
    "Tok_Do",
    "Tok_In",
	"Tok_Continue",
	"Tok_Break",
    "Tok_Import",
    "Tok_Where",
    "Tok_Enum",
    "Tok_Struct",
    "Tok_Class",

    "Tok_Newline",
    "Tok_Indent",
    "Tok_Unindent",
};

map<const char*, Token> keywords = {
    {"i8",       {Tok_I8,       NULL, 0, 0}},
    {"i16",      {Tok_I16,      NULL, 0, 0}},
    {"i32",      {Tok_I32,      NULL, 0, 0}},
    {"i64",      {Tok_I64,      NULL, 0, 0}},
    {"u8",       {Tok_U8,       NULL, 0, 0}},
    {"u16",      {Tok_U16,      NULL, 0, 0}},
    {"u32",      {Tok_U32,      NULL, 0, 0}},
    {"u64",      {Tok_U64,      NULL, 0, 0}},
    {"f32",      {Tok_F32,      NULL, 0, 0}},
    {"f64",      {Tok_F64,      NULL, 0, 0}},
    {"bool",     {Tok_Bool,     NULL, 0, 0}},
    {"void",     {Tok_Void,     NULL, 0, 0}},
    
    {"or",       {Tok_Or,       NULL, 0, 0}},
    {"and",      {Tok_And,      NULL, 0, 0}},
    {"true",     {Tok_True,     NULL, 0, 0}},
    {"false",    {Tok_False,    NULL, 0, 0}},
    
    {"return",   {Tok_Return,   NULL, 0, 0}},
    {"if",       {Tok_If,       NULL, 0, 0}},
    {"elif",     {Tok_Elif,     NULL, 0, 0}},
    {"else",     {Tok_Else,     NULL, 0, 0}},
    {"for",      {Tok_For,      NULL, 0, 0}},
    {"foreach",  {Tok_ForEach,  NULL, 0, 0}},
    {"while",    {Tok_While,    NULL, 0, 0}},
    {"do",       {Tok_Do,       NULL, 0, 0}},
    {"in",       {Tok_In,       NULL, 0, 0}},
    {"continue", {Tok_Continue, NULL, 0, 0}},
    {"break",    {Tok_Break,    NULL, 0, 0}},
    {"import",   {Tok_Import,   NULL, 0, 0}},
    {"where",    {Tok_Where,    NULL, 0, 0}},
    {"enum",     {Tok_Enum,     NULL, 0, 0}},
    {"struct",   {Tok_Struct,   NULL, 0, 0}},
    {"class",    {Tok_Class,    NULL, 0, 0}},
};

map<const char*, Token> operators = {
    {"+", {Tok_Add, NULL, 0, 0}},
    {"-", {Tok_Sub, NULL, 0, 0}},
    {"*", {Tok_Mul, NULL, 0, 0}},
    {"/", {Tok_Div, NULL, 0, 0}},
};

Lexer::Lexer(istream **f)
{
    in = *f;
    incPos();
    incPos();
    scope = 0;
    cscope = 0;
}

void Lexer::incPos()
{
    c = n;
    *in >> n;

    cout << c;
}

void Lexer::incPos(int end)
{
    for(int i = 0; i < end; i++){
        c = n;
        *in >> n;
    }
}

Token Lexer::handleComment()
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

    while(IS_NUMERICAL(c) || (c == '.' && !flt)){
        s += c;
        if(c == '.') flt = true;
    }
    return {flt? Tok_FltLit : Tok_IntLit, s.c_str()};
}

Token Lexer::genWsTok()
{
    if(c == '\n'){
        unsigned short newScope;
        
        while(IS_WHITESPACE(c)){
            switch(c){
                case ' ': newScope++; break;
                case '\t': newScope += scStep; break;
                case '\n': newScope = 0; break;
                default: break;
            }
        }
        newScope /= scStep;

        if(newScope == scope){
            return {Tok_Newline};
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
    cout << 1;
    if(cscope != scope){
        if(cscope > scope){
            cscope++;
            return {Tok_Indent};
        }else{
            cscope--;
            return {Tok_Unindent};
        }
    }

    cout << 2;
    if(IS_COMMENT(c))    return Lexer::handleComment();
    cout << 3;
    if(IS_NUMERICAL(c))  return Lexer::genNumLitTok();
    cout << 4;
    if(IS_ALPHANUM(c))   return Lexer::genAlphaNumTok();
    cout << 5;
    if(IS_WHITESPACE(c)) return Lexer::genWsTok();
    cout << 6;
    
    switch(c){
        case '"': return Lexer::genStrLitTok();
        case '-':
            if(n == '>'){
                incPos(2);
                scope++;
                return next();
            }
            break;
        case '(': return {Tok_ParenOpen};
        case ')': return {Tok_ParenClose};
        case '[': return {Tok_BraceOpen};
        case ']': return {Tok_BraceClose};
        case '{': return {Tok_BracketOpen};
        case '}': return {Tok_BracketClose};
        case EOF: case 0: return {Tok_EndOfInput};
        default: break;
    }
    
    string s = "";
    cout << 7;
    while(!IS_COMMENT(c) && !(IS_ALPHANUM(c)) && !IS_WHITESPACE(c) && c != EOF && c != 0){
        s += c;
        incPos();
    }

    cout << 8;
    auto op = operators.find(s.c_str());
    if(op != operators.end()){
        return op->second;
    }else{
        cout << "Unknown operator token '" << s << "'\n";
        exit(1);
    }
}
