#include "lexer.h"

char current, lookAhead; //The current and lookAhead chars from src
TokenType prevType; //The previous TokenType found.  Initialized with Tok_Begin

char*color = RESET_COLOR;
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
    {Tok_Print,        "print"},
    {Tok_Return,       "return"},
    {Tok_If,           "if"},
    {Tok_Else,         "else"},
    {Tok_For,          "for"},
    {Tok_While,        "while"},
    {Tok_String,       "string"},
    {Tok_Num,          "num"},
    {Tok_Continue,     "continue"},
    {Tok_Break,        "break"},
    {Tok_Boolean,      "bool"},
    {Tok_Char,         "char"},
    {Tok_BooleanTrue,  "true"},
    {Tok_BooleanFalse, "false"},
    {Tok_Import,       "import"}
};

inline void ralloc(char **ptr, size_t size){
    char *tmp = realloc(*ptr, size);
    if(tmp != NULL){
        *ptr = tmp;
    }else{
        puts("ralloc: Memory Leak\n");
        exit(11);
    }
}

Token getNextToken(){
    if(current == EOF || current == '\0'){
        Token tok = {Tok_EndOfInput, NULL};
        return tok;
    }

    //Skip comments
    if(current == '~'){ //Single line comment.  Skip until newline
        while(lookAhead != '\n') incrementPos();
        return getNextToken();
    }else if(current == '`'){ //Multi line comment.  Skip until next `
        do
            incrementPos();
        while(current != '`' && current != EOF && current != '\0');

        incrementPos();
        return getNextToken();
    }


    //Check if char is numeric, alphanumeric, or whitespace, and return corresponding
    //TokenType with full lexeme.  Note that isNumeric is checked first,
    //This ensures identifiers/keywords cannot begin with a number
    if(IS_WHITESPACE(current))         return genWhitespaceToken();
    else if(IS_NUMERIC(current))       return genNumericalToken();
    else if(IS_ALPHA_NUMERIC(current)) return genAlphaNumericalToken();

    Token tok = {Tok_Invalid, NULL};
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
        }else if(lookAhead == '-'){
            incrementPos();
            tok.lexeme[1] = '-';
            tok.type = Tok_Function;
        }else if(lookAhead == '>'){
            incrementPos();
            tok.lexeme[1] = '>';
            tok.type = Tok_TypeDef;
        }
        else tok.type = Tok_Minus;
        break;
    case '"':; // ; is not a typo, it allows i to be decalred by inserting an empty statement
        int i;
        incrementPos();
        tok.lexeme[0] = '\0';
        for(i=0; current != '"' && current != '\0'; i++){
            ralloc(&tok.lexeme, sizeof(char) * (i+3));
            tok.lexeme[i] = current;
            tok.lexeme[i+1] = '\0';
            incrementPos();
        }

        if(current == '"') 
            tok.type = Tok_StringLiteral;
        else 
            tok.type = Tok_MalformedString;
        break;
    case '\'':
        incrementPos();

        tok.lexeme[0] = '\0';
        for(i=0; current != '\'' && current != '\0'; i++){
            ralloc(&tok.lexeme, sizeof(char) * (i+3));
            tok.lexeme[i] = current;
            tok.lexeme[i+1] = '\0';
            incrementPos();
        }

        if(current=='\'') 
            tok.type = Tok_CharLiteral;
        else 
            tok.type = Tok_MalformedChar;
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
        Token tok = {Tok_Newline, NULL};
        int newScope = 0;

        while(1){
            if(current == ' ')       newScope++;
            else if(current == '\t') newScope+=4;
            else if(current == '\n') newScope ^= newScope; //reset newScope

            if(IS_WHITESPACE(lookAhead)) incrementPos();
            else break;
        }

        //Reset level if it stopped on a comment
        if(lookAhead == '~' || lookAhead == '`') return getNextToken();

        //Compare the new scope with the old.  Assign TokenType as necessary
        if(newScope > scope)
            tok.type = Tok_Indent;
        else if(newScope < scope)
            tok.type = Tok_Unindent;

        scope = newScope;
        incrementPos();
        return tok;
    }else{
        while(IS_WHITESPACE(current) && current != '\n' && current != 13) {
            if(printToks) printf(" ");
            incrementPos(); //Skip the whitespace, except for newlines
        }
        return getNextToken();
    }
}


