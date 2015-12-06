%{

#ifndef PARSER_CPP
#define PARSER_CPP

#include <stdlib.h>
#include <stdio.h>
#include <tokens.h>

int yylex();
void yyerror(const char *msg);

%}

%token EndOfInput
%token Ident

//types
%token I8
%token I16
%token I32
%token I64
%token U8
%token U16
%token U32
%token U64
%token ISz
%token Usz
%token F32
%token F64
%token C8
%token C16
%token C32
%token C64
%token Bool
%token Void

//operators
%token Operator
%token Eq
%token NotEq
%token AddEq
%token SubEq
%token MulEq
%token DivEq
%token GrtrEq
%token LesrEq
%token Or
%token And
%token StrCat

//literals
%token True
%token False
%token IntLit
%token FltLit
%token StrLit

//keywords
%token Return
%token If
%token Elif
%token Else
%token For
%token ForEach
%token While
%token Do
%token In
%token Continue
%token Break
%token Import
%token Enum
%token Struct
%token Class

//modifiers
%token Pub
%token Pri
%token Pro
%token Const
%token Ext
%token Dyn
%token Pathogen

//other
%token Where
%token Infect
%token Cleanse
%token Ct

%token Newline
%token Indent
%token Unindent
%%

module: statement_list EndOfInput
      ;

statement_list: statement_list statement
              | statement
              ;

statement: var_decl
         | var_assign
         | fn_decl
         | fn_call
         | ret_stmt
         | while_loop
         | foreach_loop
         | Newline
         ;

lit_type: I8
        | I16
        | I32
        | I64
        | U8
        | U16
        | U32
        | U64
        | ISz
        | Usz
        | F32
        | F64
        | C8
        | C16
        | C32
        | C64
        | Bool
        | Void
        ;

type: type '*'
    | type '[' empty_expr ']'
    | type ',' type
    | lit_type
    ;

modifier: Pub
        | Pri
        | Pro
        | Const
        | Ext
        | Dyn
        | Pathogen
        ;

modifier_list: modifier_list modifier
             | modifier
             | 
             ;

var_decl: modifier_list type Ident '=' expr
        | modifier_list type Ident
        ;

var_assign: var '=' expr
          ;

block: Indent statement_list Unindent
     ;

params: params ',' type Ident
      | type Ident
      ;

fn_decl: type Ident ':' params block
       ;

fn_call: Ident '(' empty_expr ')'
       ;

ret_stmt: Return expr
        ;

while_loop: While expr block
          ;

foreach_loop: ForEach var_decl In expr block
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
      | '.'
      | Eq
      | NotEq
      | AddEq
      | SubEq
      | MulEq
      | DivEq
      | GrtrEq
      | LesrEq
      | Or
      | And
      | StrCat
      ;

var: Ident '[' expr ']'
   | Ident
   ;

val: fn_call
   | var
   | IntLit
   | FltLit
   | StrLit
   | True
   | False
   ;

empty_expr: expr
          | 
          ;

expr: l_expr val
    ;

l_expr: l_expr val bin_op
      | val bin_op
      ;


%%

void yyerror(const char *s){
    fprintf(stderr, "%s\n", s);
}

#endif
