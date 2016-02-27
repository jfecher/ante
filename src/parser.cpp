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


#line 66 "src/parser.cpp" // lalr1.cc:404

# ifndef YY_NULLPTR
#  if defined __cplusplus && 201103L <= __cplusplus
#   define YY_NULLPTR nullptr
#  else
#   define YY_NULLPTR 0
#  endif
# endif

#include "yyparser.h"

// User implementation prologue.

#line 80 "src/parser.cpp" // lalr1.cc:412


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
#line 166 "src/parser.cpp" // lalr1.cc:479

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
#line 107 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 644 "src/parser.cpp" // lalr1.cc:859
    break;

  case 4:
#line 108 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 650 "src/parser.cpp" // lalr1.cc:859
    break;

  case 7:
#line 115 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 656 "src/parser.cpp" // lalr1.cc:859
    break;

  case 8:
#line 116 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 662 "src/parser.cpp" // lalr1.cc:859
    break;

  case 9:
#line 117 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 668 "src/parser.cpp" // lalr1.cc:859
    break;

  case 10:
#line 118 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 674 "src/parser.cpp" // lalr1.cc:859
    break;

  case 11:
#line 119 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 680 "src/parser.cpp" // lalr1.cc:859
    break;

  case 12:
#line 120 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 686 "src/parser.cpp" // lalr1.cc:859
    break;

  case 13:
#line 121 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 692 "src/parser.cpp" // lalr1.cc:859
    break;

  case 14:
#line 122 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 698 "src/parser.cpp" // lalr1.cc:859
    break;

  case 15:
#line 123 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 704 "src/parser.cpp" // lalr1.cc:859
    break;

  case 16:
#line 124 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 710 "src/parser.cpp" // lalr1.cc:859
    break;

  case 17:
#line 125 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 716 "src/parser.cpp" // lalr1.cc:859
    break;

  case 18:
#line 126 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 722 "src/parser.cpp" // lalr1.cc:859
    break;

  case 19:
#line 129 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (Node*)lextxt;}
#line 728 "src/parser.cpp" // lalr1.cc:859
    break;

  case 20:
#line 132 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (Node*)lextxt;}
#line 734 "src/parser.cpp" // lalr1.cc:859
    break;

  case 21:
#line 135 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkIntLitNode(lextxt);}
#line 740 "src/parser.cpp" // lalr1.cc:859
    break;

  case 22:
#line 138 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFltLitNode(lextxt);}
#line 746 "src/parser.cpp" // lalr1.cc:859
    break;

  case 23:
#line 141 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkStrLitNode(lextxt);}
#line 752 "src/parser.cpp" // lalr1.cc:859
    break;

  case 24:
#line 144 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I8,  (char*)"");}
#line 758 "src/parser.cpp" // lalr1.cc:859
    break;

  case 25:
#line 145 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I16, (char*)"");}
#line 764 "src/parser.cpp" // lalr1.cc:859
    break;

  case 26:
#line 146 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I32, (char*)"");}
#line 770 "src/parser.cpp" // lalr1.cc:859
    break;

  case 27:
#line 147 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I64, (char*)"");}
#line 776 "src/parser.cpp" // lalr1.cc:859
    break;

  case 28:
#line 148 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U8,  (char*)"");}
#line 782 "src/parser.cpp" // lalr1.cc:859
    break;

  case 29:
#line 149 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U16, (char*)"");}
#line 788 "src/parser.cpp" // lalr1.cc:859
    break;

  case 30:
#line 150 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U32, (char*)"");}
#line 794 "src/parser.cpp" // lalr1.cc:859
    break;

  case 31:
#line 151 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U64, (char*)"");}
#line 800 "src/parser.cpp" // lalr1.cc:859
    break;

  case 32:
#line 152 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Isz, (char*)"");}
#line 806 "src/parser.cpp" // lalr1.cc:859
    break;

  case 33:
#line 153 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Usz, (char*)"");}
#line 812 "src/parser.cpp" // lalr1.cc:859
    break;

  case 34:
#line 154 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_F16, (char*)"");}
#line 818 "src/parser.cpp" // lalr1.cc:859
    break;

  case 35:
#line 155 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_F32, (char*)"");}
#line 824 "src/parser.cpp" // lalr1.cc:859
    break;

  case 36:
#line 156 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_F64, (char*)"");}
#line 830 "src/parser.cpp" // lalr1.cc:859
    break;

  case 37:
#line 157 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_C8,  (char*)"");}
#line 836 "src/parser.cpp" // lalr1.cc:859
    break;

  case 38:
#line 158 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_C32, (char*)"");}
#line 842 "src/parser.cpp" // lalr1.cc:859
    break;

  case 39:
#line 159 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Bool, (char*)"");}
#line 848 "src/parser.cpp" // lalr1.cc:859
    break;

  case 40:
#line 160 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Void, (char*)"");}
#line 854 "src/parser.cpp" // lalr1.cc:859
    break;

  case 41:
#line 161 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_UserType, (char*)(yystack_[0].value));}
#line 860 "src/parser.cpp" // lalr1.cc:859
    break;

  case 42:
#line 162 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Ident, (char*)(yystack_[0].value));}
#line 866 "src/parser.cpp" // lalr1.cc:859
    break;

  case 43:
#line 165 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('*', (char*)"", (yystack_[1].value));}
#line 872 "src/parser.cpp" // lalr1.cc:859
    break;

  case 44:
#line 166 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('[', (char*)"", (yystack_[3].value));}
#line 878 "src/parser.cpp" // lalr1.cc:859
    break;

  case 45:
#line 167 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('(', (char*)"", (yystack_[3].value));}
#line 884 "src/parser.cpp" // lalr1.cc:859
    break;

  case 46:
#line 168 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('(', (char*)"", (yystack_[2].value));}
#line 890 "src/parser.cpp" // lalr1.cc:859
    break;

  case 47:
#line 169 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[2].value);}
#line 896 "src/parser.cpp" // lalr1.cc:859
    break;

  case 48:
#line 170 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 902 "src/parser.cpp" // lalr1.cc:859
    break;

  case 49:
#line 173 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 908 "src/parser.cpp" // lalr1.cc:859
    break;

  case 51:
#line 175 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 914 "src/parser.cpp" // lalr1.cc:859
    break;

  case 52:
#line 178 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 920 "src/parser.cpp" // lalr1.cc:859
    break;

  case 53:
#line 181 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pub);}
#line 926 "src/parser.cpp" // lalr1.cc:859
    break;

  case 54:
#line 182 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pri);}
#line 932 "src/parser.cpp" // lalr1.cc:859
    break;

  case 55:
#line 183 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pro);}
#line 938 "src/parser.cpp" // lalr1.cc:859
    break;

  case 56:
#line 184 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Raw);}
#line 944 "src/parser.cpp" // lalr1.cc:859
    break;

  case 57:
#line 185 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Const);}
#line 950 "src/parser.cpp" // lalr1.cc:859
    break;

  case 58:
#line 186 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Ext);}
#line 956 "src/parser.cpp" // lalr1.cc:859
    break;

  case 59:
#line 187 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Noinit);}
#line 962 "src/parser.cpp" // lalr1.cc:859
    break;

  case 60:
#line 188 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pathogen);}
#line 968 "src/parser.cpp" // lalr1.cc:859
    break;

  case 61:
#line 191 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[1].value), (yystack_[0].value));}
#line 974 "src/parser.cpp" // lalr1.cc:859
    break;

  case 62:
#line 192 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 980 "src/parser.cpp" // lalr1.cc:859
    break;

  case 63:
#line 195 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 986 "src/parser.cpp" // lalr1.cc:859
    break;

  case 64:
#line 199 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[2].value), (yystack_[4].value), (yystack_[3].value), (yystack_[0].value));}
#line 992 "src/parser.cpp" // lalr1.cc:859
    break;

  case 65:
#line 200 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[0].value), (yystack_[2].value), (yystack_[1].value),  0);}
#line 998 "src/parser.cpp" // lalr1.cc:859
    break;

  case 66:
#line 201 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[2].value), 0,  (yystack_[3].value), (yystack_[0].value));}
#line 1004 "src/parser.cpp" // lalr1.cc:859
    break;

  case 67:
#line 202 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[0].value), 0,  (yystack_[1].value),  0);}
#line 1010 "src/parser.cpp" // lalr1.cc:859
    break;

  case 68:
#line 205 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[3].value), (yystack_[4].value), (yystack_[3].value), (yystack_[0].value));}
#line 1016 "src/parser.cpp" // lalr1.cc:859
    break;

  case 69:
#line 206 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[3].value), (yystack_[3].value), 0,  (yystack_[0].value));}
#line 1022 "src/parser.cpp" // lalr1.cc:859
    break;

  case 70:
#line 207 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[2].value), 0,  (yystack_[3].value), (yystack_[0].value));}
#line 1028 "src/parser.cpp" // lalr1.cc:859
    break;

  case 71:
#line 208 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[2].value), 0,  0,  (yystack_[0].value));}
#line 1034 "src/parser.cpp" // lalr1.cc:859
    break;

  case 72:
#line 213 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarAssignNode((yystack_[2].value), (yystack_[0].value));}
#line 1040 "src/parser.cpp" // lalr1.cc:859
    break;

  case 73:
#line 216 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 1046 "src/parser.cpp" // lalr1.cc:859
    break;

  case 74:
#line 217 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 1052 "src/parser.cpp" // lalr1.cc:859
    break;

  case 75:
#line 220 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1058 "src/parser.cpp" // lalr1.cc:859
    break;

  case 76:
#line 223 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[1].value), (yystack_[0].value));}
#line 1064 "src/parser.cpp" // lalr1.cc:859
    break;

  case 77:
#line 224 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[2].value), (yystack_[0].value));}
#line 1070 "src/parser.cpp" // lalr1.cc:859
    break;

  case 78:
#line 225 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[1].value), (yystack_[0].value));}
#line 1076 "src/parser.cpp" // lalr1.cc:859
    break;

  case 79:
#line 226 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[2].value), (yystack_[0].value));}
#line 1082 "src/parser.cpp" // lalr1.cc:859
    break;

  case 91:
#line 251 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1088 "src/parser.cpp" // lalr1.cc:859
    break;

  case 92:
#line 252 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1094 "src/parser.cpp" // lalr1.cc:859
    break;

  case 93:
#line 253 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1100 "src/parser.cpp" // lalr1.cc:859
    break;

  case 94:
#line 254 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1106 "src/parser.cpp" // lalr1.cc:859
    break;

  case 95:
#line 257 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1112 "src/parser.cpp" // lalr1.cc:859
    break;

  case 96:
#line 260 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[1].value), mkVarNode((char*)(yystack_[0].value)));}
#line 1118 "src/parser.cpp" // lalr1.cc:859
    break;

  case 97:
#line 261 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkVarNode((char*)(yystack_[0].value)));}
#line 1124 "src/parser.cpp" // lalr1.cc:859
    break;

  case 98:
#line 269 "src/syntax.y" // lalr1.cc:859
    {setNext((yystack_[3].value), mkNamedValNode(getRoot(), (yystack_[1].value))); (yylhs.value) = (yystack_[0].value);}
#line 1130 "src/parser.cpp" // lalr1.cc:859
    break;

  case 99:
#line 270 "src/syntax.y" // lalr1.cc:859
    {setRoot(mkNamedValNode(getRoot(), (yystack_[1].value))); (yylhs.value) = (yystack_[0].value);}
