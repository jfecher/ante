/* A Bison parser, made by GNU Bison 3.0.4.  */

/* Bison implementation for Yacc-like parsers in C

   Copyright (C) 1984, 1989-1990, 2000-2015 Free Software Foundation, Inc.

   This program is free software: you can redistribute it and/or modify
   it under the terms of the GNU General Public License as published by
   the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU General Public License for more details.

   You should have received a copy of the GNU General Public License
   along with this program.  If not, see <http://www.gnu.org/licenses/>.  */

/* As a special exception, you may create a larger work that contains
   part or all of the Bison parser skeleton and distribute that work
   under terms of your choice, so long as that work isn't itself a
   parser generator using the skeleton or a modified version thereof
   as a parser skeleton.  Alternatively, if you modify or redistribute
   the parser skeleton itself, you may (at your option) remove this
   special exception, which will cause the skeleton and the resulting
   Bison output files to be licensed under the GNU General Public
   License without this special exception.

   This special exception was added by the Free Software Foundation in
   version 2.2 of Bison.  */

/* C LALR(1) parser skeleton written by Richard Stallman, by
   simplifying the original so-called "semantic" parser.  */

/* All symbols defined below should begin with yy or YY, to avoid
   infringing on user name space.  This should be done even for local
   variables, as they might otherwise be expanded by user macros.
   There are some unavoidable exceptions within include files to
   define necessary library symbols; they are noted "INFRINGES ON
   USER NAME SPACE" below.  */

/* Identify Bison output.  */
#define YYBISON 1

/* Bison version.  */
#define YYBISON_VERSION "3.0.4"

/* Skeleton name.  */
#define YYSKELETON_NAME "yacc.c"

/* Pure parsers.  */
#define YYPURE 0

/* Push parsers.  */
#define YYPUSH 0

/* Pull parsers.  */
#define YYPULL 1




/* Copy the first part of user declarations.  */
#line 1 "src/syntax.y" /* yacc.c:339  */

#ifndef AN_PARSER
#define AN_PARSER

#include <stdlib.h>
#include <stdio.h>
#include <tokens.h>
#include <ptree.h>

extern int yylex(...);

void yyerror(const char *msg);

#define YYSTYPE Node*
#define YYERROR_VERBOSE


#line 84 "src/parser.c" /* yacc.c:339  */

# ifndef YY_NULLPTR
#  if defined __cplusplus && 201103L <= __cplusplus
#   define YY_NULLPTR nullptr
#  else
#   define YY_NULLPTR 0
#  endif
# endif

/* Enabling verbose error messages.  */
#ifdef YYERROR_VERBOSE
# undef YYERROR_VERBOSE
# define YYERROR_VERBOSE 1
#else
# define YYERROR_VERBOSE 0
#endif


/* Debug traces.  */
#ifndef YYDEBUG
# define YYDEBUG 0
#endif
#if YYDEBUG
extern int yydebug;
#endif

/* Token type.  */
#ifndef YYTOKENTYPE
# define YYTOKENTYPE
  enum yytokentype
  {
    Ident = 258,
    UserType = 259,
    I8 = 260,
    I16 = 261,
    I32 = 262,
    I64 = 263,
    U8 = 264,
    U16 = 265,
    U32 = 266,
    U64 = 267,
    Isz = 268,
    Usz = 269,
    F16 = 270,
    F32 = 271,
    F64 = 272,
    C8 = 273,
    C32 = 274,
    Bool = 275,
    Void = 276,
    Eq = 277,
    NotEq = 278,
    AddEq = 279,
    SubEq = 280,
    MulEq = 281,
    DivEq = 282,
    GrtrEq = 283,
    LesrEq = 284,
    Or = 285,
    And = 286,
    True = 287,
    False = 288,
    IntLit = 289,
    FltLit = 290,
    StrLit = 291,
    Return = 292,
    If = 293,
    Elif = 294,
    Else = 295,
    For = 296,
    While = 297,
    Do = 298,
    In = 299,
    Continue = 300,
    Break = 301,
    Import = 302,
    Match = 303,
    Data = 304,
    Enum = 305,
    Pub = 306,
    Pri = 307,
    Pro = 308,
    Raw = 309,
    Const = 310,
    Ext = 311,
    Pathogen = 312,
    Where = 313,
    Infect = 314,
    Cleanse = 315,
    Ct = 316,
    Newline = 317,
    Indent = 318,
    Unindent = 319,
    LOW = 320
  };
#endif
/* Tokens.  */
#define Ident 258
#define UserType 259
#define I8 260
#define I16 261
#define I32 262
#define I64 263
#define U8 264
#define U16 265
#define U32 266
#define U64 267
#define Isz 268
#define Usz 269
#define F16 270
#define F32 271
#define F64 272
#define C8 273
#define C32 274
#define Bool 275
#define Void 276
#define Eq 277
#define NotEq 278
#define AddEq 279
#define SubEq 280
#define MulEq 281
#define DivEq 282
#define GrtrEq 283
#define LesrEq 284
#define Or 285
#define And 286
#define True 287
#define False 288
#define IntLit 289
#define FltLit 290
#define StrLit 291
#define Return 292
#define If 293
#define Elif 294
#define Else 295
#define For 296
#define While 297
#define Do 298
#define In 299
#define Continue 300
#define Break 301
#define Import 302
#define Match 303
#define Data 304
#define Enum 305
#define Pub 306
#define Pri 307
#define Pro 308
#define Raw 309
#define Const 310
#define Ext 311
#define Pathogen 312
#define Where 313
#define Infect 314
#define Cleanse 315
#define Ct 316
#define Newline 317
#define Indent 318
#define Unindent 319
#define LOW 320

/* Value type.  */
#if ! defined YYSTYPE && ! defined YYSTYPE_IS_DECLARED
typedef int YYSTYPE;
# define YYSTYPE_IS_TRIVIAL 1
# define YYSTYPE_IS_DECLARED 1
#endif


extern YYSTYPE yylval;

int yyparse (void);



/* Copy the second part of user declarations.  */

#line 262 "src/parser.c" /* yacc.c:358  */

#ifdef short
# undef short
#endif

#ifdef YYTYPE_UINT8
typedef YYTYPE_UINT8 yytype_uint8;
#else
typedef unsigned char yytype_uint8;
#endif

#ifdef YYTYPE_INT8
typedef YYTYPE_INT8 yytype_int8;
#else
typedef signed char yytype_int8;
#endif

#ifdef YYTYPE_UINT16
typedef YYTYPE_UINT16 yytype_uint16;
#else
typedef unsigned short int yytype_uint16;
#endif

#ifdef YYTYPE_INT16
typedef YYTYPE_INT16 yytype_int16;
#else
typedef short int yytype_int16;
#endif

#ifndef YYSIZE_T
# ifdef __SIZE_TYPE__
#  define YYSIZE_T __SIZE_TYPE__
# elif defined size_t
#  define YYSIZE_T size_t
# elif ! defined YYSIZE_T
#  include <stddef.h> /* INFRINGES ON USER NAME SPACE */
#  define YYSIZE_T size_t
# else
#  define YYSIZE_T unsigned int
# endif
#endif

#define YYSIZE_MAXIMUM ((YYSIZE_T) -1)

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


#if ! defined yyoverflow || YYERROR_VERBOSE

/* The parser invokes alloca or malloc; define the necessary symbols.  */

# ifdef YYSTACK_USE_ALLOCA
#  if YYSTACK_USE_ALLOCA
#   ifdef __GNUC__
#    define YYSTACK_ALLOC __builtin_alloca
#   elif defined __BUILTIN_VA_ARG_INCR
#    include <alloca.h> /* INFRINGES ON USER NAME SPACE */
#   elif defined _AIX
#    define YYSTACK_ALLOC __alloca
#   elif defined _MSC_VER
#    include <malloc.h> /* INFRINGES ON USER NAME SPACE */
#    define alloca _alloca
#   else
#    define YYSTACK_ALLOC alloca
#    if ! defined _ALLOCA_H && ! defined EXIT_SUCCESS
#     include <stdlib.h> /* INFRINGES ON USER NAME SPACE */
      /* Use EXIT_SUCCESS as a witness for stdlib.h.  */
#     ifndef EXIT_SUCCESS
#      define EXIT_SUCCESS 0
#     endif
#    endif
#   endif
#  endif
# endif

# ifdef YYSTACK_ALLOC
   /* Pacify GCC's 'empty if-body' warning.  */
#  define YYSTACK_FREE(Ptr) do { /* empty */; } while (0)
#  ifndef YYSTACK_ALLOC_MAXIMUM
    /* The OS might guarantee only one guard page at the bottom of the stack,
       and a page size can be as small as 4096 bytes.  So we cannot safely
       invoke alloca (N) if N exceeds 4096.  Use a slightly smaller number
       to allow for a few compiler-allocated temporary stack slots.  */
#   define YYSTACK_ALLOC_MAXIMUM 4032 /* reasonable circa 2006 */
#  endif
# else
#  define YYSTACK_ALLOC YYMALLOC
#  define YYSTACK_FREE YYFREE
#  ifndef YYSTACK_ALLOC_MAXIMUM
#   define YYSTACK_ALLOC_MAXIMUM YYSIZE_MAXIMUM
#  endif
#  if (defined __cplusplus && ! defined EXIT_SUCCESS \
       && ! ((defined YYMALLOC || defined malloc) \
             && (defined YYFREE || defined free)))
