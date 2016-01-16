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
    F32 = 270,
    F64 = 271,
    C8 = 272,
    C32 = 273,
    Bool = 274,
    Void = 275,
    Eq = 276,
    NotEq = 277,
    AddEq = 278,
    SubEq = 279,
    MulEq = 280,
    DivEq = 281,
    GrtrEq = 282,
    LesrEq = 283,
    Or = 284,
    And = 285,
    True = 286,
    False = 287,
    IntLit = 288,
    FltLit = 289,
    StrLit = 290,
    Return = 291,
    If = 292,
    Elif = 293,
    Else = 294,
    For = 295,
    While = 296,
    Do = 297,
    In = 298,
    Continue = 299,
    Break = 300,
    Import = 301,
    Match = 302,
    Data = 303,
    Enum = 304,
    Pub = 305,
    Pri = 306,
    Pro = 307,
    Const = 308,
    Ext = 309,
    Dyn = 310,
    Pathogen = 311,
    Where = 312,
    Infect = 313,
    Cleanse = 314,
    Ct = 315,
    Newline = 316,
    Indent = 317,
    Unindent = 318,
    LOW = 319
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
#define F32 270
#define F64 271
#define C8 272
#define C32 273
#define Bool 274
#define Void 275
#define Eq 276
#define NotEq 277
#define AddEq 278
#define SubEq 279
#define MulEq 280
#define DivEq 281
#define GrtrEq 282
#define LesrEq 283
#define Or 284
#define And 285
#define True 286
#define False 287
#define IntLit 288
#define FltLit 289
#define StrLit 290
#define Return 291
#define If 292
#define Elif 293
#define Else 294
#define For 295
#define While 296
#define Do 297
#define In 298
#define Continue 299
#define Break 300
#define Import 301
#define Match 302
#define Data 303
#define Enum 304
#define Pub 305
#define Pri 306
#define Pro 307
#define Const 308
#define Ext 309
#define Dyn 310
#define Pathogen 311
#define Where 312
#define Infect 313
#define Cleanse 314
#define Ct 315
#define Newline 316
#define Indent 317
#define Unindent 318
#define LOW 319

/* Value type.  */
#if ! defined YYSTYPE && ! defined YYSTYPE_IS_DECLARED
typedef int YYSTYPE;
# define YYSTYPE_IS_TRIVIAL 1
# define YYSTYPE_IS_DECLARED 1
#endif


extern YYSTYPE yylval;

int yyparse (void);



/* Copy the second part of user declarations.  */

#line 260 "src/parser.c" /* yacc.c:358  */

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
#define YYLAST   614

/* YYNTOKENS -- Number of terminals.  */
#define YYNTOKENS  82
/* YYNNTS -- Number of nonterminals.  */
#define YYNNTS  44
/* YYNRULES -- Number of rules.  */
#define YYNRULES  134
/* YYNSTATES -- Number of states.  */
#define YYNSTATES  225

/* YYTRANSLATE[YYX] -- Symbol number corresponding to YYX as returned
   by yylex, with out-of-bounds checking.  */
#define YYUNDEFTOK  2
#define YYMAXUTOK   319

#define YYTRANSLATE(YYX)                                                \
  ((unsigned int) (YYX) <= YYMAXUTOK ? yytranslate[YYX] : YYUNDEFTOK)

/* YYTRANSLATE[TOKEN-NUM] -- Symbol number corresponding to TOKEN-NUM
   as returned by yylex, without out-of-bounds checking.  */
static const yytype_uint8 yytranslate[] =
{
       0,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,    72,     2,     2,
      75,    78,    70,    68,    65,    69,    74,    71,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,    81,     2,
      66,    80,    67,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,    76,     2,    77,    73,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,    79,     2,     2,     2,     2,     2,
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
      55,    56,    57,    58,    59,    60,    61,    62,    63,    64
};

#if YYDEBUG
  /* YYRLINE[YYN] -- Source line where rule number YYN was defined.  */
static const yytype_uint16 yyrline[] =
{
       0,    90,    90,    93,    94,    97,    98,   101,   102,   103,
     104,   105,   106,   107,   108,   109,   110,   111,   114,   117,
     120,   123,   126,   129,   130,   131,   132,   133,   134,   135,
     136,   137,   138,   139,   140,   141,   142,   143,   144,   145,
     146,   149,   150,   151,   152,   153,   154,   157,   158,   159,
     162,   163,   164,   165,   166,   167,   168,   171,   172,   175,
     176,   179,   180,   184,   187,   188,   191,   194,   195,   196,
     197,   200,   201,   202,   205,   206,   209,   213,   214,   215,
     216,   217,   218,   221,   224,   225,   226,   227,   230,   233,
     234,   237,   238,   241,   242,   245,   248,   251,   252,   255,
     256,   259,   260,   263,   266,   269,   272,   275,   276,   279,
     280,   281,   282,   283,   284,   285,   286,   289,   290,   293,
     294,   295,   296,   297,   298,   299,   300,   301,   302,   303,
     304,   305,   306,   307,   308
};
#endif

