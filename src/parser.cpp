// A Bison parser, made by GNU Bison 3.0.4.

// Skeleton implementation for Bison GLR parsers in C

// Copyright (C) 2002-2015 Free Software Foundation, Inc.

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

// As a special exception, you may create a larger work that contains
// part or all of the Bison parser skeleton and distribute that work
// under terms of your choice, so long as that work isn't itself a
// parser generator using the skeleton or a modified version thereof
// as a parser skeleton.  Alternatively, if you modify or redistribute
// the parser skeleton itself, you may (at your option) remove this
// special exception, which will cause the skeleton and the resulting
// Bison output files to be licensed under the GNU General Public
// License without this special exception.

// This special exception was added by the Free Software Foundation in
// version 2.2 of Bison.

/* C GLR parser skeleton written by Paul Hilfinger.  */

/* Identify Bison output.  */
#define YYBISON 1

/* Bison version.  */
#define YYBISON_VERSION "3.0.4"

/* Skeleton name.  */
#define YYSKELETON_NAME "glr.cc"

/* Pure parsers.  */
#define YYPURE 1






/* First part of user declarations.  */
#line 1 "src/syntax.y" // glr.c:240

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


#line 84 "src/parser.cpp" // glr.c:240

# ifndef YY_NULLPTR
#  if defined __cplusplus && 201103L <= __cplusplus
#   define YY_NULLPTR nullptr
#  else
#   define YY_NULLPTR 0
#  endif
# endif

#include "yyparser.h"

/* Enabling verbose error messages.  */
#ifdef YYERROR_VERBOSE
# undef YYERROR_VERBOSE
# define YYERROR_VERBOSE 1
#else
# define YYERROR_VERBOSE 1
#endif

/* Default (constant) value used for initialization for null
   right-hand sides.  Unlike the standard yacc.c template, here we set
   the default value of $$ to a zeroed-out value.  Since the default
   value is undefined, this behavior is technically correct.  */
static YYSTYPE yyval_default;
static YYLTYPE yyloc_default
# if defined YYLTYPE_IS_TRIVIAL && YYLTYPE_IS_TRIVIAL
  = { 1, 1, 1, 1 }
# endif
;

/* Copy the second part of user declarations.  */
#line 116 "src/parser.cpp" // glr.c:263
/* YYLLOC_DEFAULT -- Set CURRENT to span from RHS[1] to RHS[N].
   If N is 0, then set CURRENT to the empty location which ends
   the previous symbol: RHS[0] (always defined).  */

# ifndef YYLLOC_DEFAULT
#  define YYLLOC_DEFAULT(Current, Rhs, N)                               \
    do                                                                  \
      if (N)                                                            \
        {                                                               \
          (Current).begin  = YYRHSLOC (Rhs, 1).begin;                   \
          (Current).end    = YYRHSLOC (Rhs, N).end;                     \
        }                                                               \
      else                                                              \
        {                                                               \
          (Current).begin = (Current).end = YYRHSLOC (Rhs, 0).end;      \
        }                                                               \
    while (/*CONSTCOND*/ false)
# endif

#define YYRHSLOC(Rhs, K) ((Rhs)[K].yystate.yyloc)
static void yyerror (const yy::parser::location_type *yylocationp, yy::parser& yyparser, const char* msg);
#line 138 "src/parser.cpp" // glr.c:263

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifndef YY_
# if defined YYENABLE_NLS && YYENABLE_NLS
#  if ENABLE_NLS
#   include <libintl.h> /* INFRINGES ON USER NAME SPACE */
#   define YY_(Msgid) dgettext ("bison-runtime", Msgid)
#  endif
# endif
# ifndef YY_
#  define YY_(Msgid) Msgid
# endif
#endif

#ifndef YYFREE
# define YYFREE free
#endif
#ifndef YYMALLOC
# define YYMALLOC malloc
#endif
#ifndef YYREALLOC
# define YYREALLOC realloc
#endif

#define YYSIZEMAX ((size_t) -1)

#ifdef __cplusplus
   typedef bool yybool;
#else
   typedef unsigned char yybool;
#endif
#define yytrue 1
#define yyfalse 0

#ifndef YYSETJMP
# include <setjmp.h>
# define YYJMP_BUF jmp_buf
# define YYSETJMP(Env) setjmp (Env)
/* Pacify clang.  */
# define YYLONGJMP(Env, Val) (longjmp (Env, Val), YYASSERT (0))
#endif

#ifndef YY_ATTRIBUTE
# if (defined __GNUC__                                               \
      && (2 < __GNUC__ || (__GNUC__ == 2 && 96 <= __GNUC_MINOR__)))  \
     || defined __SUNPRO_C && 0x5110 <= __SUNPRO_C
#  define YY_ATTRIBUTE(Spec) __attribute__(Spec)
# else
#  define YY_ATTRIBUTE(Spec) /* empty */
# endif
#endif

#ifndef YY_ATTRIBUTE_PURE
# define YY_ATTRIBUTE_PURE   YY_ATTRIBUTE ((__pure__))
#endif

#ifndef YY_ATTRIBUTE_UNUSED
# define YY_ATTRIBUTE_UNUSED YY_ATTRIBUTE ((__unused__))
#endif

#if !defined _Noreturn \
     && (!defined __STDC_VERSION__ || __STDC_VERSION__ < 201112)
# if defined _MSC_VER && 1200 <= _MSC_VER
#  define _Noreturn __declspec (noreturn)
# else
#  define _Noreturn YY_ATTRIBUTE ((__noreturn__))
# endif
#endif

/* Suppress unused-variable warnings by "using" E.  */
#if ! defined lint || defined __GNUC__
# define YYUSE(E) ((void) (E))
#else
# define YYUSE(E) /* empty */
#endif

#if defined __GNUC__ && 407 <= __GNUC__ * 100 + __GNUC_MINOR__
/* Suppress an incorrect diagnostic about yylval being uninitialized.  */
# define YY_IGNORE_MAYBE_UNINITIALIZED_BEGIN \
    _Pragma ("GCC diagnostic push") \
    _Pragma ("GCC diagnostic ignored \"-Wuninitialized\"")\
    _Pragma ("GCC diagnostic ignored \"-Wmaybe-uninitialized\"")
# define YY_IGNORE_MAYBE_UNINITIALIZED_END \
    _Pragma ("GCC diagnostic pop")
#else
# define YY_INITIAL_VALUE(Value) Value
#endif
#ifndef YY_IGNORE_MAYBE_UNINITIALIZED_BEGIN
# define YY_IGNORE_MAYBE_UNINITIALIZED_BEGIN
# define YY_IGNORE_MAYBE_UNINITIALIZED_END
#endif
#ifndef YY_INITIAL_VALUE
# define YY_INITIAL_VALUE(Value) /* Nothing. */
#endif


#ifndef YYASSERT
# define YYASSERT(Condition) ((void) ((Condition) || (abort (), 0)))
#endif

/* YYFINAL -- State number of the termination state.  */
#define YYFINAL  4
/* YYLAST -- Last index in YYTABLE.  */
#define YYLAST   2154

/* YYNTOKENS -- Number of terminals.  */
#define YYNTOKENS  94
/* YYNNTS -- Number of nonterminals.  */
#define YYNNTS  60
/* YYNRULES -- Number of rules.  */
#define YYNRULES  225
/* YYNRULES -- Number of states.  */
#define YYNSTATES  454
/* YYMAXRHS -- Maximum number of symbols on right-hand side of rule.  */
#define YYMAXRHS 8
/* YYMAXLEFT -- Maximum number of symbols to the left of a handle
   accessed by $0, $-1, etc., in any rule.  */
#define YYMAXLEFT 0

/* YYTRANSLATE(X) -- Bison symbol number corresponding to X.  */
#define YYUNDEFTOK  2
#define YYMAXUTOK   327

#define YYTRANSLATE(YYX)                                                \
  ((unsigned int) (YYX) <= YYMAXUTOK ? yytranslate[YYX] : YYUNDEFTOK)

/* YYTRANSLATE[YYLEX] -- Bison symbol number corresponding to YYLEX.  */
static const unsigned char yytranslate[] =
{
       0,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,    80,    92,    87,
      84,    83,    78,    76,    73,    77,    82,    79,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,    91,    72,
      74,    90,    75,     2,    93,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,    85,     2,    88,    81,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,    89,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     1,     2,     3,     4,
       5,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    22,    23,    24,
      25,    26,    27,    28,    29,    30,    31,    32,    33,    34,
      35,    36,    37,    38,    39,    40,    41,    42,    43,    44,
      45,    46,    47,    48,    49,    50,    51,    52,    53,    54,
      55,    56,    57,    58,    59,    60,    61,    62,    63,    64,
      65,    66,    67,    68,    69,    70,    71,    86
};

#if YYDEBUG
/* YYRLINE[YYN] -- source line where rule number YYN was defined.  */
static const unsigned short int yyrline[] =
{
       0,   116,   116,   119,   120,   123,   124,   131,   132,   133,
     134,   135,   136,   137,   138,   139,   140,   141,   142,   143,
     144,   147,   148,   149,   150,   151,   152,   153,   154,   155,
     156,   157,   158,   159,   160,   163,   165,   168,   171,   174,
     177,   180,   181,   182,   183,   184,   185,   186,   187,   188,
     189,   190,   191,   192,   193,   194,   195,   196,   197,   198,
     203,   204,   205,   206,   207,   208,   211,   212,   213,   216,
     225,   226,   227,   228,   229,   230,   231,   234,   235,   238,
     242,   243,   244,   246,   247,   250,   251,   252,   253,   257,
     258,   259,   260,   261,   264,   265,   268,   271,   272,   273,
     274,   277,   278,   279,   282,   283,   286,   290,   291,   292,
     293,   296,   299,   300,   301,   302,   305,   306,   309,   310,
     313,   320,   321,   325,   326,   330,   331,   334,   335,   336,
     337,   338,   339,   340,   341,   345,   348,   352,   356,   358,
     359,   368,   369,   372,   373,   374,   375,   378,   381,   384,
     387,   390,   393,   394,   395,   396,   399,   400,   401,   402,
     403,   404,   405,   406,   407,   408,   409,   412,   413,   416,
     417,   421,   422,   423,   424,   427,   429,   430,   431,   432,
     433,   434,   435,   436,   437,   438,   439,   440,   441,   442,
     443,   444,   445,   446,   447,   448,   449,   450,   451,   452,
     457,   460,   461,   465,   466,   467,   468,   469,   470,   471,
     472,   473,   474,   475,   476,   477,   478,   479,   480,   481,
     482,   483,   484,   485,   486,   487
};
#endif

#if YYDEBUG || YYERROR_VERBOSE || 1
/* YYTNAME[SYMBOL-NUM] -- String name of the symbol SYMBOL-NUM.
   First, the terminals, then, starting at YYNTOKENS, nonterminals.  */
static const char *const yytname[] =
{
  "$end", "error", "$undefined", "Ident", "UserType", "I8", "I16", "I32",
  "I64", "U8", "U16", "U32", "U64", "Isz", "Usz", "F16", "F32", "F64",
  "C8", "C32", "Bool", "Void", "Eq", "NotEq", "AddEq", "SubEq", "MulEq",
  "DivEq", "GrtrEq", "LesrEq", "Or", "And", "Range", "Returns", "True",
  "False", "IntLit", "FltLit", "StrLit", "Return", "If", "Elif", "Else",
  "For", "While", "Do", "In", "Continue", "Break", "Import", "Let", "Var",
  "Match", "Data", "Enum", "Fun", "Ext", "Pub", "Pri", "Pro", "Raw",
  "Const", "Noinit", "Pathogen", "Where", "Infect", "Cleanse", "Ct",
  "Newline", "Indent", "Unindent", "LOW", "';'", "','", "'<'", "'>'",
  "'+'", "'-'", "'*'", "'/'", "'%'", "'^'", "'.'", "')'", "'('", "'['",
  "HIGH", "'\\''", "']'", "'|'", "'='", "':'", "'&'", "'@'", "$accept",
  "top_level_stmt_list", "stmt_list", "maybe_newline", "stmt",
  "stmt_no_nl", "import_stmt", "ident", "usertype", "intlit", "fltlit",
  "strlit", "lit_type", "type", "type_expr_", "type_expr", "modifier",
  "modifier_list_", "modifier_list", "var_decl", "let_binding",
  "var_assign", "usertype_list", "generic", "data_decl", "type_decl",
  "type_decl_list", "type_decl_block", "val_init_list", "enum_block",
  "enum_decl", "block", "raw_ident_list", "ident_list", "_params",
  "params", "maybe_block", "fn_decl", "fn_call", "ret_stmt", "extension",
  "fn_list", "fn_list_", "elif_list", "maybe_elif_list", "if_stmt",
  "while_loop", "do_while_loop", "for_loop", "var", "ref_val", "val",
  "tuple", "array", "unary_op", "expr", "basic_expr", "expr_list",
  "expr_list_p", "nl_expr", YY_NULLPTR
};
#endif

#define YYPACT_NINF -254
#define YYTABLE_NINF -156

  // YYPACT[STATE-NUM] -- Index in YYTABLE of the portion describing
  // STATE-NUM.
static const short int yypact[] =
{
     -57,  -254,    34,   897,  -254,  -254,  -254,  -254,  -254,  -254,
    -254,  -254,  -254,  -254,  -254,  -254,  -254,  -254,  -254,  -254,
    -254,  -254,  -254,  -254,  -254,  -254,  -254,  -254,  -254,  1170,
    1170,    40,  1170,     9,  1170,  1643,    40,    68,    14,    40,
    1217,  -254,  -254,  -254,  -254,  -254,  -254,  -254,  1261,  1515,
     988,  1079,    40,  1606,  1606,   789,  -254,    17,    51,  -254,
    -254,  -254,  -254,  -254,   -11,   -35,  1388,  -254,   333,  1745,
      46,    58,    63,    80,    84,    86,  -254,    90,    92,  -254,
      96,   105,   112,  -254,    55,  -254,  -254,  -254,  -254,   119,
    1858,    40,  1515,  1515,   100,  1388,  -254,   107,   143,   107,
    1170,  -254,  1217,   113,    40,   494,   114,    19,    68,   122,
    -254,     0,   133,    40,  1261,  1479,  -254,   139,   137,  1979,
    1515,  -254,  -254,  1352,   128,  1872,  -254,   124,  -254,  -254,
    -254,  -254,  -254,  -254,  -254,  -254,   988,  1261,  -254,  -254,
    1035,   127,  1217,  1217,    40,   -55,  -254,  -254,    40,    68,
      14,    40,    40,  -254,  -254,  -254,  -254,  -254,  -254,  -254,
    -254,  -254,  -254,  -254,  1170,  1170,  1170,  1170,  1170,  -254,
    1170,  1170,  1170,  1170,  1170,  1170,  1170,  1261,  1170,  1170,
    1170,  1170,  1170,  1170,  1170,  1170,  1170,    40,  1261,  -254,
     126,   897,   149,  1170,  -254,   107,   135,  1170,   131,   134,
      40,  1170,  1661,    68,   156,  -254,   141,    -6,  -254,  1217,
    1217,  -254,  -254,   108,   142,   164,   -57,  -254,   -57,   -57,
     -57,   -57,   -57,   -57,   -57,  1261,   -57,   -57,   -57,   -57,
     -57,   -57,   -57,   -57,   -57,   -57,  1261,  -254,  -254,  -254,
    -254,  -254,   635,  -254,   152,  -254,    66,    97,  -254,  -254,
    1170,   146,    19,   122,  -254,    37,   154,  -254,  -254,  -254,
    -254,  -254,   223,   223,   223,   223,  1300,  1554,   237,  1886,
     737,   223,   223,   327,   327,   116,   116,   116,   116,  -254,
    1703,  1170,   897,   184,    17,    46,    58,    63,    80,    84,
      86,    90,    92,   186,    96,   105,   112,   119,   -22,   107,
    -254,  -254,  1794,  1170,  1170,   167,  -254,    40,   204,  -254,
      16,  -254,  -254,    57,  -254,  1170,    68,  -254,   107,    40,
     201,    28,   225,   -57,   208,   108,  1261,   213,    40,  1261,
    1261,  1261,  1261,  1261,  1261,  1261,  1950,  1261,  1261,  1261,
    1261,  1261,  1261,  1261,  1261,  1261,    40,  1779,  -254,  -254,
    -254,  1170,   156,  -254,  -254,  1217,  1217,  -254,  1170,  1170,
    -254,  1794,   212,  -254,  1170,   107,   215,  -254,  -254,  1170,
    -254,  -254,  1170,  -254,  1661,  -254,    68,  -254,  -254,   194,
    -254,  -254,    40,  -254,  1126,  1217,  -254,  -254,  -254,   -57,
    1965,  -254,  -254,  1979,   393,   393,   393,   393,  2069,   191,
    1261,  1427,   393,   393,   402,   402,   145,   145,   145,   145,
    -254,   -57,  -254,  -254,   107,    54,  -254,  2043,  -254,   107,
     217,   101,   737,  -254,  -254,  -254,  1170,  -254,   209,    40,
     107,  -254,   -57,  2057,  -254,  -254,  1217,  -254,  -254,  -254,
    1170,   107,  -254,  -254,  -254,  -254,  1261,   107,   107,   222,
    1427,  -254,  -254,  -254
};

  // YYDEFACT[STATE-NUM] -- Default reduction number in state STATE-NUM.
  // Performed when YYTABLE does not specify something else to do.  Zero
  // means the default is an error.
static const unsigned char yydefact[] =
{
       6,     5,     0,     0,     1,    36,    37,    41,    42,    43,
      44,    45,    46,    47,    48,    49,    50,    51,    52,    53,
      54,    55,    56,    57,   165,   166,    38,    39,    40,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,    70,    71,    72,    73,    74,    75,    76,     0,     0,
       0,     0,     0,     0,     0,     6,     4,     0,   151,    58,
     162,   163,   164,    65,    68,    69,     0,    78,    79,     0,
       0,     0,     0,     0,     0,     0,   156,     0,     0,    13,
       0,     0,     0,   161,     0,   198,   158,   159,   160,     0,
     175,     0,     0,     0,   151,     0,   136,     0,     0,     0,
       0,    35,     0,     0,     0,     0,     0,     0,     0,     0,
     115,   126,     0,     0,     0,     0,   224,     0,   200,   202,
       0,   173,   168,     0,     0,   202,   170,     0,    59,   152,
     172,   153,   171,     2,     3,    20,     0,     0,   135,    60,
       0,     0,     0,     0,     0,   151,   174,    77,     0,     0,
       0,     0,     0,    14,    17,    15,     8,     9,     7,    16,
      18,    10,    11,    12,     0,     0,     0,     0,     0,    19,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,   196,
       0,     0,     0,     0,   148,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,    99,   110,     0,   113,     0,
       0,   125,   134,     0,     0,     0,     6,   199,     6,     6,
       6,     6,     6,     6,     6,     0,     6,     6,     6,     6,
       6,     6,     6,     6,     6,     6,     0,   222,    64,   167,
     157,   169,     0,    63,     0,    61,    66,    67,   151,   185,
       0,     0,     0,     0,   114,   126,    81,    90,    91,    92,
      93,    89,   189,   190,   191,   192,   193,   194,   195,     0,
     186,   181,   182,   176,   177,   178,   179,   180,   183,   184,
       0,     0,     0,     0,    34,    28,    31,    29,    22,    23,
      21,    30,    32,    13,    24,    25,    26,    33,   146,     0,
     149,    88,   175,     0,     0,     0,    84,   102,     0,   105,
       0,   103,    95,     0,   100,     0,     0,   111,   126,     0,
     124,   126,     0,     6,     0,   138,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,   154,    62,
      82,     0,     0,    97,   112,     0,     0,   130,     0,     0,
     187,     0,     0,   117,     0,     0,     0,   147,   150,     0,
      87,    86,     0,   101,     0,   106,     0,    96,   109,   107,
     133,   119,   120,   122,     0,     0,   132,   140,   137,     6,
       0,   225,   212,   201,   216,   217,   218,   219,   220,   221,
       0,   213,   208,   209,   203,   204,   205,   206,   207,   210,
     211,     6,    83,    98,   126,   126,    80,   197,   116,     0,
       0,   144,   188,    85,   104,    94,     0,   118,     0,     0,
     126,   139,     6,   223,   214,   129,     0,   128,   142,   145,
       0,     0,   108,   123,   121,   131,     0,   126,     0,     0,
     215,   127,   141,   143
};

  // YYPGOTO[NTERM-NUM].
