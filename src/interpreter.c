#include "interpreter.h"
#include <stdio.h>
#include <termios.h>
#include <unistd.h>

char *typeDictionary[] = {
    "Object",
    "Num",
    "String",
    "Function",
    "Invalid"
};

funcPtr ops[] = {
    &op_initObject,
    &op_assign,
    &op_print,
    &op_function,
    &op_initNum,
    &op_initStr
};

Operator operators[] = {
    {Tok_Comma,    0, 0},
    {Tok_Plus,     1, 0},
    {Tok_Minus,    1, 0},
    {Tok_Multiply, 2, 0},
    {Tok_Divide,   2, 0},
    {Tok_Modulus,  2, 0},
    {Tok_Exponent, 3, 1}
};

/* returns the index of an identifier in the variable table.  Returns -1 if no matches are found */
Coords lookupVar(char* identifier){
    int i, j;

    for(j=0; j < stack.size; j++){
        for(i=0; i < stack.items[j].size; i++){
            if(strcmp(identifier, stack.items[j].table[i].name) == 0){
                Coords c = {j, i};
                return c;
            }
        }
    }
    Coords c = {-1, -1};
    return c;
}

Coords lookupFunc(char* identifier){
    int i, j;
    for(i = 0; i < stack.size; i++){
        for(j = 0; j < stack.items[i].size; j++){
            Variable v = stack.items[j].table[i];

            if(v.type == Function && strcmp(identifier, v.name) == 0){
                Coords c = {i, j};
                return c;
            }
        }
    }
    Coords c = {-1, -1};
    return c;
}

/*
 * Initializes a variable with name identifier, and Type t.
 * Initializes the variable to 0 if it is numerical, or ""
 * if it is a string.
 */
void initVar(char *identifier, Type t){
    Coords c = lookupVar(identifier);
    if(c.x != -1){
        runtimeError(ERR_ALREADY_INITIALIZED, identifier);
        free(identifier);
    }

    Variable v = {NULL, t, t == Object, identifier};
    if(t == Num){
        CPY_TO_STR(v.value, "0");
    }else{
        CPY_TO_STR(v.value, "");
    }
    varTable_add(&stack_top(stack), v);
}

void op_initObject(){
    CPY_TO_NEW_STR(id, toks[tIndex+1].lexeme);
    initVar(id, Object);
    INC_POS(1);
}

void setVar(Variable *v, Variable val){
    if(v->type == val.type || v->dynamic){
        NFREE(v->value);
        v->value = val.value;
        v->type = val.type;
    }else{
        runtimeError(ERR_TYPE_MISMATCH, v->name);
    }
}

Variable copyVar(Variable v){
    Variable cpy = {NULL, v.type, v.dynamic, NULL};
    CPY_TO_STR(cpy.value, v.value);
    return cpy;
}

void op_function(){
    puts("function");
}

void op_print(){
    INC_POS(1);
    printf("%s\n", (char*)initExpr().value);
}

void op_initNum(){
    CPY_TO_NEW_STR(id, toks[tIndex+2].lexeme);
    initVar(id, Num);
    INC_POS(2);
}

void op_initStr(){
    CPY_TO_NEW_STR(id, toks[tIndex+2].lexeme);
    initVar(id, String);
    INC_POS(2);
}

Type tokTypeToVarType(TokenType t){
    if(t == Tok_IntegerLiteral || t == Tok_DoubleLiteral){
        return Num;
    }else if(t == Tok_StringLiteral || t == Tok_CharLiteral){
        return String;
    }
    return Invalid;
}

Variable makeVarFromTok(Token t){
    Variable ret = {NULL, tokTypeToVarType(t.type), 0, NULL};
    CPY_TO_STR(ret.value, t.lexeme);
    return ret;
}

Variable getValue(Token t){
    if(t.type == Tok_Identifier){
        Coords c = lookupVar(t.lexeme);
        return copyVar(stack.items[c.x].table[c.y]);
    }else if(t.type == Tok_ParenOpen){
        INC_POS(1);
        Variable v = initExpr();
        return v;
    }else{
        return makeVarFromTok(t);
    }
}

/* Sets a variable to value v of type t */
void op_assign(){
    CPY_TO_NEW_STR(name, toks[tIndex].lexeme);
    getCoords(c, name);
    INC_POS(2);
    setVar(&stack.items[c.x].table[c.y], initExpr());
}

void lib_system(){

}

void lib_typeof(){
    if(toks[3].type == Tok_Identifier){
        getCoords(c, toks[3].lexeme);
        if(stack.items[c.x].table[c.y].dynamic)
            printf("dynamic ");
        puts(typeDictionary[stack.items[c.x].table[c.y].type]);
    }else{
        puts(tokenDictionary[toks[3].type]);
    }
}

Operator getOperator(TokenType t){
    int i;
    for(i = 0; i < ARR_SIZE(operators); i++){
        if(operators[i].op == t){
            return operators[i];
        }
    }
    Operator invalid = {-1, 0, 0};
    return invalid;
}

inline Variable initExpr(){
    return expression(getValue(toks[tIndex]), 0);
}