#if YYDEBUG || YYERROR_VERBOSE || 0
/* YYTNAME[SYMBOL-NUM] -- String name of the symbol SYMBOL-NUM.
   First, the terminals, then, starting at YYNTOKENS, nonterminals.  */
static const char *const yytname[] =
{
  "$end", "error", "$undefined", "Ident", "UserType", "I8", "I16", "I32",
  "I64", "U8", "U16", "U32", "U64", "Isz", "Usz", "F32", "F64", "C8",
  "C32", "Bool", "Void", "Eq", "NotEq", "AddEq", "SubEq", "MulEq", "DivEq",
  "GrtrEq", "LesrEq", "Or", "And", "True", "False", "IntLit", "FltLit",
  "StrLit", "Return", "If", "Elif", "Else", "For", "While", "Do", "In",
  "Continue", "Break", "Import", "Match", "Data", "Enum", "Pub", "Pri",
  "Pro", "Const", "Ext", "Dyn", "Pathogen", "Where", "Infect", "Cleanse",
  "Ct", "Newline", "Indent", "Unindent", "LOW", "','", "'<'", "'>'", "'+'",
  "'-'", "'*'", "'/'", "'%'", "'^'", "'.'", "'('", "'['", "']'", "')'",
  "'|'", "'='", "':'", "$accept", "top_level_stmt_list", "stmt_list",
  "maybe_newline", "stmt", "ident", "usertype", "intlit", "fltlit",
  "strlit", "lit_type", "type", "type_expr", "modifier", "modifier_list",
  "decl_prepend", "var_decl", "var_assign", "usertype_list", "generic",
  "data_decl", "type_decl", "type_decl_list", "type_decl_block",
  "val_init_list", "enum_block", "enum_decl", "block", "params",
  "maybe_params", "fn_decl", "fn_call", "ret_stmt", "maybe_else",
  "elif_list", "maybe_elif_list", "if_stmt", "while_loop", "do_while_loop",
  "for_loop", "var", "val", "maybe_expr", "expr", YY_NULLPTR
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
     315,   316,   317,   318,   319,    44,    60,    62,    43,    45,
      42,    47,    37,    94,    46,    40,    91,    93,    41,   124,
      61,    58
};
# endif

#define YYPACT_NINF -119

#define yypact_value_is_default(Yystate) \
  (!!((Yystate) == (-119)))

#define YYTABLE_NINF -109

#define yytable_value_is_error(Yytable_value) \
  0

  /* YYPACT[STATE-NUM] -- Index in YYTABLE of the portion describing
     STATE-NUM.  */
static const yytype_int16 yypact[] =
{
     -52,  -119,    26,   250,  -119,  -119,  -119,  -119,  -119,  -119,
    -119,  -119,  -119,  -119,  -119,  -119,  -119,  -119,  -119,  -119,
    -119,  -119,  -119,    14,    14,   414,    14,   -25,    71,     3,
    -119,  -119,  -119,  -119,  -119,  -119,  -119,   432,   -52,  -119,
     -24,  -119,  -119,   -17,   -41,  -119,   323,    39,  -119,  -119,
    -119,  -119,  -119,  -119,  -119,  -119,  -119,  -119,  -119,    -9,
    -119,  -119,  -119,  -119,  -119,    14,   -55,  -119,  -119,  -119,
    -119,  -119,  -119,   530,   476,  -119,   414,    39,    29,   476,
     250,    44,   -12,    71,    17,  -119,   -51,   250,    14,    14,
    -119,   232,    14,   432,   432,    71,     3,   -41,  -119,    49,
      14,   450,    14,    14,    14,    14,    14,    14,    14,    14,
      14,    14,    14,    14,    14,    14,    14,    55,    33,    14,
    -119,   -32,    14,   341,    71,    50,  -119,    37,    82,  -119,
    -119,  -119,    43,   530,   463,  -119,   -46,    57,   -17,   -17,
     -12,    17,  -119,    14,    14,   432,   530,  -119,    89,    89,
      89,    89,   540,   158,    89,    89,    67,    67,   -10,   -10,
     -10,   -10,    52,   -25,    94,   105,   476,  -119,   250,   530,
       1,   140,  -119,    15,  -119,  -119,    68,  -119,    14,    71,
    -119,    71,  -119,  -119,  -119,  -119,    50,  -119,  -119,    70,
     530,     1,    81,   -25,  -119,   -25,   -25,  -119,  -119,  -119,
     341,  -119,    71,  -119,   530,    73,    74,  -119,    69,  -119,
     432,  -119,  -119,  -119,  -119,  -119,    14,    14,   432,     1,
     530,   530,   -25,  -119,  -119
};

  /* YYDEFACT[STATE-NUM] -- Default reduction number in state STATE-NUM.
     Performed when YYTABLE does not specify something else to do.  Zero
     means the default is an error.  */
