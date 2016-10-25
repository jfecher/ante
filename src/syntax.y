%{
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
#include <cstring>

/* Defined in lexer.cpp */
extern int yylex(yy::parser::semantic_type*, yy::location*);

extern string typeNodeToStr(TypeNode*);

struct TypeNode;

Node* mangle_fn(Node *base, Node *nvns);


/*namespace ante{
    extern void error(string& msg, const char *fileName, unsigned int row, unsigned int col);
}*/

void yyerror(const char *msg);

%}

%locations
%error-verbose

%token Ident UserType TypeVar

/* types */
%token I8 I16 I32 I64 
%token U8 U16 U32 U64
%token Isz Usz F16 F32 F64
%token C8 C32 Bool Void

/* operators */
%token Eq NotEq AddEq SubEq MulEq DivEq GrtrEq LesrEq
%token Or And Range RArrow ApplyL ApplyR Append New Not

/* literals */
%token True False
%token IntLit FltLit StrLit CharLit

/* keywords */
%token Return
%token If Then Elif Else
%token For While Do In
%token Continue Break
%token Import Let Var Match With
%token Type Trait Fun Ext

/* modifiers */
%token Pub Pri Pro Raw
%token Const Noinit

/* other */
%token Where

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


%left Newline
%left ';'
%left STMT Fun Let Import Return Ext Var While For Match Trait
%left If
%left Else Elif
%left MED

%left MODIFIER Pub Pri Pro Raw Const Noinit
%left RArrow


%left ','
%left '=' AddEq SubEq MulEq DivEq

%right ApplyL
%left ApplyR

%left Or
%left And     
%left Eq  NotEq GrtrEq LesrEq '<' '>'

%left In
%left Append
%left Range
%left '#'

%left '+' '-'
%left '*' '/' '%'

%nonassoc '!'
%left '@' New Not
%left '&' TYPE UserType TypeVar I8 I16 I32 I64 U8 U16 U32 U64 Isz Usz F16 F32 F64 C8 C32 Bool Void Type '\''
%nonassoc FUNC

%nonassoc LITERALS StrLit IntLit FltLit CharLit True False Ident
%left '.'


/* 
    Being below HIGH, this ensures parenthetical expressions will be parsed
    as just order-of operations parenthesis, instead of a single-value tuple.
*/
%nonassoc ')' ']'

%nonassoc '(' '[' Indent Unindent
%nonassoc HIGH

%expect 0
%start top_level_expr_list
%%

top_level_expr_list:  maybe_newline expr {$$ = setRoot($2);}
                   ;

/*
top_level_expr_list_p: top_level_expr_list_p Newline expr                  %prec Newline {$$ = setNext($1, $3);}
                     | top_level_expr_list_p Newline if_expr Newline expr  %prec Newline {$$ = mkBinOpNode(@$, ';', getRoot(), $5);}
                     | expr                                                %prec Newline {$$ = setRoot($1);}
                     | if_expr Newline expr                                %prec Newline {$$ = setRoot(mkBinOpNode(@$, ';', getRoot()), $3);}
                     ;
*/

maybe_newline: Newline  %prec Newline
             | %empty
             ;


import_expr: Import expr {$$ = mkImportNode(@$, $2);}


ident: Ident {$$ = (Node*)lextxt;}
     ;

usertype: UserType {$$ = (Node*)lextxt;}
        ;

typevar: TypeVar {$$ = (Node*)lextxt;}
       ;

intlit: IntLit {$$ = mkIntLitNode(@$, lextxt);}
      ;

fltlit: FltLit {$$ = mkFltLitNode(@$, lextxt);}
      ;

strlit: StrLit {$$ = mkStrLitNode(@$, lextxt);}
      ;

charlit: CharLit {$$ = mkCharLitNode(@$, lextxt);}
      ;

