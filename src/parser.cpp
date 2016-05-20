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

bool is_expr_block = false;


#line 86 "src/parser.cpp" // glr.c:240

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

/* Copy the second part of user declarations.  */
#line 113 "src/parser.cpp" // glr.c:263
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
static void yyerror (yy::parser& yyparser, const char* msg);
#line 135 "src/parser.cpp" // glr.c:263

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
#define YYLAST   2311

/* YYNTOKENS -- Number of terminals.  */
#define YYNTOKENS  94
/* YYNNTS -- Number of nonterminals.  */
#define YYNNTS  60
/* YYNRULES -- Number of rules.  */
#define YYNRULES  227
/* YYNRULES -- Number of states.  */
#define YYNSTATES  462
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
       0,   118,   118,   121,   122,   125,   126,   133,   134,   135,
     136,   137,   138,   139,   140,   141,   142,   143,   144,   145,
     146,   149,   150,   151,   152,   153,   154,   155,   156,   157,
     158,   159,   160,   161,   162,   165,   167,   170,   173,   176,
     179,   182,   183,   184,   185,   186,   187,   188,   189,   190,
     191,   192,   193,   194,   195,   196,   197,   198,   199,   200,
     205,   206,   207,   208,   209,   210,   213,   214,   215,   218,
     227,   228,   229,   230,   231,   232,   233,   236,   237,   240,
     244,   245,   246,   248,   249,   252,   253,   254,   255,   259,
     260,   261,   262,   263,   266,   267,   270,   273,   274,   275,
     276,   279,   280,   281,   284,   285,   288,   292,   293,   294,
     295,   298,   301,   302,   303,   304,   307,   308,   311,   312,
     315,   322,   323,   327,   328,   332,   333,   336,   337,   338,
     339,   340,   341,   342,   343,   347,   350,   354,   358,   360,
     361,   370,   371,   374,   375,   376,   377,   380,   383,   386,
     389,   392,   395,   396,   397,   398,   401,   402,   403,   404,
     405,   406,   407,   408,   409,   410,   411,   414,   415,   418,
     419,   423,   424,   425,   426,   429,   431,   432,   433,   434,
     435,   436,   437,   438,   439,   440,   441,   442,   443,   444,
     445,   446,   447,   448,   449,   450,   451,   452,   453,   454,
     455,   460,   463,   464,   468,   469,   470,   471,   472,   473,
     474,   475,   476,   477,   478,   479,   480,   481,   482,   483,
     484,   485,   486,   487,   488,   489,   490,   491
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

#define YYPACT_NINF -255
#define YYTABLE_NINF -156

  // YYPACT[STATE-NUM] -- Index in YYTABLE of the portion describing
  // STATE-NUM.
static const short int yypact[] =
{
     -48,  -255,    24,   921,  -255,  -255,  -255,  -255,  -255,  -255,
    -255,  -255,  -255,  -255,  -255,  -255,  -255,  -255,  -255,  -255,
    -255,  -255,  -255,  -255,  -255,  -255,  -255,  -255,  -255,  1194,
    1194,    38,  1194,   -17,  1194,   744,    38,    40,    12,    38,
    1241,  -255,  -255,  -255,  -255,  -255,  -255,  -255,  1285,  1539,
    1012,  1103,    38,  1630,  1630,   830,  -255,    10,    60,  -255,
    -255,  -255,  -255,  -255,   -16,   -55,  1412,  -255,   259,  1738,
      20,    27,    30,    35,    46,    62,  -255,    83,    86,  -255,
     112,   113,   116,  -255,    33,  -255,  -255,  -255,  -255,   117,
    1856,    38,  1539,  1539,    52,  1412,  -255,    22,    79,    22,
    1194,  -255,  1241,   106,    38,  1594,   107,   -57,    40,   104,
    -255,     2,   122,    38,  1285,  1503,  -255,   125,   126,  2093,
    1539,  -255,  -255,  1376,   127,  1920,  -255,   124,  -255,  -255,
    -255,  -255,  -255,  -255,  -255,  -255,  1012,  1285,  -255,  -255,
    1059,   133,  1241,  1241,    38,   -11,  -255,  -255,    38,    40,
      12,    38,    38,  -255,  -255,  -255,  -255,  -255,  -255,  -255,
    -255,  -255,  -255,  -255,  1194,  1194,  1194,  1194,  1194,  -255,
    1194,  1194,  1194,  1194,  1194,  1194,  1194,  1285,    38,  1194,
    1194,  1194,  1194,  1194,  1194,  1194,  1194,  1194,    38,  1285,
    -255,   114,   921,   159,  1194,  -255,    22,   146,  1194,   140,
     142,    38,  1194,  1678,    40,   164,  -255,   144,   100,  -255,
    1241,  1241,  -255,  -255,   351,   154,   166,   -48,  -255,   -48,
     -48,   -48,   -48,   -48,   -48,   -48,  1285,    38,   -48,   -48,
     -48,   -48,   -48,   -48,   -48,   -48,   -48,   -48,  1285,  -255,
    -255,  -255,  -255,  -255,   545,  -255,   167,  -255,    68,    71,
    -255,  -255,  1194,   157,   -57,   104,  -255,     3,   163,  -255,
    -255,  -255,  -255,  -255,   161,   161,   161,   161,   456,  1324,
     179,  1943,   175,  2131,   161,   161,   135,   135,    57,    57,
      57,    57,  -255,  1754,  1194,   921,   181,    10,    20,    27,
      30,    35,    46,    62,    83,    86,   184,   112,   113,   116,
     117,    23,    22,  -255,  -255,  1837,  1194,  1194,   177,  -255,
      38,   195,  -255,   101,  -255,  -255,   102,  -255,  1194,    40,
    -255,    22,    38,   206,     7,   223,   -48,   210,   351,  1285,
     220,    38,  1285,  1285,  1285,  1285,  1285,  1285,  1285,  2007,
     203,  1285,  1285,  1285,  1285,  1285,  1285,  1285,  1285,  1285,
      38,  1773,  -255,  -255,  -255,  1194,   164,  -255,  -255,  1241,
    1241,  -255,  1194,  1194,  -255,  -255,  1837,   224,  -255,  1194,
      22,   227,  -255,  -255,  1194,  -255,  -255,  1194,  -255,  1678,
    -255,    40,  -255,  -255,   208,  -255,  -255,    38,  -255,  1150,
    1241,  -255,  -255,  -255,   -48,  2029,  -255,  -255,  2093,   192,
     192,   192,   192,  1451,  2226,  1285,   -48,  2214,   192,   192,
     205,   205,   121,   121,   121,   121,  -255,   -48,  -255,  -255,
      22,    28,  -255,  2112,  -255,    22,   228,    41,  2131,  -255,
    -255,  -255,  1194,  -255,   217,    38,    22,  -255,   -48,  2195,
    1285,  -255,  -255,  1241,  -255,  -255,  -255,  1194,    22,  -255,
    -255,  -255,  -255,  1285,  2214,    22,    22,   232,  2214,  -255,
    -255,  -255
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
       0,     0,     0,   161,     0,   199,   158,   159,   160,     0,
     175,     0,     0,     0,   151,     0,   136,     0,     0,     0,
       0,    35,     0,     0,     0,     0,     0,     0,     0,     0,
     115,   126,     0,     0,     0,     0,   226,     0,   201,   203,
       0,   173,   168,     0,     0,   203,   170,     0,    59,   152,
     172,   153,   171,     2,     3,    20,     0,     0,   135,    60,
       0,     0,     0,     0,     0,   151,   174,    77,     0,     0,
       0,     0,     0,    14,    17,    15,     8,     9,     7,    16,
      18,    10,    11,    12,     0,     0,     0,     0,     0,    19,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
     197,     0,     0,     0,     0,   148,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,    99,   110,     0,   113,
       0,     0,   125,   134,     0,     0,     0,     6,   200,     6,
       6,     6,     6,     6,     6,     6,     0,     0,     6,     6,
       6,     6,     6,     6,     6,     6,     6,     6,     0,   224,
      64,   167,   157,   169,     0,    63,     0,    61,    66,    67,
     151,   185,     0,     0,     0,     0,   114,   126,    81,    90,
      91,    92,    93,    89,   190,   191,   192,   193,   194,   195,
     196,     0,     0,   186,   181,   182,   176,   177,   178,   179,
     180,   183,   184,     0,     0,     0,     0,    34,    28,    31,
      29,    22,    23,    21,    30,    32,    13,    24,    25,    26,
      33,   146,     0,   149,    88,   175,     0,     0,     0,    84,
     102,     0,   105,     0,   103,    95,     0,   100,     0,     0,
     111,   126,     0,   124,   126,     0,     6,     0,   138,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,   154,    62,    82,     0,     0,    97,   112,     0,
       0,   130,     0,     0,   188,   187,     0,     0,   117,     0,
       0,     0,   147,   150,     0,    87,    86,     0,   101,     0,
     106,     0,    96,   109,   107,   133,   119,   120,   122,     0,
       0,   132,   140,   137,     6,     0,   227,   213,   202,   218,
     219,   220,   221,   222,   223,     0,     6,   214,   209,   210,
     204,   205,   206,   207,   208,   211,   212,     6,    83,    98,
     126,   126,    80,   198,   116,     0,     0,   144,   189,    85,
     104,    94,     0,   118,     0,     0,   126,   139,     6,   225,
       0,   215,   129,     0,   128,   142,   145,     0,     0,   108,
     123,   121,   131,     0,   216,   126,     0,     0,   217,   127,
     141,   143
};

  // YYPGOTO[NTERM-NUM].
