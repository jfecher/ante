%{

#include <stdlib.h>
#include <stdio.h>
#include <tokens.h>

int yylex(void);

%}

%token I8
%token I16
%token I32
%token I64
%token U8
%token U16
%token U32
%token U64
%token F32
%token F64
%token BOOL
%token VOID

%token IDENT
%%

module: statement_list
      ;

statement_list: statement_list statement
              | statement
              ;

statement: var_decl
         | '\n'
         ;

type: I8
    | I16
    | I32
    | I64
    | U8
    | U16
    | U32
    | U64
    | F32
    | F64
    | BOOL
    | VOID
    ;

var_decl: type IDENT '=' expr
        | type IDENT
        ;

bin_op: '+'
      | '-'
      | '*'
      | '/'
      | '%'
      | '^'
      | '|'
      | '&'
      | '<'
      | '>'
      ;

val: IDENT

expr: l_expr val
    ;

l_expr: l_expr val bin_op
      | val bin_op
      ;


%%

void yyerror(char *s){
    fprintf(stderr, "%s\n", s);
    return;
}

int main(void){
    yyparse();
    return 0;
}
