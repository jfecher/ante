#include "lexer.h"
using namespace zyl;

map<string, TokenType> keywords = {
    {"struct",   Struct},
    {"class",    Class},
    {"enum",     Enum},
    {"if",       If},
    {"elif",     Elif},
    {"else",     Else},
    {"import",   Import},
    {"match",    Match},
    {"for",      For},
    {"foreach",  Foreach},
    {"in",       In},
    {"do",       Do},
    {"while",    While},
    {"continue", Continue},
    {"break",    Break},
    {"where",    Where},
};

map<string, Modifiers> modifiers = {
    {"pub",      Pub},
    {"pri",      Pri},
    {"pro",      Pro},
    {"const",    Const},
    {"dyn",      Dyn},
};

map<string, Literals> literals = {
    {"true",     True},
    {"false",    False}, 
};

map<string, DataTypes> types = {
    {"i8",       I8},
    {"i16",      I16},
    {"i32",      I32},
    {"i64",      I64},
    {"u8",       U8},
    {"u16",      U16},
    {"u32",      U32},
    {"u64",      U64},
    {"f32",      F32},
    {"f64",      F64},
    {"str",      Str},
    {"bool",     Bool},
    {"void",     Void}, 
};

Lexer::Lexer(ifstream in){
    swap(in, f);
    incPos();
    incPos();
    scope = 0;
}

void Lexer::incPos(void){
    c = n;
    n = f.get();
}

inline void Lexer::skipTo(char end){
    do incPos();
    while(c != end && c != '\0');
    if(c == end) incPos();
}

inline Token Lexer::skipComment(void){
    if(c == '~')
        skipTo('\n');
    else if(c == '`')
        skipTo('`');
    return getNextToken();
}

Token Lexer::genWhitespaceToken(void){
    if(c == ' ' || c == '\t'){ //skip
        do incPos();
        while((c == ' ' || c == '\t') && c != '\0');
        return getNextToken();
    }else{ //Determine to issue indent/unindent token
        incPos();
        unsigned int newScope = 0;
        while(IS_WHITESPACE(c)){
            switch(c){
                case ' ':  newScope++;
                case '\t': newScope += SCOPE_STEP;
                case '\n': newScope = 0;
                default:
                    break;
            }
        }

        if(IS_COMMENT(c)) return skipComment();

        newScope = newScope / SCOPE_STEP;
        if(newScope > scope){
            curScope = scope + 1;
            scope = newScope;
            return {TokenType::Indent};
        }else if(newScope < scope){
            curScope = scope - 1;
            scope = newScope;
            return {TokenType::Unindent};
        }else{
            return {TokenType::Newline};
        }
    }
}

Token Lexer::genAlphaNumericToken(void){
    string *s = new string();
    while(IS_ALPHA_NUMERIC(c)){
        s += c;
        incPos();
    }
   
    try{
        TokenType t = keywords.at(*s);
        return {t};
    }catch(out_of_range e){
        try{
            DataTypes t = types.at(*s);
            return {TokenType::DataType, t};
        }catch(out_of_range e){
            try{
                Modifiers m = modifiers.at(*s);
                return {TokenType::Modifier, m};
            }catch(out_of_range e){
                try{
                    Literals l = literals.at(*s);
                    return {TokenType::Literal, l};
                }catch(out_of_range e){}
            }
        }
    }
    return {TokenType::Identifier, 0, *s};
}

Token Lexer::genNumLit(void){
    string *s = new string();
    Literals type = Literals::IntLit;
    while(IS_NUMERIC(c)){
        s += c;
        incPos();
    }
    if(c == '.' && IS_NUMERIC(n)){
        type = Literals::FloatLit;
        do{
            s += c;
            incPos();
        }while(IS_NUMERIC(c));
    }
        
    return LITTOK(type);
}

Token Lexer::genStrLit(void){
    string *s = new string();
    char delim = c;
    incPos();

    while(c != delim && c != '\0'){
        s += c;
        incPos();
    }

    return {TokenType::Literal, Literals::StrLit, *s}; 
}

int state[256] = {
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
    17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
    33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48,
    49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64,
    65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80,
    81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96,
    97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112,
    113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128,
    129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144,
    145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160,
    161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176,
    177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192,
    193, 194, 195, 196, 197, 198, 199, 200, 201, 202, 203, 204, 205, 206, 207, 208,
    209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224,
    225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240,
    241, 242, 243, 244, 245, 246, 247, 248, 249, 250, 251, 252, 253, 254, 255, 256
};

Token Lexer::getNextToken(void){
    //Check if an unindent/indent of multiple
    //levels was encountered by genWhitespaceToken
    if(curScope != scope){
        if(curScope > scope){
            curScope -= 1;
            return {TokenType::Unindent};
        }else{
            curScope += 1;
            return {TokenType::Indent};
        }
    }

    if(c == EOF || c == '\0')
        return {TokenType::EndOfInput};

    //skip comments
    if(IS_COMMENT(c)) return skipComment();
    if(IS_WHITESPACE(c)) return genWhitespaceToken();
    if(IS_NUMERIC(c)) return genNumLit();
    if(IS_ALPHA_NUMERIC(c)) return genAlphaNumericToken();
    if(c == '\'' || c=='"') return genStrLit();

    //operator
    string *s = new string();
    switch(c){
        case '=':
            incPos();
            if(n == '='){
                incPos();
                return BOPTOK(BinaryOps::Eq);
            }
            return {TokenType::Assign};
        case '(': return {TokenType::OpUnary, UnaryOps::ParenOpen};
        case ')':
        default: break;
    }
    return {TokenType::EndOfInput};
}
