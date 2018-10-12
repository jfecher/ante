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

    namespace parser {
        struct TypeNode;

        Node* externCName(Node *n);
        vector<unique_ptr<TypeNode>> toOwnedVec(Node *tn);
        vector<unique_ptr<TypeNode>> concat(vector<unique_ptr<TypeNode>>&& l, Node *tn);
    }
}


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
%token Assign NotEq AddEq SubEq MulEq DivEq GrtrEq LesrEq
%token Or And Range RArrow ApplyL ApplyR Append New Not Is Isnt

/* literals */
%token True False
%token IntLit FltLit StrLit CharLit

/* keywords */
%token Return
%token If Then Elif Else
%token For While Do In
%token Continue Break Import
%token Let Match With Ref Type
%token Trait Fun Ext Block As Self

/* modifiers */
%token Pub Pri Pro Const
%token Mut Global Ante

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
%left STMT Fun Let Import Return Ext While For Match Trait If Break Continue Type
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
%left Assign AddEq SubEq MulEq DivEq
%left ';'
%left MODIFIER Pub Pri Pro Const Mut Global Ante

%right ApplyL
%left ApplyR

%nonassoc '!'

%left Or
%left And
%left Not
%left '=' Is Isnt NotEq GrtrEq LesrEq '<' '>'

%left In
%left Append
%left Range

%left ':'

%left '+' '-'
%left '*' '/' '%'
%right '^'

%left As
%left '#'
%left '@' New '&' Ref
%left TYPE UserType TypeVar I8 I16 I32 I64 U8 U16 U32 U64 Isz Usz F16 F32 F64 C8 C32 Bool Void
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

top_level_expr: modifier maybe_newline top_level_expr     {$$ = append_modifier($1, $3);}
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

pointer_type: Ref bounded_type_expr  {$$ = mkTypeNode(@$, TT_Ptr, (char*)"", $2);}
            ;

fn_type: '(' ')'       RArrow bounded_type_expr  {$$ = mkTypeNode(@$, TT_Function, (char*)"", $4);}
       | tuple_type    RArrow bounded_type_expr  {setNext($3, $1); $$ = mkTypeNode(@$, TT_Function, (char*)"", $3);}
       | lit_type      RArrow bounded_type_expr  {setNext($3, $1); $$ = mkTypeNode(@$, TT_Function, (char*)"", $3);}
       | pointer_type  RArrow bounded_type_expr  {setNext($3, $1); $$ = mkTypeNode(@$, TT_Function, (char*)"", $3);}
       | arr_type      RArrow bounded_type_expr  {setNext($3, $1); $$ = mkTypeNode(@$, TT_Function, (char*)"", $3);}
       ;

/* val is used here instead of intlit due to parse conflicts, but only intlit is allowed */
arr_type: '[' val bounded_type_expr ']' {$3->next.reset($2);
                                 $$ = mkTypeNode(@$, TT_Array, (char*)"", $3);}
        | '[' type_expr ']'     {$2->next.reset(mkIntLitNode(@$, (char*)"0"));
                                 $$ = mkTypeNode(@$, TT_Array, (char*)"", $2);}
        ;

tuple_type: '(' type_expr ')'      {$$ = $2;}
          | '(' type_expr ',' ')'  {$$ = mkTypeNode(@$, TT_Tuple, (char*)"", $2);}
          ;

type: type non_generic_type  %prec STMT  {$$ = $1; ((TypeNode*)$1)->params.emplace_back((TypeNode*)$2);}
    | non_generic_type       %prec STMT  {$$ = $1;}
    ;

non_generic_type: pointer_type  %prec STMT  {$$ = $1;}
                | arr_type      %prec STMT  {$$ = $1;}
                | fn_type       %prec STMT  {$$ = $1;}
                | lit_type      %prec STMT  {$$ = $1;}
                | tuple_type    %prec STMT  {$$ = $1;}
                ;

bounded_type_expr: modifier bounded_type_expr   {$$ = append_modifier($1, $2);}
                 | type_expr  {$$ = $1;}
                 ;