Token genAlphaNumericalToken(){ //fail at length =
    Token tok;
    tok.lexeme = calloc(sizeof(char), 1);
    int i;

    for(i=0; IS_ALPHA_NUMERIC(current); i++){
        ralloc(&tok.lexeme, sizeof(char) * (i+2));
        tok.lexeme[i] = current;
        tok.lexeme[i+1] = '\0';
        incrementPos();
    }

    for(i=0; i < sizeof(dictionary) / sizeof(dictionary[0]); i++){
        if(strcmp(tok.lexeme, dictionary[i].lexeme) == 0){
            tok.type = dictionary[i].type;
            return tok;
        }
    }

    tok.type = Tok_Identifier;
    return tok;
}

Token genNumericalToken(){ //fail at length = 126
    Token tok;
    tok.lexeme = calloc(sizeof(char), 1);
    char isDouble = 0;
    int i;

    for(i=0; IS_NUMERIC(current) || current == '.'; i++){
        ralloc(&tok.lexeme, sizeof(char) * (i+2));
        tok.lexeme[i] = current;
        tok.lexeme[i+1] = '\0';
        if(current=='.') isDouble = 1;
        incrementPos();
    }

    tok.type = isDouble? Tok_DoubleLiteral : Tok_IntegerLiteral;
    return tok;
}

void incrementPos(){
    current = lookAhead;

    if(isTty){
        lookAhead = pos[0];
        pos++;
    }else{
        lookAhead = fgetc(src);
    }
}

void printTok(Token t){
    switch(t.type){
    case Tok_String: 
    case Tok_Char:
    case Tok_Num: 
    case Tok_For: 
    case Tok_If: 
    case Tok_While: 
    case Tok_Import: 
    case Tok_Break: 
    case Tok_Continue: 
    case Tok_Else: 
    case Tok_Return: 
    case Tok_Print:
        printf(KEYWORD_COLOR "%s" RESET_COLOR, t.lexeme);
        break;
    case Tok_StringLiteral:
        printf(STRINGL_COLOR "\"%s\"" RESET_COLOR, t.lexeme);
        break;
    case Tok_IntegerLiteral: 
    case Tok_DoubleLiteral:
        printf(INTEGERL_COLOR "%s" RESET_COLOR, t.lexeme);
        break;
    case Tok_MalformedString:
        printf(STRINGL_COLOR "\"%s" RESET_COLOR, t.lexeme);
        break;
    case Tok_CharLiteral:
        printf(STRINGL_COLOR "'%s'" RESET_COLOR, t.lexeme);
        break;
    case Tok_MalformedChar:
        printf(STRINGL_COLOR "'%s" RESET_COLOR, t.lexeme);
        break;
    case Tok_Function:
        printf(FUNCTION_COLOR "--");
        color = FUNCTION_COLOR;
        break;
    case Tok_Colon:
    case Tok_Minus:
    case Tok_ParenOpen:
        printf(RESET_COLOR "%s", t.lexeme);
        color = RESET_COLOR;
        break;
    default:
        printf("%s%s\033[1;m", color, t.lexeme);
    }
}

Token* lexer_next(char b){
    int i;
    printToks = b;
    Token *tok = malloc(sizeof(Token));
    tok[0] = getNextToken();

    if(printToks)
        printf("\r" RESET_COLOR ": ");

    for(i = 1; tok[i-1].type != Tok_EndOfInput; i++)
    {
        if(printToks && !IS_WHITESPACE_TOKEN(tok[i-1]))
            printTok(tok[i-1]);
        
        tok = realloc(tok, sizeof(Token) * (i+1));
        tok[i] = getNextToken();
    }
    color = RESET_COLOR;
    return tok;
}

void lexAndPrint(){
    initialize_lexer(0);
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

void freeToks(Token **t){
    int i;
    for(i = 0; (*t)[i].type != Tok_EndOfInput; i++)
        if((*t)[i].lexeme != NULL)
            free((*t)[i].lexeme);
    free(*t);
}

void initialize_lexer(char tty){ //Sets up the lookAhead character properly so that
    if(tty){
        isTty = 1;
        pos = srcLine;
        current = 0;
        lookAhead = 0;
    }else if(!src){
        printf("ERROR: source file not found.\n");
        exit(7);
    }

    incrementPos(); //The current character is not null
    incrementPos();
}
