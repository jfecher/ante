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
%token Or And Range

/* literals */
%token True False
%token IntLit FltLit StrLit

/* keywords */
%token Return
%token If Elif Else
%token For While Do In
%token Continue Break
%token Import Let Match
%token Data Enum

/* modifiers */
%token Pub Pri Pro Raw
%token Const Ext Noinit Pathogen

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

stmt_list: stmt_list nl_stmt Newline {$$ = setNext($1, $2);}
         | stmt_list no_nl_stmt      {$$ = setNext($1, $2);}
         | nl_stmt Newline           {$$ = setRoot($1);}
         | no_nl_stmt                {$$ = setRoot($1);}
         ;

maybe_newline: Newline  %prec Newline
             | %empty   %prec LOW
             ;

/*
 * Statements that will never end with a newline token.
 * Usually statements that require blocks, such as function declarations.
 */
no_nl_stmt: fn_decl
          | data_decl
          | enum_decl
          | while_loop
          | do_while_loop
          | for_loop
          | if_stmt
          ;

/* Statements that can possibly end in an newline */
nl_stmt: var_decl
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
        ;

let_binding: Let modifier_list type_expr ident '=' expr  {$$ = mkLetBindingNode((char*)$3, $2, $3, $6);}
           | Let modifier_list ident '=' expr            {$$ = mkLetBindingNode((char*)$2, $2, 0,  $5);}
           | Let type_expr ident '=' expr                {$$ = mkLetBindingNode((char*)$3, 0,  $2, $5);}
           | Let ident '=' expr                          {$$ = mkLetBindingNode((char*)$2, 0,  0,  $4);}
           ;

/* TODO: change arg1 to require node* instead of char* */
var_assign: ref_val '=' expr {$$ = mkVarAssignNode($1, $3);}
          | ref_val AddEq expr {$$ = mkVarAssignNode($1, mkBinOpNode('+', mkUnOpNode('@', $1), $3));}
          | ref_val SubEq expr {$$ = mkVarAssignNode($1, mkBinOpNode('-', mkUnOpNode('@', $1), $3));}
          | ref_val MulEq expr {$$ = mkVarAssignNode($1, mkBinOpNode('*', mkUnOpNode('@', $1), $3));}
          | ref_val DivEq expr {$$ = mkVarAssignNode($1, mkBinOpNode('/', mkUnOpNode('@', $1), $3));}
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

block: Indent stmt_list no_nl_stmt Unindent {setNext($2, $3); $$ = getRoot();}
     | Indent stmt_list nl_stmt Unindent    {setNext($2, $3); $$ = getRoot();}
     | Indent no_nl_stmt Unindent           {$$ = $2;}
     | Indent nl_stmt Unindent              {$$ = $2;}
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
params: params ',' type_expr ident_list {$$ = setNext($1, mkNamedValNode($4, $3));}
      | type_expr ident_list            {$$ = setRoot(mkNamedValNode($2, $1));}
      ;

maybe_params: params {$$ = getRoot();}
            | %empty {$$ = NULL;}
            ;

fn_decl: modifier_list type_expr ident ':' maybe_params block                    {$$ = mkFuncDeclNode((char*)$3, $1, $2, $5, $6);}
       | modifier_list type_expr ident '(' maybe_expr ')' ':' maybe_params block {$$ = mkFuncDeclNode((char*)$3, $1, $2, $8, $9);}
       | type_expr ident ':' maybe_params block                                  {$$ = mkFuncDeclNode((char*)$2, 0,  $1, $4, $5);}
       | type_expr ident '(' maybe_expr ')' ':' maybe_params block               {$$ = mkFuncDeclNode((char*)$2, 0,  $1, $7, $8);}
       | Let ident ':' maybe_params '=' expr                                       {$$ = mkFuncDeclNode((char*)$2, 0,  0,  $4, $6);}
       ;

fn_call: ident tuple {$$ = mkFuncCallNode((char*)$1, $2);}
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

do_while_loop: Do While expr block {$$ = NULL;}
             ;

for_loop: For ident In expr block {$$ = NULL;}
        ;

var: ident  %prec Ident {$$ = mkVarNode((char*)$1);}
   ;

ref_val: '&' ref_val         {$$ = mkUnOpNode('&', $2);}
       | '@' ref_val         {$$ = mkUnOpNode('@', $2);}
       | ident '[' expr ']'  {$$ = mkBinOpNode('[', mkRefVarNode((char*)$1), $3);}
       | ident  %prec Ident  {$$ = mkRefVarNode((char*)$1);}
       ;

val: fn_call                 {$$ = $1;}
   | '(' expr ')'            {$$ = $2;}
   | tuple                   {$$ = $1;}
   | array                   {$$ = $1;}
   | Indent nl_expr Unindent {$$ = $2;}
   | unary_op                {$$ = $1;}
   | var                     {$$ = $1;}
   | intlit                  {$$ = $1;}
   | fltlit                  {$$ = $1;}
   | strlit                  {$$ = $1;}
   | True                    {$$ = mkBoolLitNode(1);}
   | False                   {$$ = mkBoolLitNode(0);}
   ;

tuple: '(' expr_list ')'  {$$ = mkTupleNode($2);}
     | '(' ')'            {$$ = mkTupleNode(0);}
     ;

array: '[' expr_list ']' {$$ = mkArrayNode($2);}
     | '[' ']'           {$$ = mkArrayNode(0);}
     ;

maybe_expr: expr    {$$ = $1;}
          | %empty  {$$ = NULL;}
          ;

expr_list: expr_list_p {$$ = getRoot();}
         ;

