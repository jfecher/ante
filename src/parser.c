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

#include <stdlib.h>
#include <stdio.h>
#include <tokens.h>
#include <ptree.h>

extern int yylex();
extern char *yytext;

void yyerror(const char *msg);

#define YYSTYPE Node*
#define YYERROR_VERBOSE


#line 82 "src/parser.c" /* yacc.c:339  */

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
    Range = 286,
    RangeBX = 287,
    RangeEX = 288,
    RangeX = 289,
    True = 290,
    False = 291,
    IntLit = 292,
    FltLit = 293,
    StrLit = 294,
    Return = 295,
    If = 296,
    Elif = 297,
    Else = 298,
    For = 299,
    While = 300,
    Do = 301,
    In = 302,
    Continue = 303,
    Break = 304,
    Import = 305,
    Match = 306,
    Data = 307,
    Enum = 308,
    Pub = 309,
    Pri = 310,
    Pro = 311,
    Const = 312,
    Ext = 313,
    Dyn = 314,
    Pathogen = 315,
    Where = 316,
    Infect = 317,
    Cleanse = 318,
    Ct = 319,
    Newline = 320,
    Indent = 321,
    Unindent = 322,
    LOW = 323
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
#define Range 286
#define RangeBX 287
#define RangeEX 288
#define RangeX 289
#define True 290
#define False 291
#define IntLit 292
#define FltLit 293
#define StrLit 294
#define Return 295
#define If 296
#define Elif 297
#define Else 298
#define For 299
#define While 300
#define Do 301
#define In 302
#define Continue 303
#define Break 304
#define Import 305
#define Match 306
#define Data 307
#define Enum 308
#define Pub 309
#define Pri 310
#define Pro 311
#define Const 312
#define Ext 313
#define Dyn 314
#define Pathogen 315
#define Where 316
#define Infect 317
#define Cleanse 318
#define Ct 319
#define Newline 320
#define Indent 321
#define Unindent 322
#define LOW 323

/* Value type.  */
#if ! defined YYSTYPE && ! defined YYSTYPE_IS_DECLARED
typedef int YYSTYPE;
# define YYSTYPE_IS_TRIVIAL 1
# define YYSTYPE_IS_DECLARED 1
#endif


extern YYSTYPE yylval;

int yyparse (void);



/* Copy the second part of user declarations.  */

#line 266 "src/parser.c" /* yacc.c:358  */

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
#define YYLAST   648

/* YYNTOKENS -- Number of terminals.  */
#define YYNTOKENS  85
/* YYNNTS -- Number of nonterminals.  */
#define YYNNTS  44
/* YYNRULES -- Number of rules.  */
#define YYNRULES  137
/* YYNSTATES -- Number of states.  */
#define YYNSTATES  231

/* YYTRANSLATE[YYX] -- Symbol number corresponding to YYX as returned
   by yylex, with out-of-bounds checking.  */
#define YYUNDEFTOK  2
#define YYMAXUTOK   323

#define YYTRANSLATE(YYX)                                                \
  ((unsigned int) (YYX) <= YYMAXUTOK ? yytranslate[YYX] : YYUNDEFTOK)

/* YYTRANSLATE[TOKEN-NUM] -- Symbol number corresponding to TOKEN-NUM
   as returned by yylex, without out-of-bounds checking.  */
static const yytype_uint8 yytranslate[] =
{
       0,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,    76,     2,     2,
      78,    81,    74,    72,    69,    73,    77,    75,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,    84,     2,
      70,    83,    71,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,    79,     2,    80,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,    82,     2,     2,     2,     2,     2,
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
      65,    66,    67,    68
};

#if YYDEBUG
  /* YYRLINE[YYN] -- Source line where rule number YYN was defined.  */
