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
#define YYERROR_VERBOSE 1

#include "yyparser.h"
#include <cstring>
using namespace std;
using namespace ante;
using namespace ante::parser;

/* Defined in lexer.cpp */
extern int yylex(yy::parser::semantic_type*, yy::location*);

namespace ante {
    extern string typeNodeToStr(const TypeNode*);
    extern string mangle(std::string &base, NamedValNode *paramTys);

    namespace parser { struct TypeNode; }
}

Node* externCName(Node *n);
vector<unique_ptr<TypeNode>> toOwnedVec(Node *tn);
vector<unique_ptr<TypeNode>> concat(vector<unique_ptr<TypeNode>>&& l, Node *tn);

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
%token Type Trait Fun Ext Block Self

/* modifiers */
%token Pub Pri Pro Raw
%token Const Noinit Mut Global

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
%nonassoc MEDLOW


%left Newline
%left STMT Fun Let Import Return Ext Var While For Match Trait If Break Continue Type
%left RArrow

%left ENDIF
%left Else Elif

/* fake symbol for intermediate if expresstions
 *   Else expressions must have lower priority than if/elif
 *   epressions and If must have same priority as STMT to be
 *   sequenced properly, necessitating the creation of MEDIF
 *   for if/elif expressions
 */
%left MEDIF

%nonassoc Indent Unindent

%left MED

%left ','
%left '=' AddEq SubEq MulEq DivEq
%left ';'
%left MODIFIER Pub Pri Pro Raw Const Noinit Mut Global

%right ApplyL
%left ApplyR

%nonassoc '!'

%left Or
%left And
%left Not
%left Eq  NotEq GrtrEq LesrEq '<' '>'

%left In
%left Append
%left Range

%left ':'

%left '+' '-'
%left '*' '/' '%'

%left '#'
%left '@' New
%left '&' TYPE UserType TypeVar I8 I16 I32 I64 U8 U16 U32 U64 Isz Usz F16 F32 F64 C8 C32 Bool Void
%nonassoc FUNC
%left Block

%nonassoc LITERALS StrLit IntLit FltLit CharLit True False Ident Self
%left '.'


/*
    Being below HIGH, this ensures parenthetical expressions will be parsed
    as just order-of operations parenthesis, instead of a single-value tuple.
*/
%nonassoc ')' ']' '}'

%nonassoc '(' '['
%nonassoc HIGH
%nonassoc '{'

%expect 0
%start begin
%%

begin: maybe_newline top_level_expr

top_level_expr: top_level_expr expr_no_decl  %prec Newline {$$ = append_main($2);}
              | top_level_expr function                    {$$ = append_fn($2);}
              | top_level_expr data_decl                   {$$ = append_type($2);}
              | top_level_expr extension                   {$$ = append_extension($2);}
              | top_level_expr trait_decl                  {$$ = append_trait($2);}
              | top_level_expr import_expr                 {$$ = append_import($2);}
              | top_level_expr Newline
              | expr_no_decl                 %prec Newline {createRoot($1->loc); $$ = append_main($1);}
              | function                                   {createRoot($1->loc); $$ = append_fn($1);}
              | data_decl                                  {createRoot($1->loc); $$ = append_type($1);}
              | extension                                  {createRoot($1->loc); $$ = append_extension($1);}
              | trait_decl                                 {createRoot($1->loc); $$ = append_trait($1);}
              | import_expr                                {createRoot($1->loc); $$ = append_import($1);}

              | top_level_expr Elif bound_expr Then expr_no_decl_or_jump    %prec MEDIF {auto*elif = mkIfNode(@$, $3, $5, 0); $$ = setElse($1, elif);}
              | top_level_expr Else expr_no_decl_or_jump                      %prec Else  {$$ = setElse($1, $3);}
              ;

maybe_newline: Newline  %prec Newline
             |
             ;


import_expr: Import expr {$$ = mkImportNode(@$, $2);}


ident: Ident {$$ = (Node*)lextxt;}
     | Self  {$$ = (Node*)strdup("self");}
     ;

usertype: UserType {$$ = (Node*)lextxt;}
        ;

typevar: TypeVar {$$ = (Node*)lextxt;}
       ;

intlit: IntLit {$$ = mkIntLitNode(@$, lextxt); free(lextxt);}
      ;

fltlit: FltLit {$$ = mkFltLitNode(@$, lextxt); free(lextxt);}
      ;

strlit: StrLit {$$ = mkStrLitNode(@$, lextxt); free(lextxt);}
      ;

charlit: CharLit {$$ = mkCharLitNode(@$, lextxt); free(lextxt);}
      ;

lit_type: I8                  {$$ = mkTypeNode(@$, TT_I8,  (char*)"");}
        | I16                 {$$ = mkTypeNode(@$, TT_I16, (char*)"");}
        | I32                 {$$ = mkTypeNode(@$, TT_I32, (char*)"");}
        | I64                 {$$ = mkTypeNode(@$, TT_I64, (char*)"");}
        | U8                  {$$ = mkTypeNode(@$, TT_U8,  (char*)"");}
        | U16                 {$$ = mkTypeNode(@$, TT_U16, (char*)"");}
        | U32                 {$$ = mkTypeNode(@$, TT_U32, (char*)"");}
        | U64                 {$$ = mkTypeNode(@$, TT_U64, (char*)"");}
        | Isz                 {$$ = mkTypeNode(@$, TT_Isz, (char*)"");}
        | Usz                 {$$ = mkTypeNode(@$, TT_Usz, (char*)"");}
        | F16                 {$$ = mkTypeNode(@$, TT_F16, (char*)"");}
        | F32                 {$$ = mkTypeNode(@$, TT_F32, (char*)"");}
        | F64                 {$$ = mkTypeNode(@$, TT_F64, (char*)"");}
        | C8                  {$$ = mkTypeNode(@$, TT_C8,  (char*)"");}
        | C32                 {$$ = mkTypeNode(@$, TT_C32, (char*)"");}
        | Bool                {$$ = mkTypeNode(@$, TT_Bool, (char*)"");}
        | Void                {$$ = mkTypeNode(@$, TT_Void, (char*)"");}
        | usertype  %prec LOW {$$ = mkTypeNode(@$, TT_Data, (char*)$1); free($1);}
        | typevar             {$$ = mkTypeNode(@$, TT_TypeVar, (char*)$1); free($1);}
        ;