static const short int yypgoto[] =
{
    -254,  -254,   102,   467,   -54,    12,  -182,     5,   -10,  -254,
    -254,  -254,  -254,     4,  -254,    -3,   229,  -254,   -30,  -181,
    -179,  -178,  -254,    59,  -175,   -65,  -254,  -197,  -254,   -94,
    -188,   -76,  -254,  -131,  -254,   -46,  -253,  -187,  -254,  -174,
    -169,  -254,  -254,  -254,  -254,  -167,  -166,  -161,  -160,  -138,
     125,   308,   331,  -254,  -254,    95,    67,    -9,  -254,   227
};

  // YYDEFGOTO[NTERM-NUM].
static const short int yydefgoto[] =
{
      -1,     2,    55,     3,    56,   283,    57,    94,    59,    60,
      61,    62,    63,    64,    65,    95,    67,    68,    69,    70,
      71,    72,   313,   204,    73,   309,   310,   205,   207,   110,
      74,   211,   382,   383,   320,   321,   212,    75,    76,    77,
      78,   324,   325,   366,   367,    79,    80,    81,    82,    83,
      84,    85,    86,    87,    88,    89,    90,   124,   118,   119
};

  // YYTABLE[YYPACT[STATE-NUM]] -- What to do in state STATE-NUM.  If
  // positive, shift that token.  If negative, reduce the rule whose
  // number is the opposite.  If YYTABLE_NINF, syntax error.
static const short int yytable[] =
{
      66,   134,   357,   289,   290,   105,   249,   314,    58,   284,
     285,     1,   286,   287,   311,   208,   288,   291,     6,   364,
     365,   192,   292,   194,   293,   294,   323,   107,   109,   136,
     295,   296,   104,   209,     4,   250,    98,   112,   142,   117,
     103,   106,   127,     5,   111,   115,   120,   123,   115,   279,
     120,   120,    66,   100,   143,   353,   254,   128,    58,    58,
      58,   385,   316,   120,   317,   380,   152,   139,   386,   191,
     355,   145,     6,   140,   141,  -155,  -155,  -155,  -155,   164,
     165,   166,   167,   108,   374,   135,   375,   436,   202,   120,
     120,   210,   120,   203,   289,   290,   190,   191,   206,   196,
     284,   285,   200,   286,   287,   215,   191,   288,   291,   198,
     199,   115,   120,   292,   153,   293,   294,   120,   214,   300,
     120,   295,   296,   191,    96,    97,   154,    99,   356,   101,
     376,   155,   377,   115,   115,   136,   137,   244,   389,   252,
     253,  -155,   440,   441,   139,   168,   246,   247,   156,   248,
     140,   141,   157,   251,   158,   413,   255,   256,   159,   354,
     160,   435,   437,    39,   161,    41,    42,    43,    44,    45,
      46,    47,   308,   162,   115,   139,   191,   445,   129,   131,
     163,   140,   141,   322,   136,   115,   311,   169,    66,   193,
     392,   108,   248,   312,   451,   195,    58,   186,   187,   307,
     136,   188,   213,   197,   201,   305,   318,   319,   410,   217,
     218,   239,   241,   219,   220,   245,   281,   298,   238,   221,
     222,   303,   115,   368,   304,   202,   234,   235,   134,   136,
     236,   315,   326,   115,   327,   349,   351,   262,   263,   264,
     265,   266,   267,   268,   358,   270,   271,   272,   273,   274,
     275,   276,   277,   278,   363,   176,   -27,   372,   150,   257,
     258,   259,   260,   261,   302,   227,   228,   229,   230,   231,
     232,   233,   234,   235,   384,   136,   236,   125,   388,    66,
     151,   391,   418,   421,   426,   439,   297,    58,   299,   420,
     453,   443,   301,   282,   362,   322,   306,   147,   444,   181,
     182,   183,   184,   185,   186,   187,   379,   136,   188,   424,
     415,   352,   373,   181,   182,   183,   184,   185,   186,   187,
       0,   136,   188,   115,   381,     0,   115,   115,   115,   115,
     115,   115,   115,   248,   115,   115,   115,   115,   115,   115,
     115,   115,   115,   438,   308,   350,     0,     0,   361,     0,
       0,   248,   414,   319,     0,     0,   116,   121,   116,   116,
       0,   130,   132,     0,   242,   449,   425,     0,     0,     0,
       0,   307,   452,     0,   146,     0,     0,   297,     0,     0,
       0,   429,   430,     0,     0,     0,     0,   427,     0,   138,
      41,    42,    43,    44,    45,    46,    47,   115,   370,   371,
     130,   132,     0,   146,   269,   183,   184,   185,   186,   187,
     378,   136,   188,     0,     0,   280,     0,     0,     0,     0,
       0,   189,   116,   146,     0,   138,   417,     0,   146,     0,
       0,   146,     0,   447,   381,     0,   422,     0,     0,     0,
       0,     0,     0,   115,   116,   116,   412,     0,     0,     0,
     237,     0,   336,   416,     0,     0,   237,     0,     0,   419,
       0,     0,     0,   347,     0,     0,     0,   423,     0,   229,
     230,   231,   232,   233,   234,   235,   138,   136,   236,     0,
     231,   232,   233,   234,   235,   116,   136,   236,     0,     0,
       0,     0,     0,     0,     0,     0,   116,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    22,    23,     0,     0,     0,     0,
       0,   442,   133,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,   116,     0,   448,     0,     0,     0,     0,
       0,     0,     0,     0,   116,     0,     0,     0,     0,     0,
       0,     0,     0,   390,     0,     0,   393,   394,   395,   396,
     397,   398,   399,     0,   401,   402,   403,   404,   405,   406,
     407,   408,   409,   237,     0,     0,     0,     0,   102,     0,
       0,    52,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,   189,   189,   189,   189,   189,   189,   189,
     237,   189,   189,   189,   189,   189,   189,   189,   189,   189,
       0,   237,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,   433,     0,     0,
       0,     0,     0,   189,   116,     0,     0,   116,   116,   116,
     116,   116,   116,   116,     0,   116,   116,   116,   116,   116,
     116,   116,   116,   116,     0,     0,     0,   219,   220,     0,
       0,     0,     0,   221,   222,   223,   224,   237,     0,     0,
       0,     0,     0,   450,     0,   225,     0,     0,   237,     0,
       0,     0,     0,   328,     0,   329,   330,   331,   332,   333,
     334,   335,   189,   337,   338,   339,   340,   341,   342,   343,
     344,   345,   346,     0,     0,     0,     0,   226,   116,   227,
     228,   229,   230,   231,   232,   233,   234,   235,     0,   136,
     236,   237,     0,   348,   237,   237,   237,   237,   237,   237,
     237,     0,   237,   237,   237,   237,   237,   237,   237,   237,
     237,     0,     0,     0,     0,     0,     0,     0,   189,     0,
       0,     0,     0,   189,   116,     0,     0,     0,     0,   170,
     171,     0,     0,     0,   237,   172,   173,   174,   175,   176,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,   237,     0,     0,     0,     0,     0,     0,     0,     0,
     387,     0,     5,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    21,    22,
      23,   179,   180,   181,   182,   183,   184,   185,   186,   187,
       0,   136,   188,    24,    25,    26,    27,    28,    29,    30,
       0,     0,    31,    32,    33,     0,     0,     0,    34,    35,
      36,     0,    37,    38,    39,    40,    41,    42,    43,    44,
      45,    46,    47,     0,     0,     0,   431,     1,    48,     0,
       0,     0,     0,     0,     0,     0,    49,     0,     0,     0,
       0,     0,     0,    50,    51,     0,    52,     0,   434,     0,
       0,    53,    54,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,   446,
       5,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    22,    23,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,    24,    25,    26,    27,    28,    29,    30,     0,     0,
      31,    32,    33,     0,     0,     0,    34,    35,    36,     0,
      37,    38,    39,    40,    41,    42,    43,    44,    45,    46,
      47,     0,     0,     0,     0,     0,    48,     0,     0,     0,
       0,     0,     0,     0,    49,     0,     0,     0,     0,     0,
       0,    50,    51,     0,    52,     0,     0,     0,     0,    53,
      54,     5,     6,     7,     8,     9,    10,    11,    12,    13,
      14,    15,    16,    17,    18,    19,    20,    21,    22,    23,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,    24,    25,    26,    27,    28,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,   113,     6,
       7,     8,     9,    10,    11,    12,    13,    14,    15,    16,
      17,    18,    19,    20,    21,    22,    23,   114,     0,     0,
       0,     0,     0,     0,     0,    49,     0,     0,     0,     0,
       0,   122,    50,    51,     0,    52,     0,     0,     0,     0,
      92,    93,     5,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    21,    22,
      23,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,    24,    25,    26,    27,    28,   243,   102,
       0,     0,    52,     0,     0,     0,     0,     0,     0,   113,
       6,     7,     8,     9,    10,    11,    12,    13,    14,    15,
      16,    17,    18,    19,    20,    21,    22,    23,   114,     0,
       0,     0,     0,     0,     0,     0,    49,     0,   428,     0,
       0,     0,     0,    50,    51,     0,    52,   126,     0,     0,
       0,    92,    93,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
      22,    23,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,    24,    25,    26,    27,    28,     0,
     102,     0,     0,    52,     0,     0,     0,     0,     0,     0,
      91,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    22,    23,    48,
       0,     0,     0,     0,     0,     0,     0,    49,     0,     0,
       0,     0,     0,     0,    50,    51,     0,    52,     0,     0,
       0,     0,    92,    93,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    22,    23,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,    24,    25,    26,    27,    28,
       0,   102,     0,     0,    52,     0,     0,     0,     0,     0,
       0,   113,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,   170,   171,     0,     0,     0,     0,   172,   173,
     114,   175,   176,     0,     0,     0,     0,     0,    49,     0,
       0,     0,     0,     0,     0,    50,    51,     0,    52,     0,
       0,     0,     0,    92,    93,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    22,    23,   179,   180,   181,   182,   183,   184,
     185,   186,   187,     0,   136,   188,    24,    25,    26,    27,
      28,     5,     6,     7,     8,     9,    10,    11,    12,    13,
      14,    15,    16,    17,    18,    19,    20,    21,    22,    23,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,    24,    25,    26,    27,    28,     0,     0,    49,
       0,     0,     0,     0,   216,   238,    50,    51,     0,    52,
       0,     0,     0,     0,    92,    93,     0,     0,     0,   219,
     220,     0,     0,     0,     0,   221,   222,   223,   224,     0,
       0,     0,     0,     0,     0,    49,     0,     0,     0,     0,
     144,     0,    50,    51,     0,    52,     0,     0,     0,     0,
      92,    93,     5,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    21,    22,
      23,   227,   228,   229,   230,   231,   232,   233,   234,   235,
       0,   136,   236,    24,    25,    26,    27,    28,     5,     6,
       7,     8,     9,    10,    11,    12,    13,    14,    15,    16,
      17,    18,    19,    20,    21,    22,    23,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,    24,
      25,    26,    27,    28,     0,     0,    49,     0,     0,     0,
       0,   216,     0,    50,    51,     0,    52,     0,     0,     0,
       0,    92,    93,     0,     0,     0,   170,   171,     0,     0,
       0,     0,   172,   173,     0,     0,   176,     0,     0,     0,
       0,     0,    49,     0,     0,     0,     0,     0,     0,    50,
      51,     0,    52,     0,     0,     0,     0,    92,    93,     5,
       6,     7,     8,     9,    10,    11,    12,    13,    14,    15,
      16,    17,    18,    19,    20,    21,    22,    23,   179,   180,
     181,   182,   183,   184,   185,   186,   187,     0,   136,   188,
      24,    25,    26,    27,    28,     0,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    22,    23,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    22,    23,    49,     0,     0,     0,     0,     0,     0,
      50,    51,     0,    52,     0,     0,     0,     0,    53,    54,
      41,    42,    43,    44,    45,    46,    47,     0,     0,     0,
       0,     0,     0,     0,     0,    38,     0,     0,    41,    42,
      43,    44,    45,    46,    47,   219,   220,   102,     0,     0,
      52,   221,   222,   223,   224,     0,     0,     0,     0,     0,
       0,     0,     0,   225,     0,   102,     0,     0,    52,     6,
       7,     8,     9,    10,    11,    12,    13,    14,    15,    16,
      17,    18,    19,    20,    21,    22,    23,     0,     0,     0,
       0,     0,     0,     0,     0,   226,     0,   227,   228,   229,
     230,   231,   232,   233,   234,   235,     0,   136,   236,     0,
       0,   360,     0,     0,     0,     0,   148,     0,   149,   150,
     151,   219,   220,     0,     0,     0,     0,   221,   222,   223,
     224,     0,     0,     0,     0,     0,   170,   171,     0,   225,
       0,     0,   172,   173,   174,   175,   176,     0,     0,   102,
       0,     0,    52,     0,   177,     0,     0,     0,     0,     0,
     369,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,   226,     0,   227,   228,   229,   230,   231,   232,   233,
     234,   235,     0,   136,   236,     0,   178,   411,   179,   180,
     181,   182,   183,   184,   185,   186,   187,     0,   136,   188,
     170,   171,     0,     0,     0,     0,   172,   173,   174,   175,
     176,     0,     0,     0,   219,   220,     0,     0,   177,     0,
     221,   222,   223,   224,     0,     0,     0,     0,   219,   220,
       0,     0,   225,     0,   221,   222,   223,   224,     0,     0,
       0,     0,     0,     0,     0,     0,   225,     0,   359,     0,
     178,     0,   179,   180,   181,   182,   183,   184,   185,   186,
     187,     0,   136,   188,   226,     0,   227,   228,   229,   230,
     231,   232,   233,   234,   235,   240,   136,   236,   226,     0,
     227,   228,   229,   230,   231,   232,   233,   234,   235,     0,
     136,   236,   219,   220,     0,     0,     0,     0,   221,   222,
     223,   224,     0,     0,     0,     0,     0,   219,   220,     0,
     225,     0,   400,   221,   222,   223,   224,     0,     0,     0,
       0,   219,   220,     0,     0,   225,     0,   221,   222,   223,
     224,   432,     0,     0,     0,     0,     0,     0,     0,   225,
       0,     0,   226,     0,   227,   228,   229,   230,   231,   232,
     233,   234,   235,     0,   136,   236,     0,   226,     0,   227,
     228,   229,   230,   231,   232,   233,   234,   235,     0,   136,
     236,   226,     0,   227,   228,   229,   230,   231,   232,   233,
     234,   235,     0,   136,   236,   170,   171,     0,     0,     0,
       0,   172,   173,   174,   175,   176,     0,     0,     0,   219,
     220,     0,     0,     0,     0,   221,   222,   223,   224,     0,
       0,   219,   220,     0,     0,     0,     0,   221,   222,     0,
     224,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,   178,     0,   179,   180,   181,
     182,   183,   184,   185,   186,   187,     0,   136,   188,   226,
       0,   227,   228,   229,   230,   231,   232,   233,   234,   235,
       0,   136,   236,   227,   228,   229,   230,   231,   232,   233,
     234,   235,     0,   136,   236
};