type_expr_: type_expr_ ',' type  %prec '*' {$$ = setNext($1, $3);}
          | type                 %prec '*' {$$ = setRoot($1);}
          ;

type_expr__: type_expr_  %prec MED {Node* tmp = getRoot();
                          if(tmp == $1){//singular type, first type in list equals the last
                              $$ = tmp;
                          }else{ //tuple type
                              $$ = mkTypeNode(@$, TT_Tuple, (char*)"", tmp);
                          }
                         }

type_expr: type_expr__    {$$ = $1;}
         ;

preproc: '!' '[' expr ']'         {$$ = mkCompilerDirective(@$, $3);}
       | '!' var                        {$$ = mkCompilerDirective(@$, $2);}
       ;

modifier: Pub      {$$ = mkModNode(@$, Tok_Pub);}
        | Pri      {$$ = mkModNode(@$, Tok_Pri);}
        | Pro      {$$ = mkModNode(@$, Tok_Pro);}
        | Const    {$$ = mkModNode(@$, Tok_Const);}
        | Mut      {$$ = mkModNode(@$, Tok_Mut);}
        | Global   {$$ = mkModNode(@$, Tok_Global);}
        | Ante     {$$ = mkModNode(@$, Tok_Ante);}
        | Let      {$$ = mkModNode(@$, Tok_Let);}
        | preproc  %prec MODIFIER {$$ = $1;}
        ;


trait_decl: Trait usertype Indent trait_fn_list Unindent  {$$ = mkTraitNode(@$, (char*)$2, $4); free($2);}
          ;

trait_fn_list: _trait_fn_list maybe_newline {$$ = getRoot();}

_trait_fn_list: _trait_fn_list Newline trait_fn  {$$ = setNext($1, $3);}
              | trait_fn                         {$$ = setRoot($1);}
              ;


trait_fn_no_mods: Fun fn_name ':' params RArrow bounded_type_expr                 {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*ret_ty*/$6,                                  /*params*/$4, /*body*/0);}
                | Fun fn_name ':' RArrow bounded_type_expr                        {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*ret_ty*/$5,                                  /*params*/0,  /*body*/0);}
                | Fun fn_name ':' params                                  {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$4, /*body*/0);}
                | Fun fn_name ':'                                         {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/0);}
                ;

trait_fn: modifier maybe_newline trait_fn  {$$ = append_modifier($1, $3);}
        | trait_fn_no_mods
        ;


typevar_list: typevar_list typevar  %prec LOW  {$$ = setNext($1, mkTypeNode(@$, TT_TypeVar, (char*)$2)); free($2);}
            | typevar               %prec LOW  {$$ = setRoot(mkTypeNode(@$, TT_TypeVar, (char*)$1)); free($1);}
            ;

generic_params: typevar_list  %prec LOW {$$ = getRoot();}
              ;


data_decl: Type usertype generic_params '=' type_decl_block                 {$$ = mkDataDeclNode(@$, (char*)$2, $3, $5, false); free($2);}
         | Type usertype '=' type_decl_block                                {$$ = mkDataDeclNode(@$, (char*)$2,  0, $4, false); free($2);}
         | Type usertype generic_params Is type_decl_block                  {$$ = mkDataDeclNode(@$, (char*)$2, $3, $5, true); free($2);}
         | Type usertype Is type_decl_block                                 {$$ = mkDataDeclNode(@$, (char*)$2,  0, $4, true); free($2);}
         ;

type_decl_list: type_decl_list Newline params                       {$$ = setNext($1, $3);}
              | type_decl_list Newline explicit_tagged_union_list   {$$ = setNext($1, getRoot());}
              | params                                              {$$ = setRoot($1);}
              | explicit_tagged_union_list                          {$$ = $1;} /* leave root set */
              ;

