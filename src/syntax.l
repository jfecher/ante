%Start cmt mlcmt countWs issueWs

%{
#include "tokens.h"
unsigned int yyscope = 0;
unsigned int yycscope = 0;
char ignoreWs = 1;
int analyzeScope();

%}

alpha    [a-zA-Z_]
alphaNum [a-zA-Z0-9_]
digit    [0-9]
ident    {alpha}{alphaNum}*
intlit   "-"?[1-9]{digit}*
fltlit   "-"?[0-9]+"."{digit}+

%%
<INITIAL>"\`"        BEGIN(mlcmt);
<INITIAL>"\~"        BEGIN(cmt);
<cmt>.               ;
<cmt>"\n"            {BEGIN(countWs); unput('\n');}
<mlcmt>[^\`]         ;
<mlcmt>"\`"          BEGIN(INITIAL);

<countWs>"\n"           yyscope = 0;
<countWs>"    "         yyscope++;
<countWs>"\t"           yyscope++;
<countWs>./[^ \t\n]     return analyzeScope();

<INITIAL>"i8"           return Tok_I8;
<INITIAL>"i16"          return Tok_I16;
<INITIAL>"i32"          return Tok_I32;
<INITIAL>"i64"          return Tok_I64;
<INITIAL>"u8"           return Tok_U8;
<INITIAL>"u16"          return Tok_U16;
<INITIAL>"u32"          return Tok_U32;
<INITIAL>"u64"          return Tok_U64;
<INITIAL>"f32"          return Tok_F32;
<INITIAL>"f64"          return Tok_F64;
<INITIAL>"bool"         return Tok_Bool;
<INITIAL>"void"         return Tok_Void;

<INITIAL>"=="           return Tok_Eq;
<INITIAL>"!="           return Tok_NotEq;
<INITIAL>"+="           return Tok_AddEq;
<INITIAL>"-="           return Tok_SubEq;
<INITIAL>"*="           return Tok_MulEq;
<INITIAL>"/="           return Tok_DivEq;
<INITIAL>">="           return Tok_GrtrEq;
<INITIAL>"<="           return Tok_LesrEq;
<INITIAL>"->"           return Tok_Indent;
<INITIAL>".."           return Tok_StrCat;
<INITIAL>"or"           return Tok_BoolOr;
<INITIAL>"and"          return Tok_BoolAnd;
<INITIAL>"true"         return Tok_TrueLit;
<INITIAL>"false"        return Tok_FalseLit;
<INITIAL>{intlit}       return Tok_IntLit;
<INITIAL>{fltlit}       return Tok_FltLit;
<INITIAL>\".*\"         return Tok_StrLit;

<INITIAL>"return"       return Tok_Return;
<INITIAL>"if"           return Tok_If;
<INITIAL>"elif"         return Tok_Elif;
<INITIAL>"else"         return Tok_Else;
<INITIAL>"for"          return Tok_For;
<INITIAL>"foreach"      return Tok_ForEach;
<INITIAL>"while"        return Tok_While;
<INITIAL>"do"           return Tok_Do;
<INITIAL>"in"           return Tok_In;
<INITIAL>"continue"     return Tok_Continue;
<INITIAL>"break"        return Tok_Break;
<INITIAL>"import"       return Tok_Import;
<INITIAL>"enum"         return Tok_Enum;
<INITIAL>"struct"       return Tok_Struct;
<INITIAL>"class"        return Tok_Class;

<INITIAL>"\n"           {BEGIN(countWs); unput('\n');}
<INITIAL>[ \t]          ;
<INITIAL>{ident}        return Tok_Identifier;
<INITIAL>.              return yytext[0];
%%

//#define ECHO fwrite(yytext, yyleng, 1, yyout)

int analyzeScope(){
    int tok; 

    if(yycscope > yyscope){
        tok = Tok_Unindent;
        yycscope -= 1;
    }else if(yycscope < yyscope){
        tok = Tok_Indent;
        yycscope += 1;
    }else{
        tok = Tok_Newline;
        BEGIN(INITIAL);
    }
 
    //avoid sending a newline token after each indent or unindent
    if(yycscope == yyscope)
        BEGIN(INITIAL);

    unput(yytext[0]);
    return tok;
}

//Called when input is exhausted
int yywrap(void){
    return 1;        
}

int main(int argc, char *argv[]){
    int tok = yylex();
    while(tok){
        printf("tok: %s, %s\n", yytext, tokDictionary[tok]);
        tok = yylex();
    }
    return 0;
}