pointer_type: pointer_type '*'  {$$ = mkTypeNode(@$, TT_Ptr, (char*)"", $1);}
            | type '*'          {$$ = mkTypeNode(@$, TT_Ptr, (char*)"", $1);}
            ;

fn_type: '(' ')'       RArrow type  {$$ = mkTypeNode(@$, TT_Function, (char*)"", $4);}
       | tuple_type    RArrow type  {setNext($3, $1); $$ = mkTypeNode(@$, TT_Function, (char*)"", $3);}
       | lit_type      RArrow type  {setNext($3, $1); $$ = mkTypeNode(@$, TT_Function, (char*)"", $3);}
       | pointer_type  RArrow type  {setNext($3, $1); $$ = mkTypeNode(@$, TT_Function, (char*)"", $3);}
       | arr_type      RArrow type  {setNext($3, $1); $$ = mkTypeNode(@$, TT_Function, (char*)"", $3);}
       ;

/* val is used here instead of intlit due to parse conflicts, but only intlit is allowed */
arr_type: '[' val type_expr ']' {$3->next.reset($2);
                                 $$ = mkTypeNode(@$, TT_Array, (char*)"", $3);}
        | '[' type_expr ']'     {$2->next.reset(mkIntLitNode(@$, (char*)"0"));
                                 $$ = mkTypeNode(@$, TT_Array, (char*)"", $2);}
        ;

tuple_type: '(' type_expr ')'  {$$ = $2;}
          ;

generic_type: type type           %prec STMT    {$$ = $1; ((TypeNode*)$1)->params.push_back(unique_ptr<TypeNode>((TypeNode*)$2));}
            | generic_type type   %prec STMT    {$$ = $1; ((TypeNode*)$1)->params.push_back(unique_ptr<TypeNode>((TypeNode*)$2));}
            ;

type: non_generic_type %prec STMT  {$$ = $1;}
    | generic_type     %prec STMT  {$$ = $1;}
    ;

non_generic_type: pointer_type  %prec STMT  {$$ = $1;}
                | arr_type      %prec STMT  {$$ = $1;}
                | fn_type       %prec STMT  {$$ = $1;}
                | lit_type      %prec STMT  {$$ = $1;}
                | tuple_type    %prec STMT  {$$ = $1;}
                ;

type_expr_: type_expr_ ',' type  %prec MED {$$ = setNext($1, $3);}
          | type                 %prec MED {$$ = setRoot($1);}
          ;

type_expr__: type_expr_  %prec MED {Node* tmp = getRoot();
                          if(tmp == $1){//singular type, first type in list equals the last
                              $$ = tmp;
                          }else{ //tuple type
                              $$ = mkTypeNode(@$, TT_Tuple, (char*)"", tmp);
                          }
                         }

type_expr: modifier_list type_expr__  {$$ = ((TypeNode*)$2)->addModifiers((ModNode*)$1);}
         | type_expr__                {$$ = $1;}


modifier: Pub      {$$ = mkModNode(@$, Tok_Pub);}
        | Pri      {$$ = mkModNode(@$, Tok_Pri);}
        | Pro      {$$ = mkModNode(@$, Tok_Pro);}
        | Raw      {$$ = mkModNode(@$, Tok_Raw);}
        | Const    {$$ = mkModNode(@$, Tok_Const);}
        | Noinit   {$$ = mkModNode(@$, Tok_Noinit);}
        | Mut      {$$ = mkModNode(@$, Tok_Mut);}
        | Global   {$$ = mkModNode(@$, Tok_Global);}
        | preproc  {$$ = $1;}
        ;

modifier_list_: modifier_list_ modifier {$$ = setNext($1, $2);}
              | modifier {$$ = setRoot($1);}
              ;

modifier_list: modifier_list_ {$$ = getRoot();}
             ;


/*
modifier_block: modifier_list Indent mod_decl_block Unindent {$$ = applyMods($1, getRoot());}
              ;

mod_decl: function
        | data_decl
        | extension
        | trait_decl
        | modifier_block
        | let_binding
        | var_decl
        ;

mod_decls_block: mod_decls_block mod_decl  {$$ = setNext($1, $2);}
               | mod_decl                  {$$ = setRoot($1);}
               ;
*/

var_decl: modifier_list Var ident '=' expr  {$$ = mkVarDeclNode(@3, (char*)$3, $1,  0, $5); free($3);}
        | Var ident '=' expr                {$$ = mkVarDeclNode(@2, (char*)$2,  0,  0, $4); free($2);}
        ;

global: Global ident_list  {$$ = mkGlobalNode(@$, $2);}
      ;

let_binding: Let modifier_list ident '=' expr           {$$ = mkLetBindingNode(@$, (char*)$3, $2, 0,  $5); free($3);}
           | Let type_expr ident '=' expr               {$$ = mkLetBindingNode(@$, (char*)$3, 0,  $2, $5); free($3);}
           | Let ident '=' expr                         {$$ = mkLetBindingNode(@$, (char*)$2, 0,  0,  $4); free($2);}
           ;


trait_decl: Trait usertype Indent trait_fn_list Unindent  {$$ = mkTraitNode(@$, (char*)$2, $4); free($2);}
          ;

trait_fn_list: _trait_fn_list maybe_newline {$$ = getRoot();}

