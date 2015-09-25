#include "interpreter.h"

/*
 *  TODO:
 *    -Implement declaration of functions
 *    -Stop interpretation on runtime error in files
 */

char *typeDictionary[] = {
    "Object",
    "Num",
    "Int",
    "String",
    "Function",
    "Invalid"
};

funcPtr ops[] = {
    &op_initObject,
    &op_assign,
    &op_print,
    &op_initNum,
    &op_initStr,
    &op_initInt,
};

/*
 *  Returns the index of an identifier in the variable table or -1 if no
 *  matches are found
 */
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
    if(c.x != -1){ //lookupVar returns {-1, -1} if the var was not found
        runtimeError(ERR_ALREADY_INITIALIZED, identifier);
    }

    //           value, type, dynamic,     name
    Variable v = {NULL, t,    t == Object, malloc(strlen(identifier)+1)};
    strcpy(v.name, identifier);
    v.name[strlen(identifier)] = '\0';

    switch(t){
    case Num:
        v.value = bignum_new("0");
        break;
    case Int:
        v.value = bigint_new("0");
        break;
    default:
        CPY_TO_STR(v.value, "");
    }
    varTable_add(&stack_top(stack), v);
}

void op_initObject(void){
    initVar(toks[tIndex+1].lexeme, Object);
    INC_POS(1);
}

void setVar(Variable *v, Variable val){
    if(v->type == val.type || v->dynamic){
        free_value(*v);
        v->value = val.value;
        v->type = val.type;
    }else{
        free_var(val);
        runtimeError(ERR_TYPE_MISMATCH, v->name);
    }
}

Variable copyVar(Variable v){
    Variable cpy = {NULL, v.type, v.dynamic, NULL};
    switch(v.type){
    case Int:
        cpy.value = bigint_copy(v.value);
        break;
    case Num:
        cpy.value = bignum_copy(v.value);
        break;
    default: 
        CPY_TO_STR(cpy.value, v.value);        
    }
    return cpy;
}

//TODO: implement
Variable exec_function(char *funcName){
    Coords c = lookupFunc(funcName);
    if(c.x == -1){
        printf("Function '%s' not found\n", funcName);
        return VAR(NULL, Invalid);
    }

    return VAR(NULL, Invalid);
}

void op_function(){
    char *funcName = toks[tIndex+1].lexeme;
    INC_POS(2);

    if(strcmp(funcName, "typeof") == 0){
        op_typeOf();
    }else if(strcmp(funcName, "system") == 0){
        Variable v = expression();
        if(v.type == String){
            system((char*)v.value);
        }else{
            fprintf(stderr, "Function parameter type mismatch.  Expected String but got %s.\n", typeDictionary[v.type]);
        }
        free_value(v);
    }
}

void op_print(){
    INC_POS(1);
    Variable v = expression();

    switch(v.type){
        case Int:
            gmp_printf("%Zd\n", *(BigInt)v.value);
            break;
        case Num:;
            Coords c = lookupVar("_precision");
            gmp_printf("%.*Ff\n", mpz_get_ui(*(BigInt)stack.items[c.x].table[c.y].value), *(BigNum)v.value);
            break;
        case String:
            printf("%s\n", (char*)v.value);
            break;
        default:
            break;
    }
    free_var(v);
}

void op_initNum(void){
    initVar(toks[tIndex+2].lexeme, Num);
    INC_POS(2);
}

void op_initInt(void){
    initVar(toks[tIndex+2].lexeme, Int);
    INC_POS(2);
}

void op_initStr(void){
    initVar(toks[tIndex+2].lexeme, String);
    INC_POS(2);
}

Type tokTypeToVarType(TokenType t){
    if(t == Tok_DoubleLiteral){
        return Num;
    }else if(t == Tok_IntegerLiteral){
        return Int;
    }else if(t == Tok_StringLiteral){
        return String;
    }
    return Invalid;
}