#line 1136 "src/parser.cpp" // lalr1.cc:859
    break;

  case 100:
#line 273 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1142 "src/parser.cpp" // lalr1.cc:859
    break;

  case 101:
#line 274 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1148 "src/parser.cpp" // lalr1.cc:859
    break;

  case 102:
#line 277 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[3].value), (yystack_[5].value), (yystack_[4].value), (yystack_[1].value), (yystack_[0].value));}
#line 1154 "src/parser.cpp" // lalr1.cc:859
    break;

  case 103:
#line 278 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[6].value), (yystack_[8].value), (yystack_[7].value), (yystack_[1].value), (yystack_[0].value));}
#line 1160 "src/parser.cpp" // lalr1.cc:859
    break;

  case 104:
#line 279 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[3].value), 0,  (yystack_[4].value), (yystack_[1].value), (yystack_[0].value));}
#line 1166 "src/parser.cpp" // lalr1.cc:859
    break;

  case 105:
#line 280 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[6].value), 0,  (yystack_[7].value), (yystack_[1].value), (yystack_[0].value));}
#line 1172 "src/parser.cpp" // lalr1.cc:859
    break;

  case 106:
#line 283 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncCallNode((char*)(yystack_[3].value), (yystack_[1].value));}
#line 1178 "src/parser.cpp" // lalr1.cc:859
    break;

  case 107:
#line 286 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkRetNode((yystack_[0].value));}
#line 1184 "src/parser.cpp" // lalr1.cc:859
    break;

  case 108:
#line 289 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setElse((IfNode*)(yystack_[3].value), (IfNode*)mkIfNode((yystack_[1].value), (yystack_[0].value)));}
#line 1190 "src/parser.cpp" // lalr1.cc:859
    break;

  case 109:
#line 290 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkIfNode((yystack_[1].value), (yystack_[0].value)));}
#line 1196 "src/parser.cpp" // lalr1.cc:859
    break;

  case 110:
#line 293 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setElse((IfNode*)(yystack_[2].value), (IfNode*)mkIfNode(NULL, (yystack_[0].value)));}
#line 1202 "src/parser.cpp" // lalr1.cc:859
    break;

  case 111:
#line 294 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1208 "src/parser.cpp" // lalr1.cc:859
    break;

  case 112:
#line 295 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkIfNode(NULL, (yystack_[0].value)));}
#line 1214 "src/parser.cpp" // lalr1.cc:859
    break;

  case 113:
#line 296 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(NULL);}
#line 1220 "src/parser.cpp" // lalr1.cc:859
    break;

  case 114:
#line 299 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkIfNode((yystack_[2].value), (yystack_[1].value), (IfNode*)getRoot());}
#line 1226 "src/parser.cpp" // lalr1.cc:859
    break;

  case 115:
#line 302 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1232 "src/parser.cpp" // lalr1.cc:859
    break;

  case 116:
#line 305 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1238 "src/parser.cpp" // lalr1.cc:859
    break;

  case 117:
#line 308 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1244 "src/parser.cpp" // lalr1.cc:859
    break;

  case 119:
#line 312 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarNode((char*)(yystack_[0].value));}
#line 1250 "src/parser.cpp" // lalr1.cc:859
    break;

  case 120:
#line 315 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('&', (yystack_[0].value));}
#line 1256 "src/parser.cpp" // lalr1.cc:859
    break;

  case 121:
#line 316 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('@', (yystack_[0].value));}
#line 1262 "src/parser.cpp" // lalr1.cc:859
    break;

  case 123:
#line 318 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkRefVarNode((char*)(yystack_[0].value));}
#line 1268 "src/parser.cpp" // lalr1.cc:859
    break;

  case 124:
#line 321 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1274 "src/parser.cpp" // lalr1.cc:859
    break;

  case 125:
#line 322 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1280 "src/parser.cpp" // lalr1.cc:859
    break;

  case 126:
#line 323 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1286 "src/parser.cpp" // lalr1.cc:859
    break;

  case 127:
#line 324 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1292 "src/parser.cpp" // lalr1.cc:859
    break;

  case 128:
#line 325 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1298 "src/parser.cpp" // lalr1.cc:859
    break;

  case 129:
#line 326 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1304 "src/parser.cpp" // lalr1.cc:859
    break;

  case 130:
#line 327 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1310 "src/parser.cpp" // lalr1.cc:859
    break;

  case 131:
#line 328 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1316 "src/parser.cpp" // lalr1.cc:859
    break;

  case 132:
#line 329 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBoolLitNode(1);}
#line 1322 "src/parser.cpp" // lalr1.cc:859
    break;

  case 133:
#line 330 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBoolLitNode(0);}
#line 1328 "src/parser.cpp" // lalr1.cc:859
    break;

  case 134:
#line 333 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1334 "src/parser.cpp" // lalr1.cc:859
    break;

  case 135:
#line 334 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1340 "src/parser.cpp" // lalr1.cc:859
    break;

  case 136:
#line 337 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1346 "src/parser.cpp" // lalr1.cc:859
    break;

  case 137:
#line 339 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 1352 "src/parser.cpp" // lalr1.cc:859
    break;

  case 138:
#line 340 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 1358 "src/parser.cpp" // lalr1.cc:859
    break;

  case 139:
#line 343 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('@', (yystack_[0].value));}
#line 1364 "src/parser.cpp" // lalr1.cc:859
    break;

  case 140:
#line 344 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('&', (yystack_[0].value));}
#line 1370 "src/parser.cpp" // lalr1.cc:859
    break;

  case 141:
#line 345 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('-', (yystack_[0].value));}
#line 1376 "src/parser.cpp" // lalr1.cc:859
    break;

  case 142:
#line 348 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('+', (yystack_[2].value), (yystack_[0].value));}
#line 1382 "src/parser.cpp" // lalr1.cc:859
    break;

  case 143:
