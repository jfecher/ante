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
    extern string mangle(std::string const& base, NamedValNode *paramTys);
    static size_t ante_parser_errcount = 0;

    namespace parser {
        struct TypeNode;

        vector<unique_ptr<TypeNode>> toOwnedVec(Node *tn);
        vector<unique_ptr<TypeNode>> concat(vector<unique_ptr<TypeNode>>&& l, Node *tn);
        Node* name(Node *varNode);
    }
}

%}

%locations
%define parse.error verbose

%token Ident UserType TypeVar

/* types */
%token I8 I16 I32 I64
%token U8 U16 U32 U64
%token Isz Usz F16 F32 F64
%token C8 Bool Unit

/* operators */
%token Assign EqEq NotEq AddEq SubEq MulEq DivEq GrtrEq LesrEq
%token Or And Range VarArgs RArrow ApplyL ApplyR Append New Not Is Isnt

/* literals */
%token True False
%token IntLit FltLit StrLit CharLit

/* keywords */
%token Return
%token If Then Elif Else
%token For While Do In
%token Continue Break Import Let
%token Match With Ref Type Trait
%token Given Module Impl Block As Self

/* modifiers */
%token Pub Pri Pro Const
%token Mut Global Ante

/* other */
%token Where
%token InterpolateBegin InterpolateEnd UnfinishedStr

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
%left STMT Fun Let Import Return Module Impl While For Match Trait If Break Continue Type
%right RArrow Given

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

%left MED '=' '\\'

%left ','
%left Assign AddEq SubEq MulEq DivEq
%left ';'
%left MODIFIER Pub Pri Pro Const Mut Global Ante

%right ApplyL
%left ApplyR

%nonassoc '!'

%left Or
%left And
%left Not
%left EqEq Is Isnt NotEq GrtrEq LesrEq '<' '>'

%left In
%left Append
%left Range

%left '+' '-'
%left '*' '/' '%'
%right '^'

%left ':'

%left As
%left '#'
%left UNARY '@' New '&' Ref
%left TYPE UserType TypeVar I8 I16 I32 I64 U8 U16 U32 U64 Isz Usz F16 F32 F64 C8 Bool Unit VarArgs
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

begin:  maybe_newline {createRoot();} top_level_expr_list
     |  maybe_newline {createRoot();}
     ;

top_level_expr_list: top_level_expr_list top_level_expr  %prec Newline
                   | top_level_expr_list expr_no_decl    %prec Newline    {$$ = append_main($2);}
                   | top_level_expr_list Newline         
                   | top_level_expr
                   | expr_no_decl                        %prec Newline  {$$ = append_main($1);}

                   | top_level_expr_list Elif expr Then expr_no_decl_or_jump    %prec MEDIF {auto*elif = mkIfNode(@$, $3, $5, 0); $$ = setElse($1, elif);}
                   | top_level_expr_list Else expr_no_decl_or_jump                    %prec Else  {$$ = setElse($1, $3);}
                   ;

top_level_expr: modifiers top_level_expr_nm     {$$ = append_modifiers($1, $2);}
              | top_level_expr_nm
              ;

top_level_expr_nm: function                                   {$$ = append_fn($1);}
                 | data_decl                                  {$$ = append_type($1);}
                 | extension                                  {$$ = append_extension($1);}
                 | trait_decl                                 {$$ = append_trait($1);}
                 | import_expr                                {$$ = append_import($1);}
                 ;

/*
 * %empty is avoided here since it is handled differently on
 * windows and older versions of bison
 */
maybe_newline: Newline  %prec Newline
             |
             ;


import_expr: Import expr {$$ = mkImportNode(@$, $2);}


ident: Ident {$$ = (Node*)lextxt;}
     | Self  {$$ = (Node*)strdup("self");}
     ;

usertype: UserType {$$ = (Node*)lextxt;}
        ;

usertype_node: usertype {$$ = mkTypeNode(@$, TT_Data, (char*)$1);}
             ;

typevar: TypeVar {$$ = (Node*)lextxt;}
       ;

intlit: IntLit {$$ = mkIntLitNode(@$, lextxt);}
      ;

fltlit: FltLit {$$ = mkFltLitNode(@$, lextxt);}
      ;

strlit: StrLit                                       {$$ = mkStrLitNode(@$, lextxt);}
      | strlit UnfinishedStr                         {$$ = mkBinOpNode(@$, Tok_Append, $1, mkStrLitNode(@2, lextxt));}
      | strlit InterpolateBegin expr InterpolateEnd  {$$ = mkBinOpNode(@$, Tok_Append, $1, mkBinOpNode(@2, Tok_As, $3, mkTypeNode(@2, TT_Data, (char*)"Str")));}
      ;

