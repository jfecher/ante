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

inline BigNum bignum_sub(BigNum n1, BigNum n2)
{
    BigNum dif = bignum_init();
    mpf_sub(*dif, *n1, *n2);
    return dif;
}

inline BigNum bignum_mul(BigNum n1, BigNum n2)
{
    BigNum prod = bignum_init();
    mpf_mul(*prod, *n1, *n2);
    return prod;
}

inline BigNum bignum_div(BigNum n1, BigNum n2)
{
    BigNum quo = bignum_init();
    mpf_div(*quo, *n1, *n2);
    return quo;
}
//TODO: implement bignum_mod and bignum_pow


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
    BigInt i = BI_ALLOC();
    mpz_init_set_str(*i, val, 0);
    return i;
}

inline BigInt bigint_copy(BigInt src)
{
    BigInt cpy = BI_ALLOC();
    mpz_init_set(*cpy, *src);
    return cpy;
}

inline BigInt bigint_add(BigInt n1, BigInt n2)
{
    BigInt sum = bigint_init();
    mpz_add(*sum, *n1, *n2);
    return sum;
}

inline BigInt bigint_sub(BigInt n1, BigInt n2)
{
    BigInt prod = bigint_init();
    mpz_sub(*prod, *n1, *n2);
    return prod;
}

inline BigInt bigint_mul(BigInt n1, BigInt n2)
{
    BigInt prod = bigint_init();
    mpz_mul(*prod, *n1, *n2);
    return prod;
}

inline BigInt bigint_div(BigInt n1, BigInt n2)
{
    BigInt prod = bigint_init();
    mpz_div(*prod, *n1, *n2);
    return prod;
}

inline BigInt bigint_mod(BigInt n1, BigInt n2)
{
    BigInt prod = bigint_init();
    mpz_mod(*prod, *n1, *n2);
    return prod;
}

//TODO: implement for negative exponents
inline BigInt bigint_pow(BigInt n1, BigInt n2)
{
    BigInt res = bigint_init();
    BigInt mod = bigint_new("1000000000000007");
    mpz_mul(*mod,*mod,*mod);
    mpz_mul(*mod,*mod,*mod);
    mpz_mul(*mod,*mod,*mod);
    mpz_mul(*mod,*mod,*mod);
    mpz_mul(*mod,*mod,*mod);
    mpz_mul(*mod,*mod,*mod);
    mpz_mul(*mod,*mod,*mod);
    
    mpz_powm(*res, *n1, *n2, *mod); //FIXME: I break when given a negative n2

    mpz_clear(*mod);
    free(mod);
    return res;
}

