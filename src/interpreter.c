#include "interpreter.h"

/*
 *  TODO:
 *    -Implement declaration of functions
 *    -Stop interpretation on runtime error in files
 *    -Change _precision to a function
 */

char *typeDictionary[] = {
    "Object",
    "Num",
    "Int",
    "String",
    "Function",
    "Tuple",
    "Invalid"
};

funcPtr ops[] = {
    &op_initObject,
    &op_assign,
    &op_print,
    &op_initNum,
    &op_initStr,
    &op_initInt,
    &op_callFunc,
    &op_initFunc,
};

Coords shallowLookupVar(char *identifier){
    for(int i = 0; i < stack_top(stack).size; i++)
        if(strcmp(identifier, stack_top(stack).table[i].name) == 0)
            return (Coords){stack.size-1, i};
    return (Coords){-1, -1};
}

/*
 *  Returns the index of an identifier in the variable table or -1 if no
 *  matches are found
 */
Coords lookupVar(char* identifier){
    int i, j;
    for(j=stack.size-1; j >= 0; j--){
        for(i=0; i < stack.items[j].size; i++){
            if(strcmp(identifier, stack.items[j].table[i].name) == 0)
                return (Coords){j, i};
        }
    }
    return (Coords){-1, -1};
}

Coords lookupFunc(char* identifier){
    int i, j;
    for(i = 0; i < stack.size; i++){
        for(j = 0; j < stack.items[i].size; j++){
            Variable v = stack.items[i].table[j];

            if(v.type == Function && strcmp(identifier, v.name) == 0)
                return (Coords){i, j};
        }
    }
    return (Coords){-1, -1};
}

/*
 * Initializes a variable with name identifier, and Type t.
 * Initializes the variable to 0 if it is numerical, or ""
 * if it is a string.
 */
