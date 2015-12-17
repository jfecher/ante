%{

#include <stdlib.h>
#include <stdio.h>
#include <tokens.h>

extern int yylex();
extern char *yytext;

void yyerror(const char *msg);

#define YYERROR_VERBOSE

%}

%token Ident

/*types*/
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
%token C32
%token Bool
%token Void

/*operators*/
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
%token Range
%token RangeBX
%token RangeEX
%token RangeX

/*literals*/
%token True
%token False
%token IntLit
%token FltLit
%token StrLit

/*keywords*/
%token Return
%token If
%token Elif
%token Else
%token For
%token While
%token Do
%token In
%token Continue
%token Break
%token Import
%token Match
%token Data
%token Enum

/*modifiers*/
%token Pub
%token Pri
%token Pro
%token Const
%token Ext
%token Dyn
%token Pathogen

/*other*/
%token Where
%token Infect
%token Cleanse
%token Ct

%token Newline
%token Indent
%token Unindent


/*
    Now to manually fix all shift/reduce conflicts
*/
%nonassoc Ident

%left ','

%left '+' '-'
%left '*' '/' '%'

/*
    Used in type casting, high precedence to cast before
    many common operators.
*
%precedence I8
%precedence I16
%precedence I32
%precedence I64
%precedence U8
%precedence U16
%precedence U32
%precedence U64
%precedence ISz
%precedence Usz
%precedence F32
%precedence F64 
%precedence C8
%precedence C32
%precedence Bool
%precedence Void
*/

%precedence '.'

%precedence '(' '['

/*
    All shift/reduce conflicts should be manually dealt with.
*/
%expect 0
%start maybe_statement_list
%%

maybe_statement_list: statement_list
                    | %empty
                    ;

statement_list: statement_list statement { puts("statement_list"); }
              | statement { puts("statement_list: statement"); }
              ;

statement: var_decl
         | var_assign
         | fn_decl
         | fn_call
         | data_decl
         | ret_stmt
         | while_loop
         | do_while_loop
         | for_loop
         | if_stmt
         | enum_decl
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
        | C32
        | Bool
        | Void
        | Ident
        ;

type: type '*'
    | type '[' maybe_expr ']'
    | '(' type_expr ')'
    | lit_type
    ;

type_expr: type_expr ',' type
         | type_expr '|' type
         | type
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
             ;

decl_prepend: modifier_list type_expr
            | type_expr
            ;

var_decl: decl_prepend Ident '=' expr
        | decl_prepend Ident { puts("decl"); }
        ;

var_assign: var '=' expr { puts("var_assign"); }
          ;

ident_list: ident_list ',' Ident
          | Ident
          ;

generic: '<' ident_list '>'
       ;

data_decl: modifier_list Data Ident type_decl_block
         | modifier_list Data Ident generic type_decl_block
         | Data Ident type_decl_block
         | Data Ident generic type_decl_block
         ;

type_decl: type_expr Ident
         | type_expr
         | enum_decl
         ;

type_decl_list: type_decl_list Newline type_decl
              | type_decl
              ;

type_decl_block: Indent type_decl_list Unindent
               ;

val_init_list: val_init_list ',' Ident
             | val_init_list ',' Ident '=' expr
             | val_init_list Newline Ident
             | val_init_list Newline Ident '=' expr
             | Ident '=' expr
             | Ident
             ;

enum_block: Indent val_init_list Unindent
          ;

enum_decl: modifier_list Enum Ident enum_block
         | Enum Ident enum_block
         | modifier_list Enum enum_block
         | Enum enum_block
         ;

block: Indent statement_list Unindent
     ;

params: params ',' type_expr Ident
      | type_expr Ident
      ;

maybe_params: params
            | %empty
            ;

fn_decl: decl_prepend Ident ':' maybe_params block { puts("fn_decl"); }
       | decl_prepend Ident '(' maybe_expr ')' ':' maybe_params block
       ;

fn_call: Ident '(' maybe_expr ')' { puts("fn_call"); }
       ;

ret_stmt: Return expr { puts("ret_stmt"); }
        ;

maybe_else: Else block { puts("else"); }
          | %empty
          ;

elif_list: elif_list Elif block
         | Elif block { puts("elif_list"); }
         ;

maybe_elif_list: elif_list
               | %empty
               ;

if_stmt: If expr block maybe_elif_list maybe_else { puts("if_stmt"); }
       ;

while_loop: While expr block
          ;

do_while_loop: Do block While expr
             ;

for_loop: For var_decl In expr block
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
      | Range
      | RangeEX
      | RangeBX
      | RangeX
      ;

var: Ident '[' expr ']'
   | Ident { puts("var"); }
   ;

val: fn_call
   | '(' expr ')'
   | var
   | IntLit
   | FltLit
   | StrLit
   | True
   | False
   ;

maybe_expr: expr { puts("maybe_expr: true"); }
          | %empty { puts("maybe_expr: false"); }
          ;

expr: l_expr val
    | val
    ;

l_expr: l_expr val bin_op
      | val bin_op
      ;


%%

void yyerror(const char *s){
    fprintf(stderr, "%s\nerrtok = %d\n", s, yychar);
}