#   include <stdlib.h> /* INFRINGES ON USER NAME SPACE */
#   ifndef EXIT_SUCCESS
#    define EXIT_SUCCESS 0
#   endif
#  endif
#  ifndef YYMALLOC
#   define YYMALLOC malloc
#   if ! defined malloc && ! defined EXIT_SUCCESS
void *malloc (YYSIZE_T); /* INFRINGES ON USER NAME SPACE */
#   endif
#  endif
#  ifndef YYFREE
#   define YYFREE free
#   if ! defined free && ! defined EXIT_SUCCESS
void free (void *); /* INFRINGES ON USER NAME SPACE */
#   endif
#  endif
# endif
#endif /* ! defined yyoverflow || YYERROR_VERBOSE */


#if (! defined yyoverflow \
     && (! defined __cplusplus \
         || (defined YYSTYPE_IS_TRIVIAL && YYSTYPE_IS_TRIVIAL)))

/* A type that is properly aligned for any stack member.  */
union yyalloc
{
  yytype_int16 yyss_alloc;
  YYSTYPE yyvs_alloc;
};

/* The size of the maximum gap between one aligned stack and the next.  */
# define YYSTACK_GAP_MAXIMUM (sizeof (union yyalloc) - 1)

/* The size of an array large to enough to hold all stacks, each with
   N elements.  */
# define YYSTACK_BYTES(N) \
     ((N) * (sizeof (yytype_int16) + sizeof (YYSTYPE)) \
      + YYSTACK_GAP_MAXIMUM)

# define YYCOPY_NEEDED 1

/* Relocate STACK from its old location to the new one.  The
   local variables YYSIZE and YYSTACKSIZE give the old and new number of
   elements in the stack, and YYPTR gives the new location of the
   stack.  Advance YYPTR to a properly aligned location for the next
   stack.  */
# define YYSTACK_RELOCATE(Stack_alloc, Stack)                           \
    do                                                                  \
      {                                                                 \
        YYSIZE_T yynewbytes;                                            \
        YYCOPY (&yyptr->Stack_alloc, Stack, yysize);                    \
        Stack = &yyptr->Stack_alloc;                                    \
        yynewbytes = yystacksize * sizeof (*Stack) + YYSTACK_GAP_MAXIMUM; \
        yyptr += yynewbytes / sizeof (*yyptr);                          \
      }                                                                 \
    while (0)

#endif

#if defined YYCOPY_NEEDED && YYCOPY_NEEDED
/* Copy COUNT objects from SRC to DST.  The source and destination do
   not overlap.  */
# ifndef YYCOPY
#  if defined __GNUC__ && 1 < __GNUC__
#   define YYCOPY(Dst, Src, Count) \
      __builtin_memcpy (Dst, Src, (Count) * sizeof (*(Src)))
#  else
#   define YYCOPY(Dst, Src, Count)              \
      do                                        \
        {                                       \
          YYSIZE_T yyi;                         \
          for (yyi = 0; yyi < (Count); yyi++)   \
            (Dst)[yyi] = (Src)[yyi];            \
        }                                       \
      while (0)
#  endif
# endif
#endif /* !YYCOPY_NEEDED */

/* YYFINAL -- State number of the termination state.  */
#define YYFINAL  4
/* YYLAST -- Last index in YYTABLE.  */
#define YYLAST   545

/* YYNTOKENS -- Number of terminals.  */
#define YYNTOKENS  83
/* YYNNTS -- Number of nonterminals.  */
#define YYNNTS  45
/* YYNRULES -- Number of rules.  */
#define YYNRULES  136
/* YYNSTATES -- Number of states.  */
#define YYNSTATES  229

/* YYTRANSLATE[YYX] -- Symbol number corresponding to YYX as returned
   by yylex, with out-of-bounds checking.  */
#define YYUNDEFTOK  2
#define YYMAXUTOK   320

#define YYTRANSLATE(YYX)                                                \
  ((unsigned int) (YYX) <= YYMAXUTOK ? yytranslate[YYX] : YYUNDEFTOK)

/* YYTRANSLATE[TOKEN-NUM] -- Symbol number corresponding to TOKEN-NUM
   as returned by yylex, without out-of-bounds checking.  */
static const yytype_uint8 yytranslate[] =
{
       0,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,    73,     2,     2,
      76,    79,    71,    69,    66,    70,    75,    72,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,    82,     2,
      67,    81,    68,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,    77,     2,    78,    74,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,    80,     2,     2,     2,     2,     2,
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
      65
};

#if YYDEBUG
  /* YYRLINE[YYN] -- Source line where rule number YYN was defined.  */
static const yytype_uint16 yyrline[] =
{
       0,    90,    90,    93,    94,    97,    98,   101,   102,   103,
     104,   105,   106,   107,   108,   109,   110,   111,   114,   117,
     120,   123,   126,   129,   130,   131,   132,   133,   134,   135,
     136,   137,   138,   139,   140,   141,   142,   143,   144,   145,
     146,   147,   150,   151,   152,   153,   154,   155,   158,   159,
     160,   163,   164,   165,   166,   167,   168,   169,   172,   173,
     176,   177,   180,   181,   185,   188,   189,   192,   195,   196,
     197,   198,   201,   202,   203,   206,   207,   210,   214,   215,
     216,   217,   220,   223,   224,   225,   226,   229,   232,   233,
     236,   237,   240,   241,   244,   247,   250,   251,   254,   255,
     256,   257,   260,   263,   266,   269,   272,   273,   276,   277,
     278,   279,   280,   281,   282,   283,   286,   287,   290,   292,
     293,   296,   297,   298,   299,   300,   301,   302,   303,   304,
     305,   306,   307,   308,   309,   310,   311
};
#endif

#if YYDEBUG || YYERROR_VERBOSE || 0
/* YYTNAME[SYMBOL-NUM] -- String name of the symbol SYMBOL-NUM.
   First, the terminals, then, starting at YYNTOKENS, nonterminals.  */
static const char *const yytname[] =
{
  "$end", "error", "$undefined", "Ident", "UserType", "I8", "I16", "I32",
  "I64", "U8", "U16", "U32", "U64", "Isz", "Usz", "F16", "F32", "F64",
  "C8", "C32", "Bool", "Void", "Eq", "NotEq", "AddEq", "SubEq", "MulEq",
  "DivEq", "GrtrEq", "LesrEq", "Or", "And", "True", "False", "IntLit",
  "FltLit", "StrLit", "Return", "If", "Elif", "Else", "For", "While", "Do",
  "In", "Continue", "Break", "Import", "Match", "Data", "Enum", "Pub",
  "Pri", "Pro", "Raw", "Const", "Ext", "Pathogen", "Where", "Infect",
  "Cleanse", "Ct", "Newline", "Indent", "Unindent", "LOW", "','", "'<'",
  "'>'", "'+'", "'-'", "'*'", "'/'", "'%'", "'^'", "'.'", "'('", "'['",
  "']'", "')'", "'|'", "'='", "':'", "$accept", "top_level_stmt_list",
  "stmt_list", "maybe_newline", "stmt", "ident", "usertype", "intlit",
  "fltlit", "strlit", "lit_type", "type", "type_expr", "modifier",
  "modifier_list", "decl_prepend", "var_decl", "var_assign",
  "usertype_list", "generic", "data_decl", "type_decl", "type_decl_list",
  "type_decl_block", "val_init_list", "enum_block", "enum_decl", "block",
  "params", "maybe_params", "fn_decl", "fn_call", "ret_stmt", "elif_list",
  "maybe_elif_list", "if_stmt", "while_loop", "do_while_loop", "for_loop",
  "var", "val", "maybe_expr", "expr", "expr_list", "expr_p", YY_NULLPTR
};
#endif

# ifdef YYPRINT
/* YYTOKNUM[NUM] -- (External) token number corresponding to the
   (internal) symbol number NUM (which must be that of a token).  */
static const yytype_uint16 yytoknum[] =
{
       0,   256,   257,   258,   259,   260,   261,   262,   263,   264,
     265,   266,   267,   268,   269,   270,   271,   272,   273,   274,
     275,   276,   277,   278,   279,   280,   281,   282,   283,   284,
     285,   286,   287,   288,   289,   290,   291,   292,   293,   294,
     295,   296,   297,   298,   299,   300,   301,   302,   303,   304,
     305,   306,   307,   308,   309,   310,   311,   312,   313,   314,
     315,   316,   317,   318,   319,   320,    44,    60,    62,    43,
      45,    42,    47,    37,    94,    46,    40,    91,    93,    41,
     124,    61,    58
};
# endif

#define YYPACT_NINF -118

#define yypact_value_is_default(Yystate) \
  (!!((Yystate) == (-118)))

#define YYTABLE_NINF -108

#define yytable_value_is_error(Yytable_value) \
  0

  /* YYPACT[STATE-NUM] -- Index in YYTABLE of the portion describing
     STATE-NUM.  */
