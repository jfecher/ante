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


/*
 *  Returns true if the given type is able
 *  to be converted to the other and vice-versa
 */
inline char typeCompatible(Type t1, Type t2)
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
 *  TODO: clean
 */
void convertType(Variable *v, Type t)
{
        switch(t){
        case Num:; //convert Int to Num
            BigNum n = bignum_init();
            mpf_set_z(*n, *(BigInt)v->value);
            free_value(*v);
            v->value = n;
            v->type = Num;
            break;
        case Int:;
            BigInt i = bigint_init();
            mpz_set_f(*i, *(BigNum)v->value); 
            free_value(*v);
            v->value = i;
            v->type = Int;
            break;
        case String:
            switch(v->type){
                case Int:;
                    char *s = mpz_get_str(NULL, 10, *(BigInt)v->value);
                    free_value(*v);
                    v->value = s;
                    break;
                case Num:;
                    long int expptr[1];
                    char *r = mpf_get_str(NULL, expptr, 10, 0, *(BigNum)v->value);
                    free_value(*v);
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
        if(v1.type == Int){ //TODO: clean
            if(v2.type == Num){
                convertType(&v1, Int);
            }else if(v2.type == String){
                convertType(&v1, String);
            }
        }else if(v1.type == Num){
            if(v2.type == Int){
                convertType(&v2, Num);
            }else if(v2.type == String){
                convertType(&v1, String); 
            }
        }else if(v1.type == String){
            convertType(&v2, String);
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
    Operator invalid = {-1, 0, 0};
    return invalid;
}

Variable expression(void)
{
    Variable v = getValue(toks[tIndex]);
    if(v.type == Invalid){
        fprintf(stderr, "Invalid Type in expression\n");
        return v;
    }

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

        if(r.type == Invalid) return r;

        lookAhead = getOperator(toks[tIndex + 1].type);

        while(lookAhead.op != -1 && (lookAhead.prec > op.prec || (lookAhead.rAsso && lookAhead.prec >= op.prec))){
            r = _expression(r, lookAhead.prec);
            lookAhead = getOperator(toks[tIndex + 1].type);
        }
        Variable tmp = l;
        l = operate(l, op, r);
        free_value(tmp);
        free_value(r);
    }
    return l;
}