#line 349 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('-', (yystack_[2].value), (yystack_[0].value));}
#line 1388 "src/parser.cpp" // lalr1.cc:859
    break;

  case 144:
#line 350 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('*', (yystack_[2].value), (yystack_[0].value));}
#line 1394 "src/parser.cpp" // lalr1.cc:859
    break;

  case 145:
#line 351 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('/', (yystack_[2].value), (yystack_[0].value));}
#line 1400 "src/parser.cpp" // lalr1.cc:859
    break;

  case 146:
#line 352 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('%', (yystack_[2].value), (yystack_[0].value));}
#line 1406 "src/parser.cpp" // lalr1.cc:859
    break;

  case 147:
#line 353 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('<', (yystack_[2].value), (yystack_[0].value));}
#line 1412 "src/parser.cpp" // lalr1.cc:859
    break;

  case 148:
#line 354 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('>', (yystack_[2].value), (yystack_[0].value));}
#line 1418 "src/parser.cpp" // lalr1.cc:859
    break;

  case 149:
#line 355 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('^', (yystack_[2].value), (yystack_[0].value));}
#line 1424 "src/parser.cpp" // lalr1.cc:859
    break;

  case 150:
#line 356 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('.', (yystack_[2].value), (yystack_[0].value));}
#line 1430 "src/parser.cpp" // lalr1.cc:859
    break;

  case 151:
#line 357 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Eq, (yystack_[2].value), (yystack_[0].value));}
#line 1436 "src/parser.cpp" // lalr1.cc:859
    break;

  case 152:
#line 358 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_NotEq, (yystack_[2].value), (yystack_[0].value));}
#line 1442 "src/parser.cpp" // lalr1.cc:859
    break;

  case 153:
#line 359 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_GrtrEq, (yystack_[2].value), (yystack_[0].value));}
#line 1448 "src/parser.cpp" // lalr1.cc:859
    break;

  case 154:
#line 360 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_LesrEq, (yystack_[2].value), (yystack_[0].value));}
#line 1454 "src/parser.cpp" // lalr1.cc:859
    break;

  case 155:
#line 361 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Or, (yystack_[2].value), (yystack_[0].value));}
#line 1460 "src/parser.cpp" // lalr1.cc:859
    break;

  case 156:
#line 362 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_And, (yystack_[2].value), (yystack_[0].value));}
#line 1466 "src/parser.cpp" // lalr1.cc:859
    break;

  case 157:
#line 363 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1472 "src/parser.cpp" // lalr1.cc:859
    break;

  case 158:
#line 367 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1478 "src/parser.cpp" // lalr1.cc:859
    break;

  case 159:
#line 370 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[3].value), (yystack_[0].value));}
#line 1484 "src/parser.cpp" // lalr1.cc:859
    break;

  case 160:
#line 371 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 1490 "src/parser.cpp" // lalr1.cc:859
    break;

  case 161:
#line 372 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1496 "src/parser.cpp" // lalr1.cc:859
    break;

  case 162:
#line 375 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('+', (yystack_[3].value), (yystack_[0].value));}
#line 1502 "src/parser.cpp" // lalr1.cc:859
    break;

  case 163:
#line 376 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('-', (yystack_[3].value), (yystack_[0].value));}
#line 1508 "src/parser.cpp" // lalr1.cc:859
    break;

  case 164:
#line 377 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('*', (yystack_[3].value), (yystack_[0].value));}
#line 1514 "src/parser.cpp" // lalr1.cc:859
    break;

  case 165:
#line 378 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('/', (yystack_[3].value), (yystack_[0].value));}
#line 1520 "src/parser.cpp" // lalr1.cc:859
    break;

  case 166:
#line 379 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('%', (yystack_[3].value), (yystack_[0].value));}
#line 1526 "src/parser.cpp" // lalr1.cc:859
    break;

  case 167:
#line 380 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('<', (yystack_[3].value), (yystack_[0].value));}
#line 1532 "src/parser.cpp" // lalr1.cc:859
    break;

  case 168:
#line 381 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('>', (yystack_[3].value), (yystack_[0].value));}
#line 1538 "src/parser.cpp" // lalr1.cc:859
    break;

  case 169:
#line 382 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('^', (yystack_[3].value), (yystack_[0].value));}
#line 1544 "src/parser.cpp" // lalr1.cc:859
    break;

  case 170:
#line 383 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('.', (yystack_[3].value), (yystack_[0].value));}
#line 1550 "src/parser.cpp" // lalr1.cc:859
    break;

  case 171:
#line 384 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Eq, (yystack_[3].value), (yystack_[0].value));}
#line 1556 "src/parser.cpp" // lalr1.cc:859
    break;

  case 172:
#line 385 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_NotEq, (yystack_[3].value), (yystack_[0].value));}
#line 1562 "src/parser.cpp" // lalr1.cc:859
    break;

  case 173:
#line 386 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_GrtrEq, (yystack_[3].value), (yystack_[0].value));}
#line 1568 "src/parser.cpp" // lalr1.cc:859
    break;

  case 174:
#line 387 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_LesrEq, (yystack_[3].value), (yystack_[0].value));}
#line 1574 "src/parser.cpp" // lalr1.cc:859
    break;

  case 175:
#line 388 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Or, (yystack_[3].value), (yystack_[0].value));}
#line 1580 "src/parser.cpp" // lalr1.cc:859
    break;

  case 176:
#line 389 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_And, (yystack_[3].value), (yystack_[0].value));}
#line 1586 "src/parser.cpp" // lalr1.cc:859
    break;

  case 177:
#line 390 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1592 "src/parser.cpp" // lalr1.cc:859
    break;


