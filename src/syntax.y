%{
#ifndef AN_PARSER
#define AN_PARSER

#include <stdlib.h>
#include <stdio.h>
#include <tokens.h>
#include <ptree.h>

#ifndef YYSTYPE
#define YYSTYPE Node*
#endif
#include "yyparser.h"

extern int yylex(...);

void yyerror(const char *msg);

#define YYERROR_VERBOSE

%}


%token Ident UserType

/* types */
%token I8 I16 I32 I64 
%token U8 U16 U32 U64
%token Isz Usz F16 F32 F64
%token C8 C32 Bool Void

/* operators */
%token Eq NotEq AddEq SubEq MulEq DivEq GrtrEq LesrEq
%token Or And

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
%token Raw Const Ext Pathogen

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

top_level_stmt_list: maybe_newline stmt_list maybe_newline
                   ;

stmt_list: stmt_list maybe_newline stmt {$$ = setNext($1, $3);}
         | stmt   {$$ = setRoot($1);}
         ;

maybe_newline: Newline  %prec Newline
             | %empty   %prec LOW
             ;

stmt: var_decl      {$$ = $1;}
    | var_assign    {$$ = $1;}
    | fn_decl       {$$ = $1;}
    | fn_call       {$$ = $1;}
    | data_decl     {$$ = $1;}
    | ret_stmt      {$$ = $1;}
    | while_loop    {$$ = $1;}
    | do_while_loop {$$ = $1;}
    | for_loop      {$$ = $1;}
    | if_stmt       {$$ = $1;}
    | enum_decl     {$$ = $1;}
    ;

ident: Ident {$$ = (Node*)lextxt;}
     ;

usertype: UserType {$$ = (Node*)lextxt;}
        ;

intlit: IntLit {$$ = mkIntLitNode(lextxt);}
      ;

fltlit: FltLit {$$ = mkFltLitNode(lextxt);}
      ;

strlit: StrLit {$$ = mkStrLitNode(lextxt);}
      ;

lit_type: I8       {$$ = mkTypeNode(Tok_I8,  (char*)"");}
        | I16      {$$ = mkTypeNode(Tok_I16, (char*)"");}
        | I32      {$$ = mkTypeNode(Tok_I32, (char*)"");}
        | I64      {$$ = mkTypeNode(Tok_I64, (char*)"");}
        | U8       {$$ = mkTypeNode(Tok_U8,  (char*)"");}
        | U16      {$$ = mkTypeNode(Tok_U16, (char*)"");}
        | U32      {$$ = mkTypeNode(Tok_U32, (char*)"");}
        | U64      {$$ = mkTypeNode(Tok_U64, (char*)"");}
        | Isz      {$$ = mkTypeNode(Tok_Isz, (char*)"");}
        | Usz      {$$ = mkTypeNode(Tok_Usz, (char*)"");}
        | F16      {$$ = mkTypeNode(Tok_F16, (char*)"");}
        | F32      {$$ = mkTypeNode(Tok_F32, (char*)"");}
        | F64      {$$ = mkTypeNode(Tok_F64, (char*)"");}
        | C8       {$$ = mkTypeNode(Tok_C8,  (char*)"");}
        | C32      {$$ = mkTypeNode(Tok_C32, (char*)"");}
        | Bool     {$$ = mkTypeNode(Tok_Bool, (char*)"");}
        | Void     {$$ = mkTypeNode(Tok_Void, (char*)"");}
        | usertype %prec UserType {$$ = mkTypeNode(Tok_UserType, (char*)$1);}
        | ident    %prec Ident {$$ = mkTypeNode(Tok_Ident, (char*)$1);}
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
         | type {$$ = $1;}
         ;

modifier: Pub
        | Pri
        | Pro
        | Raw
        | Const
        | Ext
        | Pathogen
        ;

modifier_list: modifier_list modifier
             | modifier
             ;

decl_prepend: modifier_list type_expr {$$ = $2;} /*TODO: modifier list*/
            | type_expr {$$ = $1;}
            ;

var_decl: decl_prepend ident '=' expr  %prec Ident {$$ = mkVarDeclNode((char*)$2, $1, $4);}
        | decl_prepend ident  %prec LOW {$$ = mkVarDeclNode((char*)$2, $1, 0);}
        ;

/* TODO: change arg1 to require node* instead of char* */
var_assign: var '=' expr {$$ = mkVarAssignNode($1, $3);}
          ;

usertype_list: usertype_list ',' usertype {$$ = setNext($1, $3);}
             | usertype {$$ = setRoot($1);}
             ;

generic: '<' usertype_list '>' {$$ = getRoot();}
       ;

