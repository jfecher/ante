#include "expression.h"

Operator operators[] = {
    {Tok_Comma,     0, 0, NULL}, // op_tup
    {Tok_StrConcat, 1, 0, op_cnct},
    {Tok_Plus,      2, 0, op_add},
    {Tok_Minus,     2, 0, op_sub},
    {Tok_Multiply,  3, 0, op_mul},
    {Tok_Divide,    3, 0, op_div},
    {Tok_Modulus,   3, 0, op_mod},
    {Tok_Exponent,  4, 1, op_pow}
};


Type conversion[4][4] = {
//            Object,  Num,    Int,    String
/* Object */  {Object, Object, Object, Object},
/*  Num   */  {Object, Num,    Num,    String},
/*  Int   */  {Object, Num,    Int,    String}, 
/* String */  {Object, String, String, String},
};


/*
 *  Returns the type to convert to from any two types
 */
inline Type getTypeConversion(Type t1, Type t2)
{
    return (t1 > 3 || t2 > 3) ? Invalid : conversion[t1][t2];
}

/*
 *  Returns a new variable made from v with type t
 *  TODO: clean
 */
void convertType(Variable *v, Type t)
{
    switch(t){
    case Num:; //convert Int to Num
        BigNum n = bignum_new("0");
        mpf_set_z(*n, *(BigInt)v->value);
        v->value = n;
        v->type = Num;
        break;
    case Int:;
        BigInt i = bigint_new("0");
        mpz_set_f(*i, *(BigNum)v->value); 
        v->value = i;
        v->type = Int;
        break;
    case String:
        switch(v->type){
        case Int:;
            char *s = mpz_get_str(NULL, 10, *(BigInt)v->value);
            v->value = s;
            break;
        case Num:;
            long int expptr[1];
            char *r = mpf_get_str(NULL, expptr, 10, 0, *(BigNum)v->value);
            v->value = r;
            break;
        default: break;
        }
        v->type = String;
        break;
    default: break;
    }
}

inline Variable operate(Variable v1, Operator op, Variable v2)
{
    //check for type mismatch
    if(v1.type != v2.type){
        Type cType = getTypeConversion(v1.type, v2.type);
        
        if(v1.type == cType){
            convertType(&v2, cType);
            Variable ret = op.func(v1, v2);
            free_value(v2);
            return ret;
        }else{
            convertType(&v1, cType);
            Variable ret = op.func(v1, v2);
            free_value(v1);
            return ret;
        }
    }
    return op.func(v1, v2);
}

Operator getOperator(TokenType t)
{
    for(int i = 0; i < ARR_SIZE(operators); i++){
        if(operators[i].op == t){
            return operators[i];
        }
    }
    return (Operator) {-1, 0, 0};//return invalid
}

#define INVALID_VAR_IN_EXPR(v) {/*fprintf(stderr, "Invalid value in expression.\n");*/ return v;}

Variable expression(void)
{
    Variable v = getValue(toks[tIndex]);
    if(v.type == Invalid)
        INVALID_VAR_IN_EXPR(v);

    v = _expression(v, 0);
    INC_POS(1);
    return v;
}

Variable _expression(Variable l, uint8_t minPrecedence)
{
    Operator lookAhead = getOperator(toks[tIndex+1].type);
    while(lookAhead.op != -1 && lookAhead.prec >= minPrecedence){
        Operator op = lookAhead;
        INC_POS(2);
        Variable r = getValue(toks[tIndex]);

        if(r.type == Invalid)
            INVALID_VAR_IN_EXPR(r);

        lookAhead = getOperator(toks[tIndex + 1].type);

        while(lookAhead.op != -1 && (lookAhead.prec > op.prec || (lookAhead.rAsso && lookAhead.prec >= op.prec))){
            r = _expression(r, lookAhead.prec);
            if(r.type == Invalid)
                INVALID_VAR_IN_EXPR(r);
            
            lookAhead = getOperator(toks[tIndex + 1].type);
        }
        Variable tmp = l;
        l = operate(l, op, r);
        free_value(tmp);
        free_value(r);
        
        if(l.type == Invalid)
            INVALID_VAR_IN_EXPR(l);
    }
    return l;
}

