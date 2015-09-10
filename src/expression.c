#include "expression.h"

Operator operators[] = {
    {Tok_Comma,     0, 0, NULL},
    {Tok_StrConcat, 1, 0, NULL},
    {Tok_Plus,      2, 0, op_add},
    {Tok_Minus,     2, 0, NULL},
    {Tok_Multiply,  3, 0, op_mul},
    {Tok_Divide,    3, 0, NULL},
    {Tok_Modulus,   3, 0, NULL},
    {Tok_Exponent,  4, 1, NULL}
};

Variable addNum(Variable n1, Variable n2)
{
    return VAR(bignum_add(n1.value, n2.value), Num);
}

Variable addInt(Variable n1, Variable n2)
{
    return VAR(bigint_add(n1.value, n2.value), Int);
}

opFunc addFuncTable[] = {
    NULL, addNum, addInt, NULL, NULL, NULL
};

Variable mulNum(Variable n1, Variable n2)
{
    return VAR(bignum_mul(n1.value, n2.value), Num);
}

Variable mulInt(Variable n1, Variable n2)
{
    return VAR(bigint_mul(n1.value, n2.value), Int);
}

opFunc mulFuncTable[] = {
    NULL, mulNum, mulInt, NULL, NULL, NULL
};

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
            break;
        case Int:
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

/*
 * Returns the required type
 * for a given operator
 */
inline Type getReqType(TokenType op)
{
    return op == Tok_StrConcat ? String : Num;
}

inline Variable operate(Variable v1, Operator op, Variable v2)
{
    //check for type mismatch
    Type req = getReqType(op.op);
    if(v1.type != req){
        v1 = convertType(v1, req);
    }else if(v2.type != req){
        v2 = convertType(v2, req);
    }
    return op.func(v1, v2);
}

inline Variable op_add(Variable augend, Variable addend)
{
    return addFuncTable[augend.type](augend, addend);
}

inline Variable op_mul(Variable m1, Variable m2)
{
    return mulFuncTable[m1.type](m1, m2);
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
        fprintf(stderr, "Invalid Type in expression");
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