static const yytype_uint8 yydefact[] =
{
       6,     5,     0,     0,     1,    18,    19,    23,    24,    25,
      26,    27,    28,    29,    30,    31,    32,    33,    34,    35,
      36,    37,    38,     0,     0,     0,     0,     0,     0,     0,
      50,    51,    52,    53,    54,    55,    56,     0,     6,     4,
      40,    39,    46,    49,    60,    58,     0,     0,     7,     8,
      11,    17,     9,    10,    12,    16,    13,    14,    15,     0,
     115,   116,    20,    21,    22,     0,   108,   112,   113,   114,
     109,   111,   134,    96,     0,    40,     0,     0,     0,     0,
       0,     0,     0,     0,     0,    87,     0,     2,   118,     0,
      41,     0,   118,     0,     0,     0,     0,    59,    57,    62,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,   102,    62,     0,
     104,     6,     0,     0,     0,     0,    69,    82,     0,    85,
      43,     3,     0,   117,     0,    45,     0,     0,    47,    48,
       0,     0,    86,   118,     0,    92,    63,   110,   128,   129,
     130,   131,   132,   133,   124,   125,   119,   120,   121,   122,
     123,   126,   127,     0,   101,    98,     0,    88,     0,   105,
      72,     0,    75,     0,    73,    65,     0,    70,     0,     0,
      83,     0,    95,   107,    44,    42,     0,    67,    84,     0,
      61,     0,    91,     0,   100,     0,     0,   103,   106,    71,
       0,    76,     0,    66,    81,    79,    77,    68,     0,    90,
       0,    93,    99,    97,    74,    64,     0,     0,    92,     0,
      80,    78,     0,    89,    94
};

  /* YYPGOTO[NTERM-NUM].  */
static const yytype_int16 yypgoto[] =
{
    -119,  -119,    75,   -35,   -85,    -3,    32,  -119,  -119,  -119,
    -119,   -26,   -36,   -40,   -13,   141,   143,  -119,  -119,    31,
    -119,   -31,  -119,  -117,  -119,   -71,  -118,   -44,  -119,   -45,
    -119,    36,  -119,  -119,  -119,  -119,  -119,  -119,  -119,  -119,
      38,  -119,   -81,    -8
};

  /* YYDEFGOTO[NTERM-NUM].  */
static const yytype_int16 yydefgoto[] =
{
      -1,     2,    38,     3,    39,    66,    41,    67,    68,    69,
      42,    43,    44,    45,    46,    47,    48,    49,   176,   125,
      50,   172,   173,   126,   128,    85,    51,    81,   192,   193,
      52,    70,    54,   197,   164,   165,    55,    56,    57,    58,
      71,    72,   132,   133
};

  /* YYTABLE[YYPACT[STATE-NUM]] -- What to do in state STATE-NUM.  If
     positive, shift that token.  If negative, reduce the rule whose
     number is the opposite.  If YYTABLE_NINF, syntax error.  */