/* tagged union list with mandatory '|' before first element */
explicit_tagged_union_list: explicit_tagged_union_list '|' usertype bounded_type_expr   %prec STMT  {$$ = mkNamedValNode(@$, mkVarNode(@3, (char*)$3), mkTypeNode(@4, TT_TaggedUnion, (char*)"", $4), $1); free($3);}
                          | explicit_tagged_union_list '|' usertype             %prec STMT  {$$ = mkNamedValNode(@$, mkVarNode(@3, (char*)$3), mkTypeNode(@3, TT_TaggedUnion, (char*)"",  0), $1); free($3);}
                          | '|' usertype bounded_type_expr                              %prec STMT  {$$ = mkNamedValNode(@$, mkVarNode(@2, (char*)$2), mkTypeNode(@3, TT_TaggedUnion, (char*)"", $3),  0); free($2);}
                          | '|' usertype                                        %prec STMT  {$$ = mkNamedValNode(@$, mkVarNode(@2, (char*)$2), mkTypeNode(@2, TT_TaggedUnion, (char*)"",  0),  0); free($2);}

type_decl_block: Indent type_decl_list Unindent  {$$ = getRoot();}
               | params               %prec STMT  {$$ = $1;}
               | bounded_type_expr            %prec STMT  {$$ = mkNamedValNode(@$, mkVarNode(@$, (char*)""), $1, 0);}
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
_params: _params ',' bounded_type_expr ident_list {$$ = mkNamedValNode(@$, $4, $3, $1);}
       | _params ',' ident_list                   {$$ = mkNamedValNode(@$, $3, mkInferredTypeNode(@3), $1);}
       | bounded_type_expr ident_list             {$$ = mkNamedValNode(@$, $2, $1, 0);}
       | ident_list                               {$$ = mkNamedValNode(@$, $1, mkInferredTypeNode(@$), 0);}
//       | Self                                     {$$ = mkNamedValNode(@$, mkVarNode(@$, (char*)"self"), (Node*)1, 0);}
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
  | '^'    {$$ = (Node*)"^";}
  | '<'    {$$ = (Node*)"<";}
  | '>'    {$$ = (Node*)">";}
  | '.'    {$$ = (Node*)".";}
  | ';'    {$$ = (Node*)";";}
  | '#'    {$$ = (Node*)"#";}
  | Assign {$$ = (Node*)":=";}
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
  | Is     {$$ = (Node*)"is";}
  | Isnt   {$$ = (Node*)"isnt";}
  | As     {$$ = (Node*)"as";}
  ;

/* NOTE: lextxt contents from fn_name and the mangleFn result are freed in the call to mkFuncDeclNode */
fn_ext_def: Fun bounded_type_expr '.' fn_name ':' params RArrow bounded_type_expr block                              {$$ = mkExtNode(@4, $2, mkFuncDeclNode(@$, /*fn_name*/$4, /*ret_ty*/$8,                                  /*params*/$6, /*body*/$9)); }
          | Fun bounded_type_expr '.' fn_name ':' RArrow bounded_type_expr block                                     {$$ = mkExtNode(@4, $2, mkFuncDeclNode(@$, /*fn_name*/$4, /*ret_ty*/$7,                                  /*params*/0,  /*body*/$8)); }
          | Fun bounded_type_expr '.' fn_name ':' params block                                               {$$ = mkExtNode(@4, $2, mkFuncDeclNode(@$, /*fn_name*/$4, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$6, /*body*/$7)); }
          | Fun bounded_type_expr '.' fn_name ':' block                                                      {$$ = mkExtNode(@4, $2, mkFuncDeclNode(@$, /*fn_name*/$4, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/$6)); }
          ;

fn_ext_inferredRet: Fun bounded_type_expr '.' fn_name ':' params '=' expr       %prec Newline                        {$$ = mkExtNode(@$, $2, mkFuncDeclNode(@4, /*fn_name*/$4, /*ret_ty*/0, /*params*/$6, /*body*/$8)); }
                  | Fun bounded_type_expr '.' fn_name ':' '=' expr              %prec Newline                        {$$ = mkExtNode(@$, $2, mkFuncDeclNode(@4, /*fn_name*/$4, /*ret_ty*/0, /*params*/0,  /*body*/$7)); }
                  | Fun bounded_type_expr '.' fn_name Assign  expr              %prec Newline                        {$$ = mkExtNode(@$, $2, mkFuncDeclNode(@4, /*fn_name*/$4, /*ret_ty*/0, /*params*/0,  /*body*/$6)); }
                  ;

