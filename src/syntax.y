%{
#include <stdlib.h>
#include <stdio.h>
#include <tokens.h>
#include <ptree.h>

extern int yylex();
extern char *yytext;

void yyerror(const char *msg);

#define YYSTYPE Node*
#define YYERROR_VERBOSE

%}

%token Ident UserType

/* types */
%token I8 I16 I32 I64 
%token U8 U16 U32 U64
%token Isz Usz F32 F64
%token C8 C32 Bool Void

/* operators */
%token Eq NotEq AddEq SubEq MulEq DivEq GrtrEq LesrEq
%token Or And
%token Range RangeBX RangeEX RangeX

/* literals */
%token True False
%token IntLit FltLit StrLit

/* keywords */
%token Return
%token If Elif Else
%token For While Do In
%token Continue Break
%token Import Match
%token Data Enum

/* modifiers */
%token Pub Pri Pro
%token Const Ext Dyn Pathogen

/* other */
%token Where Infect Cleanse Ct

/* whitespace */
%token Newline Indent Unindent


/*
    Now to manually fix all shift/reduce conflicts
*/

/*
    Fake precedence rule to allow for a lower precedence
    than Ident in decl context
*/
%nonassoc LOW

%left Ident

%left IntLit FltLit StrLit True False

%left ','

%left Or
%left And     
%left Eq  NotEq GrtrEq LesrEq '<' '>'
%left Range RangeBX RangeEX RangeX  

%left '+' '-'
%left '*' '/' '%'

%right '.'

%nonassoc '(' '['

/*
    All shift/reduce conflicts should be manually dealt with.
*/
%expect 0
%start top_level_stmt_list
%%

top_level_stmt_list: maybe_newline statement_list maybe_newline
                   | %empty
                   ;

statement_list: statement_list maybe_newline statement { puts("statement_list"); }
              | statement { puts("statement_list: statement"); }
              ;

maybe_newline: Newline
             | %empty
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
         ;

ident: Ident %prec Ident { $$ = (Node*)yytext; }
     ;

usertype: UserType  %prec UserType
        ;

intlit: IntLit  %prec IntLit { $$ = makeIntLitNode(yytext); }
      ;

fltlit: FltLit  %prec FltLit { $$ = makeFltLitNode(yytext); }
      ;

strlit: StrLit  %prec StrLit { $$ = makeStrLitNode(yytext); }
      ;

lit_type: I8       { $$ = makeTypeNode(Tok_I8,  0); }
        | I16      { $$ = makeTypeNode(Tok_I16, 0); }
        | I32      { $$ = makeTypeNode(Tok_I32, 0); }
        | I64      { $$ = makeTypeNode(Tok_I64, 0); }
        | U8       { $$ = makeTypeNode(Tok_U8,  0); }
        | U16      { $$ = makeTypeNode(Tok_U16, 0); }
        | U32      { $$ = makeTypeNode(Tok_U32, 0); }
        | U64      { $$ = makeTypeNode(Tok_U64, 0); }
        | Isz      { $$ = makeTypeNode(Tok_Isz, 0); }
        | Usz      { $$ = makeTypeNode(Tok_Usz, 0); }
        | F32      { $$ = makeTypeNode(Tok_F32, 0); }
        | F64      { $$ = makeTypeNode(Tok_F64, 0); }
        | C8       { $$ = makeTypeNode(Tok_C8,  0); }
        | C32      { $$ = makeTypeNode(Tok_C32, 0); }
        | Bool     { $$ = makeTypeNode(Tok_Bool, 0); }
        | Void     { $$ = makeTypeNode(Tok_Void, 0); }
        | usertype { $$ = makeTypeNode(Tok_UserType, yytext); }
        | ident    %prec Ident { $$ = makeTypeNode(Ident, (char*)$1); }
        ;

type: type '*'
    | type '[' maybe_expr ']'
    | '(' type_expr ')'
    | type '(' type_expr ')' /* f-ptr w/ params*/
    | type '(' ')' /* f-ptr w/out params*/
    | lit_type   {$$ = $1;}
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

decl_prepend: modifier_list type_expr {$$ = $2;} /*TODO: modifier list*/
            | type_expr {$$ = $1;}
            ;

var_decl: decl_prepend ident '=' expr %prec Ident { attatchVarDeclNode((char*)$2, $1, $4); }
        | decl_prepend ident  %prec LOW { attatchVarDeclNode((char*)$2, $1, 0); }
        ;

/* TODO: change arg1 to require node* instead of char* */
var_assign: var '=' expr { attatchVarAssignNode((char*)$1, $3); }
          ;

usertype_list: usertype_list ',' usertype
             | usertype {$$ = $1;}
             ;

generic: '<' usertype_list '>'
       ;

data_decl: modifier_list Data usertype type_decl_block
         | modifier_list Data usertype generic type_decl_block
         | Data usertype type_decl_block
         | Data usertype generic type_decl_block
         ;

type_decl: type_expr ident
         | type_expr
         | enum_decl
         ;

type_decl_list: type_decl_list Newline type_decl
              | type_decl
              ;

type_decl_block: Indent type_decl_list Unindent
               ;

/* Specifying an enum member's value */
val_init_list: val_init_list ',' usertype
             | val_init_list ',' usertype '=' expr
             | val_init_list Newline usertype
             | val_init_list Newline usertype '=' expr
             | usertype '=' expr
             | usertype
             ;

enum_block: Indent val_init_list Unindent
          ;

enum_decl: modifier_list Enum usertype enum_block
         | Enum usertype enum_block
         | modifier_list Enum enum_block
         | Enum enum_block
         ;

block: Indent {newBlock();} statement_list Unindent {endBlock(); $$ = $2;}
     ;

params: params ',' type_expr ident
      | type_expr ident
      ;

maybe_params: params
            | %empty
            ;

fn_decl: decl_prepend ident ':' maybe_params block { puts("fn_decl"); }
       | decl_prepend ident '(' maybe_expr ')' ':' maybe_params block
       ;

fn_call: ident '(' maybe_expr ')'  %prec '*' { puts("fn_call"); }
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

var: ident '[' expr ']'
   | ident               %prec Ident {$$ = $1;}
   ;

val: fn_call       {$$ = $1;}
   | '(' expr ')'  {$$ = $2;}
   | var           {$$ = $1;}
   | intlit        {$$ = $1;}
   | fltlit        {$$ = $1;}
   | strlit        {$$ = $1;}
   | True          {$$ = makeBoolLitNode(1);}
   | False         {$$ = makeBoolLitNode(0);}
   ;

maybe_expr: expr { puts("maybe_expr: true"); }
          | %empty { puts("maybe_expr: false"); }
          ;

expr: expr '+' expr
    | expr '-' expr 
    | expr '*' expr
    | expr '/' expr
    | expr '%' expr 
    | expr '<' expr 
    | expr '>' expr 
    | expr '.' expr
    | expr Eq expr
    | expr NotEq expr
    | expr GrtrEq expr
    | expr LesrEq expr
    | expr Or expr
    | expr And expr
    | expr Range expr
    | expr RangeEX expr
    | expr RangeBX expr
    | expr RangeX expr
    | val
    ;

%%

void yyerror(const char *s){
    fprintf(stderr, "%s\nerrtok = %d\n", s, yychar);
}