#line 1596 "src/parser.cpp" // lalr1.cc:859
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


  const short int parser::yypact_ninf_ = -241;

  const signed char parser::yytable_ninf_ = -124;

  const short int
  parser::yypact_[] =
  {
     -44,  -241,    56,   367,  -241,  -241,  -241,  -241,  -241,  -241,
    -241,  -241,  -241,  -241,  -241,  -241,  -241,  -241,  -241,  -241,
    -241,  -241,  -241,  -241,   221,   221,   558,   221,    26,   558,
      75,    17,  -241,  -241,  -241,  -241,  -241,  -241,  -241,  -241,
     578,    15,    15,   -44,  -241,   -54,  -241,  -241,   -24,   -52,
      80,  -241,     6,   502,  -241,  -241,  -241,  -241,  -241,  -241,
    -241,  -241,  -241,  -241,  -241,  -241,     9,  -241,  -241,  -241,
    -241,  -241,   221,   221,   221,   221,   221,    -5,  -241,  -241,
    -241,  -241,  -241,  -241,  -241,    21,  -241,   596,    30,  -241,
      80,   578,    55,    30,   221,    31,    80,   578,   -38,    75,
      65,  -241,    53,    58,  -241,  -241,   367,   221,   221,  -241,
     451,   221,   578,   578,   112,  -241,    75,    17,    80,   221,
    -241,    85,   -16,   609,  -241,    68,  -241,  -241,   221,   221,
     221,   221,   221,   221,   221,   221,   221,   221,   221,   221,
     221,   221,   221,   221,   221,   367,    73,    71,    80,   221,
    -241,    30,   221,    84,    88,    80,   482,    75,   135,  -241,
     125,   -32,  -241,  -241,  -241,   132,  -241,   134,  -241,   137,
     136,   -24,   -24,   221,   221,   578,   -38,    65,  -241,   115,
    -241,  -241,  -241,   -44,   -44,   -44,   -44,   -44,   -44,   -44,
     -44,   -44,   -44,   -44,   -44,   -44,   -44,   -44,   -44,  -241,
     141,   596,   -31,   -31,   -31,   -31,    47,   619,   -31,   -31,
      52,    52,    64,    64,    64,    64,  -241,   -13,   221,    30,
     108,  -241,   139,    30,  -241,  -241,   221,   221,   140,    80,
     164,  -241,    20,  -241,  -241,    36,  -241,   221,    75,  -241,
    -241,  -241,  -241,  -241,   144,  -241,    80,   159,    30,   135,
    -241,  -241,   221,   221,   578,   221,   221,   221,   221,   221,
     221,   221,   221,   221,   221,   221,   221,   221,   221,   221,
     221,  -241,  -241,   367,    30,  -241,   221,    30,  -241,  -241,
    -241,   221,  -241,   482,  -241,    75,  -241,  -241,   145,   146,
    -241,    80,   578,  -241,  -241,   150,  -241,    30,   596,    89,
      89,    89,    89,   240,   629,    89,    89,   129,   129,    93,
      93,    93,    93,  -241,  -241,    30,  -241,  -241,  -241,  -241,
     221,   578,  -241,    80,   148,  -241,  -241,  -241,    30,    80,
     578,  -241,    30,  -241
  };

  const unsigned char
  parser::yydefact_[] =
  {
       6,     5,     0,     0,     1,    19,    20,    24,    25,    26,
      27,    28,    29,    30,    31,    32,    33,    34,    35,    36,
      37,    38,    39,    40,     0,     0,     0,     0,     0,     0,
       0,     0,    53,    54,    55,    56,    57,    58,    59,    60,
       0,     0,     0,     6,     4,    42,    41,    48,    51,    52,
       0,    62,    63,     0,     7,    18,     8,    12,    16,     9,
      10,    11,    15,    13,    17,    14,     0,   132,   133,    21,
      22,    23,     0,     0,     0,     0,     0,   119,   129,   130,
     131,   124,   128,   157,   107,   136,   127,   138,     0,    42,
       0,     0,     0,     0,     0,    42,     0,     0,     0,     0,
       0,    94,     0,   123,   120,   121,     2,   135,     0,    43,
       0,   135,     0,     0,    67,    61,     0,     0,     0,     0,
     177,     0,   158,   160,   141,     0,   140,   139,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,   113,    67,     0,     0,
     115,     0,     0,     0,    42,     0,     0,     0,     0,    78,
      89,     0,    92,    47,     3,     0,   134,     0,    46,     0,
       0,    49,    50,   135,     0,   101,     0,     0,    93,    65,
      72,   126,   161,     6,     6,     6,     6,     6,     6,     6,
       6,     6,     6,     6,     6,     6,     6,     6,     6,   125,
       0,   137,   151,   152,   153,   154,   155,   156,   147,   148,
     142,   143,   144,   145,   146,   149,   150,     6,     0,     0,
     111,   114,    65,     0,   116,    71,     0,     0,     0,    81,
       0,    84,     0,    82,    74,     0,    79,     0,     0,    90,
     106,   122,    45,    44,     0,    66,     0,   100,     0,     0,
      76,    91,   135,     0,   101,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,   118,    95,     0,     0,   112,     0,     0,   117,    70,
      69,     0,    80,     0,    85,     0,    75,    88,    86,     0,
      97,    99,     0,   104,    77,     0,    64,     0,   159,   171,
     172,   173,   174,   175,   176,   167,   168,   162,   163,   164,
     165,   166,   169,   170,   109,     0,   110,    68,    83,    73,
       0,   101,    96,     0,     0,   102,   108,    87,     0,    98,
     101,   105,     0,   103
  };

  const short int
  parser::yypgoto_[] =
  {
    -241,  -241,    92,   -10,   -84,    -3,   -19,  -241,  -241,  -241,
    -241,    79,  -241,   -25,   181,  -241,   -23,   208,  -241,  -241,
    -241,    62,  -241,   -43,  -241,  -141,  -241,   -81,  -147,   -80,
     -82,  -241,  -240,  -241,    -1,  -241,  -241,  -241,  -241,  -241,
    -241,  -241,  -241,   170,    83,  -106,   -17,  -241,  -241,   225,
    -241,  -241,   133
  };

  const short int
  parser::yydefgoto_[] =
  {
      -1,     2,    43,     3,    44,    77,    46,    78,    79,    80,
      47,    48,    49,    50,    51,    52,    53,    54,    55,    56,
     235,   158,    57,   231,   232,   159,   161,   101,    58,   146,
     291,   247,   248,    59,    81,    61,   220,   221,    62,    63,
      64,    65,    82,    66,    83,   165,   166,    85,    86,    87,
     121,   122,   123
  };

  const short int
  parser::yytable_[] =
  {
      45,    90,    60,    91,    96,   170,    97,    84,    88,   233,
      93,    98,   100,   150,   297,   102,   112,   236,     5,   162,
       1,     6,   164,    89,   107,   108,    95,   156,   118,  -123,
     113,   157,   238,   106,   239,   250,   178,    89,   103,   103,
     138,   139,   140,   141,   142,   143,   144,   114,   182,   109,
      89,     1,   183,   272,   110,   111,     4,   125,    32,    33,
      34,    35,    36,    37,    38,    39,   148,   244,    94,   130,
     131,   224,   155,   107,   128,   132,   133,   151,   135,     6,
     160,   328,    99,     5,   283,   169,   284,   147,    89,   129,
     332,   167,   119,   153,   154,   145,   251,   176,   177,   149,
      41,    42,   180,    45,   285,    60,   286,    89,   294,    89,
      89,   200,   218,   219,   152,   179,   136,   137,   138,   139,
     140,   141,   142,   143,   144,   140,   141,   142,   143,   144,
      99,   229,   223,   230,   163,   225,   233,   108,   234,   275,
     143,   144,    45,   278,    60,   222,   295,   276,   277,   199,
     246,   181,   228,    89,   174,   120,   124,   245,   126,   127,
     192,   193,   194,   195,   196,   197,   198,   226,   293,   197,
     198,   227,    89,   255,   256,   257,   258,   259,   260,   261,
     262,   263,   264,   265,   266,   267,   268,   269,   270,   164,
     173,   171,   172,   252,   314,   174,   175,   316,   253,   254,
     156,   274,   194,   195,   196,   197,   198,   273,   237,   279,
     280,   104,   105,   240,   241,   117,   243,   325,   242,   288,
     287,   271,   253,   281,     5,   289,   282,   292,   320,   246,
     321,   324,   330,   115,    92,   326,   296,   217,   249,     0,
     318,   329,     0,   290,     0,     0,     0,     0,   331,     0,
       0,    89,   333,    67,    68,    69,    70,    71,   229,   315,
     230,     0,   184,   185,   317,     0,   319,   323,   186,   187,
      45,   189,    60,     0,     0,     0,     0,     0,     0,     0,
      89,     0,     0,     0,     0,     0,    72,     0,   322,    89,
       0,     0,     0,    73,     0,     0,   246,     0,     0,    74,
       0,     0,     0,   327,     0,   246,    75,    76,     0,   190,
     191,   192,   193,   194,   195,   196,   197,   198,    89,     0,
     290,     0,     0,     0,     0,     0,   322,    89,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,   120,
     120,   120,   120,   120,   120,   120,   120,   120,   120,   120,
     120,   120,   120,   120,   201,   202,   203,   204,   205,   206,
     207,   208,   209,   210,   211,   212,   213,   214,   215,   216,
       5,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    22,    23,   299,
     300,   301,   302,   303,   304,   305,   306,   307,   308,   309,
     310,   311,   312,   313,    24,    25,     0,     0,    26,    27,
      28,     0,     0,     0,     0,    29,     0,    30,    31,    32,
      33,    34,    35,    36,    37,    38,    39,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,    40,     0,     0,     0,     0,
       0,     0,    41,    42,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    22,    23,     0,     0,     0,     0,     0,     0,     0,
     298,     0,     0,     0,     0,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    22,    23,     0,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    22,    23,     0,     0,     0,     0,     0,    40,
       0,     0,   168,    31,    32,    33,    34,    35,    36,    37,
      38,    39,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,   116,   117,     0,     0,     0,     0,     0,     0,
      40,     5,     6,     7,     8,     9,    10,    11,    12,    13,
      14,    15,    16,    17,    18,    19,    20,    21,    22,    23,
      40,     5,     6,     7,     8,     9,    10,    11,    12,    13,
      14,    15,    16,    17,    18,    19,    20,    21,    22,    23,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
      32,    33,    34,    35,    36,    37,    38,    39,   130,   131,
       0,     0,     0,     0,   132,   133,   134,   135,     0,     0,
       0,   184,   185,     0,     0,     0,    40,   186,   187,   188,
     189,   130,   131,     0,     0,     0,     0,   132,   133,     0,
       0,   184,   185,     0,     0,     0,    40,   186,   187,     0,
       0,     0,     0,     0,     0,   136,   137,   138,   139,   140,
     141,   142,   143,   144,     0,     0,     0,     0,   190,   191,
     192,   193,   194,   195,   196,   197,   198,     0,   136,   137,
     138,   139,   140,   141,   142,   143,   144,     0,   190,   191,
     192,   193,   194,   195,   196,   197,   198
  };

  const short int
  parser::yycheck_[] =
  {
       3,    26,     3,    26,    29,   111,    29,    24,    25,   156,
      27,    30,    31,    93,   254,    40,    68,   158,     3,   100,
      64,     4,   106,    26,    78,    79,    29,    65,    53,    83,
      82,    69,    64,    43,    66,   176,   117,    40,    41,    42,
      71,    72,    73,    74,    75,    76,    77,    50,    64,    73,
      53,    64,    68,    66,    78,    79,     0,    74,    52,    53,
      54,    55,    56,    57,    58,    59,    91,   173,    42,    22,
      23,   151,    97,    78,    79,    28,    29,    94,    31,     4,
      99,   321,    65,     3,    64,   110,    66,    90,    91,    68,
     330,   108,    83,    96,    97,    65,   177,   116,   117,    44,
      85,    86,   119,   106,    68,   106,    70,   110,   249,   112,
     113,   128,    39,    40,    83,   118,    69,    70,    71,    72,
      73,    74,    75,    76,    77,    73,    74,    75,    76,    77,
      65,   156,   149,   156,    81,   152,   283,    79,   157,   219,
      76,    77,   145,   223,   145,   148,   252,    39,    40,    81,
     175,    66,   155,   156,    83,    72,    73,   174,    75,    76,
      71,    72,    73,    74,    75,    76,    77,    83,   248,    76,
      77,    83,   175,   183,   184,   185,   186,   187,   188,   189,
     190,   191,   192,   193,   194,   195,   196,   197,   198,   273,
      78,   112,   113,    78,   274,    83,    84,   277,    83,    84,
      65,   218,    73,    74,    75,    76,    77,   217,    83,   226,
     227,    41,    42,    81,    80,    51,    80,   297,    81,   238,
     237,    80,    83,    83,     3,    81,   229,    68,    83,   254,
      84,    81,    84,    52,    26,   315,   253,   145,   176,    -1,
     283,   323,    -1,   246,    -1,    -1,    -1,    -1,   328,    -1,
      -1,   254,   332,    32,    33,    34,    35,    36,   283,   276,
     283,    -1,    22,    23,   281,    -1,   285,   292,    28,    29,
     273,    31,   273,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
     283,    -1,    -1,    -1,    -1,    -1,    65,    -1,   291,   292,
      -1,    -1,    -1,    72,    -1,    -1,   321,    -1,    -1,    78,
      -1,    -1,    -1,   320,    -1,   330,    85,    86,    -1,    69,
      70,    71,    72,    73,    74,    75,    76,    77,   321,    -1,
     323,    -1,    -1,    -1,    -1,    -1,   329,   330,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,   256,
     257,   258,   259,   260,   261,   262,   263,   264,   265,   266,
     267,   268,   269,   270,   129,   130,   131,   132,   133,   134,
     135,   136,   137,   138,   139,   140,   141,   142,   143,   144,
       3,     4,     5,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    21,   256,
     257,   258,   259,   260,   261,   262,   263,   264,   265,   266,
     267,   268,   269,   270,    37,    38,    -1,    -1,    41,    42,
      43,    -1,    -1,    -1,    -1,    48,    -1,    50,    51,    52,
      53,    54,    55,    56,    57,    58,    59,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    78,    -1,    -1,    -1,    -1,
      -1,    -1,    85,    86,     3,     4,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
     255,    -1,    -1,    -1,    -1,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    -1,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    -1,    -1,    -1,    -1,    -1,    78,
      -1,    -1,    81,    51,    52,    53,    54,    55,    56,    57,
      58,    59,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    50,    51,    -1,    -1,    -1,    -1,    -1,    -1,
      78,     3,     4,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
      78,     3,     4,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      52,    53,    54,    55,    56,    57,    58,    59,    22,    23,
      -1,    -1,    -1,    -1,    28,    29,    30,    31,    -1,    -1,
      -1,    22,    23,    -1,    -1,    -1,    78,    28,    29,    30,
      31,    22,    23,    -1,    -1,    -1,    -1,    28,    29,    -1,
      -1,    22,    23,    -1,    -1,    -1,    78,    28,    29,    -1,
      -1,    -1,    -1,    -1,    -1,    69,    70,    71,    72,    73,
      74,    75,    76,    77,    -1,    -1,    -1,    -1,    69,    70,
      71,    72,    73,    74,    75,    76,    77,    -1,    69,    70,
      71,    72,    73,    74,    75,    76,    77,    -1,    69,    70,
      71,    72,    73,    74,    75,    76,    77
  };

  const unsigned char
  parser::yystos_[] =
  {
       0,    64,    88,    90,     0,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    37,    38,    41,    42,    43,    48,
      50,    51,    52,    53,    54,    55,    56,    57,    58,    59,
      78,    85,    86,    89,    91,    92,    93,    97,    98,    99,
     100,   101,   102,   103,   104,   105,   106,   109,   115,   120,
     121,   122,   125,   126,   127,   128,   130,    32,    33,    34,
      35,    36,    65,    72,    78,    85,    86,    92,    94,    95,
      96,   121,   129,   131,   133,   134,   135,   136,   133,    92,
     100,   103,   104,   133,    42,    92,   100,   103,    93,    65,
      93,   114,   100,    92,   130,   130,    90,    78,    79,    73,
      78,    79,    68,    82,    92,   101,    50,    51,   100,    83,
     131,   137,   138,   139,   131,   133,   131,   131,    79,    68,
      22,    23,    28,    29,    30,    31,    69,    70,    71,    72,
      73,    74,    75,    76,    77,    65,   116,    92,   100,    44,
     116,   133,    83,    92,    92,   100,    65,    69,   108,   112,
      93,   113,   114,    81,    91,   132,   133,   133,    81,   100,
     132,    98,    98,    78,    83,    84,    93,    93,   114,    92,
     133,    66,    64,    68,    22,    23,    28,    29,    30,    31,
      69,    70,    71,    72,    73,    74,    75,    76,    77,    81,
     133,   136,   136,   136,   136,   136,   136,   136,   136,   136,
     136,   136,   136,   136,   136,   136,   136,    89,    39,    40,
     123,   124,    92,   133,   116,   133,    83,    83,    92,   100,
     103,   110,   111,   115,    93,   107,   112,    83,    64,    66,
      81,    80,    81,    80,   132,   133,   100,   118,   119,   108,
     112,   114,    78,    83,    84,    90,    90,    90,    90,    90,
      90,    90,    90,    90,    90,    90,    90,    90,    90,    90,
      90,    80,    66,    90,   133,   116,    39,    40,   116,   133,
     133,    83,    92,    64,    66,    68,    70,   133,    93,    81,
      92,   117,    68,   116,   112,   132,   133,   119,   136,   139,
     139,   139,   139,   139,   139,   139,   139,   139,   139,   139,
     139,   139,   139,   139,   116,   133,   116,   133,   110,    93,
      83,    84,    92,   100,    81,   116,   116,   133,   119,   117,
      84,   116,   119,   116
  };

  const unsigned char
  parser::yyr1_[] =
  {
       0,    87,    88,    89,    89,    90,    90,    91,    91,    91,
      91,    91,    91,    91,    91,    91,    91,    91,    91,    92,
      93,    94,    95,    96,    97,    97,    97,    97,    97,    97,
      97,    97,    97,    97,    97,    97,    97,    97,    97,    97,
      97,    97,    97,    98,    98,    98,    98,    98,    98,    99,
      99,    99,   100,   101,   101,   101,   101,   101,   101,   101,
     101,   102,   102,   103,   104,   104,   104,   104,   105,   105,
     105,   105,   106,   107,   107,   108,   109,   109,   109,   109,
     110,   110,   110,   111,   111,   112,   113,   113,   113,   113,
     114,   115,   115,   115,   115,   116,   117,   117,   118,   118,
     119,   119,   120,   120,   120,   120,   121,   122,   123,   123,
     124,   124,   124,   124,   125,   126,   127,   128,   129,   129,
     130,   130,   130,   130,   131,   131,   131,   131,   131,   131,
     131,   131,   131,   131,   132,   132,   133,   134,   134,   135,
     135,   135,   136,   136,   136,   136,   136,   136,   136,   136,
     136,   136,   136,   136,   136,   136,   136,   136,   137,   138,
     138,   138,   139,   139,   139,   139,   139,   139,   139,   139,
     139,   139,   139,   139,   139,   139,   139,   139
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
       3,     4,     3,     3,     2,     3,     2,     1,     4,     2,
       1,     0,     6,     9,     5,     8,     4,     2,     4,     3,
       3,     1,     2,     0,     4,     3,     4,     5,     4,     1,
       2,     2,     4,     1,     1,     3,     3,     1,     1,     1,
       1,     1,     1,     1,     1,     0,     1,     3,     1,     2,
       2,     2,     3,     3,     3,     3,     3,     3,     3,     3,
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
  "'.'", "'('", "'['", "']'", "')'", "'|'", "'='", "':'", "'&'", "'@'",
  "$accept", "top_level_stmt_list", "stmt_list", "maybe_newline", "stmt",
  "ident", "usertype", "intlit", "fltlit", "strlit", "lit_type", "type",
  "type_expr_", "type_expr", "modifier", "modifier_list_", "modifier_list",
  "var_decl", "let_binding", "var_assign", "usertype_list", "generic",
  "data_decl", "type_decl", "type_decl_list", "type_decl_block",
  "val_init_list", "enum_block", "enum_decl", "block", "ident_list",
  "params", "maybe_params", "fn_decl", "fn_call", "ret_stmt", "elif_list",
  "maybe_elif_list", "if_stmt", "while_loop", "do_while_loop", "for_loop",
  "var", "ref_val", "val", "maybe_expr", "expr", "expr_list", "unary_op",
  "expr_p", "nl_expr", "nl_expr_list", "nl_expr_p", YY_NULLPTR
  };