charlit: CharLit {$$ = mkCharLitNode(@$, lextxt);}
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
        | Bool                {$$ = mkTypeNode(@$, TT_Bool, (char*)"");}
        | Unit                {$$ = mkTypeNode(@$, TT_Unit, (char*)"");}
        | usertype_node       {$$ = $1;}
        | typevar             {$$ = mkTypeNode(@$, TT_TypeVar, (char*)$1);}
        ;

varargs: VarArgs    {$$ = mkTypeNode(@$, TT_TypeVar, nextVarArgsTVName());}

pointer_type: Ref type     {$$ = mkTypeNode(@$, TT_Ptr, (char*)"", $2);}
            ;

fn_type: small_type RArrow type    {setNext($3, $1); $$ = mkTypeNode(@$, TT_Function, (char*)"", $3);}
       ;

arr_type: '[' intlit small_type ']' {$3->next.reset($2);
                                     $$ = mkTypeNode(@$, TT_Array, (char*)"", $3);}
        | '[' small_type ']'        {$2->next.reset(mkIntLitNode(@$, (char*)"0"));
                                     $$ = mkTypeNode(@$, TT_Array, (char*)"", $2);}
        ;

tuple_type: '(' comma_delimited_types ')'      {$$ = mkTypeNode(@$, TT_Tuple, (char*)"", getRoot());}
          | '(' comma_delimited_types ',' ')'  {$$ = mkTypeNode(@$, TT_Tuple, (char*)"", getRoot());}
          ;

comma_delimited_types: comma_delimited_types ',' type      {$$ = setNext($1, $3);}
                     | type                                {$$ = setRoot($1);}
                     ;

type_with_generics: type_with_generics small_type   %prec STMT  {$$ = $1; ((TypeNode*)$1)->params.emplace_back((TypeNode*)$2); $1->loc = @$;}
                  | small_type                      %prec STMT  {$$ = $1;}
                  ;

small_type: lit_type
          | tuple_type
          | arr_type
          | pointer_type
          | '(' modified_type ')'   {$$ = $2;}
          | '(' fn_type ')'        {$$ = $2;}
          ;

modifiers: modifiers modifier maybe_newline   %prec MEDLOW   {$$ = setNext($1, $2);}
         | modifier maybe_newline             %prec LOW      {$$ = setRoot($1);}
         ;

modified_type: modifiers type_with_generics %prec MEDLOW  {$$ = append_modifiers(getRoot(), $2);}
             | type_with_generics           %prec LOW     {$$ = $1;}
             ;

type: modified_type     %prec LOW
    ;

preproc: '!' '[' expr ']'         {$$ = mkCompilerDirective(@$, $3);}
       | '!' var                  {$$ = mkCompilerDirective(@$, $2);}
       ;

modifier: Pub      {$$ = mkModNode(@$, Tok_Pub);}
        | Pri      {$$ = mkModNode(@$, Tok_Pri);}
        | Pro      {$$ = mkModNode(@$, Tok_Pro);}
        | Const    {$$ = mkModNode(@$, Tok_Const);}
        | Mut      {$$ = mkModNode(@$, Tok_Mut);}
        | Global   {$$ = mkModNode(@$, Tok_Global);}
        | Ante     {$$ = mkModNode(@$, Tok_Ante);}
        | preproc  %prec MODIFIER {$$ = $1;}
        ;


trait_decl: Trait usertype generic_params Indent trait_fn_list Unindent  {$$ = mkTraitNode(@$, (char*)$2, $3, $5);}
          ;

trait_fn_list: _trait_fn_list maybe_newline {$$ = getRoot();}

_trait_fn_list: _trait_fn_list Newline trait_fn    {$$ = setNext($1, $3);}
              | _trait_fn_list Newline type_family {$$ = setNext($1, $3);}
              | trait_fn                           {$$ = setRoot($1);}
              | type_family                        {$$ = setRoot($1);}
              ;


type_family: Type usertype                  {$$ = mkDataDeclNode(@2, (char*)$2,  0, 0, false);}
           | Type usertype generic_params   {$$ = mkDataDeclNode(@2, (char*)$2, $3, 0, false);}
           ;


params: params var ':' small_type          {$$ = setNext($1, mkNamedValNode(@2, $2, $4));}
      | params '(' var ':' type ')'        {$$ = setNext($1, mkNamedValNode(@3, $3, $5));}
      | params small_type                  {$$ = setNext($1, mkNamedValNode(@2, mkVarNode(@2, (char*)""), $2));}
      | var ':' small_type                 {$$ = setRoot(mkNamedValNode(@$, $1, $3));}
      | '(' var ':' type ')'               {$$ = setRoot(mkNamedValNode(@$, $2, $4));}
      | small_type                         {$$ = setRoot(mkNamedValNode(@$, mkVarNode(@1, (char*)""), $1));}
      ;