static const yytype_uint16 yyrline[] =
{
       0,    88,    88,    91,    92,    95,    96,    99,   100,   101,
     102,   103,   104,   105,   106,   107,   108,   109,   112,   115,
     118,   121,   124,   127,   128,   129,   130,   131,   132,   133,
     134,   135,   136,   137,   138,   139,   140,   141,   142,   143,
     144,   147,   148,   149,   150,   151,   152,   155,   156,   157,
     160,   161,   162,   163,   164,   165,   166,   169,   170,   173,
     174,   177,   178,   182,   185,   186,   189,   192,   193,   194,
     195,   198,   199,   200,   203,   204,   207,   211,   212,   213,
     214,   215,   216,   219,   222,   223,   224,   225,   228,   231,
     232,   235,   236,   239,   240,   243,   246,   249,   250,   253,
     254,   257,   258,   261,   264,   267,   270,   273,   274,   277,
     278,   279,   280,   281,   282,   283,   284,   287,   288,   291,
     292,   293,   294,   295,   296,   297,   298,   299,   300,   301,
     302,   303,   304,   305,   306,   307,   308,   309
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
  "GrtrEq", "LesrEq", "Or", "And", "Range", "RangeBX", "RangeEX", "RangeX",
  "True", "False", "IntLit", "FltLit", "StrLit", "Return", "If", "Elif",
  "Else", "For", "While", "Do", "In", "Continue", "Break", "Import",
  "Match", "Data", "Enum", "Pub", "Pri", "Pro", "Const", "Ext", "Dyn",
  "Pathogen", "Where", "Infect", "Cleanse", "Ct", "Newline", "Indent",
  "Unindent", "LOW", "','", "'<'", "'>'", "'+'", "'-'", "'*'", "'/'",
  "'%'", "'.'", "'('", "'['", "']'", "')'", "'|'", "'='", "':'", "$accept",
  "top_level_stmt_list", "statement_list", "maybe_newline", "statement",
  "ident", "usertype", "intlit", "fltlit", "strlit", "lit_type", "type",
  "type_expr", "modifier", "modifier_list", "decl_prepend", "var_decl",
  "var_assign", "usertype_list", "generic", "data_decl", "type_decl",
  "type_decl_list", "type_decl_block", "val_init_list", "enum_block",
  "enum_decl", "block", "params", "maybe_params", "fn_decl", "fn_call",
  "ret_stmt", "maybe_else", "elif_list", "maybe_elif_list", "if_stmt",
  "while_loop", "do_while_loop", "for_loop", "var", "val", "maybe_expr",
  "expr", YY_NULLPTR
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
     315,   316,   317,   318,   319,   320,   321,   322,   323,    44,
      60,    62,    43,    45,    42,    47,    37,    46,    40,    91,
      93,    41,   124,    61,    58
};
# endif

#define YYPACT_NINF -125

#define yypact_value_is_default(Yystate) \
  (!!((Yystate) == (-125)))

#define YYTABLE_NINF -109

#define yytable_value_is_error(Yytable_value) \
  0

  /* YYPACT[STATE-NUM] -- Index in YYTABLE of the portion describing
     STATE-NUM.  */
static const yytype_int16 yypact[] =
{
     -59,  -125,    17,   268,  -125,  -125,  -125,  -125,  -125,  -125,
    -125,  -125,  -125,  -125,  -125,  -125,  -125,  -125,  -125,  -125,
    -125,  -125,  -125,   118,   118,   446,   118,   -30,    49,     1,
    -125,  -125,  -125,  -125,  -125,  -125,  -125,   287,   -59,  -125,
     -20,  -125,  -125,     4,   -62,  -125,   344,    51,  -125,  -125,
    -125,  -125,  -125,  -125,  -125,  -125,  -125,  -125,  -125,   -27,
    -125,  -125,  -125,  -125,  -125,   118,    46,  -125,  -125,  -125,
    -125,  -125,  -125,   500,   486,  -125,   446,    51,    22,   486,
     268,    26,    -4,    49,     9,  -125,   -40,   268,   118,   118,
    -125,   238,   118,   287,   287,    49,     1,   -62,  -125,   -70,
     118,   156,   118,   118,   118,   118,   118,   118,   118,   118,
     118,   118,   118,   118,   118,   118,   118,   118,   118,   118,
      37,    43,   118,  -125,   -42,   118,   370,    49,    63,  -125,
      48,    65,  -125,  -125,  -125,    59,   500,   410,  -125,   -36,
      64,     4,     4,    -4,     9,  -125,   118,   118,   287,   500,
    -125,   132,   132,   132,   132,   514,   571,   -25,   -25,   -25,
     -25,   132,   132,    61,    61,    66,    66,    66,    66,   -30,
      99,   104,   486,  -125,   268,   500,     7,   206,  -125,   -37,
    -125,  -125,   -32,  -125,   118,    49,  -125,    49,  -125,  -125,
    -125,  -125,    63,  -125,  -125,    69,   500,     7,    82,   -30,
    -125,   -30,   -30,  -125,  -125,  -125,   370,  -125,    49,  -125,
     500,    75,    77,  -125,    68,  -125,   287,  -125,  -125,  -125,
    -125,  -125,   118,   118,   287,     7,   500,   500,   -30,  -125,
    -125
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
     109,   111,   137,    96,     0,    40,     0,     0,     0,     0,
       0,     0,     0,     0,     0,    87,     0,     2,   118,     0,
      41,     0,   118,     0,     0,     0,     0,    59,    57,    62,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
     102,    62,     0,   104,     6,     0,     0,     0,     0,    69,
      82,     0,    85,    43,     3,     0,   117,     0,    45,     0,
       0,    47,    48,     0,     0,    86,   118,     0,    92,    63,
     110,   127,   128,   129,   130,   131,   132,   133,   135,   134,
     136,   124,   125,   119,   120,   121,   122,   123,   126,     0,
     101,    98,     0,    88,     0,   105,    72,     0,    75,     0,
      73,    65,     0,    70,     0,     0,    83,     0,    95,   107,
      44,    42,     0,    67,    84,     0,    61,     0,    91,     0,
     100,     0,     0,   103,   106,    71,     0,    76,     0,    66,
      81,    79,    77,    68,     0,    90,     0,    93,    99,    97,
      74,    64,     0,     0,    92,     0,    80,    78,     0,    89,
      94
};

  /* YYPGOTO[NTERM-NUM].  */