void initVar(char *identifier, Type t, char isDynamic){
    Coords c = shallowLookupVar(identifier);
    if(c.x != -1){ //lookupVar returns {-1, -1} if the var was not found
        runtimeError(ERR_ALREADY_INITIALIZED, identifier);
    }

    //           value, type, dynamic,   name
    Variable v = {NULL, t,    isDynamic, malloc(strlen(identifier)+1), 0};
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

//TODO: put functions in a seperate table
void initFunc(char *identifier, Type retType, Token *start){
    Coords c = lookupFunc(identifier);
    if(c.x != -1){ //lookupVar returns {-1, -1} if the var was not found
        runtimeError(ERR_ALREADY_INITIALIZED, identifier);
    }
  
    size_t iLen = strlen(identifier);

    //           value, type, dynamic,   name
    Variable v = {start, Function, 0, malloc(iLen+1), 1};
    strcpy(v.name, identifier);
    v.name[iLen] = '\0';

    varTable_add(&stack_top(stack), v); 
}

void op_initObject(void){
    initVar(toks[tIndex+1].lexeme, Object, 1);
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
    Variable cpy = {NULL, v.type, v.dynamic, NULL, 0};
    switch(v.type){
    case Int:
        cpy.value = bigint_copy(v.value);
        break;
    case Num:
        cpy.value = bignum_copy(v.value);
        break;
    case Tuple:
        cpy.value = malloc(sizeof(struct Tuple));
        memcpy(cpy.value, v.value, sizeof(struct Tuple));
        struct Tuple *tupC = (struct Tuple*)cpy.value;
        struct Tuple *tupV = (struct Tuple*)v.value;
        tupC->tup = malloc(sizeof(Variable) * tupV->size);
        for(int i = 0; i < tupV->size; i++) //make a deep copy
            tupC->tup[i] = copyVar(tupV->tup[i]);
        tupC->size = tupV->size;
        break;
    default: 
        CPY_TO_STR(cpy.value, v.value);        
    }
    return cpy;
}

Variable exec_function(char *funcName){
    Coords c = lookupFunc(funcName);
    if(c.x == -1){
        printf("Function '%s' has not been declared.\n", funcName);
        return VAR(NULL, Invalid);
    }
    
    //TODO: parameters
    unsigned int pos = tIndex;
    Token *tmp = toks;
    tIndex = 0;
    toks = stack.items[c.x].table[c.y].value;


    VarTable local = {NULL, 0};
    stack_push(&stack, local);
    exec();
    varTable_free(stack_top(stack));
    stack_pop(&stack);

    tIndex = pos;
    toks = tmp;
    return VAR(NULL, Invalid);
}

void printValue(Value v, Type t){
    switch(t){
        case Int:
            mpz_out_str(stdout, 10, *(BigInt)v);
            break;
        case Num:;
            gmp_printf("%.Ff", *(BigNum)v);
            break;
        case String:
            fputs((char*)v, stdout);
            break;
        case Tuple:;
            struct Tuple tup = *(struct Tuple*)v;
            printf("Tuple of size %d: ", tup.size);
            for(int i = 0; i < tup.size; i++){
                printValue(tup.tup[i].value, tup.tup[i].type);
                if(i + 1 < tup.size) 
                    fputs(", ", stdout);
            }
            break;
        default:
            break;
    } 
}

void op_print(){
    INC_POS(1);
    Variable v = expression();
    printValue(v.value, v.type);
    putchar('\n');
    free_var(v);
}

void op_callFunc(void){
    char *funcName = toks[tIndex].lexeme;
    INC_POS(1);
    free_var(exec_function(funcName));
    
    //skip parameters for now
    while(toks[tIndex].type != Tok_Newline && toks[tIndex].type != Tok_EndOfInput) 
        INC_POS(1);
}

void op_initFunc(void){
    char *identifier = toks[tIndex].lexeme;
    unsigned char pos = 0;

    while(toks[tIndex].type != Tok_Indent && toks[tIndex].type != Tok_EndOfInput){
        INC_POS(1);
        pos++;
    }
    
    initFunc(identifier, Function, toks + tIndex);
    
    while(toks[tIndex].type != Tok_Unindent && toks[tIndex].type != Tok_EndOfInput){
        INC_POS(1);
    }
    if(toks[tIndex].type == Tok_Unindent) INC_POS(1);
}

void op_initNum(void){
    initVar(toks[tIndex+2].lexeme, Num, 0);
    INC_POS(2);
}

void op_initInt(void){
    initVar(toks[tIndex+2].lexeme, Int, 0);
    INC_POS(2);
}

void op_initStr(void){
    initVar(toks[tIndex+2].lexeme, String, 0);
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

inline void addGlobalVar(Variable v){
    initVar(v.name, v.type, 0);
    setVar(&stack.items[0].table[stack.items[0].size-1], v);
}

void init_interpreter(void){
    VarTable global = {NULL, 0};
    stack.size = 0;
    stack_push(&stack, global);
    
    //builtin variables
    //addGlobalVar(VARIABLE(bigint_new("10"), Int, 0, "_precision"));
}

#define isWhitespaceTok(t) (t==Tok_Newline||t==Tok_Indent)
void exec(void){
    uint8_t opcode = toks[tIndex].type;
    if(isWhitespaceTok(opcode))
        opcode = toks[++tIndex].type;

    while(opcode < ARR_SIZE(ops)){
        ops[opcode]();
        
        if(isWhitespaceTok(toks[tIndex].type))
            tIndex++;

        opcode = toks[tIndex].type;
    }

}

void interpret(FILE *src, char isTty){
    init_interpreter();

    if(!isTty){
        init_lexer(NULL);
        toks = lexer_next(0);
        tIndex = 0;

        if(parse(toks)){
            freeSrcLine();
            freeToks(&toks);
            return;
        }

        exec();
        freeToks(&toks);
    }else{
        init_sl();

        puts(KEYWORD_COLOR "Zy " INTEGERL_COLOR  VERSION  RESET_COLOR " - " VERDATE "\nType 'exit' to exit the interpreter.");
        fflush(stdout);
        for(;;){
            char *srcLine = NULL;
            scanLine(&srcLine);//Tokenizes the entire line, stores it in srcLine, and print it out on screen
            if(strcmp(srcLine, "exit") == 0){
                NFREE(srcLine);
                break;
            }
            
            init_lexer(srcLine);
            toks = lexer_next(0);
            tIndex = 0;

            int flag = parse(toks);
            if(flag == NEW_BLOCK){
                freeToks(&toks);
                scanBlock(&srcLine);
                init_lexer(srcLine);
                toks = lexer_next(0);
                
                if(parse(*toks)){
                    NFREE(srcLine);
                    freeToks(&toks);
                    continue;     
                }
            }
            
            exec();
            if(flag != NEW_BLOCK) 
                freeToks(&toks);
            NFREE(srcLine);
        }
        freeHistory();
    }
    stack_free(stack);
}