trait_fn_no_mods: var params RArrow type Given tc_constraints  {setNext($1, getRoot()); $$ = mkFuncDeclNode(@2, /*fn_name*/$1, /*ret_ty*/$4, /*constraints*/$6, /*body*/0);}
                | var params RArrow type                       {setNext($1, getRoot()); $$ = mkFuncDeclNode(@2, /*fn_name*/$1, /*ret_ty*/$4, /*constraints*/0,  /*body*/0);}
                ;

trait_fn: modifiers trait_fn_no_mods  {$$ = append_modifiers(getRoot(), $2);}
        | trait_fn_no_mods
        ;


typevar_list: typevar_list typevar  %prec LOW  {$$ = setNext($1, mkTypeNode(@$, TT_TypeVar, (char*)$2)); }
            | typevar               %prec LOW  {$$ = setRoot(mkTypeNode(@$, TT_TypeVar, (char*)$1)); }
            ;

generic_params: typevar_list  %prec LOW {$$ = getRoot();}
              ;


data_decl: Type usertype generic_params '=' type_decl_block                 {$$ = mkDataDeclNode(@$, (char*)$2, $3, $5, false);}
         | Type usertype '=' type_decl_block                                {$$ = mkDataDeclNode(@$, (char*)$2,  0, $4, false);}
         | Type usertype generic_params Is type_decl_block                  {$$ = mkDataDeclNode(@$, (char*)$2, $3, $5, true);}
         | Type usertype Is type_decl_block                                 {$$ = mkDataDeclNode(@$, (char*)$2,  0, $4, true);}
         ;

type_decl_list: type_decl_list Newline params                       {$$ = setNext($1, getRoot());}
              | type_decl_list Newline explicit_tagged_union_list   {$$ = setNext($1, getRoot());}
              | params                                              {$$ = $1;} /* leave root set */
              | explicit_tagged_union_list                          {$$ = $1;} /* leave root set */
              ;

/* tagged union list with mandatory '|' before first element */
explicit_tagged_union_list: explicit_tagged_union_list '|' usertype type    %prec STMT  {$$ = setNext($1, mkNamedValNode(@$, mkVarNode(@3, (char*)$3), mkTypeNode(@4, TT_TaggedUnion, (char*)"", $4)));}
                          | explicit_tagged_union_list '|' usertype         %prec STMT  {$$ = setNext($1, mkNamedValNode(@$, mkVarNode(@3, (char*)$3), mkTypeNode(@3, TT_TaggedUnion, (char*)"",  0)));}
                          | '|' usertype type                               %prec STMT  {$$ = setRoot(mkNamedValNode(@$, mkVarNode(@2, (char*)$2), mkTypeNode(@3, TT_TaggedUnion, (char*)"", $3)));}
                          | '|' usertype                                    %prec STMT  {$$ = setRoot(mkNamedValNode(@$, mkVarNode(@2, (char*)$2), mkTypeNode(@2, TT_TaggedUnion, (char*)"",  0)));}

type_decl_block: Indent type_decl_list Unindent   {$$ = getRoot();}
               | params              %prec LOW    {$$ = getRoot();}
               | explicit_tagged_union_list    %prec STMT  {$$ = getRoot();}
               ;

block: Indent expr Unindent                   {$$ = mkBlockNode(@$, $2);}
     | Indent break Unindent                  {$$ = mkBlockNode(@$, $2);}
     | Indent continue Unindent               {$$ = mkBlockNode(@$, $2);}
     | Indent ret_expr Unindent               {$$ = mkBlockNode(@$, $2);}

     | Indent expr break Unindent             {$$ = mkBlockNode(@$, mkSeqNode(@$, $2, $3));}
     | Indent expr continue Unindent          {$$ = mkBlockNode(@$, mkSeqNode(@$, $2, $3));}
     | Indent expr ret_expr Unindent          {$$ = mkBlockNode(@$, mkSeqNode(@$, $2, $3));}
     ;


explicit_block: Block block  {$$ = $2;}

function: fn_def
        | fn_decl
        | fn_inferredRet
        ;

op: '+'    {$$ = (Node*)"+";}
  | '-'    {$$ = (Node*)"-";}
  | '*'    {$$ = (Node*)"*";}
  | '/'    {$$ = (Node*)"/";}
  | '%'    {$$ = (Node*)"%";}
  | '^'    {$$ = (Node*)"^";}
  | '<'    {$$ = (Node*)"<";}
  | '>'    {$$ = (Node*)">";}
  | '.'    {$$ = (Node*)".";}
  | ';'    {$$ = (Node*)";";}
  | '#'    {$$ = (Node*)"#";}
  | '@'    {$$ = (Node*)"@";}
  | Not    {$$ = (Node*)"not";}
  | Assign {$$ = (Node*)":=";}
  | NotEq  {$$ = (Node*)"!=";}
  | GrtrEq {$$ = (Node*)">=";}
  | LesrEq {$$ = (Node*)"<=";}
  | EqEq   {$$ = (Node*)"==";}
  | AddEq  {$$ = (Node*)"+=";}
  | SubEq  {$$ = (Node*)"-=";}
  | MulEq  {$$ = (Node*)"*=";}
  | DivEq  {$$ = (Node*)"/=";}
  | ApplyR {$$ = (Node*)"|>";}
  | ApplyL {$$ = (Node*)"<|";}
  | Append {$$ = (Node*)"++";}
  | Range  {$$ = (Node*)"..";}
  | In     {$$ = (Node*)"in";}
  | Is     {$$ = (Node*)"is";}
  | Isnt   {$$ = (Node*)"isnt";}
  | As     {$$ = (Node*)"as";}
  ;