static const short int yypgoto[] =
{
    -255,  -255,   120,   328,   -54,    19,  -184,    74,   -28,  -255,
    -255,  -255,  -255,   -68,  -255,    -3,   241,  -255,   -31,  -181,
    -179,  -178,  -255,    59,  -177,   -52,  -255,  -199,  -255,  -107,
    -170,   -78,  -255,  -111,  -255,   -35,  -254,  -185,  -255,  -169,
    -161,  -255,  -255,  -255,  -255,  -154,  -153,  -150,  -138,  -139,
     105,   257,   372,  -255,  -255,    -4,   508,     5,  -255,   178
};

  // YYDEFGOTO[NTERM-NUM].
static const short int yydefgoto[] =
{
      -1,     2,    55,     3,    56,   286,    57,    94,    59,    60,
      61,    62,    63,    64,    65,    95,    67,    68,    69,    70,
      71,    72,   316,   205,    73,   312,   313,   206,   208,   110,
      74,   212,   387,   388,   323,   324,   213,    75,    76,    77,
      78,   327,   328,   371,   372,    79,    80,    81,    82,    83,
      84,    85,    86,    87,    88,    89,    90,   124,   118,   119
};

  // YYTABLE[YYPACT[STATE-NUM]] -- What to do in state STATE-NUM.  If
  // positive, shift that token.  If negative, reduce the rule whose
  // number is the opposite.  If YYTABLE_NINF, syntax error.
static const short int yytable[] =
{
      66,   134,   209,   361,   105,   251,   317,   293,   287,   107,
     109,   288,   203,   289,   290,   291,     6,   204,   142,   193,
       1,   195,   292,   294,     4,    96,    97,   100,    99,   326,
     101,   295,   104,   314,   143,   210,   359,   112,   296,   297,
     390,     5,   298,   256,     6,   115,   120,   123,   115,   282,
     120,   120,    66,   117,   299,   357,   127,   164,   165,   166,
     167,   443,   139,   120,   369,   370,   152,   385,   140,   141,
     391,   192,   192,   136,   248,   249,   192,    58,   135,   252,
     207,   108,   447,   448,  -155,  -155,  -155,  -155,   153,   120,
     120,   192,   120,   211,   360,   154,   196,   192,   155,   197,
     293,   287,   201,   156,   288,    98,   289,   290,   291,   103,
     106,   115,   120,   111,   157,   292,   294,   120,   303,   216,
     120,   254,   255,   168,   295,   194,   128,    58,    58,    58,
     158,   296,   297,   115,   115,   298,   136,   246,   187,   188,
     145,   136,   189,   394,   136,   137,   139,   299,   358,   139,
    -155,   159,   140,   141,   160,   140,   141,   419,   129,   131,
     259,   260,   261,   262,   263,   191,   442,   444,   319,   379,
     320,   380,   311,   108,   115,   381,   315,   382,   199,   200,
     161,   162,   452,   325,   163,   169,   115,   215,   300,    66,
     302,   214,   397,   176,   304,   218,   198,   202,   309,   219,
     310,   459,   236,   237,   284,   136,   238,   321,   322,   314,
     241,   416,   243,   184,   185,   186,   187,   188,   250,   136,
     189,   247,   253,   115,   373,   257,   258,   301,   125,   240,
     306,   134,   307,   203,   318,   115,   330,   182,   183,   184,
     185,   186,   187,   188,   329,   136,   189,   355,   354,   150,
     353,   368,   272,   362,   -27,   182,   183,   184,   185,   186,
     187,   188,   250,   136,   189,   364,    58,   377,   231,   232,
     233,   234,   235,   236,   237,   308,   136,   238,   151,   389,
     393,   300,    66,   233,   234,   235,   236,   237,   396,   136,
     238,   384,   426,   406,   424,   427,   446,   325,   432,   450,
     461,   340,   375,   376,   367,   116,   121,   116,   116,   147,
     130,   132,   285,   356,   383,   244,    41,    42,    43,    44,
      45,    46,    47,   146,   451,   421,   115,   430,     0,   115,
     115,   115,   115,   115,   115,   115,     0,     0,   115,   115,
     115,   115,   115,   115,   115,   115,   115,   445,   311,   130,
     132,   418,   146,   431,     0,   271,   420,   322,   422,    58,
       0,     0,     0,     0,     0,   425,     0,   283,     0,     0,
     457,   116,   146,   429,     0,     0,   310,   146,   460,     0,
     146,     0,     0,   133,   378,     0,   435,   436,     0,     0,
       0,     0,     0,   116,   116,     0,   386,     0,     0,     0,
       0,     0,   115,     0,   339,   250,    39,     0,    41,    42,
      43,    44,    45,    46,    47,     0,   351,     0,     0,     0,
       0,     0,     0,     0,   250,     0,     0,     0,   449,     0,
     138,     0,     0,     0,   116,     0,     0,   115,     0,     0,
     455,     0,     0,   456,     0,     0,   116,     0,     0,     0,
     115,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,   433,   190,     0,     0,     0,   138,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,   170,   171,
       0,     0,     0,   116,   172,   173,     0,   175,   176,     0,
       0,   239,     0,     0,     0,   116,     0,   239,     0,     0,
       0,     0,     0,     0,     0,     0,     0,   395,     0,   386,
     398,   399,   400,   401,   402,   403,   404,   138,     0,   407,
     408,   409,   410,   411,   412,   413,   414,   415,     0,     0,
     180,   181,   182,   183,   184,   185,   186,   187,   188,     0,
     136,   189,     0,     0,     0,   331,     0,   332,   333,   334,
     335,   336,   337,   338,     0,     0,   341,   342,   343,   344,
     345,   346,   347,   348,   349,   350,     0,   220,   221,     0,
       0,     0,     0,   222,   223,   224,   225,     0,     0,     0,
       0,     0,     0,   439,     0,   226,   116,     0,     0,   116,
     116,   116,   116,   116,   116,   116,     0,     0,   116,   116,
     116,   116,   116,   116,   116,   116,   116,     0,     0,   227,
       0,     0,     0,     0,     0,     0,   239,   228,   454,   229,
     230,   231,   232,   233,   234,   235,   236,   237,     0,   136,
     238,   458,     0,   352,     0,     0,   190,   190,   190,   190,
     190,   190,   190,   239,     0,   190,   190,   190,   190,   190,
     190,   190,   190,   190,   392,   239,     0,     0,     0,     0,
       0,     0,   116,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,   190,   264,   265,
     266,   267,   268,   269,   270,     0,     0,   273,   274,   275,
     276,   277,   278,   279,   280,   281,     0,   116,     0,     0,
       0,     0,     0,     0,     0,     0,   305,     0,     0,     0,
     116,   239,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,   437,   239,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,   440,     0,     0,     0,   190,     0,
       0,     0,     0,     0,     0,   441,     0,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    22,    23,   453,   239,     0,     0,
     239,   239,   239,   239,   239,   239,   239,     0,     0,   239,
     239,   239,   239,   239,   239,   239,   239,   239,     0,     0,
       0,     0,   366,     0,     0,   190,     0,     0,     0,     0,
     190,    41,    42,    43,    44,    45,    46,    47,     0,     0,
       0,   239,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,   239,     0,   102,     0,
     239,    52,     0,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
      22,    23,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,    24,    25,    26,    27,    28,    29,
      30,   423,     0,    31,    32,    33,     0,     0,     0,    34,
      35,    36,   428,    37,    38,    39,    40,    41,    42,    43,
      44,    45,    46,    47,     0,     0,     0,     0,     1,    48,
       0,     0,     0,     0,     0,     0,     0,    49,     0,     0,
       0,     0,     0,     0,    50,    51,     0,    52,     0,     0,
       0,     0,    53,    54,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    22,    23,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,    24,    25,    26,    27,    28,
      29,    30,     0,     0,    31,    32,    33,     0,     0,     0,
      34,    35,    36,     0,    37,    38,    39,    40,    41,    42,
      43,    44,    45,    46,    47,     0,     0,     0,     0,     0,
      48,     0,     0,     0,     0,     0,     0,     0,    49,     0,
       0,     0,     0,     0,     0,    50,    51,     0,    52,     0,
       0,     0,     0,    53,    54,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    22,    23,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,    24,    25,    26,    27,
      28,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,   113,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    21,    22,
      23,   114,     0,     0,     0,     0,     0,     0,     0,    49,
       0,     0,     0,     0,     0,   122,    50,    51,     0,    52,
       0,     0,     0,     0,    92,    93,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    22,    23,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,    24,    25,    26,
      27,    28,   245,   102,     0,     0,    52,     0,     0,     0,
       0,     0,     0,   113,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
      22,    23,   114,     0,     0,     0,     0,     0,     0,     0,
      49,     0,   434,     0,     0,     0,     0,    50,    51,     0,
      52,   126,     0,     0,     0,    92,    93,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    22,    23,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,    24,    25,
      26,    27,    28,     0,   102,     0,     0,    52,     0,     0,
       0,     0,     0,     0,    91,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    22,    23,    48,     0,     0,     0,     0,     0,     0,
       0,    49,     0,     0,     0,     0,     0,     0,    50,    51,
       0,    52,     0,     0,     0,     0,    92,    93,     5,     6,
       7,     8,     9,    10,    11,    12,    13,    14,    15,    16,
      17,    18,    19,    20,    21,    22,    23,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,    24,
      25,    26,    27,    28,     0,   102,     0,     0,    52,     0,
       0,     0,     0,     0,     0,   113,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,   170,   171,     0,     0,
       0,     0,   172,   173,   114,     0,   176,     0,     0,     0,
       0,     0,    49,     0,     0,     0,     0,     0,     0,    50,
      51,     0,    52,     0,     0,     0,     0,    92,    93,     5,
       6,     7,     8,     9,    10,    11,    12,    13,    14,    15,
      16,    17,    18,    19,    20,    21,    22,    23,   180,   181,
     182,   183,   184,   185,   186,   187,   188,     0,   136,   189,
      24,    25,    26,    27,    28,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    22,    23,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,    24,    25,    26,    27,
      28,     0,     0,    49,     0,     0,     0,     0,   217,   240,
      50,    51,     0,    52,     0,     0,     0,     0,    92,    93,
       0,     0,     0,   220,   221,     0,     0,     0,     0,   222,
     223,     0,   225,     0,     0,     0,     0,     0,     0,    49,
       0,     0,     0,     0,   144,     0,    50,    51,     0,    52,
       0,     0,     0,     0,    92,    93,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    22,    23,   229,   230,   231,   232,   233,
     234,   235,   236,   237,     0,   136,   238,    24,    25,    26,
      27,    28,     5,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    21,    22,
      23,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,    24,    25,    26,    27,    28,     0,     0,
      49,     0,     0,     0,     0,   217,     0,    50,    51,     0,
      52,     0,     0,     0,     0,    92,    93,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    22,    23,    49,     0,     0,     0,
       0,     0,     0,    50,    51,     0,    52,     0,     0,     0,
       0,    92,    93,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
      22,    23,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,    24,    25,    26,    27,    28,     0,
       0,     0,     0,     0,     0,     0,     0,     0,   102,     0,
       0,    52,     6,     7,     8,     9,    10,    11,    12,    13,
      14,    15,    16,    17,    18,    19,    20,    21,    22,    23,
       0,     0,     0,     0,     0,     0,     0,    49,     0,     0,
       0,     0,     0,     0,    50,    51,     0,    52,     0,     0,
       0,     0,    53,    54,     0,     0,     0,     0,     0,     0,
       0,     0,    38,     0,     0,    41,    42,    43,    44,    45,
      46,    47,     6,     7,     8,     9,    10,    11,    12,    13,
      14,    15,    16,    17,    18,    19,    20,    21,    22,    23,
       0,     0,   102,     0,     0,    52,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,   220,   221,     0,     0,
       0,     0,   222,   223,   224,   225,     0,     0,     0,   148,
       0,   149,   150,   151,   226,   220,   221,     0,     0,     0,
       0,   222,   223,   224,   225,     0,     0,     0,     0,     0,
       0,     0,     0,   226,     0,     0,     0,     0,   227,     0,
       0,     0,   102,     0,     0,    52,   228,     0,   229,   230,
     231,   232,   233,   234,   235,   236,   237,   227,   136,   238,
       0,     0,   365,     0,     0,   228,     0,   229,   230,   231,
     232,   233,   234,   235,   236,   237,     0,   136,   238,   170,
     171,   417,     0,     0,     0,   172,   173,   174,   175,   176,
       0,     0,     0,     0,     0,     0,     0,   177,   170,   171,
       0,     0,     0,   374,   172,   173,   174,   175,   176,     0,
       0,     0,     0,     0,     0,     0,   177,     0,     0,     0,
       0,   178,     0,     0,     0,     0,     0,     0,     0,   179,
       0,   180,   181,   182,   183,   184,   185,   186,   187,   188,
     178,   136,   189,     0,     0,     0,     0,     0,   179,     0,
     180,   181,   182,   183,   184,   185,   186,   187,   188,     0,
     136,   189,   220,   221,     0,     0,     0,     0,   222,   223,
     224,   225,     0,     0,     0,     0,     0,     0,     0,     0,
     226,     0,     0,     0,     0,   220,   221,     0,     0,     0,
       0,   222,   223,   224,   225,     0,     0,     0,     0,     0,
       0,     0,     0,   226,   227,   363,     0,     0,     0,     0,
       0,     0,   228,     0,   229,   230,   231,   232,   233,   234,
     235,   236,   237,   242,   136,   238,     0,   227,     0,     0,
       0,     0,     0,     0,     0,   228,     0,   229,   230,   231,
     232,   233,   234,   235,   236,   237,     0,   136,   238,   220,
     221,     0,     0,     0,     0,   222,   223,   224,   225,     0,
       0,     0,     0,     0,     0,     0,     0,   226,     0,   405,
       0,   220,   221,     0,     0,     0,     0,   222,   223,   224,
     225,     0,     0,     0,     0,     0,     0,     0,     0,   226,
       0,   227,     0,     0,     0,   438,     0,     0,     0,   228,
       0,   229,   230,   231,   232,   233,   234,   235,   236,   237,
       0,   136,   238,   227,     0,     0,     0,     0,     0,     0,
       0,   228,     0,   229,   230,   231,   232,   233,   234,   235,
     236,   237,     0,   136,   238,   220,   221,     0,     0,     0,
       0,   222,   223,   224,   225,     0,     0,     0,     0,     0,
       0,     0,     0,   226,   170,   171,     0,     0,     0,     0,
     172,   173,   174,   175,   176,     0,     0,     0,     0,     0,
       0,     0,     0,   170,   171,     0,     0,   227,     0,   172,
     173,   174,   175,   176,     0,   228,     0,   229,   230,   231,
     232,   233,   234,   235,   236,   237,   178,   136,   238,     0,
       0,     0,     0,     0,   179,     0,   180,   181,   182,   183,
     184,   185,   186,   187,   188,   178,   136,   189,     0,     0,
       0,     0,     0,     0,     0,   180,   181,   182,   183,   184,
     185,   186,   187,   188,     0,   136,   189,   220,   221,     0,
       0,     0,     0,   222,   223,   224,   225,     0,     0,     0,
       0,     0,     0,     0,     0,     0,   220,   221,     0,     0,
       0,     0,   222,   223,   224,   225,     0,     0,   220,   221,
       0,     0,     0,     0,   222,   223,     0,     0,     0,   227,
       0,     0,     0,     0,     0,     0,     0,   228,     0,   229,
     230,   231,   232,   233,   234,   235,   236,   237,   227,   136,
     238,     0,     0,     0,     0,     0,     0,     0,   229,   230,
     231,   232,   233,   234,   235,   236,   237,     0,   136,   238,
     229,   230,   231,   232,   233,   234,   235,   236,   237,     0,
     136,   238
};