_trait_fn_list: _trait_fn_list Newline trait_fn  {$$ = setNext($1, $3);}
              | trait_fn                         {$$ = setRoot($1);}
              ;


trait_fn: modifier_list Fun fn_name ':' params RArrow type_expr   {$$ = mkFuncDeclNode(@3, /*fn_name*/$3, /*mods*/$1, /*ret_ty*/$7,                                  /*params*/$5, /*body*/0);}
        | modifier_list Fun fn_name ':' RArrow type_expr          {$$ = mkFuncDeclNode(@3, /*fn_name*/$3, /*mods*/$1, /*ret_ty*/$6,                                  /*params*/0,  /*body*/0);}
        | modifier_list Fun fn_name ':' params                    {$$ = mkFuncDeclNode(@3, /*fn_name*/$3, /*mods*/$1, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$5, /*body*/0);}
        | modifier_list Fun fn_name ':'                           {$$ = mkFuncDeclNode(@3, /*fn_name*/$3, /*mods*/$1, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/0);}
        | Fun fn_name ':' params RArrow type_expr                 {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*mods*/ 0, /*ret_ty*/$6,                                  /*params*/$4, /*body*/0);}
        | Fun fn_name ':' RArrow type_expr                        {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*mods*/ 0, /*ret_ty*/$5,                                  /*params*/0,  /*body*/0);}
        | Fun fn_name ':' params                                  {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*mods*/ 0, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$4, /*body*/0);}
        | Fun fn_name ':'                                         {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*mods*/ 0, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/0);}
        ;


generic_list: generic_list type  %prec LOW  {$$ = setNext($1, $2);}
            | type               %prec LOW  {$$ = setRoot($1);}
            ;

generic_params: generic_list  %prec LOW {$$ = getRoot();}
              ;


data_decl: modifier_list Type usertype generic_params '=' type_decl_block   {$$ = mkDataDeclNode(@$, (char*)$3, $4, $6); free($3);}
         | modifier_list Type usertype '=' type_decl_block                  {$$ = mkDataDeclNode(@$, (char*)$3,  0, $5); free($3);}
         | Type usertype generic_params '=' type_decl_block                 {$$ = mkDataDeclNode(@$, (char*)$2, $3, $5); free($2);}
         | Type usertype '=' type_decl_block                                {$$ = mkDataDeclNode(@$, (char*)$2,  0, $4); free($2);}
         ;

type_decl: params          {$$ = $1;}
       /*  | '|' usertype type_expr_list  {$$ = mkNamedValNode(@$, mkVarNode(@2, (char*)$2), mkTypeNode(@$, TT_TaggedUnion, (char*)"", getRoot()));}
         | '|' usertype                 {$$ = mkNamedValNode(@$, mkVarNode(@2, (char*)$2), mkTypeNode(@$, TT_TaggedUnion, (char*)"", 0));}
       */  ;

type_decl_list: type_decl_list Newline type_decl           {$$ = setNext($1, $3);}
              | type_decl_list Newline tagged_union_list   {$$ = setNext($1, getRoot());}
              | type_decl                                  {$$ = setRoot($1);}
              | tagged_union_list                          {$$ = $1;} /* leave root set */
              ;

type_decl_block: Indent type_decl_list Unindent  {$$ = getRoot();}
               | params               %prec STMT  {$$ = $1;}
               | type_expr            %prec STMT  {$$ = mkNamedValNode(@$, mkVarNode(@$, (char*)""), $1, 0);}
               | tagged_union_list    %prec STMT  {$$ = getRoot();}
               ;

/* this rule returns a list (handled by mkNamedValNode function) */
tagged_union_list: tagged_union_list '|' usertype type_expr   %prec STMT  {$$ = mkNamedValNode(@$, mkVarNode(@3, (char*)$3), mkTypeNode(@$, TT_TaggedUnion, (char*)"", $4), $1); free($3);}
                 | tagged_union_list '|' usertype             %prec STMT  {$$ = mkNamedValNode(@$, mkVarNode(@3, (char*)$3), mkTypeNode(@$, TT_TaggedUnion, (char*)"",  0), $1); free($3);}
                 | '|' usertype type_expr                     %prec STMT  {$$ = mkNamedValNode(@$, mkVarNode(@2, (char*)$2), mkTypeNode(@$, TT_TaggedUnion, (char*)"", $3),  0); free($2);}
                 | '|' usertype                               %prec STMT  {$$ = mkNamedValNode(@$, mkVarNode(@2, (char*)$2), mkTypeNode(@$, TT_TaggedUnion, (char*)"",  0),  0); free($2);}



block: Indent bound_expr Unindent                   {$$ = mkBlockNode(@$, $2);}
     | Indent break Unindent                  {$$ = mkBlockNode(@$, $2);}
     | Indent continue Unindent               {$$ = mkBlockNode(@$, $2);}
     | Indent ret_expr Unindent               {$$ = mkBlockNode(@$, $2);}

     | Indent bound_expr break Unindent             {$$ = mkBlockNode(@$, mkSeqNode(@$, $2, $3));}
     | Indent bound_expr continue Unindent          {$$ = mkBlockNode(@$, mkSeqNode(@$, $2, $3));}
     | Indent bound_expr ret_expr Unindent          {$$ = mkBlockNode(@$, mkSeqNode(@$, $2, $3));}
     ;


explicit_block: Block block  {$$ = $2;}


raw_ident_list: raw_ident_list ident  {$$ = setNext($1, mkVarNode(@2, (char*)$2)); free($2);}
              | ident                 {$$ = setRoot(mkVarNode(@$, (char*)$1)); free($1);}
              ;

ident_list: raw_ident_list  %prec MED {$$ = getRoot();}