tc_constraints: type  %prec LOW
              ;

function_call: function_call val_no_decl    %prec LOW   {$$ = setNext($1, $2);}
             | function_call varargs        %prec LOW   {$$ = setNext($1, $2);}
             | val_no_decl val_no_decl      %prec LOW   {setRoot($1); $$ = setNext($1, $2);}
             | val_no_decl varargs          %prec LOW   {setRoot($1); $$ = setNext($1, $2);}
             ;

lambda_params: lambda_params var ':' small_type     {$$ = setNext($1, mkNamedValNode(@2, $2, $4));}
             | lambda_params '(' var ':' type ')'   {$$ = setNext($1, mkNamedValNode(@3, $3, $5));}
             | lambda_params small_type             {$$ = setNext($1, mkNamedValNode(@2, mkVarNode(@2, (char*)""), $2));}
             | lambda_params var                    {$$ = setNext($1, $2);}
             | var ':' small_type                   {$$ = setRoot(mkNamedValNode(@$, $1, $3));}
             | '(' var ':' type ')'                 {$$ = setRoot(mkNamedValNode(@$, $2, $4));}
             | small_type                           {$$ = setRoot(mkNamedValNode(@$, mkVarNode(@1, (char*)""), $1));}
             | var                                  {$$ = setRoot($1);}
             ;

/* NOTE: lextxt contents from fn_name and the mangleFn result are freed in the call to mkFuncDeclNode */
fn_def: function_call RArrow type Given tc_constraints '=' expr_or_block  {$$ = mkFuncDeclNode(@1, /*name and params*/getRoot(), /*ret_ty*/$3, /*constraints*/$5, /*body*/$7);}
      | function_call RArrow type '=' expr_or_block                       {$$ = mkFuncDeclNode(@1, /*name and params*/getRoot(), /*ret_ty*/$3, /*constraints*/0, /*body*/$5);}
      ;

fn_inferredRet: function_call Given tc_constraints '=' expr_or_block  %prec Newline  {$$ = mkFuncDeclNode(@1, /*name and params*/getRoot(), /*ret_ty*/0, /*constraints*/$3, /*body*/$5);}
              | function_call '=' expr_or_block                       %prec Newline  {$$ = mkFuncDeclNode(@1, /*name and params*/getRoot(), /*ret_ty*/0, /*constraints*/0,  /*body*/$3);}
              ;

fn_decl: function_call RArrow type Given tc_constraints  %prec Fun  {$$ = mkFuncDeclNode(@1, /*name and params*/getRoot(), /*ret_ty*/$3, /*constraints*/$5, /*body*/0);}
       | function_call RArrow type                       %prec Fun  {$$ = mkFuncDeclNode(@1, /*name and params*/getRoot(), /*ret_ty*/$3, /*constraints*/0,  /*body*/0);}
       ;

fn_lambda: '\\' lambda_params '=' expr_or_block  %prec Fun  {auto name = new VarNode(@1, ""); setNext(name, getRoot()); $$ = mkFuncDeclNode(@$, /*name and params*/name, /*ret_ty*/0,  /*constraints*/0, /*body*/$4);}
         | '\\' '=' expr_or_block                %prec Fun  {auto name = new VarNode(@1, "");                           $$ = mkFuncDeclNode(@$, /*name and params*/name, /*ret_ty*/0,  /*constraints*/0, /*body*/$3);}
         ;

ret_expr: Return expr {$$ = mkRetNode(@$, $2);}
        ;


extension: Module type Indent ext_list Unindent                     {$$ = mkExtNode(@$, $2, $4, 0);}
         | Impl   type Indent ext_list Unindent                     {$$ = mkExtNode(@$,  0, $4, $2);}
         | Impl   type Given tc_constraints Indent ext_list Unindent  {$$ = mkExtNode(@$,  0, $6, $2);}
         ;

ext_list: fn_list_ {$$ = getRoot();}

fn_list_: fn_list_ ext_fn maybe_newline  {$$ = setNext($1, $2);}
        | fn_list_ ext_dd maybe_newline  {$$ = setNext($1, $2);}
        | ext_fn maybe_newline           {$$ = setRoot($1);}
        | ext_dd maybe_newline           {$$ = setRoot($1);}
        ;

