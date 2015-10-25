#include <stdlib.h>
#include <stdio.h>
#include <gmp.h>

typedef mpf_t _Num;
typedef _Num* BigNum;

typedef mpz_t _Int;
typedef _Int* BigInt;

#define BN_ALLOC() (malloc(sizeof(_Num)))
#define BI_ALLOC() (malloc(sizeof(_Int)))

BigNum bignum_init(void);
BigNum bignum_new(char*);
BigNum bignum_copy(BigNum);
BigNum bignum_add(BigNum, BigNum);
BigNum bignum_sub(BigNum, BigNum);
BigNum bignum_mul(BigNum, BigNum);
BigNum bignum_div(BigNum, BigNum);
BigNum bignum_pow(BigNum, BigNum);

BigInt bignum_les(BigNum, BigNum);
BigInt bignum_grt(BigNum, BigNum);
BigInt bignum_eq(BigNum, BigNum);
BigInt bignum_neq(BigNum, BigNum);
BigInt bignum_geq(BigNum, BigNum);
BigInt bignum_leq(BigNum, BigNum);

//Integer functions
BigInt bigint_init(void);
BigInt bigint_new(char*);
BigInt bigint_new_ui(unsigned long);
BigInt bigint_copy(BigInt);
BigInt bigint_add(BigInt, BigInt);
BigInt bigint_sub(BigInt, BigInt);
BigInt bigint_mul(BigInt, BigInt);
BigInt bigint_div(BigInt, BigInt);
BigInt bigint_mod(BigInt, BigInt);
BigInt bigint_pow(BigInt, BigInt);

BigInt bigint_les(BigInt, BigInt);
BigInt bigint_grt(BigInt, BigInt);
BigInt bigint_eq(BigInt, BigInt);
BigInt bigint_neq(BigInt, BigInt);
BigInt bigint_leq(BigInt, BigInt);
BigInt bigint_geq(BigInt, BigInt);
