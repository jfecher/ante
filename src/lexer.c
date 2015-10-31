#include "lexer.h"

char current, lookAhead; //The current and lookAhead chars from src
TokenType prevType; //The previous TokenType found.  Initialized with Tok_Begin

char *srcLine;
char *pos;

//Level of spacing in the src.  Used to identify where to give Indent,
//Unindent, or Newline tokens.
int scope;

Token genWhitespaceToken();
Token genAlphaNumericalToken();
Token genNumericalToken();
void incrementPos();

//Dictionary used for checking if a string is a keyword, and if so,
//associating it with the corresponding TokenType
Token dictionary[] = {
    {Tok_Print,        "print",   0,0},
    {Tok_Return,       "return",  0,0},
    {Tok_If,           "if",      0,0},
    {Tok_Else,         "else",    0,0},
    {Tok_For,          "for",     0,0},
    {Tok_ForEach,      "foreach", 0,0},
    {Tok_In,           "in",      0,0},
    {Tok_While,        "while",   0,0},
    {Tok_String,       "str",  0,0},
    {Tok_Num,          "num",     0,0},
    {Tok_Int,          "int",     0,0},
    {Tok_Continue,     "continue",0,0},
    {Tok_Break,        "break",   0,0},
    {Tok_Boolean,      "bool",    0,0},
    {Tok_BooleanTrue,  "true",    0,0},
    {Tok_BooleanFalse, "false",   0,0},
    {Tok_Import,       "import",  0,0}
};

inline void ralloc(char **ptr, size_t size){
    char *tmp = realloc(*ptr, size);
    if(tmp != NULL){
        *ptr = tmp;
    }else{
        puts("ralloc: Could not allocate memory.\n");
        exit(11);
    }
}