/*
 * In case of multiple parameters declared with a single type, eg i32 a b c
 * The next parameter should be set to the first in the list, (the one returned by getRoot()),
 * but the variable returned must be the last in the last, in this case $4
 */


/* NOTE: mkNamedValNode takes care of setNext and setRoot
        for lists automatically in case the shortcut syntax
        is used and multiple NamedValNodes are made */
_params: _params ',' type_expr ident_list {$$ = mkNamedValNode(@$, $4, $3, $1);}
       | type_expr ident_list             {$$ = mkNamedValNode(@$, $2, $1, 0);}
       | Self                             {$$ = mkNamedValNode(@$, mkVarNode(@$, (char*)"self"), (Node*)1, 0);}
       ;

                          /* varargs function .. (Range) followed by . */
params: _params ',' Range '.' {mkNamedValNode(@$, mkVarNode(@$, (char*)""), 0, $1); $$ = getRoot();}
      | _params               %prec LOW {$$ = getRoot();}
      ;

function: fn_def
        | fn_decl
        | fn_inferredRet
        | fn_lambda
        ;

fn_name: ident       /* most functions */      {$$ = $1;}
       | '(' op ')'  /* operator overloads */  {$$ = (Node*)strdup((char*)$2);}
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

/* NOTE: lextxt contents from fn_name and the mangleFn result are freed in the call to mkFuncDeclNode */
fn_ext_def: modifier_list maybe_newline Fun type_expr '.' fn_name ':' params RArrow type_expr block  {$$ = mkExtNode(@6, $4, mkFuncDeclNode(@$, /*fn_name*/$6, /*mods*/$1, /*ret_ty*/$10,                                 /*params*/$8, /*body*/$11));}
          | modifier_list maybe_newline Fun type_expr '.' fn_name ':' RArrow type_expr block         {$$ = mkExtNode(@6, $4, mkFuncDeclNode(@$, /*fn_name*/$6, /*mods*/$1, /*ret_ty*/$9,                                  /*params*/0,  /*body*/$10));}
          | modifier_list maybe_newline Fun type_expr '.' fn_name ':' params block                   {$$ = mkExtNode(@6, $4, mkFuncDeclNode(@$, /*fn_name*/$6, /*mods*/$1, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$8, /*body*/$9)); }
          | modifier_list maybe_newline Fun type_expr '.' fn_name ':' block                          {$$ = mkExtNode(@6, $4, mkFuncDeclNode(@$, /*fn_name*/$6, /*mods*/$1, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/$8)); }
          | Fun type_expr '.' fn_name ':' params RArrow type_expr block                              {$$ = mkExtNode(@4, $2, mkFuncDeclNode(@$, /*fn_name*/$4, /*mods*/ 0, /*ret_ty*/$8,                                  /*params*/$6, /*body*/$9)); }
          | Fun type_expr '.' fn_name ':' RArrow type_expr block                                     {$$ = mkExtNode(@4, $2, mkFuncDeclNode(@$, /*fn_name*/$4, /*mods*/ 0, /*ret_ty*/$7,                                  /*params*/0,  /*body*/$8)); }
          | Fun type_expr '.' fn_name ':' params block                                               {$$ = mkExtNode(@4, $2, mkFuncDeclNode(@$, /*fn_name*/$4, /*mods*/ 0, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$6, /*body*/$7)); }
          | Fun type_expr '.' fn_name ':' block                                                      {$$ = mkExtNode(@4, $2, mkFuncDeclNode(@$, /*fn_name*/$4, /*mods*/ 0, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/$6)); }
          ;

fn_ext_inferredRet: modifier_list maybe_newline Fun type_expr '.' fn_name ':' params '=' expr   {$$ = mkExtNode(@$, $4, mkFuncDeclNode(@6, /*fn_name*/$6, /*mods*/$1, /*ret_ty*/0, /*params*/$8, /*body*/$10));}
                  | modifier_list maybe_newline Fun type_expr '.' fn_name ':' '=' expr          {$$ = mkExtNode(@$, $4, mkFuncDeclNode(@6, /*fn_name*/$6, /*mods*/$1, /*ret_ty*/0, /*params*/0,  /*body*/$9)); }
                  | Fun type_expr '.' fn_name ':' params '=' expr                               {$$ = mkExtNode(@$, $2, mkFuncDeclNode(@4, /*fn_name*/$4, /*mods*/ 0, /*ret_ty*/0, /*params*/$6, /*body*/$8)); }
                  | Fun type_expr '.' fn_name ':' '=' expr                                      {$$ = mkExtNode(@$, $2, mkFuncDeclNode(@4, /*fn_name*/$4, /*mods*/ 0, /*ret_ty*/0, /*params*/0,  /*body*/$7)); }
                  ;

fn_def: modifier_list maybe_newline Fun fn_name ':' params RArrow type_expr block  {$$ = mkFuncDeclNode(@4, /*fn_name*/$4, /*mods*/$1, /*ret_ty*/$8,                                  /*params*/$6, /*body*/$9);}
      | modifier_list maybe_newline Fun fn_name ':' RArrow type_expr block         {$$ = mkFuncDeclNode(@4, /*fn_name*/$4, /*mods*/$1, /*ret_ty*/$7,                                  /*params*/0,  /*body*/$8);}
      | modifier_list maybe_newline Fun fn_name ':' params block                   {$$ = mkFuncDeclNode(@4, /*fn_name*/$4, /*mods*/$1, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$6, /*body*/$7);}
      | modifier_list maybe_newline Fun fn_name ':' block                          {$$ = mkFuncDeclNode(@4, /*fn_name*/$4, /*mods*/$1, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/$6);}
      | Fun fn_name ':' params RArrow type_expr block                              {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*mods*/ 0, /*ret_ty*/$6,                                  /*params*/$4, /*body*/$7);}
      | Fun fn_name ':' RArrow type_expr block                                     {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*mods*/ 0, /*ret_ty*/$5,                                  /*params*/0,  /*body*/$6);}
      | Fun fn_name ':' params block                                               {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*mods*/ 0, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$4, /*body*/$5);}
      | Fun fn_name ':' block                                                      {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*mods*/ 0, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/$4);}
      ;