lit_type: I8        {$$ = mkTypeNode(@$, TT_I8,  (char*)"");}
        | I16       {$$ = mkTypeNode(@$, TT_I16, (char*)"");}
        | I32       {$$ = mkTypeNode(@$, TT_I32, (char*)"");}
        | I64       {$$ = mkTypeNode(@$, TT_I64, (char*)"");}
        | U8        {$$ = mkTypeNode(@$, TT_U8,  (char*)"");}
        | U16       {$$ = mkTypeNode(@$, TT_U16, (char*)"");}
        | U32       {$$ = mkTypeNode(@$, TT_U32, (char*)"");}
        | U64       {$$ = mkTypeNode(@$, TT_U64, (char*)"");}
        | Isz       {$$ = mkTypeNode(@$, TT_Isz, (char*)"");}
        | Usz       {$$ = mkTypeNode(@$, TT_Usz, (char*)"");}
        | F16       {$$ = mkTypeNode(@$, TT_F16, (char*)"");}
        | F32       {$$ = mkTypeNode(@$, TT_F32, (char*)"");}
        | F64       {$$ = mkTypeNode(@$, TT_F64, (char*)"");}
        | C8        {$$ = mkTypeNode(@$, TT_C8,  (char*)"");}
        | C32       {$$ = mkTypeNode(@$, TT_C32, (char*)"");}
        | Bool      {$$ = mkTypeNode(@$, TT_Bool, (char*)"");}
        | Void      {$$ = mkTypeNode(@$, TT_Void, (char*)"");}
        | usertype  {$$ = mkTypeNode(@$, TT_Data, (char*)$1);}
        | typevar   {$$ = mkTypeNode(@$, TT_TypeVar, (char*)$1);}
        ;

type: type '*'              %prec HIGH {$$ = mkTypeNode(@$, TT_Ptr,  (char*)"", $1);}
    | '[' type_expr ']'     {$$ = mkTypeNode(@$, TT_Array,(char*)"", $2);}
    | type '>' type         {setNext($3, $1); $$ = mkTypeNode(@$, TT_Function, (char*)"", $3);}  /* f-ptr w/ params*/
    | '(' ')' RArrow type   {$$ = mkTypeNode(@$, TT_Function, (char*)"", $4);}  /* f-ptr w/out params*/
    | '(' type_expr ')'     {$$ = $2;}
    | lit_type              {$$ = $1;}
    ;

type_expr_: type_expr_ ',' type %prec LOW {$$ = setNext($1, $3);}
          | type                %prec LOW  {$$ = setRoot($1);}
          ;

type_expr: type_expr_  %prec LOW {Node* tmp = getRoot(); 
                        if(tmp == $1){//singular type, first type in list equals the last
                            $$ = tmp;
                        }else{ //tuple type
                            $$ = mkTypeNode(@$, TT_Tuple, (char*)"", tmp);
                        }
                       }


modifier: Pub      {$$ = mkModNode(@$, Tok_Pub);} 
        | Pri      {$$ = mkModNode(@$, Tok_Pri);}
        | Pro      {$$ = mkModNode(@$, Tok_Pro);}
        | Raw      {$$ = mkModNode(@$, Tok_Raw);}
        | Const    {$$ = mkModNode(@$, Tok_Const);}
        | Noinit   {$$ = mkModNode(@$, Tok_Noinit);}
        | preproc  {$$ = $1;}
        ;

modifier_list_: modifier_list_ modifier {$$ = setNext($1, $2);}
              | modifier {$$ = setRoot($1);}
              ;

modifier_list: modifier_list_ {$$ = getRoot();}
             ;


var_decl: modifier_list Var ident '=' expr  {$$ = mkVarDeclNode(@3, (char*)$3, $1,  0, $5);}
        | Var ident '=' expr                {$$ = mkVarDeclNode(@2, (char*)$2,  0,  0, $4);}
        ;

let_binding: Let modifier_list type_expr ident '=' expr {$$ = mkLetBindingNode(@$, (char*)$4, $2, $3, $6);}
           | Let modifier_list ident '=' expr           {$$ = mkLetBindingNode(@$, (char*)$3, $2, 0,  $5);}
           | Let type_expr ident '=' expr               {$$ = mkLetBindingNode(@$, (char*)$3, 0,  $2, $5);}
           | Let ident '=' expr                         {$$ = mkLetBindingNode(@$, (char*)$2, 0,  0,  $4);}
           ;


trait_decl: Trait usertype Indent trait_fn_list Unindent  {$$ = mkTraitNode(@$, (char*)$2, $4);}
          ;

trait_fn_list: _trait_fn_list maybe_newline {$$ = getRoot();}

_trait_fn_list: _trait_fn_list Newline trait_fn  {$$ = setNext($1, $3);}
              | trait_fn                         {$$ = setRoot($1);}
              ;