static const yytype_int16 yypact[] =
{
     -34,  -118,    43,   249,  -118,  -118,  -118,  -118,  -118,  -118,
    -118,  -118,  -118,  -118,  -118,  -118,  -118,  -118,  -118,  -118,
    -118,  -118,  -118,  -118,    26,    26,   424,    26,   -30,    59,
       7,  -118,  -118,  -118,  -118,  -118,  -118,  -118,   443,   -34,
    -118,   -12,  -118,  -118,   -46,   -25,  -118,   323,   104,  -118,
    -118,  -118,  -118,  -118,  -118,  -118,  -118,  -118,  -118,  -118,
      30,  -118,  -118,  -118,  -118,  -118,    26,   -55,  -118,  -118,
    -118,  -118,  -118,  -118,  -118,    48,   175,   -30,  -118,   424,
     104,    85,   -30,   249,    91,     8,    59,    81,  -118,   -42,
     249,    26,    26,  -118,   218,    26,   443,   443,    59,     7,
     -25,  -118,    49,    26,    70,    26,    26,    26,    26,    26,
      26,    26,    26,    26,    26,    26,    26,    26,    26,    26,
      26,    61,    64,    26,  -118,    -8,    26,   350,    59,    84,
    -118,    69,     4,  -118,  -118,  -118,    72,  -118,    74,  -118,
     -40,    76,   -46,   -46,     8,    81,  -118,    26,    26,   443,
    -118,  -118,   175,   -22,   -22,   -22,   -22,   460,   470,   -22,
     -22,    68,    68,    35,    35,    35,    35,    80,    26,   -30,
      87,  -118,   -30,  -118,   249,  -118,    33,    66,  -118,    17,
    -118,  -118,    16,  -118,    26,    59,  -118,  -118,  -118,  -118,
    -118,    84,  -118,  -118,    77,  -118,    33,    92,   -30,   -30,
    -118,    26,   -30,  -118,  -118,   350,  -118,    59,  -118,  -118,
      78,  -118,    75,  -118,   443,  -118,  -118,   -30,  -118,  -118,
    -118,    26,   443,    33,  -118,  -118,   -30,  -118,  -118
};

  /* YYDEFACT[STATE-NUM] -- Default reduction number in state STATE-NUM.
     Performed when YYTABLE does not specify something else to do.  Zero
     means the default is an error.  */
static const yytype_uint8 yydefact[] =
{
       6,     5,     0,     0,     1,    18,    19,    23,    24,    25,
      26,    27,    28,    29,    30,    31,    32,    33,    34,    35,
      36,    37,    38,    39,     0,     0,     0,     0,     0,     0,
       0,    51,    52,    53,    54,    55,    56,    57,     0,     6,
       4,    41,    40,    47,    50,    61,    59,     0,     0,     7,
       8,    11,    17,     9,    10,    12,    16,    13,    14,    15,
       0,   114,   115,    20,    21,    22,     0,   107,   111,   112,
     113,   108,   110,   136,    95,   118,   120,     0,    41,     0,
       0,     0,     0,     0,     0,     0,     0,     0,    86,     0,
       2,   117,     0,    42,     0,   117,     0,     0,     0,     0,
      60,    58,    63,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,   101,    63,     0,   103,     6,     0,     0,     0,     0,
      70,    81,     0,    84,    44,     3,     0,   116,     0,    46,
       0,     0,    48,    49,     0,     0,    85,   117,     0,    91,
      64,   109,   119,   130,   131,   132,   133,   134,   135,   126,
     127,   121,   122,   123,   124,   125,   128,   129,     0,     0,
      99,   102,     0,    87,     0,   104,    73,     0,    76,     0,
      74,    66,     0,    71,     0,     0,    82,    94,   106,    45,
      43,     0,    68,    83,     0,    62,     0,    90,     0,     0,
     100,     0,     0,   105,    72,     0,    77,     0,    67,    80,
      78,    69,     0,    89,     0,    92,    97,     0,    98,    75,
      65,     0,    91,     0,    96,    79,     0,    88,    93
};

  /* YYPGOTO[NTERM-NUM].  */
static const yytype_int16 yypgoto[] =
{
    -118,  -118,    82,   -33,   -76,    -3,   -13,  -118,  -118,  -118,
    -118,    40,   -37,   -45,   -23,   134,   135,  -118,  -118,    19,
    -118,   -39,  -118,  -117,  -118,   -67,  -108,   -64,  -118,   -53,
    -118,     5,  -118,  -118,  -118,  -118,  -118,  -118,  -118,     6,
    -118,   -80,   -20,  -118,   202
};

  /* YYDEFGOTO[NTERM-NUM].  */
static const yytype_int16 yydefgoto[] =
{
      -1,     2,    39,     3,    40,    67,    42,    68,    69,    70,
      43,    44,    45,    46,    47,    48,    49,    50,   182,   129,
      51,   178,   179,   130,   132,    88,    52,    84,   197,   198,
      53,    71,    55,   170,   171,    56,    57,    58,    59,    72,
      73,   136,   137,    75,    76
};

  /* YYTABLE[YYPACT[STATE-NUM]] -- What to do in state STATE-NUM.  If
     positive, shift that token.  If negative, reduce the rule whose
     number is the opposite.  If YYTABLE_NINF, syntax error.  */
static const yytype_int16 yytable[] =
{
      41,    89,   101,    79,    74,    77,    90,    82,    54,    60,
     100,     6,   183,   121,   135,   141,    85,    87,   124,   180,
     133,    91,    92,    78,    96,    93,    96,   192,     1,     5,
      94,    95,   146,    83,   101,    78,     5,   134,    97,   189,
      97,    96,   100,     4,    78,   102,   104,   114,   115,   116,
     117,   118,   119,   120,     1,    97,   173,   140,    61,    62,
      63,    64,    65,     6,    91,    92,   185,   194,   186,  -107,
      86,   127,   138,   131,   211,   128,    78,   122,   193,   205,
      41,   206,   207,   150,   208,   144,   145,    41,    54,    60,
     176,    78,   174,    78,    78,    54,    60,   180,   135,    96,
     168,   169,    66,   172,   177,   200,   175,     5,   203,   119,
     120,   103,   196,    97,   105,   181,    99,    31,    32,    33,
      34,    35,    36,    37,    78,   147,   201,   202,   195,   123,
     148,   149,   101,   126,   215,   216,   142,   143,   218,   116,
     117,   118,   119,   120,    86,   148,    78,   127,   199,   151,
     184,   187,   188,   224,   190,   120,   212,   222,   214,   221,
      80,    81,   228,   191,   209,   125,   219,     0,   176,   226,
       0,    41,   210,   204,     0,     0,     0,   223,     0,    54,
      60,   217,   177,     0,     0,   196,     0,     0,     0,     0,
       0,     0,     0,   213,   220,     0,     0,   106,   107,     0,
       0,   225,    78,   108,   109,   110,   111,     0,     0,     0,
       0,    78,     0,     0,     0,     0,     0,     0,     0,    78,
     227,     5,     6,     7,     8,     9,    10,    11,    12,    13,
      14,    15,    16,    17,    18,    19,    20,    21,    22,    23,
       0,     0,   112,   113,   114,   115,   116,   117,   118,   119,
     120,     0,     5,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    21,    22,
      23,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,    24,    25,     0,     0,
      26,    27,    28,     0,    38,     0,     0,   139,    29,    30,
      31,    32,    33,    34,    35,    36,    37,   152,   153,   154,
     155,   156,   157,   158,   159,   160,   161,   162,   163,   164,
     165,   166,   167,     0,     0,    38,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    22,    23,     0,     0,     0,     0,     0,
       0,     0,     0,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
      22,    23,    98,    99,    31,    32,    33,    34,    35,    36,
      37,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,    38,
      30,    31,    32,    33,    34,    35,    36,    37,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,    38,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    22,    23,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    22,    23,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,    31,    32,    33,    34,    35,
      36,    37,   106,   107,     0,     0,     0,     0,   108,   109,
       0,   111,   106,   107,     0,     0,     0,     0,   108,   109,
      38,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,    38,
       0,     0,     0,     0,     0,     0,     0,   112,   113,   114,
     115,   116,   117,   118,   119,   120,     0,   112,   113,   114,
     115,   116,   117,   118,   119,   120
};