fn_def: Fun fn_name ':' params RArrow bounded_type_expr block                              {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*ret_ty*/$6,                                  /*params*/$4, /*body*/$7);}
      | Fun fn_name ':' RArrow bounded_type_expr block                                     {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*ret_ty*/$5,                                  /*params*/0,  /*body*/$6);}
      | Fun fn_name ':' params block                                               {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$4, /*body*/$5);}
      | Fun fn_name ':' block                                                      {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/$4);}
      ;

fn_inferredRet: Fun fn_name ':' params '=' expr     %prec Newline                          {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*ret_ty*/0, /*params*/$4, /*body*/$6);}
              | Fun fn_name ':' '=' expr            %prec Newline                          {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*ret_ty*/0, /*params*/0,  /*body*/$5);}
              | Fun fn_name Assign  expr            %prec Newline                          {$$ = mkFuncDeclNode(@2, /*fn_name*/$2, /*ret_ty*/0, /*params*/0,  /*body*/$4);}
              ;

fn_decl: Fun fn_name ':' params RArrow bounded_type_expr    %prec Fun     {$$ = mkFuncDeclNode(@2, /*fn_name*/externCName($2), /*ret_ty*/$6,                                  /*params*/$4, /*body*/0);}
       | Fun fn_name ':' RArrow bounded_type_expr           %prec Fun     {$$ = mkFuncDeclNode(@2, /*fn_name*/externCName($2), /*ret_ty*/$5,                                  /*params*/0,  /*body*/0);}
       | Fun fn_name ':' params                             %prec Fun     {$$ = mkFuncDeclNode(@2, /*fn_name*/externCName($2), /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$4, /*body*/0);}
       | Fun fn_name ':'                                    %prec Fun     {$$ = mkFuncDeclNode(@2, /*fn_name*/externCName($2), /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/0);}
       ;

fn_ext_decl: Fun bounded_type_expr '.' fn_name ':' params RArrow bounded_type_expr     %prec Fun    {$$ = mkExtNode(@2, $2, mkFuncDeclNode(@$, /*fn_name*/externCName($4), /*ret_ty*/$8,                                  /*params*/$6, /*body*/0));}
           | Fun bounded_type_expr '.' fn_name ':' RArrow bounded_type_expr            %prec Fun    {$$ = mkExtNode(@2, $2, mkFuncDeclNode(@$, /*fn_name*/externCName($4), /*ret_ty*/$7,                                  /*params*/0,  /*body*/0));}
           | Fun bounded_type_expr '.' fn_name ':' params                              %prec Fun    {$$ = mkExtNode(@2, $2, mkFuncDeclNode(@$, /*fn_name*/externCName($4), /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/$6, /*body*/0));}
           | Fun bounded_type_expr '.' fn_name ':'                                     %prec Fun    {$$ = mkExtNode(@2, $2, mkFuncDeclNode(@$, /*fn_name*/externCName($4), /*ret_ty*/mkTypeNode(@$, TT_Void, (char*)""),  /*params*/0,  /*body*/0));}
           ;

fn_lambda: Fun params '=' expr                              %prec Fun  {$$ = mkFuncDeclNode(@$, /*fn_name*/(Node*)strdup(""), /*ret_ty*/0,  /*params*/$2, /*body*/$4);}
         | Fun '=' expr                                     %prec Fun  {$$ = mkFuncDeclNode(@$, /*fn_name*/(Node*)strdup(""), /*ret_ty*/0,  /*params*/0,  /*body*/$3);}
         ;



ret_expr: Return expr {$$ = mkRetNode(@$, $2);}
        ;


extension: Ext bounded_type_expr Indent fn_list Unindent {$$ = mkExtNode(@$, $2, $4);}
         | Ext bounded_type_expr ':' usertype_list Indent fn_list Unindent {$$ = mkExtNode(@$, $2, $6, $4);}
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