static const yytype_int16 yytable[] =
{
      40,    86,   131,    87,     5,   174,    98,     6,   177,     1,
      97,   137,    76,   129,    93,    73,    74,     5,    79,    93,
      88,    89,    75,   187,    93,   142,     4,   130,    94,     1,
     117,   167,   184,    94,    75,   120,    98,    80,    94,    53,
      97,    59,     5,    75,    99,    60,    61,    62,    63,    64,
     123,    88,    89,    90,   124,   136,  -108,   101,    91,    92,
      82,    84,   189,   115,   116,    83,    93,   138,   139,   207,
     188,   100,   119,    75,   118,     6,   200,    40,   201,    83,
      94,   134,   174,   131,    40,   122,   168,   170,    75,    65,
      75,    75,   146,   163,   148,   149,   150,   151,   152,   153,
     154,   155,   156,   157,   158,   159,   160,   161,   162,   191,
     171,   166,   123,   144,   169,   127,    53,   178,    59,   194,
      75,   182,   198,    53,   143,    59,   116,   140,   141,   144,
     145,    98,   195,   202,   185,   203,   190,   112,   113,   114,
     115,   116,    75,   179,   196,   180,   210,   181,   208,   211,
     218,   212,   213,   216,   217,   121,   175,   110,   111,   112,
     113,   114,   115,   116,   170,    40,    77,   199,    78,   214,
     204,   186,     0,   222,   219,     0,     0,     0,   224,   102,
     103,     0,   191,     0,     0,   104,   105,   171,   209,    96,
      30,    31,    32,    33,    34,    35,    36,    75,     0,     0,
       0,     0,     0,     0,    53,     0,    59,    75,   220,   221,
       0,   205,     0,   206,     0,    75,   223,     0,     0,     0,
       0,     0,     0,     0,   108,   109,   110,   111,   112,   113,
     114,   115,   116,     0,   215,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    22,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
      22,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,    23,    24,     0,     0,
      25,    26,    27,     0,     0,     0,     0,     0,    28,    29,
      30,    31,    32,    33,    34,    35,    36,    37,     0,     0,
     135,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,    37,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    22,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    22,     0,     0,     0,     0,     0,     0,     0,     0,
       0,    95,    96,    30,    31,    32,    33,    34,    35,    36,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
      29,    30,    31,    32,    33,    34,    35,    36,    37,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,    37,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    22,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    22,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,    30,    31,    32,    33,    34,    35,
      36,   102,   103,     0,     0,     0,     0,   104,   105,   106,
     107,     0,     0,     0,   102,   103,     0,     0,     0,    37,
     104,   105,   106,   107,     0,     0,     0,   102,   103,     0,
       0,     0,     0,   104,   105,   106,   107,    37,     0,     0,
       0,     0,     0,     0,     0,     0,   108,   109,   110,   111,
     112,   113,   114,   115,   116,     0,     0,     0,   147,   108,
     109,   110,   111,   112,   113,   114,   115,   116,    80,     0,
     183,     0,   108,   109,   110,   111,   112,   113,   114,   115,
     116,   102,   103,     0,     0,     0,     0,   104,   105,   106,
     107,   102,   103,     0,     0,     0,     0,   104,   105,     0,
     107,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,   108,   109,   110,   111,
     112,   113,   114,   115,   116,     0,   108,   109,   110,   111,
     112,   113,   114,   115,   116
};

static const yytype_int16 yycheck[] =
{
       3,    37,    87,    38,     3,   123,    46,     4,   125,    61,
      46,    92,    25,    84,    65,    23,    24,     3,    26,    65,
      75,    76,    25,   140,    65,    96,     0,    78,    79,    61,
      74,    63,    78,    79,    37,    79,    76,    62,    79,     3,
      76,     3,     3,    46,    47,    31,    32,    33,    34,    35,
      62,    75,    76,    70,    66,    91,    80,    65,    75,    76,
      28,    29,   143,    73,    74,    62,    65,    93,    94,   186,
     141,    80,    43,    76,    77,     4,    61,    80,    63,    62,
      79,    89,   200,   168,    87,    41,   121,   123,    91,    75,
      93,    94,   100,    38,   102,   103,   104,   105,   106,   107,
     108,   109,   110,   111,   112,   113,   114,   115,   116,   145,
     123,   119,    62,    80,   122,    83,    80,    80,    80,   163,
     123,    78,   166,    87,    75,    87,    74,    95,    96,    80,
      81,   171,    38,    65,    77,    67,   144,    70,    71,    72,
      73,    74,   145,    61,    39,    63,    65,    65,    78,   193,
      81,   195,   196,    80,    80,    80,   124,    68,    69,    70,
      71,    72,    73,    74,   200,   168,    25,   170,    25,   200,
     178,   140,    -1,   218,   210,    -1,    -1,    -1,   222,    21,
      22,    -1,   218,    -1,    -1,    27,    28,   200,   191,    49,
      50,    51,    52,    53,    54,    55,    56,   200,    -1,    -1,
      -1,    -1,    -1,    -1,   168,    -1,   168,   210,   216,   217,
      -1,   179,    -1,   181,    -1,   218,   219,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    66,    67,    68,    69,    70,    71,
      72,    73,    74,    -1,   202,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,     3,     4,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    36,    37,    -1,    -1,
      40,    41,    42,    -1,    -1,    -1,    -1,    -1,    48,    49,
      50,    51,    52,    53,    54,    55,    56,    75,    -1,    -1,
      78,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    75,     3,     4,     5,     6,
       7,     8,     9,    10,    11,    12,    13,    14,    15,    16,
      17,    18,    19,    20,     3,     4,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    48,    49,    50,    51,    52,    53,    54,    55,    56,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      49,    50,    51,    52,    53,    54,    55,    56,    75,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    75,     3,     4,     5,
       6,     7,     8,     9,    10,    11,    12,    13,    14,    15,
      16,    17,    18,    19,    20,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    50,    51,    52,    53,    54,    55,
      56,    21,    22,    -1,    -1,    -1,    -1,    27,    28,    29,
      30,    -1,    -1,    -1,    21,    22,    -1,    -1,    -1,    75,
      27,    28,    29,    30,    -1,    -1,    -1,    21,    22,    -1,
      -1,    -1,    -1,    27,    28,    29,    30,    75,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    66,    67,    68,    69,
      70,    71,    72,    73,    74,    -1,    -1,    -1,    78,    66,
      67,    68,    69,    70,    71,    72,    73,    74,    62,    -1,
      77,    -1,    66,    67,    68,    69,    70,    71,    72,    73,
      74,    21,    22,    -1,    -1,    -1,    -1,    27,    28,    29,
      30,    21,    22,    -1,    -1,    -1,    -1,    27,    28,    -1,
      30,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    66,    67,    68,    69,
      70,    71,    72,    73,    74,    -1,    66,    67,    68,    69,
      70,    71,    72,    73,    74
};

  /* YYSTOS[STATE-NUM] -- The (internal number of the) accessing
     symbol of state STATE-NUM.  */
