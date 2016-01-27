#include "lexer.h"
#include <cstdlib>
#include <cstring>

/*
 *  Maps each non-literal token to a string representing
 *  its type.
 */
map<int, const char*> tokDict = {
    {Tok_Ident, "Identifier"},
    {Tok_UserType, "UserType"},

    //types
    {Tok_I8, "I8"},
    {Tok_I16, "I16"},
    {Tok_I32, "I32"},
    {Tok_I64, "I64"},
    {Tok_U8, "U8"},
    {Tok_U16, "U16"},
    {Tok_U32, "U32"},
    {Tok_U64, "U64"},
    {Tok_Isz, "Isz"},
    {Tok_Usz, "Usz"},
    {Tok_F32, "F32"},
    {Tok_F64, "F64"},
    {Tok_C8, "C8"},
    {Tok_C32, "C32"},
    {Tok_Bool, "Bool"},
    {Tok_Void, "Void"},

    {Tok_Eq, "Eq"},
    {Tok_NotEq, "NotEq"},
    {Tok_AddEq, "AddEq"},
    {Tok_SubEq, "SubEq"},
    {Tok_MulEq, "MulEq"},
    {Tok_DivEq, "DivEq"},
    {Tok_GrtrEq, "GrtrEq"},
    {Tok_LesrEq, "LesrEq"},
    {Tok_Or, "Or"},
    {Tok_And, "And"},

    //literals
    {Tok_True, "True"},
    {Tok_False, "False"},
    {Tok_IntLit, "IntLit"},
    {Tok_FltLit, "FltLit"},
    {Tok_StrLit, "StrLit"},

    //keywords
    {Tok_Return, "Return"},
    {Tok_If, "If"},
    {Tok_Elif, "Elif"},
    {Tok_Else, "Else"},
    {Tok_For, "For"},
    {Tok_While, "While"},
    {Tok_Do, "Do"},
    {Tok_In, "In"},
    {Tok_Continue, "Continue"},
    {Tok_Break, "Break"},
    {Tok_Import, "Import"},
    {Tok_Match, "Match"},
    {Tok_Data, "Data"},
    {Tok_Enum, "Enum"},

    //modifiers
    {Tok_Pub, "Pub"},
    {Tok_Pri, "Pri"},
    {Tok_Pro, "Pro"},
    {Tok_Raw, "Raw"},
    {Tok_Const, "Const"},
    {Tok_Ext, "Ext"},
    {Tok_Pathogen, "Pathogen"},

    //other
    {Tok_Where, "Where"},
    {Tok_Infect, "Infect"},
    {Tok_Cleanse, "Cleanse"},
    {Tok_Ct, "Ct"},

    {Tok_Newline, "Newline"},
    {Tok_Indent, "Indent"},
    {Tok_Unindent, "Unindent"},
};

/*
 *  Maps each keyword to its corresponding TokenType
 */
map<string, int> keywords = {
    {"i8",       Tok_I8},
    {"i16",      Tok_I16},
    {"i32",      Tok_I32},
    {"i64",      Tok_I64},
    {"u8",       Tok_U8},
    {"u16",      Tok_U16},
    {"u32",      Tok_U32},
    {"u64",      Tok_U64},
    {"isz",      Tok_Isz},
    {"usz",      Tok_Usz},
    {"f32",      Tok_F32},
    {"f64",      Tok_F64},
    {"c8",       Tok_C8},
    {"c32",      Tok_C32},
    {"bool",     Tok_Bool},
    {"void",     Tok_Void},
    
    {"or",       Tok_Or},
    {"and",      Tok_And},
    {"true",     Tok_True},
    {"false",    Tok_False},
    
    {"return",   Tok_Return},
    {"if",       Tok_If},
    {"elif",     Tok_Elif},
    {"else",     Tok_Else},
    {"for",      Tok_For},
    {"while",    Tok_While},
    {"do",       Tok_Do},
    {"in",       Tok_In},
    {"continue", Tok_Continue},
    {"break",    Tok_Break},
    {"import",   Tok_Import},
    {"match",    Tok_Match},
    {"data",     Tok_Data},
    {"enum",     Tok_Enum},
    
    {"pub",      Tok_Pub},
    {"pri",      Tok_Pri},
    {"pro",      Tok_Pro},
    {"raw",      Tok_Raw},
    {"const",    Tok_Const},
    {"ext",      Tok_Ext},
    {"pathogen", Tok_Pathogen},

    //other
    {"where",    Tok_Where},
    {"infect",   Tok_Infect},
    {"cleanse",  Tok_Cleanse},
    {"ct",       Tok_Ct},
};

