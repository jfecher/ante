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

%right '^'
%right '.'

%nonassoc '(' '['

/*
    All shift/reduce conflicts should be manually dealt with.
*/
%expect 0
%start top_level_stmt_list
%%

top_level_stmt_list: maybe_newline statement_list maybe_newline {setRoot($2);}
                   | %empty  %prec LOW
                   ;

statement_list: statement_list maybe_newline statement {$$ = setNext($1, $3);}
              | statement {$$ = $1;}
              ;

maybe_newline: Newline
             | %empty 
             ;

statement: var_decl      {puts("var_decl"); $$ = $1;}
         | var_assign    {puts("var_assign"); $$ = $1;}
         | fn_decl       {puts("fn_decl"); $$ = $1;}
         | fn_call       {puts("fn_call"); $$ = $1;}
         | data_decl     {puts("data_decl"); $$ = $1;}
         | ret_stmt      {puts("ret_stmt"); $$ = $1;}
         | while_loop    {puts("while_loop"); $$ = $1;}
         | do_while_loop {puts("do_while"); $$ = $1;}
         | for_loop      {puts("for_loop"); $$ = $1;}
         | if_stmt       {puts("if_stmt"); $$ = $1;}
         | enum_decl     {puts("enum_decl"); $$ = $1;}
         ;

ident: Ident %prec Ident {$$ = (Node*)yytext;}
     ;

usertype: UserType  %prec UserType
        ;

intlit: IntLit  %prec IntLit {$$ = mkIntLitNode(yytext);}
      ;

fltlit: FltLit  %prec FltLit {$$ = mkFltLitNode(yytext);}
      ;

strlit: StrLit  %prec StrLit {$$ = mkStrLitNode(yytext);}
      ;

lit_type: I8       {$$ = mkTypeNode(Tok_I8,  NULL);}
        | I16      {$$ = mkTypeNode(Tok_I16, NULL);}
        | I32      {$$ = mkTypeNode(Tok_I32, NULL);}
        | I64      {$$ = mkTypeNode(Tok_I64, NULL);}
        | U8       {$$ = mkTypeNode(Tok_U8,  NULL);}
        | U16      {$$ = mkTypeNode(Tok_U16, NULL);}
        | U32      {$$ = mkTypeNode(Tok_U32, NULL);}
        | U64      {$$ = mkTypeNode(Tok_U64, NULL);}
        | Isz      {$$ = mkTypeNode(Tok_Isz, NULL);}
        | Usz      {$$ = mkTypeNode(Tok_Usz, NULL);}
        | F32      {$$ = mkTypeNode(Tok_F32, NULL);}
        | F64      {$$ = mkTypeNode(Tok_F64, NULL);}
        | C8       {$$ = mkTypeNode(Tok_C8,  NULL);}
        | C32      {$$ = mkTypeNode(Tok_C32, NULL);}
        | Bool     {$$ = mkTypeNode(Tok_Bool, NULL);}
        | Void     {$$ = mkTypeNode(Tok_Void, NULL);}
        | usertype {$$ = mkTypeNode(Tok_UserType, yytext);}
        | ident    %prec Ident { $$ = mkTypeNode(Ident, (char*)$1);}
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

var_decl: decl_prepend ident '=' expr %prec Ident {$$ = mkVarDeclNode((char*)$2, $1, $4);}
        | decl_prepend ident  %prec LOW {$$ = mkVarDeclNode((char*)$2, $1, 0);}
        ;

/* TODO: change arg1 to require node* instead of char* */
var_assign: var '=' expr {$$ = mkVarAssignNode((char*)$1, $3);}
          ;

usertype_list: usertype_list ',' usertype
             | usertype {$$ = $1;}
             ;

generic: '<' usertype_list '>'
       ;

data_decl: modifier_list Data usertype type_decl_block         {$$ = mkVarNode("TEMP");}
         | modifier_list Data usertype generic type_decl_block {$$ = mkVarNode("TEMP");}
         | Data usertype type_decl_block                       {$$ = mkVarNode("TEMP");}
         | Data usertype generic type_decl_block               {$$ = mkVarNode("TEMP");}
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