trait_fn: modifier_list Fun fn_name ':' params RArrow type_expr   {$$ = mkFuncDeclNode(@$, /*fn_name*/$3, $3, /*mods*/$1, /*ret_ty*/$7,                                  /*params*/$5, /*body*/0);}
        | modifier_list Fun fn_name ':' RArrow type_expr          {$$ = mkFuncDeclNode(@$, /*fn_name*/$3, $3, /*mods*/$1, /*ret_ty*/$6,                                  /*params*/0,  /*body*/0);}
        | modifier_list Fun fn_name ':' params                    {$$ = mkFuncDeclNode(@$, /*fn_name*/$3, $3, /*mods*/$1, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$5, /*body*/0);}
        | modifier_list Fun fn_name ':'                           {$$ = mkFuncDeclNode(@$, /*fn_name*/$3, $3, /*mods*/$1, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/0);}
        | Fun fn_name ':' params RArrow type_expr                 {$$ = mkFuncDeclNode(@$, /*fn_name*/$2, $2, /*mods*/ 0, /*ret_ty*/$6,                                  /*params*/$4, /*body*/0);}
        | Fun fn_name ':' RArrow type_expr                        {$$ = mkFuncDeclNode(@$, /*fn_name*/$2, $2, /*mods*/ 0, /*ret_ty*/$5,                                  /*params*/0,  /*body*/0);}
        | Fun fn_name ':' params                                  {$$ = mkFuncDeclNode(@$, /*fn_name*/$2, $2, /*mods*/ 0, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$4, /*body*/0);}
        | Fun fn_name ':'                                         {$$ = mkFuncDeclNode(@$, /*fn_name*/$2, $2, /*mods*/ 0, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/0);}
        ;


data_decl: modifier_list Type usertype '=' type_decl_block         {$$ = mkDataDeclNode(@$, (char*)$3, $5);}
         | Type usertype '=' type_decl_block                       {$$ = mkDataDeclNode(@$, (char*)$2, $4);}
         ;

type_expr_list: type_expr_list type_expr  {$$ = setNext($1, $2);}
              | type_expr                 {$$ = setRoot($1);}
              ;

type_decl: params          {$$ = $1;}
       /*  | '|' usertype type_expr_list  {$$ = mkNamedValNode(@$, mkVarNode(@2, (char*)$2), mkTypeNode(@$, TT_TaggedUnion, (char*)"", getRoot()));}
         | '|' usertype                 {$$ = mkNamedValNode(@$, mkVarNode(@2, (char*)$2), mkTypeNode(@$, TT_TaggedUnion, (char*)"", 0));}
       */  ;

type_decl_list: type_decl_list Newline type_decl           {$$ = setNext($1, $3);}
              | type_decl_list Newline tagged_union_list   {setNext($1, getRoot()); $$ = $3;}
              | type_decl                                  {$$ = setRoot($1);}
              | tagged_union_list                          {$$ = $1;}
              ;

type_decl_block: Indent type_decl_list Unindent  {$$ = getRoot();}
               | params               %prec LOW  {$$ = $1;}
               | type_expr            %prec LOW  {$$ = mkNamedValNode(@$, mkVarNode(@$, (char*)""), $1);}
               | tagged_union_list    %prec LOW  {$$ = getRoot();}
               ;

tagged_union_list: tagged_union_list '|' usertype type_expr_list  %prec LOW  {$$ = setNext($1, mkNamedValNode(@$, mkVarNode(@3, (char*)$3), mkTypeNode(@$, TT_TaggedUnion, (char*)"", getRoot())));}
                 | tagged_union_list '|' usertype                 %prec LOW  {$$ = setNext($1, mkNamedValNode(@$, mkVarNode(@3, (char*)$3), mkTypeNode(@$, TT_TaggedUnion, (char*)"", 0)));}
                 | '|' usertype type_expr_list                    %prec LOW  {$$ = setRoot(mkNamedValNode(@$, mkVarNode(@2, (char*)$2), mkTypeNode(@$, TT_TaggedUnion, (char*)"", getRoot())));}
                 | '|' usertype                                   %prec LOW  {$$ = setRoot(mkNamedValNode(@$, mkVarNode(@2, (char*)$2), mkTypeNode(@$, TT_TaggedUnion, (char*)"", 0)));}



block: Indent expr Unindent  {$$ = mkBlockNode(@$, $2);}
     ;



raw_ident_list: raw_ident_list ident  {$$ = setNext($1, mkVarNode(@$, (char*)$2));}
              | ident                 {$$ = setRoot(mkVarNode(@$, (char*)$1));}
              ;

ident_list: raw_ident_list  %prec MED {$$ = getRoot();}


/* 
 * In case of multiple parameters declared with a single type, eg i32 a b c
 * The next parameter should be set to the first in the list, (the one returned by getRoot()),
 * but the variable returned must be the last in the last, in this case $4
 */


