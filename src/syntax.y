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

/* This has no effect when generating a c++ parser */
/* Setting verbose for a c++ parser requires %error-verbose, set in the next section */
#define YYERROR_VERBOSE

#include "yyparser.h"

/* Defined in lexer.cpp */
extern int yylex(...);

namespace ante{
    extern void error(string& msg, const char *fileName, unsigned int row, unsigned int col);
}

void yyerror(const char *msg);

bool is_expr_block = false;

%}

/*%locations*/
%error-verbose

%token Ident UserType

/* types */
%token I8 I16 I32 I64 
%token U8 U16 U32 U64
%token Isz Usz F16 F32 F64
%token C8 C32 Bool Void

/* operators */
%token Eq NotEq AddEq SubEq MulEq DivEq GrtrEq LesrEq
%token Or And Range Returns

/* literals */
%token True False
%token IntLit FltLit StrLit

/* keywords */
%token Return
%token If Elif Else
%token For While Do In
%token Continue Break
%token Import Let Var Match
%token Data Enum Fun Ext

/* modifiers */
%token Pub Pri Pro Raw
%token Const Noinit Pathogen

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

%left ';'
%left Let In
%right Where
%left ','

%left Or
%left And     
%left Eq  NotEq GrtrEq LesrEq '<' '>'

%left Range
%left '+' '-'
%left '*' '/' '%'

%right '^'
%left '.'

/* 
    Being below HIGH, this ensures parenthetical expressions will be parsed
    as just order-of operations parenthesis, instead of a single-value tuple.
*/
%nonassoc ')'

%nonassoc '(' '[' Indent
%nonassoc HIGH

/*
    Expect 4 shift/reduce warnings, all from type casting.  Using a glr-parser
    resolves this ambiguity.
*/
%glr-parser
%expect 4
%start top_level_stmt_list
%%

top_level_stmt_list: maybe_newline stmt_list maybe_newline
                   ;

stmt_list: stmt_list stmt {$$ = setNext($1, $2);}
         | stmt           {$$ = setRoot($1);}
         ;

maybe_newline: Newline  %prec Newline
             | %empty   %prec LOW
             ;

/*
 * Statements that will never end with a newline token.
 * Usually statements that require blocks, such as function declarations.
 */
stmt: fn_decl       Newline
    | data_decl     Newline
    | enum_decl     Newline
    | while_loop    Newline
    | do_while_loop Newline
    | for_loop      Newline
    | if_stmt /* NO Newline */
    | var_decl      Newline
    | var_assign    Newline
    | fn_call       Newline
    | ret_stmt      Newline
    | let_binding   Newline
    | extension     Newline
    ;

stmt_no_nl: fn_decl      
          | data_decl    
          | enum_decl    
          | while_loop   
          | do_while_loop
          | for_loop     
          | if_stmt
          | var_decl     
          | var_assign   
          | fn_call      
          | ret_stmt     
          | let_binding  
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

lit_type: I8       {$$ = mkTypeNode(TT_I8,  (char*)"");}
        | I16      {$$ = mkTypeNode(TT_I16, (char*)"");}
        | I32      {$$ = mkTypeNode(TT_I32, (char*)"");}
        | I64      {$$ = mkTypeNode(TT_I64, (char*)"");}
        | U8       {$$ = mkTypeNode(TT_U8,  (char*)"");}
        | U16      {$$ = mkTypeNode(TT_U16, (char*)"");}
        | U32      {$$ = mkTypeNode(TT_U32, (char*)"");}
        | U64      {$$ = mkTypeNode(TT_U64, (char*)"");}
        | Isz      {$$ = mkTypeNode(TT_Isz, (char*)"");}
        | Usz      {$$ = mkTypeNode(TT_Usz, (char*)"");}
        | F16      {$$ = mkTypeNode(TT_F16, (char*)"");}
        | F32      {$$ = mkTypeNode(TT_F32, (char*)"");}
        | F64      {$$ = mkTypeNode(TT_F64, (char*)"");}
        | C8       {$$ = mkTypeNode(TT_C8,  (char*)"");}
        | C32      {$$ = mkTypeNode(TT_C32, (char*)"");}
        | Bool     {$$ = mkTypeNode(TT_Bool, (char*)"");}
        | Void     {$$ = mkTypeNode(TT_Void, (char*)"");}
        | usertype %prec UserType {$$ = mkTypeNode(TT_Data, (char*)$1);}
        | '\'' ident %prec LOW {$$ = mkTypeNode(TT_TypeVar, (char*)$1);}
        /* Low precedence on type vars to prefer idents as normal vars when possible */
        /* Also means type vars will occasionaly be parsed as function calls */
        ;

