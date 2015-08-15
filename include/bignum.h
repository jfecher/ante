#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>

typedef unsigned char* BigNum;

BigNum bignum_new(BigNum);
BigNum bignum_copy(BigNum);

BigNum add(BigNum, BigNum);
BigNum multiply(BigNum, BigNum);

char isnumeric(BigNum);