static const short int yycheck[] =
{
       3,    55,   255,   191,   191,    35,   144,   204,     3,   191,
     191,    68,   191,   191,   202,   109,   191,   191,     4,    41,
      42,    97,   191,    99,   191,   191,   213,    37,    38,    84,
     191,   191,    35,    33,     0,    90,    31,    40,    73,    48,
      35,    36,    51,     3,    39,    48,    49,    50,    51,   187,
      53,    54,    55,    44,    89,   252,   150,    52,    53,    54,
      55,    33,    68,    66,    70,   318,    69,    78,   321,    69,
      33,    66,     4,    84,    85,    24,    25,    26,    27,    24,
      25,    26,    27,    69,    68,    68,    70,    33,    69,    92,
      93,    91,    95,    74,   282,   282,    91,    69,   108,   102,
     282,   282,   105,   282,   282,   114,    69,   282,   282,   104,
     105,   114,   115,   282,    68,   282,   282,   120,   113,   195,
     123,   282,   282,    69,    29,    30,    68,    32,    91,    34,
      73,    68,    75,   136,   137,    84,    85,   140,   325,   149,
     150,    90,    41,    42,    78,    90,   142,   143,    68,   144,
      84,    85,    68,   148,    68,   352,   151,   152,    68,   253,
      68,   414,   415,    55,    68,    57,    58,    59,    60,    61,
      62,    63,   202,    68,   177,    78,    69,   430,    53,    54,
      68,    84,    85,   213,    84,   188,   374,    68,   191,    46,
     328,    69,   187,   203,   447,   100,   191,    81,    82,   202,
      84,    85,    69,    90,    90,   200,   209,   210,   346,    70,
      73,    83,    88,    22,    23,    88,    90,    68,    83,    28,
      29,    90,   225,   299,    90,    69,    81,    82,   282,    84,
      85,    90,    90,   236,    70,    83,    90,   170,   171,   172,
     173,   174,   175,   176,    90,   178,   179,   180,   181,   182,
     183,   184,   185,   186,    70,    32,    70,    90,    54,   164,
     165,   166,   167,   168,   197,    74,    75,    76,    77,    78,
      79,    80,    81,    82,    73,    84,    85,    50,    70,   282,
      55,    68,    70,    68,    90,    68,   191,   282,   193,   365,
      68,    82,   197,   191,   282,   325,   201,    68,   429,    76,
      77,    78,    79,    80,    81,    82,   316,    84,    85,   374,
     356,   252,   307,    76,    77,    78,    79,    80,    81,    82,
      -1,    84,    85,   326,   319,    -1,   329,   330,   331,   332,
     333,   334,   335,   328,   337,   338,   339,   340,   341,   342,
     343,   344,   345,   419,   374,   250,    -1,    -1,   281,    -1,
      -1,   346,   355,   356,    -1,    -1,    48,    49,    50,    51,
      -1,    53,    54,    -1,   137,   441,   376,    -1,    -1,    -1,
      -1,   374,   448,    -1,    66,    -1,    -1,   282,    -1,    -1,
      -1,   384,   385,    -1,    -1,    -1,    -1,   382,    -1,    58,
      57,    58,    59,    60,    61,    62,    63,   400,   303,   304,
      92,    93,    -1,    95,   177,    78,    79,    80,    81,    82,
     315,    84,    85,    -1,    -1,   188,    -1,    -1,    -1,    -1,
      -1,    90,   114,   115,    -1,    94,   359,    -1,   120,    -1,
      -1,   123,    -1,   436,   429,    -1,   369,    -1,    -1,    -1,
      -1,    -1,    -1,   446,   136,   137,   351,    -1,    -1,    -1,
     119,    -1,   225,   358,    -1,    -1,   125,    -1,    -1,   364,
      -1,    -1,    -1,   236,    -1,    -1,    -1,   372,    -1,    76,
      77,    78,    79,    80,    81,    82,   145,    84,    85,    -1,
      78,    79,    80,    81,    82,   177,    84,    85,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,   188,     3,     4,     5,
       6,     7,     8,     9,    10,    11,    12,    13,    14,    15,
      16,    17,    18,    19,    20,    21,    -1,    -1,    -1,    -1,
      -1,   426,    55,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,   225,    -1,   440,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,   236,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,   326,    -1,    -1,   329,   330,   331,   332,
     333,   334,   335,    -1,   337,   338,   339,   340,   341,   342,
     343,   344,   345,   242,    -1,    -1,    -1,    -1,    84,    -1,
      -1,    87,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,   262,   263,   264,   265,   266,   267,   268,
     269,   270,   271,   272,   273,   274,   275,   276,   277,   278,
      -1,   280,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,   400,    -1,    -1,
      -1,    -1,    -1,   302,   326,    -1,    -1,   329,   330,   331,
     332,   333,   334,   335,    -1,   337,   338,   339,   340,   341,
     342,   343,   344,   345,    -1,    -1,    -1,    22,    23,    -1,
      -1,    -1,    -1,    28,    29,    30,    31,   336,    -1,    -1,
      -1,    -1,    -1,   446,    -1,    40,    -1,    -1,   347,    -1,
      -1,    -1,    -1,   216,    -1,   218,   219,   220,   221,   222,
     223,   224,   361,   226,   227,   228,   229,   230,   231,   232,
     233,   234,   235,    -1,    -1,    -1,    -1,    72,   400,    74,
      75,    76,    77,    78,    79,    80,    81,    82,    -1,    84,
      85,   390,    -1,    88,   393,   394,   395,   396,   397,   398,
     399,    -1,   401,   402,   403,   404,   405,   406,   407,   408,
     409,    -1,    -1,    -1,    -1,    -1,    -1,    -1,   417,    -1,
      -1,    -1,    -1,   422,   446,    -1,    -1,    -1,    -1,    22,
      23,    -1,    -1,    -1,   433,    28,    29,    30,    31,    32,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,   450,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
     323,    -1,     3,     4,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    74,    75,    76,    77,    78,    79,    80,    81,    82,
      -1,    84,    85,    34,    35,    36,    37,    38,    39,    40,
      -1,    -1,    43,    44,    45,    -1,    -1,    -1,    49,    50,
      51,    -1,    53,    54,    55,    56,    57,    58,    59,    60,
      61,    62,    63,    -1,    -1,    -1,   389,    68,    69,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    77,    -1,    -1,    -1,
      -1,    -1,    -1,    84,    85,    -1,    87,    -1,   411,    -1,
      -1,    92,    93,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,   432,
       3,     4,     5,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    21,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    34,    35,    36,    37,    38,    39,    40,    -1,    -1,
      43,    44,    45,    -1,    -1,    -1,    49,    50,    51,    -1,
      53,    54,    55,    56,    57,    58,    59,    60,    61,    62,
      63,    -1,    -1,    -1,    -1,    -1,    69,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    77,    -1,    -1,    -1,    -1,    -1,
      -1,    84,    85,    -1,    87,    -1,    -1,    -1,    -1,    92,
      93,     3,     4,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    34,    35,    36,    37,    38,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    50,     4,
       5,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    69,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    77,    -1,    -1,    -1,    -1,
      -1,    83,    84,    85,    -1,    87,    -1,    -1,    -1,    -1,
      92,    93,     3,     4,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    34,    35,    36,    37,    38,    83,    84,
      -1,    -1,    87,    -1,    -1,    -1,    -1,    -1,    -1,    50,
       4,     5,     6,     7,     8,     9,    10,    11,    12,    13,
      14,    15,    16,    17,    18,    19,    20,    21,    69,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    77,    -1,    32,    -1,
      -1,    -1,    -1,    84,    85,    -1,    87,    88,    -1,    -1,
      -1,    92,    93,     3,     4,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    34,    35,    36,    37,    38,    -1,
      84,    -1,    -1,    87,    -1,    -1,    -1,    -1,    -1,    -1,
      50,     4,     5,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    21,    69,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    77,    -1,    -1,
      -1,    -1,    -1,    -1,    84,    85,    -1,    87,    -1,    -1,
      -1,    -1,    92,    93,     3,     4,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    34,    35,    36,    37,    38,
      -1,    84,    -1,    -1,    87,    -1,    -1,    -1,    -1,    -1,
      -1,    50,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    22,    23,    -1,    -1,    -1,    -1,    28,    29,
      69,    31,    32,    -1,    -1,    -1,    -1,    -1,    77,    -1,
      -1,    -1,    -1,    -1,    -1,    84,    85,    -1,    87,    -1,
      -1,    -1,    -1,    92,    93,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    74,    75,    76,    77,    78,    79,
      80,    81,    82,    -1,    84,    85,    34,    35,    36,    37,
      38,     3,     4,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    34,    35,    36,    37,    38,    -1,    -1,    77,
      -1,    -1,    -1,    -1,    82,    83,    84,    85,    -1,    87,
      -1,    -1,    -1,    -1,    92,    93,    -1,    -1,    -1,    22,
      23,    -1,    -1,    -1,    -1,    28,    29,    30,    31,    -1,
      -1,    -1,    -1,    -1,    -1,    77,    -1,    -1,    -1,    -1,
      82,    -1,    84,    85,    -1,    87,    -1,    -1,    -1,    -1,
      92,    93,     3,     4,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    74,    75,    76,    77,    78,    79,    80,    81,    82,
      -1,    84,    85,    34,    35,    36,    37,    38,     3,     4,
       5,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    34,
      35,    36,    37,    38,    -1,    -1,    77,    -1,    -1,    -1,
      -1,    82,    -1,    84,    85,    -1,    87,    -1,    -1,    -1,
      -1,    92,    93,    -1,    -1,    -1,    22,    23,    -1,    -1,
      -1,    -1,    28,    29,    -1,    -1,    32,    -1,    -1,    -1,
      -1,    -1,    77,    -1,    -1,    -1,    -1,    -1,    -1,    84,
      85,    -1,    87,    -1,    -1,    -1,    -1,    92,    93,     3,
       4,     5,     6,     7,     8,     9,    10,    11,    12,    13,
      14,    15,    16,    17,    18,    19,    20,    21,    74,    75,
      76,    77,    78,    79,    80,    81,    82,    -1,    84,    85,
      34,    35,    36,    37,    38,    -1,     3,     4,     5,     6,
       7,     8,     9,    10,    11,    12,    13,    14,    15,    16,
      17,    18,    19,    20,    21,     4,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    77,    -1,    -1,    -1,    -1,    -1,    -1,
      84,    85,    -1,    87,    -1,    -1,    -1,    -1,    92,    93,
      57,    58,    59,    60,    61,    62,    63,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    54,    -1,    -1,    57,    58,
      59,    60,    61,    62,    63,    22,    23,    84,    -1,    -1,
      87,    28,    29,    30,    31,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    40,    -1,    84,    -1,    -1,    87,     4,
       5,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    72,    -1,    74,    75,    76,
      77,    78,    79,    80,    81,    82,    -1,    84,    85,    -1,
      -1,    88,    -1,    -1,    -1,    -1,    51,    -1,    53,    54,
      55,    22,    23,    -1,    -1,    -1,    -1,    28,    29,    30,
      31,    -1,    -1,    -1,    -1,    -1,    22,    23,    -1,    40,
      -1,    -1,    28,    29,    30,    31,    32,    -1,    -1,    84,
      -1,    -1,    87,    -1,    40,    -1,    -1,    -1,    -1,    -1,
      46,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    72,    -1,    74,    75,    76,    77,    78,    79,    80,
      81,    82,    -1,    84,    85,    -1,    72,    88,    74,    75,
      76,    77,    78,    79,    80,    81,    82,    -1,    84,    85,
      22,    23,    -1,    -1,    -1,    -1,    28,    29,    30,    31,
      32,    -1,    -1,    -1,    22,    23,    -1,    -1,    40,    -1,
      28,    29,    30,    31,    -1,    -1,    -1,    -1,    22,    23,
      -1,    -1,    40,    -1,    28,    29,    30,    31,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    40,    -1,    42,    -1,
      72,    -1,    74,    75,    76,    77,    78,    79,    80,    81,
      82,    -1,    84,    85,    72,    -1,    74,    75,    76,    77,
      78,    79,    80,    81,    82,    83,    84,    85,    72,    -1,
      74,    75,    76,    77,    78,    79,    80,    81,    82,    -1,
      84,    85,    22,    23,    -1,    -1,    -1,    -1,    28,    29,
      30,    31,    -1,    -1,    -1,    -1,    -1,    22,    23,    -1,
      40,    -1,    42,    28,    29,    30,    31,    -1,    -1,    -1,
      -1,    22,    23,    -1,    -1,    40,    -1,    28,    29,    30,
      31,    46,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    40,
      -1,    -1,    72,    -1,    74,    75,    76,    77,    78,    79,
      80,    81,    82,    -1,    84,    85,    -1,    72,    -1,    74,
      75,    76,    77,    78,    79,    80,    81,    82,    -1,    84,
      85,    72,    -1,    74,    75,    76,    77,    78,    79,    80,
      81,    82,    -1,    84,    85,    22,    23,    -1,    -1,    -1,
      -1,    28,    29,    30,    31,    32,    -1,    -1,    -1,    22,
      23,    -1,    -1,    -1,    -1,    28,    29,    30,    31,    -1,
      -1,    22,    23,    -1,    -1,    -1,    -1,    28,    29,    -1,
      31,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    72,    -1,    74,    75,    76,
      77,    78,    79,    80,    81,    82,    -1,    84,    85,    72,
      -1,    74,    75,    76,    77,    78,    79,    80,    81,    82,
      -1,    84,    85,    74,    75,    76,    77,    78,    79,    80,
      81,    82,    -1,    84,    85
};

  // YYSTOS[STATE-NUM] -- The (internal number of the) accessing
  // symbol of state STATE-NUM.
static const unsigned char yystos[] =
{
       0,    68,    95,    97,     0,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    34,    35,    36,    37,    38,    39,
      40,    43,    44,    45,    49,    50,    51,    53,    54,    55,
      56,    57,    58,    59,    60,    61,    62,    63,    69,    77,
      84,    85,    87,    92,    93,    96,    98,   100,   101,   102,
     103,   104,   105,   106,   107,   108,   109,   110,   111,   112,
     113,   114,   115,   118,   124,   131,   132,   133,   134,   139,
     140,   141,   142,   143,   144,   145,   146,   147,   148,   149,
     150,    50,    92,    93,   101,   109,   149,   149,   101,   149,
      44,   149,    84,   101,   109,   112,   101,   102,    69,   102,
     123,   101,   109,    50,    69,   109,   145,   151,   152,   153,
     109,   145,    83,   109,   151,   153,    88,   151,   101,   144,
     145,   144,   145,    97,    98,    68,    84,    85,   146,    78,
      84,    85,    73,    89,    82,   101,   145,   110,    51,    53,
      54,    55,   109,    68,    68,    68,    68,    68,    68,    68,
      68,    68,    68,    68,    24,    25,    26,    27,    90,    68,
      22,    23,    28,    29,    30,    31,    32,    40,    72,    74,
      75,    76,    77,    78,    79,    80,    81,    82,    85,   146,
     101,    69,   125,    46,   125,   149,   109,    90,   101,   101,
     109,    90,    69,    74,   117,   121,   102,   122,   123,    33,
      91,   125,   130,    69,   101,   151,    82,    70,    73,    22,
      23,    28,    29,    30,    31,    40,    72,    74,    75,    76,
      77,    78,    79,    80,    81,    82,    85,   146,    83,    83,
      83,    88,   153,    83,   109,    88,   107,   107,   101,   143,
      90,   101,   102,   102,   123,   101,   101,   149,   149,   149,
     149,   149,   150,   150,   150,   150,   150,   150,   150,   153,
     150,   150,   150,   150,   150,   150,   150,   150,   150,   143,
     153,    90,    96,    99,   100,   113,   114,   115,   118,   124,
     131,   133,   134,   139,   140,   141,   142,   149,    68,   149,
     125,   149,   150,    90,    90,   101,   149,   109,   112,   119,
     120,   124,   102,   116,   121,    90,    68,    70,   109,   109,
     128,   129,   112,   131,   135,   136,    90,    70,    97,    97,
      97,    97,    97,    97,    97,    97,   153,    97,    97,    97,
      97,    97,    97,    97,    97,    97,    97,   153,    88,    83,
     149,    90,   117,   121,   123,    33,    91,   130,    90,    42,
      88,   150,    99,    70,    41,    42,   137,   138,   125,    46,
     149,   149,    90,   101,    68,    70,    73,    75,   149,   102,
     130,   101,   126,   127,    73,    33,   130,    97,    70,   131,
     153,    68,   143,   153,   153,   153,   153,   153,   153,   153,
      42,   153,   153,   153,   153,   153,   153,   153,   153,   153,
     143,    88,   149,   121,   109,   129,   149,   150,    70,   149,
     125,    68,   150,   149,   119,   102,    90,   101,    32,   109,
     109,    97,    46,   153,    97,   130,    33,   130,   125,    68,
      41,    42,   149,    82,   127,   130,    97,   109,   149,   125,
     153,   130,   125,    68
};

  // YYR1[YYN] -- Symbol number of symbol that rule YYN derives.
static const unsigned char yyr1[] =
{
       0,    94,    95,    96,    96,    97,    97,    98,    98,    98,
      98,    98,    98,    98,    98,    98,    98,    98,    98,    98,
      98,    99,    99,    99,    99,    99,    99,    99,    99,    99,
      99,    99,    99,    99,    99,   100,   101,   102,   103,   104,
     105,   106,   106,   106,   106,   106,   106,   106,   106,   106,
     106,   106,   106,   106,   106,   106,   106,   106,   106,   106,
     107,   107,   107,   107,   107,   107,   108,   108,   108,   109,
     110,   110,   110,   110,   110,   110,   110,   111,   111,   112,
     113,   113,   113,   113,   113,   114,   114,   114,   114,   115,
     115,   115,   115,   115,   116,   116,   117,   118,   118,   118,
     118,   119,   119,   119,   120,   120,   121,   122,   122,   122,
     122,   123,   124,   124,   124,   124,   125,   125,   126,   126,
     127,   128,   128,   129,   129,   130,   130,   131,   131,   131,
     131,   131,   131,   131,   131,   132,   133,   134,   135,   136,
     136,   137,   137,   138,   138,   138,   138,   139,   140,   141,
     142,   143,   144,   144,   144,   144,   145,   145,   145,   145,
     145,   145,   145,   145,   145,   145,   145,   146,   146,   147,
     147,   148,   148,   148,   148,   149,   150,   150,   150,   150,
     150,   150,   150,   150,   150,   150,   150,   150,   150,   150,
     150,   150,   150,   150,   150,   150,   150,   150,   150,   150,
     151,   152,   152,   153,   153,   153,   153,   153,   153,   153,
     153,   153,   153,   153,   153,   153,   153,   153,   153,   153,
     153,   153,   153,   153,   153,   153
};

  // YYR2[YYN] -- Number of symbols on the right hand side of rule YYN.
static const unsigned char yyr2[] =
{
       0,     2,     3,     2,     1,     1,     0,     2,     2,     2,
       2,     2,     2,     1,     2,     2,     2,     2,     2,     2,
       2,     1,     1,     1,     1,     1,     1,     1,     1,     1,
       1,     1,     1,     1,     1,     2,     1,     1,     1,     1,
       1,     1,     1,     1,     1,     1,     1,     1,     1,     1,
       1,     1,     1,     1,     1,     1,     1,     1,     1,     2,
       2,     3,     4,     3,     3,     1,     3,     3,     1,     1,
       1,     1,     1,     1,     1,     1,     1,     2,     1,     1,
       5,     3,     4,     5,     4,     6,     5,     5,     4,     3,
       3,     3,     3,     3,     3,     1,     3,     4,     5,     3,
       4,     2,     1,     1,     3,     1,     3,     3,     5,     3,
       1,     3,     4,     3,     3,     2,     4,     3,     2,     1,
       1,     4,     2,     4,     1,     1,     0,     8,     6,     6,
       4,     7,     5,     5,     3,     2,     2,     5,     1,     3,
       2,     5,     3,     5,     2,     3,     0,     5,     3,     4,
       5,     1,     2,     2,     4,     1,     1,     3,     1,     1,
       1,     1,     1,     1,     1,     1,     1,     3,     2,     3,
       2,     2,     2,     2,     2,     1,     3,     3,     3,     3,
       3,     3,     3,     3,     3,     3,     3,     4,     6,     3,
       3,     3,     3,     3,     3,     3,     2,     5,     1,     3,
       1,     4,     1,     4,     4,     4,     4,     4,     4,     4,
       4,     4,     4,     4,     5,     7,     4,     4,     4,     4,
       4,     4,     2,     5,     1,     4
};


/* YYDPREC[RULE-NUM] -- Dynamic precedence of rule #RULE-NUM (0 if none).  */
static const unsigned char yydprec[] =
{
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       2,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     0,     0,     2,     0,
       0,     0,     0,     1,     1,     1,     1,     1,     1,     1,
       1,     1,     2,     1,     1,     3,     1,     1,     1,     1,
       1,     1,     0,     0,     1,     1
};

/* YYMERGER[RULE-NUM] -- Index of merging function for rule #RULE-NUM.  */
static const unsigned char yymerger[] =
{
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0
};

/* YYIMMEDIATE[RULE-NUM] -- True iff rule #RULE-NUM is not to be deferred, as
   in the case of predicates.  */
static const yybool yyimmediate[] =
{
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0
};

/* YYCONFLP[YYPACT[STATE-NUM]] -- Pointer into YYCONFL of start of
   list of conflicting reductions corresponding to action entry for
   state STATE-NUM in yytable.  0 means no conflicts.  The list in
   yyconfl is terminated by a rule number of 0.  */
static const unsigned char yyconflp[] =
{
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     1,     3,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     5,     7,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0
};

/* YYCONFL[I] -- lists of conflicting rule numbers, each terminated by
   0, pointed into by YYCONFLP.  */
static const short int yyconfl[] =
{
       0,    68,     0,    68,     0,    67,     0,    67,     0
};

/* Error token number */
#define YYTERROR 1


/* YYLLOC_DEFAULT -- Set CURRENT to span from RHS[1] to RHS[N].
   If N is 0, then set CURRENT to the empty location which ends
   the previous symbol: RHS[0] (always defined).  */

# ifndef YYLLOC_DEFAULT
#  define YYLLOC_DEFAULT(Current, Rhs, N)                               \
    do                                                                  \
      if (N)                                                            \
        {                                                               \
          (Current).begin  = YYRHSLOC (Rhs, 1).begin;                   \
          (Current).end    = YYRHSLOC (Rhs, N).end;                     \
        }                                                               \
      else                                                              \
        {                                                               \
          (Current).begin = (Current).end = YYRHSLOC (Rhs, 0).end;      \
        }                                                               \
    while (/*CONSTCOND*/ false)
# endif

# define YYRHSLOC(Rhs, K) ((Rhs)[K].yystate.yyloc)



#undef yynerrs
#define yynerrs (yystackp->yyerrcnt)
#undef yychar
#define yychar (yystackp->yyrawchar)
#undef yylval
#define yylval (yystackp->yyval)
#undef yylloc
#define yylloc (yystackp->yyloc)


static const int YYEOF = 0;
static const int YYEMPTY = -2;

typedef enum { yyok, yyaccept, yyabort, yyerr } YYRESULTTAG;

#define YYCHK(YYE)                              \
  do {                                          \
    YYRESULTTAG yychk_flag = YYE;               \
    if (yychk_flag != yyok)                     \
      return yychk_flag;                        \
  } while (0)

#if YYDEBUG

# ifndef YYFPRINTF
#  define YYFPRINTF fprintf
# endif


/* YY_LOCATION_PRINT -- Print the location on the stream.
   This macro was not mandated originally: define only if we know
   we won't break user code: when these are the locations we know.  */

#ifndef YY_LOCATION_PRINT
# if defined YYLTYPE_IS_TRIVIAL && YYLTYPE_IS_TRIVIAL

/* Print *YYLOCP on YYO.  Private, do not rely on its existence. */

YY_ATTRIBUTE_UNUSED
static unsigned
yy_location_print_ (FILE *yyo, YYLTYPE const * const yylocp)
{
  unsigned res = 0;
  int end_col = 0 != yylocp->last_column ? yylocp->last_column - 1 : 0;
  if (0 <= yylocp->first_line)
    {
      res += YYFPRINTF (yyo, "%d", yylocp->first_line);
      if (0 <= yylocp->first_column)
        res += YYFPRINTF (yyo, ".%d", yylocp->first_column);
    }
  if (0 <= yylocp->last_line)
    {
      if (yylocp->first_line < yylocp->last_line)
        {
          res += YYFPRINTF (yyo, "-%d", yylocp->last_line);
          if (0 <= end_col)
            res += YYFPRINTF (yyo, ".%d", end_col);
        }
      else if (0 <= end_col && yylocp->first_column < end_col)
        res += YYFPRINTF (yyo, "-%d", end_col);
    }
  return res;
 }

#  define YY_LOCATION_PRINT(File, Loc)          \
  yy_location_print_ (File, &(Loc))

# else
#  define YY_LOCATION_PRINT(File, Loc) ((void) 0)
# endif
#endif


# define YYDPRINTF(Args)                        \
  do {                                          \
    if (yydebug)                                \
      YYFPRINTF Args;                           \
  } while (0)


/*--------------------.
| Print this symbol.  |
`--------------------*/

static void
yy_symbol_print (FILE *, int yytype, const yy::parser::semantic_type *yyvaluep, const yy::parser::location_type *yylocationp, yy::parser& yyparser)
{
  YYUSE (yyparser);
  yyparser.yy_symbol_print_ (yytype, yyvaluep, yylocationp);
}


# define YY_SYMBOL_PRINT(Title, Type, Value, Location)                  \
  do {                                                                  \
    if (yydebug)                                                        \
      {                                                                 \
        YYFPRINTF (stderr, "%s ", Title);                               \
        yy_symbol_print (stderr, Type, Value, Location, yyparser);        \
        YYFPRINTF (stderr, "\n");                                       \
      }                                                                 \
  } while (0)

/* Nonzero means print parse trace.  It is left uninitialized so that
   multiple parsers can coexist.  */
int yydebug;

struct yyGLRStack;
static void yypstack (struct yyGLRStack* yystackp, size_t yyk)
  YY_ATTRIBUTE_UNUSED;
static void yypdumpstack (struct yyGLRStack* yystackp)
  YY_ATTRIBUTE_UNUSED;

#else /* !YYDEBUG */

# define YYDPRINTF(Args)
# define YY_SYMBOL_PRINT(Title, Type, Value, Location)

#endif /* !YYDEBUG */

/* YYINITDEPTH -- initial size of the parser's stacks.  */
#ifndef YYINITDEPTH
# define YYINITDEPTH 200
#endif