#if YYDEBUG
  const unsigned short int
  parser::yyrline_[] =
  {
       0,   104,   104,   107,   108,   111,   112,   115,   116,   117,
     118,   119,   120,   121,   122,   123,   124,   125,   126,   129,
     132,   135,   138,   141,   144,   145,   146,   147,   148,   149,
     150,   151,   152,   153,   154,   155,   156,   157,   158,   159,
     160,   161,   162,   165,   166,   167,   168,   169,   170,   173,
     174,   175,   178,   181,   182,   183,   184,   185,   186,   187,
     188,   191,   192,   195,   199,   200,   201,   202,   205,   206,
     207,   208,   213,   216,   217,   220,   223,   224,   225,   226,
     229,   230,   231,   234,   235,   238,   242,   243,   244,   245,
     248,   251,   252,   253,   254,   257,   260,   261,   269,   270,
     273,   274,   277,   278,   279,   280,   283,   286,   289,   290,
     293,   294,   295,   296,   299,   302,   305,   308,   311,   312,
     315,   316,   317,   318,   321,   322,   323,   324,   325,   326,
     327,   328,   329,   330,   333,   334,   337,   339,   340,   343,
     344,   345,   348,   349,   350,   351,   352,   353,   354,   355,
     356,   357,   358,   359,   360,   361,   362,   363,   367,   370,
     371,   372,   375,   376,   377,   378,   379,   380,   381,   382,
     383,   384,   385,   386,   387,   388,   389,   390
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
       2,     2,     2,     2,     2,     2,     2,    75,    85,     2,
      78,    81,    73,    71,    68,    72,    77,    74,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,    84,     2,
      69,    83,    70,     2,    86,     2,     2,     2,     2,     2,
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
#line 2330 "src/parser.cpp" // lalr1.cc:1167
#line 393 "src/syntax.y" // lalr1.cc:1168


void yy::parser::error(const location& loc, const string& msg){
    ante::error(msg.c_str(), yylexer->fileName, yylexer->getRow(), yylexer->getCol());
}

#endif