fn_inferredRet: modifier_list maybe_newline Fun fn_name ':' params '=' expr   {$$ = mkFuncDeclNode(@4, /*fn_name*/$4, /*mods*/$1, /*ret_ty*/0, /*params*/$6, /*body*/$8);}
              | modifier_list maybe_newline Fun fn_name ':' '=' expr          {$$ = mkFuncDeclNode(@4, /*fn_name*/$4, /*mods*/$1, /*ret_ty*/0, /*params*/0,  /*body*/$7);}
              | Fun fn_name ':' params '=' expr                               {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*mods*/ 0, /*ret_ty*/0, /*params*/$4, /*body*/$6);}
              | Fun fn_name ':' '=' expr                                      {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*mods*/ 0, /*ret_ty*/0, /*params*/0,  /*body*/$5);}
              ;

fn_decl: modifier_list maybe_newline Fun fn_name ':' params RArrow type_expr ';'   {$$ = mkFuncDeclNode(@4, /*fn_name*/externCName($4), /*mods*/$1, /*ret_ty*/$8,                                  /*params*/$6, /*body*/0);}
       | modifier_list maybe_newline Fun fn_name ':' RArrow type_expr        ';'   {$$ = mkFuncDeclNode(@4, /*fn_name*/externCName($4), /*mods*/$1, /*ret_ty*/$7,                                  /*params*/0,  /*body*/0);}
       | modifier_list maybe_newline Fun fn_name ':' params                  ';'   {$$ = mkFuncDeclNode(@4, /*fn_name*/externCName($4), /*mods*/$1, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$6, /*body*/0);}
       | modifier_list maybe_newline Fun fn_name ':'                         ';'   {$$ = mkFuncDeclNode(@4, /*fn_name*/externCName($4), /*mods*/$1, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/0);}
       | Fun fn_name ':' params RArrow type_expr                             ';'   {$$ = mkFuncDeclNode(@2, /*fn_name*/externCName($2), /*mods*/ 0, /*ret_ty*/$6,                                  /*params*/$4, /*body*/0);}
       | Fun fn_name ':' RArrow type_expr                                    ';'   {$$ = mkFuncDeclNode(@2, /*fn_name*/externCName($2), /*mods*/ 0, /*ret_ty*/$5,                                  /*params*/0,  /*body*/0);}
       | Fun fn_name ':' params                                              ';'   {$$ = mkFuncDeclNode(@2, /*fn_name*/externCName($2), /*mods*/ 0, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$4, /*body*/0);}
       | Fun fn_name ':'                                                     ';'   {$$ = mkFuncDeclNode(@2, /*fn_name*/externCName($2), /*mods*/ 0, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/0);}
       ;

fn_ext_decl: modifier_list maybe_newline Fun type_expr '.' fn_name ':' params RArrow type_expr ';'   {$$ = mkExtNode(@4, $4, mkFuncDeclNode(@$, /*fn_name*/externCName($6), /*mods*/$1, /*ret_ty*/$10,                                 /*params*/$8, /*body*/0));}
           | modifier_list maybe_newline Fun type_expr '.' fn_name ':' RArrow type_expr        ';'   {$$ = mkExtNode(@4, $4, mkFuncDeclNode(@$, /*fn_name*/externCName($6), /*mods*/$1, /*ret_ty*/$9,                                  /*params*/0,  /*body*/0));}
           | modifier_list maybe_newline Fun type_expr '.' fn_name ':' params                  ';'   {$$ = mkExtNode(@4, $4, mkFuncDeclNode(@$, /*fn_name*/externCName($6), /*mods*/$1, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$8, /*body*/0));}
           | modifier_list maybe_newline Fun type_expr '.' fn_name ':'                         ';'   {$$ = mkExtNode(@4, $4, mkFuncDeclNode(@$, /*fn_name*/externCName($6), /*mods*/$1, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/0));}
           | Fun type_expr '.' fn_name ':' params RArrow type_expr                             ';'   {$$ = mkExtNode(@2, $2, mkFuncDeclNode(@$, /*fn_name*/externCName($4), /*mods*/ 0, /*ret_ty*/$8,                                  /*params*/$6, /*body*/0));}
           | Fun type_expr '.' fn_name ':' RArrow type_expr                                    ';'   {$$ = mkExtNode(@2, $2, mkFuncDeclNode(@$, /*fn_name*/externCName($4), /*mods*/ 0, /*ret_ty*/$7,                                  /*params*/0,  /*body*/0));}
           | Fun type_expr '.' fn_name ':' params                                              ';'   {$$ = mkExtNode(@2, $2, mkFuncDeclNode(@$, /*fn_name*/externCName($4), /*mods*/ 0, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$6, /*body*/0));}
           | Fun type_expr '.' fn_name ':'                                                     ';'   {$$ = mkExtNode(@2, $2, mkFuncDeclNode(@$, /*fn_name*/externCName($4), /*mods*/ 0, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/0));}
           ;

fn_lambda: modifier_list maybe_newline Fun params '=' expr  %prec Fun  {$$ = mkFuncDeclNode(@$, /*fn_name*/(Node*)strdup(""), /*mods*/$1, /*ret_ty*/0,  /*params*/$4, /*body*/$6);}
         | modifier_list maybe_newline Fun '=' expr         %prec Fun  {$$ = mkFuncDeclNode(@$, /*fn_name*/(Node*)strdup(""), /*mods*/$1, /*ret_ty*/0,  /*params*/0,  /*body*/$5);}
         | Fun params '=' expr                              %prec Fun  {$$ = mkFuncDeclNode(@$, /*fn_name*/(Node*)strdup(""), /*mods*/ 0, /*ret_ty*/0,  /*params*/$2, /*body*/$4);}
         | Fun '=' expr                                     %prec Fun  {$$ = mkFuncDeclNode(@$, /*fn_name*/(Node*)strdup(""), /*mods*/ 0, /*ret_ty*/0,  /*params*/0,  /*body*/$3);}
         ;