while_loop: While expr Do expr  %prec While  {$$ = mkWhileNode(@$, $2, $4);}
          ;

/*            v---v this should be later changed to pattern  */
for_loop: For ident In expr Do expr  %prec For  {$$ = mkForNode(@$, $2, $4, $6); free($2);}


break: Break expr  %prec Break  {$$ = mkJumpNode(@$, Tok_Break, $2);}
     | Break                    {$$ = mkJumpNode(@$, Tok_Break, mkIntLitNode(@$, (char*)"1"));}
     ;


continue: Continue expr  %prec Continue  {$$ = mkJumpNode(@$, Tok_Continue, $2);}
        | Continue                       {$$ = mkJumpNode(@$, Tok_Continue, mkIntLitNode(@$, (char*)"1"));}
        ;


match: '|' expr RArrow expr              {$$ = mkMatchBranchNode(@$, $2, $4);}
     | '|' usertype RArrow expr  %prec Match {$$ = mkMatchBranchNode(@$, mkTypeNode(@2, TT_Data, (char*)$2), $4); free($2);}
     ;


match_expr: Match expr With Newline match  {$$ = mkMatchNode(@$, $2, $5);}
          | match_expr Newline match       {$$ = addMatch($1, $3);}
          ;

fn_brackets: '{' expr_list '}' {$$ = mkTupleNode(@$, $2);}
           | '{' '}'           {$$ = mkTupleNode(@$, 0);}
           ;

if_expr: If expr Then expr_or_jump                %prec MEDIF  {$$ = mkIfNode(@$, $2, $4, 0);}
       | if_expr Elif expr Then expr_or_jump      %prec MEDIF  {auto*elif = mkIfNode(@$, $3, $5, 0); setElse($1, elif); $$ = elif;}
       | if_expr Else expr_or_jump                             {$$ = setElse($1, $3);}
       ;

var: ident  %prec Ident {$$ = mkVarNode(@$, (char*)$1); free($1);}
   ;


val_no_decl: '(' expr ')'            {$$ = $2;}
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
           | while_loop              {$$ = $1;}
           | for_loop                {$$ = $1;}
           | if_expr     %prec STMT  {$$ = $1;}
           | match_expr  %prec LOW   {$$ = $1;}
           | explicit_block          {$$ = $1;}
           | type_expr  %prec LOW
           | block
           | Let var '=' maybe_newline var_decl_expr      %prec Newline   {$$ = mkVarAssignNode(@$, $2, $5); append_modifier(mkModNode(@1, Tok_Let), $$);}
           | Mut var '=' maybe_newline var_decl_expr      %prec Newline   {$$ = mkVarAssignNode(@$, $2, $5); append_modifier(mkModNode(@1, Tok_Mut), $$);}
           | Global var '=' maybe_newline var_decl_expr   %prec Newline   {$$ = mkVarAssignNode(@$, $2, $5); append_modifier(mkModNode(@1, Tok_Global), $$);}
           | Ante var '=' maybe_newline var_decl_expr     %prec Newline   {$$ = mkVarAssignNode(@$, $2, $5); append_modifier(mkModNode(@1, Tok_Ante), $$);}
           | Pub var '=' maybe_newline var_decl_expr      %prec Newline   {$$ = mkVarAssignNode(@$, $2, $5); append_modifier(mkModNode(@1, Tok_Pub), $$);}
           | Pri var '=' maybe_newline var_decl_expr      %prec Newline   {$$ = mkVarAssignNode(@$, $2, $5); append_modifier(mkModNode(@1, Tok_Pri), $$);}
           | Pro var '=' maybe_newline var_decl_expr      %prec Newline   {$$ = mkVarAssignNode(@$, $2, $5); append_modifier(mkModNode(@1, Tok_Pro), $$);}
           | Const var '=' maybe_newline var_decl_expr    %prec Newline   {$$ = mkVarAssignNode(@$, $2, $5); append_modifier(mkModNode(@1, Tok_Const), $$);}
           | preproc var '=' maybe_newline var_decl_expr  %prec Newline   {$$ = mkVarAssignNode(@$, $2, $5); append_modifier($1, $$);}
           ;