static const short int yycheck[] =
{
       3,    55,   109,   257,    35,   144,   205,   192,   192,    37,
      38,   192,    69,   192,   192,   192,     4,    74,    73,    97,
      68,    99,   192,   192,     0,    29,    30,    44,    32,   214,
      34,   192,    35,   203,    89,    33,    33,    40,   192,   192,
      33,     3,   192,   150,     4,    48,    49,    50,    51,   188,
      53,    54,    55,    48,   192,   254,    51,    24,    25,    26,
      27,    33,    78,    66,    41,    42,    69,   321,    84,    85,
     324,    69,    69,    84,   142,   143,    69,     3,    68,    90,
     108,    69,    41,    42,    24,    25,    26,    27,    68,    92,
      93,    69,    95,    91,    91,    68,   100,    69,    68,   102,
     285,   285,   105,    68,   285,    31,   285,   285,   285,    35,
      36,   114,   115,    39,    68,   285,   285,   120,   196,   114,
     123,   149,   150,    90,   285,    46,    52,    53,    54,    55,
      68,   285,   285,   136,   137,   285,    84,   140,    81,    82,
      66,    84,    85,   328,    84,    85,    78,   285,   255,    78,
      90,    68,    84,    85,    68,    84,    85,   356,    53,    54,
     164,   165,   166,   167,   168,    91,   420,   421,    68,    68,
      70,    70,   203,    69,   177,    73,   204,    75,   104,   105,
      68,    68,   436,   214,    68,    68,   189,   113,   192,   192,
     194,    69,   331,    32,   198,    70,    90,    90,   202,    73,
     203,   455,    81,    82,    90,    84,    85,   210,   211,   379,
      83,   350,    88,    78,    79,    80,    81,    82,   144,    84,
      85,    88,   148,   226,   302,   151,   152,    68,    50,    83,
      90,   285,    90,    69,    90,   238,    70,    76,    77,    78,
      79,    80,    81,    82,    90,    84,    85,    90,   252,    54,
      83,    70,   178,    90,    70,    76,    77,    78,    79,    80,
      81,    82,   188,    84,    85,    90,   192,    90,    76,    77,
      78,    79,    80,    81,    82,   201,    84,    85,    55,    73,
      70,   285,   285,    78,    79,    80,    81,    82,    68,    84,
      85,   319,   370,    90,    70,    68,    68,   328,    90,    82,
      68,   227,   306,   307,   285,    48,    49,    50,    51,    68,
      53,    54,   192,   254,   318,   137,    57,    58,    59,    60,
      61,    62,    63,    66,   435,   360,   329,   379,    -1,   332,
     333,   334,   335,   336,   337,   338,    -1,    -1,   341,   342,
     343,   344,   345,   346,   347,   348,   349,   425,   379,    92,
      93,   355,    95,   381,    -1,   177,   359,   360,   362,   285,
      -1,    -1,    -1,    -1,    -1,   369,    -1,   189,    -1,    -1,
     448,   114,   115,   377,    -1,    -1,   379,   120,   456,    -1,
     123,    -1,    -1,    55,   310,    -1,   389,   390,    -1,    -1,
      -1,    -1,    -1,   136,   137,    -1,   322,    -1,    -1,    -1,
      -1,    -1,   405,    -1,   226,   331,    55,    -1,    57,    58,
      59,    60,    61,    62,    63,    -1,   238,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,   350,    -1,    -1,    -1,   432,    -1,
      58,    -1,    -1,    -1,   177,    -1,    -1,   440,    -1,    -1,
     443,    -1,    -1,   447,    -1,    -1,   189,    -1,    -1,    -1,
     453,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,   387,    90,    -1,    -1,    -1,    94,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    22,    23,
      -1,    -1,    -1,   226,    28,    29,    -1,    31,    32,    -1,
      -1,   119,    -1,    -1,    -1,   238,    -1,   125,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,   329,    -1,   435,
     332,   333,   334,   335,   336,   337,   338,   145,    -1,   341,
     342,   343,   344,   345,   346,   347,   348,   349,    -1,    -1,
      74,    75,    76,    77,    78,    79,    80,    81,    82,    -1,
      84,    85,    -1,    -1,    -1,   217,    -1,   219,   220,   221,
     222,   223,   224,   225,    -1,    -1,   228,   229,   230,   231,
     232,   233,   234,   235,   236,   237,    -1,    22,    23,    -1,
      -1,    -1,    -1,    28,    29,    30,    31,    -1,    -1,    -1,
      -1,    -1,    -1,   405,    -1,    40,   329,    -1,    -1,   332,
     333,   334,   335,   336,   337,   338,    -1,    -1,   341,   342,
     343,   344,   345,   346,   347,   348,   349,    -1,    -1,    64,
      -1,    -1,    -1,    -1,    -1,    -1,   244,    72,   440,    74,
      75,    76,    77,    78,    79,    80,    81,    82,    -1,    84,
      85,   453,    -1,    88,    -1,    -1,   264,   265,   266,   267,
     268,   269,   270,   271,    -1,   273,   274,   275,   276,   277,
     278,   279,   280,   281,   326,   283,    -1,    -1,    -1,    -1,
      -1,    -1,   405,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,   305,   170,   171,
     172,   173,   174,   175,   176,    -1,    -1,   179,   180,   181,
     182,   183,   184,   185,   186,   187,    -1,   440,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,   198,    -1,    -1,    -1,
     453,   339,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,   394,   351,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,   406,    -1,    -1,    -1,   366,    -1,
      -1,    -1,    -1,    -1,    -1,   417,    -1,     3,     4,     5,
       6,     7,     8,     9,    10,    11,    12,    13,    14,    15,
      16,    17,    18,    19,    20,    21,   438,   395,    -1,    -1,
     398,   399,   400,   401,   402,   403,   404,    -1,    -1,   407,
     408,   409,   410,   411,   412,   413,   414,   415,    -1,    -1,
      -1,    -1,   284,    -1,    -1,   423,    -1,    -1,    -1,    -1,
     428,    57,    58,    59,    60,    61,    62,    63,    -1,    -1,
      -1,   439,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,   454,    -1,    84,    -1,
     458,    87,    -1,     3,     4,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    34,    35,    36,    37,    38,    39,
      40,   363,    -1,    43,    44,    45,    -1,    -1,    -1,    49,
      50,    51,   374,    53,    54,    55,    56,    57,    58,    59,
      60,    61,    62,    63,    -1,    -1,    -1,    -1,    68,    69,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    77,    -1,    -1,
      -1,    -1,    -1,    -1,    84,    85,    -1,    87,    -1,    -1,
      -1,    -1,    92,    93,     3,     4,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    34,    35,    36,    37,    38,
      39,    40,    -1,    -1,    43,    44,    45,    -1,    -1,    -1,
      49,    50,    51,    -1,    53,    54,    55,    56,    57,    58,
      59,    60,    61,    62,    63,    -1,    -1,    -1,    -1,    -1,
      69,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    77,    -1,
      -1,    -1,    -1,    -1,    -1,    84,    85,    -1,    87,    -1,
      -1,    -1,    -1,    92,    93,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    34,    35,    36,    37,
      38,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    50,     4,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    69,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    77,
      -1,    -1,    -1,    -1,    -1,    83,    84,    85,    -1,    87,
      -1,    -1,    -1,    -1,    92,    93,     3,     4,     5,     6,
       7,     8,     9,    10,    11,    12,    13,    14,    15,    16,
      17,    18,    19,    20,    21,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    34,    35,    36,
      37,    38,    83,    84,    -1,    -1,    87,    -1,    -1,    -1,
      -1,    -1,    -1,    50,     4,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    69,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      77,    -1,    32,    -1,    -1,    -1,    -1,    84,    85,    -1,
      87,    88,    -1,    -1,    -1,    92,    93,     3,     4,     5,
       6,     7,     8,     9,    10,    11,    12,    13,    14,    15,
      16,    17,    18,    19,    20,    21,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    34,    35,
      36,    37,    38,    -1,    84,    -1,    -1,    87,    -1,    -1,
      -1,    -1,    -1,    -1,    50,     4,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    69,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    77,    -1,    -1,    -1,    -1,    -1,    -1,    84,    85,
      -1,    87,    -1,    -1,    -1,    -1,    92,    93,     3,     4,
       5,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    34,
      35,    36,    37,    38,    -1,    84,    -1,    -1,    87,    -1,
      -1,    -1,    -1,    -1,    -1,    50,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    22,    23,    -1,    -1,
      -1,    -1,    28,    29,    69,    -1,    32,    -1,    -1,    -1,
      -1,    -1,    77,    -1,    -1,    -1,    -1,    -1,    -1,    84,
      85,    -1,    87,    -1,    -1,    -1,    -1,    92,    93,     3,
       4,     5,     6,     7,     8,     9,    10,    11,    12,    13,
      14,    15,    16,    17,    18,    19,    20,    21,    74,    75,
      76,    77,    78,    79,    80,    81,    82,    -1,    84,    85,
      34,    35,    36,    37,    38,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    34,    35,    36,    37,
      38,    -1,    -1,    77,    -1,    -1,    -1,    -1,    82,    83,
      84,    85,    -1,    87,    -1,    -1,    -1,    -1,    92,    93,
      -1,    -1,    -1,    22,    23,    -1,    -1,    -1,    -1,    28,
      29,    -1,    31,    -1,    -1,    -1,    -1,    -1,    -1,    77,
      -1,    -1,    -1,    -1,    82,    -1,    84,    85,    -1,    87,
      -1,    -1,    -1,    -1,    92,    93,     3,     4,     5,     6,
       7,     8,     9,    10,    11,    12,    13,    14,    15,    16,
      17,    18,    19,    20,    21,    74,    75,    76,    77,    78,
      79,    80,    81,    82,    -1,    84,    85,    34,    35,    36,
      37,    38,     3,     4,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    34,    35,    36,    37,    38,    -1,    -1,
      77,    -1,    -1,    -1,    -1,    82,    -1,    84,    85,    -1,
      87,    -1,    -1,    -1,    -1,    92,    93,     3,     4,     5,
       6,     7,     8,     9,    10,    11,    12,    13,    14,    15,
      16,    17,    18,    19,    20,    21,    77,    -1,    -1,    -1,
      -1,    -1,    -1,    84,    85,    -1,    87,    -1,    -1,    -1,
      -1,    92,    93,     3,     4,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    34,    35,    36,    37,    38,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    84,    -1,
      -1,    87,     4,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    77,    -1,    -1,
      -1,    -1,    -1,    -1,    84,    85,    -1,    87,    -1,    -1,
      -1,    -1,    92,    93,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    54,    -1,    -1,    57,    58,    59,    60,    61,
      62,    63,     4,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
      -1,    -1,    84,    -1,    -1,    87,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    22,    23,    -1,    -1,
      -1,    -1,    28,    29,    30,    31,    -1,    -1,    -1,    51,
      -1,    53,    54,    55,    40,    22,    23,    -1,    -1,    -1,
      -1,    28,    29,    30,    31,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    40,    -1,    -1,    -1,    -1,    64,    -1,
      -1,    -1,    84,    -1,    -1,    87,    72,    -1,    74,    75,
      76,    77,    78,    79,    80,    81,    82,    64,    84,    85,
      -1,    -1,    88,    -1,    -1,    72,    -1,    74,    75,    76,
      77,    78,    79,    80,    81,    82,    -1,    84,    85,    22,
      23,    88,    -1,    -1,    -1,    28,    29,    30,    31,    32,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    40,    22,    23,
      -1,    -1,    -1,    46,    28,    29,    30,    31,    32,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    40,    -1,    -1,    -1,
      -1,    64,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    72,
      -1,    74,    75,    76,    77,    78,    79,    80,    81,    82,
      64,    84,    85,    -1,    -1,    -1,    -1,    -1,    72,    -1,
      74,    75,    76,    77,    78,    79,    80,    81,    82,    -1,
      84,    85,    22,    23,    -1,    -1,    -1,    -1,    28,    29,
      30,    31,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      40,    -1,    -1,    -1,    -1,    22,    23,    -1,    -1,    -1,
      -1,    28,    29,    30,    31,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    40,    64,    42,    -1,    -1,    -1,    -1,
      -1,    -1,    72,    -1,    74,    75,    76,    77,    78,    79,
      80,    81,    82,    83,    84,    85,    -1,    64,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    72,    -1,    74,    75,    76,
      77,    78,    79,    80,    81,    82,    -1,    84,    85,    22,
      23,    -1,    -1,    -1,    -1,    28,    29,    30,    31,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    40,    -1,    42,
      -1,    22,    23,    -1,    -1,    -1,    -1,    28,    29,    30,
      31,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    40,
      -1,    64,    -1,    -1,    -1,    46,    -1,    -1,    -1,    72,
      -1,    74,    75,    76,    77,    78,    79,    80,    81,    82,
      -1,    84,    85,    64,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    72,    -1,    74,    75,    76,    77,    78,    79,    80,
      81,    82,    -1,    84,    85,    22,    23,    -1,    -1,    -1,
      -1,    28,    29,    30,    31,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    40,    22,    23,    -1,    -1,    -1,    -1,
      28,    29,    30,    31,    32,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    22,    23,    -1,    -1,    64,    -1,    28,
      29,    30,    31,    32,    -1,    72,    -1,    74,    75,    76,
      77,    78,    79,    80,    81,    82,    64,    84,    85,    -1,
      -1,    -1,    -1,    -1,    72,    -1,    74,    75,    76,    77,
      78,    79,    80,    81,    82,    64,    84,    85,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    74,    75,    76,    77,    78,
      79,    80,    81,    82,    -1,    84,    85,    22,    23,    -1,
      -1,    -1,    -1,    28,    29,    30,    31,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    22,    23,    -1,    -1,
      -1,    -1,    28,    29,    30,    31,    -1,    -1,    22,    23,
      -1,    -1,    -1,    -1,    28,    29,    -1,    -1,    -1,    64,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    72,    -1,    74,
      75,    76,    77,    78,    79,    80,    81,    82,    64,    84,
      85,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    74,    75,
      76,    77,    78,    79,    80,    81,    82,    -1,    84,    85,
      74,    75,    76,    77,    78,    79,    80,    81,    82,    -1,
      84,    85
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
      22,    23,    28,    29,    30,    31,    32,    40,    64,    72,
      74,    75,    76,    77,    78,    79,    80,    81,    82,    85,
     146,   101,    69,   125,    46,   125,   149,   109,    90,   101,
     101,   109,    90,    69,    74,   117,   121,   102,   122,   123,
      33,    91,   125,   130,    69,   101,   151,    82,    70,    73,
      22,    23,    28,    29,    30,    31,    40,    64,    72,    74,
      75,    76,    77,    78,    79,    80,    81,    82,    85,   146,
      83,    83,    83,    88,   153,    83,   109,    88,   107,   107,
     101,   143,    90,   101,   102,   102,   123,   101,   101,   149,
     149,   149,   149,   149,   150,   150,   150,   150,   150,   150,
     150,   153,   101,   150,   150,   150,   150,   150,   150,   150,
     150,   150,   143,   153,    90,    96,    99,   100,   113,   114,
     115,   118,   124,   131,   133,   134,   139,   140,   141,   142,
     149,    68,   149,   125,   149,   150,    90,    90,   101,   149,
     109,   112,   119,   120,   124,   102,   116,   121,    90,    68,
      70,   109,   109,   128,   129,   112,   131,   135,   136,    90,
      70,    97,    97,    97,    97,    97,    97,    97,    97,   153,
     101,    97,    97,    97,    97,    97,    97,    97,    97,    97,
      97,   153,    88,    83,   149,    90,   117,   121,   123,    33,
      91,   130,    90,    42,    90,    88,   150,    99,    70,    41,
      42,   137,   138,   125,    46,   149,   149,    90,   101,    68,
      70,    73,    75,   149,   102,   130,   101,   126,   127,    73,
      33,   130,    97,    70,   131,   153,    68,   143,   153,   153,
     153,   153,   153,   153,   153,    42,    90,   153,   153,   153,
     153,   153,   153,   153,   153,   153,   143,    88,   149,   121,
     109,   129,   149,   150,    70,   149,   125,    68,   150,   149,
     119,   102,    90,   101,    32,   109,   109,    97,    46,   153,
      97,    97,   130,    33,   130,   125,    68,    41,    42,   149,
      82,   127,   130,    97,   153,   109,   149,   125,   153,   130,
     125,    68
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
     150,   151,   152,   152,   153,   153,   153,   153,   153,   153,
     153,   153,   153,   153,   153,   153,   153,   153,   153,   153,
     153,   153,   153,   153,   153,   153,   153,   153
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
       3,     3,     3,     3,     3,     3,     3,     4,     4,     6,
       3,     3,     3,     3,     3,     3,     3,     2,     5,     1,
       3,     1,     4,     1,     4,     4,     4,     4,     4,     4,
       4,     4,     4,     4,     4,     5,     6,     7,     4,     4,
       4,     4,     4,     4,     2,     5,     1,     4
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
       2,     2,     2,     2,     2,     2,     2,     0,     0,     2,
       0,     0,     0,     0,     1,     1,     1,     1,     1,     1,
       1,     1,     1,     2,     1,     1,     3,     3,     1,     1,
       1,     1,     1,     1,     0,     0,     1,     1
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
       0,     0,     0,     0,     0,     0,     0,     0
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
       0,     0,     0,     0,     0,     0,     0,     0
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
       0,     0,     0,     0,     0,     0,     0,     0,     1,     3,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     5,     7,     0,     0,     0,
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
       0,     0
};