char c = 0; 
char n = 0;
char* lextxt = 0;
ifstream *in;
#define scStep 4

/*
 *  Current scope (indent level) of file
 */
int scope;

/*
 *  Used to remember a new indentation level to issue multiple Indent
 *  or Unindent tokens when required.
 */
int cscope;

void ante::lexer::init(const char* file)
{
    in = new ifstream(file);
    c = 0;
    n = 0;
    incPos();
    incPos();
    scope = 0;
    cscope = 0;
}

int yylex(...)
{
    return ante::lexer::next();
}

/*
 *  Prints a token's type to stdout
 */
void ante::lexer::printTok(int t)
{
    cout << getTokStr(t).c_str();
}

/*
 *  Translates a token's type to a string
 */
string ante::lexer::getTokStr(int t)
{
    string s = "";
    if(IS_LITERAL(t)){
        s += (char)t;
    }else{
        s += tokDict[t];
    }
    return s;
}

inline void ante::lexer::incPos(void)
{
    c = n;
    if(in->good())
        in->get(n);
    else
        n = 0;
}

void ante::lexer::incPos(int end)
{
    for(int i = 0; i < end; i++){
        c = n;
        if(in->good())
            in->get(n);
        else
            n = 0;
    }
}

int ante::lexer::handleComment(void)
{
    if(c == '`'){
        do incPos(); while(c != '`' && c != EOF);
        incPos();
    }else{ // c == '~'
        while(c != '\n' && c != EOF) incPos();
    }
    return next();
}

/*
 *  Allocates a new string for lextxt without
 *  freeing its previous value.  The previous value
 *  should always be stored in a node during parsing
 *  and freed later.
 */
void ante::lexer::setlextxt(string *str)
{
    size_t size = str->size() + 1;
    lextxt = (char*)malloc(size);
    strcpy(lextxt, str->c_str());
    lextxt[size-1] = '\0';
}

int ante::lexer::genAlphaNumTok()
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
        setlextxt(&s);
        return (s[0] >= 'A' && s[0] <= 'Z') ? Tok_UserType : Tok_Ident;
    }
}

int ante::lexer::genNumLitTok()
{
    string s = "";
    bool flt = false;

    while(IS_NUMERICAL(c) || (c == '.' && !flt && IS_NUMERICAL(n)) || c == '_'){
        if(c != '_'){
            s += c;
            if(c == '.'){ 
                flt = true;
            }
        }
        incPos();
    }

    setlextxt(&s);
    return flt? Tok_FltLit : Tok_IntLit;
}

int ante::lexer::genWsTok()
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
            if(IS_COMMENT(c)) return ante::lexer::handleComment();
        }
        newScope /= scStep;

        if(newScope == scope){
            return Tok_Newline; /* Scope did not change, just return a Newline */
        }
        scope = newScope;
        return next();
    }else{
        incPos();
        return next();
    }
}

int ante::lexer::genStrLitTok(char delim)
{
    string s = "";
    incPos();
    while(c != delim && c != EOF){
        if(c == '\\'){
            switch(n){
                case 'a': s += '\a'; break;
                case 'b': s += '\b'; break;
                case 'f': s += '\f'; break;
                case 'n': s += '\n'; break;
                case 'r': s += '\r'; break;
                case 't': s += '\t'; break;
                case 'v': s += '\v'; break;
                default:  s += n; break;
            }
            incPos();
        }else{
            s += c;
        }
        incPos();
    }
    incPos();
    setlextxt(&s);
    return Tok_StrLit;
}

int ante::lexer::next()
{
    if(scope != cscope){
        if(scope > cscope){
            cscope++;
            return Tok_Indent;
        }else{
            cscope--;
            return Tok_Unindent;
        }
    }

    if(IS_COMMENT(c))    return ante::lexer::handleComment();
    if(IS_NUMERICAL(c))  return ante::lexer::genNumLitTok();
    if(IS_ALPHANUM(c))   return ante::lexer::genAlphaNumTok();
    if(IS_WHITESPACE(c)) return ante::lexer::genWsTok();

    if(c == '"' || c == '\'') 
        return ante::lexer::genStrLitTok(c);

    //substitute -> for an indent and ;; for an unindent
    if(PAIR('-', '>')){
        scope++;
        RETURN_PAIR(next());
    }else if(PAIR(';', ';')){
        scope--;
        RETURN_PAIR(next());
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
    
    if(c == 0 || c == EOF) return 0; //End of input

    //If the character is nota, assume it is an operator and store
    //the character in the string for identification
    char ret = c;
    incPos();
    return ret;
}
