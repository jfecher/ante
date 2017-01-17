#include "lexer.h"
#include <cstdlib>
#include <cstring>

using namespace ante;

/*
 *  Maps each non-literal token to a string representing
 *  its type.
 */
map<int, const char*> tokDict = {
    {Tok_Ident, "Identifier"},
    {Tok_UserType, "UserType"},
    {Tok_TypeVar, "TypeVar"},

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
    {Tok_F16, "F16"},
    {Tok_F32, "F32"},
    {Tok_F64, "F64"},
    {Tok_C8, "C8"},
    {Tok_C32, "C32"},
    {Tok_Bool, "Bool"},
    {Tok_Void, "Void"},

    {Tok_Eq, "=="},
    {Tok_NotEq, "!="},
    {Tok_AddEq, "+="},
    {Tok_SubEq, "-="},
    {Tok_MulEq, "*="},
    {Tok_DivEq, "/="},
    {Tok_GrtrEq, ">="},
    {Tok_LesrEq, "<="},
    {Tok_Or, "or"},
    {Tok_And, "and"},
    {Tok_Range, ".."},
    {Tok_RArrow, "->"},
    {Tok_ApplyL, "<|"},
    {Tok_ApplyR, "|>"},
    {Tok_Append, "++"},
    {Tok_New, "new"},
    {Tok_Not, "not"},

    //literals
    {Tok_True, "True"},
    {Tok_False, "False"},
    {Tok_IntLit, "IntLit"},
    {Tok_FltLit, "FltLit"},
    {Tok_StrLit, "StrLit"},

    //keywords
    {Tok_Return, "Return"},
    {Tok_If, "If"},
    {Tok_Then, "Then"},
    {Tok_Elif, "Elif"},
    {Tok_Else, "Else"},
    {Tok_For, "For"},
    {Tok_While, "While"},
    {Tok_Do, "Do"},
    {Tok_In, "In"},
    {Tok_Continue, "Continue"},
    {Tok_Break, "Break"},
    {Tok_Import, "Import"},
    {Tok_Let, "Let"},
    {Tok_Var, "Var"},
    {Tok_Match, "Match"},
    {Tok_With, "With"},
    {Tok_Type, "Type"},
    {Tok_Trait, "Trait"},
    {Tok_Fun, "Fun"},
    {Tok_Ext, "Ext"},

    //modifiers
    {Tok_Pub, "pub"},
    {Tok_Pri, "pri"},
    {Tok_Pro, "pro"},
    {Tok_Raw, "raw"},
    {Tok_Const, "const"},
    {Tok_Noinit, "noinit"},
    {Tok_Mut, "mut"},

    //other
    {Tok_Where, "Where"},
    
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
    {"f16",      Tok_F16},
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
    {"new",      Tok_New},
    {"not",      Tok_Not},

    {"return",   Tok_Return},
    {"if",       Tok_If},
    {"then",     Tok_Then},
    {"elif",     Tok_Elif},
    {"else",     Tok_Else},
    {"for",      Tok_For},
    {"while",    Tok_While},
    {"do",       Tok_Do},
    {"in",       Tok_In},
    {"continue", Tok_Continue},
    {"break",    Tok_Break},
    {"import",   Tok_Import},
    {"let",      Tok_Let},
    {"var",      Tok_Var},
    {"match",    Tok_Match},
    {"with",     Tok_With},
    {"type",     Tok_Type},
    {"trait",    Tok_Trait},
    {"fun",      Tok_Fun},
    {"ext",      Tok_Ext},
    
    {"pub",      Tok_Pub},
    {"pri",      Tok_Pri},
    {"pro",      Tok_Pro},
    {"raw",      Tok_Raw},
    {"const",    Tok_Const},
    {"noinit",   Tok_Noinit},
    {"mut",      Tok_Mut},

    //other
    {"where",    Tok_Where},
};

        
/* Raw text to store identifiers and usertypes in */
char *lextxt;

Lexer *yylexer;


/* Sets lexer instance for yylex to use */
void setLexer(Lexer *l){
    if(yylexer)
        delete yylexer;
    
    yylexer = l;
}

int yylex(yy::parser::semantic_type* st, yy::location* yyloc){
    return yylexer->next(yyloc);
}



/*
 * Initializes lexer
 */