var_decl_expr: expr   %prec Newline

val: val_no_decl
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


unary_op: '@' expr                              {$$ = mkUnOpNode(@$, '@', $2);}
        | '&' expr                              {$$ = mkUnOpNode(@$, '&', $2);}
        | New expr                              {$$ = mkUnOpNode(@$, Tok_New, $2);}
        | Not expr                              {$$ = mkUnOpNode(@$, Tok_Not, $2);}
        | non_generic_type expr      %prec TYPE {$$ = mkTypeCastNode(@$, $1, $2);}
        | explicit_generic_type expr %prec TYPE {$$ = mkTypeCastNode(@$, $1, $2);}
        ;

explicit_generic_type: non_generic_type '<' type_list '>'    %prec TYPE {$$ = $1; ((TypeNode*)$1)->params = toOwnedVec(getRoot());}
                     ;

type_list: type_list ',' type  %prec TYPE {$$ = setNext($1, $3);}
         | type                %prec TYPE {$$ = setRoot($1);}
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
            | '-' expr_no_decl                                                 {$$ = mkUnOpNode(@$, '-', $2);}
            | expr_no_decl '-' Newline expr_no_decl                            {$$ = mkBinOpNode(@$, '-', $1, $4);}
            | expr_no_decl '*' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '*', $1, $4);}
            | expr_no_decl '/' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '/', $1, $4);}
            | expr_no_decl '%' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '%', $1, $4);}
            | expr_no_decl '^' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '^', $1, $4);}
            | expr_no_decl '<' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '<', $1, $4);}
            | expr_no_decl '>' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '>', $1, $4);}
            | type_expr '.' maybe_newline var                                  {$$ = mkBinOpNode(@$, '.', $1, $4);}
            | type_expr '.' maybe_newline type_expr                            {$$ = mkBinOpNode(@$, '.', $1, $4);}
            | expr_no_decl '.' maybe_newline var                               {$$ = mkBinOpNode(@$, '.', $1, $4);}
            | expr_no_decl '.' maybe_newline type_expr                         {$$ = mkBinOpNode(@$, '.', $1, $4);}
            | expr_no_decl ';' maybe_newline expr_no_decl                      {$$ = mkSeqNode(@$, $1, $4);}
            | expr_no_decl '#' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '#', $1, $4);}
            | expr_no_decl '=' maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, '=', $1, $4);}
            | expr_no_decl Is maybe_newline expr_no_decl                       {$$ = mkBinOpNode(@$, Tok_Is, $1, $4);}
            | expr_no_decl Isnt maybe_newline expr_no_decl                     {$$ = mkBinOpNode(@$, Tok_Isnt, $1, $4);}
            | expr_no_decl NotEq maybe_newline expr_no_decl                    {$$ = mkBinOpNode(@$, Tok_NotEq, $1, $4);}
            | expr_no_decl GrtrEq maybe_newline expr_no_decl                   {$$ = mkBinOpNode(@$, Tok_GrtrEq, $1, $4);}
            | expr_no_decl LesrEq maybe_newline expr_no_decl                   {$$ = mkBinOpNode(@$, Tok_LesrEq, $1, $4);}
            | expr_no_decl Or maybe_newline expr_no_decl                       {$$ = mkBinOpNode(@$, Tok_Or, $1, $4);}
            | expr_no_decl And maybe_newline expr_no_decl                      {$$ = mkBinOpNode(@$, Tok_And, $1, $4);}
            | expr_no_decl ApplyR maybe_newline expr_no_decl                   {$$ = mkBinOpNode(@$, '(', $4, $1);}
            | expr_no_decl ApplyL maybe_newline expr_no_decl                   {$$ = mkBinOpNode(@$, '(', $1, $4);}
            | expr_no_decl Append maybe_newline expr_no_decl                   {$$ = mkBinOpNode(@$, Tok_Append, $1, $4);}
            | expr_no_decl Range maybe_newline expr_no_decl                    {$$ = mkBinOpNode(@$, Tok_Range, $1, $4);}
            | expr_no_decl In maybe_newline expr_no_decl                       {$$ = mkBinOpNode(@$, Tok_In, $1, $4);}
            | expr_no_decl Not In maybe_newline expr_no_decl                   {$$ = mkUnOpNode(@$, Tok_Not, mkBinOpNode(@$, Tok_In, $1, $5));}
            | expr_no_decl As maybe_newline bounded_type_expr                  {$$ = mkBinOpNode(@$, Tok_As, $1, $4);}
            | expr_no_decl fn_brackets                                         {$$ = mkBinOpNode(@$, '(', $1, $2);}
            | expr_no_decl arg_list                                            {$$ = mkBinOpNode(@$, '(', $1, $2);}
            | val_no_decl                                           %prec MED  {$$ = $1;}

            | expr_no_decl AddEq maybe_newline expr_no_decl             {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '+', $1, $4), false);}
            | expr_no_decl SubEq maybe_newline expr_no_decl             {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '-', $1, $4), false);}
            | expr_no_decl MulEq maybe_newline expr_no_decl             {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '*', $1, $4), false);}
            | expr_no_decl DivEq maybe_newline expr_no_decl             {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '/', $1, $4), false);}
            | expr_no_decl Assign maybe_newline expr_no_decl            {$$ = mkVarAssignNode(@$, $1, $4);} /* All VarAssignNodes return void values */
            | modifier maybe_newline expr_no_decl  %prec Newline        {$$ = append_modifier($1, $3);}


            /* this rule returns the original If for precedence reasons compared to its mirror rule in if_expr
             * that returns the elif node itself.  The former necessitates setElse to travel through the first IfNode's
             * internal linked list of elsenodes to find the last one and append the new elif */
            | expr_no_decl Elif expr Then expr_no_decl_or_jump    %prec MEDIF  {auto*elif = mkIfNode(@$, $3, $5, 0); $$ = setElse($1, elif);}
            | expr_no_decl Else expr_no_decl_or_jump                        %prec Else {$$ = setElse($1, $3);}

            | match_expr Newline expr_no_decl                      %prec Match  {$$ = mkSeqNode(@$, $1, $3);}
            | match_expr Newline                                   %prec LOW    {$$ = $1;}
            ;