ret_expr: Return expr {$$ = mkRetNode(@$, $2);}
        ;


extension: Ext type_expr Indent fn_list Unindent {$$ = mkExtNode(@$, $2, $4);}
         | Ext type_expr ':' usertype_list Indent fn_list Unindent {$$ = mkExtNode(@$, $2, $6, $4);}
         | fn_ext_def
         | fn_ext_inferredRet
         | fn_ext_decl
         ;

usertype_list: usertype_list_  {$$ = getRoot();}

usertype_list_: usertype_list_ ',' usertype {$$ = setNext($1, mkTypeNode(@3, TT_Data, (char*)$3)); free($3);}
              | usertype                    {$$ = setRoot(mkTypeNode(@$, TT_Data, (char*)$1)); free($1);}
              ;


fn_list: fn_list_ {$$ = getRoot();}

fn_list_: fn_list_ function maybe_newline  {$$ = setNext($1, $2);}
        | function maybe_newline           {$$ = setRoot($1);}
        ;


while_loop: While bound_expr Do expr  %prec While  {$$ = mkWhileNode(@$, $2, $4);}
          ;

/*            vvvvv this will be later changed to pattern  */
for_loop: For ident In bound_expr Do expr  %prec For  {$$ = mkForNode(@$, $2, $4, $6); free($2);}


break: Break expr  %prec Break  {$$ = mkJumpNode(@$, Tok_Break, $2);}
     | Break                    {$$ = mkJumpNode(@$, Tok_Break, mkIntLitNode(@$, (char*)"1"));}
     ;


continue: Continue expr  %prec Continue  {$$ = mkJumpNode(@$, Tok_Continue, $2);}
        | Continue                       {$$ = mkJumpNode(@$, Tok_Continue, mkIntLitNode(@$, (char*)"1"));}
        ;


match: '|' bound_expr RArrow expr              {$$ = mkMatchBranchNode(@$, $2, $4);}
     | '|' usertype RArrow expr  %prec Match {$$ = mkMatchBranchNode(@$, mkTypeNode(@2, TT_Data, (char*)$2), $4); free($2);}
     ;


match_expr: Match bound_expr With Newline match  {$$ = mkMatchNode(@$, $2, $5);}
          | match_expr Newline match       {$$ = addMatch($1, $3);}
          ;

fn_brackets: '{' expr_list '}' {$$ = mkTupleNode(@$, $2);}
           | '{' '}'           {$$ = mkTupleNode(@$, 0);}
           ;

if_expr: If bound_expr Then expr_or_jump                %prec MEDIF  {$$ = mkIfNode(@$, $2, $4, 0);}
       | if_expr Elif bound_expr Then expr_or_jump      %prec MEDIF  {auto*elif = mkIfNode(@$, $3, $5, 0); setElse($1, elif); $$ = elif;}
       | if_expr Else expr_or_jump                             {$$ = setElse($1, $3);}
       ;

var: ident  %prec Ident {$$ = mkVarNode(@$, (char*)$1); free($1);}
   ;


val_no_decl: '(' bound_expr ')'            {$$ = $2;}
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
           | if_expr     %prec STMT  {$$ = $1;}
           | match_expr  %prec LOW   {$$ = $1;}
           | explicit_block          {$$ = $1;}
           | type_expr   %prec LOW
           | global
           ;

val: val_no_decl
   | data_decl
   | trait_decl
   | function
   | extension
   ;

tuple: '(' expr_list ')' {$$ = mkTupleNode(@$, $2);}
     | '(' ')'           {$$ = mkTupleNode(@$, 0);}
     ;

array: '[' expr_list ']' {$$ = mkArrayNode(@$, $2);}
     | '[' ']'           {$$ = mkArrayNode(@$, 0);}
     ;


unary_op: '@' expr_with_decls                              {$$ = mkUnOpNode(@$, '@', $2);}
        | '&' expr_with_decls                              {$$ = mkUnOpNode(@$, '&', $2);}
        | New expr_with_decls                              {$$ = mkUnOpNode(@$, Tok_New, $2);}
        | Not expr_with_decls                              {$$ = mkUnOpNode(@$, Tok_Not, $2);}
        | non_generic_type expr_with_decls      %prec TYPE {$$ = mkTypeCastNode(@$, $1, $2);}
        | explicit_generic_type expr_with_decls %prec TYPE {$$ = mkTypeCastNode(@$, $1, $2);}
        ;

explicit_generic_type: non_generic_type '<' type_list '>'    %prec TYPE {$$ = $1; ((TypeNode*)$1)->params = toOwnedVec(getRoot());}
                     ;

type_list: type_list ',' type  %prec TYPE {$$ = setNext($1, $3);}
         | type                %prec TYPE {$$ = setRoot($1);}
         ;

preproc: '!' '[' bound_expr ']'  {$$ = mkPreProcNode(@$, $3);}
       | '!' var                 {$$ = mkPreProcNode(@$, $2);}
       ;

arg_list: arg_list_p  %prec FUNC {$$ = mkTupleNode(@$, getRoot());}
        ;

arg_list_p: arg_list_p arg        %prec FUNC {$$ = setNext($1, $2);}
          | arg                   %prec FUNC {$$ = setRoot($1);}
          ;

arg: val
   | arg '.' var        {$$ = mkBinOpNode(@$, '.', $1, $3);}
   | type_expr '.' var  {$$ = mkBinOpNode(@$, '.', $1, $3);}
   | arg fn_brackets    {$$ = mkBinOpNode(@$, '(', $1, $2);}
   ;