Lexer::Lexer(string* file) :
    isPseudoFile(false),
    row{1},
    col{1},
    rowOffset{0},
    colOffset{0},
    cur{0},
    nxt{0},
    scopes{new stack<unsigned int>()},
    cscope{0},
    shouldReturnNewline(false)
{
    if(file){
        in = new ifstream(*file);
        fileName = file;
    }else{
        in = (ifstream*) &cin;
        fileName = new string("stdin");
    }

    if(!*in){
        cerr << "Error: Unable to open file '" << file << "'\n";
        exit(EXIT_FAILURE);
    }

    incPos();
    incPos();
    scopes->push(0);
}


Lexer::Lexer(string* fName, string& pFile, unsigned int ro, unsigned int co) :
    isPseudoFile(true),
    row{1},
    col{1},
    rowOffset{ro},
    colOffset{co},
    cur{0},
    nxt{0},
    scopes{new stack<unsigned int>()},
    cscope{0},
    shouldReturnNewline(false)
{
    fileName = fName;
    pseudoFile = (char*)pFile.c_str();

    incPos();
    incPos();

    scopes->push(0);
}

Lexer::~Lexer(){
    delete scopes;
    if(!isPseudoFile && in != &cin)
        delete in;
}

char Lexer::peek() const{
    return cur;
}

extern yy::position mkPos(string*, unsigned int, unsigned int);

yy::position Lexer::getPos(bool inclusiveEnd) const{
    return mkPos(fileName, row + rowOffset, col + colOffset -
                (inclusiveEnd ? 0 : 1));
}

/*
*  Prints a token's type to stdout
*/
void Lexer::printTok(int t){
    cout << getTokStr(t);
}

/*
*  Translates a token's type to a string
*/
string Lexer::getTokStr(int t){
    string s = "";
    if(IS_LITERAL(t)){
        s += (char)t;
    }else{
        s += tokDict[t];
    }
    return s;
}

inline void Lexer::incPos(){
    cur = nxt;
    col++;

    if(isPseudoFile){
        nxt = *(pseudoFile++);
    }else{
        if(in->good())
            in->get(nxt);
        else
            nxt = 0;
    }
}

void Lexer::incPos(int end){
    for(int i = 0; i < end; i++){
        incPos();
    }
}


int Lexer::handleComment(yy::parser::location_type* loc){
    if(nxt == '*'){
        int level = 1;

        do{
            incPos();
            if(cur == '\n'){
                row++;
                col = 0;

            //handle nested comments
            }else if(cur == '/' && nxt == '*'){
                level += 1;
            }else if(cur == '*' && nxt == '/'){
                if(level != 0){
                    level -= 1;
                }else{
                    loc->end = getPos();
                    lexErr("Extraneous closing comment", loc);
                }
            }
        }while(level && cur != '\0');
        incPos();
        incPos();
    }else{ //single line comment
        while(cur != '\n' && cur != '\0') incPos();
    }
    return next(loc);
}

/*
*  Allocates a new string for lextxt without
*  freeing its previous value.  The previous value
*  should always be stored in a node during parsing
*  and freed later.
*/
void Lexer::setlextxt(string *str){
    size_t size = str->size() + 1;
    lextxt = (char*)malloc(size);
    strcpy(lextxt, str->c_str());
    lextxt[size-1] = '\0';
}

int Lexer::genAlphaNumTok(yy::parser::location_type* loc){
    string s = "";
    loc->begin = getPos();

    bool isUsertype = cur >= 'A' && cur <= 'Z';
    if(isUsertype){
        while(IS_ALPHANUM(cur)){
            if(cur == '_'){
                loc->end = getPos();
                lexErr("Usertypes cannot contain an underscore.", loc);
            }

            s += cur;
            incPos();
        }
    }else{
        while(IS_ALPHANUM(cur)){
            s += cur;
            incPos();
        }
    }
   
    loc->end = getPos(false);

    if(isUsertype){
        setlextxt(&s);
        return Tok_UserType;
    }else{ //ident or keyword
        auto key = keywords.find(s.c_str());
        if(key != keywords.end()){
            return key->second;
        }else{//ident
            setlextxt(&s);
            return Tok_Ident;
        }
    }
}