data_decl: modifier_list Data usertype type_decl_block         {$$ = mkDataDeclNode((char*)$3, $4);}
         | modifier_list Data usertype generic type_decl_block {$$ = mkDataDeclNode((char*)$3, $5);}
         | Data usertype type_decl_block                       {$$ = mkDataDeclNode((char*)$2, $3);}
         | Data usertype generic type_decl_block               {$$ = mkDataDeclNode((char*)$2, $4);}
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
val_init_list: val_init_list Newline usertype
             | val_init_list Newline usertype '=' expr
             | usertype '=' expr
             | usertype
             ;

enum_block: Indent val_init_list Unindent
          ;

enum_decl: modifier_list Enum usertype enum_block  {$$ = NULL;}
         | Enum usertype enum_block                {$$ = NULL;}
         | modifier_list Enum enum_block           {$$ = NULL;}
         | Enum enum_block                         {$$ = NULL;}
         ;

block: Indent stmt_list Unindent {$$ = getRoot();}
     ;

params: params ',' type_expr ident {$$ = setNext($1, mkNamedValNode((char*)$4, $3));}
      | type_expr ident            {$$ = setRoot(mkNamedValNode((char*)$2, $1));}
      ;

maybe_params: params {$$ = getRoot();}
            | %empty {$$ = NULL;}
            ;

fn_decl: decl_prepend ident ':' maybe_params block {$$ = mkFuncDeclNode((char*)$2, $1, $4, $5);}
       | decl_prepend ident '(' maybe_expr ')' ':' maybe_params block {$$ = mkFuncDeclNode((char*)$2, $1, $7, $8);}
       ;

fn_call: ident '(' maybe_expr ')' {$$ = mkFuncCallNode((char*)$1, $3);}
       ;

ret_stmt: Return expr {$$ = mkRetNode($2);}
        ;

elif_list: elif_list Elif expr block {$$ = setElse((IfNode*)$1, (IfNode*)mkIfNode($3, $4));}
         | Elif expr block {$$ = setRoot(mkIfNode($2, $3));}
         ;

maybe_elif_list: elif_list Else block {$$ = setElse((IfNode*)$1, (IfNode*)mkIfNode(NULL, $3));}
               | elif_list {$$ = $1;}
               | Else block {$$ = setRoot(mkIfNode(NULL, $2));}
               | %empty {$$ = setRoot(NULL);}
               ;

if_stmt: If expr block maybe_elif_list {$$ = mkIfNode($2, $3, (IfNode*)getRoot());}
       ;

while_loop: While expr block {$$ = NULL;}
          ;

do_while_loop: Do block While expr {$$ = NULL;}
             ;

for_loop: For var_decl In expr block {$$ = NULL;}
        ;

var: ident '[' expr ']'  {$$ = mkVarNode((char*)$1);} /*TODO: arrays*/
   | ident               %prec Ident {$$ = mkVarNode((char*)$1);}
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

expr: expr_list {$$ = getRoot();}

expr_list: expr_list ',' expr_p    {$$ = setNext($1, $3);}
         | expr_p             {$$ = setRoot($1);}
         ;

expr_p: expr_p '+' expr_p     {$$ = mkBinOpNode('+', $1, $3);}
      | expr_p '-' expr_p     {$$ = mkBinOpNode('-', $1, $3);}
      | expr_p '*' expr_p     {$$ = mkBinOpNode('*', $1, $3);}
      | expr_p '/' expr_p     {$$ = mkBinOpNode('/', $1, $3);}
      | expr_p '%' expr_p     {$$ = mkBinOpNode('%', $1, $3);}
      | expr_p '<' expr_p     {$$ = mkBinOpNode('<', $1, $3);}
      | expr_p '>' expr_p     {$$ = mkBinOpNode('>', $1, $3);}
      | expr_p '^' expr_p     {$$ = mkBinOpNode('^', $1, $3);}
      | expr_p '.' expr_p     {$$ = mkBinOpNode('.', $1, $3);}
      | expr_p Eq expr_p      {$$ = mkBinOpNode(Tok_Eq, $1, $3);}
      | expr_p NotEq expr_p   {$$ = mkBinOpNode(Tok_NotEq, $1, $3);}
      | expr_p GrtrEq expr_p  {$$ = mkBinOpNode(Tok_GrtrEq, $1, $3);}
      | expr_p LesrEq expr_p  {$$ = mkBinOpNode(Tok_LesrEq, $1, $3);}
      | expr_p Or expr_p      {$$ = mkBinOpNode(Tok_Or, $1, $3);}
      | expr_p And expr_p     {$$ = mkBinOpNode(Tok_And, $1, $3);}
      | val                   {$$ = $1;}
      ;

%%

void yy::parser::error(const string& msg){
    cerr << msg << endl;
}

#endif
