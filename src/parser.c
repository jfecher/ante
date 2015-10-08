#include "parser.h"

//#define DEBUG
#define PARSER_CUR_TOK  (tokenizedInput[tokenIndex])
#define PARSER_NEXT_TOK (tokenizedInput[tokenIndex+1])

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
int foreach_loop();
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
int function_call();
int paren_expression();
int math_expression();

void syntaxError(const char* msg, int showErrTok){
    fprintf(stderr, "Syntax Error: %s", msg);
    
    if(showErrTok) 
        fprintf(stderr, "%s (of type %s)", (PARSER_CUR_TOK).lexeme, tokenDictionary[(PARSER_CUR_TOK).type]);
    
    fprintf(stderr, " at row %d, col %d.\n", (PARSER_CUR_TOK).row, (PARSER_CUR_TOK).col);
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
    }else{
        if(isTty && type == Tok_Indent){
            exitFlag = NEW_BLOCK;
        }else{
            fprintf(stderr, "Syntax Error: Expected %s, but found %s at row %d, col %d.\n", tokenDictionary[type], tokenDictionary[(PARSER_CUR_TOK).type], (PARSER_CUR_TOK).row, (PARSER_CUR_TOK).col);
            exitFlag = 2;
        }
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
    return 1;
}

/* class_statement
 *     : class_def
 *     | function_def
 *     | statement
 *     ;
 */
int class_statement(){
    switch((PARSER_CUR_TOK).type){
        case Tok_TypeDef:  return class_def();
        case Tok_FuncDef:  return function_def();
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
    if(check(Tok_Num) || check(Tok_Int) || check(Tok_String) || check(Tok_Boolean) || check(Tok_Identifier)){
        return (PARSER_CUR_TOK).type;
    }else{
        return Tok_Invalid;
    }
}


/* function_def
 *     : FUNCDEF : function_args INDENT function_body UNINDENT
 *     ;
 */
int function_def(){
    //Assume the return value has already been parsed
    accept(Tok_FuncDef);
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
    if(!check(Tok_Indent) && !check(Tok_EndOfInput)){
        while(!check(Tok_EndOfInput) && !check(Tok_Indent) && !check(Tok_Unindent) && !check(Tok_Newline)){
            if(type() == Tok_Invalid){
                syntaxError("Expected Type in function arguments.  Got ", 1);
                return 0;
            }
            
            tokenIndex++;
            accept(Tok_Identifier);
            
            if(!check(Tok_EndOfInput) && !check(Tok_Indent) && !check(Tok_Unindent) && !check(Tok_Newline))
                expect(Tok_Comma);
        }    
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
    expect(Tok_Indent);
    while(!(accept(Tok_Unindent) || check(Tok_EndOfInput))){
        if(!statement()) return 0;
        accept(Tok_Newline);
    }
    return 1;
}

/* function_call
 *     : FUNCTION IDENTIFIER ( function_args )
 *     | FUNCTION IDENTIFIER ( )
 *     ;
 */
int function_call(){
    expect(Tok_FuncCall);
    if(!paren_expression()) return 0;
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
    switch(tokenizedInput[tokenIndex].type){
    case Tok_For:         return for_loop();
    case Tok_ForEach:     return foreach_loop();
    case Tok_While:       return while_loop();
    case Tok_Print:       return print_statement();
    case Tok_Greater:     return initialize_value();
    case Tok_Identifier:  return assign_value();
    case Tok_Return:      return return_statement();
    case Tok_If:          return if_statement();
    case Tok_FuncCall:    return function_call();
    case Tok_Divide:
    case Tok_Newline:
        tokenIndex++;
        return 1;
    default:
        if(type() != Tok_Invalid) return initialize_value();
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
    expect(Tok_For);
    if(check(Tok_Identifier) && (PARSER_NEXT_TOK).type == Tok_Assign){
        tokenIndex++;
        if(!assign_value()) return 0;
    }else if(type() && (PARSER_NEXT_TOK).type == Tok_Greater){
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

/* foreach_loop
 *     : FOREACH initialize_value IN variable function_body
 *     ;
 */
int foreach_loop(){
    expect(Tok_ForEach);
    if(!initialize_value()) return 0;
    expect(Tok_In);
    if(!variable()) return 0;
    if(!function_body()) return 0;
    return 1;
}

/* while_loop
 *     : WHILE expression function_body
 *     ;
 */
int while_loop(){
    expect(Tok_While);
    if(!math_expression()) return 0;
    if(!function_body()) return 0;
    return 1;
}

/* print_statement
 *     : PRINT expression , expression...
 *     ;
 */
int print_statement(){
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
    if(type() != Tok_Invalid) tokenIndex++;

    //array declaration
    if(accept(Tok_BracketOpen)){
        math_expression();
        expect(Tok_BracketClose);
    }

    expect(Tok_Greater);
    if((PARSER_NEXT_TOK).type == Tok_Assign){
        if(!assign_value()) return 0;
        return 1;
    }else{
        if(check(Tok_FuncDef))
            return function_def();

        if(!variable()){ 
            syntaxError("No variable found to initialize.", 0); 
            return 0;
        }
    }
    return 1; // >var   or   int>var    (No initialization of value)
}

/* assign_value
 *     : IDENTIFIER = expression
 *     ;
 */
int assign_value(){
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
    expect(Tok_If);
    if(!math_expression()) return 0;
    if(!function_body()) return 0;

    if(accept(Tok_Else)){
        if(!function_body()) return 0;
    }
    return 1;
}

/* variable
 *     : array_value
 *     | IDENTIFIER
 *     ;
 */
inline int variable(){
    if((PARSER_NEXT_TOK).type == Tok_BracketOpen)
        return array_value();
    else
        return accept(Tok_Identifier);
}

/* array_value
 *     : IDENTIFIER [ expression ]
 *     ;
 */
int array_value(){
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
    if(check(Tok_EndOfInput)){
        syntaxError("Expected value in expression, got ", 1);
        return 0;
    }else if(check(Tok_FuncCall))
        return function_call();
    else if(check(Tok_ParenOpen))
        return paren_expression();
    else
        return literal_value() || variable();
}

int math_expression(){
    if(check(Tok_EndOfInput))
        syntaxError("Invalid empty expression", 0);
    
    if(!value()) return 0;
    if(!parse_expression()) return 0;
    return 1;
}

/* expression
 *     : + expression
 *     | - expression
 *     | > expression
 *     | < expression
 *     | .. expression
 *     | == expression
 *     | , expression
 *     | * expression
 *     | / expression
 *     | ^ expression
 *     | % expression
 *     | paren_expression
 *     ;
 */
int parse_expression(){
    if(accept(Tok_Plus) || accept(Tok_Minus) || accept(Tok_Greater) || 
            accept(Tok_Lesser)   || accept(Tok_StrConcat) || accept(Tok_EqualsEquals) ||
            accept(Tok_Comma)    || accept(Tok_Multiply)  || accept(Tok_Divide) ||
            accept(Tok_Exponent) || accept(Tok_Modulus)){
     
        if(!value()) return 0;
        if(!parse_expression()) return 0;
        return 1;
    }else{
        return paren_expression();
    }
}

/*  paren_expression
 *      : ( expression )
 *      ;
 */
int paren_expression(){
    if(accept(Tok_ParenOpen)){
        if(!math_expression()) return 0;
        expect(Tok_ParenClose);
    }
    return 1;
}

int parse(Token *t){ //Parses tokenstream, returns an error if one occured
    exitFlag = 0;

    tokenizedInput = t;
    tokenIndex = 0;

    namespace(); //parse tree

    return exitFlag;
}