static const yytype_int16 yycheck[] =
{
       3,    38,    47,    26,    24,    25,    39,    27,     3,     3,
      47,     4,   129,    77,    90,    95,    29,    30,    82,   127,
      87,    76,    77,    26,    66,    71,    66,   144,    62,     3,
      76,    77,    99,    63,    79,    38,     3,    79,    80,    79,
      80,    66,    79,     0,    47,    48,    66,    69,    70,    71,
      72,    73,    74,    75,    62,    80,    64,    94,    32,    33,
      34,    35,    36,     4,    76,    77,    62,   147,    64,    81,
      63,    63,    92,    86,   191,    67,    79,    80,   145,    62,
      83,    64,    66,   103,    68,    98,    99,    90,    83,    83,
     127,    94,   125,    96,    97,    90,    90,   205,   174,    66,
      39,    40,    76,   123,   127,   169,   126,     3,   172,    74,
      75,    81,   149,    80,    66,   128,    50,    51,    52,    53,
      54,    55,    56,    57,   127,    76,    39,    40,   148,    44,
      81,    82,   177,    42,   198,   199,    96,    97,   202,    71,
      72,    73,    74,    75,    63,    81,   149,    63,   168,    79,
      81,    79,    78,   217,    78,    75,    79,    82,    66,    81,
      26,    26,   226,   144,   184,    83,   205,    -1,   205,   222,
      -1,   174,   185,   176,    -1,    -1,    -1,   214,    -1,   174,
     174,   201,   205,    -1,    -1,   222,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,   196,   207,    -1,    -1,    22,    23,    -1,
      -1,   221,   205,    28,    29,    30,    31,    -1,    -1,    -1,
      -1,   214,    -1,    -1,    -1,    -1,    -1,    -1,    -1,   222,
     223,     3,     4,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
      -1,    -1,    67,    68,    69,    70,    71,    72,    73,    74,
      75,    -1,     3,     4,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    37,    38,    -1,    -1,
      41,    42,    43,    -1,    76,    -1,    -1,    79,    49,    50,
      51,    52,    53,    54,    55,    56,    57,   105,   106,   107,
     108,   109,   110,   111,   112,   113,   114,   115,   116,   117,
     118,   119,   120,    -1,    -1,    76,     3,     4,     5,     6,
       7,     8,     9,    10,    11,    12,    13,    14,    15,    16,
      17,    18,    19,    20,    21,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,     3,     4,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    49,    50,    51,    52,    53,    54,    55,    56,
      57,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    76,
      50,    51,    52,    53,    54,    55,    56,    57,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    76,     3,     4,     5,
       6,     7,     8,     9,    10,    11,    12,    13,    14,    15,
      16,    17,    18,    19,    20,    21,     3,     4,     5,     6,
       7,     8,     9,    10,    11,    12,    13,    14,    15,    16,
      17,    18,    19,    20,    21,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    51,    52,    53,    54,    55,
      56,    57,    22,    23,    -1,    -1,    -1,    -1,    28,    29,
      -1,    31,    22,    23,    -1,    -1,    -1,    -1,    28,    29,
      76,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    76,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    67,    68,    69,
      70,    71,    72,    73,    74,    75,    -1,    67,    68,    69,
      70,    71,    72,    73,    74,    75
};

  /* YYSTOS[STATE-NUM] -- The (internal number of the) accessing
     symbol of state STATE-NUM.  */
static const yytype_uint8 yystos[] =
{
       0,    62,    84,    86,     0,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    37,    38,    41,    42,    43,    49,
      50,    51,    52,    53,    54,    55,    56,    57,    76,    85,
      87,    88,    89,    93,    94,    95,    96,    97,    98,    99,
     100,   103,   109,   113,   114,   115,   118,   119,   120,   121,
     122,    32,    33,    34,    35,    36,    76,    88,    90,    91,
      92,   114,   122,   123,   125,   126,   127,   125,    88,    97,
      98,    99,   125,    63,   110,    89,    63,    89,   108,    95,
      86,    76,    77,    71,    76,    77,    66,    80,    49,    50,
      95,    96,    88,    81,   125,    66,    22,    23,    28,    29,
      30,    31,    67,    68,    69,    70,    71,    72,    73,    74,
      75,   110,    88,    44,   110,    85,    42,    63,    67,   102,
     106,    89,   107,   108,    79,    87,   124,   125,   125,    79,
      95,   124,    94,    94,    89,    89,   108,    76,    81,    82,
     125,    79,   127,   127,   127,   127,   127,   127,   127,   127,
     127,   127,   127,   127,   127,   127,   127,   127,    39,    40,
     116,   117,   125,    64,    86,   125,    95,    97,   104,   105,
     109,    89,   101,   106,    81,    62,    64,    79,    78,    79,
      78,   102,   106,   108,   124,   125,    95,   111,   112,   125,
     110,    39,    40,   110,    88,    62,    64,    66,    68,   125,
      89,   106,    79,    88,    66,   110,   110,   125,   110,   104,
      89,    81,    82,    95,   110,   125,   112,    88,   110
};

  /* YYR1[YYN] -- Symbol number of symbol that rule YYN derives.  */
static const yytype_uint8 yyr1[] =
{
       0,    83,    84,    85,    85,    86,    86,    87,    87,    87,
      87,    87,    87,    87,    87,    87,    87,    87,    88,    89,
      90,    91,    92,    93,    93,    93,    93,    93,    93,    93,
      93,    93,    93,    93,    93,    93,    93,    93,    93,    93,
      93,    93,    94,    94,    94,    94,    94,    94,    95,    95,
      95,    96,    96,    96,    96,    96,    96,    96,    97,    97,
      98,    98,    99,    99,   100,   101,   101,   102,   103,   103,
     103,   103,   104,   104,   104,   105,   105,   106,   107,   107,
     107,   107,   108,   109,   109,   109,   109,   110,   111,   111,
     112,   112,   113,   113,   114,   115,   116,   116,   117,   117,
     117,   117,   118,   119,   120,   121,   122,   122,   123,   123,
     123,   123,   123,   123,   123,   123,   124,   124,   125,   126,
     126,   127,   127,   127,   127,   127,   127,   127,   127,   127,
     127,   127,   127,   127,   127,   127,   127
};

  /* YYR2[YYN] -- Number of symbols on the right hand side of rule YYN.  */
static const yytype_uint8 yyr2[] =
{
       0,     2,     3,     3,     1,     1,     0,     1,     1,     1,
       1,     1,     1,     1,     1,     1,     1,     1,     1,     1,
       1,     1,     1,     1,     1,     1,     1,     1,     1,     1,
       1,     1,     1,     1,     1,     1,     1,     1,     1,     1,
       1,     1,     2,     4,     3,     4,     3,     1,     3,     3,
       1,     1,     1,     1,     1,     1,     1,     1,     2,     1,
       2,     1,     4,     2,     3,     3,     1,     3,     4,     5,
       3,     4,     2,     1,     1,     3,     1,     3,     3,     5,
       3,     1,     3,     4,     3,     3,     2,     3,     4,     2,
       1,     0,     5,     8,     4,     2,     4,     3,     3,     1,
       2,     0,     4,     3,     4,     5,     4,     1,     1,     3,
       1,     1,     1,     1,     1,     1,     1,     0,     1,     3,
       1,     3,     3,     3,     3,     3,     3,     3,     3,     3,
       3,     3,     3,     3,     3,     3,     1
};


#define yyerrok         (yyerrstatus = 0)
#define yyclearin       (yychar = YYEMPTY)
#define YYEMPTY         (-2)
#define YYEOF           0

#define YYACCEPT        goto yyacceptlab
#define YYABORT         goto yyabortlab
#define YYERROR         goto yyerrorlab


#define YYRECOVERING()  (!!yyerrstatus)

#define YYBACKUP(Token, Value)                                  \
do                                                              \
  if (yychar == YYEMPTY)                                        \
    {                                                           \
      yychar = (Token);                                         \
      yylval = (Value);                                         \
      YYPOPSTACK (yylen);                                       \
      yystate = *yyssp;                                         \
      goto yybackup;                                            \
    }                                                           \
  else                                                          \
    {                                                           \
      yyerror (YY_("syntax error: cannot back up")); \
      YYERROR;                                                  \
    }                                                           \
while (0)

/* Error token number */
#define YYTERROR        1
#define YYERRCODE       256



/* Enable debugging if requested.  */
#if YYDEBUG

# ifndef YYFPRINTF
#  include <stdio.h> /* INFRINGES ON USER NAME SPACE */
#  define YYFPRINTF fprintf
# endif

# define YYDPRINTF(Args)                        \
do {                                            \
  if (yydebug)                                  \
    YYFPRINTF Args;                             \
} while (0)

/* This macro is provided for backward compatibility. */
#ifndef YY_LOCATION_PRINT
# define YY_LOCATION_PRINT(File, Loc) ((void) 0)
#endif


# define YY_SYMBOL_PRINT(Title, Type, Value, Location)                    \
do {                                                                      \
  if (yydebug)                                                            \
    {                                                                     \
      YYFPRINTF (stderr, "%s ", Title);                                   \
      yy_symbol_print (stderr,                                            \
                  Type, Value); \
      YYFPRINTF (stderr, "\n");                                           \
    }                                                                     \
} while (0)


/*----------------------------------------.
| Print this symbol's value on YYOUTPUT.  |
`----------------------------------------*/

static void
yy_symbol_value_print (FILE *yyoutput, int yytype, YYSTYPE const * const yyvaluep)
{
  FILE *yyo = yyoutput;
  YYUSE (yyo);
  if (!yyvaluep)
    return;
# ifdef YYPRINT
  if (yytype < YYNTOKENS)
    YYPRINT (yyoutput, yytoknum[yytype], *yyvaluep);
# endif
  YYUSE (yytype);
}


/*--------------------------------.
| Print this symbol on YYOUTPUT.  |
`--------------------------------*/

static void
yy_symbol_print (FILE *yyoutput, int yytype, YYSTYPE const * const yyvaluep)
{
  YYFPRINTF (yyoutput, "%s %s (",
             yytype < YYNTOKENS ? "token" : "nterm", yytname[yytype]);

  yy_symbol_value_print (yyoutput, yytype, yyvaluep);
  YYFPRINTF (yyoutput, ")");
}

/*------------------------------------------------------------------.
| yy_stack_print -- Print the state stack from its BOTTOM up to its |
| TOP (included).                                                   |
`------------------------------------------------------------------*/