_params: _params ',' type_expr ident_list {$$ = setNext($1, mkNamedValNode(@$, $4, $3));}
      | type_expr ident_list              {$$ = mkNamedValNode(@$, $2, $1, true);}
      ;

                          /* varargs function .. (Range) followed by . */
params: _params ',' Range '.' {setNext($1, mkNamedValNode(@$, mkVarNode(@$, (char*)""), 0)); $$ = getRoot();}
      | _params               %prec LOW {$$ = getRoot();}
      ;

function: fn_def
        | fn_decl
        | fn_inferredRet
        | fn_lambda
        | fn_ext_def
        | fn_ext_inferredRet
        ;

fn_name: ident       /* most functions */      {$$ = $1;}
       | type_expr   /* cast function */       {$$ = (Node*)typeNodeToStr((TypeNode*)$1).c_str();}
       | '(' op ')'  /* operator overloads */  {$$ = $2;}
       ;

op: '+'    {$$ = (Node*)"+";} 
  | '-'    {$$ = (Node*)"-";} 
  | '*'    {$$ = (Node*)"*";}
  | '/'    {$$ = (Node*)"/";}
  | '%'    {$$ = (Node*)"%";}
  | '<'    {$$ = (Node*)"<";}
  | '>'    {$$ = (Node*)">";}
  | '.'    {$$ = (Node*)".";}
  | ';'    {$$ = (Node*)";";}
  | '#'    {$$ = (Node*)"#";}
  | Eq     {$$ = (Node*)"==";}
  | NotEq  {$$ = (Node*)"!=";}
  | GrtrEq {$$ = (Node*)">=";}
  | LesrEq {$$ = (Node*)"<=";}
  | Or     {$$ = (Node*)"or";}
  | And    {$$ = (Node*)"and";}
  | '='    {$$ = (Node*)"=";}
  | AddEq  {$$ = (Node*)"+=";}
  | SubEq  {$$ = (Node*)"-=";}
  | MulEq  {$$ = (Node*)"*=";}
  | DivEq  {$$ = (Node*)"/=";}
  | ApplyR {$$ = (Node*)"|>";}
  | ApplyL {$$ = (Node*)"<|";}
  | Append {$$ = (Node*)"++";}
  | Range  {$$ = (Node*)"..";}
  | In     {$$ = (Node*)"in";}
  ;

fn_ext_def: modifier_list maybe_newline Fun type_expr '.' fn_name ':' params RArrow type_expr block  {$$ = mkExtNode(@$, $4, mkFuncDeclNode(@$, /*fn_name*/mangle_fn($6, $8), $6, /*mods*/$1, /*ret_ty*/$10,                                 /*params*/$8, /*body*/$11));}
          | modifier_list maybe_newline Fun type_expr '.' fn_name ':' RArrow type_expr block         {$$ = mkExtNode(@$, $4, mkFuncDeclNode(@$, /*fn_name*/$6,                $6, /*mods*/$1, /*ret_ty*/$9,                                  /*params*/0,  /*body*/$10));}
          | modifier_list maybe_newline Fun type_expr '.' fn_name ':' params block                   {$$ = mkExtNode(@$, $4, mkFuncDeclNode(@$, /*fn_name*/mangle_fn($6, $8), $6, /*mods*/$1, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$8, /*body*/$9));}
          | modifier_list maybe_newline Fun type_expr '.' fn_name ':' block                          {$$ = mkExtNode(@$, $4, mkFuncDeclNode(@$, /*fn_name*/$6,                $6, /*mods*/$1, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/$8));}
          | Fun type_expr '.' fn_name ':' params RArrow type_expr block                              {$$ = mkExtNode(@$, $2, mkFuncDeclNode(@$, /*fn_name*/mangle_fn($4, $6), $4, /*mods*/ 0, /*ret_ty*/$8,                                  /*params*/$6, /*body*/$9));}
          | Fun type_expr '.' fn_name ':' RArrow type_expr block                                     {$$ = mkExtNode(@$, $2, mkFuncDeclNode(@$, /*fn_name*/$4,                $4, /*mods*/ 0, /*ret_ty*/$7,                                  /*params*/0,  /*body*/$8));}
          | Fun type_expr '.' fn_name ':' params block                                               {$$ = mkExtNode(@$, $2, mkFuncDeclNode(@$, /*fn_name*/mangle_fn($4, $6), $4, /*mods*/ 0, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$6, /*body*/$7));}
          | Fun type_expr '.' fn_name ':' block                                                      {$$ = mkExtNode(@$, $2, mkFuncDeclNode(@$, /*fn_name*/$4,                $4, /*mods*/ 0, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/$6));}
          ;

