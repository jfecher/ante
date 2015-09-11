#include "expression.h"

Operator operators[] = {
    {Tok_Comma,     0, 0, NULL},
    {Tok_StrConcat, 1, 0, op_cnct},
    {Tok_Plus,      2, 0, op_add},
    {Tok_Minus,     2, 0, op_sub},
    {Tok_Multiply,  3, 0, op_mul},
    {Tok_Divide,    3, 0, op_div},
    {Tok_Modulus,   3, 0, op_mod},
    {Tok_Exponent,  4, 1, op_pow}
};

//TODO: move [op][type] style functions to new file along with their function tables

/*
 *  Returns true if the given type is able
 *  to be converted to the other and vice-versa
 */
inline bool typeCompatible(Type t1, Type t2)
{
    switch(t1){
        case Num:
        case Int:
            return t2 == Num || t2 == Int;
        case String:
            return t2 == Num || t2 == Int || t2 == String;
        default:
            return 0;
    }
}

/*
 *  Returns a new variable made from v with type t
 *  TODO: Not yet implemented
 */
Variable convertType(Variable v, Type t)
{
    Variable ret = {NULL, t, NULL, NULL};
    
    if(typeCompatible(v.type, t)){
        switch(t){
        case Num:
            //mpz_clear(*(BigInt)v.value);
            //v.value = bigint_new(v.value);
            break;
        case Int:
            //mpf_get_str(v.value, v.value, 10, 1000, *(BigNum)v.value); 
            //mpz_init_set(*(BigInt)v.value, v.value);
            //v.value = bignum_new(v.value);
            break;
        case String:
            break;
        default: break;
        }
        return ret;
    }
   
    ret.type = Invalid;
    return ret;
}

inline Variable operate(Variable v1, Operator op, Variable v2)
{
    //check for type mismatch
    if(v1.type != v2.type){
        if(v1.type > v2.type){
            //TODO: v1 = convertType(v1, v2.type);
        }else{
            //TODO: v2 = convertType(v2, v2.type);
        }
    }
    return op.func(v1, v2);
}

Operator getOperator(TokenType t)
{
    int i;
    for(i = 0; i < ARR_SIZE(operators); i++){
        if(operators[i].op == t){
            return operators[i];
        }
    }
    Operator invalid = {-1, 0, 0};
    return invalid;
}

Variable expression(void){
    Variable v = getValue(toks[tIndex]);
    if(v.type == Invalid){
        fprintf(stderr, "Invalid Type in expression\n");
        return v;
    }

    v = _expression(v, 0);
    INC_POS(1);
    return v;
}

Variable _expression(Variable l, uint8_t minPrecedence){
    Operator lookAhead = getOperator(toks[tIndex+1].type);
    while(lookAhead.op != -1 && lookAhead.prec >= minPrecedence){
        Operator op = lookAhead;
        INC_POS(2);
        Variable r = getValue(toks[tIndex]);
        lookAhead = getOperator(toks[tIndex + 1].type);

        while(lookAhead.op != -1 && (lookAhead.prec > op.prec || (lookAhead.rAsso && lookAhead.prec >= op.prec))){
            r = _expression(r, lookAhead.prec);
            lookAhead = getOperator(toks[tIndex + 1].type);
        }
        Value tmp = l.value;
        l = operate(l, op, r);
        free(tmp);
        free(r.value);
    }
    return l;
}