/* expr is used in expression blocks and can span multiple lines */
expr_list: expr_list_p {$$ = getRoot();}
         ;


expr_list_p: expr_list_p ',' maybe_newline bound_expr  %prec ',' {$$ = setNext($1, $4);}
           | bound_expr                                %prec LOW {$$ = setRoot($1);}
           ;

expr_no_decl_or_jump: expr_no_decl  %prec MEDIF
                    | block
                    | break
                    | continue
                    | ret_expr
                    ;

expr_no_decl: expr_no_decl '+' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '+', $1, $4);}
            | expr_no_decl '-' expr_no_decl                                    {$$ = mkBinOpNode(@$, '-', $1, $3);}
            | '-' expr_no_decl                                                 {$$ = mkUnOpNode(@$, '-', $2);}
            | expr_no_decl '-' Newline expr_no_decl                            {$$ = mkBinOpNode(@$, '-', $1, $4);}
            | expr_no_decl '*' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '*', $1, $4);}
            | expr_no_decl '/' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '/', $1, $4);}
            | expr_no_decl '%' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '%', $1, $4);}
            | expr_no_decl '<' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '<', $1, $4);}
            | expr_no_decl '>' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '>', $1, $4);}
            | type_expr '.' maybe_newline var                                  {$$ = mkBinOpNode(@$, '.', $1, $4);}
            | expr_no_decl '.' maybe_newline var                               {$$ = mkBinOpNode(@$, '.', $1, $4);}
            | expr_no_decl ';' maybe_newline expr_no_decl                      {$$ = mkSeqNode(@$, $1, $4);}
            | expr_no_decl '#' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '#', $1, $4);}
            | expr_no_decl Eq maybe_newline expr_no_decl                       {$$ = mkBinOpNode(@$, Tok_Eq, $1, $4);}
            | expr_no_decl NotEq maybe_newline expr_no_decl                    {$$ = mkBinOpNode(@$, Tok_NotEq, $1, $4);}
            | expr_no_decl GrtrEq maybe_newline expr_no_decl                   {$$ = mkBinOpNode(@$, Tok_GrtrEq, $1, $4);}
            | expr_no_decl LesrEq maybe_newline expr_no_decl                   {$$ = mkBinOpNode(@$, Tok_LesrEq, $1, $4);}
            | expr_no_decl Or maybe_newline expr_no_decl                       {$$ = mkBinOpNode(@$, Tok_Or, $1, $4);}
            | expr_no_decl And maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, Tok_And, $1, $4);}
            | expr_no_decl '=' maybe_newline expr_no_decl                      {$$ = mkVarAssignNode(@$, $1, $4);} /* All VarAssignNodes return void values */
            | expr_no_decl AddEq maybe_newline expr_no_decl                    {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '+', $1, $4), false);}
            | expr_no_decl SubEq maybe_newline expr_no_decl                    {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '-', $1, $4), false);}
            | expr_no_decl MulEq maybe_newline expr_no_decl                    {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '*', $1, $4), false);}
            | expr_no_decl DivEq maybe_newline expr_no_decl                    {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '/', $1, $4), false);}
            | expr_no_decl ApplyR maybe_newline expr_no_decl                   {$$ = mkBinOpNode(@$, '(', $4, $1);}
            | expr_no_decl ApplyL maybe_newline expr_no_decl                   {$$ = mkBinOpNode(@$, '(', $1, $4);}
            | expr_no_decl Append maybe_newline expr_no_decl                   {$$ = mkBinOpNode(@$, Tok_Append, $1, $4);}
            | expr_no_decl Range maybe_newline expr_no_decl                    {$$ = mkBinOpNode(@$, Tok_Range, $1, $4);}
            | expr_no_decl In maybe_newline expr_no_decl                       {$$ = mkBinOpNode(@$, Tok_In, $1, $4);}
            | expr_no_decl fn_brackets                                         {$$ = mkBinOpNode(@$, '(', $1, $2);}
            | expr_no_decl arg_list                                            {$$ = mkBinOpNode(@$, '(', $1, $2);}
            | val_no_decl                                           %prec MED  {$$ = $1;}

            /* this rule returns the original If for precedence reasons compared to its mirror rule in if_expr
             * that returns the elif node itself.  The former necessitates setElse to travel through the first IfNode's
             * internal linked list of elsenodes to find the last one and append the new elif */
            | expr_no_decl Elif bound_expr Then expr_no_decl_or_jump    %prec MEDIF  {auto*elif = mkIfNode(@$, $3, $5, 0); $$ = setElse($1, elif);}
            | expr_no_decl Else expr_no_decl_or_jump                        %prec Else {$$ = setElse($1, $3);}

            | match_expr Newline expr_no_decl                      %prec Match  {$$ = mkSeqNode(@$, $1, $3);}
            | match_expr Newline                                   %prec LOW    {$$ = $1;}
//            | expr_no_decl Newline                                              {$$ = $1;}
//            | expr_no_decl Newline expr_no_decl                                 {$$ = mkSeqNode(@$, $1, $3);}
            ;


expr_or_jump: expr  %prec MEDIF
            | break
            | continue
            | ret_expr
            ;

expr: expr_with_decls  %prec MEDIF
    | block
    ;

bound_expr: expr_with_decls  %prec MEDIF
          | block Newline
          ;