/* YYCONFL[I] -- lists of conflicting rule numbers, each terminated by
   0, pointed into by YYCONFLP.  */
static const short int yyconfl[] =
{
       0,    68,     0,    68,     0,    67,     0,    67,     0
};

/* Error token number */
#define YYTERROR 1




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

/* This macro is provided for backward compatibility. */
#ifndef YY_LOCATION_PRINT
# define YY_LOCATION_PRINT(File, Loc) ((void) 0)
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
yy_symbol_print (FILE *, int yytype, const yy::parser::semantic_type *yyvaluep, yy::parser& yyparser)
{
  YYUSE (yyparser);
  yyparser.yy_symbol_print_ (yytype, yyvaluep);
}


# define YY_SYMBOL_PRINT(Title, Type, Value, Location)                  \
  do {                                                                  \
    if (yydebug)                                                        \
      {                                                                 \
        YYFPRINTF (stderr, "%s ", Title);                               \
        yy_symbol_print (stderr, Type, Value, yyparser);        \
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


  int yyerrcnt;
  int yyrawchar;
  YYSTYPE yyval;

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
yyFail (yyGLRStack* yystackp, yy::parser& yyparser, const char* yymsg)
{
  if (yymsg != YY_NULLPTR)
    yyerror (yyparser, yymsg);
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
              YYSTYPE* yyvalp, yy::parser& yyparser)
{
  yybool yynormal YY_ATTRIBUTE_UNUSED = (yystackp->yysplitPoint == YY_NULLPTR);
  int yylow;
  YYUSE (yyvalp);
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
  return yyerror (yyparser, YY_("syntax error: cannot back up")),     \
         yyerrok, yyerr

  yylow = 1;
  if (yyrhslen == 0)
    *yyvalp = yyval_default;
  else
    *yyvalp = yyvsp[YYFILL (1-yyrhslen)].yystate.yysemantics.yysval;
  switch (yyn)
    {
        case 3:
#line 121 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 1817 "src/parser.cpp" // glr.c:816
    break;

  case 4:
#line 122 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot((((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 1823 "src/parser.cpp" // glr.c:816
    break;

  case 35:
#line 165 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkImportNode((((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 1829 "src/parser.cpp" // glr.c:816
    break;

  case 36:
#line 167 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (Node*)lextxt;}
#line 1835 "src/parser.cpp" // glr.c:816
    break;

  case 37:
#line 170 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (Node*)lextxt;}
#line 1841 "src/parser.cpp" // glr.c:816
    break;

  case 38:
#line 173 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkIntLitNode(lextxt);}
#line 1847 "src/parser.cpp" // glr.c:816
    break;

  case 39:
#line 176 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFltLitNode(lextxt);}
#line 1853 "src/parser.cpp" // glr.c:816
    break;

  case 40:
#line 179 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkStrLitNode(lextxt);}
#line 1859 "src/parser.cpp" // glr.c:816
    break;

  case 41:
#line 182 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_I8,  (char*)"");}
#line 1865 "src/parser.cpp" // glr.c:816
    break;

  case 42:
#line 183 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_I16, (char*)"");}
#line 1871 "src/parser.cpp" // glr.c:816
    break;

  case 43:
#line 184 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_I32, (char*)"");}
#line 1877 "src/parser.cpp" // glr.c:816
    break;

  case 44:
#line 185 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_I64, (char*)"");}
#line 1883 "src/parser.cpp" // glr.c:816
    break;

  case 45:
#line 186 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_U8,  (char*)"");}
#line 1889 "src/parser.cpp" // glr.c:816
    break;

  case 46:
#line 187 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_U16, (char*)"");}
#line 1895 "src/parser.cpp" // glr.c:816
    break;

  case 47:
#line 188 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_U32, (char*)"");}
#line 1901 "src/parser.cpp" // glr.c:816
    break;

  case 48:
#line 189 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_U64, (char*)"");}
#line 1907 "src/parser.cpp" // glr.c:816
    break;

  case 49:
#line 190 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_Isz, (char*)"");}
#line 1913 "src/parser.cpp" // glr.c:816
    break;

  case 50:
#line 191 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_Usz, (char*)"");}
#line 1919 "src/parser.cpp" // glr.c:816
    break;

  case 51:
#line 192 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_F16, (char*)"");}
#line 1925 "src/parser.cpp" // glr.c:816
    break;

  case 52:
#line 193 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_F32, (char*)"");}
#line 1931 "src/parser.cpp" // glr.c:816
    break;

  case 53:
#line 194 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_F64, (char*)"");}
#line 1937 "src/parser.cpp" // glr.c:816
    break;

  case 54:
#line 195 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_C8,  (char*)"");}
#line 1943 "src/parser.cpp" // glr.c:816
    break;

  case 55:
#line 196 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_C32, (char*)"");}
#line 1949 "src/parser.cpp" // glr.c:816
    break;

  case 56:
#line 197 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_Bool, (char*)"");}
#line 1955 "src/parser.cpp" // glr.c:816
    break;

  case 57:
#line 198 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_Void, (char*)"");}
#line 1961 "src/parser.cpp" // glr.c:816
    break;

  case 58:
#line 199 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_Data, (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 1967 "src/parser.cpp" // glr.c:816
    break;

  case 59:
#line 200 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_TypeVar, (char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 1973 "src/parser.cpp" // glr.c:816
    break;

  case 60:
#line 205 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_Ptr,  (char*)"", (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 1979 "src/parser.cpp" // glr.c:816
    break;

  case 61:
#line 206 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_Array,(char*)"", (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval));}
#line 1985 "src/parser.cpp" // glr.c:816
    break;

  case 62:
#line 207 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_Func, (char*)"", (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval));}
#line 1991 "src/parser.cpp" // glr.c:816
    break;

  case 63:
#line 208 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeNode(TT_Func, (char*)"", (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval));}
#line 1997 "src/parser.cpp" // glr.c:816
    break;

  case 64:
#line 209 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval);}
#line 2003 "src/parser.cpp" // glr.c:816
    break;

  case 65:
#line 210 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2009 "src/parser.cpp" // glr.c:816
    break;

  case 66:
#line 213 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2015 "src/parser.cpp" // glr.c:816
    break;

  case 68:
#line 215 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot((((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2021 "src/parser.cpp" // glr.c:816
    break;

  case 69:
#line 218 "src/syntax.y" // glr.c:816
    {Node* tmp = getRoot(); 
                        if(tmp == (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)){//singular type, first type in list equals the last
                            ((*yyvalp)) = tmp;
                        }else{ //tuple type
                            ((*yyvalp)) = mkTypeNode(TT_Tuple, (char*)"", tmp);
                        }
                       }
#line 2033 "src/parser.cpp" // glr.c:816
    break;

  case 70:
#line 227 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkModNode(Tok_Pub);}
#line 2039 "src/parser.cpp" // glr.c:816
    break;

  case 71:
#line 228 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkModNode(Tok_Pri);}
#line 2045 "src/parser.cpp" // glr.c:816
    break;

  case 72:
#line 229 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkModNode(Tok_Pro);}
#line 2051 "src/parser.cpp" // glr.c:816
    break;

  case 73:
#line 230 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkModNode(Tok_Raw);}
#line 2057 "src/parser.cpp" // glr.c:816
    break;

  case 74:
#line 231 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkModNode(Tok_Const);}
#line 2063 "src/parser.cpp" // glr.c:816
    break;

  case 75:
#line 232 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkModNode(Tok_Noinit);}
#line 2069 "src/parser.cpp" // glr.c:816
    break;

  case 76:
#line 233 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkModNode(Tok_Pathogen);}
#line 2075 "src/parser.cpp" // glr.c:816
    break;

  case 77:
#line 236 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2081 "src/parser.cpp" // glr.c:816
    break;

  case 78:
#line 237 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot((((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2087 "src/parser.cpp" // glr.c:816
    break;

  case 79:
#line 240 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = getRoot();}
#line 2093 "src/parser.cpp" // glr.c:816
    break;

  case 80:
#line 244 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarDeclNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-4)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2099 "src/parser.cpp" // glr.c:816
    break;

  case 81:
#line 245 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarDeclNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval),  0);}
#line 2105 "src/parser.cpp" // glr.c:816
    break;

  case 82:
#line 246 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarDeclNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), 0,  (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2111 "src/parser.cpp" // glr.c:816
    break;

  case 83:
#line 248 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarDeclNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-4)].yystate.yysemantics.yysval),  0, (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2117 "src/parser.cpp" // glr.c:816
    break;

  case 84:
#line 249 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarDeclNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), 0,   0, (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2123 "src/parser.cpp" // glr.c:816
    break;

  case 85:
#line 252 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkLetBindingNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-4)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2129 "src/parser.cpp" // glr.c:816
    break;

  case 86:
#line 253 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkLetBindingNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), 0,  (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2135 "src/parser.cpp" // glr.c:816
    break;

  case 87:
#line 254 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkLetBindingNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), 0,  (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2141 "src/parser.cpp" // glr.c:816
    break;

  case 88:
#line 255 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkLetBindingNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), 0,  0,  (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2147 "src/parser.cpp" // glr.c:816
    break;

  case 89:
#line 259 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarAssignNode((((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2153 "src/parser.cpp" // glr.c:816
    break;

  case 90:
#line 260 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarAssignNode((((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), mkBinOpNode('+', mkUnOpNode('@', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval)), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)), false);}
#line 2159 "src/parser.cpp" // glr.c:816
    break;

  case 91:
#line 261 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarAssignNode((((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), mkBinOpNode('-', mkUnOpNode('@', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval)), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)), false);}
#line 2165 "src/parser.cpp" // glr.c:816
    break;

  case 92:
#line 262 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarAssignNode((((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), mkBinOpNode('*', mkUnOpNode('@', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval)), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)), false);}
#line 2171 "src/parser.cpp" // glr.c:816
    break;

  case 93:
#line 263 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarAssignNode((((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), mkBinOpNode('/', mkUnOpNode('@', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval)), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)), false);}
#line 2177 "src/parser.cpp" // glr.c:816
    break;

  case 94:
#line 266 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2183 "src/parser.cpp" // glr.c:816
    break;

  case 95:
#line 267 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot((((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2189 "src/parser.cpp" // glr.c:816
    break;

  case 96:
#line 270 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = getRoot();}
#line 2195 "src/parser.cpp" // glr.c:816
    break;

  case 97:
#line 273 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkDataDeclNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2201 "src/parser.cpp" // glr.c:816
    break;

  case 98:
#line 274 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkDataDeclNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2207 "src/parser.cpp" // glr.c:816
    break;

  case 99:
#line 275 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkDataDeclNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2213 "src/parser.cpp" // glr.c:816
    break;

  case 100:
#line 276 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkDataDeclNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2219 "src/parser.cpp" // glr.c:816
    break;

  case 101:
#line 279 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkNamedValNode(mkVarNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2225 "src/parser.cpp" // glr.c:816
    break;

  case 102:
#line 280 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkNamedValNode(0, (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2231 "src/parser.cpp" // glr.c:816
    break;

  case 104:
#line 284 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2237 "src/parser.cpp" // glr.c:816
    break;

  case 105:
#line 285 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot((((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2243 "src/parser.cpp" // glr.c:816
    break;

  case 106:
#line 288 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = getRoot();}
#line 2249 "src/parser.cpp" // glr.c:816
    break;

  case 112:
#line 301 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = NULL;}
#line 2255 "src/parser.cpp" // glr.c:816
    break;

  case 113:
#line 302 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = NULL;}
#line 2261 "src/parser.cpp" // glr.c:816
    break;

  case 114:
#line 303 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = NULL;}
#line 2267 "src/parser.cpp" // glr.c:816
    break;

  case 115:
#line 304 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = NULL;}
#line 2273 "src/parser.cpp" // glr.c:816
    break;

  case 116:
#line 307 "src/syntax.y" // glr.c:816
    {setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval)); ((*yyvalp)) = getRoot();}
#line 2279 "src/parser.cpp" // glr.c:816
    break;

  case 117:
#line 308 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval);}
#line 2285 "src/parser.cpp" // glr.c:816
    break;

  case 118:
#line 311 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), mkVarNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)));}
#line 2291 "src/parser.cpp" // glr.c:816
    break;

  case 119:
#line 312 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot(mkVarNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)));}
#line 2297 "src/parser.cpp" // glr.c:816
    break;

  case 120:
#line 315 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = getRoot();}
#line 2303 "src/parser.cpp" // glr.c:816
    break;

  case 121:
#line 322 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), mkNamedValNode((((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval)));}
#line 2309 "src/parser.cpp" // glr.c:816
    break;

  case 122:
#line 323 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot(mkNamedValNode((((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval)));}
#line 2315 "src/parser.cpp" // glr.c:816
    break;

  case 123:
#line 327 "src/syntax.y" // glr.c:816
    {setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), mkNamedValNode(mkVarNode((char*)""), 0)); ((*yyvalp)) = getRoot();}