Variable makeVarFromTok(Token t){
    Variable ret = {NULL, tokTypeToVarType(t.type), 0, NULL};
    if(ret.type == Int){
        ret.value = bigint_new(t.lexeme);
    }else if(ret.type == Num){
        ret.value = bignum_new(t.lexeme);
    }else{
        CPY_TO_STR(ret.value, t.lexeme);
    }
    return ret;
}

Variable getValue(Token t){
    if(t.type == Tok_Identifier){
        Coords c = lookupVar(t.lexeme);
        if(c.x == -1){
            fprintf(stderr, ERR_UNINITIALIZED_VALUE_IN_EXPRESSION, t.lexeme);
            return VAR(NULL, Invalid);
        }
        return copyVar(stack.items[c.x].table[c.y]);
    }else if(t.type == Tok_FuncCall){
        return exec_function(toks[tIndex].lexeme);
    }else if(t.type == Tok_ParenOpen){
        INC_POS(1);
        return expression();
    }else{
        return makeVarFromTok(t);
    }
}

/* Sets a variable to value v of type t */
void op_assign(void){
    if(toks[tIndex+1].type != Tok_Assign){
        INC_POS(1);
        return;
    }

    Coords c = lookupVar(toks[tIndex].lexeme);
    if(c.x == -1){
        INC_POS(1);
        runtimeError(ERR_NOT_INITIALIZED, toks[tIndex-1].lexeme);
    }
    INC_POS(2);
    setVar(&stack.items[c.x].table[c.y], expression());
}

void op_typeOf(){
    Variable v = expression();
    if(v.dynamic)
        printf("dynamic ");
    printf("%s\n", typeDictionary[v.type]);
    INC_POS(4);
    free_var(v); //TODO: function to automatically clear values if var is a num or int
}

void add_global_var(Variable v){
    initVar(v.name, v.type);
    setVar(&stack.items[0].table[stack.items[0].size-1], v);
}

//char **history;
void init_interpreter(void){
    VarTable global = {NULL, 0};
    stack.size = 0;
    stack_push(&stack, global);
    
    //builtin variables
    add_global_var(VARIABLE(bigint_new("10"), Int, 0, "_precision"));
}

void setupTerm(){
    struct termios oldt, newt;
    tcgetattr(STDIN_FILENO, &oldt);
    newt = oldt;
    newt.c_lflag &= ~(ICANON | ECHO);
    tcsetattr(STDIN_FILENO, TCSANOW, &newt);
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
        }else if(c == 27){ //up: (91, 65), down, right, left
            getchar(); //discard escape sequence
            getchar();
            continue;
        }

        //seperate input by tokens for syntax highlighting
        init_lexer(1);
        toks = lexer_next(1);
        freeToks(&toks);
    }while(c != '\n');
    puts("");
}

char exec(){
    toks = lexer_next(0);
    tIndex = 0;

    if(parse(toks)){
        NFREE(srcLine);
        freeToks(&toks);
        return 0;
    }

    uint8_t opcode = toks[tIndex].type;
    if(opcode == Tok_Newline)
        opcode = toks[++tIndex].type;

    while(opcode < ARR_SIZE(ops)){
        ops[opcode]();
        
        if(toks[tIndex].type == Tok_Newline)
            tIndex++;

        opcode = toks[tIndex].type;
    }

    NFREE(srcLine);
    freeToks(&toks);    
    return 1;
}

void interpret(FILE *src, char isTty){
    init_interpreter();

    if(!isTty){
        init_lexer(0);
        exec();
    }else{
        setupTerm();

        puts(KEYWORD_COLOR "Zy " INTEGERL_COLOR  VERSION  RESET_COLOR " - " VERDATE "\nType 'exit' to exit the interpreter.");
        int i;
        fflush(stdout);
        for(;;){
            for(i=1; i < stack.size; i++){ printf(":"); }

            getln();//Tokenizes the entire line, and print it out on screen
            if(strcmp(srcLine, "exit") == 0)
                break;
            init_lexer(1);
            exec();

        }
    }

    stack_free(stack);
}
