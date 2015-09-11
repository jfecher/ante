#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <math.h>
#include <gmp.h>

typedef mpf_t _Num;
typedef _Num* BigNum;

typedef mpz_t _Int;
typedef _Int* BigInt;

#define BN_ALLOC() (malloc(sizeof(_Num)))
#define BI_ALLOC() (malloc(sizeof(_Int)))

BigNum bignum_new(char*);
BigNum bignum_copy(BigNum);
BigNum bignum_add(BigNum, BigNum);
BigNum bignum_sub(BigNum, BigNum);
BigNum bignum_mul(BigNum, BigNum);
BigNum bignum_div(BigNum, BigNum);

BigInt bigint_new(char*);
BigInt bigint_copy(BigInt);
BigInt bigint_add(BigInt, BigInt);
BigInt bigint_sub(BigInt, BigInt);
BigInt bigint_mul(BigInt, BigInt);
BigInt bigint_div(BigInt, BigInt);
BigInt bigint_mod(BigInt, BigInt);
BigInt bigint_pow(BigInt, BigInt);
