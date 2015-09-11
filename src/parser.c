#include "parser.h"

//#define DEBUG

Token *tokenizedInput;
unsigned int tokenIndex;

int class_def();
int class_body();
int class_statements();
int function_def();
int function_args();
int function_body();
int type();
int for_loop();
int while_loop();
int print_statement();
int if_statement();
int return_statement();
int statement();
int initialize_value();
int assign_value();
int variable();
int value();
int array_value();
int parse_expression();
int term();
int function_call();
int function_def_or_call();
int paren_expression();
int math_expression();

void debugLog(const char *s){
#ifdef DEBUG
    puts(s);
#endif
}

void syntaxError(const char* msg, int showErrTok){
    fprintf(stderr, "Syntax Error: %s", msg);
    if(showErrTok) fprintf(stderr, "%s (of type %s)", tokenizedInput[tokenIndex].lexeme, tokenDictionary[tokenizedInput[tokenIndex].type]);
    fprintf(stderr, " at row %d, col %d.\n", tokenizedInput[tokenIndex].row, tokenizedInput[tokenIndex].col);

    exitFlag = 1;
}

#define check(t) (tokenizedInput[tokenIndex].type == t)

int accept(TokenType type){
    if(check(type)){
        tokenIndex++;
        return 1;
    }
    return 0;
}

int _expect(TokenType type){
    if(accept(type)){
        return 1;
    }else{ //TODO: expand to include column and line number
        fprintf(stderr, "Syntax Error: Expected %s, but found %s at row %d, col %d.\n", tokenDictionary[type], tokenDictionary[tokenizedInput[tokenIndex].type], tokenizedInput[tokenIndex].row, tokenizedInput[tokenIndex].col);
        exitFlag = 2;
        return 0;
    }
}

#define expect(type) if(!_expect(type)){return 0;}

/* namespace
 *     : class_body EOF
 *     ;
 */
int namespace(){ //Start Point
    if(!class_body()) return 0;
    expect(Tok_EndOfInput);
    debugLog("\nParser: Finished");
    return 1;
}

/* class_statement
 *     : class_def
 *     | function_def
 *     | statement
 *     ;
 */
int class_statement(){
    debugLog("Parser: Entering class statement.");

    switch(tokenizedInput[tokenIndex].type){
        case Tok_TypeDef:  return class_def();
        case Tok_Function: return function_def_or_call();
        default:           return statement();
    }
    return 1;
}

/* class_body
 *     : class_statement NEWLINE class_statement...
 *     | class_statement
 *     ;
 */
int class_body(){ //May not work correctly for one-line classes
    debugLog("Parser: Entering class body.");

    while(!check(Tok_Unindent) && !check(Tok_EndOfInput) && !exitFlag){
        if(!class_statement()) return 0;
    }
    return 1;
}

/* class_def
 *     : TYPEDEF type > INDENT class_body UNINDENT
 *     ;
 */
int class_def(){
    debugLog("Parser: Entering class definition.");

    expect(Tok_TypeDef);
    expect(Tok_Identifier);
    expect(Tok_Greater);
    expect(Tok_Indent);
    if(!class_body()) return 0;
    expect(Tok_Unindent);
    return 1;
}

/* type
 *     : INT
 *     | STRING
 *     | DOUBLE
 *     | IDENTIFIER
 *     ;
 */
int type(){
    debugLog("Parser: Checking type.");
    if(check(Tok_Num) || check(Tok_Int) || check(Tok_String) || check(Tok_Boolean) || check(Tok_Identifier)){
        return tokenizedInput[tokenIndex].type;
    }else{
        return Tok_Invalid;
    }
}


int function_def_or_call(){
    expect(Tok_Function);
    Token n = tokenizedInput[tokenIndex + 1];
    if(n.type == Tok_Colon || n.type == Tok_ParenOpen) return function_call();
    else return function_def();
}

/* function_def
 *     : FUNCTION type > IDENTIFIER : function_args INDENT function_body UNINDENT
 *     | FUNCTION > IDENTIFIER : function_args INDENT function_body UNINDENT
 *     ;
 */
int function_def(){
    debugLog("Parser: defining function...");

    if(type() != Tok_Invalid){
        tokenIndex++;
    }
    expect(Tok_Greater);
    accept(Tok_Identifier);
    expect(Tok_Colon);
    if(!function_args()) return 0;
    if(!function_body()) return 0; //function_body checks within itself for an indent and unindent
    return 1;
}

/* function_args
 *     : type , type       Perhaps this should be changed to 'type_list'?
 *     | type
 *     ;
 */
int function_args(){
    debugLog("Parser: getting function args...");

    if(tokenizedInput[tokenIndex].type != Tok_Indent){

        while(tokenizedInput[tokenIndex + 1].type == Tok_Comma){
            if(type() == Tok_Invalid){
                syntaxError("Expected Type in function arguments.  Got ", 1);
                return 0;
            }

            debugLog("Parser: got a function arg.");
            tokenIndex += 2;
        }

        if(type() == Tok_Invalid){
            syntaxError("Expected Type in function arguments.  Got ", 1);
            return 0;
        }

        debugLog("Parser: got the final function arg.");

        tokenIndex++;
    }else{
        debugLog("Parser: no function args found.");
    }

    return 1;
}