/* YYMAXDEPTH -- maximum size the stacks can grow to (effective only
   if the built-in stack extension method is used).

   Do not make this value too large; the results are undefined if
   SIZE_MAX < YYMAXDEPTH * sizeof (GLRStackItem)
   evaluated with infinite-precision integer arithmetic.  */

#ifndef YYMAXDEPTH
# define YYMAXDEPTH 10000
#endif

/* Minimum number of free items on the stack allowed after an
   allocation.  This is to allow allocation and initialization
   to be completed by functions that call yyexpandGLRStack before the
   stack is expanded, thus insuring that all necessary pointers get
   properly redirected to new data.  */
#define YYHEADROOM 2

#ifndef YYSTACKEXPANDABLE
#  define YYSTACKEXPANDABLE 1
#endif

#if YYSTACKEXPANDABLE
# define YY_RESERVE_GLRSTACK(Yystack)                   \
  do {                                                  \
    if (Yystack->yyspaceLeft < YYHEADROOM)              \
      yyexpandGLRStack (Yystack);                       \
  } while (0)
#else
# define YY_RESERVE_GLRSTACK(Yystack)                   \
  do {                                                  \
    if (Yystack->yyspaceLeft < YYHEADROOM)              \
      yyMemoryExhausted (Yystack);                      \
  } while (0)
#endif


#if YYERROR_VERBOSE

# ifndef yystpcpy
#  if defined __GLIBC__ && defined _STRING_H && defined _GNU_SOURCE
#   define yystpcpy stpcpy
#  else
/* Copy YYSRC to YYDEST, returning the address of the terminating '\0' in
   YYDEST.  */
static char *
yystpcpy (char *yydest, const char *yysrc)
{
  char *yyd = yydest;
  const char *yys = yysrc;

  while ((*yyd++ = *yys++) != '\0')
    continue;

  return yyd - 1;
}
#  endif
# endif

# ifndef yytnamerr
/* Copy to YYRES the contents of YYSTR after stripping away unnecessary
   quotes and backslashes, so that it's suitable for yyerror.  The
   heuristic is that double-quoting is unnecessary unless the string
   contains an apostrophe, a comma, or backslash (other than
   backslash-backslash).  YYSTR is taken from yytname.  If YYRES is
   null, do not copy; instead, return the length of what the result
   would have been.  */
static size_t
yytnamerr (char *yyres, const char *yystr)
{
  if (*yystr == '"')
    {
      size_t yyn = 0;
      char const *yyp = yystr;

      for (;;)
        switch (*++yyp)
          {
          case '\'':
          case ',':
            goto do_not_strip_quotes;

          case '\\':
            if (*++yyp != '\\')
              goto do_not_strip_quotes;
            /* Fall through.  */
          default:
            if (yyres)
              yyres[yyn] = *yyp;
            yyn++;
            break;

          case '"':
            if (yyres)
              yyres[yyn] = '\0';
            return yyn;
          }
    do_not_strip_quotes: ;
    }

  if (! yyres)
    return strlen (yystr);

  return yystpcpy (yyres, yystr) - yyres;
}
# endif

#endif /* !YYERROR_VERBOSE */

/** State numbers, as in LALR(1) machine */
typedef int yyStateNum;

/** Rule numbers, as in LALR(1) machine */
typedef int yyRuleNum;

/** Grammar symbol */
typedef int yySymbol;

/** Item references, as in LALR(1) machine */
typedef short int yyItemNum;

typedef struct yyGLRState yyGLRState;
typedef struct yyGLRStateSet yyGLRStateSet;
typedef struct yySemanticOption yySemanticOption;
typedef union yyGLRStackItem yyGLRStackItem;
typedef struct yyGLRStack yyGLRStack;

struct yyGLRState {
  /** Type tag: always true.  */
  yybool yyisState;
  /** Type tag for yysemantics.  If true, yysval applies, otherwise
   *  yyfirstVal applies.  */
  yybool yyresolved;
  /** Number of corresponding LALR(1) machine state.  */
  yyStateNum yylrState;
  /** Preceding state in this stack */
  yyGLRState* yypred;
  /** Source position of the last token produced by my symbol */
  size_t yyposn;
  union {
    /** First in a chain of alternative reductions producing the
     *  non-terminal corresponding to this state, threaded through
     *  yynext.  */
    yySemanticOption* yyfirstVal;
    /** Semantic value for this state.  */
    YYSTYPE yysval;
  } yysemantics;
  /** Source location for this state.  */
  YYLTYPE yyloc;
};

struct yyGLRStateSet {
  yyGLRState** yystates;
  /** During nondeterministic operation, yylookaheadNeeds tracks which
   *  stacks have actually needed the current lookahead.  During deterministic
   *  operation, yylookaheadNeeds[0] is not maintained since it would merely
   *  duplicate yychar != YYEMPTY.  */
  yybool* yylookaheadNeeds;
  size_t yysize, yycapacity;
};

struct yySemanticOption {
  /** Type tag: always false.  */
  yybool yyisState;
  /** Rule number for this reduction */
  yyRuleNum yyrule;
  /** The last RHS state in the list of states to be reduced.  */
  yyGLRState* yystate;
  /** The lookahead for this reduction.  */
  int yyrawchar;
  YYSTYPE yyval;
  YYLTYPE yyloc;
  /** Next sibling in chain of options.  To facilitate merging,
   *  options are chained in decreasing order by address.  */
  yySemanticOption* yynext;
};

/** Type of the items in the GLR stack.  The yyisState field
 *  indicates which item of the union is valid.  */
union yyGLRStackItem {
  yyGLRState yystate;
  yySemanticOption yyoption;
};

struct yyGLRStack {
  int yyerrState;
  /* To compute the location of the error token.  */
  yyGLRStackItem yyerror_range[3];

  int yyerrcnt;
  int yyrawchar;
  YYSTYPE yyval;
  YYLTYPE yyloc;

  YYJMP_BUF yyexception_buffer;
  yyGLRStackItem* yyitems;
  yyGLRStackItem* yynextFree;
  size_t yyspaceLeft;
  yyGLRState* yysplitPoint;
  yyGLRState* yylastDeleted;
  yyGLRStateSet yytops;
};

#if YYSTACKEXPANDABLE
static void yyexpandGLRStack (yyGLRStack* yystackp);
#endif

static _Noreturn void
yyFail (yyGLRStack* yystackp, YYLTYPE *yylocp, yy::parser& yyparser, const char* yymsg)
{
  if (yymsg != YY_NULLPTR)
    yyerror (yylocp, yyparser, yymsg);
  YYLONGJMP (yystackp->yyexception_buffer, 1);
}

static _Noreturn void
yyMemoryExhausted (yyGLRStack* yystackp)
{
  YYLONGJMP (yystackp->yyexception_buffer, 2);
}

#if YYDEBUG || YYERROR_VERBOSE
/** A printable representation of TOKEN.  */
static inline const char*
yytokenName (yySymbol yytoken)
{
  if (yytoken == YYEMPTY)
    return "";

  return yytname[yytoken];
}
#endif

/** Fill in YYVSP[YYLOW1 .. YYLOW0-1] from the chain of states starting
 *  at YYVSP[YYLOW0].yystate.yypred.  Leaves YYVSP[YYLOW1].yystate.yypred
 *  containing the pointer to the next state in the chain.  */
static void yyfillin (yyGLRStackItem *, int, int) YY_ATTRIBUTE_UNUSED;
static void
yyfillin (yyGLRStackItem *yyvsp, int yylow0, int yylow1)
{
  int i;
  yyGLRState *s = yyvsp[yylow0].yystate.yypred;
  for (i = yylow0-1; i >= yylow1; i -= 1)
    {
#if YYDEBUG
      yyvsp[i].yystate.yylrState = s->yylrState;
#endif
      yyvsp[i].yystate.yyresolved = s->yyresolved;
      if (s->yyresolved)
        yyvsp[i].yystate.yysemantics.yysval = s->yysemantics.yysval;
      else
        /* The effect of using yysval or yyloc (in an immediate rule) is
         * undefined.  */
        yyvsp[i].yystate.yysemantics.yyfirstVal = YY_NULLPTR;
      yyvsp[i].yystate.yyloc = s->yyloc;
      s = yyvsp[i].yystate.yypred = s->yypred;
    }
}

/* Do nothing if YYNORMAL or if *YYLOW <= YYLOW1.  Otherwise, fill in
 * YYVSP[YYLOW1 .. *YYLOW-1] as in yyfillin and set *YYLOW = YYLOW1.
 * For convenience, always return YYLOW1.  */
static inline int yyfill (yyGLRStackItem *, int *, int, yybool)
     YY_ATTRIBUTE_UNUSED;
static inline int
yyfill (yyGLRStackItem *yyvsp, int *yylow, int yylow1, yybool yynormal)
{
  if (!yynormal && yylow1 < *yylow)
    {
      yyfillin (yyvsp, *yylow, yylow1);
      *yylow = yylow1;
    }
  return yylow1;
}

/** Perform user action for rule number YYN, with RHS length YYRHSLEN,
 *  and top stack item YYVSP.  YYLVALP points to place to put semantic
 *  value ($$), and yylocp points to place for location information
 *  (@$).  Returns yyok for normal return, yyaccept for YYACCEPT,
 *  yyerr for YYERROR, yyabort for YYABORT.  */
static YYRESULTTAG
yyuserAction (yyRuleNum yyn, size_t yyrhslen, yyGLRStackItem* yyvsp,
              yyGLRStack* yystackp,
              YYSTYPE* yyvalp, YYLTYPE *yylocp, yy::parser& yyparser)
{
  yybool yynormal YY_ATTRIBUTE_UNUSED = (yystackp->yysplitPoint == YY_NULLPTR);
  int yylow;
  YYUSE (yyvalp);
  YYUSE (yylocp);
  YYUSE (yyparser);
  YYUSE (yyrhslen);
# undef yyerrok
# define yyerrok (yystackp->yyerrState = 0)
# undef YYACCEPT
# define YYACCEPT return yyaccept
# undef YYABORT
# define YYABORT return yyabort
# undef YYERROR
# define YYERROR return yyerrok, yyerr
# undef YYRECOVERING
# define YYRECOVERING() (yystackp->yyerrState != 0)
# undef yyclearin
# define yyclearin (yychar = YYEMPTY)
# undef YYFILL
# define YYFILL(N) yyfill (yyvsp, &yylow, N, yynormal)
# undef YYBACKUP
# define YYBACKUP(Token, Value)                                              \
  return yyerror (yylocp, yyparser, YY_("syntax error: cannot back up")),     \
         yyerrok, yyerr

  yylow = 1;
  if (yyrhslen == 0)
    *yyvalp = yyval_default;
  else
    *yyvalp = yyvsp[YYFILL (1-yyrhslen)].yystate.yysemantics.yysval;
  YYLLOC_DEFAULT ((*yylocp), (yyvsp - yyrhslen), yyrhslen);
  yystackp->yyerror_range[1].yystate.yyloc = *yylocp;

  switch (yyn)
    {
        case 3:
#line 119 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 1839 "src/parser.cpp" // glr.c:816
    break;

  case 4:
#line 120 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot((((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 1845 "src/parser.cpp" // glr.c:816
    break;

  case 35:
#line 163 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkImportNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 1851 "src/parser.cpp" // glr.c:816
    break;

  case 36:
#line 165 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (Node*)lextxt;}
#line 1857 "src/parser.cpp" // glr.c:816
    break;

  case 37:
#line 168 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (Node*)lextxt;}
#line 1863 "src/parser.cpp" // glr.c:816
    break;

  case 38:
#line 171 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkIntLitNode((*yylocp), lextxt);}
#line 1869 "src/parser.cpp" // glr.c:816
    break;

  case 39:
#line 174 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFltLitNode((*yylocp), lextxt);}
#line 1875 "src/parser.cpp" // glr.c:816
    break;

  case 40:
#line 177 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkStrLitNode((*yylocp), lextxt);}
#line 1881 "src/parser.cpp" // glr.c:816
    break;

  case 41:
#line 180 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_I8,  (char*)"");}
#line 1887 "src/parser.cpp" // glr.c:816
    break;

  case 42:
#line 181 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_I16, (char*)"");}
#line 1893 "src/parser.cpp" // glr.c:816
    break;

  case 43:
#line 182 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_I32, (char*)"");}
#line 1899 "src/parser.cpp" // glr.c:816
    break;

  case 44:
#line 183 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_I64, (char*)"");}
#line 1905 "src/parser.cpp" // glr.c:816
    break;

  case 45:
#line 184 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_U8,  (char*)"");}
#line 1911 "src/parser.cpp" // glr.c:816
    break;

  case 46:
#line 185 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_U16, (char*)"");}
#line 1917 "src/parser.cpp" // glr.c:816
    break;

  case 47:
#line 186 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_U32, (char*)"");}
#line 1923 "src/parser.cpp" // glr.c:816
    break;

  case 48:
#line 187 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_U64, (char*)"");}
#line 1929 "src/parser.cpp" // glr.c:816
    break;

  case 49:
#line 188 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_Isz, (char*)"");}
#line 1935 "src/parser.cpp" // glr.c:816
    break;

  case 50:
#line 189 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_Usz, (char*)"");}
#line 1941 "src/parser.cpp" // glr.c:816
    break;

  case 51:
#line 190 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_F16, (char*)"");}
#line 1947 "src/parser.cpp" // glr.c:816
    break;

  case 52:
#line 191 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_F32, (char*)"");}
#line 1953 "src/parser.cpp" // glr.c:816
    break;

  case 53:
#line 192 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_F64, (char*)"");}
#line 1959 "src/parser.cpp" // glr.c:816
    break;

  case 54:
#line 193 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_C8,  (char*)"");}
#line 1965 "src/parser.cpp" // glr.c:816
    break;

  case 55:
#line 194 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_C32, (char*)"");}
#line 1971 "src/parser.cpp" // glr.c:816
    break;

  case 56:
#line 195 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_Bool, (char*)"");}
#line 1977 "src/parser.cpp" // glr.c:816
    break;

  case 57:
#line 196 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_Void, (char*)"");}
#line 1983 "src/parser.cpp" // glr.c:816
    break;

  case 58:
#line 197 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_Data, (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 1989 "src/parser.cpp" // glr.c:816
    break;

  case 59:
#line 198 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_TypeVar, (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 1995 "src/parser.cpp" // glr.c:816
    break;

  case 60:
#line 203 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_Ptr,  (char*)"", (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2001 "src/parser.cpp" // glr.c:816
    break;

  case 61:
#line 204 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_Array,(char*)"", (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval));}
#line 2007 "src/parser.cpp" // glr.c:816
    break;

  case 62:
#line 205 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_Func, (char*)"", (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval));}
#line 2013 "src/parser.cpp" // glr.c:816
    break;

  case 63:
#line 206 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode((*yylocp), TT_Func, (char*)"", (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval));}
#line 2019 "src/parser.cpp" // glr.c:816
    break;

  case 64:
#line 207 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval);}
#line 2025 "src/parser.cpp" // glr.c:816
    break;

  case 65:
#line 208 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2031 "src/parser.cpp" // glr.c:816
    break;

  case 66:
#line 211 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2037 "src/parser.cpp" // glr.c:816
    break;

  case 68:
#line 213 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot((((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2043 "src/parser.cpp" // glr.c:816
    break;

  case 69:
#line 216 "src/syntax.y" // glr.c:816
    {Node* tmp = getRoot(); 
                        if(tmp == (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)){//singular type, first type in list equals the last
                            ((*yyvalp)) = tmp;
                        }else{ //tuple type
                            ((*yyvalp)) = mkTypeNode((*yylocp), TT_Tuple, (char*)"", tmp);
                        }
                       }
#line 2055 "src/parser.cpp" // glr.c:816
    break;

  case 70:
#line 225 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkModNode((*yylocp), Tok_Pub);}
#line 2061 "src/parser.cpp" // glr.c:816
    break;

  case 71:
#line 226 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkModNode((*yylocp), Tok_Pri);}
#line 2067 "src/parser.cpp" // glr.c:816
    break;

  case 72:
#line 227 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkModNode((*yylocp), Tok_Pro);}
#line 2073 "src/parser.cpp" // glr.c:816
    break;

  case 73:
#line 228 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkModNode((*yylocp), Tok_Raw);}
#line 2079 "src/parser.cpp" // glr.c:816
    break;

  case 74:
#line 229 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkModNode((*yylocp), Tok_Const);}
#line 2085 "src/parser.cpp" // glr.c:816
    break;

  case 75:
#line 230 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkModNode((*yylocp), Tok_Noinit);}
#line 2091 "src/parser.cpp" // glr.c:816
    break;

  case 76:
#line 231 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkModNode((*yylocp), Tok_Pathogen);}
#line 2097 "src/parser.cpp" // glr.c:816
    break;

  case 77:
#line 234 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2103 "src/parser.cpp" // glr.c:816
    break;

  case 78:
#line 235 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot((((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2109 "src/parser.cpp" // glr.c:816
    break;

  case 79:
#line 238 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = getRoot();}
#line 2115 "src/parser.cpp" // glr.c:816
    break;

  case 80:
#line 242 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarDeclNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-4)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2121 "src/parser.cpp" // glr.c:816
    break;

  case 81:
#line 243 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarDeclNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval),  0);}
#line 2127 "src/parser.cpp" // glr.c:816
    break;

  case 82:
#line 244 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarDeclNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), 0,  (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2133 "src/parser.cpp" // glr.c:816
    break;

  case 83:
#line 246 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarDeclNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-4)].yystate.yysemantics.yysval),  0, (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2139 "src/parser.cpp" // glr.c:816
    break;

  case 84:
#line 247 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarDeclNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), 0,   0, (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2145 "src/parser.cpp" // glr.c:816
    break;

  case 85:
#line 250 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkLetBindingNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-4)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2151 "src/parser.cpp" // glr.c:816
    break;

  case 86:
#line 251 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkLetBindingNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), 0,  (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2157 "src/parser.cpp" // glr.c:816
    break;

  case 87:
#line 252 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkLetBindingNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), 0,  (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2163 "src/parser.cpp" // glr.c:816
    break;

  case 88:
#line 253 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkLetBindingNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), 0,  0,  (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2169 "src/parser.cpp" // glr.c:816
    break;

  case 89:
#line 257 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarAssignNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2175 "src/parser.cpp" // glr.c:816
    break;

  case 90:
#line 258 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarAssignNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), mkBinOpNode((*yylocp), '+', mkUnOpNode((*yylocp), '@', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval)), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)), false);}
#line 2181 "src/parser.cpp" // glr.c:816
    break;

  case 91:
#line 259 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarAssignNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), mkBinOpNode((*yylocp), '-', mkUnOpNode((*yylocp), '@', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval)), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)), false);}
#line 2187 "src/parser.cpp" // glr.c:816
    break;

  case 92:
#line 260 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarAssignNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), mkBinOpNode((*yylocp), '*', mkUnOpNode((*yylocp), '@', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval)), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)), false);}
#line 2193 "src/parser.cpp" // glr.c:816
    break;

  case 93:
#line 261 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarAssignNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), mkBinOpNode((*yylocp), '/', mkUnOpNode((*yylocp), '@', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval)), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)), false);}
#line 2199 "src/parser.cpp" // glr.c:816
    break;

  case 94:
#line 264 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2205 "src/parser.cpp" // glr.c:816
    break;

  case 95:
#line 265 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot((((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2211 "src/parser.cpp" // glr.c:816
    break;

  case 96:
#line 268 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = getRoot();}
#line 2217 "src/parser.cpp" // glr.c:816
    break;

  case 97:
#line 271 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkDataDeclNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2223 "src/parser.cpp" // glr.c:816
    break;

  case 98:
#line 272 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkDataDeclNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2229 "src/parser.cpp" // glr.c:816
    break;

  case 99:
#line 273 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkDataDeclNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2235 "src/parser.cpp" // glr.c:816
    break;

  case 100:
#line 274 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkDataDeclNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2241 "src/parser.cpp" // glr.c:816
    break;

  case 101:
#line 277 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkNamedValNode((*yylocp), mkVarNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2247 "src/parser.cpp" // glr.c:816
    break;

  case 102:
#line 278 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkNamedValNode((*yylocp), 0, (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2253 "src/parser.cpp" // glr.c:816
    break;

  case 104:
#line 282 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2259 "src/parser.cpp" // glr.c:816
    break;

  case 105:
#line 283 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot((((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2265 "src/parser.cpp" // glr.c:816
    break;

  case 106:
#line 286 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = getRoot();}
#line 2271 "src/parser.cpp" // glr.c:816
    break;

  case 112:
#line 299 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = NULL;}
#line 2277 "src/parser.cpp" // glr.c:816
    break;

  case 113:
#line 300 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = NULL;}
#line 2283 "src/parser.cpp" // glr.c:816
    break;

  case 114:
#line 301 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = NULL;}
#line 2289 "src/parser.cpp" // glr.c:816
    break;

  case 115:
#line 302 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = NULL;}
#line 2295 "src/parser.cpp" // glr.c:816
    break;

  case 116:
#line 305 "src/syntax.y" // glr.c:816
    {setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval)); ((*yyvalp)) = getRoot();}