Token getNextToken(){
    if(current == EOF || current == '\0'){
        return (Token) {Tok_EndOfInput, NULL, row, col};
    }

    //Skip comments
    if(current == '~'){ //Single line comment.  Skip until newline
        do 
            incrementPos();
        while(current != '\n' && lookAhead != EOF);
        return getNextToken();
    }else if(current == '`'){ //Multi line comment.  Skip until next `
        do
            incrementPos();
        while(current != '`' && current != EOF && current != '\0');

        if(current == '`') 
            incrementPos();

        return getNextToken();
    }

    //Check if char is numeric, alphanumeric, or whitespace, and return corresponding
    //TokenType with full lexeme.  Note that isNumeric is checked before isAlphaNumeric,
    //This ensures identifiers/keywords cannot begin with a number
    if(IS_WHITESPACE(current))         return genWhitespaceToken();
    else if(IS_NUMERIC(current))       return genNumericalToken();
    else if(IS_ALPHA_NUMERIC(current)) return genAlphaNumericalToken();

    Token tok = {Tok_Invalid, NULL, row, col};
    tok.lexeme = calloc(sizeof(char), 3);
    tok.lexeme[0] = current;
    switch(current){ //Here at last: the glorified switch statement
    case '>':
        if(lookAhead == '='){
            tok.type = Tok_GreaterEquals;
            tok.lexeme[1] = '=';
            incrementPos();
        }
        else tok.type = Tok_Greater;
        break;
    case '<':
        if(lookAhead == '='){
            incrementPos();
            tok.lexeme[1] = '=';
            tok.type = Tok_LesserEquals;
        }
        else tok.type = Tok_Lesser;
        break;
    case '|':
        if(lookAhead == '|'){
            incrementPos();
            tok.lexeme[1] = '|';
            tok.type = Tok_BooleanOr;
        }
        else tok.type = Tok_ListInitializer;
        break;
    case '&':
        if(lookAhead == '&'){
            incrementPos();
            tok.lexeme[1] = '&';
            tok.type = Tok_BooleanAnd;
        }
        else tok.type = Tok_Invalid;
        break;
    case '=':
        if(lookAhead == '='){
            incrementPos();
            tok.lexeme[1] = '=';
            tok.type = Tok_EqualsEquals;
        }
        else tok.type = Tok_Assign;
        break;
    case '+':
        if(lookAhead == '='){
            incrementPos();
            tok.lexeme[1] = '=';
            tok.type = Tok_PlusEquals;
        }
        else tok.type = Tok_Plus;
        break;
    case '-':
        if(lookAhead == '='){
            incrementPos();
            tok.lexeme[1] = '=';
            tok.type = Tok_MinusEquals;
        }else if(lookAhead == '>'){
            incrementPos();
            tok.lexeme[1] = '>';
            tok.type = Tok_TypeDef;
        }else if(IS_NUMERIC(lookAhead)){
            free(tok.lexeme);
            return genNumericalToken(); 
        }
        else tok.type = Tok_Minus;
        break;
    case '"': // ; is not a typo, it allows c to be decalred by inserting an empty statement
    case '\'':;
        char c = current; 
        tok.lexeme[0] = '\0';
        
        if(lookAhead != '\0' && lookAhead != EOF){
            incrementPos();
       
            int i = 0;
            for(; current != c && lookAhead != '\0'; i++, incrementPos()){
                ralloc(&tok.lexeme, sizeof(char) * (i+3));
                tok.lexeme[i] = current;
                tok.lexeme[i+1] = '\0';
            }
  
            if(current == c){
                tok.type = Tok_StringLiteral;
            }else{ 
                tok.type = Tok_MalformedString;
                ralloc(&tok.lexeme, sizeof(char) * (i+3)); //input did not end in ' or " so add the final char anyways
                tok.lexeme[i] = current;
                tok.lexeme[i+1] = '\0';
            }
            break;
        }

        tok.type = Tok_MalformedString;
        break;
    case '*':
        if(lookAhead == '='){
            incrementPos();
            tok.lexeme[1] = '=';
            tok.type = Tok_MultiplyEquals;
        }
        else tok.type = Tok_Multiply;
        break;
    case '/':
        if(lookAhead == '='){
            incrementPos();
            tok.lexeme[1] = '=';
            tok.type = Tok_DivideEquals;
        }else tok.type = Tok_Divide;
        break ;
    case '.':
        if(lookAhead == '.'){
            incrementPos();
            tok.lexeme[1] = '.';
            tok.type = Tok_StrConcat;
        }else tok.type = Tok_Invalid;
        break;
    case '!':
        if(lookAhead == '='){
            incrementPos();
            tok.lexeme[1] = '=';
            tok.type = Tok_NotEquals;
        }else tok.type = Tok_Not;
        break;
    case '%':
        tok.type = Tok_Modulus;
        break;
    case ',':
        tok.type = Tok_Comma;
        break;
    case ':':
        tok.type = Tok_Colon;
        break;
    case '(':
        tok.type = Tok_ParenOpen;
        break;
    case ')':
        tok.type = Tok_ParenClose;
        break;
    case '[':
        tok.type = Tok_BracketOpen;
        break;
    case ']':
        tok.type = Tok_BracketClose;
        break;
    case '^':
        tok.type = Tok_Exponent;
        break;
    case '\0': case -1:
        tok.type = Tok_EndOfInput;
        break;
    default:
        tok.type = Tok_Invalid;
        break;
    }

    incrementPos();
    return tok;
}


Token genWhitespaceToken(){
    //If the whitespace is a newline, then check to see if the scope has changed
    if(current == '\n' || current == 13){
        Token tok = {Tok_Newline, NULL, row, col};
        int newScope = 0;

        while(1){
            if(current == ' ')       newScope++;
            else if(current == '\t') newScope+=4;
            else if(current == '\n') newScope ^= newScope; //reset newScope

            if(IS_WHITESPACE(lookAhead)) incrementPos();
            else break;
        }

        //Reset level if it stopped on a comment
        if(lookAhead == '~' || lookAhead == '`'){ 
            incrementPos();
            return getNextToken();
        }

        //Compare the new scope with the old.  Assign TokenType as necessary
        if(newScope > scope)
            tok.type = Tok_Indent;
        else if(newScope < scope)
            tok.type = Tok_Unindent;

        scope = newScope;
        row++;
        col = -1;
        incrementPos();
        return tok;
    }else{
        while(IS_WHITESPACE(current) && current != '\n' && current != 13) {
            incrementPos(); //Skip the whitespace, except for newlines
        }
        return getNextToken();
    }
}


Token genAlphaNumericalToken(){ //fail at length =
    Token tok = {0, NULL, row, col};
    tok.lexeme = calloc(sizeof(char), 1);

    for(int i=0; IS_ALPHA_NUMERIC(current); i++){
        ralloc(&tok.lexeme, sizeof(char) * (i+2));
        tok.lexeme[i] = current;
        tok.lexeme[i+1] = '\0';
        incrementPos();
    }

    for(int i=0; i < sizeof(dictionary) / sizeof(dictionary[0]); i++){
        if(strcmp(tok.lexeme, dictionary[i].lexeme) == 0){
            tok.type = dictionary[i].type;
            return tok;
        }
    }

    tok.type = Tok_Identifier;
    return tok;
}

