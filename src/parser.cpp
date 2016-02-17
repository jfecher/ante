// A Bison parser, made by GNU Bison 3.0.4.

// Skeleton implementation for Bison LALR(1) parsers in C++

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


// First part of user declarations.
#line 1 "src/syntax.y" // lalr1.cc:404

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

struct ModTyPair{ Node *mod, *ty; };


#line 68 "src/parser.cpp" // lalr1.cc:404

# ifndef YY_NULLPTR
#  if defined __cplusplus && 201103L <= __cplusplus
#   define YY_NULLPTR nullptr
#  else
#   define YY_NULLPTR 0
#  endif
# endif

#include "yyparser.h"

// User implementation prologue.

#line 82 "src/parser.cpp" // lalr1.cc:412


#ifndef YY_
# if defined YYENABLE_NLS && YYENABLE_NLS
#  if ENABLE_NLS
#   include <libintl.h> // FIXME: INFRINGES ON USER NAME SPACE.
#   define YY_(msgid) dgettext ("bison-runtime", msgid)
#  endif
# endif
# ifndef YY_
#  define YY_(msgid) msgid
# endif
#endif

#define YYRHSLOC(Rhs, K) ((Rhs)[K].location)
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


// Suppress unused-variable warnings by "using" E.
#define YYUSE(E) ((void) (E))

// Enable debugging if requested.
#if YYDEBUG

// A pseudo ostream that takes yydebug_ into account.
# define YYCDEBUG if (yydebug_) (*yycdebug_)

# define YY_SYMBOL_PRINT(Title, Symbol)         \
  do {                                          \
    if (yydebug_)                               \
    {                                           \
      *yycdebug_ << Title << ' ';               \
      yy_print_ (*yycdebug_, Symbol);           \
      *yycdebug_ << std::endl;                  \
    }                                           \
  } while (false)

# define YY_REDUCE_PRINT(Rule)          \
  do {                                  \
    if (yydebug_)                       \
      yy_reduce_print_ (Rule);          \
  } while (false)

# define YY_STACK_PRINT()               \
  do {                                  \
    if (yydebug_)                       \
      yystack_print_ ();                \
  } while (false)

#else // !YYDEBUG

# define YYCDEBUG if (false) std::cerr
# define YY_SYMBOL_PRINT(Title, Symbol)  YYUSE(Symbol)
# define YY_REDUCE_PRINT(Rule)           static_cast<void>(0)
# define YY_STACK_PRINT()                static_cast<void>(0)

#endif // !YYDEBUG

#define yyerrok         (yyerrstatus_ = 0)
#define yyclearin       (yyla.clear ())

#define YYACCEPT        goto yyacceptlab
#define YYABORT         goto yyabortlab
#define YYERROR         goto yyerrorlab
#define YYRECOVERING()  (!!yyerrstatus_)


namespace yy {
#line 168 "src/parser.cpp" // lalr1.cc:479

  /* Return YYSTR after stripping away unnecessary quotes and
     backslashes, so that it's suitable for yyerror.  The heuristic is
     that double-quoting is unnecessary unless the string contains an
     apostrophe, a comma, or backslash (other than backslash-backslash).
     YYSTR is taken from yytname.  */
  std::string
  parser::yytnamerr_ (const char *yystr)
  {
    if (*yystr == '"')
      {
        std::string yyr = "";
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
              // Fall through.
            default:
              yyr += *yyp;
              break;

            case '"':
              return yyr;
            }
      do_not_strip_quotes: ;
      }

    return yystr;
  }


  /// Build a parser object.
  parser::parser ()
#if YYDEBUG
     :yydebug_ (false),
      yycdebug_ (&std::cerr)
#endif
  {}

  parser::~parser ()
  {}


  /*---------------.
  | Symbol types.  |
  `---------------*/

  inline
  parser::syntax_error::syntax_error (const location_type& l, const std::string& m)
    : std::runtime_error (m)
    , location (l)
  {}

  // basic_symbol.
  template <typename Base>
  inline
  parser::basic_symbol<Base>::basic_symbol ()
    : value ()
  {}

  template <typename Base>
  inline
  parser::basic_symbol<Base>::basic_symbol (const basic_symbol& other)
    : Base (other)
    , value ()
    , location (other.location)
  {
    value = other.value;
  }


  template <typename Base>
  inline
  parser::basic_symbol<Base>::basic_symbol (typename Base::kind_type t, const semantic_type& v, const location_type& l)
    : Base (t)
    , value (v)
    , location (l)
  {}


  /// Constructor for valueless symbols.
  template <typename Base>
  inline
  parser::basic_symbol<Base>::basic_symbol (typename Base::kind_type t, const location_type& l)
    : Base (t)
    , value ()
    , location (l)
  {}

  template <typename Base>
  inline
  parser::basic_symbol<Base>::~basic_symbol ()
  {
    clear ();
  }

  template <typename Base>
  inline
  void
  parser::basic_symbol<Base>::clear ()
  {
    Base::clear ();
  }

  template <typename Base>
  inline
  bool
  parser::basic_symbol<Base>::empty () const
  {
    return Base::type_get () == empty_symbol;
  }

  template <typename Base>
  inline
  void
  parser::basic_symbol<Base>::move (basic_symbol& s)
  {
    super_type::move(s);
    value = s.value;
    location = s.location;
  }

  // by_type.
  inline
  parser::by_type::by_type ()
    : type (empty_symbol)
  {}

  inline
  parser::by_type::by_type (const by_type& other)
    : type (other.type)
  {}

  inline
  parser::by_type::by_type (token_type t)
    : type (yytranslate_ (t))
  {}

  inline
  void
  parser::by_type::clear ()
  {
    type = empty_symbol;
  }

  inline
  void
  parser::by_type::move (by_type& that)
  {
    type = that.type;
    that.clear ();
  }

  inline
  int
  parser::by_type::type_get () const
  {
    return type;
  }


  // by_state.
  inline
  parser::by_state::by_state ()
    : state (empty_state)
  {}

  inline
  parser::by_state::by_state (const by_state& other)
    : state (other.state)
  {}

  inline
  void
  parser::by_state::clear ()
  {
    state = empty_state;
  }

  inline
  void
  parser::by_state::move (by_state& that)
  {
    state = that.state;
    that.clear ();
  }

  inline
  parser::by_state::by_state (state_type s)
    : state (s)
  {}

  inline
  parser::symbol_number_type
  parser::by_state::type_get () const
  {
    if (state == empty_state)
      return empty_symbol;
    else
      return yystos_[state];
  }

  inline
  parser::stack_symbol_type::stack_symbol_type ()
  {}


  inline
  parser::stack_symbol_type::stack_symbol_type (state_type s, symbol_type& that)
    : super_type (s, that.location)
  {
    value = that.value;
    // that is emptied.
    that.type = empty_symbol;
  }

  inline
  parser::stack_symbol_type&
  parser::stack_symbol_type::operator= (const stack_symbol_type& that)
  {
    state = that.state;
    value = that.value;
    location = that.location;
    return *this;
  }


  template <typename Base>
  inline
  void
  parser::yy_destroy_ (const char* yymsg, basic_symbol<Base>& yysym) const
  {
    if (yymsg)
      YY_SYMBOL_PRINT (yymsg, yysym);

    // User destructor.
    YYUSE (yysym.type_get ());
  }

#if YYDEBUG
  template <typename Base>
  void
  parser::yy_print_ (std::ostream& yyo,
                                     const basic_symbol<Base>& yysym) const
  {
    std::ostream& yyoutput = yyo;
    YYUSE (yyoutput);
    symbol_number_type yytype = yysym.type_get ();
    // Avoid a (spurious) G++ 4.8 warning about "array subscript is
    // below array bounds".
    if (yysym.empty ())
      std::abort ();
    yyo << (yytype < yyntokens_ ? "token" : "nterm")
        << ' ' << yytname_[yytype] << " ("
        << yysym.location << ": ";
    YYUSE (yytype);
    yyo << ')';
  }
#endif

  inline
  void
  parser::yypush_ (const char* m, state_type s, symbol_type& sym)
  {
    stack_symbol_type t (s, sym);
    yypush_ (m, t);
  }

  inline
  void
  parser::yypush_ (const char* m, stack_symbol_type& s)
  {
    if (m)
      YY_SYMBOL_PRINT (m, s);
    yystack_.push (s);
  }

  inline
  void
  parser::yypop_ (unsigned int n)
  {
    yystack_.pop (n);
  }

