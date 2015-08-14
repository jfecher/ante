#include "expression.h"

Operator operators[] = {
    {Tok_Comma,     0, 0, NULL},
    {Tok_StrConcat, 1, 0, NULL},
    {Tok_Plus,      2, 0, NULL},
    {Tok_Minus,     2, 0, NULL},
    {Tok_Multiply,  3, 0, NULL},
    {Tok_Divide,    3, 0, NULL},
    {Tok_Modulus,   3, 0, NULL},
    {Tok_Exponent,  4, 1, NULL}
};

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

inline Variable expression(void){
    return _expression(getValue(toks[tIndex]), 0);
}

Variable _expression(Variable l, uint8_t minPrecedence){
    Operator lookAhead = getOperator(toks[tIndex+1].type);
    while(lookAhead.op != -1 && lookAhead.prec >= minPrecedence){
        //Operator op = lookAhead;
        INC_POS(2);
        Variable r = getValue(toks[tIndex]);
        lookAhead = getOperator(toks[tIndex + 1].type);

        while(lookAhead.op != -1 && (lookAhead.prec > minPrecedence || (lookAhead.rAsso && lookAhead.prec >= minPrecedence))){
            r = _expression(r, lookAhead.prec);
            lookAhead = getOperator(toks[tIndex + 1].type);
        }
        l.value = add(l.value, r.value);
        free(r.value);
    }
    INC_POS(1);
    return l;
}