static const yytype_int16 yypgoto[] =
{
    -125,  -125,    81,   -11,   -52,    -3,    32,  -125,  -125,  -125,
    -125,   -29,    -6,   -44,   -14,   137,   142,  -125,  -125,    25,
    -125,   -34,  -125,  -124,  -125,   -72,   -88,   -53,  -125,   -55,
    -125,     0,  -125,  -125,  -125,  -125,  -125,  -125,  -125,  -125,
       6,  -125,   -91,    -8
};

  /* YYDEFGOTO[NTERM-NUM].  */
static const yytype_int16 yydefgoto[] =
{
      -1,     2,    38,     3,    39,    66,    41,    67,    68,    69,
      42,    43,    44,    45,    46,    47,    48,    49,   182,   128,
      50,   178,   179,   129,   131,    85,    51,    81,   198,   199,
      52,    70,    54,   203,   170,   171,    55,    56,    57,    58,
      71,    72,   135,   136
};

  /* YYTABLE[YYPACT[STATE-NUM]] -- What to do in state STATE-NUM.  If
     positive, shift that token.  If negative, reduce the rule whose
     number is the opposite.  If YYTABLE_NINF, syntax error.  */
static const yytype_int16 yytable[] =
{
      40,   140,    98,    53,   183,     6,     1,    93,   146,    59,
       5,    76,   132,   147,   148,    73,    74,     4,    79,   193,
      94,   120,    75,     1,   145,   173,   123,    87,   206,    93,
     207,    86,    98,    93,    75,   134,    80,   208,   180,   209,
      97,   133,    94,    75,    99,   190,    94,   114,   115,   116,
     117,   118,   119,     6,     5,   195,   100,   101,    88,    89,
      82,    84,   126,  -108,   141,   142,   127,    83,   213,   122,
      97,   125,   194,    75,   121,    83,    93,    40,    90,   169,
      53,   137,    91,    92,    40,   139,    59,    53,    75,    94,
      75,    75,   149,    59,   151,   152,   153,   154,   155,   156,
     157,   158,   159,   160,   161,   162,   163,   164,   165,   166,
     167,   168,   177,   174,   172,   130,   200,   175,   180,   204,
     176,     5,   134,    75,    88,    89,   147,   143,   144,   126,
     185,   184,   186,    98,   187,   116,   117,   118,   119,   196,
     188,   201,   197,   119,   191,    75,   217,   202,   218,   219,
     214,   216,   224,    60,    61,    62,    63,    64,   222,   181,
     223,   124,    77,   108,   109,   110,   111,    78,   192,   228,
       0,    40,   220,   205,    53,   230,   210,   102,   103,     0,
      59,     0,     0,   104,   105,   106,   107,   108,   109,   110,
     111,     0,   177,     0,   215,     0,    65,     0,     0,     0,
     176,     0,     0,    75,   114,   115,   116,   117,   118,   119,
     225,     0,     0,    75,   226,   227,     0,   211,   197,   212,
       0,    75,   229,     0,     0,     0,   112,   113,   114,   115,
     116,   117,   118,   119,     0,     0,     0,   150,     0,     0,
     221,     5,     6,     7,     8,     9,    10,    11,    12,    13,
      14,    15,    16,    17,    18,    19,    20,    21,    22,    96,
      30,    31,    32,    33,    34,    35,    36,     0,     0,     0,
       0,     5,     6,     7,     8,     9,    10,    11,    12,    13,
      14,    15,    16,    17,    18,    19,    20,    21,    22,     0,
       5,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    22,    23,    24,
       0,     0,    25,    26,    27,     0,    37,     0,     0,   138,
      28,    29,    30,    31,    32,    33,    34,    35,    36,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,    37,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    22,    37,     0,     0,     0,     0,
       0,     0,     0,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
      22,     0,     0,     0,     0,     0,    95,    96,    30,    31,
      32,    33,    34,    35,    36,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,    37,    29,    30,    31,    32,    33,    34,    35,
      36,   102,   103,     0,     0,     0,     0,   104,   105,   106,
     107,   108,   109,   110,   111,     0,     0,     0,    37,     5,
       6,     7,     8,     9,    10,    11,    12,    13,    14,    15,
      16,    17,    18,    19,    20,    21,    22,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
     112,   113,   114,   115,   116,   117,   118,   119,     0,     0,
     189,     0,     0,     0,     0,     0,     0,     0,     0,     0,
      30,    31,    32,    33,    34,    35,    36,   102,   103,     0,
       0,     0,     0,   104,   105,   106,   107,   108,   109,   110,
     111,   102,   103,     0,    37,     0,     0,   104,   105,   106,
     107,   108,   109,   110,   111,   102,   103,     0,     0,     0,
       0,   104,   105,     0,   107,   108,   109,   110,   111,     0,
       0,     0,    80,     0,     0,     0,   112,   113,   114,   115,
     116,   117,   118,   119,     0,     0,     0,     0,     0,     0,
     112,   113,   114,   115,   116,   117,   118,   119,     0,     0,
       0,     0,     0,     0,   112,   113,   114,   115,   116,   117,
     118,   119,   102,   103,     0,     0,     0,     0,   104,   105,
       0,     0,   108,   109,   110,   111,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,   112,   113,   114,   115,   116,   117,   118,   119
};