#if YYDEBUG
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
    return yydebug_;
  }

  void
  parser::set_debug_level (debug_level_type l)
  {
    yydebug_ = l;
  }
#endif // YYDEBUG

  inline parser::state_type
  parser::yy_lr_goto_state_ (state_type yystate, int yysym)
  {
    int yyr = yypgoto_[yysym - yyntokens_] + yystate;
    if (0 <= yyr && yyr <= yylast_ && yycheck_[yyr] == yystate)
      return yytable_[yyr];
    else
      return yydefgoto_[yysym - yyntokens_];
  }

  inline bool
  parser::yy_pact_value_is_default_ (int yyvalue)
  {
    return yyvalue == yypact_ninf_;
  }

  inline bool
  parser::yy_table_value_is_error_ (int yyvalue)
  {
    return yyvalue == yytable_ninf_;
  }

  int
  parser::parse ()
  {
    // State.
    int yyn;
    /// Length of the RHS of the rule being reduced.
    int yylen = 0;

    // Error handling.
    int yynerrs_ = 0;
    int yyerrstatus_ = 0;

    /// The lookahead symbol.
    symbol_type yyla;

    /// The locations where the error started and ended.
    stack_symbol_type yyerror_range[3];

    /// The return value of parse ().
    int yyresult;

    // FIXME: This shoud be completely indented.  It is not yet to
    // avoid gratuitous conflicts when merging into the master branch.
    try
      {
    YYCDEBUG << "Starting parse" << std::endl;


    /* Initialize the stack.  The initial state will be set in
       yynewstate, since the latter expects the semantical and the
       location values to have been already stored, initialize these
       stacks with a primary value.  */
    yystack_.clear ();
    yypush_ (YY_NULLPTR, 0, yyla);

    // A new symbol was pushed on the stack.
  yynewstate:
    YYCDEBUG << "Entering state " << yystack_[0].state << std::endl;

    // Accept?
    if (yystack_[0].state == yyfinal_)
      goto yyacceptlab;

    goto yybackup;

    // Backup.
  yybackup:

    // Try to take a decision without lookahead.
    yyn = yypact_[yystack_[0].state];
    if (yy_pact_value_is_default_ (yyn))
      goto yydefault;

    // Read a lookahead token.
    if (yyla.empty ())
      {
        YYCDEBUG << "Reading a token: ";
        try
          {
            yyla.type = yytranslate_ (yylex (&yyla.value, &yyla.location));
          }
        catch (const syntax_error& yyexc)
          {
            error (yyexc);
            goto yyerrlab1;
          }
      }
    YY_SYMBOL_PRINT ("Next token is", yyla);

    /* If the proper action on seeing token YYLA.TYPE is to reduce or
       to detect an error, take that action.  */
    yyn += yyla.type_get ();
    if (yyn < 0 || yylast_ < yyn || yycheck_[yyn] != yyla.type_get ())
      goto yydefault;

    // Reduce or error.
    yyn = yytable_[yyn];
    if (yyn <= 0)
      {
        if (yy_table_value_is_error_ (yyn))
          goto yyerrlab;
        yyn = -yyn;
        goto yyreduce;
      }

    // Count tokens shifted since error; after three, turn off error status.
    if (yyerrstatus_)
      --yyerrstatus_;

    // Shift the lookahead token.
    yypush_ ("Shifting", yyn, yyla);
    goto yynewstate;

  /*-----------------------------------------------------------.
  | yydefault -- do the default action for the current state.  |
  `-----------------------------------------------------------*/
  yydefault:
    yyn = yydefact_[yystack_[0].state];
    if (yyn == 0)
      goto yyerrlab;
    goto yyreduce;

  /*-----------------------------.
  | yyreduce -- Do a reduction.  |
  `-----------------------------*/
  yyreduce:
    yylen = yyr2_[yyn];
    {
      stack_symbol_type yylhs;
      yylhs.state = yy_lr_goto_state_(yystack_[yylen].state, yyr1_[yyn]);
      /* If YYLEN is nonzero, implement the default value of the
         action: '$$ = $1'.  Otherwise, use the top of the stack.

         Otherwise, the following line sets YYLHS.VALUE to garbage.
         This behavior is undocumented and Bison users should not rely
         upon it.  */
      if (yylen)
        yylhs.value = yystack_[yylen - 1].value;
      else
        yylhs.value = yystack_[0].value;

      // Compute the default @$.
      {
        slice<stack_symbol_type, stack_type> slice (yystack_, yylen);
        YYLLOC_DEFAULT (yylhs.location, slice, yylen);
      }

      // Perform the reduction.
      YY_REDUCE_PRINT (yyn);
      try
        {
          switch (yyn)
            {
  case 3:
#line 109 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 646 "src/parser.cpp" // lalr1.cc:859
    break;

  case 4:
#line 110 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 652 "src/parser.cpp" // lalr1.cc:859
    break;

  case 7:
#line 117 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 658 "src/parser.cpp" // lalr1.cc:859
    break;

  case 8:
#line 118 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 664 "src/parser.cpp" // lalr1.cc:859
    break;

  case 9:
#line 119 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 670 "src/parser.cpp" // lalr1.cc:859
    break;

  case 10:
#line 120 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 676 "src/parser.cpp" // lalr1.cc:859
    break;

  case 11:
#line 121 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 682 "src/parser.cpp" // lalr1.cc:859
    break;

  case 12:
#line 122 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 688 "src/parser.cpp" // lalr1.cc:859
    break;

  case 13:
#line 123 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 694 "src/parser.cpp" // lalr1.cc:859
    break;

  case 14:
#line 124 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 700 "src/parser.cpp" // lalr1.cc:859
    break;

  case 15:
#line 125 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 706 "src/parser.cpp" // lalr1.cc:859
    break;

  case 16:
#line 126 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 712 "src/parser.cpp" // lalr1.cc:859
    break;

  case 17:
#line 127 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 718 "src/parser.cpp" // lalr1.cc:859
    break;

  case 18:
#line 128 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 724 "src/parser.cpp" // lalr1.cc:859
    break;

  case 19:
#line 131 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (Node*)lextxt;}
#line 730 "src/parser.cpp" // lalr1.cc:859
    break;

  case 20:
#line 134 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (Node*)lextxt;}
#line 736 "src/parser.cpp" // lalr1.cc:859
    break;

  case 21:
#line 137 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkIntLitNode(lextxt);}
#line 742 "src/parser.cpp" // lalr1.cc:859
    break;

  case 22:
#line 140 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFltLitNode(lextxt);}
#line 748 "src/parser.cpp" // lalr1.cc:859
    break;

  case 23:
#line 143 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkStrLitNode(lextxt);}
#line 754 "src/parser.cpp" // lalr1.cc:859
    break;

  case 24:
#line 146 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I8,  (char*)"");}
#line 760 "src/parser.cpp" // lalr1.cc:859
    break;

  case 25:
#line 147 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I16, (char*)"");}
#line 766 "src/parser.cpp" // lalr1.cc:859
    break;

  case 26:
#line 148 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I32, (char*)"");}
#line 772 "src/parser.cpp" // lalr1.cc:859
    break;

  case 27:
#line 149 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I64, (char*)"");}
#line 778 "src/parser.cpp" // lalr1.cc:859
    break;

  case 28:
#line 150 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U8,  (char*)"");}
#line 784 "src/parser.cpp" // lalr1.cc:859
    break;

  case 29:
#line 151 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U16, (char*)"");}
#line 790 "src/parser.cpp" // lalr1.cc:859
    break;

  case 30:
#line 152 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U32, (char*)"");}
#line 796 "src/parser.cpp" // lalr1.cc:859
    break;

  case 31:
#line 153 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U64, (char*)"");}
#line 802 "src/parser.cpp" // lalr1.cc:859
    break;

  case 32:
#line 154 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Isz, (char*)"");}
#line 808 "src/parser.cpp" // lalr1.cc:859
    break;

  case 33:
#line 155 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Usz, (char*)"");}
#line 814 "src/parser.cpp" // lalr1.cc:859
    break;

  case 34:
#line 156 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_F16, (char*)"");}
#line 820 "src/parser.cpp" // lalr1.cc:859
    break;

  case 35:
#line 157 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_F32, (char*)"");}
#line 826 "src/parser.cpp" // lalr1.cc:859
    break;

  case 36:
#line 158 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_F64, (char*)"");}
#line 832 "src/parser.cpp" // lalr1.cc:859
    break;

  case 37:
#line 159 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_C8,  (char*)"");}
#line 838 "src/parser.cpp" // lalr1.cc:859
    break;

  case 38:
#line 160 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_C32, (char*)"");}
#line 844 "src/parser.cpp" // lalr1.cc:859
    break;

  case 39:
#line 161 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Bool, (char*)"");}
#line 850 "src/parser.cpp" // lalr1.cc:859
    break;

  case 40:
#line 162 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Void, (char*)"");}
#line 856 "src/parser.cpp" // lalr1.cc:859
    break;

  case 41:
#line 163 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_UserType, (char*)(yystack_[0].value));}
#line 862 "src/parser.cpp" // lalr1.cc:859
    break;

  case 42:
#line 164 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Ident, (char*)(yystack_[0].value));}
#line 868 "src/parser.cpp" // lalr1.cc:859
    break;

  case 43:
#line 167 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('*', (char*)"", (yystack_[1].value));}
#line 874 "src/parser.cpp" // lalr1.cc:859
    break;

  case 44:
#line 168 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('[', (char*)"", (yystack_[3].value));}
#line 880 "src/parser.cpp" // lalr1.cc:859
    break;

  case 45:
#line 169 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('(', (char*)"", (yystack_[3].value));}
#line 886 "src/parser.cpp" // lalr1.cc:859
    break;

  case 46:
#line 170 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('(', (char*)"", (yystack_[2].value));}
#line 892 "src/parser.cpp" // lalr1.cc:859
    break;

  case 47:
#line 171 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[2].value);}
#line 898 "src/parser.cpp" // lalr1.cc:859
    break;

  case 48:
#line 172 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 904 "src/parser.cpp" // lalr1.cc:859
    break;

  case 49:
#line 175 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 910 "src/parser.cpp" // lalr1.cc:859
    break;

  case 51:
#line 177 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 916 "src/parser.cpp" // lalr1.cc:859
    break;

  case 52:
#line 180 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 922 "src/parser.cpp" // lalr1.cc:859
    break;

  case 53:
#line 183 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pub);}
#line 928 "src/parser.cpp" // lalr1.cc:859
    break;

  case 54:
#line 184 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pri);}
#line 934 "src/parser.cpp" // lalr1.cc:859
    break;

  case 55:
#line 185 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pro);}
#line 940 "src/parser.cpp" // lalr1.cc:859
    break;

  case 56:
#line 186 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Raw);}
#line 946 "src/parser.cpp" // lalr1.cc:859
    break;

  case 57:
#line 187 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Const);}
#line 952 "src/parser.cpp" // lalr1.cc:859
    break;

  case 58:
#line 188 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Ext);}
#line 958 "src/parser.cpp" // lalr1.cc:859
    break;

  case 59:
#line 189 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Noinit);}
#line 964 "src/parser.cpp" // lalr1.cc:859
    break;

  case 60:
#line 190 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pathogen);}
#line 970 "src/parser.cpp" // lalr1.cc:859
    break;

  case 61:
#line 193 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[1].value), (yystack_[0].value));}
#line 976 "src/parser.cpp" // lalr1.cc:859
    break;

  case 62:
#line 194 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 982 "src/parser.cpp" // lalr1.cc:859
    break;

  case 63:
#line 197 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 988 "src/parser.cpp" // lalr1.cc:859
    break;

  case 64:
#line 201 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[2].value), (yystack_[4].value), (yystack_[3].value), (yystack_[0].value));}
#line 994 "src/parser.cpp" // lalr1.cc:859
    break;

  case 65:
#line 202 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[0].value), (yystack_[2].value), (yystack_[1].value),  0);}
#line 1000 "src/parser.cpp" // lalr1.cc:859
    break;

  case 66:
#line 203 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[2].value), 0,  (yystack_[3].value), (yystack_[0].value));}
#line 1006 "src/parser.cpp" // lalr1.cc:859
    break;

  case 67:
#line 204 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[0].value), 0,  (yystack_[1].value),  0);}
#line 1012 "src/parser.cpp" // lalr1.cc:859
    break;

  case 68:
#line 207 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[3].value), (yystack_[4].value), (yystack_[3].value), (yystack_[0].value));}
#line 1018 "src/parser.cpp" // lalr1.cc:859
    break;

  case 69:
#line 208 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[3].value), (yystack_[3].value), 0,  (yystack_[0].value));}
#line 1024 "src/parser.cpp" // lalr1.cc:859
    break;

  case 70:
#line 209 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[2].value), 0,  (yystack_[3].value), (yystack_[0].value));}
#line 1030 "src/parser.cpp" // lalr1.cc:859
    break;

  case 71:
#line 210 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[2].value), 0,  0,  (yystack_[0].value));}
#line 1036 "src/parser.cpp" // lalr1.cc:859
    break;

  case 72:
#line 215 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarAssignNode((yystack_[2].value), (yystack_[0].value));}
#line 1042 "src/parser.cpp" // lalr1.cc:859
    break;

  case 73:
#line 218 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 1048 "src/parser.cpp" // lalr1.cc:859
    break;

  case 74:
#line 219 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 1054 "src/parser.cpp" // lalr1.cc:859
    break;

  case 75:
#line 222 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1060 "src/parser.cpp" // lalr1.cc:859
    break;

  case 76:
#line 225 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[1].value), (yystack_[0].value));}
#line 1066 "src/parser.cpp" // lalr1.cc:859
    break;

  case 77:
#line 226 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[2].value), (yystack_[0].value));}
#line 1072 "src/parser.cpp" // lalr1.cc:859
    break;

  case 78:
#line 227 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[1].value), (yystack_[0].value));}
#line 1078 "src/parser.cpp" // lalr1.cc:859
    break;

  case 79:
#line 228 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[2].value), (yystack_[0].value));}
#line 1084 "src/parser.cpp" // lalr1.cc:859
    break;

  case 91:
#line 253 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1090 "src/parser.cpp" // lalr1.cc:859
    break;

  case 92:
#line 254 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1096 "src/parser.cpp" // lalr1.cc:859
    break;

  case 93:
#line 255 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1102 "src/parser.cpp" // lalr1.cc:859
    break;

  case 94:
#line 256 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1108 "src/parser.cpp" // lalr1.cc:859
    break;

  case 95:
#line 259 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1114 "src/parser.cpp" // lalr1.cc:859
    break;

  case 96:
#line 262 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[3].value), mkNamedValNode((char*)(yystack_[0].value), (yystack_[1].value)));}
#line 1120 "src/parser.cpp" // lalr1.cc:859
    break;

  case 97:
#line 263 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkNamedValNode((char*)(yystack_[0].value), (yystack_[1].value)));}
#line 1126 "src/parser.cpp" // lalr1.cc:859
    break;

  case 98:
#line 266 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1132 "src/parser.cpp" // lalr1.cc:859
    break;

  case 99:
#line 267 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1138 "src/parser.cpp" // lalr1.cc:859
    break;

  case 100:
#line 270 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[3].value), (yystack_[5].value), (yystack_[4].value), (yystack_[1].value), (yystack_[0].value));}
#line 1144 "src/parser.cpp" // lalr1.cc:859
    break;

  case 101:
#line 271 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[6].value), (yystack_[8].value), (yystack_[7].value), (yystack_[1].value), (yystack_[0].value));}
#line 1150 "src/parser.cpp" // lalr1.cc:859
    break;

  case 102:
#line 272 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[3].value), 0,  (yystack_[4].value), (yystack_[1].value), (yystack_[0].value));}
#line 1156 "src/parser.cpp" // lalr1.cc:859
    break;

  case 103:
#line 273 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[6].value), 0,  (yystack_[7].value), (yystack_[1].value), (yystack_[0].value));}
#line 1162 "src/parser.cpp" // lalr1.cc:859
    break;

  case 104:
#line 276 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncCallNode((char*)(yystack_[3].value), (yystack_[1].value));}
#line 1168 "src/parser.cpp" // lalr1.cc:859
    break;

  case 105:
#line 279 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkRetNode((yystack_[0].value));}
#line 1174 "src/parser.cpp" // lalr1.cc:859
    break;

  case 106:
#line 282 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setElse((IfNode*)(yystack_[3].value), (IfNode*)mkIfNode((yystack_[1].value), (yystack_[0].value)));}
#line 1180 "src/parser.cpp" // lalr1.cc:859
    break;

  case 107:
#line 283 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkIfNode((yystack_[1].value), (yystack_[0].value)));}
#line 1186 "src/parser.cpp" // lalr1.cc:859
    break;

  case 108:
#line 286 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setElse((IfNode*)(yystack_[2].value), (IfNode*)mkIfNode(NULL, (yystack_[0].value)));}
#line 1192 "src/parser.cpp" // lalr1.cc:859
    break;

  case 109:
#line 287 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1198 "src/parser.cpp" // lalr1.cc:859
    break;

  case 110:
#line 288 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkIfNode(NULL, (yystack_[0].value)));}
#line 1204 "src/parser.cpp" // lalr1.cc:859
    break;

  case 111:
#line 289 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(NULL);}
#line 1210 "src/parser.cpp" // lalr1.cc:859
    break;

  case 112:
#line 292 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkIfNode((yystack_[2].value), (yystack_[1].value), (IfNode*)getRoot());}
#line 1216 "src/parser.cpp" // lalr1.cc:859
    break;

  case 113:
#line 295 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1222 "src/parser.cpp" // lalr1.cc:859
    break;

  case 114:
#line 298 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1228 "src/parser.cpp" // lalr1.cc:859
    break;

  case 115:
#line 301 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1234 "src/parser.cpp" // lalr1.cc:859
    break;

  case 116:
#line 304 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarNode((char*)(yystack_[3].value));}
#line 1240 "src/parser.cpp" // lalr1.cc:859
    break;

  case 117:
#line 305 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarNode((char*)(yystack_[0].value));}
#line 1246 "src/parser.cpp" // lalr1.cc:859
    break;

  case 118:
#line 308 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1252 "src/parser.cpp" // lalr1.cc:859
    break;

  case 119:
#line 309 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1258 "src/parser.cpp" // lalr1.cc:859
    break;

  case 120:
#line 310 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1264 "src/parser.cpp" // lalr1.cc:859
    break;

  case 121:
#line 311 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1270 "src/parser.cpp" // lalr1.cc:859
    break;

  case 122:
#line 312 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1276 "src/parser.cpp" // lalr1.cc:859
    break;

  case 123:
#line 313 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1282 "src/parser.cpp" // lalr1.cc:859
    break;

  case 124:
#line 314 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1288 "src/parser.cpp" // lalr1.cc:859
    break;

  case 125:
#line 315 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBoolLitNode(1);}
#line 1294 "src/parser.cpp" // lalr1.cc:859
    break;

  case 126:
#line 316 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBoolLitNode(0);}
#line 1300 "src/parser.cpp" // lalr1.cc:859
    break;

  case 127:
#line 319 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1306 "src/parser.cpp" // lalr1.cc:859
    break;

  case 128:
#line 320 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1312 "src/parser.cpp" // lalr1.cc:859
    break;

  case 129:
#line 323 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1318 "src/parser.cpp" // lalr1.cc:859
    break;

  case 130:
#line 325 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 1324 "src/parser.cpp" // lalr1.cc:859
    break;

  case 131:
#line 326 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 1330 "src/parser.cpp" // lalr1.cc:859
    break;

  case 132:
#line 330 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('+', (yystack_[2].value), (yystack_[0].value));}
#line 1336 "src/parser.cpp" // lalr1.cc:859
    break;

  case 133:
#line 331 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('-', (yystack_[2].value), (yystack_[0].value));}
#line 1342 "src/parser.cpp" // lalr1.cc:859
    break;

  case 134:
#line 332 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('*', (yystack_[2].value), (yystack_[0].value));}
#line 1348 "src/parser.cpp" // lalr1.cc:859
    break;

  case 135:
#line 333 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('/', (yystack_[2].value), (yystack_[0].value));}
#line 1354 "src/parser.cpp" // lalr1.cc:859
    break;

  case 136:
#line 334 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('%', (yystack_[2].value), (yystack_[0].value));}
#line 1360 "src/parser.cpp" // lalr1.cc:859
    break;

  case 137:
#line 335 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('<', (yystack_[2].value), (yystack_[0].value));}
#line 1366 "src/parser.cpp" // lalr1.cc:859
    break;

  case 138:
#line 336 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('>', (yystack_[2].value), (yystack_[0].value));}
#line 1372 "src/parser.cpp" // lalr1.cc:859
    break;

  case 139:
#line 337 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('^', (yystack_[2].value), (yystack_[0].value));}
#line 1378 "src/parser.cpp" // lalr1.cc:859
    break;

  case 140:
#line 338 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('.', (yystack_[2].value), (yystack_[0].value));}
#line 1384 "src/parser.cpp" // lalr1.cc:859
    break;

  case 141:
#line 339 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Eq, (yystack_[2].value), (yystack_[0].value));}
#line 1390 "src/parser.cpp" // lalr1.cc:859
    break;

  case 142:
#line 340 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_NotEq, (yystack_[2].value), (yystack_[0].value));}
#line 1396 "src/parser.cpp" // lalr1.cc:859
    break;

  case 143:
#line 341 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_GrtrEq, (yystack_[2].value), (yystack_[0].value));}
#line 1402 "src/parser.cpp" // lalr1.cc:859
    break;

  case 144:
#line 342 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_LesrEq, (yystack_[2].value), (yystack_[0].value));}
#line 1408 "src/parser.cpp" // lalr1.cc:859
    break;

  case 145:
#line 343 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Or, (yystack_[2].value), (yystack_[0].value));}
#line 1414 "src/parser.cpp" // lalr1.cc:859
    break;

  case 146:
#line 344 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_And, (yystack_[2].value), (yystack_[0].value));}
#line 1420 "src/parser.cpp" // lalr1.cc:859
    break;

  case 147:
#line 345 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1426 "src/parser.cpp" // lalr1.cc:859
    break;

  case 148:
#line 349 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1432 "src/parser.cpp" // lalr1.cc:859
    break;

  case 149:
#line 352 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[3].value), (yystack_[0].value));}
#line 1438 "src/parser.cpp" // lalr1.cc:859
    break;

  case 150:
#line 353 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 1444 "src/parser.cpp" // lalr1.cc:859
    break;

  case 151:
#line 354 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1450 "src/parser.cpp" // lalr1.cc:859
    break;

  case 152:
#line 357 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('+', (yystack_[3].value), (yystack_[0].value));}
#line 1456 "src/parser.cpp" // lalr1.cc:859
    break;

  case 153:
#line 358 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('-', (yystack_[3].value), (yystack_[0].value));}
#line 1462 "src/parser.cpp" // lalr1.cc:859
    break;

  case 154:
#line 359 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('*', (yystack_[3].value), (yystack_[0].value));}
#line 1468 "src/parser.cpp" // lalr1.cc:859
    break;

  case 155:
#line 360 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('/', (yystack_[3].value), (yystack_[0].value));}
#line 1474 "src/parser.cpp" // lalr1.cc:859
    break;

  case 156:
#line 361 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('%', (yystack_[3].value), (yystack_[0].value));}
#line 1480 "src/parser.cpp" // lalr1.cc:859
    break;

  case 157:
#line 362 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('<', (yystack_[3].value), (yystack_[0].value));}
#line 1486 "src/parser.cpp" // lalr1.cc:859
    break;

  case 158:
#line 363 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('>', (yystack_[3].value), (yystack_[0].value));}
#line 1492 "src/parser.cpp" // lalr1.cc:859
    break;

  case 159:
#line 364 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('^', (yystack_[3].value), (yystack_[0].value));}
#line 1498 "src/parser.cpp" // lalr1.cc:859
    break;

  case 160:
#line 365 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('.', (yystack_[3].value), (yystack_[0].value));}
#line 1504 "src/parser.cpp" // lalr1.cc:859
    break;

  case 161:
#line 366 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Eq, (yystack_[3].value), (yystack_[0].value));}
#line 1510 "src/parser.cpp" // lalr1.cc:859
    break;

  case 162:
#line 367 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_NotEq, (yystack_[3].value), (yystack_[0].value));}
#line 1516 "src/parser.cpp" // lalr1.cc:859
    break;

  case 163:
#line 368 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_GrtrEq, (yystack_[3].value), (yystack_[0].value));}
#line 1522 "src/parser.cpp" // lalr1.cc:859
    break;

  case 164:
#line 369 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_LesrEq, (yystack_[3].value), (yystack_[0].value));}
#line 1528 "src/parser.cpp" // lalr1.cc:859
    break;

  case 165:
#line 370 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Or, (yystack_[3].value), (yystack_[0].value));}
#line 1534 "src/parser.cpp" // lalr1.cc:859
    break;

  case 166:
#line 371 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_And, (yystack_[3].value), (yystack_[0].value));}
#line 1540 "src/parser.cpp" // lalr1.cc:859
    break;

  case 167:
#line 372 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1546 "src/parser.cpp" // lalr1.cc:859
    break;


#line 1550 "src/parser.cpp" // lalr1.cc:859
            default:
              break;
            }
        }
      catch (const syntax_error& yyexc)
        {
          error (yyexc);
          YYERROR;
        }
      YY_SYMBOL_PRINT ("-> $$ =", yylhs);
      yypop_ (yylen);
      yylen = 0;
      YY_STACK_PRINT ();

      // Shift the result of the reduction.
      yypush_ (YY_NULLPTR, yylhs);
    }
    goto yynewstate;

  /*--------------------------------------.
  | yyerrlab -- here on detecting error.  |
  `--------------------------------------*/
  yyerrlab:
    // If not already recovering from an error, report this error.
    if (!yyerrstatus_)
      {
        ++yynerrs_;
        error (yyla.location, yysyntax_error_ (yystack_[0].state, yyla));
      }


    yyerror_range[1].location = yyla.location;
    if (yyerrstatus_ == 3)
      {
        /* If just tried and failed to reuse lookahead token after an
           error, discard it.  */

        // Return failure if at end of input.
        if (yyla.type_get () == yyeof_)
          YYABORT;
        else if (!yyla.empty ())
          {
            yy_destroy_ ("Error: discarding", yyla);
            yyla.clear ();
          }
      }

    // Else will try to reuse lookahead token after shifting the error token.
    goto yyerrlab1;


  /*---------------------------------------------------.
  | yyerrorlab -- error raised explicitly by YYERROR.  |
  `---------------------------------------------------*/
  yyerrorlab:

    /* Pacify compilers like GCC when the user code never invokes
       YYERROR and the label yyerrorlab therefore never appears in user
       code.  */
    if (false)
      goto yyerrorlab;
    yyerror_range[1].location = yystack_[yylen - 1].location;
    /* Do not reclaim the symbols of the rule whose action triggered
       this YYERROR.  */
    yypop_ (yylen);
    yylen = 0;
    goto yyerrlab1;

  /*-------------------------------------------------------------.
  | yyerrlab1 -- common code for both syntax error and YYERROR.  |
  `-------------------------------------------------------------*/
  yyerrlab1:
    yyerrstatus_ = 3;   // Each real token shifted decrements this.
    {
      stack_symbol_type error_token;
      for (;;)
        {
          yyn = yypact_[yystack_[0].state];
          if (!yy_pact_value_is_default_ (yyn))
            {
              yyn += yyterror_;
              if (0 <= yyn && yyn <= yylast_ && yycheck_[yyn] == yyterror_)
                {
                  yyn = yytable_[yyn];
                  if (0 < yyn)
                    break;
                }
            }

          // Pop the current state because it cannot handle the error token.
          if (yystack_.size () == 1)
            YYABORT;

          yyerror_range[1].location = yystack_[0].location;
          yy_destroy_ ("Error: popping", yystack_[0]);
          yypop_ ();
          YY_STACK_PRINT ();
        }

      yyerror_range[2].location = yyla.location;
      YYLLOC_DEFAULT (error_token.location, yyerror_range, 2);

      // Shift the error token.
      error_token.state = yyn;
      yypush_ ("Shifting", error_token);
    }
    goto yynewstate;

    // Accept.
  yyacceptlab:
    yyresult = 0;
    goto yyreturn;

    // Abort.
  yyabortlab:
    yyresult = 1;
    goto yyreturn;

  yyreturn:
    if (!yyla.empty ())
      yy_destroy_ ("Cleanup: discarding lookahead", yyla);

    /* Do not reclaim the symbols of the rule whose action triggered
       this YYABORT or YYACCEPT.  */
    yypop_ (yylen);
    while (1 < yystack_.size ())
      {
        yy_destroy_ ("Cleanup: popping", yystack_[0]);
        yypop_ ();
      }

    return yyresult;
  }
    catch (...)
      {
        YYCDEBUG << "Exception caught: cleaning lookahead and stack"
                 << std::endl;
        // Do not try to display the values of the reclaimed symbols,
        // as their printer might throw an exception.
        if (!yyla.empty ())
          yy_destroy_ (YY_NULLPTR, yyla);

        while (1 < yystack_.size ())
          {
            yy_destroy_ (YY_NULLPTR, yystack_[0]);
            yypop_ ();
          }
        throw;
      }
  }

  void
  parser::error (const syntax_error& yyexc)
  {
    error (yyexc.location, yyexc.what());
  }

  // Generate an error message.
  std::string
  parser::yysyntax_error_ (state_type yystate, const symbol_type& yyla) const
  {
    // Number of reported tokens (one for the "unexpected", one per
    // "expected").
    size_t yycount = 0;
    // Its maximum.
    enum { YYERROR_VERBOSE_ARGS_MAXIMUM = 5 };
    // Arguments of yyformat.
    char const *yyarg[YYERROR_VERBOSE_ARGS_MAXIMUM];

    /* There are many possibilities here to consider:
       - If this state is a consistent state with a default action, then
         the only way this function was invoked is if the default action
         is an error action.  In that case, don't check for expected
         tokens because there are none.
       - The only way there can be no lookahead present (in yyla) is
         if this state is a consistent state with a default action.
         Thus, detecting the absence of a lookahead is sufficient to
         determine that there is no unexpected or expected token to
         report.  In that case, just report a simple "syntax error".
       - Don't assume there isn't a lookahead just because this state is
         a consistent state with a default action.  There might have
         been a previous inconsistent state, consistent state with a
         non-default action, or user semantic action that manipulated
         yyla.  (However, yyla is currently not documented for users.)
       - Of course, the expected token list depends on states to have
         correct lookahead information, and it depends on the parser not
         to perform extra reductions after fetching a lookahead from the
         scanner and before detecting a syntax error.  Thus, state
         merging (from LALR or IELR) and default reductions corrupt the
         expected token list.  However, the list is correct for
         canonical LR with one exception: it will still contain any
         token that will not be accepted due to an error action in a
         later state.
    */
    if (!yyla.empty ())
      {
        int yytoken = yyla.type_get ();
        yyarg[yycount++] = yytname_[yytoken];
        int yyn = yypact_[yystate];
        if (!yy_pact_value_is_default_ (yyn))
          {
            /* Start YYX at -YYN if negative to avoid negative indexes in
               YYCHECK.  In other words, skip the first -YYN actions for
               this state because they are default actions.  */
            int yyxbegin = yyn < 0 ? -yyn : 0;
            // Stay within bounds of both yycheck and yytname.
            int yychecklim = yylast_ - yyn + 1;
            int yyxend = yychecklim < yyntokens_ ? yychecklim : yyntokens_;
            for (int yyx = yyxbegin; yyx < yyxend; ++yyx)
              if (yycheck_[yyx + yyn] == yyx && yyx != yyterror_
                  && !yy_table_value_is_error_ (yytable_[yyx + yyn]))
                {
                  if (yycount == YYERROR_VERBOSE_ARGS_MAXIMUM)
                    {
                      yycount = 1;
                      break;
                    }
                  else
                    yyarg[yycount++] = yytname_[yyx];
                }
          }
      }

    char const* yyformat = YY_NULLPTR;
    switch (yycount)
      {
#define YYCASE_(N, S)                         \
        case N:                               \
          yyformat = S;                       \
        break
        YYCASE_(0, YY_("syntax error"));
        YYCASE_(1, YY_("syntax error, unexpected %s"));
        YYCASE_(2, YY_("syntax error, unexpected %s, expecting %s"));
        YYCASE_(3, YY_("syntax error, unexpected %s, expecting %s or %s"));
        YYCASE_(4, YY_("syntax error, unexpected %s, expecting %s or %s or %s"));
        YYCASE_(5, YY_("syntax error, unexpected %s, expecting %s or %s or %s or %s"));
#undef YYCASE_
      }

    std::string yyres;
    // Argument number.
    size_t yyi = 0;
    for (char const* yyp = yyformat; *yyp; ++yyp)
      if (yyp[0] == '%' && yyp[1] == 's' && yyi < yycount)
        {
          yyres += yytnamerr_ (yyarg[yyi++]);
          ++yyp;
        }
      else
        yyres += *yyp;
    return yyres;
  }


  const short int parser::yypact_ninf_ = -209;

  const signed char parser::yytable_ninf_ = -118;

  const short int
  parser::yypact_[] =
  {
     -57,  -209,    28,   371,  -209,  -209,  -209,  -209,  -209,  -209,
    -209,  -209,  -209,  -209,  -209,  -209,  -209,  -209,  -209,  -209,
    -209,  -209,  -209,  -209,    18,    18,   477,    18,   -48,   477,
      35,    14,  -209,  -209,  -209,  -209,  -209,  -209,  -209,  -209,
     572,   -57,  -209,   -36,  -209,  -209,   -38,   -46,    29,  -209,
      17,   553,  -209,  -209,  -209,  -209,  -209,  -209,  -209,  -209,
    -209,  -209,  -209,  -209,   -34,  -209,  -209,  -209,  -209,  -209,
      18,    18,    34,  -209,  -209,  -209,  -209,  -209,  -209,  -209,
     -12,   583,   -48,  -209,    29,   572,    33,   -48,   371,    25,
      32,    29,   572,   -40,    35,    38,  -209,    53,   371,    18,
      18,  -209,   339,    18,   572,   572,   -21,  -209,    35,    14,
      29,    18,  -209,    40,    -9,   593,    55,    18,    18,    18,
      18,    18,    18,    18,    18,    18,    18,    18,    18,    18,
      18,    18,    18,    78,    59,    29,    18,  -209,   -20,    18,
      18,    61,    65,    29,   450,    35,    73,  -209,    74,    20,
    -209,  -209,  -209,    77,  -209,    81,  -209,   103,   107,   -38,
     -38,    18,    18,   572,   -40,    38,  -209,   -18,  -209,  -209,
    -209,   -57,   -57,   -57,   -57,   -57,   -57,   -57,   -57,   -57,
     -57,   -57,   -57,   -57,   -57,   -57,   -57,  -209,   583,    79,
      79,    79,    79,   604,   187,    79,    79,   106,   106,    45,
      45,    45,    45,    82,    18,   -48,    85,  -209,   116,   -48,
    -209,   371,  -209,  -209,    18,    18,   117,    29,   135,  -209,
      26,  -209,  -209,    23,  -209,    18,    35,  -209,  -209,  -209,
    -209,  -209,   108,  -209,    29,   122,   -48,    73,  -209,  -209,
      18,    18,   572,    18,    18,    18,    18,    18,    18,    18,
      18,    18,    18,    18,    18,    18,    18,    18,    18,   -48,
    -209,    18,   -48,  -209,  -209,  -209,    18,  -209,   450,  -209,
      35,  -209,  -209,   120,   127,  -209,   572,  -209,  -209,   123,
    -209,   -48,   583,   121,   121,   121,   121,   614,   525,   121,
     121,   159,   159,    52,    52,    52,    52,   128,  -209,   -48,
    -209,  -209,  -209,  -209,    18,   572,    29,   129,  -209,  -209,
    -209,   -48,  -209,   572,  -209,   -48,  -209
  };

  const unsigned char
  parser::yydefact_[] =
  {
       6,     5,     0,     0,     1,    19,    20,    24,    25,    26,
      27,    28,    29,    30,    31,    32,    33,    34,    35,    36,
      37,    38,    39,    40,     0,     0,     0,     0,     0,     0,
       0,     0,    53,    54,    55,    56,    57,    58,    59,    60,
       0,     6,     4,    42,    41,    48,    51,    52,     0,    62,
      63,     0,     7,    18,     8,    11,    17,     9,    10,    12,
      16,    13,    14,    15,     0,   125,   126,    21,    22,    23,
       0,     0,   117,   122,   123,   124,   118,   121,   147,   105,
     129,   131,     0,    42,     0,     0,     0,     0,     0,     0,
      42,     0,     0,     0,     0,     0,    94,     0,     2,   128,
       0,    43,     0,   128,     0,     0,    67,    61,     0,     0,
       0,     0,   167,     0,   148,   150,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,   111,    67,     0,     0,   113,     6,     0,
       0,     0,    42,     0,     0,     0,     0,    78,    89,     0,
      92,    47,     3,     0,   127,     0,    46,     0,     0,    49,
      50,   128,     0,    99,     0,     0,    93,    65,    72,   120,
     151,     6,     6,     6,     6,     6,     6,     6,     6,     6,
       6,     6,     6,     6,     6,     6,     6,   119,   130,   141,
     142,   143,   144,   145,   146,   137,   138,   132,   133,   134,
     135,   136,   139,   140,     0,     0,   109,   112,    65,     0,
      95,     0,   114,    71,     0,     0,     0,    81,     0,    84,
       0,    82,    74,     0,    79,     0,     0,    90,   104,   116,
      45,    44,     0,    66,     0,    98,     0,     0,    76,    91,
     128,     0,    99,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
     110,     0,     0,   115,    70,    69,     0,    80,     0,    85,
       0,    75,    88,    86,     0,    97,     0,   102,    77,     0,
      64,     0,   149,   161,   162,   163,   164,   165,   166,   157,
     158,   152,   153,   154,   155,   156,   159,   160,   107,     0,
     108,    68,    83,    73,     0,    99,     0,     0,   100,   106,
      87,     0,    96,    99,   103,     0,   101
  };

  const short int
  parser::yypgoto_[] =
  {
    -209,  -209,   118,    -8,   -95,    -3,     0,  -209,  -209,  -209,
    -209,    42,  -209,   -24,   167,  -209,   -25,   193,  -209,  -209,
    -209,    56,  -209,   -47,  -209,  -126,  -209,   -85,  -135,   -74,
    -209,  -208,  -209,    12,  -209,  -209,  -209,  -209,  -209,  -209,
    -209,    16,    67,   -97,   -13,  -209,   209,  -209,  -209,   149
  };

  const short int
  parser::yydefgoto_[] =
  {
      -1,     2,    41,     3,    42,    72,    44,    73,    74,    75,
      45,    46,    47,    48,    49,    50,    51,    52,    53,    54,
     223,   146,    55,   219,   220,   147,   149,    96,    56,    89,
     235,   236,    57,    76,    59,   206,   207,    60,    61,    62,
      63,    77,    78,   153,   154,    80,    81,   113,   114,   115
  };

  const short int
  parser::yytable_[] =
  {
      43,    85,    84,   152,    92,    91,   158,     1,   133,   221,
     150,    79,    82,   137,    87,    58,    97,    88,     6,    64,
     224,     5,   104,    83,   166,   144,    90,   110,     4,   145,
      93,    95,     5,    98,   281,   101,   105,    83,   238,     6,
     102,   103,    99,   100,     1,   106,   210,  -117,    83,   111,
      65,    66,    67,    68,    69,   170,   117,   161,   116,   171,
     240,   135,   162,   163,   232,   241,   242,   139,   143,    32,
      33,    34,    35,    36,    37,    38,    39,   136,   157,    94,
     239,   134,    83,    70,   226,    43,   227,   155,   141,   142,
     268,   270,   269,   271,   148,    43,    71,   311,   168,    83,
      58,    83,    83,    94,    64,   315,   169,   167,   164,   165,
      58,   278,    99,   100,    64,   140,   152,   204,   205,   218,
     217,   131,   132,   209,   261,   262,   212,   213,   185,   186,
     211,   260,   208,   221,   151,   263,   187,   112,   144,   234,
     216,    83,   162,   279,   214,   222,   159,   160,   215,   233,
     126,   127,   128,   129,   130,   131,   132,   225,   228,   132,
      83,   229,   277,   243,   244,   245,   246,   247,   248,   249,
     250,   251,   252,   253,   254,   255,   256,   257,   258,   128,
     129,   130,   131,   132,   230,   298,   109,   231,   300,   274,
     276,   259,   180,   181,   182,   183,   184,   185,   186,   241,
     266,   264,   265,   304,   307,   186,   138,   308,    43,   118,
     119,   305,   272,   313,   267,   120,   121,   107,   234,    86,
     237,   302,     0,    58,     0,   309,   273,    64,   280,     0,
       0,   275,   182,   183,   184,   185,   186,   314,     0,    83,
       0,   316,     0,   218,   217,     0,     0,     0,   299,     0,
       0,     0,   306,   301,     0,     0,   124,   125,   126,   127,
     128,   129,   130,   131,   132,    83,     0,     0,     0,     0,
     303,     0,     0,    83,     0,     0,     0,     0,     0,     0,
       0,   234,     0,     0,     0,     0,     0,     0,     0,   234,
       0,   310,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,    83,   312,     0,     0,     0,     0,     0,     0,
      83,   112,   112,   112,   112,   112,   112,   112,   112,   112,
     112,   112,   112,   112,   112,   112,   188,   189,   190,   191,
     192,   193,   194,   195,   196,   197,   198,   199,   200,   201,
     202,   203,     5,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    21,    22,
      23,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    22,    23,   283,   284,   285,   286,   287,   288,   289,
     290,   291,   292,   293,   294,   295,   296,   297,    24,    25,
       0,     0,    26,    27,    28,     0,     0,    40,     0,    29,
     156,    30,    31,    32,    33,    34,    35,    36,    37,    38,
      39,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,    40,
       0,     0,   282,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
      22,    23,     0,     0,     0,     0,     0,     0,     0,     0,
       5,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    22,    23,     0,
       0,    31,    32,    33,    34,    35,    36,    37,    38,    39,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,    40,    32,
      33,    34,    35,    36,    37,    38,    39,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,   172,   173,     0,
       0,     0,     0,   174,   175,    40,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    22,    23,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    22,    23,   178,   179,   180,   181,   182,   183,
     184,   185,   186,   108,   109,   118,   119,     0,     0,     0,
       0,   120,   121,   122,   123,   172,   173,     0,     0,     0,
       0,   174,   175,   176,   177,     0,   118,   119,     0,     0,
       0,    40,   120,   121,     0,   123,   172,   173,     0,     0,
       0,     0,   174,   175,     0,   177,     0,     0,     0,     0,
      40,     0,   124,   125,   126,   127,   128,   129,   130,   131,
     132,     0,   178,   179,   180,   181,   182,   183,   184,   185,
     186,     0,     0,   124,   125,   126,   127,   128,   129,   130,
     131,   132,     0,   178,   179,   180,   181,   182,   183,   184,
     185,   186
  };

  const short int
  parser::yycheck_[] =
  {
       3,    26,    26,    98,    29,    29,   103,    64,    82,   144,
      95,    24,    25,    87,    27,     3,    40,    65,     4,     3,
     146,     3,    68,    26,   109,    65,    29,    51,     0,    69,
      30,    31,     3,    41,   242,    73,    82,    40,   164,     4,
      78,    79,    78,    79,    64,    48,    66,    83,    51,    83,
      32,    33,    34,    35,    36,    64,    68,    78,    71,    68,
      78,    85,    83,    84,   161,    83,    84,    42,    92,    52,
      53,    54,    55,    56,    57,    58,    59,    44,   102,    65,
     165,    84,    85,    65,    64,    88,    66,   100,    91,    92,
      64,    68,    66,    70,    94,    98,    78,   305,   111,   102,
      88,   104,   105,    65,    88,   313,    66,   110,   108,   109,
      98,   237,    78,    79,    98,    83,   211,    39,    40,   144,
     144,    76,    77,   136,    39,    40,   139,   140,    76,    77,
     138,   205,   135,   268,    81,   209,    81,    70,    65,   163,
     143,   144,    83,   240,    83,   145,   104,   105,    83,   162,
      71,    72,    73,    74,    75,    76,    77,    83,    81,    77,
     163,    80,   236,   171,   172,   173,   174,   175,   176,   177,
     178,   179,   180,   181,   182,   183,   184,   185,   186,    73,
      74,    75,    76,    77,    81,   259,    51,    80,   262,    81,
      68,   204,    71,    72,    73,    74,    75,    76,    77,    83,
      83,   214,   215,    83,    81,    77,    88,   281,   211,    22,
      23,    84,   225,    84,   217,    28,    29,    50,   242,    26,
     164,   268,    -1,   211,    -1,   299,   226,   211,   241,    -1,
      -1,   234,    73,    74,    75,    76,    77,   311,    -1,   242,
      -1,   315,    -1,   268,   268,    -1,    -1,    -1,   261,    -1,
      -1,    -1,   276,   266,    -1,    -1,    69,    70,    71,    72,
      73,    74,    75,    76,    77,   268,    -1,    -1,    -1,    -1,
     270,    -1,    -1,   276,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,   305,    -1,    -1,    -1,    -1,    -1,    -1,    -1,   313,
      -1,   304,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,   305,   306,    -1,    -1,    -1,    -1,    -1,    -1,
     313,   244,   245,   246,   247,   248,   249,   250,   251,   252,
     253,   254,   255,   256,   257,   258,   117,   118,   119,   120,
     121,   122,   123,   124,   125,   126,   127,   128,   129,   130,
     131,   132,     3,     4,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,     3,     4,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,   244,   245,   246,   247,   248,   249,   250,
     251,   252,   253,   254,   255,   256,   257,   258,    37,    38,
      -1,    -1,    41,    42,    43,    -1,    -1,    78,    -1,    48,
      81,    50,    51,    52,    53,    54,    55,    56,    57,    58,
      59,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    78,
      -1,    -1,   243,     3,     4,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
       3,     4,     5,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    21,    -1,
      -1,    51,    52,    53,    54,    55,    56,    57,    58,    59,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    78,    52,
      53,    54,    55,    56,    57,    58,    59,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    22,    23,    -1,
      -1,    -1,    -1,    28,    29,    78,     3,     4,     5,     6,
       7,     8,     9,    10,    11,    12,    13,    14,    15,    16,
      17,    18,    19,    20,    21,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    69,    70,    71,    72,    73,    74,
      75,    76,    77,    50,    51,    22,    23,    -1,    -1,    -1,
      -1,    28,    29,    30,    31,    22,    23,    -1,    -1,    -1,
      -1,    28,    29,    30,    31,    -1,    22,    23,    -1,    -1,
      -1,    78,    28,    29,    -1,    31,    22,    23,    -1,    -1,
      -1,    -1,    28,    29,    -1,    31,    -1,    -1,    -1,    -1,
      78,    -1,    69,    70,    71,    72,    73,    74,    75,    76,
      77,    -1,    69,    70,    71,    72,    73,    74,    75,    76,
      77,    -1,    -1,    69,    70,    71,    72,    73,    74,    75,
      76,    77,    -1,    69,    70,    71,    72,    73,    74,    75,
      76,    77
  };

  const unsigned char
  parser::yystos_[] =
  {
       0,    64,    86,    88,     0,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    37,    38,    41,    42,    43,    48,
      50,    51,    52,    53,    54,    55,    56,    57,    58,    59,
      78,    87,    89,    90,    91,    95,    96,    97,    98,    99,
     100,   101,   102,   103,   104,   107,   113,   117,   118,   119,
     122,   123,   124,   125,   126,    32,    33,    34,    35,    36,
      65,    78,    90,    92,    93,    94,   118,   126,   127,   129,
     130,   131,   129,    90,    98,   101,   102,   129,    65,   114,
      90,    98,   101,    91,    65,    91,   112,    98,    88,    78,
      79,    73,    78,    79,    68,    82,    90,    99,    50,    51,
      98,    83,   127,   132,   133,   134,   129,    68,    22,    23,
      28,    29,    30,    31,    69,    70,    71,    72,    73,    74,
      75,    76,    77,   114,    90,    98,    44,   114,    87,    42,
      83,    90,    90,    98,    65,    69,   106,   110,    91,   111,
     112,    81,    89,   128,   129,   129,    81,    98,   128,    96,
      96,    78,    83,    84,    91,    91,   112,    90,   129,    66,
      64,    68,    22,    23,    28,    29,    30,    31,    69,    70,
      71,    72,    73,    74,    75,    76,    77,    81,   131,   131,
     131,   131,   131,   131,   131,   131,   131,   131,   131,   131,
     131,   131,   131,   131,    39,    40,   120,   121,    90,   129,
      66,    88,   129,   129,    83,    83,    90,    98,   101,   108,
     109,   113,    91,   105,   110,    83,    64,    66,    81,    80,
      81,    80,   128,   129,    98,   115,   116,   106,   110,   112,
      78,    83,    84,    88,    88,    88,    88,    88,    88,    88,
      88,    88,    88,    88,    88,    88,    88,    88,    88,   129,
     114,    39,    40,   114,   129,   129,    83,    90,    64,    66,
      68,    70,   129,    91,    81,    90,    68,   114,   110,   128,
     129,   116,   131,   134,   134,   134,   134,   134,   134,   134,
     134,   134,   134,   134,   134,   134,   134,   134,   114,   129,
     114,   129,   108,    91,    83,    84,    98,    81,   114,   114,
     129,   116,    90,    84,   114,   116,   114
  };

  const unsigned char
  parser::yyr1_[] =
  {
       0,    85,    86,    87,    87,    88,    88,    89,    89,    89,
      89,    89,    89,    89,    89,    89,    89,    89,    89,    90,
      91,    92,    93,    94,    95,    95,    95,    95,    95,    95,
      95,    95,    95,    95,    95,    95,    95,    95,    95,    95,
      95,    95,    95,    96,    96,    96,    96,    96,    96,    97,
      97,    97,    98,    99,    99,    99,    99,    99,    99,    99,
      99,   100,   100,   101,   102,   102,   102,   102,   103,   103,
     103,   103,   104,   105,   105,   106,   107,   107,   107,   107,
     108,   108,   108,   109,   109,   110,   111,   111,   111,   111,
     112,   113,   113,   113,   113,   114,   115,   115,   116,   116,
     117,   117,   117,   117,   118,   119,   120,   120,   121,   121,
     121,   121,   122,   123,   124,   125,   126,   126,   127,   127,
     127,   127,   127,   127,   127,   127,   127,   128,   128,   129,
     130,   130,   131,   131,   131,   131,   131,   131,   131,   131,
     131,   131,   131,   131,   131,   131,   131,   131,   132,   133,
     133,   133,   134,   134,   134,   134,   134,   134,   134,   134,
     134,   134,   134,   134,   134,   134,   134,   134
  };

  const unsigned char
  parser::yyr2_[] =
  {
       0,     2,     3,     3,     1,     1,     0,     1,     1,     1,
       1,     1,     1,     1,     1,     1,     1,     1,     1,     1,
       1,     1,     1,     1,     1,     1,     1,     1,     1,     1,
       1,     1,     1,     1,     1,     1,     1,     1,     1,     1,
       1,     1,     1,     2,     4,     4,     3,     3,     1,     3,
       3,     1,     1,     1,     1,     1,     1,     1,     1,     1,
       1,     2,     1,     1,     5,     3,     4,     2,     6,     5,
       5,     4,     3,     3,     1,     3,     4,     5,     3,     4,
       2,     1,     1,     3,     1,     3,     3,     5,     3,     1,
       3,     4,     3,     3,     2,     3,     4,     2,     1,     0,
       6,     9,     5,     8,     4,     2,     4,     3,     3,     1,
       2,     0,     4,     3,     4,     5,     4,     1,     1,     3,
       3,     1,     1,     1,     1,     1,     1,     1,     0,     1,
       3,     1,     3,     3,     3,     3,     3,     3,     3,     3,
       3,     3,     3,     3,     3,     3,     3,     1,     1,     4,
       1,     2,     4,     4,     4,     4,     4,     4,     4,     4,
       4,     4,     4,     4,     4,     4,     4,     1
  };



  // YYTNAME[SYMBOL-NUM] -- String name of the symbol SYMBOL-NUM.
  // First, the terminals, then, starting at \a yyntokens_, nonterminals.
  const char*
  const parser::yytname_[] =
  {
  "$end", "error", "$undefined", "Ident", "UserType", "I8", "I16", "I32",
  "I64", "U8", "U16", "U32", "U64", "Isz", "Usz", "F16", "F32", "F64",
  "C8", "C32", "Bool", "Void", "Eq", "NotEq", "AddEq", "SubEq", "MulEq",
  "DivEq", "GrtrEq", "LesrEq", "Or", "And", "True", "False", "IntLit",
  "FltLit", "StrLit", "Return", "If", "Elif", "Else", "For", "While", "Do",
  "In", "Continue", "Break", "Import", "Let", "Match", "Data", "Enum",
  "Pub", "Pri", "Pro", "Raw", "Const", "Ext", "Noinit", "Pathogen",
  "Where", "Infect", "Cleanse", "Ct", "Newline", "Indent", "Unindent",
  "LOW", "','", "'<'", "'>'", "'+'", "'-'", "'*'", "'/'", "'%'", "'^'",
  "'.'", "'('", "'['", "']'", "')'", "'|'", "'='", "':'", "$accept",
  "top_level_stmt_list", "stmt_list", "maybe_newline", "stmt", "ident",
  "usertype", "intlit", "fltlit", "strlit", "lit_type", "type",
  "type_expr_", "type_expr", "modifier", "modifier_list_", "modifier_list",
  "var_decl", "let_binding", "var_assign", "usertype_list", "generic",
  "data_decl", "type_decl", "type_decl_list", "type_decl_block",
  "val_init_list", "enum_block", "enum_decl", "block", "params",
  "maybe_params", "fn_decl", "fn_call", "ret_stmt", "elif_list",
  "maybe_elif_list", "if_stmt", "while_loop", "do_while_loop", "for_loop",
  "var", "val", "maybe_expr", "expr", "expr_list", "expr_p", "nl_expr",
  "nl_expr_list", "nl_expr_p", YY_NULLPTR
  };

