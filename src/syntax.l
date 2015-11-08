%Start cmt mlcmt countWs issueWs strLit

%{
#include "tokens.h"
unsigned int yyscope = 0;
unsigned int yycscope = 0;
char ignoreWs = 1;
int analyzeScope();
Token yylex();
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

<strLit>[^\"]*          return TOKLEX(Tok_StrLit, yylineno, yytext);
<strLit>"\""            BEGIN(INITIAL);

<INITIAL>"\""           BEGIN(strLit);
<INITIAL>"i8"           return TOK(Tok_I8, yylineno);
<INITIAL>"i16"          return TOK(Tok_I16, yylineno);
<INITIAL>"i32"          return TOK(Tok_I32, yylineno);
<INITIAL>"i64"          return TOK(Tok_I64, yylineno);
<INITIAL>"u8"           return TOK(Tok_U8, yylineno);
<INITIAL>"u16"          return TOK(Tok_U16, yylineno);
<INITIAL>"u32"          return TOK(Tok_U32, yylineno);
<INITIAL>"u64"          return TOK(Tok_U64, yylineno);
<INITIAL>"f32"          return TOK(Tok_F32, yylineno);
<INITIAL>"f64"          return TOK(Tok_F64, yylineno);
<INITIAL>"bool"         return TOK(Tok_Bool, yylineno);
<INITIAL>"void"         return TOK(Tok_Void, yylineno);

<INITIAL>"=="           return TOK(Tok_Eq, yylineno);
<INITIAL>"!="           return TOK(Tok_NotEq, yylineno);
<INITIAL>"+="           return TOK(Tok_AddEq, yylineno);
<INITIAL>"-="           return TOK(Tok_SubEq, yylineno);
<INITIAL>"*="           return TOK(Tok_MulEq, yylineno);
<INITIAL>"/="           return TOK(Tok_DivEq, yylineno);
<INITIAL>">="           return TOK(Tok_GrtrEq, yylineno);
<INITIAL>"<="           return TOK(Tok_LesrEq, yylineno);
<INITIAL>"->"           return TOK(Tok_Indent, yylineno);
<INITIAL>".."           return TOK(Tok_StrCat, yylineno);
<INITIAL>"or"           return TOK(Tok_BoolOr, yylineno);
<INITIAL>"and"          return TOK(Tok_BoolAnd, yylineno);
<INITIAL>"true"         return TOK(Tok_TrueLit, yylineno);
<INITIAL>"false"        return TOK(Tok_FalseLit, yylineno);
<INITIAL>{intlit}       return TOKLEX(Tok_IntLit, yylineno, yytext);
<INITIAL>{fltlit}       return TOKLEX(Tok_FltLit, yylineno, yytext);

<INITIAL>"return"       return TOK(Tok_Return, yylineno);
<INITIAL>"if"           return TOK(Tok_If, yylineno);
<INITIAL>"elif"         return TOK(Tok_Elif, yylineno);
<INITIAL>"else"         return TOK(Tok_Else, yylineno);
<INITIAL>"for"          return TOK(Tok_For, yylineno);
<INITIAL>"foreach"      return TOK(Tok_ForEach, yylineno);
<INITIAL>"while"        return TOK(Tok_While, yylineno);
<INITIAL>"do"           return TOK(Tok_Do, yylineno);
<INITIAL>"in"           return TOK(Tok_In, yylineno);
<INITIAL>"continue"     return TOK(Tok_Continue, yylineno);
<INITIAL>"break"        return TOK(Tok_Break, yylineno);
<INITIAL>"import"       return TOK(Tok_Import, yylineno);
<INITIAL>"enum"         return TOK(Tok_Enum, yylineno);
<INITIAL>"struct"       return TOK(Tok_Struct, yylineno);
<INITIAL>"class"        return TOK(Tok_Class, yylineno);

<INITIAL>"\n"           {BEGIN(countWs); unput('\n');}
<INITIAL>[ \t]          ;
<INITIAL>{ident}        return Tok_Identifier;
<INITIAL>.              return yytext[0];
%%

//#define ECHO fwrite(yytext, yyleng, 1, yyout)

int analyzeScope(){
    Token t;

    if(yycscope > yyscope){
        tok = TOK(Tok_Unindent, yylineno);
        yycscope -= 1;
    }else if(yycscope < yyscope){
        tok = TOK(Tok_Indent, yylineno);
        yycscope += 1;
    }else{
        tok = TOK(Tok_Newline, yylineno);
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
