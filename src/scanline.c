#include "scanline.h"

unsigned int sl_pos = 0;

#define APPEND_STR(dest, src, destLen, srcLen)           \
    for(int i = destLen; i < (destLen) + (srcLen); i++){ \
        (dest)[i] = (src)[i-(destLen)];                  \
    }

void setupTerm(){
    struct termios oldt, newt;
    tcgetattr(STDIN_FILENO, &oldt);
    newt = oldt;
    newt.c_lflag &= ~(ICANON | ECHO);
    tcsetattr(STDIN_FILENO, TCSANOW, &newt);
}

void removeCharAt(char **str, unsigned int pos){
    size_t size = strlen(*str) + 2;
    size_t endSize = size - pos + 1;
    
    char *end = malloc(endSize);
    end[endSize-1] = '\0';
    
    strcpy(end, *str + pos + 1);
    *str = realloc(*str, size-1);
    (*str)[size-2] = '\0';

    APPEND_STR(*str, end, pos, endSize-2);
    free(end);
}

void concatChar(char **str, char c, unsigned int pos){
    size_t len = strlen(*str);
    size_t endLen = len-pos+1;

    char *end = malloc(endLen+1);
    strcpy(end, *str + pos);
    end[endLen] = '\0';

    len += 2;
    ralloc(str, len + 1);

    (*str)[pos] = c;
    (*str)[len-1] = '\0';
    (*str)[len] = '\0'; //This additional null character is for the lookAhead char in the lexer

    APPEND_STR(*str, end, pos+1, endLen);

    free(end);
}

void scanLine(){
    char c = 0;
    int len = 0;
    srcLine = calloc(sizeof(char), 2);
    printf(": ");

    do{
        c = getchar();
        len = strlen(srcLine);

        if(c == 9 || (c >= 32 && c <= 126)){
            concatChar(&srcLine, c, sl_pos);
            sl_pos++;
            len++;
        }else if((c == 8 || c == 127) && sl_pos > 0){ //backspace
            sl_pos--;
            removeCharAt(&srcLine, sl_pos);
            printf("\r: %s  ", srcLine); //screen must be manually cleared of deleted character
        }else if(c == 27){ //up: (91, 65), down, right, left
            if(getchar() == 91){ //discard escape sequence
                char escSeq = getchar();
                if(escSeq == 68 && sl_pos > 0) sl_pos--;
                else if(escSeq == 67 && sl_pos < strlen(srcLine)) sl_pos++;
            }
            //continue;
        }

        //seperate input by tokens for syntax highlighting
        init_lexer(1);
        Token *t = lexer_next(1);
        freeToks(&t);

        if(sl_pos != len){
            printf("\r: ");
            for(int i=0; i<sl_pos; i++) printf("%c", srcLine[i]);
        }
    }while(c != '\n');
    
    sl_pos = 0;
    puts("");
}