/* value_list
 *     : value value_list
 *     | value
 *     ;
 */
int value_list(){
    if(!value()){ syntaxError("value_list: invalid value.  Got:", 1); return 0;}

    tokenIndex++;
    while(tokenizedInput[tokenIndex].type == Tok_Comma){
        if(value() == Tok_Invalid){
            syntaxError("value_list: invalid value in list.  Got:", 1);
            return 0;
        }
    }

    return 1;
}

/* function_body
 *     : statement NEWLINE statement
 *     | statement
 *     ;
 */
int function_body(){
    debugLog("Parser: entering function body.");
    expect(Tok_Indent);
    while(!(accept(Tok_Unindent) || check(Tok_EndOfInput))){
        debugLog("Parser: now finding next function statement.");
        if(!statement()) return 0;
        accept(Tok_Newline);
    }
    return 1;
}

/* function_call
 *     : FUNCTION IDENTIFIER : function_args
 *     | FUNCTION IDENTIFIER
 *     ;
 */
int function_call(){
    debugLog("Parser: Calling function.");
    expect(Tok_Identifier);
    if(accept(Tok_Colon) || accept(Tok_ParenOpen)){
        debugLog("Parser: getting value_list in function call.");
        if(!value_list()) return 0;
    }
    return 1;
}

/* statement
 *     : for_loop
 *     | while_loop
 *     | print_statement
 *     | initialize_value
 *     | assign_value
 *     | return_statement
 *     | if_statement
 *     | /              (used to force unindent token)(temporary)
 *     | NEWLINE
 *     ;
 */
int statement(){
    debugLog("Parser: entering statement.");

    switch(tokenizedInput[tokenIndex].type){
    case Tok_For:         return for_loop();
    case Tok_While:       return while_loop();
    case Tok_Print:       return print_statement();
    case Tok_Greater:     return initialize_value();
    case Tok_Identifier:  return assign_value();
    case Tok_Return:      return return_statement();
    case Tok_If:          return if_statement();
    case Tok_Function:    return function_call();
    case Tok_Divide:
    case Tok_Newline:
        tokenIndex++;
        return 1;
    default:
        if(type() != Tok_Invalid) initialize_value();
        else syntaxError("Invalid Statement starting with ", 1);
    }
    return 1;
}

/* for_loop
 *     : FOR assign_value , boolean_statement , assign_value INDENT function_body UNINDENT
 *     | FOR initialize_value , boolean_statement , assign_value INDENT function_body UNINDENT
 *     ;
 */
int for_loop(){
    debugLog("Parser: Entering for loop.");

    expect(Tok_For);
    if(check(Tok_Identifier) && tokenizedInput[tokenIndex + 1].type == Tok_Assign){
        tokenIndex++;
        if(!assign_value()) return 0;
    }else if(type() && tokenizedInput[tokenIndex + 1].type == Tok_Greater){
        tokenIndex++;
        if(!initialize_value()) return 0;
    }

    expect(Tok_Comma);
    if(!math_expression()) return 0;
    expect(Tok_Comma);

    if(!assign_value()) return 0;

    if(!function_body()) return 0;
    return 1;
}

/* while_loop
 *     : WHILE boolean_statement INDENT function_body UNINDENT
 *     ;
 */
int while_loop(){
    debugLog("Parser: Entering while loop.");

    expect(Tok_While);
    if(math_expression()) return 0;
    if(function_body()) return 0;
    return 1;
}

/* print_statement
 *     : PRINT expression , expression...
 *     ;
 */
int print_statement(){
    debugLog("Parser: entering print statement.");

    expect(Tok_Print);

    if(!math_expression()) return 0;

    while(accept(Tok_Comma)) if(!math_expression()) return 0;

    return 1;
}

/* initialize_value
 *     : type > IDENTIFIER assign_value
 *     | > IDENTIFIER assign_value
 *     | type > IDENTIFIER
 *     | > IDENTIFIER
 *     ;
 */
int initialize_value(){
    debugLog("Parser: initializing value...");

    if(type() != Tok_Invalid){
        debugLog("Parser: found type initialization.");
        tokenIndex++;
    }
    expect(Tok_Greater);
    if(tokenizedInput[tokenIndex + 1].type == Tok_Assign){
        debugLog("Parser: next token in initializion is an assignment.  Returning assign_value");
        if(!assign_value()) return 0;
        return 1;
    }else{
        if(!variable()){ syntaxError("No variable found to initialize.", 0); return 0;}
    }

    return 1; // >var   or   int>var    (No initialization of value)
}

/* assign_value
 *     : IDENTIFIER = expression
 *     ;
 */