#line 2301 "src/parser.cpp" // glr.c:816
    break;

  case 117:
#line 306 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval);}
#line 2307 "src/parser.cpp" // glr.c:816
    break;

  case 118:
#line 309 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), mkVarNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)));}
#line 2313 "src/parser.cpp" // glr.c:816
    break;

  case 119:
#line 310 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot(mkVarNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)));}
#line 2319 "src/parser.cpp" // glr.c:816
    break;

  case 120:
#line 313 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = getRoot();}
#line 2325 "src/parser.cpp" // glr.c:816
    break;

  case 121:
#line 320 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), mkNamedValNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval)));}
#line 2331 "src/parser.cpp" // glr.c:816
    break;

  case 122:
#line 321 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot(mkNamedValNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval)));}
#line 2337 "src/parser.cpp" // glr.c:816
    break;

  case 123:
#line 325 "src/syntax.y" // glr.c:816
    {setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), mkNamedValNode((*yylocp), mkVarNode((*yylocp), (char*)""), 0)); ((*yyvalp)) = getRoot();}
#line 2343 "src/parser.cpp" // glr.c:816
    break;

  case 124:
#line 326 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = getRoot();}
#line 2349 "src/parser.cpp" // glr.c:816
    break;

  case 125:
#line 330 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2355 "src/parser.cpp" // glr.c:816
    break;

  case 126:
#line 331 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = 0;}
#line 2361 "src/parser.cpp" // glr.c:816
    break;

  case 127:
#line 334 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFuncDeclNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-5)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-7)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval),                             (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2367 "src/parser.cpp" // glr.c:816
    break;

  case 128:
#line 335 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFuncDeclNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-5)].yystate.yysemantics.yysval), mkTypeNode((*yylocp), TT_Void, (char*)""), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2373 "src/parser.cpp" // glr.c:816
    break;

  case 129:
#line 336 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFuncDeclNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-5)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval),                              0, (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2379 "src/parser.cpp" // glr.c:816
    break;

  case 130:
#line 337 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFuncDeclNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), mkTypeNode((*yylocp), TT_Void, (char*)""),  0, (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2385 "src/parser.cpp" // glr.c:816
    break;

  case 131:
#line 338 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFuncDeclNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-5)].yystate.yysemantics.yysval),  0, (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval),                             (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2391 "src/parser.cpp" // glr.c:816
    break;

  case 132:
#line 339 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFuncDeclNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval),  0, mkTypeNode((*yylocp), TT_Void, (char*)""), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2397 "src/parser.cpp" // glr.c:816
    break;

  case 133:
#line 340 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFuncDeclNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval),  0, (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval),                              0, (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2403 "src/parser.cpp" // glr.c:816
    break;

  case 134:
#line 341 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFuncDeclNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval),  0, mkTypeNode((*yylocp), TT_Void, (char*)""),  0, (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2409 "src/parser.cpp" // glr.c:816
    break;

  case 135:
#line 345 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFuncCallNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2415 "src/parser.cpp" // glr.c:816
    break;

  case 136:
#line 348 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkRetNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2421 "src/parser.cpp" // glr.c:816
    break;

  case 137:
#line 352 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkExtNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2427 "src/parser.cpp" // glr.c:816
    break;

  case 138:
#line 356 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = getRoot();}
#line 2433 "src/parser.cpp" // glr.c:816
    break;

  case 139:
#line 358 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2439 "src/parser.cpp" // glr.c:816
    break;

  case 140:
#line 359 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot((((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2445 "src/parser.cpp" // glr.c:816
    break;

  case 141:
#line 368 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setElse((IfNode*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-4)].yystate.yysemantics.yysval), (IfNode*)mkIfNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)));}
#line 2451 "src/parser.cpp" // glr.c:816
    break;

  case 142:
#line 369 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot(mkIfNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)));}
#line 2457 "src/parser.cpp" // glr.c:816
    break;

  case 143:
#line 372 "src/syntax.y" // glr.c:816
    {setElse((IfNode*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-4)].yystate.yysemantics.yysval), (IfNode*)mkIfNode((*yylocp), NULL, (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval)));}
#line 2463 "src/parser.cpp" // glr.c:816
    break;

  case 144:
#line 373 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot((((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2469 "src/parser.cpp" // glr.c:816
    break;

  case 145:
#line 374 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot(mkIfNode((*yylocp), NULL, (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval)));}
#line 2475 "src/parser.cpp" // glr.c:816
    break;

  case 146:
#line 375 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot(NULL);}
#line 2481 "src/parser.cpp" // glr.c:816
    break;

  case 147:
#line 378 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkIfNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (IfNode*)getRoot());}
#line 2487 "src/parser.cpp" // glr.c:816
    break;

  case 148:
#line 381 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkWhileNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2493 "src/parser.cpp" // glr.c:816
    break;

  case 149:
#line 384 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = NULL;}
#line 2499 "src/parser.cpp" // glr.c:816
    break;

  case 150:
#line 387 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = NULL;}
#line 2505 "src/parser.cpp" // glr.c:816
    break;

  case 151:
#line 390 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2511 "src/parser.cpp" // glr.c:816
    break;

  case 152:
#line 393 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkUnOpNode((*yylocp), '&', (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2517 "src/parser.cpp" // glr.c:816
    break;

  case 153:
#line 394 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkUnOpNode((*yylocp), '@', (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2523 "src/parser.cpp" // glr.c:816
    break;

  case 154:
#line 395 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '[', mkRefVarNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval)), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2529 "src/parser.cpp" // glr.c:816
    break;

  case 155:
#line 396 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkRefVarNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2535 "src/parser.cpp" // glr.c:816
    break;

  case 156:
#line 399 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2541 "src/parser.cpp" // glr.c:816
    break;

  case 157:
#line 400 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval);}
#line 2547 "src/parser.cpp" // glr.c:816
    break;

  case 158:
#line 401 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2553 "src/parser.cpp" // glr.c:816
    break;

  case 159:
#line 402 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2559 "src/parser.cpp" // glr.c:816
    break;

  case 160:
#line 403 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2565 "src/parser.cpp" // glr.c:816
    break;

  case 161:
#line 404 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2571 "src/parser.cpp" // glr.c:816
    break;

  case 162:
#line 405 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2577 "src/parser.cpp" // glr.c:816
    break;

  case 163:
#line 406 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2583 "src/parser.cpp" // glr.c:816
    break;

  case 164:
#line 407 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2589 "src/parser.cpp" // glr.c:816
    break;

  case 165:
#line 408 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBoolLitNode((*yylocp), 1);}
#line 2595 "src/parser.cpp" // glr.c:816
    break;

  case 166:
#line 409 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBoolLitNode((*yylocp), 0);}
#line 2601 "src/parser.cpp" // glr.c:816
    break;

  case 167:
#line 412 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTupleNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2607 "src/parser.cpp" // glr.c:816
    break;

  case 168:
#line 413 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTupleNode((*yylocp), 0);}
#line 2613 "src/parser.cpp" // glr.c:816
    break;

  case 169:
#line 416 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkArrayNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2619 "src/parser.cpp" // glr.c:816
    break;

  case 170:
#line 417 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkArrayNode((*yylocp), 0);}
#line 2625 "src/parser.cpp" // glr.c:816
    break;

  case 171:
#line 421 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkUnOpNode((*yylocp), '@', (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2631 "src/parser.cpp" // glr.c:816
    break;

  case 172:
#line 422 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkUnOpNode((*yylocp), '&', (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2637 "src/parser.cpp" // glr.c:816
    break;

  case 173:
#line 423 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkUnOpNode((*yylocp), '-', (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2643 "src/parser.cpp" // glr.c:816
    break;

  case 174:
#line 424 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeCastNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2649 "src/parser.cpp" // glr.c:816
    break;

  case 175:
#line 427 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2655 "src/parser.cpp" // glr.c:816
    break;

  case 176:
#line 429 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '+', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2661 "src/parser.cpp" // glr.c:816
    break;

  case 177:
#line 430 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '-', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2667 "src/parser.cpp" // glr.c:816
    break;

  case 178:
#line 431 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '*', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2673 "src/parser.cpp" // glr.c:816
    break;

  case 179:
#line 432 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '/', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2679 "src/parser.cpp" // glr.c:816
    break;

  case 180:
#line 433 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '%', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2685 "src/parser.cpp" // glr.c:816
    break;

  case 181:
#line 434 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '<', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2691 "src/parser.cpp" // glr.c:816
    break;

  case 182:
#line 435 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '>', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2697 "src/parser.cpp" // glr.c:816
    break;

  case 183:
#line 436 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '^', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2703 "src/parser.cpp" // glr.c:816
    break;

  case 184:
#line 437 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '.', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2709 "src/parser.cpp" // glr.c:816
    break;

  case 185:
#line 438 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '.', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2715 "src/parser.cpp" // glr.c:816
    break;

  case 186:
#line 439 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), ';', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2721 "src/parser.cpp" // glr.c:816
    break;

  case 187:
#line 440 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '[', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2727 "src/parser.cpp" // glr.c:816
    break;

  case 188:
#line 441 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), Tok_Let, mkLetBindingNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-4)].yystate.yysemantics.yysval), 0, 0, (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval)), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2733 "src/parser.cpp" // glr.c:816
    break;

  case 189:
#line 442 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), Tok_Eq, (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2739 "src/parser.cpp" // glr.c:816
    break;

  case 190:
#line 443 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), Tok_NotEq, (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2745 "src/parser.cpp" // glr.c:816
    break;

  case 191:
#line 444 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), Tok_GrtrEq, (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2751 "src/parser.cpp" // glr.c:816
    break;

  case 192:
#line 445 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), Tok_LesrEq, (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2757 "src/parser.cpp" // glr.c:816
    break;

  case 193:
#line 446 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), Tok_Or, (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2763 "src/parser.cpp" // glr.c:816
    break;

  case 194:
#line 447 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), Tok_And, (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2769 "src/parser.cpp" // glr.c:816
    break;

  case 195:
#line 448 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), Tok_Range, (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2775 "src/parser.cpp" // glr.c:816
    break;

  case 196:
#line 449 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '(', (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2781 "src/parser.cpp" // glr.c:816
    break;

  case 197:
#line 450 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkExprIfNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (-4)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2787 "src/parser.cpp" // glr.c:816
    break;

  case 198:
#line 451 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2793 "src/parser.cpp" // glr.c:816
    break;

  case 199:
#line 452 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval);}
#line 2799 "src/parser.cpp" // glr.c:816
    break;

  case 200:
#line 457 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = getRoot();}
#line 2805 "src/parser.cpp" // glr.c:816
    break;

  case 201:
#line 460 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2811 "src/parser.cpp" // glr.c:816
    break;

  case 202:
#line 461 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot((((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2817 "src/parser.cpp" // glr.c:816
    break;

  case 203:
#line 465 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '+', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2823 "src/parser.cpp" // glr.c:816
    break;

  case 204:
#line 466 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '-', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2829 "src/parser.cpp" // glr.c:816
    break;

  case 205:
#line 467 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '*', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2835 "src/parser.cpp" // glr.c:816
    break;

  case 206:
#line 468 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '/', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2841 "src/parser.cpp" // glr.c:816
    break;

  case 207:
#line 469 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '%', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2847 "src/parser.cpp" // glr.c:816
    break;

  case 208:
#line 470 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '<', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2853 "src/parser.cpp" // glr.c:816
    break;

  case 209:
#line 471 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '>', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2859 "src/parser.cpp" // glr.c:816
    break;

  case 210:
#line 472 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '^', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2865 "src/parser.cpp" // glr.c:816
    break;

  case 211:
#line 473 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '.', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2871 "src/parser.cpp" // glr.c:816
    break;

  case 212:
#line 474 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '.', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2877 "src/parser.cpp" // glr.c:816
    break;

  case 213:
#line 475 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), ';', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2883 "src/parser.cpp" // glr.c:816
    break;

  case 214:
#line 476 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '[', (((yyGLRStackItem const *)yyvsp)[YYFILL (-4)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval));}
#line 2889 "src/parser.cpp" // glr.c:816
    break;

  case 215:
#line 477 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), Tok_Let, mkLetBindingNode((*yylocp), (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-5)].yystate.yysemantics.yysval), 0, 0, (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval)), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2895 "src/parser.cpp" // glr.c:816
    break;

  case 216:
#line 478 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), Tok_Eq, (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2901 "src/parser.cpp" // glr.c:816
    break;

  case 217:
#line 479 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), Tok_NotEq, (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2907 "src/parser.cpp" // glr.c:816
    break;

  case 218:
#line 480 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), Tok_GrtrEq, (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2913 "src/parser.cpp" // glr.c:816
    break;

  case 219:
#line 481 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), Tok_LesrEq, (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2919 "src/parser.cpp" // glr.c:816
    break;

  case 220:
#line 482 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), Tok_Or, (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2925 "src/parser.cpp" // glr.c:816
    break;

  case 221:
#line 483 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), Tok_And, (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2931 "src/parser.cpp" // glr.c:816
    break;

  case 222:
#line 484 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode((*yylocp), '(', (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2937 "src/parser.cpp" // glr.c:816
    break;

  case 223:
#line 485 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkExprIfNode((*yylocp), (((yyGLRStackItem const *)yyvsp)[YYFILL (-4)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2943 "src/parser.cpp" // glr.c:816
    break;

  case 224:
#line 486 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2949 "src/parser.cpp" // glr.c:816
    break;

  case 225:
#line 487 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval);}
#line 2955 "src/parser.cpp" // glr.c:816
    break;


#line 2959 "src/parser.cpp" // glr.c:816
      default: break;
    }

  return yyok;
# undef yyerrok
# undef YYABORT
# undef YYACCEPT
# undef YYERROR
# undef YYBACKUP
# undef yyclearin
# undef YYRECOVERING
}


static void
yyuserMerge (int yyn, YYSTYPE* yy0, YYSTYPE* yy1)
{
  YYUSE (yy0);
  YYUSE (yy1);

  switch (yyn)
    {

      default: break;
    }
}

                              /* Bison grammar-table manipulation.  */

/*-----------------------------------------------.
| Release the memory associated to this symbol.  |
`-----------------------------------------------*/

static void
yydestruct (const char *yymsg, int yytype, YYSTYPE *yyvaluep, YYLTYPE *yylocationp, yy::parser& yyparser)
{
  YYUSE (yyvaluep);
  YYUSE (yylocationp);
  YYUSE (yyparser);
  if (!yymsg)
    yymsg = "Deleting";
  YY_SYMBOL_PRINT (yymsg, yytype, yyvaluep, yylocationp);

  YY_IGNORE_MAYBE_UNINITIALIZED_BEGIN
  YYUSE (yytype);
  YY_IGNORE_MAYBE_UNINITIALIZED_END
}

/** Number of symbols composing the right hand side of rule #RULE.  */
static inline int
yyrhsLength (yyRuleNum yyrule)
{
  return yyr2[yyrule];
}

static void
yydestroyGLRState (char const *yymsg, yyGLRState *yys, yy::parser& yyparser)
{
  if (yys->yyresolved)
    yydestruct (yymsg, yystos[yys->yylrState],
                &yys->yysemantics.yysval, &yys->yyloc, yyparser);
  else
    {
#if YYDEBUG
      if (yydebug)
        {
          if (yys->yysemantics.yyfirstVal)
            YYFPRINTF (stderr, "%s unresolved", yymsg);
          else
            YYFPRINTF (stderr, "%s incomplete", yymsg);
          YY_SYMBOL_PRINT ("", yystos[yys->yylrState], YY_NULLPTR, &yys->yyloc);
        }
#endif

      if (yys->yysemantics.yyfirstVal)
        {
          yySemanticOption *yyoption = yys->yysemantics.yyfirstVal;
          yyGLRState *yyrh;
          int yyn;
          for (yyrh = yyoption->yystate, yyn = yyrhsLength (yyoption->yyrule);
               yyn > 0;
               yyrh = yyrh->yypred, yyn -= 1)
            yydestroyGLRState (yymsg, yyrh, yyparser);
        }
    }
}

/** Left-hand-side symbol for rule #YYRULE.  */
static inline yySymbol
yylhsNonterm (yyRuleNum yyrule)
{
  return yyr1[yyrule];
}

#define yypact_value_is_default(Yystate) \
  (!!((Yystate) == (-254)))

/** True iff LR state YYSTATE has only a default reduction (regardless
 *  of token).  */
static inline yybool
yyisDefaultedState (yyStateNum yystate)
{
  return yypact_value_is_default (yypact[yystate]);
}

/** The default reduction for YYSTATE, assuming it has one.  */
static inline yyRuleNum
yydefaultAction (yyStateNum yystate)
{
  return yydefact[yystate];
}

#define yytable_value_is_error(Yytable_value) \
  0

/** Set *YYACTION to the action to take in YYSTATE on seeing YYTOKEN.
 *  Result R means
 *    R < 0:  Reduce on rule -R.
 *    R = 0:  Error.
 *    R > 0:  Shift to state R.
 *  Set *YYCONFLICTS to a pointer into yyconfl to a 0-terminated list
 *  of conflicting reductions.
 */
static inline void
yygetLRActions (yyStateNum yystate, int yytoken,
                int* yyaction, const short int** yyconflicts)
{
  int yyindex = yypact[yystate] + yytoken;
  if (yypact_value_is_default (yypact[yystate])
      || yyindex < 0 || YYLAST < yyindex || yycheck[yyindex] != yytoken)
    {
      *yyaction = -yydefact[yystate];
      *yyconflicts = yyconfl;
    }
  else if (! yytable_value_is_error (yytable[yyindex]))
    {
      *yyaction = yytable[yyindex];
      *yyconflicts = yyconfl + yyconflp[yyindex];
    }
  else
    {
      *yyaction = 0;
      *yyconflicts = yyconfl + yyconflp[yyindex];
    }
}

/** Compute post-reduction state.
 * \param yystate   the current state
 * \param yysym     the nonterminal to push on the stack
 */
static inline yyStateNum
yyLRgotoState (yyStateNum yystate, yySymbol yysym)
{
  int yyr = yypgoto[yysym - YYNTOKENS] + yystate;
  if (0 <= yyr && yyr <= YYLAST && yycheck[yyr] == yystate)
    return yytable[yyr];
  else
    return yydefgoto[yysym - YYNTOKENS];
}

static inline yybool
yyisShiftAction (int yyaction)
{
  return 0 < yyaction;
}

static inline yybool
yyisErrorAction (int yyaction)
{
  return yyaction == 0;
}

                                /* GLRStates */

/** Return a fresh GLRStackItem in YYSTACKP.  The item is an LR state
 *  if YYISSTATE, and otherwise a semantic option.  Callers should call
 *  YY_RESERVE_GLRSTACK afterwards to make sure there is sufficient
 *  headroom.  */

static inline yyGLRStackItem*
yynewGLRStackItem (yyGLRStack* yystackp, yybool yyisState)
{
  yyGLRStackItem* yynewItem = yystackp->yynextFree;
  yystackp->yyspaceLeft -= 1;
  yystackp->yynextFree += 1;
  yynewItem->yystate.yyisState = yyisState;
  return yynewItem;
}

/** Add a new semantic action that will execute the action for rule
 *  YYRULE on the semantic values in YYRHS to the list of
 *  alternative actions for YYSTATE.  Assumes that YYRHS comes from
 *  stack #YYK of *YYSTACKP. */
static void
yyaddDeferredAction (yyGLRStack* yystackp, size_t yyk, yyGLRState* yystate,
                     yyGLRState* yyrhs, yyRuleNum yyrule)
{
  yySemanticOption* yynewOption =
    &yynewGLRStackItem (yystackp, yyfalse)->yyoption;
  YYASSERT (!yynewOption->yyisState);
  yynewOption->yystate = yyrhs;
  yynewOption->yyrule = yyrule;
  if (yystackp->yytops.yylookaheadNeeds[yyk])
    {
      yynewOption->yyrawchar = yychar;
      yynewOption->yyval = yylval;
      yynewOption->yyloc = yylloc;
    }
  else
    yynewOption->yyrawchar = YYEMPTY;
  yynewOption->yynext = yystate->yysemantics.yyfirstVal;
  yystate->yysemantics.yyfirstVal = yynewOption;

  YY_RESERVE_GLRSTACK (yystackp);
}

                                /* GLRStacks */