expr_or_jump: expr  %prec MEDIF
            | break
            | continue
            | ret_expr
            ;

expr: expr '+' maybe_newline expr                    {$$ = mkBinOpNode(@$, '+', $1, $4);}
    | expr '-' expr                                  {$$ = mkBinOpNode(@$, '-', $1, $3);}
    | '-' expr                                                  {$$ = mkUnOpNode(@$, '-', $2);}
    | expr '-' Newline expr                          {$$ = mkBinOpNode(@$, '-', $1, $4);}
    | expr '*' maybe_newline expr                    {$$ = mkBinOpNode(@$, '*', $1, $4);}
    | expr '/' maybe_newline expr                    {$$ = mkBinOpNode(@$, '/', $1, $4);}
    | expr '%' maybe_newline expr                    {$$ = mkBinOpNode(@$, '%', $1, $4);}
    | expr '^' maybe_newline expr                    {$$ = mkBinOpNode(@$, '^', $1, $4);}
    | expr '<' maybe_newline expr                    {$$ = mkBinOpNode(@$, '<', $1, $4);}
    | expr '>' maybe_newline expr                    {$$ = mkBinOpNode(@$, '>', $1, $4);}
    | type_expr '.' maybe_newline var                                      {$$ = mkBinOpNode(@$, '.', $1, $4);}
    | type_expr '.' maybe_newline type_expr                                {$$ = mkBinOpNode(@$, '.', $1, $4);}
    | expr '.' maybe_newline var                                {$$ = mkBinOpNode(@$, '.', $1, $4);}
    | expr '.' maybe_newline type_expr                          {$$ = mkBinOpNode(@$, '.', $1, $4);}
    | expr ';' maybe_newline expr                    {$$ = mkSeqNode(@$, $1, $4);}
    | expr '#' maybe_newline expr                    {$$ = mkBinOpNode(@$, '#', $1, $4);}
    | expr '=' maybe_newline expr                    {$$ = mkBinOpNode(@$, '=', $1, $4);}
    | expr Is maybe_newline expr                     {$$ = mkBinOpNode(@$, Tok_Is, $1, $4);}
    | expr Isnt maybe_newline expr                   {$$ = mkBinOpNode(@$, Tok_Isnt, $1, $4);}
    | expr NotEq maybe_newline expr                  {$$ = mkBinOpNode(@$, Tok_NotEq, $1, $4);}
    | expr GrtrEq maybe_newline expr                 {$$ = mkBinOpNode(@$, Tok_GrtrEq, $1, $4);}
    | expr LesrEq maybe_newline expr                 {$$ = mkBinOpNode(@$, Tok_LesrEq, $1, $4);}
    | expr Or maybe_newline expr                     {$$ = mkBinOpNode(@$, Tok_Or, $1, $4);}
    | expr And maybe_newline expr                    {$$ = mkBinOpNode(@$, Tok_And, $1, $4);}
    | expr ApplyR maybe_newline expr                 {$$ = mkBinOpNode(@$, '(', $4, $1);}
    | expr ApplyL maybe_newline expr                 {$$ = mkBinOpNode(@$, '(', $1, $4);}
    | expr Append maybe_newline expr                 {$$ = mkBinOpNode(@$, Tok_Append, $1, $4);}
    | expr Range maybe_newline expr                  {$$ = mkBinOpNode(@$, Tok_Range, $1, $4);}
    | expr In maybe_newline expr                     {$$ = mkBinOpNode(@$, Tok_In, $1, $4);}
    | expr Not In maybe_newline expr                 {$$ = mkUnOpNode(@$, Tok_Not, mkBinOpNode(@$, Tok_In, $1, $5));}
    | expr As maybe_newline bounded_type_expr                   {$$ = mkBinOpNode(@$, Tok_As, $1, $4);}
    | expr fn_brackets                                          {$$ = mkBinOpNode(@$, '(', $1, $2);}
    | expr arg_list                                             {$$ = mkBinOpNode(@$, '(', $1, $2);}
    | val                                                       %prec MED  {$$ = $1;}

    | expr AddEq maybe_newline expr             {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '+', $1, $4), false);}
    | expr SubEq maybe_newline expr             {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '-', $1, $4), false);}
    | expr MulEq maybe_newline expr             {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '*', $1, $4), false);}
    | expr DivEq maybe_newline expr             {$$ = mkVarAssignNode(@$, $1, mkBinOpNode(@$, '/', $1, $4), false);}
    | expr Assign maybe_newline expr            {$$ = mkVarAssignNode(@$, $1, $4);} /* All VarAssignNodes return void values */
    | modifier maybe_newline expr  %prec Newline        {$$ = append_modifier($1, $3);}

    /* this rule returns the original If for precedence reasons compared to its mirror rule in if_expr
     * that returns the elif node itself.  The former necessitates setElse to travel through the first IfNode's
     * internal linked list of elsenodes to find the last one and append the new elif */
    | expr Elif expr Then expr_or_jump  %prec MEDIF             {auto*elif = mkIfNode(@$, $3, $5, 0); $$ = setElse($1, elif);}
    | expr Else expr_or_jump                                    {$$ = setElse($1, $3);}

    | match_expr Newline expr                      %prec Match  {$$ = mkSeqNode(@$, $1, $3);}
    | match_expr Newline                                                   {$$ = $1;}
    | expr Newline                                              {$$ = $1;}
    | expr Newline expr                              {$$ = mkSeqNode(@$, $1, $3);}
    ;

%%

/* location parser error */
void yy::parser::error(const location& loc, const string& msg){
    location l = loc;
    ante::error(msg.c_str(), l);
}

namespace ante {
    namespace parser {
        Node* externCName(Node *n){
            char *str = (char*)n;
            size_t len = strlen(str);
            char *c = (char*)realloc(str, len+2);
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
    }
}