Token genNumericalToken(){
    Token tok = {0, NULL, row, col};
    tok.lexeme = calloc(sizeof(char), 1);
    char isDouble = 0;

    for(int i=0; IS_NUMERIC(current) || (current == '.' && !isDouble) || (i==0 && current=='-'); i++){
        ralloc(&tok.lexeme, sizeof(char) * (i+2));
        tok.lexeme[i] = current;
        tok.lexeme[i+1] = '\0';
        if(current=='.'){
            isDouble = 1;
        }
        incrementPos();
    }

    tok.type = isDouble? Tok_DoubleLiteral : Tok_IntegerLiteral;
    return tok;
}

void incrementPos(){
    current = lookAhead;

    if(isTty){
        lookAhead = *pos;
        pos++;
    }else{
        lookAhead = fgetc(src);
    }
    col++;
}

char* getTokColor(TokenType t){
    switch(t){
    case Tok_String: case Tok_Num: case Tok_Int:
        return TYPE_COLOR;
    case Tok_For: case Tok_ForEach: case Tok_In: case Tok_If: case Tok_While: case Tok_Import: case Tok_Break: case Tok_Continue: case Tok_Else: case Tok_Return: case Tok_Print:
        return KEYWORD_COLOR;
    case Tok_StringLiteral: case Tok_MalformedString:
        return STRINGL_COLOR;
    case Tok_IntegerLiteral: case Tok_DoubleLiteral: case Tok_BooleanTrue: case Tok_BooleanFalse:
        return INTEGERL_COLOR;
    case Tok_FuncCall: case Tok_FuncDef:
        return FUNCTION_COLOR;
    default:
        return RESET_COLOR;
    }
}

void printSrcLine(Token *t){
    unsigned int j = 0;
    for(unsigned int i = 0; srcLine[i] != '\0'; i++){
        if(t[j].type != Tok_EndOfInput && i + 1 == t[j].col){
            printf(getTokColor(t[j].type));
            j++;
        }
        
        putchar(srcLine[i]);
    }
    printf(RESET_COLOR);
}

Token* lexer_next(char b){
    printToks = b;
    Token *tok = malloc(sizeof(Token));
    tok[0] = getNextToken();

    for(int i = 1; tok[i-1].type != Tok_EndOfInput; i++)
    { 
        tok = realloc(tok, sizeof(Token) * (i+1));
        tok[i] = getNextToken();

        if(tok[i].type == Tok_ParenOpen && tok[i-1].type == Tok_Identifier)
            tok[i-1].type = Tok_FuncCall;
        else if(tok[i].type == Tok_Colon && tok[i-1].type == Tok_Identifier)
            tok[i-1].type = Tok_FuncDef; 
    }

    if(printToks) printSrcLine(tok);
    return tok;
}

void lexAndPrint(){
    init_lexer(0);
    Token *toks = lexer_next(0);
    int i;
    for(i = 0; toks[i].type != Tok_EndOfInput; i++){
        switch(toks[i].type){
        case Tok_Newline: 
        case Tok_Indent: 
        case Tok_Unindent:
            printf("     \t%s\n", tokenDictionary[toks[i].type]);
            break;
        default:
            printf("%s \t%s\n", toks[i].lexeme, tokenDictionary[toks[i].type]);
            break;
        }
        free(toks[i].lexeme);
    }
    free(toks);
}

inline void freeToks(Token **t){
    for(int i = 0; (*t)[i].type != Tok_EndOfInput; i++)
        NFREE((*t)[i].lexeme);
    NFREE(*t);
}

inline void freeSrcLine(){
    NFREE(srcLine);
}

//Initializes lexer.
//str is NULL if reading from a file,
//otherwise, tty mode is automatically set
void init_lexer(char* str){
    if(str){
        isTty = 1;
        srcLine = str;
        pos = str;
        current = 0;
        lookAhead = 0;
    }else if(!src){
        printf("ERROR: source file not found.\n");
        exit(7);
    }
    
    row = 1;
    col = -1;
    incrementPos(); //The current character is not null
    incrementPos();
}
