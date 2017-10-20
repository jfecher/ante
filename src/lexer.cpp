#include "lexer.h"
#include "lazystr.h"
#include <cstdlib>
#include <cstring>

using namespace ante;
using namespace std;

/*
 *  Maps each non-literal token to a string representing
 *  its type.
 */
map<int, const char*> tokDict = {
    {Tok_Ident, "Identifier"},
    {Tok_UserType, "UserType"},
    {Tok_TypeVar, "TypeVar"},

    //types
    {Tok_I8, "i8"},
    {Tok_I16, "i16"},
    {Tok_I32, "i32"},
    {Tok_I64, "i64"},
    {Tok_U8, "u8"},
    {Tok_U16, "u16"},
    {Tok_U32, "u32"},
    {Tok_U64, "u64"},
    {Tok_Isz, "isz"},
    {Tok_Usz, "usz"},
    {Tok_F16, "f16"},
    {Tok_F32, "f32"},
    {Tok_F64, "f64"},
    {Tok_C8, "c8"},
    {Tok_C32, "c32"},
    {Tok_Bool, "bool"},
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
    {Tok_True, "true"},
    {Tok_False, "false"},
    {Tok_IntLit, "IntLit"},
    {Tok_FltLit, "FltLit"},
    {Tok_StrLit, "StrLit"},
    {Tok_CharLit, "CharLit"},

    //keywords
    {Tok_Return, "return"},
    {Tok_If, "if"},
    {Tok_Then, "then"},
    {Tok_Elif, "elif"},
    {Tok_Else, "else"},
    {Tok_For, "for"},
    {Tok_While, "while"},
    {Tok_Do, "do"},
    {Tok_In, "in"},
    {Tok_Continue, "continue"},
    {Tok_Break, "break"},
    {Tok_Import, "import"},
    {Tok_Let, "let"},
    {Tok_Var, "var"},
    {Tok_Match, "match"},
    {Tok_With, "with"},
    {Tok_Type, "type"},
    {Tok_Trait, "trait"},
    {Tok_Fun, "fun"},
    {Tok_Ext, "ext"},
    {Tok_Block, "block"},

    //pseudo-keywords
    {Tok_Self, "self"},

    //modifiers
    {Tok_Pub, "pub"},
    {Tok_Pri, "pri"},
    {Tok_Pro, "pro"},
    {Tok_Raw, "raw"},
    {Tok_Const, "const"},
    {Tok_Noinit, "noinit"},
    {Tok_Mut, "mut"},
    {Tok_Global, "global"},
    {Tok_Ante, "ante"},

    //reserved
    {Tok_Where, "where"},

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
    {"block",    Tok_Block},

    {"self",     Tok_Self},

    {"pub",      Tok_Pub},
    {"pri",      Tok_Pri},
    {"pro",      Tok_Pro},
    {"raw",      Tok_Raw},
    {"const",    Tok_Const},
    {"noinit",   Tok_Noinit},
    {"mut",      Tok_Mut},
    {"global",   Tok_Global},
    {"ante",     Tok_Ante},

    //reserved
    {"where",    Tok_Where},
};


/* Raw text to store identifiers and usertypes in */
char *lextxt;

Lexer *yylexer;

bool ante::colored_output = true;

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
 * Initializes lexer from a filename to be opened
 * If file = nullptr then stdin will be opened instead
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
    shouldReturnNewline(false),
    printInput(false)
{
    if(file){
        in = new ifstream(*file);
        fileName = file;
    }else{
        in = (ifstream*) &cin;
        fileName = new string("stdin");
    }

    if(!*in){
        cerr << "Error: Unable to open file '" << *file << "'\n";
        exit(EXIT_FAILURE);
    }

    incPos();
    incPos();
    scopes->push(0);

    if(cur == '#' && nxt == '!')
        while(cur != '\n') incPos();
}


/*
 * Initializes lexer from a string, the 'pseudofile' to be
 * lexed instead of an actual file
 */
