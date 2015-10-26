#include "expression.h"

Operator operators[] = {
    {Tok_Comma,         0, 0, op_tup,  tc_any},
    {Tok_Lesser,        1, 0, op_les,  tc_num},
    {Tok_Greater,       1, 0, op_grt,  tc_num},
    {Tok_EqualsEquals,  1, 0, op_eq,   tc_any},
    {Tok_NotEquals,     1, 0, op_neq,  tc_any},
    {Tok_GreaterEquals, 1, 0, op_geq,  tc_num},
    {Tok_LesserEquals,  1, 0, op_leq,  tc_num},

    {Tok_StrConcat,     2, 0, op_cnct, tc_str},
    {Tok_Plus,          3, 0, op_add,  tc_num},
    {Tok_Minus,         3, 0, op_sub,  tc_num},
    {Tok_Multiply,      4, 0, op_mul,  tc_num},
    {Tok_Divide,        4, 0, op_div,  tc_num},
    {Tok_Modulus,       4, 0, op_mod,  tc_num},
    {Tok_Exponent,      5, 1, op_pow,  tc_num},
};


Type conversion[4][4] = {
//            Object,  Num,    Int,    String
/* Object */  {Object, Object, Object, Object},
/*  Num   */  {Object, Num,    Num,    String},
/*  Int   */  {Object, Num,    Int,    String}, 
/* String */  {Object, String, String, String},
};

char tc_any(Type t1, Type t2){
    return 0;
}

/*
 * Checks if two types are numeric or able to be converted
 * to matching numeric types.
 *
 * Returns 0 if nothing needs to be done, 1 if t1 needs
 * to be converted, 2 if t2 needs to be converted, or 3
 * if both types are incompatible
 */
char tc_num(Type t1, Type t2)
{
    if(t1 == Int || t1 == Num){
        if(t1 == t2) return 0;
        if(t1 == Int && t2 == Num) return 1;
        if(t2 == Int && t1 == Num) return 2;
    }
    return 3;
}

/*
 * Checks if two types are strings or able to be converted
 * to strings.
 *
 * Returns 0 if nothing needs to be done, 1 if t1 needs
 * to be converted, 2 if t2 needs to be converted, or 3
 * if both types are incompatible
 */
char tc_str(Type t1, Type t2)
{
    if(t1 == String){
        if(t2 == String) return 0;
        if(t2 == Int || t2 == Num) return 2;
    }else if(t2 == String && (t1 == Int || t1 == Num)){
        return 1;
    }
    return 3;
}

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
    char c = op.typeImpl(v1.type, v2.type);
    if(c == 1){
        convertType(&v1, getTypeConversion(v1.type, v2.type));
        Variable ret = op.func(v1, v2);
        free_value(v1);
        return ret;
    }else if(c == 2){
        convertType(&v2, getTypeConversion(v1.type, v2.type));
        Variable ret = op.func(v1, v2);
        free_value(v2);
        return ret;
    }else if(c == 3){
        printf("%s operator is not implemented for types %s and %s.\n", tokenDictionary[op.op], typeDictionary[v1.type], typeDictionary[v2.type]);
        return VAR(NULL, Invalid);
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