type: type '*'      %dprec 2  {$$ = mkTypeNode(TT_Ptr,  (char*)"", $1);}
    | type '[' ']'            {$$ = mkTypeNode(TT_Array,(char*)"", $1);}
    | type '(' type_expr ')'  {$$ = mkTypeNode(TT_Func, (char*)"", $1);}  /* f-ptr w/ params*/
    | type '(' ')'            {$$ = mkTypeNode(TT_Func, (char*)"", $1);}  /* f-ptr w/out params*/
    | '(' type_expr ')'       {$$ = $2;}
    | lit_type                {$$ = $1;}
    ;

type_expr_: type_expr_ ',' type {$$ = setNext($1, $3);}
          | type_expr_ '|' type
          | type                {$$ = setRoot($1);}
          ;

type_expr: type_expr_  {Node* tmp = getRoot(); 
                        if(tmp == $1){//singular type, first type in list equals the last
                            $$ = tmp;
                        }else{ //tuple type
                            $$ = mkTypeNode(TT_Tuple, (char*)"", tmp);
                        }
                       }


modifier: Pub      {$$ = mkModNode(Tok_Pub);} 
        | Pri      {$$ = mkModNode(Tok_Pri);}
        | Pro      {$$ = mkModNode(Tok_Pro);}
        | Raw      {$$ = mkModNode(Tok_Raw);}
        | Const    {$$ = mkModNode(Tok_Const);}
        | Ext      {$$ = mkModNode(Tok_Ext);}
        | Noinit   {$$ = mkModNode(Tok_Noinit);}
        | Pathogen {$$ = mkModNode(Tok_Pathogen);}
        ;

modifier_list_: modifier_list_ modifier {$$ = setNext($1, $2);}
              | modifier {$$ = setRoot($1);}
              ;

modifier_list: modifier_list_ {$$ = getRoot();}
             ;


var_decl: modifier_list type_expr ident '=' expr  %prec Ident {$$ = mkVarDeclNode((char*)$3, $1, $2, $5);}
        | modifier_list type_expr ident           %prec LOW   {$$ = mkVarDeclNode((char*)$3, $1, $2,  0);}
        | type_expr ident '=' expr                %prec Ident {$$ = mkVarDeclNode((char*)$2, 0,  $1, $4);}
        | type_expr ident                         %prec LOW   {$$ = mkVarDeclNode((char*)$2, 0,  $1,  0);}
        | modifier_list Var ident '=' expr                    {$$ = mkVarDeclNode((char*)$2, $1,  0, $5);}
        | Var ident '=' expr                                  {$$ = mkVarDeclNode((char*)$2, 0,   0, $4);}
        ;

let_binding: Let modifier_list type_expr ident '=' expr  {$$ = mkLetBindingNode((char*)$3, $2, $3, $6);}
           | Let modifier_list ident '=' expr            {$$ = mkLetBindingNode((char*)$2, $2, 0,  $5);}
           | Let type_expr ident '=' expr                {$$ = mkLetBindingNode((char*)$3, 0,  $2, $5);}
           | Let ident '=' expr                          {$$ = mkLetBindingNode((char*)$2, 0,  0,  $4);}
           ;

/* TODO: change arg1 to require node* instead of char* */
var_assign: ref_val '=' expr   {$$ = mkVarAssignNode($1, $3);}
          | ref_val AddEq expr {$$ = mkVarAssignNode($1, mkBinOpNode('+', mkUnOpNode('@', $1), $3), false);}
          | ref_val SubEq expr {$$ = mkVarAssignNode($1, mkBinOpNode('-', mkUnOpNode('@', $1), $3), false);}
          | ref_val MulEq expr {$$ = mkVarAssignNode($1, mkBinOpNode('*', mkUnOpNode('@', $1), $3), false);}
          | ref_val DivEq expr {$$ = mkVarAssignNode($1, mkBinOpNode('/', mkUnOpNode('@', $1), $3), false);}
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

type_decl: type_expr ident {$$ = mkNamedValNode(mkVarNode((char*)$2), $1);}
         | type_expr       {$$ = mkNamedValNode(0, $1);}
         | enum_decl /* TODO */
         ;