ext_fn: modifiers function  {$$ = append_modifiers(getRoot(), $2);}
      | function
      ;

ext_dd: modifiers data_decl  {$$ = append_modifiers(getRoot(), $2);}
      | data_decl
      ;


while_loop: While expr_or_block maybe_newline Do expr_or_block  %prec While  {$$ = mkWhileNode(@$, $2, $5);}
          ;

/*            v---v this should be later changed to pattern  */
for_loop: For ident In expr_or_block maybe_newline Do expr_or_block  %prec For  {$$ = mkForNode(@$, $2, $4, $7);}


break: Break expr  %prec Break  {$$ = mkJumpNode(@$, Tok_Break, $2);}
     | Break                    {$$ = mkJumpNode(@$, Tok_Break, mkIntLitNode(@$, (char*)"1"));}
     ;


continue: Continue expr  %prec Continue  {$$ = mkJumpNode(@$, Tok_Continue, $2);}
        | Continue                       {$$ = mkJumpNode(@$, Tok_Continue, mkIntLitNode(@$, (char*)"1"));}
        ;


match: '|' expr RArrow expr_or_block  {$$ = mkMatchBranchNode(@$, $2, $4);}
     ;


match_expr: Match expr_or_block maybe_newline With Newline match  {$$ = mkMatchNode(@$, $2, $6);}
          | match_expr Newline match       {$$ = addMatch($1, $3);}
          ;

if_expr: If expr_or_block maybe_newline Then expr_or_jump                %prec MEDIF  {$$ = mkIfNode(@$, $2, $5, 0);}
       | if_expr Elif expr_or_block maybe_newline Then expr_or_jump      %prec MEDIF  {auto*elif = mkIfNode(@$, $3, $6, 0); setElse($1, elif); $$ = elif;}
       | if_expr Else expr_or_jump                             {$$ = setElse($1, $3);}
       ;

var: ident  %prec Ident {$$ = mkVarNode(@$, (char*)$1);}
   | '(' op ')'         {$$ = mkVarNode(@$, strdup((char*)$2));}
   ;


val_no_decl: '(' expr ')'            {$$ = $2;}
           | tuple                   {$$ = $1;}
           | array                   {$$ = $1;}
           | var      %prec LOW      {$$ = $1;}
           | intlit                  {$$ = $1;}
           | fltlit                  {$$ = $1;}
           | strlit                  {$$ = $1;}
           | charlit                 {$$ = $1;}
           | True                    {$$ = mkBoolLitNode(@$, 1);}
           | False                   {$$ = mkBoolLitNode(@$, 0);}
           | while_loop              {$$ = $1;}
           | for_loop                {$$ = $1;}
           | if_expr     %prec STMT  {$$ = $1;}
           | match_expr  %prec LOW   {$$ = $1;}
           | explicit_block          {$$ = $1;}
           | usertype_node  %prec LOW  {$$ = $1;} // Union variant
           | fn_lambda
           | val_no_decl '.' maybe_newline var            {$$ = mkBinOpNode(@$, '.', $1, $4);}
           | val_no_decl '.' maybe_newline usertype_node  {$$ = mkBinOpNode(@$, '.', $1, $4);}
           | val_no_decl '.' maybe_newline intlit         {$$ = mkBinOpNode(@$, '.', $1, $4);}
           ;

expr_or_block: expr   %prec STMT
             | block
             ;

val: val_no_decl   %prec LOW
   | data_decl
   | trait_decl
   | function
   | extension
   | import_expr
   ;

tuple: '(' expr_list ')' {$$ = mkTupleNode(@$, $2);}
     | '(' ')'           {$$ = mkTupleNode(@$, 0);}
     ;

array: '[' expr_list ']' {$$ = mkArrayNode(@$, $2);}
     | '[' ']'           {$$ = mkArrayNode(@$, 0);}
     ;

constructor_args: constructor_args val_no_decl   %prec LOW   {$$ = setNext($1, $2);}
                | val_no_decl                    %prec LOW   {setRoot($1);}
                ;

unary_op: '@' expr                                    {$$ = mkUnOpNode(@$, '@', $2);}
        | '&' expr                                    {$$ = mkUnOpNode(@$, '&', $2);}
        | New expr                                    {$$ = mkUnOpNode(@$, Tok_New, $2);}
        | Not expr                                    {$$ = mkUnOpNode(@$, Tok_Not, $2);}
        ;

/* expr is used in expression blocks and can span multiple lines */
expr_list: expr_list_p {$$ = getRoot();}
         ;


expr_list_p: expr_list_p ',' maybe_newline expr  %prec ',' {$$ = setNext($1, $4);}
           | expr                                %prec LOW {$$ = setRoot($1);}
           ;

expr_no_decl_or_jump: expr_no_decl  %prec MEDIF
                    | break
                    | continue
                    | ret_expr
                    ;