static const yytype_int16 yycheck[] =
{
       3,    92,    46,     3,   128,     4,    65,    69,    78,     3,
       3,    25,    84,    83,    84,    23,    24,     0,    26,   143,
      82,    74,    25,    65,    96,    67,    79,    38,    65,    69,
      67,    37,    76,    69,    37,    87,    66,    69,   126,    71,
      46,    81,    82,    46,    47,    81,    82,    72,    73,    74,
      75,    76,    77,     4,     3,   146,    83,    65,    78,    79,
      28,    29,    66,    83,    93,    94,    70,    66,   192,    47,
      76,    45,   144,    76,    77,    66,    69,    80,    74,    42,
      80,    89,    78,    79,    87,    91,    80,    87,    91,    82,
      93,    94,   100,    87,   102,   103,   104,   105,   106,   107,
     108,   109,   110,   111,   112,   113,   114,   115,   116,   117,
     118,   119,   126,   124,   122,    83,   169,   125,   206,   172,
     126,     3,   174,   126,    78,    79,    83,    95,    96,    66,
      65,    83,    67,   177,    69,    74,    75,    76,    77,   147,
      81,    42,   148,    77,    80,   148,   199,    43,   201,   202,
      81,    69,    84,    35,    36,    37,    38,    39,    83,   127,
      83,    80,    25,    31,    32,    33,    34,    25,   143,   224,
      -1,   174,   206,   176,   174,   228,   184,    21,    22,    -1,
     174,    -1,    -1,    27,    28,    29,    30,    31,    32,    33,
      34,    -1,   206,    -1,   197,    -1,    78,    -1,    -1,    -1,
     206,    -1,    -1,   206,    72,    73,    74,    75,    76,    77,
     216,    -1,    -1,   216,   222,   223,    -1,   185,   224,   187,
      -1,   224,   225,    -1,    -1,    -1,    70,    71,    72,    73,
      74,    75,    76,    77,    -1,    -1,    -1,    81,    -1,    -1,
     208,     3,     4,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    53,
      54,    55,    56,    57,    58,    59,    60,    -1,    -1,    -1,
      -1,     3,     4,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    -1,
       3,     4,     5,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    40,    41,
      -1,    -1,    44,    45,    46,    -1,    78,    -1,    -1,    81,
      52,    53,    54,    55,    56,    57,    58,    59,    60,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    78,     3,     4,     5,
       6,     7,     8,     9,    10,    11,    12,    13,    14,    15,
      16,    17,    18,    19,    20,    78,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,     3,     4,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    -1,    -1,    -1,    -1,    -1,    52,    53,    54,    55,
      56,    57,    58,    59,    60,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    78,    53,    54,    55,    56,    57,    58,    59,
      60,    21,    22,    -1,    -1,    -1,    -1,    27,    28,    29,
      30,    31,    32,    33,    34,    -1,    -1,    -1,    78,     3,
       4,     5,     6,     7,     8,     9,    10,    11,    12,    13,
      14,    15,    16,    17,    18,    19,    20,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      70,    71,    72,    73,    74,    75,    76,    77,    -1,    -1,
      80,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      54,    55,    56,    57,    58,    59,    60,    21,    22,    -1,
      -1,    -1,    -1,    27,    28,    29,    30,    31,    32,    33,
      34,    21,    22,    -1,    78,    -1,    -1,    27,    28,    29,
      30,    31,    32,    33,    34,    21,    22,    -1,    -1,    -1,
      -1,    27,    28,    -1,    30,    31,    32,    33,    34,    -1,
      -1,    -1,    66,    -1,    -1,    -1,    70,    71,    72,    73,
      74,    75,    76,    77,    -1,    -1,    -1,    -1,    -1,    -1,
      70,    71,    72,    73,    74,    75,    76,    77,    -1,    -1,
      -1,    -1,    -1,    -1,    70,    71,    72,    73,    74,    75,
      76,    77,    21,    22,    -1,    -1,    -1,    -1,    27,    28,
      -1,    -1,    31,    32,    33,    34,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    70,    71,    72,    73,    74,    75,    76,    77
};

  /* YYSTOS[STATE-NUM] -- The (internal number of the) accessing
     symbol of state STATE-NUM.  */