Lexer::Lexer(string* fName, string& pFile,
        unsigned int ro, unsigned int co, bool pi) :
    isPseudoFile(true),
    row{1},
    col{1},
    rowOffset{ro},
    colOffset{co},
    cur{0},
    nxt{0},
    scopes{new stack<unsigned int>()},
    cscope{0},
    shouldReturnNewline(false),
    printInput(pi)
{
    fileName = fName;
    pseudoFile = (char*)pFile.c_str();

    if(pFile.length() >= 1){
        col += 2;
        cur = *(pseudoFile++);
        nxt = *(pseudoFile++);
    }else if(pFile.length() == 1){
        col++;
        cur = *(pseudoFile++);
    }

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

namespace ante{
    namespace parser{
        yy::position mkPos(string*, unsigned int, unsigned int);
    }
}

yy::position Lexer::getPos(bool inclusiveEnd) const{
    return parser::mkPos(fileName, row + rowOffset, col + colOffset -
                (inclusiveEnd ? 0 : 1));
}

bool isKeywordAType(int tok){
    return tok == Tok_I8 or tok == Tok_I16 or tok == Tok_I32 or tok == Tok_I64 or tok == Tok_Isz
        or tok == Tok_U8 or tok == Tok_U16 or tok == Tok_U32 or tok == Tok_U64 or tok == Tok_Usz
        or tok == Tok_F16 or tok == Tok_F32 or tok == Tok_F64
        or tok == Tok_C8 or tok == Tok_C32 or tok == Tok_Bool or tok == Tok_Void;
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
        nxt = !nxt ? 0 : *(pseudoFile++);
    }else{
        if(in->good())
            in->get(nxt);
		else {
//fix a windows lexing bug where the last character is duplicated before an eof
#ifdef _WIN32
			cur = 0;
#endif
			nxt = 0;
		}
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
        incPos();

        if(printInput){
            setTermFGColor(AN_COMMENT_COLOR);
            putchar('/');
            putchar('*');
        }

        do{
            incPos();
            if(cur == '\n'){
                row++;
                col = 0;

            if(printInput)
                putchar(cur);

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

        if(printInput && nxt == '/')
            putchar('/');

        incPos();
        incPos();
    }else{ //single line comment
        while(cur != '\n' && cur != '\0'){
            if(printInput)
                putchar(cur);
            incPos();
        }
    }
    setTermFGColor(AN_CONSOLE_RESET);
    return next(loc);
}

/*
*  Allocates a new string for lextxt without
*  freeing its previous value.  The previous value
*  should always be stored in a node during parsing
*  and freed later.
*/
void Lexer::setlextxt(string &str){
    lextxt = strdup(str.c_str());
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
        if(printInput)
            cout << AN_TYPE_COLOR << s << AN_CONSOLE_RESET;
        setlextxt(s);
        return Tok_UserType;
    }else{ //ident or keyword
        auto key = keywords.find(s.c_str());
        if(key != keywords.end()){
            if(printInput)
                cout << (isKeywordAType(key->second) ? AN_TYPE_COLOR : AN_KEYWORD_COLOR)
                     << key->first << AN_CONSOLE_RESET;
            return key->second;
        }else{//ident
            if(printInput)
                cout << s;
            setlextxt(s);
            return Tok_Ident;
        }
    }
}

int Lexer::genNumLitTok(yy::parser::location_type* loc){
    string s = "";
    bool flt = false;
    loc->begin = getPos();

    if(printInput)
        cout << AN_CONSTANT_COLOR;

    while(IS_NUMERICAL(cur) || (cur == '.' && !flt && IS_NUMERICAL(nxt)) || cur == '_'){
        if(cur != '_'){
            s += cur;
            if(cur == '.') flt = true;
        }
        if(printInput)
            putchar(cur);
        incPos();
    }

    //check for type suffix
    if(flt){
        if(cur == 'f'){
            if(printInput)
                putchar('f');

            s += 'f';
            incPos();
            if(cur == '1' && nxt == '6'){
                if(printInput)
                    fputs("16", stdout);
                s += "16";
                incPos();
                incPos();
            }else if(cur == '3' && nxt == '2'){
                if(printInput)
                    fputs("32", stdout);
                s += "32";
                incPos();
                incPos();
            }else if(cur == '6' && nxt == '4'){
                if(printInput)
                    fputs("64", stdout);
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
            if(printInput)
                putchar(cur);

            s += cur;
            incPos();
            if(cur == '8'){
                if(printInput)
                    fputs("8", stdout);
                s += '8';
                incPos();
            }else if(cur == '1' && nxt == '6'){
                if(printInput)
                    fputs("16", stdout);
                s += "16";
                incPos();
                incPos();
            }else if(cur == '3' && nxt == '2'){
                if(printInput)
                    fputs("32", stdout);
                s += "32";
                incPos();
                incPos();
            }else if(cur == '6' && nxt == '4'){
                if(printInput)
                    fputs("64", stdout);
                s += "64";
                incPos();
                incPos();
            }else if(cur == 's' && nxt == 'z'){
                if(printInput)
                    fputs("sz", stdout);
                s += "sz";
                incPos();
                incPos();
            }

            if(IS_NUMERICAL(cur)){
                loc->end = getPos();
                lexErr("Extraneous numbers after type suffix.", loc);
            }
        }
    }

    if(printInput)
        cout << AN_CONSOLE_RESET;

    loc->end = getPos(false);
    setlextxt(s);
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

            if(printInput)
                putchar(cur);

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

        if(abs((long)cscope - (long)newScope) < 2)
            lexErr("Changes in significant whitespace cannot be less than 2 spaces in size", loc);

        loc->end = getPos();
        cscope = newScope;
        return next(loc);
    }else{
        return skipWsAndReturnNext(loc);
    }
}

int Lexer::skipWsAndReturnNext(yy::parser::location_type* loc){
    do{
        if(printInput)
            putchar(cur);

        incPos();
    }while(cur == ' ');
    return next(loc);
}

int Lexer::genStrLitTok(yy::parser::location_type* loc){
    string s = "";
    loc->begin = getPos();

    incPos();

    if(!cur){
        if(printInput)
            putchar('"');
        return '"';
    }

    if(printInput)
        cout << AN_STRING_COLOR << '"';

    while(cur != '"' && cur != '\0'){
        if(cur == '\\'){
            if(printInput)
                putchar('\\');
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
                            if(printInput)
                                putchar(nxt);
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

        if(printInput)
            putchar(cur);
        incPos();
    }

    loc->end = getPos();

    if(printInput){
        if(cur == '"') putchar('"');
        cout << AN_CONSOLE_RESET;
    }

	if(cur != '"')
		lexErr("Missing closing string delimiter", loc);

    incPos(); //consume ending delim
    setlextxt(s);
    return Tok_StrLit;
}

int Lexer::genCharLitTok(yy::parser::location_type* loc){
    string s = "";
    loc->begin = getPos();
    bool hasEscapeSequence = false;
    incPos();

    if(!cur){
        if(printInput)
            putchar('\'');
        return '\'';
    }

    if(cur == '\\'){
        //because there is an escape sequence, this is known to be a string
        if(printInput)
            cout << AN_STRING_COLOR << "'\\'" << nxt;

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
                        if(printInput)
                            putchar(nxt);
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

        if(nxt != '\''){
            loc->end = getPos();
            lexErr("Missing terminating ' for character literal", loc);
        }

        hasEscapeSequence = true;
    }else{
        s += cur;
    }

    incPos();

    if(cur != '\''){ //typevar
        s = '\'' + s;

        while(IS_ALPHANUM(cur)){
            s += cur;
            incPos();
        }
        
        if(printInput)
            cout << AN_TYPE_COLOR << s << AN_CONSOLE_RESET;

        loc->end = getPos(false);
        setlextxt(s);
        return Tok_TypeVar;
    }

    if(printInput){
        if(!hasEscapeSequence){
            if(printInput){
                cout << AN_STRING_COLOR << '\'' << s[0];
            }
        }
        if(cur == '\'')
            putchar('\'');
        cout << AN_CONSOLE_RESET;
    }

    loc->end = getPos();
    setlextxt(s);
    incPos();
    return Tok_CharLit;
}

#define RETURN_PAIR(t){\
if(printInput){        \
    putchar(cur);      \
    putchar(nxt);      \
}                      \
incPos(2);             \
loc->end = getPos();   \
return (t);            \
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
        if(printInput){
            putchar('\\');
            putchar('\n');
        }
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

    if(printInput && ret)
        putchar(ret);
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
        if(printInput) putchar('(');                            \
        return '(';                                             \
    }                                                           \
                                                                \
    if(cur == '['){                                             \
        matchingToks.push(']');                                 \
        incPos();                                               \
        if(printInput) putchar('[');                            \
        return '[';                                             \
    }                                                           \
                                                                \
                                                                \
    if(matchingToks.size() > 0 && cur == matchingToks.top()){   \
        int top = matchingToks.top();                           \
        matchingToks.pop();                                     \
        if(printInput) putchar(cur);                            \
        incPos();                                               \
        return top;                                             \
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
        if(matchingToks.size() > 0){
            //make sure to maintain line count if a newline is skipped in skipWsAndReturnNext
            if(cur == '\n'){
                row++;
                col = 0;
            }
            return skipWsAndReturnNext(loc);
        }else{
            return genWsTok(loc);
        }
    }

    CHECK_FOR_MATCHING_TOKS();

    //IF NOTA, then the token must be an operator.
    //if not, return it by value anyway.
    return genOpTok(loc);
}


void Lexer::lexErr(const char *msg, yy::parser::location_type* loc){
    //If printInput is specified, the user may still be typing
    if(!printInput){
        error(msg, *loc);
        exit(EXIT_FAILURE);//lexing errors are always fatal
    }
}