expr_with_decls: expr_with_decls '+' maybe_newline expr_with_decls                    {$$ = mkBinOpNode(@$, '+', $1, $4);}
               | expr_with_decls '-' expr_with_decls                                  {$$ = mkBinOpNode(@$, '-', $1, $3);}
               | '-' expr_with_decls                                                  {$$ = mkUnOpNode(@$, '-', $2);}
               | expr_with_decls '-' Newline expr_with_decls                          {$$ = mkBinOpNode(@$, '-', $1, $4);}
               | expr_with_decls '*' maybe_newline expr_with_decls                    {$$ = mkBinOpNode(@$, '*', $1, $4);}
               | expr_with_decls '/' maybe_newline expr_with_decls                    {$$ = mkBinOpNode(@$, '/', $1, $4);}
               | expr_with_decls '%' maybe_newline expr_with_decls                    {$$ = mkBinOpNode(@$, '%', $1, $4);}
               | expr_with_decls '<' maybe_newline expr_with_decls                    {$$ = mkBinOpNode(@$, '<', $1, $4);}
               | expr_with_decls '>' maybe_newline expr_with_decls                    {$$ = mkBinOpNode(@$, '>', $1, $4);}
               | type_expr '.' maybe_newline var                                      {$$ = mkBinOpNode(@$, '.', $1, $4);}
               | expr_with_decls '.' maybe_newline var                                {$$ = mkBinOpNode(@$, '.', $1, $4);}
               | expr_with_decls ';' maybe_newline expr_with_decls                    {$$ = mkSeqNode(@$, $1, $4);}
               | expr_with_decls '#' maybe_newline expr_with_decls                    {$$ = mkBinOpNode(@$, '#', $1, $4);}
               | expr_with_decls Eq maybe_newline expr_with_decls                     {$$ = mkBinOpNode(@$, Tok_Eq, $1, $4);}
               | expr_with_decls NotEq maybe_newline expr_with_decls                  {$$ = mkBinOpNode(@$, Tok_NotEq, $1, $4);}
               | expr_with_decls GrtrEq maybe_newline expr_with_decls                 {$$ = mkBinOpNode(@$, Tok_GrtrEq, $1, $4);}
               | expr_with_decls LesrEq maybe_newline expr_with_decls                 {$$ = mkBinOpNode(@$, Tok_LesrEq, $1, $4);}
               | expr_with_decls Or maybe_newline expr_with_decls                     {$$ = mkBinOpNode(@$, Tok_Or, $1, $4);}
               | expr_with_decls And maybe_newline expr_with_decls                    {$$ = mkBinOpNode(@$, Tok_And, $1, $4);}
               | expr_with_decls '=' maybe_newline expr_with_decls                    {$$ = mkVarAssignNode(@$, $1, $4);} /* All VarAssignNodes return void values */
               | expr_with_decls AddEq maybe_newline expr_with_decls                  {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '+', $1, $4), false);}
               | expr_with_decls SubEq maybe_newline expr_with_decls                  {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '-', $1, $4), false);}
               | expr_with_decls MulEq maybe_newline expr_with_decls                  {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '*', $1, $4), false);}
               | expr_with_decls DivEq maybe_newline expr_with_decls                  {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '/', $1, $4), false);}
               | expr_with_decls ApplyR maybe_newline expr_with_decls                 {$$ = mkBinOpNode(@$, '(', $4, $1);}
               | expr_with_decls ApplyL maybe_newline expr_with_decls                 {$$ = mkBinOpNode(@$, '(', $1, $4);}
               | expr_with_decls Append maybe_newline expr_with_decls                 {$$ = mkBinOpNode(@$, Tok_Append, $1, $4);}
               | expr_with_decls Range maybe_newline expr_with_decls                  {$$ = mkBinOpNode(@$, Tok_Range, $1, $4);}
               | expr_with_decls In maybe_newline expr_with_decls                     {$$ = mkBinOpNode(@$, Tok_In, $1, $4);}
               | expr_with_decls fn_brackets                                          {$$ = mkBinOpNode(@$, '(', $1, $2);}
               | expr_with_decls arg_list                                             {$$ = mkBinOpNode(@$, '(', $1, $2);}
               | val                                                       %prec MED  {$$ = $1;}

               /* this rule returns the original If for precedence reasons compared to its mirror rule in if_expr
                * that returns the elif node itself.  The former necessitates setElse to travel through the first IfNode's
                * internal linked list of elsenodes to find the last one and append the new elif */
               | expr_with_decls Elif bound_expr Then expr_or_jump  %prec MEDIF             {auto*elif = mkIfNode(@$, $3, $5, 0); $$ = setElse($1, elif);}
               | expr_with_decls Else expr_or_jump                                    {$$ = setElse($1, $3);}

               | match_expr Newline expr_with_decls                      %prec Match  {$$ = mkSeqNode(@$, $1, $3);}
               | match_expr Newline                                                   {$$ = $1;}
               | expr_with_decls Newline                                              {$$ = $1;}
               | expr_with_decls Newline expr_with_decls                              {$$ = mkSeqNode(@$, $1, $3);}
               ;

%%

/* location parser error */
void yy::parser::error(const location& loc, const string& msg){
    location l = loc;
    ante::error(msg.c_str(), l);
}

Node* externCName(Node *n){
    size_t len = strlen((char*)n);
    char *c = (char*)realloc(n, len+2);
    c[len] = ';';
    c[len+1] = '\0';
    return (Node*)c;
}

vector<unique_ptr<TypeNode>> toOwnedVec(Node *tn){
    vector<unique_ptr<TypeNode>> ret;
    while(tn){
        ret.push_back(unique_ptr<TypeNode>((TypeNode*)tn));
        tn = tn->next.get();
    }
    return ret;
}

vector<unique_ptr<TypeNode>> concat(vector<unique_ptr<TypeNode>>&& l, Node *tn){
    auto r = toOwnedVec(tn);
    vector<unique_ptr<TypeNode>> ret;
    ret.reserve(l.size() + r.size());
    for(auto &&e : l) ret.insert(ret.end(), move(e));
    for(auto &&e : r) ret.insert(ret.end(), move(e));
    return ret;
}