expr_list_p: expr_list_p ',' expr  {$$ = setNext($1, $3);}
           | expr       %prec LOW  {$$ = setRoot($1);} 
           /* Low precedence here to favor parenthesis as grouping when possible 
              instead of being parsed as a single-value tuple.*/
           ;

unary_op: '@' val       %dprec 1  {$$ = mkUnOpNode('@', $2);}
        | '&' val                 {$$ = mkUnOpNode('&', $2);}
        | '-' val                 {$$ = mkUnOpNode('-', $2);}
        | type_expr val %dprec 2  {$$ = mkTypeCastNode($1, $2);}
        ;

expr: binop {$$ = $1;}
    ;

binop: binop '+' binop                          {$$ = mkBinOpNode('+', $1, $3);}
     | binop '-' binop                          {$$ = mkBinOpNode('-', $1, $3);}
     | binop '*' binop                          {$$ = mkBinOpNode('*', $1, $3);}
     | binop '/' binop                          {$$ = mkBinOpNode('/', $1, $3);}
     | binop '%' binop                          {$$ = mkBinOpNode('%', $1, $3);}
     | binop '<' binop                          {$$ = mkBinOpNode('<', $1, $3);}
     | binop '>' binop                          {$$ = mkBinOpNode('>', $1, $3);}
     | binop '^' binop                          {$$ = mkBinOpNode('^', $1, $3);}
     | binop '.' var                            {$$ = mkBinOpNode('.', $1, $3);}
     | binop ';' maybe_newline binop            {$$ = mkBinOpNode(';', $1, $4);}
     | binop '[' expr ']'                       {$$ = mkBinOpNode('[', $1, $3);}
     | binop Where ident '=' binop %prec Where  {$$ = mkBinOpNode(Tok_Where, $1, mkLetBindingNode((char*)$3, 0, 0, $5));}
     | Let ident '=' expr In binop  %prec Let   {$$ = mkBinOpNode(Tok_Let, mkLetBindingNode((char*)$2, 0, 0, $4), $6);}
     | binop Eq binop                           {$$ = mkBinOpNode(Tok_Eq, $1, $3);}
     | binop NotEq binop                        {$$ = mkBinOpNode(Tok_NotEq, $1, $3);}
     | binop GrtrEq binop                       {$$ = mkBinOpNode(Tok_GrtrEq, $1, $3);}
     | binop LesrEq binop                       {$$ = mkBinOpNode(Tok_LesrEq, $1, $3);}
     | binop Or binop                           {$$ = mkBinOpNode(Tok_Or, $1, $3);}
     | binop And binop                          {$$ = mkBinOpNode(Tok_And, $1, $3);}
     | binop Range binop                        {$$ = mkBinOpNode(Tok_Range, $1, $3);}
     | val                                      {$$ = $1;}
     ;


/* nl_expr is used in expression blocks and can span multiple lines */
nl_expr: nl_expr_list {$$ = getRoot();}
       ;

nl_expr_list: nl_expr_list ',' maybe_newline expr_block_p {$$ = setNext($1, $4);}
            | expr_block_p                                {$$ = setRoot($1);}
            ;

expr_block_p: expr_block_p '+' maybe_newline expr_block_p            {$$ = mkBinOpNode('+', $1, $4);}
            | expr_block_p '-' maybe_newline expr_block_p            {$$ = mkBinOpNode('-', $1, $4);}
            | expr_block_p '*' maybe_newline expr_block_p            {$$ = mkBinOpNode('*', $1, $4);}
            | expr_block_p '/' maybe_newline expr_block_p            {$$ = mkBinOpNode('/', $1, $4);}
            | expr_block_p '%' maybe_newline expr_block_p            {$$ = mkBinOpNode('%', $1, $4);}
            | expr_block_p '<' maybe_newline expr_block_p            {$$ = mkBinOpNode('<', $1, $4);}
            | expr_block_p '>' maybe_newline expr_block_p            {$$ = mkBinOpNode('>', $1, $4);}
            | expr_block_p '^' maybe_newline expr_block_p            {$$ = mkBinOpNode('^', $1, $4);}
            | expr_block_p '.' maybe_newline expr_block_p            {$$ = mkBinOpNode('.', $1, $4);}
            | expr_block_p ';' maybe_newline expr_block_p            {$$ = mkBinOpNode(';', $1, $4);}
            | expr_block_p '[' expr_block_p ']' maybe_newline        {$$ = mkBinOpNode('[', $1, $3);}
            | expr_block_p Where ident '=' maybe_newline expr_block_p %prec Where  {$$ = mkBinOpNode(Tok_Where, $1, mkLetBindingNode((char*)$3, 0, 0, $6));}
            | Let ident '=' expr_block_p In maybe_newline expr_block_p  %prec Let  {$$ = mkBinOpNode(Tok_Let, mkLetBindingNode((char*)$2, 0, 0, $4), $7);}
            | expr_block_p Eq maybe_newline  expr_block_p            {$$ = mkBinOpNode(Tok_Eq, $1, $4);}
            | expr_block_p NotEq maybe_newline expr_block_p          {$$ = mkBinOpNode(Tok_NotEq, $1, $4);}
            | expr_block_p GrtrEq maybe_newline expr_block_p         {$$ = mkBinOpNode(Tok_GrtrEq, $1, $4);}
            | expr_block_p LesrEq maybe_newline expr_block_p         {$$ = mkBinOpNode(Tok_LesrEq, $1, $4);}
            | expr_block_p Or maybe_newline expr_block_p             {$$ = mkBinOpNode(Tok_Or, $1, $4);}
            | expr_block_p And maybe_newline expr_block_p            {$$ = mkBinOpNode(Tok_And, $1, $4);}
            | val                                                    {$$ = $1;}
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