fn_ext_inferredRet: modifier_list maybe_newline Fun type_expr '.' fn_name ':' params '=' expr   {$$ = mkExtNode(@$, $4, mkFuncDeclNode(@$, /*fn_name*/mangle_fn($6, $8), $6, /*mods*/$1, /*ret_ty*/0, /*params*/$8, /*body*/$10));}
                  | modifier_list maybe_newline Fun type_expr '.' fn_name ':' '=' expr          {$$ = mkExtNode(@$, $4, mkFuncDeclNode(@$, /*fn_name*/$6,                $6, /*mods*/$1, /*ret_ty*/0, /*params*/0,  /*body*/$9));}
                  | Fun type_expr '.' fn_name ':' params '=' expr                               {$$ = mkExtNode(@$, $2, mkFuncDeclNode(@$, /*fn_name*/mangle_fn($4, $6), $4, /*mods*/ 0, /*ret_ty*/0, /*params*/$6, /*body*/$8));}
                  | Fun type_expr '.' fn_name ':' '=' expr                                      {$$ = mkExtNode(@$, $2, mkFuncDeclNode(@$, /*fn_name*/$4,                $4, /*mods*/ 0, /*ret_ty*/0, /*params*/0,  /*body*/$7));}
                  ;

fn_def: modifier_list maybe_newline Fun fn_name ':' params RArrow type_expr block  {$$ = mkFuncDeclNode(@$, /*fn_name*/mangle_fn($4, $6), $4, /*mods*/$1, /*ret_ty*/$8,                                  /*params*/$6, /*body*/$9);}
      | modifier_list maybe_newline Fun fn_name ':' RArrow type_expr block         {$$ = mkFuncDeclNode(@$, /*fn_name*/$4,                $4, /*mods*/$1, /*ret_ty*/$7,                                  /*params*/0,  /*body*/$8);}
      | modifier_list maybe_newline Fun fn_name ':' params block                   {$$ = mkFuncDeclNode(@$, /*fn_name*/mangle_fn($4, $6), $4, /*mods*/$1, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$6, /*body*/$7);}
      | modifier_list maybe_newline Fun fn_name ':' block                          {$$ = mkFuncDeclNode(@$, /*fn_name*/$4,                $4, /*mods*/$1, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/$6);}
      | Fun fn_name ':' params RArrow type_expr block                              {$$ = mkFuncDeclNode(@$, /*fn_name*/mangle_fn($2, $4), $2, /*mods*/ 0, /*ret_ty*/$6,                                  /*params*/$4, /*body*/$7);}
      | Fun fn_name ':' RArrow type_expr block                                     {$$ = mkFuncDeclNode(@$, /*fn_name*/$2,                $2, /*mods*/ 0, /*ret_ty*/$5,                                  /*params*/0,  /*body*/$6);}
      | Fun fn_name ':' params block                                               {$$ = mkFuncDeclNode(@$, /*fn_name*/mangle_fn($2, $4), $2, /*mods*/ 0, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$4, /*body*/$5);}
      | Fun fn_name ':' block                                                      {$$ = mkFuncDeclNode(@$, /*fn_name*/$2,                $2, /*mods*/ 0, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/$4);}
      ;

fn_inferredRet: modifier_list maybe_newline Fun fn_name ':' params '=' expr   {$$ = mkFuncDeclNode(@$, /*fn_name*/mangle_fn($4, $6), $4, /*mods*/$1, /*ret_ty*/0, /*params*/$6, /*body*/$8);}
              | modifier_list maybe_newline Fun fn_name ':' '=' expr          {$$ = mkFuncDeclNode(@$, /*fn_name*/$4,                $4, /*mods*/$1, /*ret_ty*/0, /*params*/0,  /*body*/$7);}
              | Fun fn_name ':' params '=' expr                               {$$ = mkFuncDeclNode(@$, /*fn_name*/mangle_fn($2, $4), $2, /*mods*/ 0, /*ret_ty*/0, /*params*/$4, /*body*/$6);}
              | Fun fn_name ':' '=' expr                                      {$$ = mkFuncDeclNode(@$, /*fn_name*/$2,                $2, /*mods*/ 0, /*ret_ty*/0, /*params*/0,  /*body*/$5);}
              ;