expr_no_decl: expr_no_decl '+' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '+', $1, $4);}
            | expr_no_decl '-' expr_no_decl                                    {$$ = mkBinOpNode(@$, '-', $1, $3);}
            | '-' expr_no_decl                               %prec UNARY       {$$ = mkUnOpNode(@$, '-', $2);}
            | expr_no_decl '-' Newline expr_no_decl                            {$$ = mkBinOpNode(@$, '-', $1, $4);}
            | expr_no_decl '*' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '*', $1, $4);}
            | expr_no_decl '/' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '/', $1, $4);}
            | expr_no_decl '%' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '%', $1, $4);}
            | expr_no_decl '^' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '^', $1, $4);}
            | expr_no_decl '<' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '<', $1, $4);}
            | expr_no_decl '>' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '>', $1, $4);}
            | expr_no_decl ';' maybe_newline expr_no_decl                      {$$ = mkSeqNode(@$, $1, $4);}
            | expr_no_decl ':' maybe_newline type                              {$$ = mkBinOpNode(@$, ':', $1, $4);}
            | expr_no_decl '#' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '#', $1, $4);}
            | expr_no_decl EqEq maybe_newline expr_no_decl                     {$$ = mkBinOpNode(@$, Tok_EqEq, $1, $4);}
            | expr_no_decl Is maybe_newline expr_no_decl                       {$$ = mkBinOpNode(@$, Tok_Is, $1, $4);}
            | expr_no_decl Isnt maybe_newline expr_no_decl                     {$$ = mkBinOpNode(@$, Tok_Isnt, $1, $4);}
            | expr_no_decl NotEq maybe_newline expr_no_decl                    {$$ = mkBinOpNode(@$, Tok_NotEq, $1, $4);}
            | expr_no_decl GrtrEq maybe_newline expr_no_decl                   {$$ = mkBinOpNode(@$, Tok_GrtrEq, $1, $4);}
            | expr_no_decl LesrEq maybe_newline expr_no_decl                   {$$ = mkBinOpNode(@$, Tok_LesrEq, $1, $4);}
            | expr_no_decl Or maybe_newline expr_no_decl                       {$$ = mkBinOpNode(@$, Tok_Or, $1, $4);}
            | expr_no_decl And maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, Tok_And, $1, $4);}
            | expr_no_decl ApplyR maybe_newline expr_no_decl                   {$$ = mkBinOpNode(@$, '(', $4, mkTupleNode(@1, $1));}
            | expr_no_decl ApplyL maybe_newline expr_no_decl                   {$$ = mkBinOpNode(@$, '(', $1, mkTupleNode(@4, $4));}
            | expr_no_decl Append maybe_newline expr_no_decl                   {$$ = mkBinOpNode(@$, Tok_Append, $1, $4);}
            | expr_no_decl Range maybe_newline expr_no_decl                    {$$ = mkBinOpNode(@$, Tok_Range, $1, $4);}
            | expr_no_decl In maybe_newline expr_no_decl                       {$$ = mkBinOpNode(@$, Tok_In, $1, $4);}
            | expr_no_decl Not In maybe_newline expr_no_decl                   {$$ = mkUnOpNode(@$, Tok_Not, mkBinOpNode(@$, Tok_In, $1, $5));}
            | expr_no_decl As maybe_newline small_type                         {$$ = mkBinOpNode(@$, Tok_As, $1, $4);}
            | val_no_decl                                           %prec MED  {$$ = $1;}
            | unary_op                                                         {$$ = $1;}

            | function_call                                         %prec LOW  {$$ = mkFuncCallNode(@$, getRoot());}
            | usertype_node constructor_args                        %prec LOW  {$$ = mkTypeCastNode(@$, $1, mkTupleNode(@2, getRoot()));}
            | var '=' maybe_newline expr_or_block                              {$$ = mkVarAssignNode(@$, $1, $4); append_modifiers(mkModNode(@1, Tok_Let), $$);}
            | var '=' Mut maybe_newline expr_or_block                          {$$ = mkVarAssignNode(@$, $1, $5); append_modifiers(mkModNode(@1, Tok_Mut), $$);}
            | var '=' Global maybe_newline expr_or_block                       {$$ = mkVarAssignNode(@$, $1, $5); append_modifiers(mkModNode(@1, Tok_Global), $$);}
            | var '=' Ante maybe_newline expr_or_block                         {$$ = mkVarAssignNode(@$, $1, $5); append_modifiers(mkModNode(@1, Tok_Ante), $$);}
            | var '=' Pub maybe_newline expr_or_block                          {$$ = mkVarAssignNode(@$, $1, $5); append_modifiers(mkModNode(@1, Tok_Pub), $$);}
            | var '=' Pri maybe_newline expr_or_block                          {$$ = mkVarAssignNode(@$, $1, $5); append_modifiers(mkModNode(@1, Tok_Pri), $$);}
            | var '=' Pro maybe_newline expr_or_block                          {$$ = mkVarAssignNode(@$, $1, $5); append_modifiers(mkModNode(@1, Tok_Pro), $$);}
            | var '=' Const maybe_newline expr_or_block                        {$$ = mkVarAssignNode(@$, $1, $5); append_modifiers(mkModNode(@1, Tok_Const), $$);}
            | var '=' preproc maybe_newline expr_or_block                      {$$ = mkVarAssignNode(@$, $1, $5); append_modifiers($3, $$);}

            | expr_no_decl AddEq maybe_newline expr_no_decl             {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '+', $1, $4), false);}
            | expr_no_decl SubEq maybe_newline expr_no_decl             {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '-', $1, $4), false);}
            | expr_no_decl MulEq maybe_newline expr_no_decl             {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '*', $1, $4), false);}
            | expr_no_decl DivEq maybe_newline expr_no_decl             {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '/', $1, $4), false);}
            | expr_no_decl Assign maybe_newline expr_no_decl            {$$ = mkVarAssignNode(@$, $1, $4);} /* All VarAssignNodes return unit values */
            | expr_no_decl Assign maybe_newline block                   {$$ = mkVarAssignNode(@$, $1, $4);} /* All VarAssignNodes return unit values */