type_decl_list: type_decl_list Newline type_decl  {$$ = setNext($1, $3);}
              | type_decl                         {$$ = setRoot($1);}
              ;

type_decl_block: Indent type_decl_list Unindent  {$$ = getRoot();}
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

block: Indent stmt_list stmt_no_nl Unindent {setNext($2, $3); $$ = getRoot();}
     | Indent stmt_no_nl Unindent {$$ = $2;}
     ;

raw_ident_list: raw_ident_list ident  {$$ = setNext($1, mkVarNode((char*)$2));}
              | ident             {$$ = setRoot(mkVarNode((char*)$1));}
              ;

ident_list: raw_ident_list {$$ = getRoot();}

/* 
 * In case of multiple parameters declared with a single type, eg i32 a b c
 * The next parameter should be set to the first in the list, (the one returned by getRoot()),
 * but the variable returned must be the last in the last, in this case $4
 */
_params: _params ',' type_expr ident_list {$$ = setNext($1, mkNamedValNode($4, $3));}
      | type_expr ident_list            {$$ = setRoot(mkNamedValNode($2, $1));}
      ;

params: _params {$$ = getRoot();}


fn_decl: modifier_list Fun ident ':' params Returns type_expr block {$$ = mkFuncDeclNode((char*)$3, $1, $7,                             $5, $8);}
       | modifier_list Fun ident ':' params block                   {$$ = mkFuncDeclNode((char*)$3, $1, mkTypeNode(TT_Void, (char*)""), $5, $6);}
       | modifier_list Fun ident Returns type_expr block            {$$ = mkFuncDeclNode((char*)$3, $1, $5,                              0, $6);}
       | modifier_list Fun ident block                              {$$ = mkFuncDeclNode((char*)$3, $1, mkTypeNode(TT_Void, (char*)""),  0, $4);}
       | Fun ident ':' params Returns type_expr block               {$$ = mkFuncDeclNode((char*)$2,  0, $6,                             $4, $7);}
       | Fun ident ':' params block                                 {$$ = mkFuncDeclNode((char*)$2,  0, mkTypeNode(TT_Void, (char*)""), $4, $5);}
       | Fun ident Returns type_expr block                          {$$ = mkFuncDeclNode((char*)$2,  0, $4,                              0, $5);}
       | Fun ident block                                            {$$ = mkFuncDeclNode((char*)$2,  0, mkTypeNode(TT_Void, (char*)""),  0, $3);}
       ;


fn_call: ident tuple {$$ = mkFuncCallNode((char*)$1, $2);}
       ;

ret_stmt: Return expr {$$ = mkRetNode($2);}
        ;


extension: Ext type_expr Indent fn_list Unindent {$$ = mkExtNode($2, $4);}
         ;


fn_list: fn_list_ {$$ = getRoot();}

fn_list_: fn_list_ fn_decl maybe_newline  {$$ = setNext($1, $2);} 
        | fn_decl maybe_newline           {$$ = setRoot($1);}
        ;


/*
 * Due to parsing ambiguities with elif_lists, elif_list, maybe_elif_list, and if_stmt
 * must all manually deal with Newlines seperating the statements, and must have a following
 * Newline under their declaration under 'stmt' like the other statements do
 */
elif_list: elif_list Newline Elif expr block {$$ = setElse((IfNode*)$1, (IfNode*)mkIfNode($4, $5));}
         | Elif expr block                   {$$ = setRoot(mkIfNode($2, $3));}
         ;

maybe_elif_list: elif_list Newline Else block Newline {setElse((IfNode*)$1, (IfNode*)mkIfNode(NULL, $4));}
               | elif_list Newline                    {$$ = setRoot($1);}
               | Else block Newline                   {$$ = setRoot(mkIfNode(NULL, $2));}
               | %empty                               {$$ = setRoot(NULL);}
               ;

if_stmt: If expr block Newline maybe_elif_list {$$ = mkIfNode($2, $3, (IfNode*)getRoot());}
       ;

while_loop: While expr block {$$ = mkWhileNode($2, $3);}
          ;

do_while_loop: Do While expr block {$$ = NULL;}
             ;

for_loop: For ident In expr block {$$ = NULL;}
        ;

var: ident  %prec Ident {$$ = mkVarNode((char*)$1);}
   ;