/** Initialize YYSET to a singleton set containing an empty stack.  */
static yybool
yyinitStateSet (yyGLRStateSet* yyset)
{
  yyset->yysize = 1;
  yyset->yycapacity = 16;
  yyset->yystates = (yyGLRState**) YYMALLOC (16 * sizeof yyset->yystates[0]);
  if (! yyset->yystates)
    return yyfalse;
  yyset->yystates[0] = YY_NULLPTR;
  yyset->yylookaheadNeeds =
    (yybool*) YYMALLOC (16 * sizeof yyset->yylookaheadNeeds[0]);
  if (! yyset->yylookaheadNeeds)
    {
      YYFREE (yyset->yystates);
      return yyfalse;
    }
  return yytrue;
}

static void yyfreeStateSet (yyGLRStateSet* yyset)
{
  YYFREE (yyset->yystates);
  YYFREE (yyset->yylookaheadNeeds);
}

/** Initialize *YYSTACKP to a single empty stack, with total maximum
 *  capacity for all stacks of YYSIZE.  */
static yybool
yyinitGLRStack (yyGLRStack* yystackp, size_t yysize)
{
  yystackp->yyerrState = 0;
  yynerrs = 0;
  yystackp->yyspaceLeft = yysize;
  yystackp->yyitems =
    (yyGLRStackItem*) YYMALLOC (yysize * sizeof yystackp->yynextFree[0]);
  if (!yystackp->yyitems)
    return yyfalse;
  yystackp->yynextFree = yystackp->yyitems;
  yystackp->yysplitPoint = YY_NULLPTR;
  yystackp->yylastDeleted = YY_NULLPTR;
  return yyinitStateSet (&yystackp->yytops);
}


#if YYSTACKEXPANDABLE
# define YYRELOC(YYFROMITEMS,YYTOITEMS,YYX,YYTYPE) \
  &((YYTOITEMS) - ((YYFROMITEMS) - (yyGLRStackItem*) (YYX)))->YYTYPE

/** If *YYSTACKP is expandable, extend it.  WARNING: Pointers into the
    stack from outside should be considered invalid after this call.
    We always expand when there are 1 or fewer items left AFTER an
    allocation, so that we can avoid having external pointers exist
    across an allocation.  */
static void
yyexpandGLRStack (yyGLRStack* yystackp)
{
  yyGLRStackItem* yynewItems;
  yyGLRStackItem* yyp0, *yyp1;
  size_t yynewSize;
  size_t yyn;
  size_t yysize = yystackp->yynextFree - yystackp->yyitems;
  if (YYMAXDEPTH - YYHEADROOM < yysize)
    yyMemoryExhausted (yystackp);
  yynewSize = 2*yysize;
  if (YYMAXDEPTH < yynewSize)
    yynewSize = YYMAXDEPTH;
  yynewItems = (yyGLRStackItem*) YYMALLOC (yynewSize * sizeof yynewItems[0]);
  if (! yynewItems)
    yyMemoryExhausted (yystackp);
  for (yyp0 = yystackp->yyitems, yyp1 = yynewItems, yyn = yysize;
       0 < yyn;
       yyn -= 1, yyp0 += 1, yyp1 += 1)
    {
      *yyp1 = *yyp0;
      if (*(yybool *) yyp0)
        {
          yyGLRState* yys0 = &yyp0->yystate;
          yyGLRState* yys1 = &yyp1->yystate;
          if (yys0->yypred != YY_NULLPTR)
            yys1->yypred =
              YYRELOC (yyp0, yyp1, yys0->yypred, yystate);
          if (! yys0->yyresolved && yys0->yysemantics.yyfirstVal != YY_NULLPTR)
            yys1->yysemantics.yyfirstVal =
              YYRELOC (yyp0, yyp1, yys0->yysemantics.yyfirstVal, yyoption);
        }
      else
        {
          yySemanticOption* yyv0 = &yyp0->yyoption;
          yySemanticOption* yyv1 = &yyp1->yyoption;
          if (yyv0->yystate != YY_NULLPTR)
            yyv1->yystate = YYRELOC (yyp0, yyp1, yyv0->yystate, yystate);
          if (yyv0->yynext != YY_NULLPTR)
            yyv1->yynext = YYRELOC (yyp0, yyp1, yyv0->yynext, yyoption);
        }
    }
  if (yystackp->yysplitPoint != YY_NULLPTR)
    yystackp->yysplitPoint = YYRELOC (yystackp->yyitems, yynewItems,
                                      yystackp->yysplitPoint, yystate);

  for (yyn = 0; yyn < yystackp->yytops.yysize; yyn += 1)
    if (yystackp->yytops.yystates[yyn] != YY_NULLPTR)
      yystackp->yytops.yystates[yyn] =
        YYRELOC (yystackp->yyitems, yynewItems,
                 yystackp->yytops.yystates[yyn], yystate);
  YYFREE (yystackp->yyitems);
  yystackp->yyitems = yynewItems;
  yystackp->yynextFree = yynewItems + yysize;
  yystackp->yyspaceLeft = yynewSize - yysize;
}
#endif

static void
yyfreeGLRStack (yyGLRStack* yystackp)
{
  YYFREE (yystackp->yyitems);
  yyfreeStateSet (&yystackp->yytops);
}

/** Assuming that YYS is a GLRState somewhere on *YYSTACKP, update the
 *  splitpoint of *YYSTACKP, if needed, so that it is at least as deep as
 *  YYS.  */
static inline void
yyupdateSplit (yyGLRStack* yystackp, yyGLRState* yys)
{
  if (yystackp->yysplitPoint != YY_NULLPTR && yystackp->yysplitPoint > yys)
    yystackp->yysplitPoint = yys;
}

/** Invalidate stack #YYK in *YYSTACKP.  */
static inline void
yymarkStackDeleted (yyGLRStack* yystackp, size_t yyk)
{
  if (yystackp->yytops.yystates[yyk] != YY_NULLPTR)
    yystackp->yylastDeleted = yystackp->yytops.yystates[yyk];
  yystackp->yytops.yystates[yyk] = YY_NULLPTR;
}

/** Undelete the last stack in *YYSTACKP that was marked as deleted.  Can
    only be done once after a deletion, and only when all other stacks have
    been deleted.  */
static void
yyundeleteLastStack (yyGLRStack* yystackp)
{
  if (yystackp->yylastDeleted == YY_NULLPTR || yystackp->yytops.yysize != 0)
    return;
  yystackp->yytops.yystates[0] = yystackp->yylastDeleted;
  yystackp->yytops.yysize = 1;
  YYDPRINTF ((stderr, "Restoring last deleted stack as stack #0.\n"));
  yystackp->yylastDeleted = YY_NULLPTR;
}

static inline void
yyremoveDeletes (yyGLRStack* yystackp)
{
  size_t yyi, yyj;
  yyi = yyj = 0;
  while (yyj < yystackp->yytops.yysize)
    {
      if (yystackp->yytops.yystates[yyi] == YY_NULLPTR)
        {
          if (yyi == yyj)
            {
              YYDPRINTF ((stderr, "Removing dead stacks.\n"));
            }
          yystackp->yytops.yysize -= 1;
        }
      else
        {
          yystackp->yytops.yystates[yyj] = yystackp->yytops.yystates[yyi];
          /* In the current implementation, it's unnecessary to copy
             yystackp->yytops.yylookaheadNeeds[yyi] since, after
             yyremoveDeletes returns, the parser immediately either enters
             deterministic operation or shifts a token.  However, it doesn't
             hurt, and the code might evolve to need it.  */
          yystackp->yytops.yylookaheadNeeds[yyj] =
            yystackp->yytops.yylookaheadNeeds[yyi];
          if (yyj != yyi)
            {
              YYDPRINTF ((stderr, "Rename stack %lu -> %lu.\n",
                          (unsigned long int) yyi, (unsigned long int) yyj));
            }
          yyj += 1;
        }
      yyi += 1;
    }
}

/** Shift to a new state on stack #YYK of *YYSTACKP, corresponding to LR
 * state YYLRSTATE, at input position YYPOSN, with (resolved) semantic
 * value *YYVALP and source location *YYLOCP.  */
static inline void
yyglrShift (yyGLRStack* yystackp, size_t yyk, yyStateNum yylrState,
            size_t yyposn,
            YYSTYPE* yyvalp, YYLTYPE* yylocp)
{
  yyGLRState* yynewState = &yynewGLRStackItem (yystackp, yytrue)->yystate;

  yynewState->yylrState = yylrState;
  yynewState->yyposn = yyposn;
  yynewState->yyresolved = yytrue;
  yynewState->yypred = yystackp->yytops.yystates[yyk];
  yynewState->yysemantics.yysval = *yyvalp;
  yynewState->yyloc = *yylocp;
  yystackp->yytops.yystates[yyk] = yynewState;

  YY_RESERVE_GLRSTACK (yystackp);
}

/** Shift stack #YYK of *YYSTACKP, to a new state corresponding to LR
 *  state YYLRSTATE, at input position YYPOSN, with the (unresolved)
 *  semantic value of YYRHS under the action for YYRULE.  */
static inline void
yyglrShiftDefer (yyGLRStack* yystackp, size_t yyk, yyStateNum yylrState,
                 size_t yyposn, yyGLRState* yyrhs, yyRuleNum yyrule)
{
  yyGLRState* yynewState = &yynewGLRStackItem (yystackp, yytrue)->yystate;
  YYASSERT (yynewState->yyisState);

  yynewState->yylrState = yylrState;
  yynewState->yyposn = yyposn;
  yynewState->yyresolved = yyfalse;
  yynewState->yypred = yystackp->yytops.yystates[yyk];
  yynewState->yysemantics.yyfirstVal = YY_NULLPTR;
  yystackp->yytops.yystates[yyk] = yynewState;

  /* Invokes YY_RESERVE_GLRSTACK.  */
  yyaddDeferredAction (yystackp, yyk, yynewState, yyrhs, yyrule);
}

#if !YYDEBUG
# define YY_REDUCE_PRINT(Args)
#else
# define YY_REDUCE_PRINT(Args)          \
do {                                    \
  if (yydebug)                          \
    yy_reduce_print Args;               \
} while (0)

/*----------------------------------------------------------------------.
| Report that stack #YYK of *YYSTACKP is going to be reduced by YYRULE. |
`----------------------------------------------------------------------*/

static inline void
yy_reduce_print (int yynormal, yyGLRStackItem* yyvsp, size_t yyk,
                 yyRuleNum yyrule, yy::parser& yyparser)
{
  int yynrhs = yyrhsLength (yyrule);
  int yylow = 1;
  int yyi;
  YYFPRINTF (stderr, "Reducing stack %lu by rule %d (line %lu):\n",
             (unsigned long int) yyk, yyrule - 1,
             (unsigned long int) yyrline[yyrule]);
  if (! yynormal)
    yyfillin (yyvsp, 1, -yynrhs);
  /* The symbols being reduced.  */
  for (yyi = 0; yyi < yynrhs; yyi++)
    {
      YYFPRINTF (stderr, "   $%d = ", yyi + 1);
      yy_symbol_print (stderr,
                       yystos[yyvsp[yyi - yynrhs + 1].yystate.yylrState],
                       &yyvsp[yyi - yynrhs + 1].yystate.yysemantics.yysval
                       , &(((yyGLRStackItem const *)yyvsp)[YYFILL ((yyi + 1) - (yynrhs))].yystate.yyloc)                       , yyparser);
      if (!yyvsp[yyi - yynrhs + 1].yystate.yyresolved)
        YYFPRINTF (stderr, " (unresolved)");
      YYFPRINTF (stderr, "\n");
    }
}
#endif

/** Pop the symbols consumed by reduction #YYRULE from the top of stack
 *  #YYK of *YYSTACKP, and perform the appropriate semantic action on their
 *  semantic values.  Assumes that all ambiguities in semantic values
 *  have been previously resolved.  Set *YYVALP to the resulting value,
 *  and *YYLOCP to the computed location (if any).  Return value is as
 *  for userAction.  */
static inline YYRESULTTAG
yydoAction (yyGLRStack* yystackp, size_t yyk, yyRuleNum yyrule,
            YYSTYPE* yyvalp, YYLTYPE *yylocp, yy::parser& yyparser)
{
  int yynrhs = yyrhsLength (yyrule);

  if (yystackp->yysplitPoint == YY_NULLPTR)
    {
      /* Standard special case: single stack.  */
      yyGLRStackItem* yyrhs = (yyGLRStackItem*) yystackp->yytops.yystates[yyk];
      YYASSERT (yyk == 0);
      yystackp->yynextFree -= yynrhs;
      yystackp->yyspaceLeft += yynrhs;
      yystackp->yytops.yystates[0] = & yystackp->yynextFree[-1].yystate;
      YY_REDUCE_PRINT ((1, yyrhs, yyk, yyrule, yyparser));
      return yyuserAction (yyrule, yynrhs, yyrhs, yystackp,
                           yyvalp, yylocp, yyparser);
    }
  else
    {
      int yyi;
      yyGLRState* yys;
      yyGLRStackItem yyrhsVals[YYMAXRHS + YYMAXLEFT + 1];
      yys = yyrhsVals[YYMAXRHS + YYMAXLEFT].yystate.yypred
        = yystackp->yytops.yystates[yyk];
      if (yynrhs == 0)
        /* Set default location.  */
        yyrhsVals[YYMAXRHS + YYMAXLEFT - 1].yystate.yyloc = yys->yyloc;
      for (yyi = 0; yyi < yynrhs; yyi += 1)
        {
          yys = yys->yypred;
          YYASSERT (yys);
        }
      yyupdateSplit (yystackp, yys);
      yystackp->yytops.yystates[yyk] = yys;
      YY_REDUCE_PRINT ((0, yyrhsVals + YYMAXRHS + YYMAXLEFT - 1, yyk, yyrule, yyparser));
      return yyuserAction (yyrule, yynrhs, yyrhsVals + YYMAXRHS + YYMAXLEFT - 1,
                           yystackp, yyvalp, yylocp, yyparser);
    }
}

/** Pop items off stack #YYK of *YYSTACKP according to grammar rule YYRULE,
 *  and push back on the resulting nonterminal symbol.  Perform the
 *  semantic action associated with YYRULE and store its value with the
 *  newly pushed state, if YYFORCEEVAL or if *YYSTACKP is currently
 *  unambiguous.  Otherwise, store the deferred semantic action with
 *  the new state.  If the new state would have an identical input
 *  position, LR state, and predecessor to an existing state on the stack,
 *  it is identified with that existing state, eliminating stack #YYK from
 *  *YYSTACKP.  In this case, the semantic value is
 *  added to the options for the existing state's semantic value.
 */
static inline YYRESULTTAG
yyglrReduce (yyGLRStack* yystackp, size_t yyk, yyRuleNum yyrule,
             yybool yyforceEval, yy::parser& yyparser)
{
  size_t yyposn = yystackp->yytops.yystates[yyk]->yyposn;

  if (yyforceEval || yystackp->yysplitPoint == YY_NULLPTR)
    {
      YYSTYPE yysval;
      YYLTYPE yyloc;

      YYRESULTTAG yyflag = yydoAction (yystackp, yyk, yyrule, &yysval, &yyloc, yyparser);
      if (yyflag == yyerr && yystackp->yysplitPoint != YY_NULLPTR)
        {
          YYDPRINTF ((stderr, "Parse on stack %lu rejected by rule #%d.\n",
                     (unsigned long int) yyk, yyrule - 1));
        }
      if (yyflag != yyok)
        return yyflag;
      YY_SYMBOL_PRINT ("-> $$ =", yyr1[yyrule], &yysval, &yyloc);
      yyglrShift (yystackp, yyk,
                  yyLRgotoState (yystackp->yytops.yystates[yyk]->yylrState,
                                 yylhsNonterm (yyrule)),
                  yyposn, &yysval, &yyloc);
    }
  else
    {
      size_t yyi;
      int yyn;
      yyGLRState* yys, *yys0 = yystackp->yytops.yystates[yyk];
      yyStateNum yynewLRState;

      for (yys = yystackp->yytops.yystates[yyk], yyn = yyrhsLength (yyrule);
           0 < yyn; yyn -= 1)
        {
          yys = yys->yypred;
          YYASSERT (yys);
        }
      yyupdateSplit (yystackp, yys);
      yynewLRState = yyLRgotoState (yys->yylrState, yylhsNonterm (yyrule));
      YYDPRINTF ((stderr,
                  "Reduced stack %lu by rule #%d; action deferred.  "
                  "Now in state %d.\n",
                  (unsigned long int) yyk, yyrule - 1, yynewLRState));
      for (yyi = 0; yyi < yystackp->yytops.yysize; yyi += 1)
        if (yyi != yyk && yystackp->yytops.yystates[yyi] != YY_NULLPTR)
          {
            yyGLRState *yysplit = yystackp->yysplitPoint;
            yyGLRState *yyp = yystackp->yytops.yystates[yyi];
            while (yyp != yys && yyp != yysplit && yyp->yyposn >= yyposn)
              {
                if (yyp->yylrState == yynewLRState && yyp->yypred == yys)
                  {
                    yyaddDeferredAction (yystackp, yyk, yyp, yys0, yyrule);
                    yymarkStackDeleted (yystackp, yyk);
                    YYDPRINTF ((stderr, "Merging stack %lu into stack %lu.\n",
                                (unsigned long int) yyk,
                                (unsigned long int) yyi));
                    return yyok;
                  }
                yyp = yyp->yypred;
              }
          }
      yystackp->yytops.yystates[yyk] = yys;
      yyglrShiftDefer (yystackp, yyk, yynewLRState, yyposn, yys0, yyrule);
    }
  return yyok;
}

static size_t
yysplitStack (yyGLRStack* yystackp, size_t yyk)
{
  if (yystackp->yysplitPoint == YY_NULLPTR)
    {
      YYASSERT (yyk == 0);
      yystackp->yysplitPoint = yystackp->yytops.yystates[yyk];
    }
  if (yystackp->yytops.yysize >= yystackp->yytops.yycapacity)
    {
      yyGLRState** yynewStates;
      yybool* yynewLookaheadNeeds;

      yynewStates = YY_NULLPTR;

      if (yystackp->yytops.yycapacity
          > (YYSIZEMAX / (2 * sizeof yynewStates[0])))
        yyMemoryExhausted (yystackp);
      yystackp->yytops.yycapacity *= 2;

      yynewStates =
        (yyGLRState**) YYREALLOC (yystackp->yytops.yystates,
                                  (yystackp->yytops.yycapacity
                                   * sizeof yynewStates[0]));
      if (yynewStates == YY_NULLPTR)
        yyMemoryExhausted (yystackp);
      yystackp->yytops.yystates = yynewStates;

      yynewLookaheadNeeds =
        (yybool*) YYREALLOC (yystackp->yytops.yylookaheadNeeds,
                             (yystackp->yytops.yycapacity
                              * sizeof yynewLookaheadNeeds[0]));
      if (yynewLookaheadNeeds == YY_NULLPTR)
        yyMemoryExhausted (yystackp);
      yystackp->yytops.yylookaheadNeeds = yynewLookaheadNeeds;
    }
  yystackp->yytops.yystates[yystackp->yytops.yysize]
    = yystackp->yytops.yystates[yyk];
  yystackp->yytops.yylookaheadNeeds[yystackp->yytops.yysize]
    = yystackp->yytops.yylookaheadNeeds[yyk];
  yystackp->yytops.yysize += 1;
  return yystackp->yytops.yysize-1;
}

/** True iff YYY0 and YYY1 represent identical options at the top level.
 *  That is, they represent the same rule applied to RHS symbols
 *  that produce the same terminal symbols.  */
static yybool
yyidenticalOptions (yySemanticOption* yyy0, yySemanticOption* yyy1)
{
  if (yyy0->yyrule == yyy1->yyrule)
    {
      yyGLRState *yys0, *yys1;
      int yyn;
      for (yys0 = yyy0->yystate, yys1 = yyy1->yystate,
           yyn = yyrhsLength (yyy0->yyrule);
           yyn > 0;
           yys0 = yys0->yypred, yys1 = yys1->yypred, yyn -= 1)
        if (yys0->yyposn != yys1->yyposn)
          return yyfalse;
      return yytrue;
    }
  else
    return yyfalse;
}

/** Assuming identicalOptions (YYY0,YYY1), destructively merge the
 *  alternative semantic values for the RHS-symbols of YYY1 and YYY0.  */