#if YYDEBUG
  const unsigned short int
  parser::yyrline_[] =
  {
       0,   106,   106,   109,   110,   113,   114,   117,   118,   119,
     120,   121,   122,   123,   124,   125,   126,   127,   128,   131,
     134,   137,   140,   143,   146,   147,   148,   149,   150,   151,
     152,   153,   154,   155,   156,   157,   158,   159,   160,   161,
     162,   163,   164,   167,   168,   169,   170,   171,   172,   175,
     176,   177,   180,   183,   184,   185,   186,   187,   188,   189,
     190,   193,   194,   197,   201,   202,   203,   204,   207,   208,
     209,   210,   215,   218,   219,   222,   225,   226,   227,   228,
     231,   232,   233,   236,   237,   240,   244,   245,   246,   247,
     250,   253,   254,   255,   256,   259,   262,   263,   266,   267,
     270,   271,   272,   273,   276,   279,   282,   283,   286,   287,
     288,   289,   292,   295,   298,   301,   304,   305,   308,   309,
     310,   311,   312,   313,   314,   315,   316,   319,   320,   323,
     325,   326,   330,   331,   332,   333,   334,   335,   336,   337,
     338,   339,   340,   341,   342,   343,   344,   345,   349,   352,
     353,   354,   357,   358,   359,   360,   361,   362,   363,   364,
     365,   366,   367,   368,   369,   370,   371,   372
  };

  // Print the state stack on the debug stream.
  void
  parser::yystack_print_ ()
  {
    *yycdebug_ << "Stack now";
    for (stack_type::const_iterator
           i = yystack_.begin (),
           i_end = yystack_.end ();
         i != i_end; ++i)
      *yycdebug_ << ' ' << i->state;
    *yycdebug_ << std::endl;
  }

  // Report on the debug stream that the rule \a yyrule is going to be reduced.
  void
  parser::yy_reduce_print_ (int yyrule)
  {
    unsigned int yylno = yyrline_[yyrule];
    int yynrhs = yyr2_[yyrule];
    // Print the symbols being reduced, and their result.
    *yycdebug_ << "Reducing stack by rule " << yyrule - 1
               << " (line " << yylno << "):" << std::endl;
    // The symbols being reduced.
    for (int yyi = 0; yyi < yynrhs; yyi++)
      YY_SYMBOL_PRINT ("   $" << yyi + 1 << " =",
                       yystack_[(yynrhs) - (yyi + 1)]);
  }
#endif // YYDEBUG

  // Symbol number corresponding to token number t.
  inline
  parser::token_number_type
  parser::yytranslate_ (int t)
  {
    static
    const token_number_type
    translate_table[] =
    {
     0,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,    75,     2,     2,
      78,    81,    73,    71,    68,    72,    77,    74,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,    84,     2,
      69,    83,    70,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,    79,     2,    80,    76,     2,     2,     2,     2,     2,
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
      65,    66,    67
    };
    const unsigned int user_token_number_max_ = 322;
    const token_number_type undef_token_ = 2;

    if (static_cast<int>(t) <= yyeof_)
      return yyeof_;
    else if (static_cast<unsigned int> (t) <= user_token_number_max_)
      return translate_table[t];
    else
      return undef_token_;
  }


} // yy
#line 2271 "src/parser.cpp" // lalr1.cc:1167
#line 375 "src/syntax.y" // lalr1.cc:1168


void yy::parser::error(const location& loc, const string& msg){
    ante::error(msg.c_str(), yylexer->fileName, yylexer->getRow(), yylexer->getCol());
}

#endif