ref_val: '&' ref_val         {$$ = mkUnOpNode('&', $2);}
       | '@' ref_val         {$$ = mkUnOpNode('@', $2);}
       | ident '[' nl_expr ']'  {$$ = mkBinOpNode('[', mkRefVarNode((char*)$1), $3);}
       | ident  %prec Ident  {$$ = mkRefVarNode((char*)$1);}
       ;

val: fn_call                               {$$ = $1;}
   | '(' nl_expr ')'                          {$$ = $2;}
   | tuple                                 {$$ = $1;}
   | array                                 {$$ = $1;}
   | unary_op                              {$$ = $1;}
   | var                                   {$$ = $1;}
   | intlit                                {$$ = $1;}
   | fltlit                                {$$ = $1;}
   | strlit                                {$$ = $1;}
   | True                                  {$$ = mkBoolLitNode(1);}
   | False                                 {$$ = mkBoolLitNode(0);}
   ;

tuple: '(' expr_list ')'             {$$ = mkTupleNode($2);}
     | '(' ')'                     {$$ = mkTupleNode(0);}
     ;

array: '[' expr_list ']' {$$ = mkArrayNode($2);}
     | '[' ']'           {$$ = mkArrayNode(0);}
     ;

/*
maybe_expr: expr    {$$ = $1;}
          | %empty  {$$ = NULL;}
          ;
*/

/*
expr_list: expr_list_p {$$ = getRoot();}
         ;

expr_list_p: expr_list_p ',' expr  {$$ = setNext($1, $3);}
           | expr       %prec LOW  {$$ = setRoot($1);} 
           /* Low precedence here to favor parenthesis as grouping when possible 
              instead of being parsed as a single-value tuple. */


unary_op: '@' val                 {$$ = mkUnOpNode('@', $2);}
        | '&' val                 {$$ = mkUnOpNode('&', $2);}
        | '-' val                 {$$ = mkUnOpNode('-', $2);}
        | type_expr val           {$$ = mkTypeCastNode($1, $2);}
        ;

expr: basic_expr {$$ = $1;}

basic_expr: basic_expr '+' maybe_newline basic_expr            %dprec 2 {$$ = mkBinOpNode('+', $1, $4);}
          | basic_expr '-' maybe_newline basic_expr            %dprec 2 {$$ = mkBinOpNode('-', $1, $4);}
          | basic_expr '*' maybe_newline basic_expr            %dprec 2 {$$ = mkBinOpNode('*', $1, $4);}
          | basic_expr '/' maybe_newline basic_expr            %dprec 2 {$$ = mkBinOpNode('/', $1, $4);}
          | basic_expr '%' maybe_newline basic_expr            %dprec 2 {$$ = mkBinOpNode('%', $1, $4);}
          | basic_expr '<' maybe_newline basic_expr            %dprec 2 {$$ = mkBinOpNode('<', $1, $4);}
          | basic_expr '>' maybe_newline basic_expr            %dprec 2 {$$ = mkBinOpNode('>', $1, $4);}
          | basic_expr '^' maybe_newline basic_expr            %dprec 2 {$$ = mkBinOpNode('^', $1, $4);}
          | basic_expr '.' maybe_newline var                   %dprec 2 {$$ = mkBinOpNode('.', $1, $4);}
          | basic_expr ';' maybe_newline basic_expr            %dprec 2 {$$ = mkBinOpNode(';', $1, $4);}
          | basic_expr '[' nl_expr ']'                         %dprec 2 {$$ = mkBinOpNode('[', $1, $3);}
          | basic_expr Where ident '=' basic_expr %prec Where  %dprec 2 {$$ = mkBinOpNode(Tok_Where, $1, mkLetBindingNode((char*)$3, 0, 0, $5));}
          | Let ident '=' basic_expr In basic_expr  %prec Let  %dprec 2 {$$ = mkBinOpNode(Tok_Let, mkLetBindingNode((char*)$2, 0, 0, $4), $6);}
          | basic_expr Eq maybe_newline basic_expr             %dprec 2 {$$ = mkBinOpNode(Tok_Eq, $1, $4);}
          | basic_expr NotEq maybe_newline basic_expr          %dprec 2 {$$ = mkBinOpNode(Tok_NotEq, $1, $4);}
          | basic_expr GrtrEq maybe_newline basic_expr         %dprec 2 {$$ = mkBinOpNode(Tok_GrtrEq, $1, $4);}
          | basic_expr LesrEq maybe_newline basic_expr         %dprec 2 {$$ = mkBinOpNode(Tok_LesrEq, $1, $4);}
          | basic_expr Or maybe_newline basic_expr             %dprec 2 {$$ = mkBinOpNode(Tok_Or, $1, $4);}
          | basic_expr And maybe_newline basic_expr            %dprec 2 {$$ = mkBinOpNode(Tok_And, $1, $4);}
          | basic_expr Range maybe_newline basic_expr          %dprec 2 {$$ = mkBinOpNode(Tok_Range, $1, $4);}
          | val                                     %prec LOW  %dprec 2 {$$ = $1;}
          | Indent expr_list Unindent                                 {$$ = $2;}
          ;


