#include "bignum.h"

BigNum bignum_new(BigNum val){
    if(isnumeric(val)){
        return bignum_copy(val);
    }else{
        printf("Cannot create bignum from %s\n", val);
        return NULL;
    }
}

BigNum bignum_copy(BigNum src){
    BigNum dest = malloc(sizeof(char) * strlen(src) + 1);
    strcpy(dest, src);
    return dest;
}

void shift(BigNum n){
    char p,c;
    int i;
    for(i=0, p=48; i < strlen(n)+1; i++){
        c = n[i];
        n[i] = p;
        p = c;
    }
}

//Fills a BigNum or string with a single character
void fill(BigNum n, char c, size_t len){
    size_t i;
    for(i=0; i < len; i++){
        n[i] = c;
    }
    n[i] = '\0';
}

BigNum add(BigNum augend, BigNum addend){
    size_t len1 = strlen(augend);
    size_t len2 = strlen(addend);

    bool b = true;
    bool arb[10];
    char arc[10];
    printf("sizeof(arb) = %lu, sizeof(arc) = %lu\n", sizeof(arb), sizeof(arc));
    //If len2 is larger, swap the values so that
    //the augend is larger
    if(len2 > len1){
        BigNum buf = augend;
        augend = addend;
        addend = buf;

        size_t buffer = len1;
        len1 = len2;
        len2 = buffer;
    }

    BigNum sum = bignum_new(augend);

    unsigned long i;
    unsigned char dsum, rem;
    for(i = 0, rem = 0; i < len2 || rem; i++){
        if( addend[len2-i-1] == 48){
            continue;
        }else if( i < len2 ){
            dsum = sum[len1 - i - 1]-48 + addend[len2 - i - 1]-48 + rem;
        }else{
            if(len1-i-1 == -1){
                sum = realloc(sum, sizeof(char) * strlen(sum) + 2);
                shift(sum);
                sum[0] = '1';
                break;
            }
            dsum = (sum[len1 - i - 1]-48 + rem);
        }
        rem = dsum / 10;
        dsum %= 10;
        sum[len1 - i - 1] = dsum + 48;
    }

    return sum;
}

BigNum multiply(BigNum multiplicand, BigNum multiplier){
    size_t len1 = strlen(multiplicand);
    size_t len2 = strlen(multiplier);

    //The maximum result when two integers are multiplied can have no more
    //digits than the multiplicand's digits + the multiplier's digits
    BigNum product = malloc(len1 + len2);
    fill(product, '0', len1 + len2);

    BigNum addend;

    int i, j, base;
    char dproduct;
    for(i = 0; i < len1; i++){
        for(j = 0; j < len2; j++){
            base = i + j;
            dproduct = (multiplicand[len1-i-1]-48) * (multiplier[len2-j-1]-48);

            if(dproduct > 9){
                addend = malloc( 3 + base ); //2 extra chars for each digit of dproduct, 1 extra for \0
                fill(addend, '0', 2 + base);
                addend[0] = (dproduct / 10) + 48;
                addend[1] = (dproduct % 10) + 48;
            }else{
                addend = malloc( 2 + base );
                fill(addend, '0', 1 +base);
                addend[0] = (dproduct % 10) + 48;
            }

            BigNum temp = add(product, addend);
            free(product);
            product = temp;
            free(addend);
        }
    }
    return product;
}

char isnumeric(BigNum str){
    size_t len = strlen(str);
    int i, decimal=0;
    for(i = 0; i < len; i++){
        if(str[i] < 48 || str[i] > 57){
            if(str[i] == '.' && !decimal){
                decimal = 1;
            }else if(!(i == 0 && str[0] == '-')){
                return 0;
            }
        }
    }
    return 1;
}