static const yytype_uint8 yystos[] =
{
       0,    61,    83,    85,     0,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    36,    37,    40,    41,    42,    48,    49,
      50,    51,    52,    53,    54,    55,    56,    75,    84,    86,
      87,    88,    92,    93,    94,    95,    96,    97,    98,    99,
     102,   108,   112,   113,   114,   118,   119,   120,   121,   122,
      31,    32,    33,    34,    35,    75,    87,    89,    90,    91,
     113,   122,   123,   125,   125,    87,    96,    97,    98,   125,
      62,   109,    88,    62,    88,   107,    94,    85,    75,    76,
      70,    75,    76,    65,    79,    48,    49,    94,    95,    87,
      80,   125,    21,    22,    27,    28,    29,    30,    66,    67,
      68,    69,    70,    71,    72,    73,    74,   109,    87,    43,
     109,    84,    41,    62,    66,   101,   105,    88,   106,   107,
      78,    86,   124,   125,   125,    78,    94,   124,    93,    93,
      88,    88,   107,    75,    80,    81,   125,    78,   125,   125,
     125,   125,   125,   125,   125,   125,   125,   125,   125,   125,
     125,   125,   125,    38,   116,   117,   125,    63,    85,   125,
      94,    96,   103,   104,   108,    88,   100,   105,    80,    61,
      63,    65,    78,    77,    78,    77,   101,   105,   107,   124,
     125,    94,   110,   111,   109,    38,    39,   115,   109,    87,
      61,    63,    65,    67,   125,    88,    88,   105,    78,    87,
      65,   109,   109,   109,   103,    88,    80,    80,    81,    94,
     125,   125,   111,    87,   109
};

  /* YYR1[YYN] -- Symbol number of symbol that rule YYN derives.  */
static const yytype_uint8 yyr1[] =
{
       0,    82,    83,    84,    84,    85,    85,    86,    86,    86,
      86,    86,    86,    86,    86,    86,    86,    86,    87,    88,
      89,    90,    91,    92,    92,    92,    92,    92,    92,    92,
      92,    92,    92,    92,    92,    92,    92,    92,    92,    92,
      92,    93,    93,    93,    93,    93,    93,    94,    94,    94,
      95,    95,    95,    95,    95,    95,    95,    96,    96,    97,
      97,    98,    98,    99,   100,   100,   101,   102,   102,   102,
     102,   103,   103,   103,   104,   104,   105,   106,   106,   106,
     106,   106,   106,   107,   108,   108,   108,   108,   109,   110,
     110,   111,   111,   112,   112,   113,   114,   115,   115,   116,
     116,   117,   117,   118,   119,   120,   121,   122,   122,   123,
     123,   123,   123,   123,   123,   123,   123,   124,   124,   125,
     125,   125,   125,   125,   125,   125,   125,   125,   125,   125,
     125,   125,   125,   125,   125
};

  /* YYR2[YYN] -- Number of symbols on the right hand side of rule YYN.  */
