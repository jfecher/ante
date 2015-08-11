#include "stdio.h"
#include "stdlib.h"
#include "string.h"
#include "math.h"

typedef char* BigNum;


BigNum bignum_new(char* val);
BigNum bignum_copy(BigNum n);

BigNum add(BigNum addend1, BigNum addend2);
BigNum multiply(BigNum a1, BigNum a2);

char isnumeric(char*str);
void swap(void**p1, void**p2);

