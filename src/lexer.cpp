#include "lexer.h"
#include <cstring>

const char* tokDictionary[] = {
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
    "Isz",
    "Usz",
    "F32",
    "F64",
    "C8",
    "C32",
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
    "Range",
    "Beginning Exclusive Range", //TODO: find a better name
    "Ending Exclusive Range",
    "Exclusive Range",

    //literals
    "True",
    "False",
	"IntLit",
	"FltLit",
	"StrLit",

    //keywords
    "Return",
	"If",
    "Elif",
	"Else",
	"For",
	"While",
    "Do",
    "In",
	"Continue",
	"Break",
    "Import",
    "Match",
    "Enum",

    //modifiers
    "Pub",
    "Pri",
    "Pro",
    "Const",
    "Ext",
    "Dyn",
    "Pathogen",

    //other
    "Where",
    "Infect",
    "Cleanse",
    "Ct",

    "Newline",
    "Indent",
    "Unindent",
};

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
    {"enum",     Tok_Enum},
    
    {"Pub",      Tok_Pub},
    {"Pri",      Tok_Pri},
    {"Pro",      Tok_Pro},
    {"Const",    Tok_Const},
    {"Ext",      Tok_Ext},
    {"Dyn",      Tok_Dyn},
    {"Pathogen", Tok_Pathogen},

    //other
    {"Where",    Tok_Where},
    {"Infect",   Tok_Infect},
    {"Cleanse",  Tok_Cleanse},
    {"Ct",       Tok_Ct},
};

char c = 0; 
char n = 0;
static string yytext;
ifstream *in;
const char scStep = 4;

unsigned short scope;
unsigned short cscope;

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

extern "C" int yylex(...)
{
    return ante::lexer::next();
}

void ante::lexer::printTok(int t)
{
    if(IS_LITERAL(t))
        cout << (char)t << "\t\t" << t << endl;
    else
        cout << TOK_TYPE_STR(t) << "\t\t" << t << endl;
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
        yytext = s;
        return Tok_Ident;
    }
}

int ante::lexer::genNumLitTok()
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

    yytext = s;
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
            return Tok_Newline;
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
        s += c;
        incPos();
    }
    incPos();
    yytext = s;
    return Tok_StrLit;
}

int ante::lexer::next()
{
    if(cscope != scope){
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
        RETURN_PAIR(Tok_Indent);
    }else if(PAIR(';', ';')){
        RETURN_PAIR(Tok_Unindent);
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
    
    if(PAIR('.', '.')) RETURN_PAIR(Tok_Range);
    if(PAIR('^', '.')) RETURN_PAIR(Tok_RangeBX);
    if(PAIR('.', '^')) RETURN_PAIR(Tok_RangeEX);
    if(PAIR('^', '^')) RETURN_PAIR(Tok_RangeX);

    //If a backslash is placed before a newline, skip
    //the newline token
    if(PAIR('\\', '\n')) RETURN_PAIR(ante::lexer::next());

    if(c == 0 || c == EOF) return 0; //End of input

    //If the character is nota, assume it is an operator and store
    //the character in the string for identification
    char ret = c;
    yytext = ret;
    incPos();
    return ret;
}