static const yytype_uint8 yyr2[] =
{
       0,     2,     3,     3,     1,     1,     0,     1,     1,     1,
       1,     1,     1,     1,     1,     1,     1,     1,     1,     1,
       1,     1,     1,     1,     1,     1,     1,     1,     1,     1,
       1,     1,     1,     1,     1,     1,     1,     1,     1,     1,
       1,     2,     4,     3,     4,     3,     1,     3,     3,     1,
       1,     1,     1,     1,     1,     1,     1,     2,     1,     2,
       1,     4,     2,     3,     3,     1,     3,     4,     5,     3,
       4,     2,     1,     1,     3,     1,     3,     3,     5,     3,
       5,     3,     1,     3,     4,     3,     3,     2,     3,     4,
       2,     1,     0,     5,     8,     4,     2,     2,     0,     3,
       2,     1,     0,     5,     3,     4,     5,     4,     1,     1,
       3,     1,     1,     1,     1,     1,     1,     1,     0,     3,
       3,     3,     3,     3,     3,     3,     3,     3,     3,     3,
       3,     3,     3,     3,     1
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
#line 1590 "src/parser.c" /* yacc.c:1646  */
    break;

  case 4:
#line 94 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = setRoot((yyvsp[0]));}
#line 1596 "src/parser.c" /* yacc.c:1646  */
    break;

  case 7:
#line 101 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1602 "src/parser.c" /* yacc.c:1646  */
    break;

  case 8:
#line 102 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1608 "src/parser.c" /* yacc.c:1646  */
    break;

  case 9:
#line 103 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1614 "src/parser.c" /* yacc.c:1646  */
    break;

  case 10:
#line 104 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1620 "src/parser.c" /* yacc.c:1646  */
    break;

  case 11:
#line 105 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1626 "src/parser.c" /* yacc.c:1646  */
    break;

  case 12:
#line 106 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1632 "src/parser.c" /* yacc.c:1646  */
    break;

  case 13:
#line 107 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1638 "src/parser.c" /* yacc.c:1646  */
    break;

  case 14:
#line 108 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1644 "src/parser.c" /* yacc.c:1646  */
    break;

  case 15:
#line 109 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1650 "src/parser.c" /* yacc.c:1646  */
    break;

  case 16:
#line 110 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1656 "src/parser.c" /* yacc.c:1646  */
    break;

  case 17:
#line 111 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1662 "src/parser.c" /* yacc.c:1646  */
    break;

  case 18:
#line 114 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (Node*)lextxt;}
#line 1668 "src/parser.c" /* yacc.c:1646  */
    break;

  case 19:
#line 117 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (Node*)lextxt;}
#line 1674 "src/parser.c" /* yacc.c:1646  */
    break;

  case 20:
#line 120 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkIntLitNode(lextxt);}
#line 1680 "src/parser.c" /* yacc.c:1646  */
    break;

  case 21:
#line 123 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkFltLitNode(lextxt);}
#line 1686 "src/parser.c" /* yacc.c:1646  */
    break;

  case 22:
#line 126 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkStrLitNode(lextxt);}
#line 1692 "src/parser.c" /* yacc.c:1646  */
    break;

  case 23:
#line 129 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_I8,  (char*)"");}
#line 1698 "src/parser.c" /* yacc.c:1646  */
    break;

  case 24:
#line 130 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_I16, (char*)"");}
#line 1704 "src/parser.c" /* yacc.c:1646  */
    break;

  case 25:
#line 131 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_I32, (char*)"");}
#line 1710 "src/parser.c" /* yacc.c:1646  */
    break;

  case 26:
#line 132 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_I64, (char*)"");}
#line 1716 "src/parser.c" /* yacc.c:1646  */
    break;

  case 27:
#line 133 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_U8,  (char*)"");}
#line 1722 "src/parser.c" /* yacc.c:1646  */
    break;

  case 28:
#line 134 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_U16, (char*)"");}
#line 1728 "src/parser.c" /* yacc.c:1646  */
    break;

  case 29:
#line 135 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_U32, (char*)"");}
#line 1734 "src/parser.c" /* yacc.c:1646  */
    break;

  case 30:
#line 136 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_U64, (char*)"");}
#line 1740 "src/parser.c" /* yacc.c:1646  */
    break;

  case 31:
#line 137 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_Isz, (char*)"");}
#line 1746 "src/parser.c" /* yacc.c:1646  */
    break;

  case 32:
#line 138 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_Usz, (char*)"");}
#line 1752 "src/parser.c" /* yacc.c:1646  */
    break;

  case 33:
#line 139 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_F32, (char*)"");}
#line 1758 "src/parser.c" /* yacc.c:1646  */
    break;

  case 34:
#line 140 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_F64, (char*)"");}
#line 1764 "src/parser.c" /* yacc.c:1646  */
    break;

  case 35:
#line 141 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_C8,  (char*)"");}
#line 1770 "src/parser.c" /* yacc.c:1646  */
    break;

  case 36:
#line 142 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_C32, (char*)"");}
#line 1776 "src/parser.c" /* yacc.c:1646  */
    break;

  case 37:
#line 143 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_Bool, (char*)"");}
#line 1782 "src/parser.c" /* yacc.c:1646  */
    break;

  case 38:
#line 144 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_Void, (char*)"");}
#line 1788 "src/parser.c" /* yacc.c:1646  */
    break;

  case 39:
#line 145 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_UserType, (char*)(yyvsp[0]));}
#line 1794 "src/parser.c" /* yacc.c:1646  */
    break;

  case 40:
#line 146 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_Ident, (char*)(yyvsp[0]));}
#line 1800 "src/parser.c" /* yacc.c:1646  */
    break;

  case 46:
#line 154 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1806 "src/parser.c" /* yacc.c:1646  */
    break;

  case 49:
#line 159 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1812 "src/parser.c" /* yacc.c:1646  */
    break;

  case 59:
#line 175 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1818 "src/parser.c" /* yacc.c:1646  */
    break;

  case 60:
#line 176 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1824 "src/parser.c" /* yacc.c:1646  */
    break;

  case 61:
#line 179 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarDeclNode((char*)(yyvsp[-2]), (yyvsp[-3]), (yyvsp[0]));}
#line 1830 "src/parser.c" /* yacc.c:1646  */
    break;

  case 62:
#line 180 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarDeclNode((char*)(yyvsp[0]), (yyvsp[-1]), 0);}
#line 1836 "src/parser.c" /* yacc.c:1646  */
    break;

  case 63:
#line 184 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarAssignNode((yyvsp[-2]), (yyvsp[0]));}
#line 1842 "src/parser.c" /* yacc.c:1646  */
    break;

  case 64:
#line 187 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = setNext((yyvsp[-2]), (yyvsp[0]));}
#line 1848 "src/parser.c" /* yacc.c:1646  */
    break;

  case 65:
#line 188 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = setRoot((yyvsp[0]));}
#line 1854 "src/parser.c" /* yacc.c:1646  */
    break;

  case 66:
#line 191 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = getRoot();}
#line 1860 "src/parser.c" /* yacc.c:1646  */
    break;

  case 67:
#line 194 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkDataDeclNode((char*)(yyvsp[-1]), (yyvsp[0]));}
#line 1866 "src/parser.c" /* yacc.c:1646  */
    break;

  case 68:
#line 195 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkDataDeclNode((char*)(yyvsp[-2]), (yyvsp[0]));}
#line 1872 "src/parser.c" /* yacc.c:1646  */
    break;

  case 69:
#line 196 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkDataDeclNode((char*)(yyvsp[-1]), (yyvsp[0]));}
#line 1878 "src/parser.c" /* yacc.c:1646  */
    break;

  case 70:
#line 197 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkDataDeclNode((char*)(yyvsp[-2]), (yyvsp[0]));}
#line 1884 "src/parser.c" /* yacc.c:1646  */
    break;

  case 84:
#line 224 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 1890 "src/parser.c" /* yacc.c:1646  */
    break;

  case 85:
#line 225 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 1896 "src/parser.c" /* yacc.c:1646  */
    break;

  case 86:
#line 226 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 1902 "src/parser.c" /* yacc.c:1646  */
    break;

  case 87:
#line 227 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 1908 "src/parser.c" /* yacc.c:1646  */
    break;

  case 88:
#line 230 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = getRoot();}
#line 1914 "src/parser.c" /* yacc.c:1646  */
    break;

  case 89:
#line 233 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = setNext((yyvsp[-3]), mkNamedValNode((char*)(yyvsp[0]), (yyvsp[-1])));}
#line 1920 "src/parser.c" /* yacc.c:1646  */
    break;

  case 90:
#line 234 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkNamedValNode((char*)(yyvsp[0]), (yyvsp[-1]));}
#line 1926 "src/parser.c" /* yacc.c:1646  */
    break;

  case 91:
#line 237 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1932 "src/parser.c" /* yacc.c:1646  */
    break;

  case 92:
#line 238 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 1938 "src/parser.c" /* yacc.c:1646  */
    break;

  case 93:
#line 241 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkFuncDeclNode((char*)(yyvsp[-3]), (yyvsp[-4]), (yyvsp[-1]), (yyvsp[0]));}
#line 1944 "src/parser.c" /* yacc.c:1646  */
    break;

  case 94:
#line 242 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkFuncDeclNode((char*)(yyvsp[-6]), (yyvsp[-7]), (yyvsp[-1]), (yyvsp[0]));}
#line 1950 "src/parser.c" /* yacc.c:1646  */
    break;

  case 95:
#line 245 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkFuncCallNode((char*)(yyvsp[-3]), (yyvsp[-1]));}
#line 1956 "src/parser.c" /* yacc.c:1646  */
    break;

  case 96:
#line 248 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkRetNode((yyvsp[0]));}
#line 1962 "src/parser.c" /* yacc.c:1646  */
    break;

  case 97:
#line 251 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 1968 "src/parser.c" /* yacc.c:1646  */
    break;

  case 98:
#line 252 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 1974 "src/parser.c" /* yacc.c:1646  */
    break;

  case 99:
#line 255 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 1980 "src/parser.c" /* yacc.c:1646  */
    break;

  case 100:
#line 256 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 1986 "src/parser.c" /* yacc.c:1646  */
    break;

  case 101:
#line 259 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 1992 "src/parser.c" /* yacc.c:1646  */
    break;

  case 102:
#line 260 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 1998 "src/parser.c" /* yacc.c:1646  */
    break;

  case 103:
#line 263 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkIfNode((yyvsp[-3]), (yyvsp[-2]));}
#line 2004 "src/parser.c" /* yacc.c:1646  */
    break;

  case 104:
#line 266 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2010 "src/parser.c" /* yacc.c:1646  */
    break;

  case 105:
#line 269 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2016 "src/parser.c" /* yacc.c:1646  */
    break;

  case 106:
#line 272 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2022 "src/parser.c" /* yacc.c:1646  */
    break;

  case 107:
#line 275 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarNode((char*)(yyvsp[-3]));}
#line 2028 "src/parser.c" /* yacc.c:1646  */
    break;

  case 108:
#line 276 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarNode((char*)(yyvsp[0]));}
#line 2034 "src/parser.c" /* yacc.c:1646  */
    break;

  case 109:
#line 279 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 2040 "src/parser.c" /* yacc.c:1646  */
    break;

  case 110:
#line 280 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[-1]);}
#line 2046 "src/parser.c" /* yacc.c:1646  */
    break;

  case 111:
#line 281 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 2052 "src/parser.c" /* yacc.c:1646  */
    break;

  case 112:
#line 282 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 2058 "src/parser.c" /* yacc.c:1646  */
    break;

  case 113:
#line 283 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 2064 "src/parser.c" /* yacc.c:1646  */
    break;

  case 114:
#line 284 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 2070 "src/parser.c" /* yacc.c:1646  */
    break;

  case 115:
#line 285 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBoolLitNode(1);}
#line 2076 "src/parser.c" /* yacc.c:1646  */
    break;

  case 116:
#line 286 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBoolLitNode(0);}
#line 2082 "src/parser.c" /* yacc.c:1646  */
    break;

  case 117:
#line 289 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 2088 "src/parser.c" /* yacc.c:1646  */
    break;

  case 118:
#line 290 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2094 "src/parser.c" /* yacc.c:1646  */
    break;

  case 119:
#line 293 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode('+', (yyvsp[-2]), (yyvsp[0]));}
#line 2100 "src/parser.c" /* yacc.c:1646  */
    break;

  case 120:
#line 294 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode('-', (yyvsp[-2]), (yyvsp[0]));}
#line 2106 "src/parser.c" /* yacc.c:1646  */
    break;

  case 121:
#line 295 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode('*', (yyvsp[-2]), (yyvsp[0]));}
#line 2112 "src/parser.c" /* yacc.c:1646  */
    break;

  case 122:
#line 296 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode('/', (yyvsp[-2]), (yyvsp[0]));}
#line 2118 "src/parser.c" /* yacc.c:1646  */
    break;

  case 123:
#line 297 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode('%', (yyvsp[-2]), (yyvsp[0]));}
#line 2124 "src/parser.c" /* yacc.c:1646  */
    break;

  case 124:
#line 298 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode('<', (yyvsp[-2]), (yyvsp[0]));}
#line 2130 "src/parser.c" /* yacc.c:1646  */
    break;

  case 125:
#line 299 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode('>', (yyvsp[-2]), (yyvsp[0]));}
#line 2136 "src/parser.c" /* yacc.c:1646  */
    break;

  case 126:
#line 300 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode('^', (yyvsp[-2]), (yyvsp[0]));}
#line 2142 "src/parser.c" /* yacc.c:1646  */
    break;

  case 127:
#line 301 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode('.', (yyvsp[-2]), (yyvsp[0]));}
#line 2148 "src/parser.c" /* yacc.c:1646  */
    break;

  case 128:
#line 302 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode(Tok_Eq, (yyvsp[-2]), (yyvsp[0]));}
#line 2154 "src/parser.c" /* yacc.c:1646  */
    break;

  case 129:
#line 303 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode(Tok_NotEq, (yyvsp[-2]), (yyvsp[0]));}
#line 2160 "src/parser.c" /* yacc.c:1646  */
    break;

  case 130:
#line 304 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode(Tok_GrtrEq, (yyvsp[-2]), (yyvsp[0]));}
#line 2166 "src/parser.c" /* yacc.c:1646  */
    break;

  case 131:
#line 305 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode(Tok_LesrEq, (yyvsp[-2]), (yyvsp[0]));}
#line 2172 "src/parser.c" /* yacc.c:1646  */
    break;

  case 132:
#line 306 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode(Tok_Or, (yyvsp[-2]), (yyvsp[0]));}
#line 2178 "src/parser.c" /* yacc.c:1646  */
    break;

  case 133:
#line 307 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBinOpNode(Tok_And, (yyvsp[-2]), (yyvsp[0]));}
#line 2184 "src/parser.c" /* yacc.c:1646  */
    break;

  case 134:
#line 308 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 2190 "src/parser.c" /* yacc.c:1646  */
    break;


#line 2194 "src/parser.c" /* yacc.c:1646  */
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
#line 311 "src/syntax.y" /* yacc.c:1906  */


void yyerror(const char *s){
    fprintf(stderr, "%s\n", s);
}

#endif