//            | modifiers expr_no_decl  %prec Newline       {$$ = append_modifiers($1, $2);}


            /* this rule returns the original If for precedence reasons compared to its mirror rule in if_expr
             * that returns the elif node itself.  The former necessitates setElse to travel through the first IfNode's
             * internal linked list of elsenodes to find the last one and append the new elif */
            | expr_no_decl Elif expr Then expr_no_decl_or_jump    %prec MEDIF  {auto*elif = mkIfNode(@$, $3, $5, 0); $$ = setElse($1, elif);}
            | expr_no_decl Else expr_no_decl_or_jump                        %prec Else {$$ = setElse($1, $3);}

            | match_expr Newline expr_no_decl                      %prec Match  {$$ = mkSeqNode(@$, $1, $3);}
            | match_expr Newline                                   %prec LOW    {$$ = $1;}
            ;


expr_or_jump: expr  %prec MEDIF
            | block
            | break
            | continue
            | ret_expr
            ;

expr: expr '+' maybe_newline expr                               {$$ = mkBinOpNode(@$, '+', $1, $4);}
    | expr '-' expr                                             {$$ = mkBinOpNode(@$, '-', $1, $3);}
    | '-' expr                                     %prec UNARY  {$$ = mkUnOpNode(@$, '-', $2);}
    | expr '-' Newline expr                                     {$$ = mkBinOpNode(@$, '-', $1, $4);}
    | expr '*' maybe_newline expr                               {$$ = mkBinOpNode(@$, '*', $1, $4);}
    | expr '/' maybe_newline expr                               {$$ = mkBinOpNode(@$, '/', $1, $4);}
    | expr '%' maybe_newline expr                               {$$ = mkBinOpNode(@$, '%', $1, $4);}
    | expr '^' maybe_newline expr                               {$$ = mkBinOpNode(@$, '^', $1, $4);}
    | expr '<' maybe_newline expr                               {$$ = mkBinOpNode(@$, '<', $1, $4);}
    | expr '>' maybe_newline expr                               {$$ = mkBinOpNode(@$, '>', $1, $4);}
    | expr ';' maybe_newline expr                               {$$ = mkSeqNode(@$, $1, $4);}
    | expr ':' maybe_newline type                               {$$ = mkBinOpNode(@$, ':', $1, $4);}
    | expr '#' maybe_newline expr                               {$$ = mkBinOpNode(@$, '#', $1, $4);}
    | expr EqEq maybe_newline expr                              {$$ = mkBinOpNode(@$, Tok_EqEq, $1, $4);}
    | expr Is maybe_newline expr                                {$$ = mkBinOpNode(@$, Tok_Is, $1, $4);}
    | expr Isnt maybe_newline expr                              {$$ = mkBinOpNode(@$, Tok_Isnt, $1, $4);}
    | expr NotEq maybe_newline expr                             {$$ = mkBinOpNode(@$, Tok_NotEq, $1, $4);}
    | expr GrtrEq maybe_newline expr                            {$$ = mkBinOpNode(@$, Tok_GrtrEq, $1, $4);}
    | expr LesrEq maybe_newline expr                            {$$ = mkBinOpNode(@$, Tok_LesrEq, $1, $4);}
    | expr Or maybe_newline expr                                {$$ = mkBinOpNode(@$, Tok_Or, $1, $4);}
    | expr And maybe_newline expr                               {$$ = mkBinOpNode(@$, Tok_And, $1, $4);}
    | expr ApplyR maybe_newline expr                            {$$ = mkBinOpNode(@$, '(', $4, mkTupleNode(@1, $1));}
    | expr ApplyL maybe_newline expr                            {$$ = mkBinOpNode(@$, '(', $1, mkTupleNode(@4, $4));}
    | expr Append maybe_newline expr                            {$$ = mkBinOpNode(@$, Tok_Append, $1, $4);}
    | expr Range maybe_newline expr                             {$$ = mkBinOpNode(@$, Tok_Range, $1, $4);}
    | expr In maybe_newline expr                                {$$ = mkBinOpNode(@$, Tok_In, $1, $4);}
    | expr Not In maybe_newline expr                            {$$ = mkUnOpNode(@$, Tok_Not, mkBinOpNode(@$, Tok_In, $1, $5));}
    | expr As maybe_newline small_type                          {$$ = mkBinOpNode(@$, Tok_As, $1, $4);}
    | val                                            %prec MED  {$$ = $1;}
    | unary_op                                                  {$$ = $1;}

    | function_call                                  %prec LOW  {$$ = mkFuncCallNode(@$, getRoot());}
    | usertype_node constructor_args                 %prec LOW  {$$ = mkTypeCastNode(@$, $1, mkTupleNode(@2, getRoot()));}
    | var '=' maybe_newline expr_or_block                       {$$ = mkVarAssignNode(@$, $1, $4); append_modifiers(mkModNode(@1, Tok_Let), $$);}
    | var '=' Mut maybe_newline expr_or_block                   {$$ = mkVarAssignNode(@$, $1, $5); append_modifiers(mkModNode(@1, Tok_Mut), $$);}
    | var '=' Global maybe_newline expr_or_block                {$$ = mkVarAssignNode(@$, $1, $5); append_modifiers(mkModNode(@1, Tok_Global), $$);}
    | var '=' Ante maybe_newline expr_or_block                  {$$ = mkVarAssignNode(@$, $1, $5); append_modifiers(mkModNode(@1, Tok_Ante), $$);}
    | var '=' Pub maybe_newline expr_or_block                   {$$ = mkVarAssignNode(@$, $1, $5); append_modifiers(mkModNode(@1, Tok_Pub), $$);}
    | var '=' Pri maybe_newline expr_or_block                   {$$ = mkVarAssignNode(@$, $1, $5); append_modifiers(mkModNode(@1, Tok_Pri), $$);}
    | var '=' Pro maybe_newline expr_or_block                   {$$ = mkVarAssignNode(@$, $1, $5); append_modifiers(mkModNode(@1, Tok_Pro), $$);}
    | var '=' Const maybe_newline expr_or_block                 {$$ = mkVarAssignNode(@$, $1, $5); append_modifiers(mkModNode(@1, Tok_Const), $$);}
    | var '=' preproc maybe_newline expr_or_block               {$$ = mkVarAssignNode(@$, $1, $5); append_modifiers($3, $$);}

    | expr AddEq maybe_newline expr                             {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '+', $1, $4), false);}
    | expr SubEq maybe_newline expr                             {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '-', $1, $4), false);}
    | expr MulEq maybe_newline expr                             {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '*', $1, $4), false);}
    | expr DivEq maybe_newline expr                             {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '/', $1, $4), false);}
    | expr Assign maybe_newline expr                            {$$ = mkVarAssignNode(@$, $1, $4);} /* All VarAssignNodes return unit values */
    | expr Assign maybe_newline block                           {$$ = mkVarAssignNode(@$, $1, $4);} /* All VarAssignNodes return unit values */