static void
yymergeOptionSets (yySemanticOption* yyy0, yySemanticOption* yyy1)
{
  yyGLRState *yys0, *yys1;
  int yyn;
  for (yys0 = yyy0->yystate, yys1 = yyy1->yystate,
       yyn = yyrhsLength (yyy0->yyrule);
       yyn > 0;
       yys0 = yys0->yypred, yys1 = yys1->yypred, yyn -= 1)
    {
      if (yys0 == yys1)
        break;
      else if (yys0->yyresolved)
        {
          yys1->yyresolved = yytrue;
          yys1->yysemantics.yysval = yys0->yysemantics.yysval;
        }
      else if (yys1->yyresolved)
        {
          yys0->yyresolved = yytrue;
          yys0->yysemantics.yysval = yys1->yysemantics.yysval;
        }
      else
        {
          yySemanticOption** yyz0p = &yys0->yysemantics.yyfirstVal;
          yySemanticOption* yyz1 = yys1->yysemantics.yyfirstVal;
          while (yytrue)
            {
              if (yyz1 == *yyz0p || yyz1 == YY_NULLPTR)
                break;
              else if (*yyz0p == YY_NULLPTR)
                {
                  *yyz0p = yyz1;
                  break;
                }
              else if (*yyz0p < yyz1)
                {
                  yySemanticOption* yyz = *yyz0p;
                  *yyz0p = yyz1;
                  yyz1 = yyz1->yynext;
                  (*yyz0p)->yynext = yyz;
                }
              yyz0p = &(*yyz0p)->yynext;
            }
          yys1->yysemantics.yyfirstVal = yys0->yysemantics.yyfirstVal;
        }
    }
}

/** Y0 and Y1 represent two possible actions to take in a given
 *  parsing state; return 0 if no combination is possible,
 *  1 if user-mergeable, 2 if Y0 is preferred, 3 if Y1 is preferred.  */
static int
yypreference (yySemanticOption* y0, yySemanticOption* y1)
{
  yyRuleNum r0 = y0->yyrule, r1 = y1->yyrule;
  int p0 = yydprec[r0], p1 = yydprec[r1];

  if (p0 == p1)
    {
      if (yymerger[r0] == 0 || yymerger[r0] != yymerger[r1])
        return 0;
      else
        return 1;
    }
  if (p0 == 0 || p1 == 0)
    return 0;
  if (p0 < p1)
    return 3;
  if (p1 < p0)
    return 2;
  return 0;
}

static YYRESULTTAG yyresolveValue (yyGLRState* yys,
                                   yyGLRStack* yystackp, yy::parser& yyparser);


/** Resolve the previous YYN states starting at and including state YYS
 *  on *YYSTACKP. If result != yyok, some states may have been left
 *  unresolved possibly with empty semantic option chains.  Regardless
 *  of whether result = yyok, each state has been left with consistent
 *  data so that yydestroyGLRState can be invoked if necessary.  */
static YYRESULTTAG
yyresolveStates (yyGLRState* yys, int yyn,
                 yyGLRStack* yystackp, yy::parser& yyparser)
{
  if (0 < yyn)
    {
      YYASSERT (yys->yypred);
      YYCHK (yyresolveStates (yys->yypred, yyn-1, yystackp, yyparser));
      if (! yys->yyresolved)
        YYCHK (yyresolveValue (yys, yystackp, yyparser));
    }
  return yyok;
}

/** Resolve the states for the RHS of YYOPT on *YYSTACKP, perform its
 *  user action, and return the semantic value and location in *YYVALP
 *  and *YYLOCP.  Regardless of whether result = yyok, all RHS states
 *  have been destroyed (assuming the user action destroys all RHS
 *  semantic values if invoked).  */
static YYRESULTTAG
yyresolveAction (yySemanticOption* yyopt, yyGLRStack* yystackp,
                 YYSTYPE* yyvalp, YYLTYPE *yylocp, yy::parser& yyparser)
{
  yyGLRStackItem yyrhsVals[YYMAXRHS + YYMAXLEFT + 1];
  int yynrhs = yyrhsLength (yyopt->yyrule);
  YYRESULTTAG yyflag =
    yyresolveStates (yyopt->yystate, yynrhs, yystackp, yyparser);
  if (yyflag != yyok)
    {
      yyGLRState *yys;
      for (yys = yyopt->yystate; yynrhs > 0; yys = yys->yypred, yynrhs -= 1)
        yydestroyGLRState ("Cleanup: popping", yys, yyparser);
      return yyflag;
    }

  yyrhsVals[YYMAXRHS + YYMAXLEFT].yystate.yypred = yyopt->yystate;
  if (yynrhs == 0)
    /* Set default location.  */
    yyrhsVals[YYMAXRHS + YYMAXLEFT - 1].yystate.yyloc = yyopt->yystate->yyloc;
  {
    int yychar_current = yychar;
    YYSTYPE yylval_current = yylval;
    YYLTYPE yylloc_current = yylloc;
    yychar = yyopt->yyrawchar;
    yylval = yyopt->yyval;
    yylloc = yyopt->yyloc;
    yyflag = yyuserAction (yyopt->yyrule, yynrhs,
                           yyrhsVals + YYMAXRHS + YYMAXLEFT - 1,
                           yystackp, yyvalp, yylocp, yyparser);
    yychar = yychar_current;
    yylval = yylval_current;
    yylloc = yylloc_current;
  }
  return yyflag;
}

#if YYDEBUG
static void
yyreportTree (yySemanticOption* yyx, int yyindent)
{
  int yynrhs = yyrhsLength (yyx->yyrule);
  int yyi;
  yyGLRState* yys;
  yyGLRState* yystates[1 + YYMAXRHS];
  yyGLRState yyleftmost_state;

  for (yyi = yynrhs, yys = yyx->yystate; 0 < yyi; yyi -= 1, yys = yys->yypred)
    yystates[yyi] = yys;
  if (yys == YY_NULLPTR)
    {
      yyleftmost_state.yyposn = 0;
      yystates[0] = &yyleftmost_state;
    }
  else
    yystates[0] = yys;

  if (yyx->yystate->yyposn < yys->yyposn + 1)
    YYFPRINTF (stderr, "%*s%s -> <Rule %d, empty>\n",
               yyindent, "", yytokenName (yylhsNonterm (yyx->yyrule)),
               yyx->yyrule - 1);
  else
    YYFPRINTF (stderr, "%*s%s -> <Rule %d, tokens %lu .. %lu>\n",
               yyindent, "", yytokenName (yylhsNonterm (yyx->yyrule)),
               yyx->yyrule - 1, (unsigned long int) (yys->yyposn + 1),
               (unsigned long int) yyx->yystate->yyposn);
  for (yyi = 1; yyi <= yynrhs; yyi += 1)
    {
      if (yystates[yyi]->yyresolved)
        {
          if (yystates[yyi-1]->yyposn+1 > yystates[yyi]->yyposn)
            YYFPRINTF (stderr, "%*s%s <empty>\n", yyindent+2, "",
                       yytokenName (yystos[yystates[yyi]->yylrState]));
          else
            YYFPRINTF (stderr, "%*s%s <tokens %lu .. %lu>\n", yyindent+2, "",
                       yytokenName (yystos[yystates[yyi]->yylrState]),
                       (unsigned long int) (yystates[yyi-1]->yyposn + 1),
                       (unsigned long int) yystates[yyi]->yyposn);
        }
      else
        yyreportTree (yystates[yyi]->yysemantics.yyfirstVal, yyindent+2);
    }
}
#endif

static YYRESULTTAG
yyreportAmbiguity (yySemanticOption* yyx0,
                   yySemanticOption* yyx1, YYLTYPE *yylocp, yy::parser& yyparser)
{
  YYUSE (yyx0);
  YYUSE (yyx1);

#if YYDEBUG
  YYFPRINTF (stderr, "Ambiguity detected.\n");
  YYFPRINTF (stderr, "Option 1,\n");
  yyreportTree (yyx0, 2);
  YYFPRINTF (stderr, "\nOption 2,\n");
  yyreportTree (yyx1, 2);
  YYFPRINTF (stderr, "\n");
#endif

  yyerror (yylocp, yyparser, YY_("syntax is ambiguous"));
  return yyabort;
}

/** Resolve the locations for each of the YYN1 states in *YYSTACKP,
 *  ending at YYS1.  Has no effect on previously resolved states.
 *  The first semantic option of a state is always chosen.  */
static void
yyresolveLocations (yyGLRState* yys1, int yyn1,
                    yyGLRStack *yystackp, yy::parser& yyparser)
{
  if (0 < yyn1)
    {
      yyresolveLocations (yys1->yypred, yyn1 - 1, yystackp, yyparser);
      if (!yys1->yyresolved)
        {
          yyGLRStackItem yyrhsloc[1 + YYMAXRHS];
          int yynrhs;
          yySemanticOption *yyoption = yys1->yysemantics.yyfirstVal;
          YYASSERT (yyoption != YY_NULLPTR);
          yynrhs = yyrhsLength (yyoption->yyrule);
          if (yynrhs > 0)
            {
              yyGLRState *yys;
              int yyn;
              yyresolveLocations (yyoption->yystate, yynrhs,
                                  yystackp, yyparser);
              for (yys = yyoption->yystate, yyn = yynrhs;
                   yyn > 0;
                   yys = yys->yypred, yyn -= 1)
                yyrhsloc[yyn].yystate.yyloc = yys->yyloc;
            }
          else
            {
              /* Both yyresolveAction and yyresolveLocations traverse the GSS
                 in reverse rightmost order.  It is only necessary to invoke
                 yyresolveLocations on a subforest for which yyresolveAction
                 would have been invoked next had an ambiguity not been
                 detected.  Thus the location of the previous state (but not
                 necessarily the previous state itself) is guaranteed to be
                 resolved already.  */
              yyGLRState *yyprevious = yyoption->yystate;
              yyrhsloc[0].yystate.yyloc = yyprevious->yyloc;
            }
          {
            int yychar_current = yychar;
            YYSTYPE yylval_current = yylval;
            YYLTYPE yylloc_current = yylloc;
            yychar = yyoption->yyrawchar;
            yylval = yyoption->yyval;
            yylloc = yyoption->yyloc;
            YYLLOC_DEFAULT ((yys1->yyloc), yyrhsloc, yynrhs);
            yychar = yychar_current;
            yylval = yylval_current;
            yylloc = yylloc_current;
          }
        }
    }
}

/** Resolve the ambiguity represented in state YYS in *YYSTACKP,
 *  perform the indicated actions, and set the semantic value of YYS.
 *  If result != yyok, the chain of semantic options in YYS has been
 *  cleared instead or it has been left unmodified except that
 *  redundant options may have been removed.  Regardless of whether
 *  result = yyok, YYS has been left with consistent data so that
 *  yydestroyGLRState can be invoked if necessary.  */
static YYRESULTTAG
yyresolveValue (yyGLRState* yys, yyGLRStack* yystackp, yy::parser& yyparser)
{
  yySemanticOption* yyoptionList = yys->yysemantics.yyfirstVal;
  yySemanticOption* yybest = yyoptionList;
  yySemanticOption** yypp;
  yybool yymerge = yyfalse;
  YYSTYPE yysval;
  YYRESULTTAG yyflag;
  YYLTYPE *yylocp = &yys->yyloc;

  for (yypp = &yyoptionList->yynext; *yypp != YY_NULLPTR; )
    {
      yySemanticOption* yyp = *yypp;

      if (yyidenticalOptions (yybest, yyp))
        {
          yymergeOptionSets (yybest, yyp);
          *yypp = yyp->yynext;
        }
      else
        {
          switch (yypreference (yybest, yyp))
            {
            case 0:
              yyresolveLocations (yys, 1, yystackp, yyparser);
              return yyreportAmbiguity (yybest, yyp, yylocp, yyparser);
              break;
            case 1:
              yymerge = yytrue;
              break;
            case 2:
              break;
            case 3:
              yybest = yyp;
              yymerge = yyfalse;
              break;
            default:
              /* This cannot happen so it is not worth a YYASSERT (yyfalse),
                 but some compilers complain if the default case is
                 omitted.  */
              break;
            }
          yypp = &yyp->yynext;
        }
    }

  if (yymerge)
    {
      yySemanticOption* yyp;
      int yyprec = yydprec[yybest->yyrule];
      yyflag = yyresolveAction (yybest, yystackp, &yysval, yylocp, yyparser);
      if (yyflag == yyok)
        for (yyp = yybest->yynext; yyp != YY_NULLPTR; yyp = yyp->yynext)
          {
            if (yyprec == yydprec[yyp->yyrule])
              {
                YYSTYPE yysval_other;
                YYLTYPE yydummy;
                yyflag = yyresolveAction (yyp, yystackp, &yysval_other, &yydummy, yyparser);
                if (yyflag != yyok)
                  {
                    yydestruct ("Cleanup: discarding incompletely merged value for",
                                yystos[yys->yylrState],
                                &yysval, yylocp, yyparser);
                    break;
                  }
                yyuserMerge (yymerger[yyp->yyrule], &yysval, &yysval_other);
              }
          }
    }
  else
    yyflag = yyresolveAction (yybest, yystackp, &yysval, yylocp, yyparser);

  if (yyflag == yyok)
    {
      yys->yyresolved = yytrue;
      yys->yysemantics.yysval = yysval;
    }
  else
    yys->yysemantics.yyfirstVal = YY_NULLPTR;
  return yyflag;
}

static YYRESULTTAG
yyresolveStack (yyGLRStack* yystackp, yy::parser& yyparser)
{
  if (yystackp->yysplitPoint != YY_NULLPTR)
    {
      yyGLRState* yys;
      int yyn;

      for (yyn = 0, yys = yystackp->yytops.yystates[0];
           yys != yystackp->yysplitPoint;
           yys = yys->yypred, yyn += 1)
        continue;
      YYCHK (yyresolveStates (yystackp->yytops.yystates[0], yyn, yystackp
                             , yyparser));
    }
  return yyok;
}

static void
yycompressStack (yyGLRStack* yystackp)
{
  yyGLRState* yyp, *yyq, *yyr;

  if (yystackp->yytops.yysize != 1 || yystackp->yysplitPoint == YY_NULLPTR)
    return;

  for (yyp = yystackp->yytops.yystates[0], yyq = yyp->yypred, yyr = YY_NULLPTR;
       yyp != yystackp->yysplitPoint;
       yyr = yyp, yyp = yyq, yyq = yyp->yypred)
    yyp->yypred = yyr;

  yystackp->yyspaceLeft += yystackp->yynextFree - yystackp->yyitems;
  yystackp->yynextFree = ((yyGLRStackItem*) yystackp->yysplitPoint) + 1;
  yystackp->yyspaceLeft -= yystackp->yynextFree - yystackp->yyitems;
  yystackp->yysplitPoint = YY_NULLPTR;
  yystackp->yylastDeleted = YY_NULLPTR;

  while (yyr != YY_NULLPTR)
    {
      yystackp->yynextFree->yystate = *yyr;
      yyr = yyr->yypred;
      yystackp->yynextFree->yystate.yypred = &yystackp->yynextFree[-1].yystate;
      yystackp->yytops.yystates[0] = &yystackp->yynextFree->yystate;
      yystackp->yynextFree += 1;
      yystackp->yyspaceLeft -= 1;
    }
}

static YYRESULTTAG
yyprocessOneStack (yyGLRStack* yystackp, size_t yyk,
                   size_t yyposn, YYLTYPE *yylocp, yy::parser& yyparser)
{
  while (yystackp->yytops.yystates[yyk] != YY_NULLPTR)
    {
      yyStateNum yystate = yystackp->yytops.yystates[yyk]->yylrState;
      YYDPRINTF ((stderr, "Stack %lu Entering state %d\n",
                  (unsigned long int) yyk, yystate));

      YYASSERT (yystate != YYFINAL);

      if (yyisDefaultedState (yystate))
        {
          YYRESULTTAG yyflag;
          yyRuleNum yyrule = yydefaultAction (yystate);
          if (yyrule == 0)
            {
              YYDPRINTF ((stderr, "Stack %lu dies.\n",
                          (unsigned long int) yyk));
              yymarkStackDeleted (yystackp, yyk);
              return yyok;
            }
          yyflag = yyglrReduce (yystackp, yyk, yyrule, yyimmediate[yyrule], yyparser);
          if (yyflag == yyerr)
            {
              YYDPRINTF ((stderr,
                          "Stack %lu dies "
                          "(predicate failure or explicit user error).\n",
                          (unsigned long int) yyk));
              yymarkStackDeleted (yystackp, yyk);
              return yyok;
            }
          if (yyflag != yyok)
            return yyflag;
        }
      else
        {
          yySymbol yytoken;
          int yyaction;
          const short int* yyconflicts;

          yystackp->yytops.yylookaheadNeeds[yyk] = yytrue;
          if (yychar == YYEMPTY)
            {
              YYDPRINTF ((stderr, "Reading a token: "));
              yychar = yylex (&yylval, &yylloc);
            }

          if (yychar <= YYEOF)
            {
              yychar = yytoken = YYEOF;
              YYDPRINTF ((stderr, "Now at end of input.\n"));
            }
          else
            {
              yytoken = YYTRANSLATE (yychar);
              YY_SYMBOL_PRINT ("Next token is", yytoken, &yylval, &yylloc);
            }

          yygetLRActions (yystate, yytoken, &yyaction, &yyconflicts);

          while (*yyconflicts != 0)
            {
              YYRESULTTAG yyflag;
              size_t yynewStack = yysplitStack (yystackp, yyk);
              YYDPRINTF ((stderr, "Splitting off stack %lu from %lu.\n",
                          (unsigned long int) yynewStack,
                          (unsigned long int) yyk));
              yyflag = yyglrReduce (yystackp, yynewStack,
                                    *yyconflicts,
                                    yyimmediate[*yyconflicts], yyparser);
              if (yyflag == yyok)
                YYCHK (yyprocessOneStack (yystackp, yynewStack,
                                          yyposn, yylocp, yyparser));
              else if (yyflag == yyerr)
                {
                  YYDPRINTF ((stderr, "Stack %lu dies.\n",
                              (unsigned long int) yynewStack));
                  yymarkStackDeleted (yystackp, yynewStack);
                }
              else
                return yyflag;
              yyconflicts += 1;
            }

          if (yyisShiftAction (yyaction))
            break;
          else if (yyisErrorAction (yyaction))
            {
              YYDPRINTF ((stderr, "Stack %lu dies.\n",
                          (unsigned long int) yyk));
              yymarkStackDeleted (yystackp, yyk);
              break;
            }
          else
            {
              YYRESULTTAG yyflag = yyglrReduce (yystackp, yyk, -yyaction,
                                                yyimmediate[-yyaction], yyparser);
              if (yyflag == yyerr)
                {
                  YYDPRINTF ((stderr,
                              "Stack %lu dies "
                              "(predicate failure or explicit user error).\n",
                              (unsigned long int) yyk));
                  yymarkStackDeleted (yystackp, yyk);
                  break;
                }
              else if (yyflag != yyok)
                return yyflag;
            }
        }
    }
  return yyok;
}

static void
yyreportSyntaxError (yyGLRStack* yystackp, yy::parser& yyparser)
{
  if (yystackp->yyerrState != 0)
    return;
#if ! YYERROR_VERBOSE
  yyerror (&yylloc, yyparser, YY_("syntax error"));
#else
  {
  yySymbol yytoken = yychar == YYEMPTY ? YYEMPTY : YYTRANSLATE (yychar);
  size_t yysize0 = yytnamerr (YY_NULLPTR, yytokenName (yytoken));
  size_t yysize = yysize0;
  yybool yysize_overflow = yyfalse;
  char* yymsg = YY_NULLPTR;
  enum { YYERROR_VERBOSE_ARGS_MAXIMUM = 5 };
  /* Internationalized format string. */
  const char *yyformat = YY_NULLPTR;
  /* Arguments of yyformat. */
  char const *yyarg[YYERROR_VERBOSE_ARGS_MAXIMUM];
  /* Number of reported tokens (one for the "unexpected", one per
     "expected").  */
  int yycount = 0;

  /* There are many possibilities here to consider:
     - If this state is a consistent state with a default action, then
       the only way this function was invoked is if the default action
       is an error action.  In that case, don't check for expected
       tokens because there are none.
     - The only way there can be no lookahead present (in yychar) is if
       this state is a consistent state with a default action.  Thus,
       detecting the absence of a lookahead is sufficient to determine
       that there is no unexpected or expected token to report.  In that
       case, just report a simple "syntax error".
     - Don't assume there isn't a lookahead just because this state is a
       consistent state with a default action.  There might have been a
       previous inconsistent state, consistent state with a non-default
       action, or user semantic action that manipulated yychar.
     - Of course, the expected token list depends on states to have
       correct lookahead information, and it depends on the parser not
       to perform extra reductions after fetching a lookahead from the
       scanner and before detecting a syntax error.  Thus, state merging
       (from LALR or IELR) and default reductions corrupt the expected
       token list.  However, the list is correct for canonical LR with
       one exception: it will still contain any token that will not be
       accepted due to an error action in a later state.
  */
  if (yytoken != YYEMPTY)
    {
      int yyn = yypact[yystackp->yytops.yystates[0]->yylrState];
      yyarg[yycount++] = yytokenName (yytoken);
      if (!yypact_value_is_default (yyn))
        {
          /* Start YYX at -YYN if negative to avoid negative indexes in
             YYCHECK.  In other words, skip the first -YYN actions for this
             state because they are default actions.  */
          int yyxbegin = yyn < 0 ? -yyn : 0;
          /* Stay within bounds of both yycheck and yytname.  */
          int yychecklim = YYLAST - yyn + 1;
          int yyxend = yychecklim < YYNTOKENS ? yychecklim : YYNTOKENS;
          int yyx;
          for (yyx = yyxbegin; yyx < yyxend; ++yyx)
            if (yycheck[yyx + yyn] == yyx && yyx != YYTERROR
                && !yytable_value_is_error (yytable[yyx + yyn]))
              {
                if (yycount == YYERROR_VERBOSE_ARGS_MAXIMUM)
                  {
                    yycount = 1;
                    yysize = yysize0;
                    break;
                  }
                yyarg[yycount++] = yytokenName (yyx);
                {
                  size_t yysz = yysize + yytnamerr (YY_NULLPTR, yytokenName (yyx));
                  yysize_overflow |= yysz < yysize;
                  yysize = yysz;
                }
              }
        }
    }

  switch (yycount)
    {
#define YYCASE_(N, S)                   \
      case N:                           \
        yyformat = S;                   \
      break
      YYCASE_(0, YY_("syntax error"));
      YYCASE_(1, YY_("syntax error, unexpected %s"));
      YYCASE_(2, YY_("syntax error, unexpected %s, expecting %s"));
      YYCASE_(3, YY_("syntax error, unexpected %s, expecting %s or %s"));
      YYCASE_(4, YY_("syntax error, unexpected %s, expecting %s or %s or %s"));
      YYCASE_(5, YY_("syntax error, unexpected %s, expecting %s or %s or %s or %s"));
#undef YYCASE_
    }

  {
    size_t yysz = yysize + strlen (yyformat);
    yysize_overflow |= yysz < yysize;
    yysize = yysz;
  }

  if (!yysize_overflow)
    yymsg = (char *) YYMALLOC (yysize);

  if (yymsg)
    {
      char *yyp = yymsg;
      int yyi = 0;
      while ((*yyp = *yyformat))
        {
          if (*yyp == '%' && yyformat[1] == 's' && yyi < yycount)
            {
              yyp += yytnamerr (yyp, yyarg[yyi++]);
              yyformat += 2;
            }
          else
            {
              yyp++;
              yyformat++;
            }
        }
      yyerror (&yylloc, yyparser, yymsg);
      YYFREE (yymsg);
    }
  else
    {
      yyerror (&yylloc, yyparser, YY_("syntax error"));
      yyMemoryExhausted (yystackp);
    }
  }
#endif /* YYERROR_VERBOSE */
  yynerrs += 1;
}

