#include "bignum.h"

inline BigNum bignum_init()
{
    BigNum bn = BN_ALLOC();
    mpf_init(*bn);
    return bn;
}

inline BigNum bignum_new(char *val)
{
    BigNum n = malloc(sizeof(_Num));
    mpf_init_set_str(*n, val, 0);
    return n;
}

inline BigNum bignum_copy(BigNum src)
{
    BigNum cpy = malloc(sizeof(_Num));
    mpf_init_set(*cpy, *src);
    return cpy;
}

inline BigNum bignum_add(BigNum n1, BigNum n2)
{
    BigNum sum = bignum_init();
    mpf_add(*sum, *n1, *n2);
    return sum;
}

inline BigNum bignum_mul(BigNum n1, BigNum n2)
{
    BigNum prod = bignum_init();
    mpf_mul(*prod, *n1, *n2);
    return prod;
}


/*
 *  Beginning of bigint functions
 */
inline BigInt bigint_init()
{
    BigInt bi = BI_ALLOC();
    mpz_init(*bi);
    return bi;
}

inline BigInt bigint_new(char *val)
{
    BigInt i = malloc(sizeof(_Int));
    mpz_init_set_str(*i, val, 0);
    return i;
}

inline BigInt bigint_copy(BigInt src)
{
    BigInt cpy = malloc(sizeof(_Int));
    mpz_init_set(*cpy, *src);
    return cpy;
}

inline BigInt bigint_add(BigInt n1, BigInt n2)
{
    BigInt sum = bigint_init();
    mpz_add(*sum, *n1, *n2);
    return sum;
}

inline BigInt bigint_mul(BigInt n1, BigInt n2)
{
    BigInt prod = bigint_init();
    mpz_mul(*prod, *n1, *n2);
    return prod;
}