fn_decl: modifier_list maybe_newline Fun fn_name ':' params RArrow type_expr ';'   {$$ = mkFuncDeclNode(@$, /*fn_name*/$4, $4, /*mods*/$1, /*ret_ty*/$8,                                  /*params*/$6, /*body*/0);}
       | modifier_list maybe_newline Fun fn_name ':' RArrow type_expr        ';'   {$$ = mkFuncDeclNode(@$, /*fn_name*/$4, $4, /*mods*/$1, /*ret_ty*/$7,                                  /*params*/0,  /*body*/0);}
       | modifier_list maybe_newline Fun fn_name ':' params                  ';'   {$$ = mkFuncDeclNode(@$, /*fn_name*/$4, $4, /*mods*/$1, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$6, /*body*/0);}
       | modifier_list maybe_newline Fun fn_name ':'                         ';'   {$$ = mkFuncDeclNode(@$, /*fn_name*/$4, $4, /*mods*/$1, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/0);}
       | Fun fn_name ':' params RArrow type_expr                             ';'   {$$ = mkFuncDeclNode(@$, /*fn_name*/$2, $2, /*mods*/ 0, /*ret_ty*/$6,                                  /*params*/$4, /*body*/0);}
       | Fun fn_name ':' RArrow type_expr                                    ';'   {$$ = mkFuncDeclNode(@$, /*fn_name*/$2, $2, /*mods*/ 0, /*ret_ty*/$5,                                  /*params*/0,  /*body*/0);}
       | Fun fn_name ':' params                                              ';'   {$$ = mkFuncDeclNode(@$, /*fn_name*/$2, $2, /*mods*/ 0, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$4, /*body*/0);}
       | Fun fn_name ':'                                                     ';'   {$$ = mkFuncDeclNode(@$, /*fn_name*/$2, $2, /*mods*/ 0, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/0);}
       ;

fn_lambda: modifier_list maybe_newline Fun params '=' expr  %prec Fun  {$$ = mkFuncDeclNode(@$, /*fn_name*/(Node*)"", (Node*)"", /*mods*/$1, /*ret_ty*/0,  /*params*/$4, /*body*/$6);}
         | modifier_list maybe_newline Fun '=' expr         %prec Fun  {$$ = mkFuncDeclNode(@$, /*fn_name*/(Node*)"", (Node*)"", /*mods*/$1, /*ret_ty*/0,  /*params*/0,  /*body*/$5);}
         | Fun params '=' expr                              %prec Fun  {$$ = mkFuncDeclNode(@$, /*fn_name*/(Node*)"", (Node*)"", /*mods*/ 0, /*ret_ty*/0,  /*params*/$2, /*body*/$4);}
         | Fun '=' expr                                     %prec Fun  {$$ = mkFuncDeclNode(@$, /*fn_name*/(Node*)"", (Node*)"", /*mods*/ 0, /*ret_ty*/0,  /*params*/0,  /*body*/$3);}
         ;



ret_expr: Return expr {$$ = mkRetNode(@$, $2);}
        ;


extension: Ext type_expr Indent fn_list Unindent {$$ = mkExtNode(@$, $2, $4);}

         /* TODO: add traits field to ExtNode to store this usertype_list of traits */
         | Ext type_expr ':' usertype_list Indent fn_list Unindent {$$ = mkExtNode(@$, $2, $6);}
         ;
 
usertype_list: usertype_list_  {$$ = getRoot();}

usertype_list_: usertype_list_ ',' usertype {$$ = setNext($1, $3);}
              | usertype                    {$$ = setRoot($1);}
              ;


fn_list: fn_list_ {$$ = getRoot();}

fn_list_: fn_list_ function maybe_newline  {$$ = setNext($1, $2);} 
        | function maybe_newline           {$$ = setRoot($1);}
        ;
/*
*/
/* 
 *  Two stage rule needed (if_pre and if_expr) to properly handle all
 *  Instances of Newline-sequencing with if-exprs, particularly when the
 *  else clause is not present, the LOW precedence needed to 'absorb' the final
 *  Newline from the Then (if an expr-block was found) would also incorrectly cause
 *  all following expressions to be within the If's body.  This is possible for all
 *  statement-like expressions that terminate in an expr and have a LOW precedence.
 */
/*
if_expr: if_pre Else expr                      {((IfNode*)$1)->elseN.reset($3); $$ = $1;}
       | if_pre Newline Else expr  %prec Else  {((IfNode*)$1)->elseN.reset($4); $$ = $1;}
       | if_pre                    %prec LOW   {$$ = $1;}
       ;
*/