/* Recover from a syntax error on *YYSTACKP, assuming that *YYSTACKP->YYTOKENP,
   yylval, and yylloc are the syntactic category, semantic value, and location
   of the lookahead.  */
static void
yyrecoverSyntaxError (yyGLRStack* yystackp, yy::parser& yyparser)
{
  size_t yyk;
  int yyj;

  if (yystackp->yyerrState == 3)
    /* We just shifted the error token and (perhaps) took some
       reductions.  Skip tokens until we can proceed.  */
    while (yytrue)
      {
        yySymbol yytoken;
        if (yychar == YYEOF)
          yyFail (yystackp, &yylloc, yyparser, YY_NULLPTR);
        if (yychar != YYEMPTY)
          {
            /* We throw away the lookahead, but the error range
               of the shifted error token must take it into account.  */
            yyGLRState *yys = yystackp->yytops.yystates[0];
            yyGLRStackItem yyerror_range[3];
            yyerror_range[1].yystate.yyloc = yys->yyloc;
            yyerror_range[2].yystate.yyloc = yylloc;
            YYLLOC_DEFAULT ((yys->yyloc), yyerror_range, 2);
            yytoken = YYTRANSLATE (yychar);
            yydestruct ("Error: discarding",
                        yytoken, &yylval, &yylloc, yyparser);
          }
        YYDPRINTF ((stderr, "Reading a token: "));
        yychar = yylex (&yylval, &yylloc);
        if (yychar <= YYEOF)
          {
            yychar = yytoken = YYEOF;
            YYDPRINTF ((stderr, "Now at end of input.\n"));
          }
        else
          {
            yytoken = YYTRANSLATE (yychar);
            YY_SYMBOL_PRINT ("Next token is", yytoken, &yylval, &yylloc);
          }
        yyj = yypact[yystackp->yytops.yystates[0]->yylrState];
        if (yypact_value_is_default (yyj))
          return;
        yyj += yytoken;
        if (yyj < 0 || YYLAST < yyj || yycheck[yyj] != yytoken)
          {
            if (yydefact[yystackp->yytops.yystates[0]->yylrState] != 0)
              return;
          }
        else if (! yytable_value_is_error (yytable[yyj]))
          return;
      }

  /* Reduce to one stack.  */
  for (yyk = 0; yyk < yystackp->yytops.yysize; yyk += 1)
    if (yystackp->yytops.yystates[yyk] != YY_NULLPTR)
      break;
  if (yyk >= yystackp->yytops.yysize)
    yyFail (yystackp, &yylloc, yyparser, YY_NULLPTR);
  for (yyk += 1; yyk < yystackp->yytops.yysize; yyk += 1)
    yymarkStackDeleted (yystackp, yyk);
  yyremoveDeletes (yystackp);
  yycompressStack (yystackp);

  /* Now pop stack until we find a state that shifts the error token.  */
  yystackp->yyerrState = 3;
  while (yystackp->yytops.yystates[0] != YY_NULLPTR)
    {
      yyGLRState *yys = yystackp->yytops.yystates[0];
      yyj = yypact[yys->yylrState];
      if (! yypact_value_is_default (yyj))
        {
          yyj += YYTERROR;
          if (0 <= yyj && yyj <= YYLAST && yycheck[yyj] == YYTERROR
              && yyisShiftAction (yytable[yyj]))
            {
              /* Shift the error token.  */
              /* First adjust its location.*/
              YYLTYPE yyerrloc;
              yystackp->yyerror_range[2].yystate.yyloc = yylloc;
              YYLLOC_DEFAULT (yyerrloc, (yystackp->yyerror_range), 2);
              YY_SYMBOL_PRINT ("Shifting", yystos[yytable[yyj]],
                               &yylval, &yyerrloc);
              yyglrShift (yystackp, 0, yytable[yyj],
                          yys->yyposn, &yylval, &yyerrloc);
              yys = yystackp->yytops.yystates[0];
              break;
            }
        }
      yystackp->yyerror_range[1].yystate.yyloc = yys->yyloc;
      if (yys->yypred != YY_NULLPTR)
        yydestroyGLRState ("Error: popping", yys, yyparser);
      yystackp->yytops.yystates[0] = yys->yypred;
      yystackp->yynextFree -= 1;
      yystackp->yyspaceLeft += 1;
    }
  if (yystackp->yytops.yystates[0] == YY_NULLPTR)
    yyFail (yystackp, &yylloc, yyparser, YY_NULLPTR);
}

#define YYCHK1(YYE)                                                          \
  do {                                                                       \
    switch (YYE) {                                                           \
    case yyok:                                                               \
      break;                                                                 \
    case yyabort:                                                            \
      goto yyabortlab;                                                       \
    case yyaccept:                                                           \
      goto yyacceptlab;                                                      \
    case yyerr:                                                              \
      goto yyuser_error;                                                     \
    default:                                                                 \
      goto yybuglab;                                                         \
    }                                                                        \
  } while (0)

/*----------.
| yyparse.  |
`----------*/

int
yyparse (yy::parser& yyparser)
{
  int yyresult;
  yyGLRStack yystack;
  yyGLRStack* const yystackp = &yystack;
  size_t yyposn;

  YYDPRINTF ((stderr, "Starting parse\n"));

  yychar = YYEMPTY;
  yylval = yyval_default;
  yylloc = yyloc_default;

  /* User initialization code.  */
  yylloc.initialize ();
#line 4433 "src/parser.cpp" // glr.c:2270

  if (! yyinitGLRStack (yystackp, YYINITDEPTH))
    goto yyexhaustedlab;
  switch (YYSETJMP (yystack.yyexception_buffer))
    {
    case 0: break;
    case 1: goto yyabortlab;
    case 2: goto yyexhaustedlab;
    default: goto yybuglab;
    }
  yyglrShift (&yystack, 0, 0, 0, &yylval, &yylloc);
  yyposn = 0;

  while (yytrue)
    {
      /* For efficiency, we have two loops, the first of which is
         specialized to deterministic operation (single stack, no
         potential ambiguity).  */
      /* Standard mode */
      while (yytrue)
        {
          yyRuleNum yyrule;
          int yyaction;
          const short int* yyconflicts;

          yyStateNum yystate = yystack.yytops.yystates[0]->yylrState;
          YYDPRINTF ((stderr, "Entering state %d\n", yystate));
          if (yystate == YYFINAL)
            goto yyacceptlab;
          if (yyisDefaultedState (yystate))
            {
              yyrule = yydefaultAction (yystate);
              if (yyrule == 0)
                {
               yystack.yyerror_range[1].yystate.yyloc = yylloc;
                  yyreportSyntaxError (&yystack, yyparser);
                  goto yyuser_error;
                }
              YYCHK1 (yyglrReduce (&yystack, 0, yyrule, yytrue, yyparser));
            }
          else
            {
              yySymbol yytoken;
              if (yychar == YYEMPTY)
                {
                  YYDPRINTF ((stderr, "Reading a token: "));
                  yychar = yylex (&yylval, &yylloc);
                }

              if (yychar <= YYEOF)
                {
                  yychar = yytoken = YYEOF;
                  YYDPRINTF ((stderr, "Now at end of input.\n"));
                }
              else
                {
                  yytoken = YYTRANSLATE (yychar);
                  YY_SYMBOL_PRINT ("Next token is", yytoken, &yylval, &yylloc);
                }

              yygetLRActions (yystate, yytoken, &yyaction, &yyconflicts);
              if (*yyconflicts != 0)
                break;
              if (yyisShiftAction (yyaction))
                {
                  YY_SYMBOL_PRINT ("Shifting", yytoken, &yylval, &yylloc);
                  yychar = YYEMPTY;
                  yyposn += 1;
                  yyglrShift (&yystack, 0, yyaction, yyposn, &yylval, &yylloc);
                  if (0 < yystack.yyerrState)
                    yystack.yyerrState -= 1;
                }
              else if (yyisErrorAction (yyaction))
                {
               yystack.yyerror_range[1].yystate.yyloc = yylloc;
                  yyreportSyntaxError (&yystack, yyparser);
                  goto yyuser_error;
                }
              else
                YYCHK1 (yyglrReduce (&yystack, 0, -yyaction, yytrue, yyparser));
            }
        }

      while (yytrue)
        {
          yySymbol yytoken_to_shift;
          size_t yys;

          for (yys = 0; yys < yystack.yytops.yysize; yys += 1)
            yystackp->yytops.yylookaheadNeeds[yys] = yychar != YYEMPTY;

          /* yyprocessOneStack returns one of three things:

              - An error flag.  If the caller is yyprocessOneStack, it
                immediately returns as well.  When the caller is finally
                yyparse, it jumps to an error label via YYCHK1.

              - yyok, but yyprocessOneStack has invoked yymarkStackDeleted
                (&yystack, yys), which sets the top state of yys to NULL.  Thus,
                yyparse's following invocation of yyremoveDeletes will remove
                the stack.

              - yyok, when ready to shift a token.

             Except in the first case, yyparse will invoke yyremoveDeletes and
             then shift the next token onto all remaining stacks.  This
             synchronization of the shift (that is, after all preceding
             reductions on all stacks) helps prevent double destructor calls
             on yylval in the event of memory exhaustion.  */

          for (yys = 0; yys < yystack.yytops.yysize; yys += 1)
            YYCHK1 (yyprocessOneStack (&yystack, yys, yyposn, &yylloc, yyparser));
          yyremoveDeletes (&yystack);
          if (yystack.yytops.yysize == 0)
            {
              yyundeleteLastStack (&yystack);
              if (yystack.yytops.yysize == 0)
                yyFail (&yystack, &yylloc, yyparser, YY_("syntax error"));
              YYCHK1 (yyresolveStack (&yystack, yyparser));
              YYDPRINTF ((stderr, "Returning to deterministic operation.\n"));
           yystack.yyerror_range[1].yystate.yyloc = yylloc;
              yyreportSyntaxError (&yystack, yyparser);
              goto yyuser_error;
            }

          /* If any yyglrShift call fails, it will fail after shifting.  Thus,
             a copy of yylval will already be on stack 0 in the event of a
             failure in the following loop.  Thus, yychar is set to YYEMPTY
             before the loop to make sure the user destructor for yylval isn't
             called twice.  */
          yytoken_to_shift = YYTRANSLATE (yychar);
          yychar = YYEMPTY;
          yyposn += 1;
          for (yys = 0; yys < yystack.yytops.yysize; yys += 1)
            {
              int yyaction;
              const short int* yyconflicts;
              yyStateNum yystate = yystack.yytops.yystates[yys]->yylrState;
              yygetLRActions (yystate, yytoken_to_shift, &yyaction,
                              &yyconflicts);
              /* Note that yyconflicts were handled by yyprocessOneStack.  */
              YYDPRINTF ((stderr, "On stack %lu, ", (unsigned long int) yys));
              YY_SYMBOL_PRINT ("shifting", yytoken_to_shift, &yylval, &yylloc);
              yyglrShift (&yystack, yys, yyaction, yyposn,
                          &yylval, &yylloc);
              YYDPRINTF ((stderr, "Stack %lu now in state #%d\n",
                          (unsigned long int) yys,
                          yystack.yytops.yystates[yys]->yylrState));
            }

          if (yystack.yytops.yysize == 1)
            {
              YYCHK1 (yyresolveStack (&yystack, yyparser));
              YYDPRINTF ((stderr, "Returning to deterministic operation.\n"));
              yycompressStack (&yystack);
              break;
            }
        }
      continue;
    yyuser_error:
      yyrecoverSyntaxError (&yystack, yyparser);
      yyposn = yystack.yytops.yystates[0]->yyposn;
    }

 yyacceptlab:
  yyresult = 0;
  goto yyreturn;

 yybuglab:
  YYASSERT (yyfalse);
  goto yyabortlab;

 yyabortlab:
  yyresult = 1;
  goto yyreturn;

 yyexhaustedlab:
  yyerror (&yylloc, yyparser, YY_("memory exhausted"));
  yyresult = 2;
  goto yyreturn;

 yyreturn:
  if (yychar != YYEMPTY)
    yydestruct ("Cleanup: discarding lookahead",
                YYTRANSLATE (yychar), &yylval, &yylloc, yyparser);

  /* If the stack is well-formed, pop the stack until it is empty,
     destroying its entries as we go.  But free the stack regardless
     of whether it is well-formed.  */
  if (yystack.yyitems)
    {
      yyGLRState** yystates = yystack.yytops.yystates;
      if (yystates)
        {
          size_t yysize = yystack.yytops.yysize;
          size_t yyk;
          for (yyk = 0; yyk < yysize; yyk += 1)
            if (yystates[yyk])
              {
                while (yystates[yyk])
                  {
                    yyGLRState *yys = yystates[yyk];
                 yystack.yyerror_range[1].yystate.yyloc = yys->yyloc;
                  if (yys->yypred != YY_NULLPTR)
                      yydestroyGLRState ("Cleanup: popping", yys, yyparser);
                    yystates[yyk] = yys->yypred;
                    yystack.yynextFree -= 1;
                    yystack.yyspaceLeft += 1;
                  }
                break;
              }
        }
      yyfreeGLRStack (&yystack);
    }

  return yyresult;
}

/* DEBUGGING ONLY */
#if YYDEBUG
static void
yy_yypstack (yyGLRState* yys)
{
  if (yys->yypred)
    {
      yy_yypstack (yys->yypred);
      YYFPRINTF (stderr, " -> ");
    }
  YYFPRINTF (stderr, "%d@%lu", yys->yylrState,
             (unsigned long int) yys->yyposn);
}

static void
yypstates (yyGLRState* yyst)
{
  if (yyst == YY_NULLPTR)
    YYFPRINTF (stderr, "<null>");
  else
    yy_yypstack (yyst);
  YYFPRINTF (stderr, "\n");
}

static void
yypstack (yyGLRStack* yystackp, size_t yyk)
{
  yypstates (yystackp->yytops.yystates[yyk]);
}

#define YYINDEX(YYX)                                                         \
    ((YYX) == YY_NULLPTR ? -1 : (yyGLRStackItem*) (YYX) - yystackp->yyitems)


static void
yypdumpstack (yyGLRStack* yystackp)
{
  yyGLRStackItem* yyp;
  size_t yyi;
  for (yyp = yystackp->yyitems; yyp < yystackp->yynextFree; yyp += 1)
    {
      YYFPRINTF (stderr, "%3lu. ",
                 (unsigned long int) (yyp - yystackp->yyitems));
      if (*(yybool *) yyp)
        {
          YYASSERT (yyp->yystate.yyisState);
          YYASSERT (yyp->yyoption.yyisState);
          YYFPRINTF (stderr, "Res: %d, LR State: %d, posn: %lu, pred: %ld",
                     yyp->yystate.yyresolved, yyp->yystate.yylrState,
                     (unsigned long int) yyp->yystate.yyposn,
                     (long int) YYINDEX (yyp->yystate.yypred));
          if (! yyp->yystate.yyresolved)
            YYFPRINTF (stderr, ", firstVal: %ld",
                       (long int) YYINDEX (yyp->yystate
                                             .yysemantics.yyfirstVal));
        }
      else
        {
          YYASSERT (!yyp->yystate.yyisState);
          YYASSERT (!yyp->yyoption.yyisState);
          YYFPRINTF (stderr, "Option. rule: %d, state: %ld, next: %ld",
                     yyp->yyoption.yyrule - 1,
                     (long int) YYINDEX (yyp->yyoption.yystate),
                     (long int) YYINDEX (yyp->yyoption.yynext));
        }
      YYFPRINTF (stderr, "\n");
    }
  YYFPRINTF (stderr, "Tops:");
  for (yyi = 0; yyi < yystackp->yytops.yysize; yyi += 1)
    YYFPRINTF (stderr, "%lu: %ld; ", (unsigned long int) yyi,
               (long int) YYINDEX (yystackp->yytops.yystates[yyi]));
  YYFPRINTF (stderr, "\n");
}
#endif

#undef yylval
#undef yychar
#undef yynerrs
#undef yylloc



#line 490 "src/syntax.y" // glr.c:2584


/* location parser error */
void yy::parser::error(const location& loc, const string& msg){
    ante::error(msg.c_str(), yylexer->fileName, loc.begin.line, loc.begin.column);
} 

/*
void yy::parser::error(const string& msg){
    ante::error(msg.c_str(), yylexer->fileName, yylexer->getRow(), yylexer->getCol());
}*/

#endif
#line 4748 "src/parser.cpp" // glr.c:2584

/*------------------.
| Report an error.  |
`------------------*/

static void
yyerror (const yy::parser::location_type *yylocationp, yy::parser& yyparser, const char* msg)
{
  YYUSE (yyparser);
  yyparser.error (*yylocationp, msg);
}



namespace yy {
#line 4764 "src/parser.cpp" // glr.c:2584
  /// Build a parser object.
  parser::parser ()
#if YYDEBUG
     :yycdebug_ (&std::cerr)
#endif
  {
  }

  parser::~parser ()
  {
  }

  int
  parser::parse ()
  {
    return ::yyparse (*this);
  }

#if YYDEBUG
  /*--------------------.
  | Print this symbol.  |
  `--------------------*/

  inline void
  parser::yy_symbol_value_print_ (int yytype,
                           const semantic_type* yyvaluep,
                           const location_type* yylocationp)
  {
    YYUSE (yylocationp);
    YYUSE (yyvaluep);
    std::ostream& yyoutput = debug_stream ();
    std::ostream& yyo = yyoutput;
    YYUSE (yyo);
    YYUSE (yytype);
  }


  void
  parser::yy_symbol_print_ (int yytype,
                           const semantic_type* yyvaluep,
                           const location_type* yylocationp)
  {
    *yycdebug_ << (yytype < YYNTOKENS ? "token" : "nterm")
               << ' ' << yytname[yytype] << " ("
               << *yylocationp << ": ";
    yy_symbol_value_print_ (yytype, yyvaluep, yylocationp);
    *yycdebug_ << ')';
  }

  std::ostream&
  parser::debug_stream () const
  {
    return *yycdebug_;
  }

  void
  parser::set_debug_stream (std::ostream& o)
  {
    yycdebug_ = &o;
  }


  parser::debug_level_type
  parser::debug_level () const
  {
    return yydebug;
  }

  void
  parser::set_debug_level (debug_level_type l)
  {
    // Actually, it is yydebug which is really used.
    yydebug = l;
  }

#endif

} // yy
#line 4843 "src/parser.cpp" // glr.c:2584