static void
yy_stack_print (yytype_int16 *yybottom, yytype_int16 *yytop)
{
  YYFPRINTF (stderr, "Stack now");
  for (; yybottom <= yytop; yybottom++)
    {
      int yybot = *yybottom;
      YYFPRINTF (stderr, " %d", yybot);
    }
  YYFPRINTF (stderr, "\n");
}

# define YY_STACK_PRINT(Bottom, Top)                            \
do {                                                            \
  if (yydebug)                                                  \
    yy_stack_print ((Bottom), (Top));                           \
} while (0)


/*------------------------------------------------.
| Report that the YYRULE is going to be reduced.  |
`------------------------------------------------*/

static void
yy_reduce_print (yytype_int16 *yyssp, YYSTYPE *yyvsp, int yyrule)
{
  unsigned long int yylno = yyrline[yyrule];
  int yynrhs = yyr2[yyrule];
  int yyi;
  YYFPRINTF (stderr, "Reducing stack by rule %d (line %lu):\n",
             yyrule - 1, yylno);
  /* The symbols being reduced.  */
  for (yyi = 0; yyi < yynrhs; yyi++)
    {
      YYFPRINTF (stderr, "   $%d = ", yyi + 1);
      yy_symbol_print (stderr,
                       yystos[yyssp[yyi + 1 - yynrhs]],
                       &(yyvsp[(yyi + 1) - (yynrhs)])
                                              );
      YYFPRINTF (stderr, "\n");
    }
}

# define YY_REDUCE_PRINT(Rule)          \
do {                                    \
  if (yydebug)                          \
    yy_reduce_print (yyssp, yyvsp, Rule); \
} while (0)

/* Nonzero means print parse trace.  It is left uninitialized so that
   multiple parsers can coexist.  */
int yydebug;
#else /* !YYDEBUG */
# define YYDPRINTF(Args)
# define YY_SYMBOL_PRINT(Title, Type, Value, Location)
# define YY_STACK_PRINT(Bottom, Top)
# define YY_REDUCE_PRINT(Rule)
#endif /* !YYDEBUG */


/* YYINITDEPTH -- initial size of the parser's stacks.  */
#ifndef YYINITDEPTH
# define YYINITDEPTH 200
#endif

/* YYMAXDEPTH -- maximum size the stacks can grow to (effective only
   if the built-in stack extension method is used).

   Do not make this value too large; the results are undefined if
   YYSTACK_ALLOC_MAXIMUM < YYSTACK_BYTES (YYMAXDEPTH)
   evaluated with infinite-precision integer arithmetic.  */

#ifndef YYMAXDEPTH
# define YYMAXDEPTH 10000
#endif


#if YYERROR_VERBOSE

# ifndef yystrlen
#  if defined __GLIBC__ && defined _STRING_H
#   define yystrlen strlen
#  else
/* Return the length of YYSTR.  */
static YYSIZE_T
yystrlen (const char *yystr)
{
  YYSIZE_T yylen;
  for (yylen = 0; yystr[yylen]; yylen++)
    continue;
  return yylen;
}
#  endif
# endif

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
static YYSIZE_T
yytnamerr (char *yyres, const char *yystr)
{
  if (*yystr == '"')
    {
      YYSIZE_T yyn = 0;
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
    return yystrlen (yystr);

  return yystpcpy (yyres, yystr) - yyres;
}
# endif

/* Copy into *YYMSG, which is of size *YYMSG_ALLOC, an error message
   about the unexpected token YYTOKEN for the state stack whose top is
   YYSSP.

   Return 0 if *YYMSG was successfully written.  Return 1 if *YYMSG is
   not large enough to hold the message.  In that case, also set
   *YYMSG_ALLOC to the required number of bytes.  Return 2 if the
   required number of bytes is too large to store.  */
static int
yysyntax_error (YYSIZE_T *yymsg_alloc, char **yymsg,
                yytype_int16 *yyssp, int yytoken)
{
  YYSIZE_T yysize0 = yytnamerr (YY_NULLPTR, yytname[yytoken]);
  YYSIZE_T yysize = yysize0;
  enum { YYERROR_VERBOSE_ARGS_MAXIMUM = 5 };
  /* Internationalized format string. */
  const char *yyformat = YY_NULLPTR;
  /* Arguments of yyformat. */
  char const *yyarg[YYERROR_VERBOSE_ARGS_MAXIMUM];
  /* Number of reported tokens (one for the "unexpected", one per
     "expected"). */
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
      int yyn = yypact[*yyssp];
      yyarg[yycount++] = yytname[yytoken];
      if (!yypact_value_is_default (yyn))
        {
          /* Start YYX at -YYN if negative to avoid negative indexes in
             YYCHECK.  In other words, skip the first -YYN actions for
             this state because they are default actions.  */
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
                yyarg[yycount++] = yytname[yyx];
                {
                  YYSIZE_T yysize1 = yysize + yytnamerr (YY_NULLPTR, yytname[yyx]);
                  if (! (yysize <= yysize1
                         && yysize1 <= YYSTACK_ALLOC_MAXIMUM))
                    return 2;
                  yysize = yysize1;
                }
              }
        }
    }

  switch (yycount)
    {
# define YYCASE_(N, S)                      \
      case N:                               \
        yyformat = S;                       \
      break
      YYCASE_(0, YY_("syntax error"));
      YYCASE_(1, YY_("syntax error, unexpected %s"));
      YYCASE_(2, YY_("syntax error, unexpected %s, expecting %s"));
      YYCASE_(3, YY_("syntax error, unexpected %s, expecting %s or %s"));
      YYCASE_(4, YY_("syntax error, unexpected %s, expecting %s or %s or %s"));
      YYCASE_(5, YY_("syntax error, unexpected %s, expecting %s or %s or %s or %s"));
# undef YYCASE_
    }

  {
    YYSIZE_T yysize1 = yysize + yystrlen (yyformat);
    if (! (yysize <= yysize1 && yysize1 <= YYSTACK_ALLOC_MAXIMUM))
      return 2;
    yysize = yysize1;
  }

  if (*yymsg_alloc < yysize)
    {
      *yymsg_alloc = 2 * yysize;
      if (! (yysize <= *yymsg_alloc
             && *yymsg_alloc <= YYSTACK_ALLOC_MAXIMUM))
        *yymsg_alloc = YYSTACK_ALLOC_MAXIMUM;
      return 1;
    }

  /* Avoid sprintf, as that infringes on the user's name space.
     Don't have undefined behavior even if the translation
     produced a string with the wrong number of "%s"s.  */
  {
    char *yyp = *yymsg;
    int yyi = 0;
    while ((*yyp = *yyformat) != '\0')
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
  return 0;
}
#endif /* YYERROR_VERBOSE */

/*-----------------------------------------------.
| Release the memory associated to this symbol.  |
`-----------------------------------------------*/

static void
yydestruct (const char *yymsg, int yytype, YYSTYPE *yyvaluep)
{
  YYUSE (yyvaluep);
  if (!yymsg)
    yymsg = "Deleting";
  YY_SYMBOL_PRINT (yymsg, yytype, yyvaluep, yylocationp);

  YY_IGNORE_MAYBE_UNINITIALIZED_BEGIN
  YYUSE (yytype);
  YY_IGNORE_MAYBE_UNINITIALIZED_END
}




/* The lookahead symbol.  */
int yychar;

/* The semantic value of the lookahead symbol.  */
YYSTYPE yylval;
/* Number of syntax errors so far.  */
int yynerrs;


/*----------.
| yyparse.  |
`----------*/