int Lexer::genNumLitTok(yy::parser::location_type* loc){
    string s = "";
    bool flt = false;
    loc->begin = getPos();

    while(IS_NUMERICAL(cur) || (cur == '.' && !flt && IS_NUMERICAL(nxt)) || cur == '_'){
        if(cur != '_'){
            s += cur;
            if(cur == '.') flt = true;
        }
        incPos();
    }

    //check for type suffix
    if(flt){
        if(cur == 'f'){
            s += 'f';
            incPos();
            if(cur == '1' && nxt == '6'){
                s += "16";
                incPos();
                incPos();
            }else if(cur == '3' && nxt == '2'){
                s += "32";
                incPos();
                incPos();
            }else if(cur == '6' && nxt == '4'){
                s += "64";
                incPos();
                incPos();
            }
            
            if(IS_NUMERICAL(cur)){
                loc->end = getPos();
                lexErr("Extraneous numbers after type suffix.", loc);
            }
        }
    }else{
        if(cur == 'i' || cur == 'u'){
            s += cur;
            incPos();
            if(cur == '8'){
                s += '8';
                incPos();
            }else if(cur == '1' && nxt == '6'){
                s += "16";
                incPos();
                incPos();
            }else if(cur == '3' && nxt == '2'){
                s += "32";
                incPos();
                incPos();
            }else if(cur == '6' && nxt == '4'){
                s += "64";
                incPos();
                incPos();
            }

            if(IS_NUMERICAL(cur)){
                loc->end = getPos();
                lexErr("Extraneous numbers after type suffix.", loc);
            }
        }
    }
    
    loc->end = getPos(false);
    setlextxt(&s);
    return flt? Tok_FltLit : Tok_IntLit;
}

int Lexer::genWsTok(yy::parser::location_type* loc){
    if(cur == '\n'){
        loc->begin = getPos();
        
        unsigned int newScope = 0;

        while(IS_WHITESPACE(cur) && cur != '\0'){
            switch(cur){
                case ' ': newScope++; break;
                case '\n': 
                    newScope = 0; 
                    row++; 
                    col = 0; 
                    loc->begin = getPos();
                    break;
                case '\t':
                    loc->end = getPos();
                    lexErr("Tab characters are invalid whitespace.", loc);
                default: break;
            }
            incPos();
            if(IS_COMMENT(cur, nxt)) return handleComment(loc);
        }

        if(!scopes->empty() && newScope == scopes->top()){
            //do not return an end-of-file Newline
            if(!nxt) return 0;
            
            //the row is not set to row for newline tokens in case there are several newlines.
            //In this case, if set to row, it would become the row of the last newline.
            //Incrementing it from its previous token (guarenteed to be non-newline) fixes this.
            loc->end = getPos();
            return Tok_Newline; /* Scope did not change, just return a Newline */
        }
        loc->end = getPos();
        cscope = newScope;
        return next(loc);
    }else{
        return skipWsAndReturnNext(loc);
    }
}

int Lexer::skipWsAndReturnNext(yy::location* loc){
    do{
        incPos();
    }while(cur == ' ');
    return next(loc);
}

int Lexer::genStrLitTok(yy::parser::location_type* loc){
    string s = "";
    loc->begin = getPos();
    incPos();
    while(cur != '"' && cur != '\0'){
        if(cur == '\\'){
            switch(nxt){
                case 'a': s += '\a'; break;
                case 'b': s += '\b'; break;
                case 'f': s += '\f'; break;
                case 'n': s += '\n'; break;
                case 'r': s += '\r'; break;
                case 't': s += '\t'; break;
                case 'v': s += '\v'; break;
                default: 
                    if(!IS_NUMERICAL(nxt)) s += nxt;
                    else{
                        int cha = 0;
                        incPos();
                        while(IS_NUMERICAL(cur) && IS_NUMERICAL(nxt)){
                            cha *= 8;
                            cha += cur - '0';
                            incPos();
                        }
                        //the final char must be added here becuase cur and nxt
                        //must not be consumed until after the switch
                        cha *= 8;
                        cha += cur - '0';

                        s += cha;
                        in->putback(nxt);
                        nxt = cur;
                    }
                    break;
            }


            incPos();
        }else{
            s += cur;
        }
        incPos();
    }

    loc->end = getPos();
    incPos(); //consume ending delim

    setlextxt(&s);
    return Tok_StrLit;
}