//    | modifiers expr  %prec Newline                {$$ = append_modifiers($1, $2);}

    /* this rule returns the original If for precedence reasons compared to its mirror rule in if_expr
     * that returns the elif node itself.  The former necessitates setElse to travel through the first IfNode's
     * internal linked list of elsenodes to find the last one and append the new elif */
    | expr Elif expr Then expr_or_jump  %prec MEDIF             {auto*elif = mkIfNode(@$, $3, $5, 0); $$ = setElse($1, elif);}
    | expr Else expr_or_jump                                    {$$ = setElse($1, $3);}

    | match_expr Newline expr                      %prec Match  {$$ = mkSeqNode(@$, $1, $3);}
    | match_expr Newline                                        {$$ = $1;}
    | expr Newline                                              {$$ = $1;}
    | expr Newline expr                                         {$$ = mkSeqNode(@$, $1, $3);}
    ;

%%

/* location parser error */
void yy::parser::error(const location& loc, const string& msg){
    if(++ante_parser_errcount > 5){
        std::cerr << "Too many errors, exiting.\n";
        exit(2);
    }
    location l = loc;
    ante::showError(msg.c_str(), l);
}

namespace ante {
    namespace parser {
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

        Node* name(Node *varNode){
            char* name = strdup(((VarNode*)varNode)->name.c_str());
            delete varNode;
            return (Node*)name;
        }
    }
}
