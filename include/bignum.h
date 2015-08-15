#include "stdio.h"
#include "stdlib.h"
#include "string.h"
#include "math.h"

typedef char* BigNum;

BigNum bignum_new(char*);
BigNum bignum_copy(BigNum);

BigNum add(BigNum, BigNum);
BigNum multiply(BigNum, BigNum);

char isnumeric(char*);