int Lexer::genCharLitTok(yy::parser::location_type* loc){
    string s = "";
    loc->begin = getPos();

    incPos();
    if(cur == '\\'){
        switch(nxt){
            case 'a': s += '\a'; break;
            case 'b': s += '\b'; break;
            case 'f': s += '\f'; break;
            case 'n': s += '\n'; break;
            case 'r': s += '\r'; break;
            case 't': s += '\t'; break;
            case 'v': s += '\v'; break;
            case '0': s += '\0'; break;
            default:
                if(!IS_NUMERICAL(nxt)) s += nxt;
                else{
                    int cha = 0;
                    incPos();
                    while(IS_NUMERICAL(cur) && IS_NUMERICAL(nxt)){
                        cha *= 8;
                        cha += cur - '0';
                        incPos();
                    }
                    //the final char must be added here becuase cur and nxt
                    //must not be consumed until after the switch
                    cha *= 8;
                    cha += cur - '0';

                    s += cha;
                    in->putback(nxt);
                    nxt = cur;
                }
                break;
        }
        incPos();
    }else{
        s += cur;
    }

    incPos();

    if(cur != '\''){ //typevar
        while(IS_ALPHANUM(cur)){
            s += cur;
            incPos();
        }
        loc->end = getPos(false);
        setlextxt(&s);
        return Tok_TypeVar;
    }

    loc->end = getPos();
    setlextxt(&s);
    incPos();
    return Tok_CharLit;
}

/*
 *  Returns an operator token from the lexer's current position.
 *  Checks for possible multi-char operators, and if none are found,
 *  returns the token by its value.
 */
int Lexer::genOpTok(yy::parser::location_type* loc){
    if(cur == '"') 
        return genStrLitTok(loc);
    if(cur == '\'')
        return genCharLitTok(loc);
   
    //If the token is none of the above, it must be a symbol, or a pair of symbols.
    //Set the beginning of the token about to be created here.
    loc->begin = getPos();

    if(cur == '\\' && nxt == '\n'){ //ignore newline
        incPos(2);
        col = 1;
        row++;
        return next(loc);
    }

    if(cur == '.' && nxt == '.') RETURN_PAIR(Tok_Range);
    if(cur == '-' && nxt == '>') RETURN_PAIR(Tok_RArrow);
    
    if(cur == '<' && nxt == '|') RETURN_PAIR(Tok_ApplyL);
    if(cur == '|' && nxt == '>') RETURN_PAIR(Tok_ApplyR);
    
    if(cur == '+' && nxt == '+') RETURN_PAIR(Tok_Append);

    if(nxt == '='){
        switch(cur){
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
    
    loc->end = getPos();
    
    if(cur == 0) return 0; //End of input

    //If the character is nota, assume it is an operator and return it by value.
    char ret = cur;
    incPos();
    return ret;
}


/*
 *  Psuedo-function macro to check for () [] and {} tokens and
 *  match them when necessary.  Only used in next() function.
 */
#define CHECK_FOR_MATCHING_TOKS() {                             \
    if(cur == '('){                                             \
        matchingToks.push(')');                                 \
        incPos();                                               \
        return '(';                                             \
    }                                                           \
                                                                \
    if(cur == '['){                                             \
        matchingToks.push(']');                                 \
        incPos();                                               \
        return '[';                                             \
    }                                                           \
                                                                \
                                                                \
    if(matchingToks.size() > 0 && cur == matchingToks.top()){   \
        int top = matchingToks.top();                           \
        matchingToks.pop();                                     \
        incPos();                                               \
        return top == '}' ? Tok_Unindent : top;                 \
    }                                                           \
}


int Lexer::next(yy::parser::location_type* loc){
    if(shouldReturnNewline){
        shouldReturnNewline = false;
        return Tok_Newline;
    }

    if(scopes->top() != cscope){
        if(cscope > scopes->top()){
            scopes->push(cscope);
            return Tok_Indent;
        }else{
            scopes->pop();

            //do not return an end-of-file newline
            shouldReturnNewline = (bool) nxt;
            return Tok_Unindent;
        }
    }

    if(IS_COMMENT(cur, nxt)) return handleComment(loc);
    if(IS_NUMERICAL(cur))    return genNumLitTok(loc);
    if(IS_ALPHANUM(cur))     return genAlphaNumTok(loc);
    
    //only check for significant whitespace if the lexer is not trying to match brackets.
    if(IS_WHITESPACE(cur)){
        if(matchingToks.size() > 0)
            return skipWsAndReturnNext(loc);
        else
            return genWsTok(loc);
    }

    CHECK_FOR_MATCHING_TOKS();

    //IF NOTA, then the token must be an operator.
    //if not, return it by value anyway.
    return genOpTok(loc);
}


void Lexer::lexErr(const char *msg, yy::parser::location_type* loc){
    error(msg, *loc);
    exit(EXIT_FAILURE);//lexing errors are always fatal
}