if_expr: If expr Then expr                     %prec If {$$ = setRoot(mkIfNode(@$, $2, $4, 0));}
       | if_expr Elif expr Then expr           %prec If {auto*elif = mkIfNode(@$, $3, $5, 0); setElse($1, elif); $$ = elif;}
       | if_expr Else expr                     %prec If {$$ = setElse($1, $3);}
       | if_expr Newline Elif expr Then expr   %prec If {auto*elif = mkIfNode(@$, $4, $6, 0); setElse($1, elif); $$ = elif;}
       | if_expr Newline Else expr             %prec If {$$ = setElse($1, $4);}
       ;


while_loop: While expr Do expr  %prec While  {$$ = mkWhileNode(@$, $2, $4);}
          ;

/*            vvvvv this will be later changed to pattern  */
for_loop: For ident In expr Do expr  %prec For  {$$ = mkForNode(@$, $2, $4, $6);}


match: '|' expr RArrow expr {$$ = mkMatchBranchNode(@$, $2, $4);}
     ;


match_expr: Match expr With Newline match  {$$ = mkMatchNode(@$, $2, $5);}
          | match_expr Newline match       {$$ = addMatch($1, $3);}
          ;



var: ident  %prec Ident {$$ = mkVarNode(@$, (char*)$1);}
   ;


val: '(' expr ')'            {$$ = $2;}
   | tuple                   {$$ = $1;}
   | array                   {$$ = $1;}
   | unary_op                {$$ = $1;}
   | var                     {$$ = $1;}
   | intlit                  {$$ = $1;}
   | fltlit                  {$$ = $1;}
   | strlit                  {$$ = $1;}
   | charlit                 {$$ = $1;}
   | True                    {$$ = mkBoolLitNode(@$, 1);}
   | False                   {$$ = mkBoolLitNode(@$, 0);}
   | let_binding             {$$ = $1;}
   | var_decl                {$$ = $1;}
   | while_loop              {$$ = $1;}
   | for_loop                {$$ = $1;}
   | if_expr       %prec LOW {$$ = getRoot();}
   | function                {$$ = $1;}
   | data_decl               {$$ = $1;}
   | extension               {$$ = $1;}
   | trait_decl              {$$ = $1;}
   | ret_expr                {$$ = $1;}
   | import_expr             {$$ = $1;}
   | match_expr    %prec LOW {$$ = $1;}
   | block                   {$$ = $1;}
   | type_expr      %prec LOW
   ;

tuple: '(' expr_list ')' {$$ = mkTupleNode(@$, $2);}
     | '(' ')'           {$$ = mkTupleNode(@$, 0);}
     ;

array: '[' expr_list ']' {$$ = mkArrayNode(@$, $2);}
     | '[' ']'           {$$ = mkArrayNode(@$, 0);}
     ;


unary_op: '@' expr                    {$$ = mkUnOpNode(@$, '@', $2);}
        | '&' expr                    {$$ = mkUnOpNode(@$, '&', $2);}
        | New expr                    {$$ = mkUnOpNode(@$, Tok_New, $2);}
        | Not expr                    {$$ = mkUnOpNode(@$, Tok_Not, $2);}
        | type_expr expr  %prec TYPE  {$$ = mkTypeCastNode(@$, $1, $2);}
        ;

preproc: '!' '[' expr ']'  {$$ = mkPreProcNode(@$, $3);}
       ;

arg_list: arg_list_p  %prec FUNC {$$ = mkTupleNode(@$, getRoot());}
        ;

arg_list_p: arg_list_p arg  %prec FUNC {$$ = setNext($1, $2);}
          | arg             %prec FUNC {$$ = setRoot($1);}
          ;

arg: val
   | arg '.' var        {$$ = mkBinOpNode(@$, '.', $1, $3);}
   | type_expr '.' var  {$$ = mkBinOpNode(@$, '.', $1, $3);}
   ;

/* expr is used in expression blocks and can span multiple lines */
expr_list: expr_list_p {$$ = getRoot();}
         ;


expr_list_p: expr_list_p ',' maybe_newline expr  %prec ',' {$$ = setNext($1, $4);}
           | expr                                %prec LOW {$$ = setRoot($1);}
           ;

