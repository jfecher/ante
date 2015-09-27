#include "scanline.h"

unsigned int sl_pos = 0;
unsigned int sl_len = 0;
unsigned int sl_hPos = 0;

#define OS_UNIX defined(unix) || defined(__unix__) || defined(__unix) || (defined(__APPLE__) && defined(__MACH__))

#define SET_TERM_X_POS(x) {printf("\r"); for(int i=0; i<x; i++) printf("\033[C");}
#define APPEND_STR(dest, src, destLen, srcLen)           \
    for(int i = destLen; i < (destLen) + (srcLen); i++){ \
        (dest)[i] = (src)[i-(destLen)];                  \
    }

#define STR_TERMINATE(s,x) {(s)[x] = '\0';}
#define STR_DUAL_TERMINATE(s,x) {(s)[x] = '\0'; (s)[x-1] = '\0';}

void setupTerm(){
#ifdef OS_UNIX
    struct termios oldt, newt;
    tcgetattr(STDIN_FILENO, &oldt);
    newt = oldt;
    newt.c_lflag &= ~(ICANON | ECHO);
    tcsetattr(STDIN_FILENO, TCSANOW, &newt);
#endif
}

void init_sl(){
    sl_history = malloc(sizeof(char*) * SL_HISTORY_LEN);
    for(int i = 0; i < SL_HISTORY_LEN; i++)
        sl_history[i] = NULL;
}

void appendHistory(char *str, size_t len){
    char *cpy = malloc(len + 2);
    strcpy(cpy, str);
    STR_DUAL_TERMINATE(cpy, len+1);

    NFREE(sl_history[SL_HISTORY_LEN-1]);

    for(int i = SL_HISTORY_LEN-2; i >= 0; i--){
        if(sl_history[i] != NULL){
            sl_history[i+1] = sl_history[i];
        }
    }

    sl_history[0] = cpy;
}

void removeCharAt(char **str, unsigned int pos){
    size_t size = strlen(*str) + 2;
    size_t endLen = size - pos;
    
    char *end = malloc(endLen+1);
    STR_TERMINATE(end, endLen)
    
    strcpy(end, *str + pos + 1);
    ralloc(str, size-1);
    STR_TERMINATE(*str, size-2);

    APPEND_STR(*str, end, pos, endLen-1);
    free(end);
}

void freeHistory(){
    for(int i = 0; i < SL_HISTORY_LEN; i++){
        NFREE(sl_history[i]);
    }
    NFREE(sl_history);
}

void concatChar(char **str, char c, unsigned int pos){
    size_t len = strlen(*str);
    size_t endLen = len-pos+1;

    char *end = malloc(endLen+1);
    strcpy(end, *str + pos);
    STR_TERMINATE(end, endLen);

    len += 2;
    ralloc(str, len + 1);

    (*str)[pos] = c;
    STR_DUAL_TERMINATE(*str, len);

    APPEND_STR(*str, end, pos+1, endLen);
    free(end);
}

void setSrcLineFromHistory(){
    printf("\r: ");
    for(int i=0;i<sl_len;i++)
        printf(" ");
 
    NFREE(srcLine);
    sl_len = strlen(sl_history[sl_hPos]);
    srcLine = malloc(sl_len+2);
    STR_DUAL_TERMINATE(srcLine, sl_len+1);
    strcpy(srcLine, sl_history[sl_hPos]);
    sl_pos = sl_len; 
}

void handleEsqSeq(){
    if(getchar() == 91){ //discard escape sequence otherwise
        char escSeq = getchar();
        if(escSeq == 68 && sl_pos > 0){//left
            sl_pos--;
        }else if(escSeq == 67 && sl_pos < sl_len){//right
            sl_pos++;
        }else if(escSeq == 65){//up
            if(sl_hPos < SL_HISTORY_LEN-1 && sl_history[sl_hPos]){
                if(sl_hPos == 0){
                    if(sl_len > 0){
                        if(strcmp(srcLine, sl_history[0]) != 0){
                            appendHistory(srcLine, sl_len);
                        }
                    }else{
                        sl_hPos--;
                    }
                }

                if(sl_history[sl_hPos+1]){
                    sl_hPos++;
                    setSrcLineFromHistory();
                }
            }
        }else if(escSeq == 66){//down
            if(sl_hPos > 0 && sl_history[sl_hPos-1]){
                sl_hPos--;
                setSrcLineFromHistory();
            }
        }
    }
}

void scanLine(){
    char c = 0;
    sl_len = 0;
    NFREE(srcLine);
    srcLine = calloc(sizeof(char), 2);
    printf(": ");

    do{
        c = getchar();
        sl_len = strlen(srcLine);

        if(c == 9 || (c >= 32 && c <= 126)){
            concatChar(&srcLine, c, sl_pos);
            sl_pos++;
            sl_len++;
        }else if((c == 8 || c == 127) && sl_pos > 0){ //backspace
            sl_pos--;
            removeCharAt(&srcLine, sl_pos);
            printf("\r: %s  ", srcLine); //screen must be manually cleared of deleted character
        }else if(c == 27){
            handleEsqSeq();
        }

        //seperate input by tokens for syntax highlighting
        init_lexer(1);
        Token *t = lexer_next(1);
        freeToks(&t);

        if(sl_pos != sl_len){
            SET_TERM_X_POS(sl_pos+2);
        }
    }while(c != '\n');
   
    puts("");
    sl_pos = 0;
    sl_hPos = 0;
    
    if(sl_len > 0)
        appendHistory(srcLine, sl_len);
}