static const yytype_uint8 yystos[] =
{
       0,    65,    86,    88,     0,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    40,    41,    44,    45,    46,    52,    53,
      54,    55,    56,    57,    58,    59,    60,    78,    87,    89,
      90,    91,    95,    96,    97,    98,    99,   100,   101,   102,
     105,   111,   115,   116,   117,   121,   122,   123,   124,   125,
      35,    36,    37,    38,    39,    78,    90,    92,    93,    94,
     116,   125,   126,   128,   128,    90,    99,   100,   101,   128,
      66,   112,    91,    66,    91,   110,    97,    88,    78,    79,
      74,    78,    79,    69,    82,    52,    53,    97,    98,    90,
      83,   128,    21,    22,    27,    28,    29,    30,    31,    32,
      33,    34,    70,    71,    72,    73,    74,    75,    76,    77,
     112,    90,    47,   112,    87,    45,    66,    70,   104,   108,
      91,   109,   110,    81,    89,   127,   128,   128,    81,    97,
     127,    96,    96,    91,    91,   110,    78,    83,    84,   128,
      81,   128,   128,   128,   128,   128,   128,   128,   128,   128,
     128,   128,   128,   128,   128,   128,   128,   128,   128,    42,
     119,   120,   128,    67,    88,   128,    97,    99,   106,   107,
     111,    91,   103,   108,    83,    65,    67,    69,    81,    80,
      81,    80,   104,   108,   110,   127,   128,    97,   113,   114,
     112,    42,    43,   118,   112,    90,    65,    67,    69,    71,
     128,    91,    91,   108,    81,    90,    69,   112,   112,   112,
     106,    91,    83,    83,    84,    97,   128,   128,   114,    90,
     112
};

  /* YYR1[YYN] -- Symbol number of symbol that rule YYN derives.  */