int assign_value(){
    debugLog("Parser: assigning value...");

    if(!variable()){ syntaxError("No variable found to assign to.",0); return 0; }

    if(!(accept(Tok_Assign) || accept(Tok_PlusEquals) || accept(Tok_MinusEquals) || accept(Tok_MultiplyEquals) || accept(Tok_DivideEquals))){
        syntaxError("invalid token after variable. Got ", 1);
        return 0;
    }

    if(!math_expression()) return 0;
    return 1;
}

/* return_statement
 *     : RETURN expression
 *     ;
 */
int return_statement(){
    debugLog("Parser: Entering return statement.");

    expect(Tok_Return);
    if(!math_expression()) return 0;
    return 1;
}

/* if_statement
 *     : IF boolean_expression INDENT function_body UNINDENT ELSE INDENT function_body UNINDENT
 *     | IF boolean_expression INDENT function_body UNINDENT
 *     ;
 */
int if_statement(){
    debugLog("Parser: entering if statement.");

    expect(Tok_If);
    if(!math_expression()) return 0;
    if(!function_body()) return 0;

    if(accept(Tok_Else)){
        debugLog("Parser: entering else statement.");

        if(!function_body()) return 0;
    }
    return 1;
}

/* variable
 *     : array_value
 *     | IDENTIFIER
 *     ;
 */
int variable(){
    debugLog("Parser: Checking for variable.");

    if(tokenizedInput[tokenIndex + 1].type == Tok_BracketOpen){
        return array_value();
    }else{
        debugLog("Found Variable");
        return accept(Tok_Identifier);
    }
}

/* array_value
 *     : IDENTIFIER [ expression ]
 *     ;
 */
int array_value(){
    debugLog("Parser: Checking for array value.");
    expect(Tok_Identifier);
    expect(Tok_BracketOpen);
    if(!math_expression()) return 0;
    expect(Tok_BracketClose);
    return 1;
}

/* literal_value
 *     : array_value
 *     | INTEGERLITERAL
 *     | DOUBLELITERAL
 *     | STRINGLITERAL
 *     | IDENTIFIER
 *     ;
 */
int literal_value(){
    debugLog("Parser: Checking for literal value.");
    if(check(Tok_BooleanTrue) || check(Tok_BooleanFalse) || check(Tok_IntegerLiteral) || check(Tok_DoubleLiteral) || check(Tok_StringLiteral)){
        tokenIndex++;
        return 1;//c->type;
    }else{
        return 0;
    }
}


/* value
 *     : literal_value
 *     | variable
 *     ;
 */
int value(){
    debugLog("Parser: Checking for value.");
    return literal_value() || variable() || paren_expression();
}


int math_expression(){
    paren_expression();

    if(!value()) return 0;
    if(!parse_expression()) return 0;
    return 1;
}
/* expression
 *     : + expression
 *     | - expression
 *     | term
 *     ;
 */
int parse_expression(){
    debugLog("Parser: evaluating expression.");

    if(accept(Tok_Plus) || accept(Tok_Minus) || accept(Tok_Greater) || accept(Tok_Lesser) || accept(Tok_StrConcat)){
        if(!value()) return 0;
        if(!parse_expression()) return 0;
        return 1;
    }else{
        return term();
    }
}

/* term
 *     : value * expression
 *     | value / expression
 *     | paren_expression
 *     ;
 */
int term(){
    debugLog("Parser: evaluating term...");

    if(accept(Tok_Multiply) || accept(Tok_Divide)){
        if(!value()) return 0;
        if(!parse_expression()) return 0;
        return 1;
    }else{
        return paren_expression();
    }
}

/*  paren_expression
 *      : ( expression )
 *      | value
 *      ;
 */
int paren_expression(){
    debugLog("Parser: evaluating paren_expression...");
    if(accept(Tok_ParenOpen)){
        debugLog("Parser: paren_expression: found open parenthesi, finding expression...");
        if(!math_expression()) return 0;
        expect(Tok_ParenClose);
    }
    return 1;
}

/*
void drawParseTree(){
    currentNode = root;
    currentNode->index = 0;

    int level = 0, count = 0, i;
    while(currentNode->token != NULL){
        for(i=0;i<level;i++){
            printf("   ");
        }

        printf("(%s:%s)\n", tokenDictionary[currentNode->token->type], currentNode->token->lexeme);

        if(currentNode->children && currentNode->children[currentNode->index]){
            level++;
            currentNode = currentNode->children[currentNode->index++];
            currentNode->index = 0;
        }else if(currentNode->parent){
            while(currentNode->parent && currentNode->token->type != Tok_Begin){ //loop towards root of parse tree
                node_climb(); //Climb one step

                if(currentNode->children[currentNode->index]){ //Found parent with next child
                    currentNode = currentNode->children[currentNode->index++];
                    currentNode->index = 0;
                    break;
                }else{
                    free(currentNode->children);
                }
                level--;
            }

            if(currentNode->token->type == Tok_Begin) break;
        }else break;
    }
}*/

int parse(Token *t){ //Parses file, returns error if one occured
    exitFlag = 0;

    tokenizedInput = t;
    tokenIndex = 0;

    namespace(); //build parse tree

    return exitFlag;
}