Variable expression(Variable l, uint8_t minPrecedence){
    Operator lookAhead = getOperator(toks[tIndex+1].type);
    while(lookAhead.op != -1 && lookAhead.precedence >= minPrecedence){
        //Operator op = lookAhead;
        INC_POS(2);
        Variable r = getValue(toks[tIndex]);
        lookAhead = getOperator(toks[tIndex + 1].type);

        while(lookAhead.op != -1 && (lookAhead.precedence > minPrecedence || (lookAhead.associativity && lookAhead.precedence >= minPrecedence))){
            r = expression(r, lookAhead.precedence);
            lookAhead = getOperator(toks[tIndex + 1].type);
        }
        l.value = add(l.value, r.value);
    }
    INC_POS(1);
    return l;
}
/*
 *      expression()
 * Returns a variable containing the result of
 * any expression.  Utilizes stacks to maintain
 * order of operations.
 *
 *      9 * 6 + 2 * (3 + 7)^2 + 3
 *      9 * 6 + 2 * 10^2 + 3
 *      9 * 6 + 2 * 100 + 3
 *      54 + 2 * 100 + 3
 *      54 + 200 + 3
 *      254 + 3
 *      257
 *
 */
/*Variable expression(Variable v, uint8_t minP){
    ExprValue *t = NULL;
    uint8_t scope = 0;
    int start = tIndex;

    for(; !IS_ENDING_TOKEN(toks[tIndex].type); tIndex++){
        t = realloc(t, sizeof(ExprValue) * (tIndex-start+1));
        t[tIndex-start].isOp = 0;

        if(toks[tIndex].type == Tok_ParenOpen){
            scope++;
            tIndex++;
            t[tIndex-start].v = expression();
        }else if(toks[tIndex].type == Tok_ParenClose){
            scope--;
            if(scope < 0)
                break;
        }else if(IS_OPERATOR(toks[tIndex].type)){
            t[tIndex-start].isOp = 1;
            t[tIndex-start].t = toks[tIndex].type;
        }else{
            t[tIndex-start].v = getValue(toks[tIndex]);
        }
    }

    int i;
    int len = tIndex-start;

    for(i=0; i < len; i++){
        if(t[i].isOp && t[i].t == Tok_Multiply){
            t[i-1].v.value = multiply(t[i-1].v.value, t[i+1].v.value);
            removeEVPair(&t, i);
            i--;
            len -= 2;
        }
    }
    for(i=0; i < len; i++){
        if(t[i].isOp && t[i].t == Tok_Plus){
            printf("i = %d, len = %d, t[%d].v->value = %s, t[%d].v->value = %s\n", i, ARR_SIZE(t), i-1, t[i-1].v.value, i+1, t[i+1].v.value);
            t[i-1].v.value = add(t[i-1].v.value, t[i+1].v.value);
            removeEVPair(&t, i);
            printf("Size: %lu\n", ARR_SIZE(t));
            len -= 2;
            i--;
        }
    }

    return t[0].v;
}*/

//char **history;
void initializeInterpreter(){
    VarTable global = {NULL, 0};
    stack.size = 0;
    stack_push(&stack, global);
}

void setupTerm(){
    struct termios oldt, newt;
    tcgetattr(STDIN_FILENO, &oldt);
    newt = oldt;
    newt.c_lflag &= ~( ICANON | ECHO );
    tcsetattr( STDIN_FILENO, TCSANOW, &newt);
}

void getln(){
    char c = 0;
    int len = 0;
    srcLine = calloc(sizeof(char), 2);
    printf(": ");

    do{
        c = getchar();
        len = strlen(srcLine);

        if(c == 9 || (c >= 32 && c <= 126)){
            ralloc(&srcLine, sizeof(char)* (len+3));
            srcLine[len] = c;
            srcLine[len+1] = '\0';
            srcLine[len+2] = '\0';
        }else if(c == 8 || c == 127){ //backspace
            if(len > 0){
                srcLine[len-1] = '\0';
                printf("\r: %s  ", srcLine); //screen must be manually cleared of deleted character
            }
        }else if(c != '\n'){
            printf("\rUnknown char %c of value %d\n", c, c);
        }

        //seperate input by tokens for syntax highlighting
        initialize_lexer(1);
        toks = lexer_next(1);
        freeToks(&toks);
    }while(c != '\n');
    puts("");
}

void interpret(FILE *src, int tty){
    initializeInterpreter();

    if(!tty){
        parse(src);
        puts("Running file...");
    }else{
        setupTerm();

        puts(KEYWORD_COLOR "Zy " INTEGERL_COLOR  VERSION  RESET_COLOR " - " VERDATE "\nType 'exit' to exit the interpreter.");
        int i;
        fflush(stdout);
        for(;;){
            for(i=1; i < stack.size; i++){ printf(":"); }

            getln();//Tokenizes the entire line, and prints it out on screen

            initialize_lexer(1);
            toks = lexer_next(0);
            tIndex = 0;

            if(strcmp(srcLine, "exit") == 0)
                break;

            if(parse(toks)){
                printf("Parser returned %d.\n", exitFlag);
                continue;
            }

            uint8_t opcode = toks[tIndex].type;
            while(opcode < ARR_SIZE(ops) && opcode >= 0){
                ops[opcode]();
                opcode = toks[tIndex].type;
            }

            free(srcLine);
            freeToks(&toks);
        }
    }

    stack_free(stack);
}