/* nl_expr is used in expression blocks and can span multiple lines */
expr_list: expr_list_p {$$ = getRoot();}
         ;

expr_list_p: expr_list_p ',' maybe_newline nl_expr  %prec ',' {$$ = setNext($1, $4);}
           | nl_expr                                %prec LOW {$$ = setRoot($1);}
           ;


nl_expr: nl_expr '+' maybe_newline nl_expr               %dprec 1 {$$ = mkBinOpNode('+', $1, $4);}
       | nl_expr '-' maybe_newline nl_expr               %dprec 1 {$$ = mkBinOpNode('-', $1, $4);}
       | nl_expr '*' maybe_newline nl_expr               %dprec 1 {$$ = mkBinOpNode('*', $1, $4);}
       | nl_expr '/' maybe_newline nl_expr               %dprec 1 {$$ = mkBinOpNode('/', $1, $4);}
       | nl_expr '%' maybe_newline nl_expr               %dprec 1 {$$ = mkBinOpNode('%', $1, $4);}
       | nl_expr '<' maybe_newline nl_expr               %dprec 1 {$$ = mkBinOpNode('<', $1, $4);}
       | nl_expr '>' maybe_newline nl_expr               %dprec 1 {$$ = mkBinOpNode('>', $1, $4);}
       | nl_expr '^' maybe_newline nl_expr               %dprec 1 {$$ = mkBinOpNode('^', $1, $4);}
       | nl_expr '.' maybe_newline nl_expr               %dprec 1 {$$ = mkBinOpNode('.', $1, $4);}
       | nl_expr ';' maybe_newline nl_expr               %dprec 1 {$$ = mkBinOpNode(';', $1, $4);}
       | nl_expr '[' nl_expr ']' maybe_newline           %dprec 1 {$$ = mkBinOpNode('[', $1, $3);}
       | nl_expr Where ident '=' maybe_newline nl_expr   %prec Where  %dprec 3 {$$ = mkBinOpNode(Tok_Where, $1, mkLetBindingNode((char*)$3, 0, 0, $6));}
       | Let ident '=' nl_expr In maybe_newline nl_expr  %prec Let  %dprec 3 {$$ = mkBinOpNode(Tok_Let, mkLetBindingNode((char*)$2, 0, 0, $4), $7);}
       | nl_expr Eq maybe_newline  nl_expr               %dprec 1 {$$ = mkBinOpNode(Tok_Eq, $1, $4);}
       | nl_expr NotEq maybe_newline nl_expr             %dprec 1 {$$ = mkBinOpNode(Tok_NotEq, $1, $4);}
       | nl_expr GrtrEq maybe_newline nl_expr            %dprec 1 {$$ = mkBinOpNode(Tok_GrtrEq, $1, $4);}
       | nl_expr LesrEq maybe_newline nl_expr            %dprec 1 {$$ = mkBinOpNode(Tok_LesrEq, $1, $4);}
       | nl_expr Or maybe_newline nl_expr                %dprec 1 {$$ = mkBinOpNode(Tok_Or, $1, $4);}
       | nl_expr And maybe_newline nl_expr               %dprec 1 {$$ = mkBinOpNode(Tok_And, $1, $4);}
       | val                                 %prec LOW   %dprec 1 {$$ = $1;}
       | Indent expr_list Unindent Newline     %prec HIGH  %dprec 1 {$$ = $2;}
       ;

%%

/* location parser error
void yy::parser::error(const location& loc, const string& msg){
    ante::error(msg.c_str(), yylexer->fileName, yylexer->getRow(), yylexer->getCol());
} */

void yy::parser::error(const string& msg){
    ante::error(msg.c_str(), yylexer->fileName, yylexer->getRow(), yylexer->getCol());
}

#endif
