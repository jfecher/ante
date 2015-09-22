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
    return VAR(bignum_div(n1.value, n2.value), Num);
}

Variable divInt(Variable n1, Variable n2)
{
    return VAR(bigint_div(n1.value, n2.value), Int);
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
    return VAR(bigint_mod(n1.value, n2.value), Int);
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
    Variable v = {malloc(size), String, 0, 0};
    strcpy(v.value, m1.value);
    ((char*)v.value)[size-1] = '\0';
    strcat(v.value, m2.value);
    return v;
}