static const yytype_uint8 yyr1[] =
{
       0,    85,    86,    87,    87,    88,    88,    89,    89,    89,
      89,    89,    89,    89,    89,    89,    89,    89,    90,    91,
      92,    93,    94,    95,    95,    95,    95,    95,    95,    95,
      95,    95,    95,    95,    95,    95,    95,    95,    95,    95,
      95,    96,    96,    96,    96,    96,    96,    97,    97,    97,
      98,    98,    98,    98,    98,    98,    98,    99,    99,   100,
     100,   101,   101,   102,   103,   103,   104,   105,   105,   105,
     105,   106,   106,   106,   107,   107,   108,   109,   109,   109,
     109,   109,   109,   110,   111,   111,   111,   111,   112,   113,
     113,   114,   114,   115,   115,   116,   117,   118,   118,   119,
     119,   120,   120,   121,   122,   123,   124,   125,   125,   126,
     126,   126,   126,   126,   126,   126,   126,   127,   127,   128,
     128,   128,   128,   128,   128,   128,   128,   128,   128,   128,
     128,   128,   128,   128,   128,   128,   128,   128
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
       3,     3,     3,     3,     3,     3,     3,     1
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
        case 2:
#line 88 "src/syntax.y" /* yacc.c:1646  */
    {setRoot((yyvsp[-1]));}
#line 1607 "src/parser.c" /* yacc.c:1646  */
    break;

  case 3:
#line 91 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = setNext((yyvsp[-2]), (yyvsp[0]));}
#line 1613 "src/parser.c" /* yacc.c:1646  */
    break;

  case 4:
#line 92 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1619 "src/parser.c" /* yacc.c:1646  */
    break;

  case 7:
#line 99 "src/syntax.y" /* yacc.c:1646  */
    {puts("var_decl"); (yyval) = (yyvsp[0]);}
#line 1625 "src/parser.c" /* yacc.c:1646  */
    break;

  case 8:
#line 100 "src/syntax.y" /* yacc.c:1646  */
    {puts("var_assign"); (yyval) = (yyvsp[0]);}
#line 1631 "src/parser.c" /* yacc.c:1646  */
    break;

  case 9:
#line 101 "src/syntax.y" /* yacc.c:1646  */
    {puts("fn_decl"); (yyval) = (yyvsp[0]);}
#line 1637 "src/parser.c" /* yacc.c:1646  */
    break;

  case 10:
#line 102 "src/syntax.y" /* yacc.c:1646  */
    {puts("fn_call"); (yyval) = (yyvsp[0]);}
#line 1643 "src/parser.c" /* yacc.c:1646  */
    break;

  case 11:
#line 103 "src/syntax.y" /* yacc.c:1646  */
    {puts("data_decl"); (yyval) = (yyvsp[0]);}
#line 1649 "src/parser.c" /* yacc.c:1646  */
    break;

  case 12:
#line 104 "src/syntax.y" /* yacc.c:1646  */
    {puts("ret_stmt"); (yyval) = (yyvsp[0]);}
#line 1655 "src/parser.c" /* yacc.c:1646  */
    break;

  case 13:
#line 105 "src/syntax.y" /* yacc.c:1646  */
    {puts("while_loop"); (yyval) = (yyvsp[0]);}
#line 1661 "src/parser.c" /* yacc.c:1646  */
    break;

  case 14:
#line 106 "src/syntax.y" /* yacc.c:1646  */
    {puts("do_while"); (yyval) = (yyvsp[0]);}
#line 1667 "src/parser.c" /* yacc.c:1646  */
    break;

  case 15:
#line 107 "src/syntax.y" /* yacc.c:1646  */
    {puts("for_loop"); (yyval) = (yyvsp[0]);}
#line 1673 "src/parser.c" /* yacc.c:1646  */
    break;

  case 16:
#line 108 "src/syntax.y" /* yacc.c:1646  */
    {puts("if_stmt"); (yyval) = (yyvsp[0]);}
#line 1679 "src/parser.c" /* yacc.c:1646  */
    break;

  case 17:
#line 109 "src/syntax.y" /* yacc.c:1646  */
    {puts("enum_decl"); (yyval) = (yyvsp[0]);}
#line 1685 "src/parser.c" /* yacc.c:1646  */
    break;

  case 18:
#line 112 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (Node*)yytext;}
#line 1691 "src/parser.c" /* yacc.c:1646  */
    break;

  case 20:
#line 118 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkIntLitNode(yytext);}
#line 1697 "src/parser.c" /* yacc.c:1646  */
    break;

  case 21:
#line 121 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkFltLitNode(yytext);}
#line 1703 "src/parser.c" /* yacc.c:1646  */
    break;

  case 22:
#line 124 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkStrLitNode(yytext);}
#line 1709 "src/parser.c" /* yacc.c:1646  */
    break;

  case 23:
#line 127 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_I8,  NULL);}
#line 1715 "src/parser.c" /* yacc.c:1646  */
    break;

  case 24:
#line 128 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_I16, NULL);}
#line 1721 "src/parser.c" /* yacc.c:1646  */
    break;

  case 25:
#line 129 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_I32, NULL);}
#line 1727 "src/parser.c" /* yacc.c:1646  */
    break;

  case 26:
#line 130 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_I64, NULL);}
#line 1733 "src/parser.c" /* yacc.c:1646  */
    break;

  case 27:
#line 131 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_U8,  NULL);}
#line 1739 "src/parser.c" /* yacc.c:1646  */
    break;

  case 28:
#line 132 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_U16, NULL);}
#line 1745 "src/parser.c" /* yacc.c:1646  */
    break;

  case 29:
#line 133 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_U32, NULL);}
#line 1751 "src/parser.c" /* yacc.c:1646  */
    break;

  case 30:
#line 134 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_U64, NULL);}
#line 1757 "src/parser.c" /* yacc.c:1646  */
    break;

  case 31:
#line 135 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_Isz, NULL);}
#line 1763 "src/parser.c" /* yacc.c:1646  */
    break;

  case 32:
#line 136 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_Usz, NULL);}
#line 1769 "src/parser.c" /* yacc.c:1646  */
    break;

  case 33:
#line 137 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_F32, NULL);}
#line 1775 "src/parser.c" /* yacc.c:1646  */
    break;

  case 34:
#line 138 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_F64, NULL);}
#line 1781 "src/parser.c" /* yacc.c:1646  */
    break;

  case 35:
#line 139 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_C8,  NULL);}
#line 1787 "src/parser.c" /* yacc.c:1646  */
    break;

  case 36:
#line 140 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_C32, NULL);}
#line 1793 "src/parser.c" /* yacc.c:1646  */
    break;

  case 37:
#line 141 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_Bool, NULL);}
#line 1799 "src/parser.c" /* yacc.c:1646  */
    break;

  case 38:
#line 142 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_Void, NULL);}
#line 1805 "src/parser.c" /* yacc.c:1646  */
    break;

  case 39:
#line 143 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkTypeNode(Tok_UserType, yytext);}
#line 1811 "src/parser.c" /* yacc.c:1646  */
    break;

  case 40:
#line 144 "src/syntax.y" /* yacc.c:1646  */
    { (yyval) = mkTypeNode(Ident, (char*)(yyvsp[0]));}
#line 1817 "src/parser.c" /* yacc.c:1646  */
    break;

  case 46:
#line 152 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1823 "src/parser.c" /* yacc.c:1646  */
    break;

  case 59:
#line 173 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1829 "src/parser.c" /* yacc.c:1646  */
    break;

  case 60:
#line 174 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1835 "src/parser.c" /* yacc.c:1646  */
    break;

  case 61:
#line 177 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarDeclNode((char*)(yyvsp[-2]), (yyvsp[-3]), (yyvsp[0]));}
#line 1841 "src/parser.c" /* yacc.c:1646  */
    break;

  case 62:
#line 178 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarDeclNode((char*)(yyvsp[0]), (yyvsp[-1]), 0);}
#line 1847 "src/parser.c" /* yacc.c:1646  */
    break;

  case 63:
#line 182 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarAssignNode((char*)(yyvsp[-2]), (yyvsp[0]));}
#line 1853 "src/parser.c" /* yacc.c:1646  */
    break;

  case 65:
#line 186 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1859 "src/parser.c" /* yacc.c:1646  */
    break;

  case 67:
#line 192 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarNode("TEMP");}
#line 1865 "src/parser.c" /* yacc.c:1646  */
    break;

  case 68:
#line 193 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarNode("TEMP");}
#line 1871 "src/parser.c" /* yacc.c:1646  */
    break;

  case 69:
#line 194 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarNode("TEMP");}
#line 1877 "src/parser.c" /* yacc.c:1646  */
    break;

  case 70:
#line 195 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarNode("TEMP");}
#line 1883 "src/parser.c" /* yacc.c:1646  */
    break;

  case 84:
#line 222 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarNode("TODO: enum_decl node");}
#line 1889 "src/parser.c" /* yacc.c:1646  */
    break;

  case 85:
#line 223 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarNode("TODO: enum_decl node");}
#line 1895 "src/parser.c" /* yacc.c:1646  */
    break;

  case 86:
#line 224 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarNode("TODO: enum_decl node");}
#line 1901 "src/parser.c" /* yacc.c:1646  */
    break;

  case 87:
#line 225 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarNode("TODO: enum_decl node");}
#line 1907 "src/parser.c" /* yacc.c:1646  */
    break;

  case 88:
#line 228 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[-1]);}
#line 1913 "src/parser.c" /* yacc.c:1646  */
    break;

  case 93:
#line 239 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkFuncDeclNode((char*)(yyvsp[-3]), (yyvsp[-4]), (yyvsp[-1]), (yyvsp[0]));}
#line 1919 "src/parser.c" /* yacc.c:1646  */
    break;

  case 94:
#line 240 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkFuncDeclNode((char*)(yyvsp[-6]), (yyvsp[-7]), (yyvsp[-1]), (yyvsp[0]));}
#line 1925 "src/parser.c" /* yacc.c:1646  */
    break;

  case 95:
#line 243 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkFuncCallNode((char*)(yyvsp[-3]), (yyvsp[-1]));}
#line 1931 "src/parser.c" /* yacc.c:1646  */
    break;

  case 96:
#line 246 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkRetNode((yyvsp[0]));}
#line 1937 "src/parser.c" /* yacc.c:1646  */
    break;

  case 97:
#line 249 "src/syntax.y" /* yacc.c:1646  */
    { puts("TODO: else"); }
#line 1943 "src/parser.c" /* yacc.c:1646  */
    break;

  case 100:
#line 254 "src/syntax.y" /* yacc.c:1646  */
    { puts("TODO: elif"); }
#line 1949 "src/parser.c" /* yacc.c:1646  */
    break;

  case 103:
#line 261 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkIfNode((yyvsp[-3]), (yyvsp[-2]));}
#line 1955 "src/parser.c" /* yacc.c:1646  */
    break;

  case 104:
#line 264 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarNode("TODO: while_loop node");}
#line 1961 "src/parser.c" /* yacc.c:1646  */
    break;

  case 105:
#line 267 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarNode("TODO: do_while_loop node");}
#line 1967 "src/parser.c" /* yacc.c:1646  */
    break;

  case 106:
#line 270 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkVarNode("TODO: for_loop node");}
#line 1973 "src/parser.c" /* yacc.c:1646  */
    break;

  case 108:
#line 274 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1979 "src/parser.c" /* yacc.c:1646  */
    break;

  case 109:
#line 277 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1985 "src/parser.c" /* yacc.c:1646  */
    break;

  case 110:
#line 278 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[-1]);}
#line 1991 "src/parser.c" /* yacc.c:1646  */
    break;

  case 111:
#line 279 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 1997 "src/parser.c" /* yacc.c:1646  */
    break;

  case 112:
#line 280 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 2003 "src/parser.c" /* yacc.c:1646  */
    break;

  case 113:
#line 281 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 2009 "src/parser.c" /* yacc.c:1646  */
    break;

  case 114:
#line 282 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = (yyvsp[0]);}
#line 2015 "src/parser.c" /* yacc.c:1646  */
    break;

  case 115:
#line 283 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBoolLitNode(1);}
#line 2021 "src/parser.c" /* yacc.c:1646  */
    break;

  case 116:
#line 284 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = mkBoolLitNode(0);}
#line 2027 "src/parser.c" /* yacc.c:1646  */
    break;

  case 117:
#line 287 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2033 "src/parser.c" /* yacc.c:1646  */
    break;

  case 118:
#line 288 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2039 "src/parser.c" /* yacc.c:1646  */
    break;

  case 119:
#line 291 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2045 "src/parser.c" /* yacc.c:1646  */
    break;

  case 120:
#line 292 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2051 "src/parser.c" /* yacc.c:1646  */
    break;

  case 121:
#line 293 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2057 "src/parser.c" /* yacc.c:1646  */
    break;

  case 122:
#line 294 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2063 "src/parser.c" /* yacc.c:1646  */
    break;

  case 123:
#line 295 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2069 "src/parser.c" /* yacc.c:1646  */
    break;

  case 124:
#line 296 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2075 "src/parser.c" /* yacc.c:1646  */
    break;

  case 125:
#line 297 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2081 "src/parser.c" /* yacc.c:1646  */
    break;

  case 126:
#line 298 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2087 "src/parser.c" /* yacc.c:1646  */
    break;

  case 127:
#line 299 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2093 "src/parser.c" /* yacc.c:1646  */
    break;

  case 128:
#line 300 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2099 "src/parser.c" /* yacc.c:1646  */
    break;

  case 129:
#line 301 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2105 "src/parser.c" /* yacc.c:1646  */
    break;

  case 130:
#line 302 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2111 "src/parser.c" /* yacc.c:1646  */
    break;

  case 131:
#line 303 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2117 "src/parser.c" /* yacc.c:1646  */
    break;

  case 132:
#line 304 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2123 "src/parser.c" /* yacc.c:1646  */
    break;

  case 133:
#line 305 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2129 "src/parser.c" /* yacc.c:1646  */
    break;

  case 134:
#line 306 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2135 "src/parser.c" /* yacc.c:1646  */
    break;

  case 135:
#line 307 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2141 "src/parser.c" /* yacc.c:1646  */
    break;

  case 136:
#line 308 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2147 "src/parser.c" /* yacc.c:1646  */
    break;

  case 137:
#line 309 "src/syntax.y" /* yacc.c:1646  */
    {(yyval) = NULL;}
#line 2153 "src/parser.c" /* yacc.c:1646  */
    break;


#line 2157 "src/parser.c" /* yacc.c:1646  */
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
#line 312 "src/syntax.y" /* yacc.c:1906  */


void yyerror(const char *s){
    fprintf(stderr, "%s\nerrtok = %d\n", s, yychar);
}

