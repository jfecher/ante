#include "operations.h"

/*
 *  Add functions
 */
Variable addNum(Variable n1, Variable n2)
{
    return VAR(bignum_add(n1.value, n2.value), Num);
}

Variable addInt(Variable n1, Variable n2)
{
    return VAR(bigint_add(n1.value, n2.value), Int);
}

opFunc addFuncTable[] = {
  //Object, Num,    Int,    String, Function, Invalid  
    NULL,   addNum, addInt, NULL,   NULL,     NULL
};

/*
 *  Subtract functions
 */
Variable subNum(Variable n1, Variable n2)
{
    return VAR(bignum_sub(n1.value, n2.value), Num);
}

Variable subInt(Variable n1, Variable n2)
{
    return VAR(bigint_sub(n1.value, n2.value), Int);
}

opFunc subFuncTable[] = {
  //Object, Num,    Int,    String, Function, Invalid  
    NULL,   subNum, subInt, NULL,   NULL,     NULL
};

/*
 *  Multiply functions
 */
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
 *  Division functions
 */
Variable divNum(Variable n1, Variable n2)
{
    Value v = {bignum_div(n1.value, n2.value)};
    return VAR(v, v != NULL? Num : Invalid);
}

Variable divInt(Variable n1, Variable n2)
{
    Value v = {bigint_div(n1.value, n2.value)};
    return VAR(v, v != NULL? Int : Invalid);
}

opFunc divFuncTable[] = {
  //Object, Num,    Int,    String, Function, Invalid  
    NULL,   divNum, divInt, NULL,   NULL,     NULL
};

/*
 *  Modulus functions
 */
Variable modInt(Variable n1, Variable n2)
{
    Value v = {bigint_mod(n1.value, n2.value)};
    return VAR(v, v != NULL? Int : Invalid);
}

opFunc modFuncTable[] = {
  //Object, Num,    Int,    String, Function, Invalid  
    NULL,   NULL,   modInt, NULL,   NULL,     NULL
};

/*
 *  Pow functions
 */
Variable powInt(Variable n1, Variable n2)
{
    return VAR(bigint_pow(n1.value, n2.value), Int);
}

Variable powNum(Variable n1, Variable n2)
{
    return VAR(bignum_pow(n1.value, n2.value), Num);
}

opFunc powFuncTable[] = {
  //Object, Num,    Int,    String, Function, Invalid  
    NULL,   powNum, powInt, NULL,   NULL,     NULL
};



/*
 *  Exported op functions
 *  Note: these assume both given variables
 *        have the same type
 */
inline Variable op_add(Variable augend, Variable addend)
{
    return addFuncTable[augend.type](augend, addend);
}

inline Variable op_sub(Variable m1, Variable m2)
{
    return subFuncTable[m1.type](m1, m2);
}

inline Variable op_mul(Variable m1, Variable m2)
{
    return mulFuncTable[m1.type](m1, m2);
}

inline Variable op_div(Variable m1, Variable m2)
{
    return divFuncTable[m1.type](m1, m2);
}

inline Variable op_mod(Variable m1, Variable m2)
{
    return modFuncTable[m1.type](m1, m2);
}

inline Variable op_pow(Variable b, Variable e)
{
    return powFuncTable[b.type](b,e);
}

inline Variable op_cnct(Variable m1, Variable m2)
{
    size_t size = strlen(m1.value) + strlen(m2.value) + 1;
    Variable v = VAR(malloc(size), String);
    strcpy(v.value, m1.value);
    ((char*)v.value)[size-1] = '\0';
    strcat(v.value, m2.value);
    return v;
}

//TODO: finish
inline Variable op_tup(Variable v1, Variable v2)
{
    Variable v = VAR(NULL, Tuple);
    struct Tuple *tuple;

    if(v1.type == Tuple){
        v = copyVar(v1);
        tuple = (struct Tuple*)v.value;
    }else{
        v = VAR(malloc(sizeof(struct Tuple)), Tuple);
        tuple = (struct Tuple*)v.value;
        tuple->tup = malloc(sizeof(Variable));
        tuple->tup[0] = copyVar(v1);
        tuple->size = 1;
    }
    
    if(v2.type == Tuple){
        struct Tuple *tup2 = v2.value;
        tuple->tup = realloc(tuple->tup, sizeof(Variable) * (tuple->size + tup2->size));
        for(int i = tuple->size; i < tuple->size + tup2->size; i++){
            tuple->tup[i] = copyVar(tup2->tup[i-tuple->size]);
        }
        tuple->size += tup2->size;
    }else{
        tuple->tup = realloc(tuple->tup, sizeof(Variable) * (tuple->size + 1));
        tuple->tup[tuple->size] = copyVar(v2);
        tuple->size += 1;
    }
    return v;
}


inline Variable op_les(Variable v1, Variable v2)
{
    if(v1.type == Int){
        return VAR(bigint_les(v1.value, v2.value), Int);
    }else{//Num
        return VAR(bignum_les(v1.value, v2.value), Int);
    }
}

inline Variable op_grt(Variable v1, Variable v2)
{
    if(v1.type == Int){
        return VAR(bigint_grt(v1.value, v2.value), Int);
    }else{//Num
        return VAR(bignum_grt(v1.value, v2.value), Int);
    }
}

inline Variable op_eq(Variable v1, Variable v2)
{
    if(v1.type != v2.type) return VAR(bigint_new_ui(0), Int);

    if(v1.type == Int){
        return VAR(bigint_eq(v1.value, v2.value), Int);
    }else if(v1.type == Num){//Num
        return VAR(bignum_eq(v1.value, v2.value), Int);
    }else if(v1.type == String){
        return VAR(bigint_new_ui(streq(v1.value, v2.value)), Int);
    }else{
        return VAR(bigint_new_ui(v1.value == v2.value), Int);
    }
}

inline Variable op_neq(Variable v1, Variable v2)
{
    if(v1.type != v2.type) return VAR(bigint_new_ui(1), Int);

    if(v1.type == Int){
        return VAR(bigint_neq(v1.value, v2.value), Int);
    }else if(v1.type == Num){//Num
        return VAR(bignum_neq(v1.value, v2.value), Int);
    }else if(v1.type == String){
        return VAR(bigint_new_ui(streq(v1.value, v2.value) == 0), Int);
    }else{
        return VAR(bigint_new_ui(v1.value != v2.value), Int);
    }
}

inline Variable op_leq(Variable v1, Variable v2)
{
    if(v1.type == Int){
        return VAR(bigint_leq(v1.value, v2.value), Int);
    }else{//Num
        return VAR(bignum_leq(v1.value, v2.value), Int);
    }
}

inline Variable op_geq(Variable v1, Variable v2)
{
    if(v1.type == Int){
        return VAR(bigint_geq(v1.value, v2.value), Int);
    }else{//Num
        return VAR(bignum_geq(v1.value, v2.value), Int);
    }
}