expr: expr '+' maybe_newline expr                {$$ = mkBinOpNode(@$, '+', $1, $4);}
    | expr '-' expr                              {$$ = mkBinOpNode(@$, '-', $1, $3);}
    | '-' expr                                   {$$ = mkUnOpNode(@$, '-', $2);}
    | expr '-' Newline expr                      {$$ = mkBinOpNode(@$, '-', $1, $4);}
    | expr '*' maybe_newline expr                {$$ = mkBinOpNode(@$, '*', $1, $4);}
    | expr '/' maybe_newline expr                {$$ = mkBinOpNode(@$, '/', $1, $4);}
    | expr '%' maybe_newline expr                {$$ = mkBinOpNode(@$, '%', $1, $4);}
    | expr '<' maybe_newline expr                {$$ = mkBinOpNode(@$, '<', $1, $4);}
    | expr '>' maybe_newline expr                {$$ = mkBinOpNode(@$, '>', $1, $4);}
    | expr '.' maybe_newline var                 {$$ = mkBinOpNode(@$, '.', $1, $4);}
    | type_expr '.' maybe_newline var            {$$ = mkBinOpNode(@$, '.', $1, $4);}
    | expr ';' maybe_newline expr                {$$ = mkBinOpNode(@$, ';', $1, $4);}
    | expr '#' maybe_newline expr                {$$ = mkBinOpNode(@$, '#', $1, $4);}
    | expr Eq maybe_newline expr                 {$$ = mkBinOpNode(@$, Tok_Eq, $1, $4);}
    | expr NotEq maybe_newline expr              {$$ = mkBinOpNode(@$, Tok_NotEq, $1, $4);}
    | expr GrtrEq maybe_newline expr             {$$ = mkBinOpNode(@$, Tok_GrtrEq, $1, $4);}
    | expr LesrEq maybe_newline expr             {$$ = mkBinOpNode(@$, Tok_LesrEq, $1, $4);}
    | expr Or maybe_newline expr                 {$$ = mkBinOpNode(@$, Tok_Or, $1, $4);}
    | expr And maybe_newline expr                {$$ = mkBinOpNode(@$, Tok_And, $1, $4);}
    | expr '=' maybe_newline expr                {$$ = mkVarAssignNode(@$, $1, $4);} /* All VarAssignNodes return void values */
    | expr AddEq maybe_newline expr              {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '+', $1, $4), false);}
    | expr SubEq maybe_newline expr              {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '-', $1, $4), false);}
    | expr MulEq maybe_newline expr              {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '*', $1, $4), false);}
    | expr DivEq maybe_newline expr              {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '/', $1, $4), false);}
    | expr ApplyR maybe_newline expr             {$$ = mkBinOpNode(@$, '(', $4, $1);}
    | expr ApplyL maybe_newline expr             {$$ = mkBinOpNode(@$, '(', $1, $4);}
    | expr Append maybe_newline expr             {$$ = mkBinOpNode(@$, Tok_Append, $1, $4);}
    | expr Range maybe_newline expr              {$$ = mkBinOpNode(@$, Tok_Range, $1, $4);}
    | expr In maybe_newline expr                 {$$ = mkBinOpNode(@$, Tok_In, $1, $4);}
    | expr arg_list                              {$$ = mkBinOpNode(@$, '(', $1, $2);}
    | val                             %prec MED  {$$ = $1;}

    /* 
        if_expr prec must be low to absorb the newlines before Else / Elif tokens, so 
       This rule is needed to properly sequence them
    */
    | if_expr Newline expr          %prec If     {$$ = mkBinOpNode(@$, ';', getRoot(), $3);}
    | if_expr Newline               %prec LOW    {$$ = getRoot();}
    | match_expr Newline expr       %prec Match  {$$ = mkBinOpNode(@$, ';', $1, $3);}
    | match_expr Newline            %prec LOW    {$$ = $1;}
    | expr Newline                               {$$ = $1;}
    | expr Newline expr                          {$$ = mkBinOpNode(@$, ';', $1, $3);}
    ;

%%

/* location parser error */
void yy::parser::error(const location& loc, const string& msg){
    location l = loc;
    ante::error(msg.c_str(), l);
} 


extern string mangle(std::string &base, TypeNode *paramTys);
TypeNode* createFnTyNode(NamedValNode *params, TypeNode *retTy);
TypeNode* mkAnonTypeNode(TypeTag t);
extern char* strcpy(char* dest, const char* src);

Node* mangle_fn(Node *basename, Node *nvns_){
    string base = (char*)basename;

    auto *nvn = (NamedValNode*)nvns_;

    auto *fakeRetTy = mkAnonTypeNode(TT_Void);
    auto *fnTy = createFnTyNode(nvn, fakeRetTy);

    string name =  mangle(base, (TypeNode*)fnTy->extTy->next.get());
 
    char* ret = (char*)malloc(name.length());
    strcpy(ret, name.c_str());

    return (Node*)ret;
}