#line 2321 "src/parser.cpp" // glr.c:816
    break;

  case 124:
#line 328 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = getRoot();}
#line 2327 "src/parser.cpp" // glr.c:816
    break;

  case 125:
#line 332 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2333 "src/parser.cpp" // glr.c:816
    break;

  case 126:
#line 333 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = 0;}
#line 2339 "src/parser.cpp" // glr.c:816
    break;

  case 127:
#line 336 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFuncDeclNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-5)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-7)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval),                             (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2345 "src/parser.cpp" // glr.c:816
    break;

  case 128:
#line 337 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFuncDeclNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-5)].yystate.yysemantics.yysval), mkTypeNode(TT_Void, (char*)""), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2351 "src/parser.cpp" // glr.c:816
    break;

  case 129:
#line 338 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFuncDeclNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-5)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval),                              0, (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2357 "src/parser.cpp" // glr.c:816
    break;

  case 130:
#line 339 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFuncDeclNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), mkTypeNode(TT_Void, (char*)""),  0, (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2363 "src/parser.cpp" // glr.c:816
    break;

  case 131:
#line 340 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFuncDeclNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-5)].yystate.yysemantics.yysval),  0, (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval),                             (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2369 "src/parser.cpp" // glr.c:816
    break;

  case 132:
#line 341 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFuncDeclNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval),  0, mkTypeNode(TT_Void, (char*)""), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2375 "src/parser.cpp" // glr.c:816
    break;

  case 133:
#line 342 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFuncDeclNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval),  0, (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval),                              0, (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2381 "src/parser.cpp" // glr.c:816
    break;

  case 134:
#line 343 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFuncDeclNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval),  0, mkTypeNode(TT_Void, (char*)""),  0, (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2387 "src/parser.cpp" // glr.c:816
    break;

  case 135:
#line 347 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkFuncCallNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2393 "src/parser.cpp" // glr.c:816
    break;

  case 136:
#line 350 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkRetNode((((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2399 "src/parser.cpp" // glr.c:816
    break;

  case 137:
#line 354 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkExtNode((((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2405 "src/parser.cpp" // glr.c:816
    break;

  case 138:
#line 358 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = getRoot();}
#line 2411 "src/parser.cpp" // glr.c:816
    break;

  case 139:
#line 360 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2417 "src/parser.cpp" // glr.c:816
    break;

  case 140:
#line 361 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot((((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2423 "src/parser.cpp" // glr.c:816
    break;

  case 141:
#line 370 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setElse((IfNode*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-4)].yystate.yysemantics.yysval), (IfNode*)mkIfNode((((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)));}
#line 2429 "src/parser.cpp" // glr.c:816
    break;

  case 142:
#line 371 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot(mkIfNode((((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)));}
#line 2435 "src/parser.cpp" // glr.c:816
    break;

  case 143:
#line 374 "src/syntax.y" // glr.c:816
    {setElse((IfNode*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-4)].yystate.yysemantics.yysval), (IfNode*)mkIfNode(NULL, (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval)));}
#line 2441 "src/parser.cpp" // glr.c:816
    break;

  case 144:
#line 375 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot((((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2447 "src/parser.cpp" // glr.c:816
    break;

  case 145:
#line 376 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot(mkIfNode(NULL, (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval)));}
#line 2453 "src/parser.cpp" // glr.c:816
    break;

  case 146:
#line 377 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot(NULL);}
#line 2459 "src/parser.cpp" // glr.c:816
    break;

  case 147:
#line 380 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkIfNode((((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (IfNode*)getRoot());}
#line 2465 "src/parser.cpp" // glr.c:816
    break;

  case 148:
#line 383 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkWhileNode((((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2471 "src/parser.cpp" // glr.c:816
    break;

  case 149:
#line 386 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = NULL;}
#line 2477 "src/parser.cpp" // glr.c:816
    break;

  case 150:
#line 389 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = NULL;}
#line 2483 "src/parser.cpp" // glr.c:816
    break;

  case 151:
#line 392 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkVarNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2489 "src/parser.cpp" // glr.c:816
    break;

  case 152:
#line 395 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkUnOpNode('&', (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2495 "src/parser.cpp" // glr.c:816
    break;

  case 153:
#line 396 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkUnOpNode('@', (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2501 "src/parser.cpp" // glr.c:816
    break;

  case 154:
#line 397 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('[', mkRefVarNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval)), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2507 "src/parser.cpp" // glr.c:816
    break;

  case 155:
#line 398 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkRefVarNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2513 "src/parser.cpp" // glr.c:816
    break;

  case 156:
#line 401 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2519 "src/parser.cpp" // glr.c:816
    break;

  case 157:
#line 402 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval);}
#line 2525 "src/parser.cpp" // glr.c:816
    break;

  case 158:
#line 403 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2531 "src/parser.cpp" // glr.c:816
    break;

  case 159:
#line 404 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2537 "src/parser.cpp" // glr.c:816
    break;

  case 160:
#line 405 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2543 "src/parser.cpp" // glr.c:816
    break;

  case 161:
#line 406 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2549 "src/parser.cpp" // glr.c:816
    break;

  case 162:
#line 407 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2555 "src/parser.cpp" // glr.c:816
    break;

  case 163:
#line 408 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2561 "src/parser.cpp" // glr.c:816
    break;

  case 164:
#line 409 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2567 "src/parser.cpp" // glr.c:816
    break;

  case 165:
#line 410 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBoolLitNode(1);}
#line 2573 "src/parser.cpp" // glr.c:816
    break;

  case 166:
#line 411 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBoolLitNode(0);}
#line 2579 "src/parser.cpp" // glr.c:816
    break;

  case 167:
#line 414 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTupleNode((((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2585 "src/parser.cpp" // glr.c:816
    break;

  case 168:
#line 415 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTupleNode(0);}
#line 2591 "src/parser.cpp" // glr.c:816
    break;

  case 169:
#line 418 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkArrayNode((((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2597 "src/parser.cpp" // glr.c:816
    break;

  case 170:
#line 419 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkArrayNode(0);}
#line 2603 "src/parser.cpp" // glr.c:816
    break;

  case 171:
#line 423 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkUnOpNode('@', (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2609 "src/parser.cpp" // glr.c:816
    break;

  case 172:
#line 424 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkUnOpNode('&', (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2615 "src/parser.cpp" // glr.c:816
    break;

  case 173:
#line 425 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkUnOpNode('-', (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2621 "src/parser.cpp" // glr.c:816
    break;

  case 174:
#line 426 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkTypeCastNode((((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2627 "src/parser.cpp" // glr.c:816
    break;

  case 175:
#line 429 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2633 "src/parser.cpp" // glr.c:816
    break;

  case 176:
#line 431 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('+', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2639 "src/parser.cpp" // glr.c:816
    break;

  case 177:
#line 432 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('-', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2645 "src/parser.cpp" // glr.c:816
    break;

  case 178:
#line 433 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('*', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2651 "src/parser.cpp" // glr.c:816
    break;

  case 179:
#line 434 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('/', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2657 "src/parser.cpp" // glr.c:816
    break;

  case 180:
#line 435 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('%', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2663 "src/parser.cpp" // glr.c:816
    break;

  case 181:
#line 436 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('<', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2669 "src/parser.cpp" // glr.c:816
    break;

  case 182:
#line 437 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('>', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2675 "src/parser.cpp" // glr.c:816
    break;

  case 183:
#line 438 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('^', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2681 "src/parser.cpp" // glr.c:816
    break;

  case 184:
#line 439 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('.', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2687 "src/parser.cpp" // glr.c:816
    break;

  case 185:
#line 440 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('.', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2693 "src/parser.cpp" // glr.c:816
    break;

  case 186:
#line 441 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(';', (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2699 "src/parser.cpp" // glr.c:816
    break;

  case 187:
#line 442 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('[', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval));}
#line 2705 "src/parser.cpp" // glr.c:816
    break;

  case 188:
#line 443 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(Tok_Where, (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), mkLetBindingNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), 0, 0, (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)));}
#line 2711 "src/parser.cpp" // glr.c:816
    break;

  case 189:
#line 444 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(Tok_Let, mkLetBindingNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-4)].yystate.yysemantics.yysval), 0, 0, (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval)), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2717 "src/parser.cpp" // glr.c:816
    break;

  case 190:
#line 445 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(Tok_Eq, (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2723 "src/parser.cpp" // glr.c:816
    break;

  case 191:
#line 446 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(Tok_NotEq, (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2729 "src/parser.cpp" // glr.c:816
    break;

  case 192:
#line 447 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(Tok_GrtrEq, (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2735 "src/parser.cpp" // glr.c:816
    break;

  case 193:
#line 448 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(Tok_LesrEq, (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2741 "src/parser.cpp" // glr.c:816
    break;

  case 194:
#line 449 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(Tok_Or, (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2747 "src/parser.cpp" // glr.c:816
    break;

  case 195:
#line 450 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(Tok_And, (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2753 "src/parser.cpp" // glr.c:816
    break;

  case 196:
#line 451 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(Tok_Range, (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2759 "src/parser.cpp" // glr.c:816
    break;

  case 197:
#line 452 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('(', (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2765 "src/parser.cpp" // glr.c:816
    break;

  case 198:
#line 453 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkExprIfNode((((yyGLRStackItem const *)yyvsp)[YYFILL (-4)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2771 "src/parser.cpp" // glr.c:816
    break;

  case 199:
#line 454 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2777 "src/parser.cpp" // glr.c:816
    break;

  case 200:
#line 455 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval);}
#line 2783 "src/parser.cpp" // glr.c:816
    break;

  case 201:
#line 460 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = getRoot();}
#line 2789 "src/parser.cpp" // glr.c:816
    break;

  case 202:
#line 463 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setNext((((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2795 "src/parser.cpp" // glr.c:816
    break;

  case 203:
#line 464 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = setRoot((((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2801 "src/parser.cpp" // glr.c:816
    break;

  case 204:
#line 468 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('+', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2807 "src/parser.cpp" // glr.c:816
    break;

  case 205:
#line 469 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('-', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2813 "src/parser.cpp" // glr.c:816
    break;

  case 206:
#line 470 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('*', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2819 "src/parser.cpp" // glr.c:816
    break;

  case 207:
#line 471 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('/', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2825 "src/parser.cpp" // glr.c:816
    break;

  case 208:
#line 472 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('%', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2831 "src/parser.cpp" // glr.c:816
    break;

  case 209:
#line 473 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('<', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2837 "src/parser.cpp" // glr.c:816
    break;

  case 210:
#line 474 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('>', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2843 "src/parser.cpp" // glr.c:816
    break;

  case 211:
#line 475 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('^', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2849 "src/parser.cpp" // glr.c:816
    break;

  case 212:
#line 476 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('.', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2855 "src/parser.cpp" // glr.c:816
    break;

  case 213:
#line 477 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('.', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2861 "src/parser.cpp" // glr.c:816
    break;

  case 214:
#line 478 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(';', (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2867 "src/parser.cpp" // glr.c:816
    break;

  case 215:
#line 479 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('[', (((yyGLRStackItem const *)yyvsp)[YYFILL (-4)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval));}
#line 2873 "src/parser.cpp" // glr.c:816
    break;

  case 216:
#line 480 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(Tok_Where, (((yyGLRStackItem const *)yyvsp)[YYFILL (-5)].yystate.yysemantics.yysval), mkLetBindingNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), 0, 0, (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval)));}
#line 2879 "src/parser.cpp" // glr.c:816
    break;

  case 217:
#line 481 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(Tok_Let, mkLetBindingNode((char*)(((yyGLRStackItem const *)yyvsp)[YYFILL (-5)].yystate.yysemantics.yysval), 0, 0, (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval)), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2885 "src/parser.cpp" // glr.c:816
    break;

  case 218:
#line 482 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(Tok_Eq, (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2891 "src/parser.cpp" // glr.c:816
    break;

  case 219:
#line 483 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(Tok_NotEq, (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2897 "src/parser.cpp" // glr.c:816
    break;

  case 220:
#line 484 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(Tok_GrtrEq, (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2903 "src/parser.cpp" // glr.c:816
    break;

  case 221:
#line 485 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(Tok_LesrEq, (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2909 "src/parser.cpp" // glr.c:816
    break;

  case 222:
#line 486 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(Tok_Or, (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2915 "src/parser.cpp" // glr.c:816
    break;

  case 223:
#line 487 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode(Tok_And, (((yyGLRStackItem const *)yyvsp)[YYFILL (-3)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2921 "src/parser.cpp" // glr.c:816
    break;

  case 224:
#line 488 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkBinOpNode('(', (((yyGLRStackItem const *)yyvsp)[YYFILL (-1)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2927 "src/parser.cpp" // glr.c:816
    break;

  case 225:
#line 489 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = mkExprIfNode((((yyGLRStackItem const *)yyvsp)[YYFILL (-4)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval), (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval));}
#line 2933 "src/parser.cpp" // glr.c:816
    break;

  case 226:
#line 490 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (0)].yystate.yysemantics.yysval);}
#line 2939 "src/parser.cpp" // glr.c:816
    break;

  case 227:
#line 491 "src/syntax.y" // glr.c:816
    {((*yyvalp)) = (((yyGLRStackItem const *)yyvsp)[YYFILL (-2)].yystate.yysemantics.yysval);}
#line 2945 "src/parser.cpp" // glr.c:816
    break;


#line 2949 "src/parser.cpp" // glr.c:816
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
yydestruct (const char *yymsg, int yytype, YYSTYPE *yyvaluep, yy::parser& yyparser)
{
  YYUSE (yyvaluep);
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
                &yys->yysemantics.yysval, yyparser);
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
  (!!((Yystate) == (-255)))

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
            YYSTYPE* yyvalp)
{
  yyGLRState* yynewState = &yynewGLRStackItem (yystackp, yytrue)->yystate;

  yynewState->yylrState = yylrState;
  yynewState->yyposn = yyposn;
  yynewState->yyresolved = yytrue;
  yynewState->yypred = yystackp->yytops.yystates[yyk];
  yynewState->yysemantics.yysval = *yyvalp;
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
                                              , yyparser);
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
            YYSTYPE* yyvalp, yy::parser& yyparser)
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
                           yyvalp, yyparser);
    }
  else
    {
      int yyi;
      yyGLRState* yys;
      yyGLRStackItem yyrhsVals[YYMAXRHS + YYMAXLEFT + 1];
      yys = yyrhsVals[YYMAXRHS + YYMAXLEFT].yystate.yypred
        = yystackp->yytops.yystates[yyk];
      for (yyi = 0; yyi < yynrhs; yyi += 1)
        {
          yys = yys->yypred;
          YYASSERT (yys);
        }
      yyupdateSplit (yystackp, yys);
      yystackp->yytops.yystates[yyk] = yys;
      YY_REDUCE_PRINT ((0, yyrhsVals + YYMAXRHS + YYMAXLEFT - 1, yyk, yyrule, yyparser));
      return yyuserAction (yyrule, yynrhs, yyrhsVals + YYMAXRHS + YYMAXLEFT - 1,
                           yystackp, yyvalp, yyparser);
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

      YYRESULTTAG yyflag = yydoAction (yystackp, yyk, yyrule, &yysval, yyparser);
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
                  yyposn, &yysval);
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
                 YYSTYPE* yyvalp, yy::parser& yyparser)
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
  {
    int yychar_current = yychar;
    YYSTYPE yylval_current = yylval;
    yychar = yyopt->yyrawchar;
    yylval = yyopt->yyval;
    yyflag = yyuserAction (yyopt->yyrule, yynrhs,
                           yyrhsVals + YYMAXRHS + YYMAXLEFT - 1,
                           yystackp, yyvalp, yyparser);
    yychar = yychar_current;
    yylval = yylval_current;
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
                   yySemanticOption* yyx1, yy::parser& yyparser)
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

  yyerror (yyparser, YY_("syntax is ambiguous"));
  return yyabort;
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
              return yyreportAmbiguity (yybest, yyp, yyparser);
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
      yyflag = yyresolveAction (yybest, yystackp, &yysval, yyparser);
      if (yyflag == yyok)
        for (yyp = yybest->yynext; yyp != YY_NULLPTR; yyp = yyp->yynext)
          {
            if (yyprec == yydprec[yyp->yyrule])
              {
                YYSTYPE yysval_other;
                yyflag = yyresolveAction (yyp, yystackp, &yysval_other, yyparser);
                if (yyflag != yyok)
                  {
                    yydestruct ("Cleanup: discarding incompletely merged value for",
                                yystos[yys->yylrState],
                                &yysval, yyparser);
                    break;
                  }
                yyuserMerge (yymerger[yyp->yyrule], &yysval, &yysval_other);
              }
          }
    }
  else
    yyflag = yyresolveAction (yybest, yystackp, &yysval, yyparser);

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
                   size_t yyposn, yy::parser& yyparser)
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
              yychar = yylex (&yylval);
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
                                          yyposn, yyparser));
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
  yyerror (yyparser, YY_("syntax error"));
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
      yyerror (yyparser, yymsg);
      YYFREE (yymsg);
    }
  else
    {
      yyerror (yyparser, YY_("syntax error"));
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
          yyFail (yystackp, yyparser, YY_NULLPTR);
        if (yychar != YYEMPTY)
          {
            yytoken = YYTRANSLATE (yychar);
            yydestruct ("Error: discarding",
                        yytoken, &yylval, yyparser);
          }
        YYDPRINTF ((stderr, "Reading a token: "));
        yychar = yylex (&yylval);
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
    yyFail (yystackp, yyparser, YY_NULLPTR);
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
              YY_SYMBOL_PRINT ("Shifting", yystos[yytable[yyj]],
                               &yylval, &yyerrloc);
              yyglrShift (yystackp, 0, yytable[yyj],
                          yys->yyposn, &yylval);
              yys = yystackp->yytops.yystates[0];
              break;
            }
        }
      if (yys->yypred != YY_NULLPTR)
        yydestroyGLRState ("Error: popping", yys, yyparser);
      yystackp->yytops.yystates[0] = yys->yypred;
      yystackp->yynextFree -= 1;
      yystackp->yyspaceLeft += 1;
    }
  if (yystackp->yytops.yystates[0] == YY_NULLPTR)
    yyFail (yystackp, yyparser, YY_NULLPTR);
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

  if (! yyinitGLRStack (yystackp, YYINITDEPTH))
    goto yyexhaustedlab;
  switch (YYSETJMP (yystack.yyexception_buffer))
    {
    case 0: break;
    case 1: goto yyabortlab;
    case 2: goto yyexhaustedlab;
    default: goto yybuglab;
    }
  yyglrShift (&yystack, 0, 0, 0, &yylval);
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
                  yychar = yylex (&yylval);
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
                  yyglrShift (&yystack, 0, yyaction, yyposn, &yylval);
                  if (0 < yystack.yyerrState)
                    yystack.yyerrState -= 1;
                }
              else if (yyisErrorAction (yyaction))
                {

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
            YYCHK1 (yyprocessOneStack (&yystack, yys, yyposn, yyparser));
          yyremoveDeletes (&yystack);
          if (yystack.yytops.yysize == 0)
            {
              yyundeleteLastStack (&yystack);
              if (yystack.yytops.yysize == 0)
                yyFail (&yystack, yyparser, YY_("syntax error"));
              YYCHK1 (yyresolveStack (&yystack, yyparser));
              YYDPRINTF ((stderr, "Returning to deterministic operation.\n"));

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
                          &yylval);
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
  yyerror (yyparser, YY_("memory exhausted"));
  yyresult = 2;
  goto yyreturn;

 yyreturn:
  if (yychar != YYEMPTY)
    yydestruct ("Cleanup: discarding lookahead",
                YYTRANSLATE (yychar), &yylval, yyparser);

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



#line 494 "src/syntax.y" // glr.c:2584


/* location parser error
void yy::parser::error(const location& loc, const string& msg){
    ante::error(msg.c_str(), yylexer->fileName, yylexer->getRow(), yylexer->getCol());
} */

void yy::parser::error(const string& msg){
    ante::error(msg.c_str(), yylexer->fileName, yylexer->getRow(), yylexer->getCol());
}

#endif
#line 4645 "src/parser.cpp" // glr.c:2584

/*------------------.
| Report an error.  |
`------------------*/

static void
yyerror (yy::parser& yyparser, const char* msg)
{
  YYUSE (yyparser);
  yyparser.error (msg);
}



namespace yy {
#line 4661 "src/parser.cpp" // glr.c:2584
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
                           const semantic_type* yyvaluep)
  {
    YYUSE (yyvaluep);
    std::ostream& yyoutput = debug_stream ();
    std::ostream& yyo = yyoutput;
    YYUSE (yyo);
    YYUSE (yytype);
  }


  void
  parser::yy_symbol_print_ (int yytype,
                           const semantic_type* yyvaluep)
  {
    *yycdebug_ << (yytype < YYNTOKENS ? "token" : "nterm")
               << ' ' << yytname[yytype] << " (";
    yy_symbol_value_print_ (yytype, yyvaluep);
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
#line 4736 "src/parser.cpp" // glr.c:2584