enum_decl: modifier_list Enum usertype enum_block  {$$ = mkVarNode("TODO: enum_decl node");}
         | Enum usertype enum_block                {$$ = mkVarNode("TODO: enum_decl node");}
         | modifier_list Enum enum_block           {$$ = mkVarNode("TODO: enum_decl node");}
         | Enum enum_block                         {$$ = mkVarNode("TODO: enum_decl node");}
         ;

block: Indent statement_list Unindent {$$ = $2;}
     ;

params: params ',' type_expr ident {$$ = setNext($1, mkNamedValNode((char*)$4, $3));}
      | type_expr ident            {$$ = mkNamedValNode((char*)$2, $1);}
      ;

maybe_params: params {$$ = $1;}
            | %empty {$$ = NULL;}
            ;

fn_decl: decl_prepend ident ':' maybe_params block {$$ = mkFuncDeclNode((char*)$2, $1, $4, $5);}
       | decl_prepend ident '(' maybe_expr ')' ':' maybe_params block {$$ = mkFuncDeclNode((char*)$2, $1, $7, $8);}
       ;

fn_call: ident '(' maybe_expr ')'  %prec '*' {$$ = mkFuncCallNode((char*)$1, $3);}
       ;

ret_stmt: Return expr {$$ = mkRetNode($2);}
        ;

maybe_else: Else block {puts("TODO: else");}
          | %empty
          ;

elif_list: elif_list Elif block
         | Elif block {puts("TODO: elif");}
         ;

maybe_elif_list: elif_list
               | %empty
               ;

if_stmt: If expr block maybe_elif_list maybe_else {$$ = mkIfNode($2, $3);}
       ;

while_loop: While expr block {$$ = mkVarNode("TODO: while_loop node");}
          ;

do_while_loop: Do block While expr {$$ = mkVarNode("TODO: do_while_loop node");}
             ;

for_loop: For var_decl In expr block {$$ = mkVarNode("TODO: for_loop node");}
        ;

var: ident '[' expr ']'  {$$ = $1;} /*TODO*/
   | ident               %prec Ident {$$ = $1;}
   ;

val: fn_call       {$$ = $1;}
   | '(' expr ')'  {$$ = $2;}
   | var           {$$ = $1;}
   | intlit        {$$ = $1;}
   | fltlit        {$$ = $1;}
   | strlit        {$$ = $1;}
   | True          {$$ = mkBoolLitNode(1);}
   | False         {$$ = mkBoolLitNode(0);}
   ;

maybe_expr: expr   {$$ = $1;}
          | %empty {$$ = NULL;}
          ;

expr: expr '+' expr     {$$ = mkVarNode("TODO: expr node");}
    | expr '-' expr     {$$ = mkVarNode("TODO: expr node");}
    | expr '*' expr     {$$ = mkVarNode("TODO: expr node");}
    | expr '/' expr     {$$ = mkVarNode("TODO: expr node");}
    | expr '%' expr     {$$ = mkVarNode("TODO: expr node");}
    | expr '<' expr     {$$ = mkVarNode("TODO: expr node");}
    | expr '>' expr     {$$ = mkVarNode("TODO: expr node");}
    | expr '^' expr     {$$ = mkVarNode("TODO: expr node");}
    | expr '.' expr     {$$ = mkVarNode("TODO: expr node");}
    | expr Eq expr      {$$ = mkVarNode("TODO: expr node");}
    | expr NotEq expr   {$$ = mkVarNode("TODO: expr node");}
    | expr GrtrEq expr  {$$ = mkVarNode("TODO: expr node");}
    | expr LesrEq expr  {$$ = mkVarNode("TODO: expr node");}
    | expr Or expr      {$$ = mkVarNode("TODO: expr node");}
    | expr And expr     {$$ = mkVarNode("TODO: expr node");}
    | expr Range expr   {$$ = mkVarNode("TODO: expr node");}
    | expr RangeEX expr {$$ = mkVarNode("TODO: expr node");}
    | expr RangeBX expr {$$ = mkVarNode("TODO: expr node");}
    | expr RangeX expr  {$$ = mkVarNode("TODO: expr node");}
    | val               {$$ = $1;}
    ;

%%

void yyerror(const char *s){
    fprintf(stderr, "%s\nerrtok = %d\n", s, yychar);
}