int
yyparse (void)
{
    int yystate;
    /* Number of tokens to shift before error messages enabled.  */
    int yyerrstatus;

    /* The stacks and their tools:
       'yyss': related to states.
       'yyvs': related to semantic values.

       Refer to the stacks through separate pointers, to allow yyoverflow
       to reallocate them elsewhere.  */

    /* The state stack.  */
    yytype_int16 yyssa[YYINITDEPTH];
    yytype_int16 *yyss;
    yytype_int16 *yyssp;

    /* The semantic value stack.  */
    YYSTYPE yyvsa[YYINITDEPTH];
    YYSTYPE *yyvs;
    YYSTYPE *yyvsp;

    YYSIZE_T yystacksize;

  int yyn;
  int yyresult;
  /* Lookahead token as an internal (translated) token number.  */
  int yytoken = 0;
  /* The variables used to return semantic value and location from the
     action routines.  */
  YYSTYPE yyval;

#if YYERROR_VERBOSE
  /* Buffer for error messages, and its allocated size.  */
  char yymsgbuf[128];
  char *yymsg = yymsgbuf;
  YYSIZE_T yymsg_alloc = sizeof yymsgbuf;
#endif

#define YYPOPSTACK(N)   (yyvsp -= (N), yyssp -= (N))

  /* The number of symbols on the RHS of the reduced rule.
     Keep to zero when no symbol should be popped.  */
  int yylen = 0;

  yyssp = yyss = yyssa;
  yyvsp = yyvs = yyvsa;
  yystacksize = YYINITDEPTH;

  YYDPRINTF ((stderr, "Starting parse\n"));

  yystate = 0;
  yyerrstatus = 0;
  yynerrs = 0;
  yychar = YYEMPTY; /* Cause a token to be read.  */
  goto yysetstate;

/*------------------------------------------------------------.
| yynewstate -- Push a new state, which is found in yystate.  |
`------------------------------------------------------------*/
 yynewstate:
  /* In all cases, when you get here, the value and location stacks
     have just been pushed.  So pushing a state here evens the stacks.  */
  yyssp++;

 yysetstate:
  *yyssp = yystate;

  if (yyss + yystacksize - 1 <= yyssp)
    {
      /* Get the current used size of the three stacks, in elements.  */
      YYSIZE_T yysize = yyssp - yyss + 1;

#ifdef yyoverflow
      {
        /* Give user a chance to reallocate the stack.  Use copies of
           these so that the &'s don't force the real ones into
           memory.  */
        YYSTYPE *yyvs1 = yyvs;
        yytype_int16 *yyss1 = yyss;

        /* Each stack pointer address is followed by the size of the
           data in use in that stack, in bytes.  This used to be a
           conditional around just the two extra args, but that might
           be undefined if yyoverflow is a macro.  */
        yyoverflow (YY_("memory exhausted"),
                    &yyss1, yysize * sizeof (*yyssp),
                    &yyvs1, yysize * sizeof (*yyvsp),
                    &yystacksize);

        yyss = yyss1;
        yyvs = yyvs1;
      }
#else /* no yyoverflow */
# ifndef YYSTACK_RELOCATE
      goto yyexhaustedlab;
# else
      /* Extend the stack our own way.  */
      if (YYMAXDEPTH <= yystacksize)
        goto yyexhaustedlab;
      yystacksize *= 2;
      if (YYMAXDEPTH < yystacksize)
        yystacksize = YYMAXDEPTH;

      {
        yytype_int16 *yyss1 = yyss;
        union yyalloc *yyptr =
          (union yyalloc *) YYSTACK_ALLOC (YYSTACK_BYTES (yystacksize));
        if (! yyptr)
          goto yyexhaustedlab;
        YYSTACK_RELOCATE (yyss_alloc, yyss);
        YYSTACK_RELOCATE (yyvs_alloc, yyvs);
#  undef YYSTACK_RELOCATE
        if (yyss1 != yyssa)
          YYSTACK_FREE (yyss1);
      }
# endif
#endif /* no yyoverflow */

      yyssp = yyss + yysize - 1;
      yyvsp = yyvs + yysize - 1;

      YYDPRINTF ((stderr, "Stack size increased to %lu\n",
                  (unsigned long int) yystacksize));

      if (yyss + yystacksize - 1 <= yyssp)
        YYABORT;
    }

  YYDPRINTF ((stderr, "Entering state %d\n", yystate));

  if (yystate == YYFINAL)
    YYACCEPT;

  goto yybackup;

/*-----------.
| yybackup.  |
`-----------*/
yybackup:

  /* Do appropriate processing given the current state.  Read a
     lookahead token if we need one and don't already have one.  */

  /* First try to decide what to do without reference to lookahead token.  */
  yyn = yypact[yystate];
  if (yypact_value_is_default (yyn))
    goto yydefault;

  /* Not known => get a lookahead token if don't already have one.  */

  /* YYCHAR is either YYEMPTY or YYEOF or a valid lookahead symbol.  */
  if (yychar == YYEMPTY)
    {
      YYDPRINTF ((stderr, "Reading a token: "));
      yychar = yylex ();
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

  /* If the proper action on seeing token YYTOKEN is to reduce or to
     detect an error, take that action.  */
  yyn += yytoken;
  if (yyn < 0 || YYLAST < yyn || yycheck[yyn] != yytoken)
    goto yydefault;
  yyn = yytable[yyn];
  if (yyn <= 0)
    {
      if (yytable_value_is_error (yyn))
        goto yyerrlab;
      yyn = -yyn;
      goto yyreduce;
    }

  /* Count tokens shifted since error; after three, turn off error
     status.  */
  if (yyerrstatus)
    yyerrstatus--;

  /* Shift the lookahead token.  */
  YY_SYMBOL_PRINT ("Shifting", yytoken, &yylval, &yylloc);

  /* Discard the shifted token.  */
  yychar = YYEMPTY;

  yystate = yyn;
  YY_IGNORE_MAYBE_UNINITIALIZED_BEGIN
  *++yyvsp = yylval;
  YY_IGNORE_MAYBE_UNINITIALIZED_END

  goto yynewstate;


/*-----------------------------------------------------------.
| yydefault -- do the default action for the current state.  |
`-----------------------------------------------------------*/
yydefault:
  yyn = yydefact[yystate];
  if (yyn == 0)
    goto yyerrlab;
  goto yyreduce;


/*-----------------------------.
| yyreduce -- Do a reduction.  |
`-----------------------------*/
yyreduce:
  /* yyn is the number of a rule to reduce with.  */
  yylen = yyr2[yyn];

  /* If YYLEN is nonzero, implement the default value of the action:
     '$$ = $1'.

     Otherwise, the following line sets YYVAL to garbage.
     This behavior is undocumented and Bison
     users should not rely upon it.  Assigning to YYVAL
     unconditionally makes the parser a bit smaller, and it avoids a
     GCC warning that YYVAL may be used uninitialized.  */
  yyval = yyvsp[1-yylen];


  YY_REDUCE_PRINT (yyn);
  switch (yyn)
    {
        case 3:
#line 93 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = setNext((yyvsp[-2]), (yyvsp[0]));}
#line 1579 "src/parser.c" /* yacc.c:1646  */
    break;

  case 4:
#line 94 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = setRoot((yyvsp[0]));}
#line 1585 "src/parser.c" /* yacc.c:1646  */
    break;

  case 7:
#line 101 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1591 "src/parser.c" /* yacc.c:1646  */
    break;

  case 8:
#line 102 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1597 "src/parser.c" /* yacc.c:1646  */
    break;

  case 9:
#line 103 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1603 "src/parser.c" /* yacc.c:1646  */
    break;

  case 10:
#line 104 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1609 "src/parser.c" /* yacc.c:1646  */
    break;

  case 11:
#line 105 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1615 "src/parser.c" /* yacc.c:1646  */
    break;

  case 12:
#line 106 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1621 "src/parser.c" /* yacc.c:1646  */
    break;

  case 13:
#line 107 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1627 "src/parser.c" /* yacc.c:1646  */
    break;

  case 14:
#line 108 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1633 "src/parser.c" /* yacc.c:1646  */
    break;

  case 15:
#line 109 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1639 "src/parser.c" /* yacc.c:1646  */
    break;

  case 16:
#line 110 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1645 "src/parser.c" /* yacc.c:1646  */
    break;

  case 17:
#line 111 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1651 "src/parser.c" /* yacc.c:1646  */
    break;

  case 18:
#line 114 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (Node*)lextxt;}
#line 1657 "src/parser.c" /* yacc.c:1646  */
    break;

  case 19:
#line 117 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (Node*)lextxt;}
#line 1663 "src/parser.c" /* yacc.c:1646  */
    break;

  case 20:
#line 120 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkIntLitNode(lextxt);}
#line 1669 "src/parser.c" /* yacc.c:1646  */
    break;

  case 21:
#line 123 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkFltLitNode(lextxt);}
#line 1675 "src/parser.c" /* yacc.c:1646  */
    break;

  case 22:
#line 126 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkStrLitNode(lextxt);}
#line 1681 "src/parser.c" /* yacc.c:1646  */
    break;

  case 23:
#line 129 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_I8,  (char*)"");}
#line 1687 "src/parser.c" /* yacc.c:1646  */
    break;

  case 24:
#line 130 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_I16, (char*)"");}
#line 1693 "src/parser.c" /* yacc.c:1646  */
    break;

  case 25:
#line 131 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_I32, (char*)"");}
#line 1699 "src/parser.c" /* yacc.c:1646  */
    break;

  case 26:
#line 132 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_I64, (char*)"");}
#line 1705 "src/parser.c" /* yacc.c:1646  */
    break;

  case 27:
#line 133 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_U8,  (char*)"");}
#line 1711 "src/parser.c" /* yacc.c:1646  */
    break;

  case 28:
#line 134 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_U16, (char*)"");}
#line 1717 "src/parser.c" /* yacc.c:1646  */
    break;

  case 29:
#line 135 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_U32, (char*)"");}
#line 1723 "src/parser.c" /* yacc.c:1646  */
    break;

  case 30:
#line 136 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_U64, (char*)"");}
#line 1729 "src/parser.c" /* yacc.c:1646  */
    break;

  case 31:
#line 137 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_Isz, (char*)"");}
#line 1735 "src/parser.c" /* yacc.c:1646  */
    break;

  case 32:
#line 138 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_Usz, (char*)"");}
#line 1741 "src/parser.c" /* yacc.c:1646  */
    break;

  case 33:
#line 139 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_F16, (char*)"");}
#line 1747 "src/parser.c" /* yacc.c:1646  */
    break;

  case 34:
#line 140 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_F32, (char*)"");}
#line 1753 "src/parser.c" /* yacc.c:1646  */
    break;

  case 35:
#line 141 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_F64, (char*)"");}
#line 1759 "src/parser.c" /* yacc.c:1646  */
    break;

  case 36:
#line 142 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_C8,  (char*)"");}
#line 1765 "src/parser.c" /* yacc.c:1646  */
    break;

  case 37:
#line 143 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_C32, (char*)"");}
#line 1771 "src/parser.c" /* yacc.c:1646  */
    break;

  case 38:
#line 144 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_Bool, (char*)"");}
#line 1777 "src/parser.c" /* yacc.c:1646  */
    break;

  case 39:
#line 145 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_Void, (char*)"");}
#line 1783 "src/parser.c" /* yacc.c:1646  */
    break;

  case 40:
#line 146 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_UserType, (char*)(yyvsp[0]));}
#line 1789 "src/parser.c" /* yacc.c:1646  */
    break;

  case 41:
#line 147 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_Ident, (char*)(yyvsp[0]));}
#line 1795 "src/parser.c" /* yacc.c:1646  */
    break;

  case 47:
#line 155 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1801 "src/parser.c" /* yacc.c:1646  */
    break;

  case 50:
#line 160 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1807 "src/parser.c" /* yacc.c:1646  */
    break;

  case 60:
#line 176 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1813 "src/parser.c" /* yacc.c:1646  */
    break;

  case 61:
#line 177 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1819 "src/parser.c" /* yacc.c:1646  */
    break;

  case 62:
#line 180 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarDeclNode((char*)(yyvsp[-2]), (yyvsp[-3]), (yyvsp[0]));}
#line 1825 "src/parser.c" /* yacc.c:1646  */
    break;

  case 63:
#line 181 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarDeclNode((char*)(yyvsp[0]), (yyvsp[-1]), 0);}
#line 1831 "src/parser.c" /* yacc.c:1646  */
    break;

  case 64:
#line 185 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarAssignNode((yyvsp[-2]), (yyvsp[0]));}
#line 1837 "src/parser.c" /* yacc.c:1646  */
    break;

  case 65:
#line 188 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = setNext((yyvsp[-2]), (yyvsp[0]));}
#line 1843 "src/parser.c" /* yacc.c:1646  */
    break;

  case 66:
#line 189 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = setRoot((yyvsp[0]));}
#line 1849 "src/parser.c" /* yacc.c:1646  */
    break;

  case 67:
#line 192 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = getRoot();}
#line 1855 "src/parser.c" /* yacc.c:1646  */
    break;

  case 68:
#line 195 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkDataDeclNode((char*)(yyvsp[-1]), (yyvsp[0]));}
#line 1861 "src/parser.c" /* yacc.c:1646  */
    break;

  case 69:
#line 196 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkDataDeclNode((char*)(yyvsp[-2]), (yyvsp[0]));}
#line 1867 "src/parser.c" /* yacc.c:1646  */
    break;

  case 70:
#line 197 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkDataDeclNode((char*)(yyvsp[-1]), (yyvsp[0]));}
#line 1873 "src/parser.c" /* yacc.c:1646  */
    break;

  case 71:
#line 198 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkDataDeclNode((char*)(yyvsp[-2]), (yyvsp[0]));}
#line 1879 "src/parser.c" /* yacc.c:1646  */
    break;

  case 83:
#line 223 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 1885 "src/parser.c" /* yacc.c:1646  */
    break;

  case 84:
#line 224 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 1891 "src/parser.c" /* yacc.c:1646  */
    break;

  case 85:
#line 225 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 1897 "src/parser.c" /* yacc.c:1646  */
    break;

  case 86:
#line 226 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 1903 "src/parser.c" /* yacc.c:1646  */
    break;

  case 87:
#line 229 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = getRoot();}
#line 1909 "src/parser.c" /* yacc.c:1646  */
    break;

  case 88:
#line 232 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = setNext((yyvsp[-3]), mkNamedValNode((char*)(yyvsp[0]), (yyvsp[-1])));}
#line 1915 "src/parser.c" /* yacc.c:1646  */
    break;

  case 89:
#line 233 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = setRoot(mkNamedValNode((char*)(yyvsp[0]), (yyvsp[-1])));}
#line 1921 "src/parser.c" /* yacc.c:1646  */
    break;

  case 90:
#line 236 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = getRoot();}
#line 1927 "src/parser.c" /* yacc.c:1646  */
    break;

  case 91:
#line 237 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 1933 "src/parser.c" /* yacc.c:1646  */
    break;

  case 92:
#line 240 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkFuncDeclNode((char*)(yyvsp[-3]), (yyvsp[-4]), (yyvsp[-1]), (yyvsp[0]));}
#line 1939 "src/parser.c" /* yacc.c:1646  */
    break;

  case 93:
#line 241 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkFuncDeclNode((char*)(yyvsp[-6]), (yyvsp[-7]), (yyvsp[-1]), (yyvsp[0]));}
#line 1945 "src/parser.c" /* yacc.c:1646  */
    break;

  case 94:
#line 244 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkFuncCallNode((char*)(yyvsp[-3]), (yyvsp[-1]));}
#line 1951 "src/parser.c" /* yacc.c:1646  */
    break;

  case 95:
#line 247 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkRetNode((yyvsp[0]));}
#line 1957 "src/parser.c" /* yacc.c:1646  */
    break;

  case 96:
#line 250 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = setElse((IfNode*)(yyvsp[-3]), (IfNode*)mkIfNode((yyvsp[-1]), (yyvsp[0])));}
#line 1963 "src/parser.c" /* yacc.c:1646  */
    break;

  case 97:
#line 251 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = setRoot(mkIfNode((yyvsp[-1]), (yyvsp[0])));}
#line 1969 "src/parser.c" /* yacc.c:1646  */
    break;

  case 98:
#line 254 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = setElse((IfNode*)(yyvsp[-2]), (IfNode*)mkIfNode(NULL, (yyvsp[0])));}
#line 1975 "src/parser.c" /* yacc.c:1646  */
    break;

  case 99:
#line 255 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1981 "src/parser.c" /* yacc.c:1646  */
    break;

  case 100:
#line 256 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = setRoot(mkIfNode(NULL, (yyvsp[0])));}
#line 1987 "src/parser.c" /* yacc.c:1646  */
    break;

  case 101:
#line 257 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = setRoot(NULL);}
#line 1993 "src/parser.c" /* yacc.c:1646  */
    break;

  case 102:
#line 260 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkIfNode((yyvsp[-2]), (yyvsp[-1]), (IfNode*)getRoot());}
#line 1999 "src/parser.c" /* yacc.c:1646  */
    break;

  case 103:
#line 263 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2005 "src/parser.c" /* yacc.c:1646  */
    break;

  case 104:
#line 266 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2011 "src/parser.c" /* yacc.c:1646  */
    break;

  case 105:
#line 269 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2017 "src/parser.c" /* yacc.c:1646  */
    break;

  case 106:
#line 272 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarNode((char*)(yyvsp[-3]));}
#line 2023 "src/parser.c" /* yacc.c:1646  */
    break;

  case 107:
#line 273 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarNode((char*)(yyvsp[0]));}
#line 2029 "src/parser.c" /* yacc.c:1646  */
    break;

  case 108:
#line 276 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 2035 "src/parser.c" /* yacc.c:1646  */
    break;

  case 109:
#line 277 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[-1]);}
#line 2041 "src/parser.c" /* yacc.c:1646  */
    break;

  case 110:
#line 278 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 2047 "src/parser.c" /* yacc.c:1646  */
    break;

  case 111:
#line 279 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 2053 "src/parser.c" /* yacc.c:1646  */
    break;

  case 112:
#line 280 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 2059 "src/parser.c" /* yacc.c:1646  */
    break;

  case 113:
#line 281 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 2065 "src/parser.c" /* yacc.c:1646  */
    break;

  case 114:
#line 282 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBoolLitNode(1);}
#line 2071 "src/parser.c" /* yacc.c:1646  */
    break;

  case 115:
#line 283 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBoolLitNode(0);}
#line 2077 "src/parser.c" /* yacc.c:1646  */
    break;

  case 116:
#line 286 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 2083 "src/parser.c" /* yacc.c:1646  */
    break;

  case 117:
#line 287 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2089 "src/parser.c" /* yacc.c:1646  */
    break;

  case 118:
#line 290 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = getRoot();}
#line 2095 "src/parser.c" /* yacc.c:1646  */
    break;

  case 119:
#line 292 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = setNext((yyvsp[-2]), (yyvsp[0]));}
#line 2101 "src/parser.c" /* yacc.c:1646  */
    break;

  case 120:
#line 293 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = setRoot((yyvsp[0]));}
#line 2107 "src/parser.c" /* yacc.c:1646  */
    break;

  case 121:
#line 296 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode('+', (yyvsp[-2]), (yyvsp[0]));}
#line 2113 "src/parser.c" /* yacc.c:1646  */
    break;

  case 122:
#line 297 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode('-', (yyvsp[-2]), (yyvsp[0]));}
#line 2119 "src/parser.c" /* yacc.c:1646  */
    break;

  case 123:
#line 298 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode('*', (yyvsp[-2]), (yyvsp[0]));}
#line 2125 "src/parser.c" /* yacc.c:1646  */
    break;

  case 124:
#line 299 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode('/', (yyvsp[-2]), (yyvsp[0]));}
#line 2131 "src/parser.c" /* yacc.c:1646  */
    break;

  case 125:
#line 300 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode('%', (yyvsp[-2]), (yyvsp[0]));}
#line 2137 "src/parser.c" /* yacc.c:1646  */
    break;

  case 126:
#line 301 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode('<', (yyvsp[-2]), (yyvsp[0]));}
#line 2143 "src/parser.c" /* yacc.c:1646  */
    break;

  case 127:
#line 302 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode('>', (yyvsp[-2]), (yyvsp[0]));}
#line 2149 "src/parser.c" /* yacc.c:1646  */
    break;

  case 128:
#line 303 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode('^', (yyvsp[-2]), (yyvsp[0]));}
#line 2155 "src/parser.c" /* yacc.c:1646  */
    break;

  case 129:
#line 304 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode('.', (yyvsp[-2]), (yyvsp[0]));}
#line 2161 "src/parser.c" /* yacc.c:1646  */
    break;

  case 130:
#line 305 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode(Tok_Eq, (yyvsp[-2]), (yyvsp[0]));}
#line 2167 "src/parser.c" /* yacc.c:1646  */
    break;

  case 131:
#line 306 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode(Tok_NotEq, (yyvsp[-2]), (yyvsp[0]));}
#line 2173 "src/parser.c" /* yacc.c:1646  */
    break;

  case 132:
#line 307 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode(Tok_GrtrEq, (yyvsp[-2]), (yyvsp[0]));}
#line 2179 "src/parser.c" /* yacc.c:1646  */
    break;

  case 133:
#line 308 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode(Tok_LesrEq, (yyvsp[-2]), (yyvsp[0]));}
#line 2185 "src/parser.c" /* yacc.c:1646  */
    break;

  case 134:
#line 309 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode(Tok_Or, (yyvsp[-2]), (yyvsp[0]));}
#line 2191 "src/parser.c" /* yacc.c:1646  */
    break;

  case 135:
#line 310 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode(Tok_And, (yyvsp[-2]), (yyvsp[0]));}
#line 2197 "src/parser.c" /* yacc.c:1646  */
    break;

  case 136:
#line 311 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 2203 "src/parser.c" /* yacc.c:1646  */
    break;


#line 2207 "src/parser.c" /* yacc.c:1646  */
      default: break;
    }
  /* User semantic actions sometimes alter yychar, and that requires
     that yytoken be updated with the new translation.  We take the
     approach of translating immediately before every use of yytoken.
     One alternative is translating here after every semantic action,
     but that translation would be missed if the semantic action invokes
     YYABORT, YYACCEPT, or YYERROR immediately after altering yychar or
     if it invokes YYBACKUP.  In the case of YYABORT or YYACCEPT, an
     incorrect destructor might then be invoked immediately.  In the
     case of YYERROR or YYBACKUP, subsequent parser actions might lead
     to an incorrect destructor call or verbose syntax error message
     before the lookahead is translated.  */
  YY_SYMBOL_PRINT ("-> $$ =", yyr1[yyn], &yyval, &yyloc);

  YYPOPSTACK (yylen);
  yylen = 0;
  YY_STACK_PRINT (yyss, yyssp);

  *++yyvsp = yyval;

  /* Now 'shift' the result of the reduction.  Determine what state
     that goes to, based on the state we popped back to and the rule
     number reduced by.  */

  yyn = yyr1[yyn];

  yystate = yypgoto[yyn - YYNTOKENS] + *yyssp;
  if (0 <= yystate && yystate <= YYLAST && yycheck[yystate] == *yyssp)
    yystate = yytable[yystate];
  else
    yystate = yydefgoto[yyn - YYNTOKENS];

  goto yynewstate;


/*--------------------------------------.
| yyerrlab -- here on detecting error.  |
`--------------------------------------*/
yyerrlab:
  /* Make sure we have latest lookahead translation.  See comments at
     user semantic actions for why this is necessary.  */
  yytoken = yychar == YYEMPTY ? YYEMPTY : YYTRANSLATE (yychar);

  /* If not already recovering from an error, report this error.  */
  if (!yyerrstatus)
    {
      ++yynerrs;
#if ! YYERROR_VERBOSE
      yyerror (YY_("syntax error"));
#else
# define YYSYNTAX_ERROR yysyntax_error (&yymsg_alloc, &yymsg, \
                                        yyssp, yytoken)
      {
        char const *yymsgp = YY_("syntax error");
        int yysyntax_error_status;
        yysyntax_error_status = YYSYNTAX_ERROR;
        if (yysyntax_error_status == 0)
          yymsgp = yymsg;
        else if (yysyntax_error_status == 1)
          {
            if (yymsg != yymsgbuf)
              YYSTACK_FREE (yymsg);
            yymsg = (char *) YYSTACK_ALLOC (yymsg_alloc);
            if (!yymsg)
              {
                yymsg = yymsgbuf;
                yymsg_alloc = sizeof yymsgbuf;
                yysyntax_error_status = 2;
              }
            else
              {
                yysyntax_error_status = YYSYNTAX_ERROR;
                yymsgp = yymsg;
              }
          }
        yyerror (yymsgp);
        if (yysyntax_error_status == 2)
          goto yyexhaustedlab;
      }
# undef YYSYNTAX_ERROR
#endif
    }



  if (yyerrstatus == 3)
    {
      /* If just tried and failed to reuse lookahead token after an
         error, discard it.  */

      if (yychar <= YYEOF)
        {
          /* Return failure if at end of input.  */
          if (yychar == YYEOF)
            YYABORT;
        }
      else
        {
          yydestruct ("Error: discarding",
                      yytoken, &yylval);
          yychar = YYEMPTY;
        }
    }

  /* Else will try to reuse lookahead token after shifting the error
     token.  */
  goto yyerrlab1;


/*---------------------------------------------------.
| yyerrorlab -- error raised explicitly by YYERROR.  |
`---------------------------------------------------*/
yyerrorlab:

  /* Pacify compilers like GCC when the user code never invokes
     YYERROR and the label yyerrorlab therefore never appears in user
     code.  */
  if (/*CONSTCOND*/ 0)
     goto yyerrorlab;

  /* Do not reclaim the symbols of the rule whose action triggered
     this YYERROR.  */
  YYPOPSTACK (yylen);
  yylen = 0;
  YY_STACK_PRINT (yyss, yyssp);
  yystate = *yyssp;
  goto yyerrlab1;


/*-------------------------------------------------------------.
| yyerrlab1 -- common code for both syntax error and YYERROR.  |
`-------------------------------------------------------------*/
yyerrlab1:
  yyerrstatus = 3;      /* Each real token shifted decrements this.  */

  for (;;)
    {
      yyn = yypact[yystate];
      if (!yypact_value_is_default (yyn))
        {
          yyn += YYTERROR;
          if (0 <= yyn && yyn <= YYLAST && yycheck[yyn] == YYTERROR)
            {
              yyn = yytable[yyn];
              if (0 < yyn)
                break;
            }
        }

      /* Pop the current state because it cannot handle the error token.  */
      if (yyssp == yyss)
        YYABORT;


      yydestruct ("Error: popping",
                  yystos[yystate], yyvsp);
      YYPOPSTACK (1);
      yystate = *yyssp;
      YY_STACK_PRINT (yyss, yyssp);
    }

  YY_IGNORE_MAYBE_UNINITIALIZED_BEGIN
  *++yyvsp = yylval;
  YY_IGNORE_MAYBE_UNINITIALIZED_END


  /* Shift the error token.  */
  YY_SYMBOL_PRINT ("Shifting", yystos[yyn], yyvsp, yylsp);

  yystate = yyn;
  goto yynewstate;


/*-------------------------------------.
| yyacceptlab -- YYACCEPT comes here.  |
`-------------------------------------*/
yyacceptlab:
  yyresult = 0;
  goto yyreturn;

/*-----------------------------------.
| yyabortlab -- YYABORT comes here.  |
`-----------------------------------*/
yyabortlab:
  yyresult = 1;
  goto yyreturn;

#if !defined yyoverflow || YYERROR_VERBOSE
/*-------------------------------------------------.
| yyexhaustedlab -- memory exhaustion comes here.  |
`-------------------------------------------------*/
yyexhaustedlab:
  yyerror (YY_("memory exhausted"));
  yyresult = 2;
  /* Fall through.  */
#endif

yyreturn:
  if (yychar != YYEMPTY)
    {
      /* Make sure we have latest lookahead translation.  See comments at
         user semantic actions for why this is necessary.  */
      yytoken = YYTRANSLATE (yychar);
      yydestruct ("Cleanup: discarding lookahead",
                  yytoken, &yylval);
    }
  /* Do not reclaim the symbols of the rule whose action triggered
     this YYABORT or YYACCEPT.  */
  YYPOPSTACK (yylen);
  YY_STACK_PRINT (yyss, yyssp);
  while (yyssp != yyss)
    {
      yydestruct ("Cleanup: popping",
                  yystos[*yyssp], yyvsp);
      YYPOPSTACK (1);
    }
#ifndef yyoverflow
  if (yyss != yyssa)
    YYSTACK_FREE (yyss);
#endif
#if YYERROR_VERBOSE
  if (yymsg != yymsgbuf)
    YYSTACK_FREE (yymsg);
#endif
  return yyresult;
}
#line 314 "src/syntax.y" /* yacc.c:1906  */


void yyerror(const char *s){
    fprintf(stderr, "%s\n", s);
}

#endif
