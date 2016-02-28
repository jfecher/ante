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
#line 108 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[1].value));}
#line 644 "src/parser.cpp" // lalr1.cc:859
    break;

  case 4:
#line 109 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[1].value), (yystack_[0].value));}
#line 650 "src/parser.cpp" // lalr1.cc:859
    break;

  case 5:
#line 110 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[1].value));}
#line 656 "src/parser.cpp" // lalr1.cc:859
    break;

  case 6:
#line 111 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 662 "src/parser.cpp" // lalr1.cc:859
    break;

  case 21:
#line 139 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (Node*)lextxt;}
#line 668 "src/parser.cpp" // lalr1.cc:859
    break;

  case 22:
#line 142 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (Node*)lextxt;}
#line 674 "src/parser.cpp" // lalr1.cc:859
    break;

  case 23:
#line 145 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkIntLitNode(lextxt);}
#line 680 "src/parser.cpp" // lalr1.cc:859
    break;

  case 24:
#line 148 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFltLitNode(lextxt);}
#line 686 "src/parser.cpp" // lalr1.cc:859
    break;

  case 25:
#line 151 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkStrLitNode(lextxt);}
#line 692 "src/parser.cpp" // lalr1.cc:859
    break;

  case 26:
#line 154 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I8,  (char*)"");}
#line 698 "src/parser.cpp" // lalr1.cc:859
    break;

  case 27:
#line 155 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I16, (char*)"");}
#line 704 "src/parser.cpp" // lalr1.cc:859
    break;

  case 28:
#line 156 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I32, (char*)"");}
#line 710 "src/parser.cpp" // lalr1.cc:859
    break;

  case 29:
#line 157 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I64, (char*)"");}
#line 716 "src/parser.cpp" // lalr1.cc:859
    break;

  case 30:
#line 158 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U8,  (char*)"");}
#line 722 "src/parser.cpp" // lalr1.cc:859
    break;

  case 31:
#line 159 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U16, (char*)"");}
#line 728 "src/parser.cpp" // lalr1.cc:859
    break;

  case 32:
#line 160 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U32, (char*)"");}
#line 734 "src/parser.cpp" // lalr1.cc:859
    break;

  case 33:
#line 161 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U64, (char*)"");}
#line 740 "src/parser.cpp" // lalr1.cc:859
    break;

  case 34:
#line 162 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Isz, (char*)"");}
#line 746 "src/parser.cpp" // lalr1.cc:859
    break;

  case 35:
#line 163 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Usz, (char*)"");}
#line 752 "src/parser.cpp" // lalr1.cc:859
    break;

  case 36:
#line 164 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_F16, (char*)"");}
#line 758 "src/parser.cpp" // lalr1.cc:859
    break;

  case 37:
#line 165 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_F32, (char*)"");}
#line 764 "src/parser.cpp" // lalr1.cc:859
    break;

  case 38:
#line 166 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_F64, (char*)"");}
#line 770 "src/parser.cpp" // lalr1.cc:859
    break;

  case 39:
#line 167 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_C8,  (char*)"");}
#line 776 "src/parser.cpp" // lalr1.cc:859
    break;

  case 40:
#line 168 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_C32, (char*)"");}
#line 782 "src/parser.cpp" // lalr1.cc:859
    break;

  case 41:
#line 169 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Bool, (char*)"");}
#line 788 "src/parser.cpp" // lalr1.cc:859
    break;

  case 42:
#line 170 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Void, (char*)"");}
#line 794 "src/parser.cpp" // lalr1.cc:859
    break;

  case 43:
#line 171 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_UserType, (char*)(yystack_[0].value));}
#line 800 "src/parser.cpp" // lalr1.cc:859
    break;

  case 44:
#line 172 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Ident, (char*)(yystack_[0].value));}
#line 806 "src/parser.cpp" // lalr1.cc:859
    break;

  case 45:
#line 175 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('*', (char*)"", (yystack_[1].value));}
#line 812 "src/parser.cpp" // lalr1.cc:859
    break;

  case 46:
#line 176 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('[', (char*)"", (yystack_[3].value));}
#line 818 "src/parser.cpp" // lalr1.cc:859
    break;

  case 47:
#line 177 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('(', (char*)"", (yystack_[3].value));}
#line 824 "src/parser.cpp" // lalr1.cc:859
    break;

  case 48:
#line 178 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('(', (char*)"", (yystack_[2].value));}
#line 830 "src/parser.cpp" // lalr1.cc:859
    break;

  case 49:
#line 179 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[2].value);}
#line 836 "src/parser.cpp" // lalr1.cc:859
    break;

  case 50:
#line 180 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 842 "src/parser.cpp" // lalr1.cc:859
    break;

  case 51:
#line 183 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 848 "src/parser.cpp" // lalr1.cc:859
    break;

  case 53:
#line 185 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 854 "src/parser.cpp" // lalr1.cc:859
    break;

  case 54:
#line 188 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 860 "src/parser.cpp" // lalr1.cc:859
    break;

  case 55:
#line 191 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pub);}
#line 866 "src/parser.cpp" // lalr1.cc:859
    break;

  case 56:
#line 192 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pri);}
#line 872 "src/parser.cpp" // lalr1.cc:859
    break;

  case 57:
#line 193 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pro);}
#line 878 "src/parser.cpp" // lalr1.cc:859
    break;

  case 58:
#line 194 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Raw);}
#line 884 "src/parser.cpp" // lalr1.cc:859
    break;

  case 59:
#line 195 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Const);}
#line 890 "src/parser.cpp" // lalr1.cc:859
    break;

  case 60:
#line 196 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Ext);}
#line 896 "src/parser.cpp" // lalr1.cc:859
    break;

  case 61:
#line 197 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Noinit);}
#line 902 "src/parser.cpp" // lalr1.cc:859
    break;

  case 62:
#line 198 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pathogen);}
#line 908 "src/parser.cpp" // lalr1.cc:859
    break;

  case 63:
#line 201 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[1].value), (yystack_[0].value));}
#line 914 "src/parser.cpp" // lalr1.cc:859
    break;

  case 64:
#line 202 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 920 "src/parser.cpp" // lalr1.cc:859
    break;

  case 65:
#line 205 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 926 "src/parser.cpp" // lalr1.cc:859
    break;

  case 66:
#line 209 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[2].value), (yystack_[4].value), (yystack_[3].value), (yystack_[0].value));}
#line 932 "src/parser.cpp" // lalr1.cc:859
    break;

  case 67:
#line 210 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[0].value), (yystack_[2].value), (yystack_[1].value),  0);}
#line 938 "src/parser.cpp" // lalr1.cc:859
    break;

  case 68:
#line 211 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[2].value), 0,  (yystack_[3].value), (yystack_[0].value));}
#line 944 "src/parser.cpp" // lalr1.cc:859
    break;

  case 69:
#line 212 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[0].value), 0,  (yystack_[1].value),  0);}
#line 950 "src/parser.cpp" // lalr1.cc:859
    break;

  case 70:
#line 215 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[3].value), (yystack_[4].value), (yystack_[3].value), (yystack_[0].value));}
#line 956 "src/parser.cpp" // lalr1.cc:859
    break;

  case 71:
#line 216 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[3].value), (yystack_[3].value), 0,  (yystack_[0].value));}
#line 962 "src/parser.cpp" // lalr1.cc:859
    break;

  case 72:
#line 217 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[2].value), 0,  (yystack_[3].value), (yystack_[0].value));}
#line 968 "src/parser.cpp" // lalr1.cc:859
    break;

  case 73:
#line 218 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[2].value), 0,  0,  (yystack_[0].value));}
#line 974 "src/parser.cpp" // lalr1.cc:859
    break;

  case 74:
#line 222 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarAssignNode((yystack_[2].value), (yystack_[0].value));}
#line 980 "src/parser.cpp" // lalr1.cc:859
    break;

  case 75:
#line 225 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 986 "src/parser.cpp" // lalr1.cc:859
    break;

  case 76:
#line 226 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 992 "src/parser.cpp" // lalr1.cc:859
    break;

  case 77:
#line 229 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 998 "src/parser.cpp" // lalr1.cc:859
    break;

  case 78:
#line 232 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[1].value), (yystack_[0].value));}
#line 1004 "src/parser.cpp" // lalr1.cc:859
    break;

  case 79:
#line 233 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[2].value), (yystack_[0].value));}
#line 1010 "src/parser.cpp" // lalr1.cc:859
    break;

  case 80:
#line 234 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[1].value), (yystack_[0].value));}
#line 1016 "src/parser.cpp" // lalr1.cc:859
    break;

  case 81:
#line 235 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[2].value), (yystack_[0].value));}
#line 1022 "src/parser.cpp" // lalr1.cc:859
    break;

  case 93:
#line 260 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1028 "src/parser.cpp" // lalr1.cc:859
    break;

  case 94:
#line 261 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1034 "src/parser.cpp" // lalr1.cc:859
    break;

  case 95:
#line 262 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1040 "src/parser.cpp" // lalr1.cc:859
    break;

  case 96:
#line 263 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1046 "src/parser.cpp" // lalr1.cc:859
    break;

  case 97:
#line 266 "src/syntax.y" // lalr1.cc:859
    {setNext((yystack_[2].value), (yystack_[1].value)); (yylhs.value) = getRoot();}
#line 1052 "src/parser.cpp" // lalr1.cc:859
    break;

  case 98:
#line 267 "src/syntax.y" // lalr1.cc:859
    {setNext((yystack_[2].value), (yystack_[1].value)); (yylhs.value) = getRoot();}
#line 1058 "src/parser.cpp" // lalr1.cc:859
    break;

  case 99:
#line 268 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1064 "src/parser.cpp" // lalr1.cc:859
    break;

  case 100:
#line 269 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1070 "src/parser.cpp" // lalr1.cc:859
    break;

  case 101:
#line 272 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[1].value), mkVarNode((char*)(yystack_[0].value)));}
#line 1076 "src/parser.cpp" // lalr1.cc:859
    break;

  case 102:
#line 273 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkVarNode((char*)(yystack_[0].value)));}
#line 1082 "src/parser.cpp" // lalr1.cc:859
    break;

  case 103:
#line 281 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[3].value), mkNamedValNode(getRoot(), (yystack_[1].value)));}
#line 1088 "src/parser.cpp" // lalr1.cc:859
    break;

  case 104:
#line 282 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkNamedValNode(getRoot(), (yystack_[1].value)));}
#line 1094 "src/parser.cpp" // lalr1.cc:859
    break;

  case 105:
#line 285 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1100 "src/parser.cpp" // lalr1.cc:859
    break;

  case 106:
#line 286 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1106 "src/parser.cpp" // lalr1.cc:859
    break;

  case 107:
#line 289 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[3].value), (yystack_[5].value), (yystack_[4].value), (yystack_[1].value), (yystack_[0].value));}
#line 1112 "src/parser.cpp" // lalr1.cc:859
    break;

  case 108:
#line 290 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[6].value), (yystack_[8].value), (yystack_[7].value), (yystack_[1].value), (yystack_[0].value));}
#line 1118 "src/parser.cpp" // lalr1.cc:859
    break;

  case 109:
#line 291 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[3].value), 0,  (yystack_[4].value), (yystack_[1].value), (yystack_[0].value));}
#line 1124 "src/parser.cpp" // lalr1.cc:859
    break;

  case 110:
#line 292 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[6].value), 0,  (yystack_[7].value), (yystack_[1].value), (yystack_[0].value));}
#line 1130 "src/parser.cpp" // lalr1.cc:859
    break;

  case 111:
#line 295 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncCallNode((char*)(yystack_[3].value), (yystack_[1].value));}
#line 1136 "src/parser.cpp" // lalr1.cc:859
    break;

  case 112:
#line 298 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkRetNode((yystack_[0].value));}
#line 1142 "src/parser.cpp" // lalr1.cc:859
    break;

  case 113:
#line 301 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setElse((IfNode*)(yystack_[3].value), (IfNode*)mkIfNode((yystack_[1].value), (yystack_[0].value)));}
#line 1148 "src/parser.cpp" // lalr1.cc:859
    break;

  case 114:
#line 302 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkIfNode((yystack_[1].value), (yystack_[0].value)));}
#line 1154 "src/parser.cpp" // lalr1.cc:859
    break;

  case 115:
#line 305 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setElse((IfNode*)(yystack_[2].value), (IfNode*)mkIfNode(NULL, (yystack_[0].value)));}
#line 1160 "src/parser.cpp" // lalr1.cc:859
    break;

  case 116:
#line 306 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1166 "src/parser.cpp" // lalr1.cc:859
    break;

  case 117:
#line 307 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkIfNode(NULL, (yystack_[0].value)));}
#line 1172 "src/parser.cpp" // lalr1.cc:859
    break;

  case 118:
#line 308 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(NULL);}
#line 1178 "src/parser.cpp" // lalr1.cc:859
    break;

  case 119:
#line 311 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkIfNode((yystack_[2].value), (yystack_[1].value), (IfNode*)getRoot());}
#line 1184 "src/parser.cpp" // lalr1.cc:859
    break;

  case 120:
#line 314 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1190 "src/parser.cpp" // lalr1.cc:859
    break;

  case 121:
#line 317 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1196 "src/parser.cpp" // lalr1.cc:859
    break;

  case 122:
#line 320 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1202 "src/parser.cpp" // lalr1.cc:859
    break;

  case 124:
#line 324 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarNode((char*)(yystack_[0].value));}
#line 1208 "src/parser.cpp" // lalr1.cc:859
    break;

  case 125:
#line 327 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('&', (yystack_[0].value));}
#line 1214 "src/parser.cpp" // lalr1.cc:859
    break;

  case 126:
#line 328 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('*', (yystack_[0].value));}
#line 1220 "src/parser.cpp" // lalr1.cc:859
    break;

  case 128:
#line 330 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkRefVarNode((char*)(yystack_[0].value));}
#line 1226 "src/parser.cpp" // lalr1.cc:859
    break;

  case 129:
#line 333 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1232 "src/parser.cpp" // lalr1.cc:859
    break;

  case 130:
#line 334 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1238 "src/parser.cpp" // lalr1.cc:859
    break;

  case 131:
#line 335 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1244 "src/parser.cpp" // lalr1.cc:859
    break;

  case 132:
#line 336 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1250 "src/parser.cpp" // lalr1.cc:859
    break;

  case 133:
#line 337 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1256 "src/parser.cpp" // lalr1.cc:859
    break;

  case 134:
#line 338 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1262 "src/parser.cpp" // lalr1.cc:859
    break;

  case 135:
#line 339 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1268 "src/parser.cpp" // lalr1.cc:859
    break;

  case 136:
#line 340 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1274 "src/parser.cpp" // lalr1.cc:859
    break;

  case 137:
#line 341 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBoolLitNode(1);}
#line 1280 "src/parser.cpp" // lalr1.cc:859
    break;

  case 138:
#line 342 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBoolLitNode(0);}
#line 1286 "src/parser.cpp" // lalr1.cc:859
    break;

  case 139:
#line 346 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1292 "src/parser.cpp" // lalr1.cc:859
    break;

  case 140:
#line 347 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1298 "src/parser.cpp" // lalr1.cc:859
    break;

  case 141:
#line 350 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1304 "src/parser.cpp" // lalr1.cc:859
    break;

  case 142:
#line 352 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 1310 "src/parser.cpp" // lalr1.cc:859
    break;

  case 143:
#line 353 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 1316 "src/parser.cpp" // lalr1.cc:859
    break;

  case 144:
#line 356 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('*', (yystack_[0].value));}
#line 1322 "src/parser.cpp" // lalr1.cc:859
    break;

  case 145:
#line 357 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('&', (yystack_[0].value));}
#line 1328 "src/parser.cpp" // lalr1.cc:859
    break;

  case 146:
#line 358 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('-', (yystack_[0].value));}
#line 1334 "src/parser.cpp" // lalr1.cc:859
    break;

  case 147:
#line 361 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('+', (yystack_[2].value), (yystack_[0].value));}
#line 1340 "src/parser.cpp" // lalr1.cc:859
    break;

  case 148:
#line 362 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('-', (yystack_[2].value), (yystack_[0].value));}
#line 1346 "src/parser.cpp" // lalr1.cc:859
    break;

  case 149:
#line 363 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('*', (yystack_[2].value), (yystack_[0].value));}
#line 1352 "src/parser.cpp" // lalr1.cc:859
    break;

  case 150:
#line 364 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('/', (yystack_[2].value), (yystack_[0].value));}
#line 1358 "src/parser.cpp" // lalr1.cc:859
    break;

  case 151:
#line 365 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('%', (yystack_[2].value), (yystack_[0].value));}
#line 1364 "src/parser.cpp" // lalr1.cc:859
    break;

  case 152:
#line 366 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('<', (yystack_[2].value), (yystack_[0].value));}
#line 1370 "src/parser.cpp" // lalr1.cc:859
    break;

  case 153:
#line 367 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('>', (yystack_[2].value), (yystack_[0].value));}
#line 1376 "src/parser.cpp" // lalr1.cc:859
    break;

  case 154:
#line 368 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('^', (yystack_[2].value), (yystack_[0].value));}
#line 1382 "src/parser.cpp" // lalr1.cc:859
    break;

  case 155:
#line 369 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('.', (yystack_[2].value), (yystack_[0].value));}
#line 1388 "src/parser.cpp" // lalr1.cc:859
    break;

  case 156:
#line 370 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Eq, (yystack_[2].value), (yystack_[0].value));}
#line 1394 "src/parser.cpp" // lalr1.cc:859
    break;

  case 157:
#line 371 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_NotEq, (yystack_[2].value), (yystack_[0].value));}
#line 1400 "src/parser.cpp" // lalr1.cc:859
    break;

  case 158:
#line 372 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_GrtrEq, (yystack_[2].value), (yystack_[0].value));}
#line 1406 "src/parser.cpp" // lalr1.cc:859
    break;

  case 159:
#line 373 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_LesrEq, (yystack_[2].value), (yystack_[0].value));}
#line 1412 "src/parser.cpp" // lalr1.cc:859
    break;

  case 160:
#line 374 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Or, (yystack_[2].value), (yystack_[0].value));}
#line 1418 "src/parser.cpp" // lalr1.cc:859
    break;

  case 161:
#line 375 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_And, (yystack_[2].value), (yystack_[0].value));}
#line 1424 "src/parser.cpp" // lalr1.cc:859
    break;

  case 162:
#line 376 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1430 "src/parser.cpp" // lalr1.cc:859
    break;

  case 163:
#line 381 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1436 "src/parser.cpp" // lalr1.cc:859
    break;

  case 164:
#line 384 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[3].value), (yystack_[0].value));}
#line 1442 "src/parser.cpp" // lalr1.cc:859
    break;

  case 165:
#line 385 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 1448 "src/parser.cpp" // lalr1.cc:859
    break;

  case 166:
#line 388 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('+', (yystack_[3].value), (yystack_[0].value));}
#line 1454 "src/parser.cpp" // lalr1.cc:859
    break;

  case 167:
#line 389 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('-', (yystack_[3].value), (yystack_[0].value));}
#line 1460 "src/parser.cpp" // lalr1.cc:859
    break;

  case 168:
#line 390 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('*', (yystack_[3].value), (yystack_[0].value));}
#line 1466 "src/parser.cpp" // lalr1.cc:859
    break;

  case 169:
#line 391 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('/', (yystack_[3].value), (yystack_[0].value));}
#line 1472 "src/parser.cpp" // lalr1.cc:859
    break;

  case 170:
#line 392 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('%', (yystack_[3].value), (yystack_[0].value));}
#line 1478 "src/parser.cpp" // lalr1.cc:859
    break;

  case 171:
#line 393 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('<', (yystack_[3].value), (yystack_[0].value));}
#line 1484 "src/parser.cpp" // lalr1.cc:859
    break;

  case 172:
#line 394 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('>', (yystack_[3].value), (yystack_[0].value));}
#line 1490 "src/parser.cpp" // lalr1.cc:859
    break;

  case 173:
#line 395 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('^', (yystack_[3].value), (yystack_[0].value));}
#line 1496 "src/parser.cpp" // lalr1.cc:859
    break;

  case 174:
#line 396 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('.', (yystack_[3].value), (yystack_[0].value));}
#line 1502 "src/parser.cpp" // lalr1.cc:859
    break;

  case 175:
#line 397 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Eq, (yystack_[3].value), (yystack_[0].value));}
#line 1508 "src/parser.cpp" // lalr1.cc:859
    break;

  case 176:
#line 398 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_NotEq, (yystack_[3].value), (yystack_[0].value));}
#line 1514 "src/parser.cpp" // lalr1.cc:859
    break;

  case 177:
#line 399 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_GrtrEq, (yystack_[3].value), (yystack_[0].value));}
#line 1520 "src/parser.cpp" // lalr1.cc:859
    break;

  case 178:
#line 400 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_LesrEq, (yystack_[3].value), (yystack_[0].value));}
#line 1526 "src/parser.cpp" // lalr1.cc:859
    break;

  case 179:
#line 401 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Or, (yystack_[3].value), (yystack_[0].value));}
#line 1532 "src/parser.cpp" // lalr1.cc:859
    break;

  case 180:
#line 402 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_And, (yystack_[3].value), (yystack_[0].value));}
#line 1538 "src/parser.cpp" // lalr1.cc:859
    break;

  case 181:
#line 403 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1544 "src/parser.cpp" // lalr1.cc:859
    break;


#line 1548 "src/parser.cpp" // lalr1.cc:859
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


  const short int parser::yypact_ninf_ = -239;

  const short int parser::yytable_ninf_ = -129;

  const short int
  parser::yypact_[] =
  {
     -45,  -239,    28,   459,  -239,  -239,  -239,  -239,  -239,  -239,
    -239,  -239,  -239,  -239,  -239,  -239,  -239,  -239,  -239,  -239,
    -239,  -239,  -239,  -239,   205,   205,   651,   205,    27,   651,
      41,    18,  -239,  -239,  -239,  -239,  -239,  -239,  -239,  -239,
       7,   671,     7,   375,  -239,     9,   -37,  -239,  -239,    -2,
     -50,    88,  -239,     0,   595,  -239,  -239,  -239,  -239,  -239,
    -239,  -239,  -239,  -239,  -239,  -239,  -239,    13,  -239,  -239,
    -239,  -239,  -239,   205,   205,   205,   205,   205,    60,  -239,
    -239,  -239,  -239,  -239,  -239,  -239,    37,  -239,    95,    16,
    -239,    88,   671,    68,    16,   205,    26,    88,   671,   -38,
      41,    50,  -239,    43,  -239,    38,  -239,  -239,  -239,    70,
    -239,   205,   205,  -239,   543,   205,   671,   671,   -49,  -239,
      41,    18,    88,   205,  -239,    79,    83,   413,  -239,  -239,
      65,  -239,   205,   205,   205,   205,   205,   205,   205,   205,
     205,   205,   205,   205,   205,   205,   205,   205,   205,   459,
     102,   107,    88,   205,  -239,    16,   205,   109,   112,    88,
     575,    41,   114,  -239,   115,   -32,  -239,  -239,  -239,   116,
    -239,   119,  -239,   120,   122,    -2,    -2,   205,   205,   671,
     -38,    50,  -239,   -35,  -239,  -239,   -45,   -45,   -45,   -45,
     -45,   -45,   -45,   -45,   -45,   -45,   -45,   -45,   -45,   -45,
     -45,   -45,  -239,   124,    95,   -10,   -10,   -10,   -10,   497,
     699,   -10,   -10,   108,   108,    77,    77,    77,    77,  -239,
     459,   131,     6,   205,    16,   135,  -239,   125,    16,  -239,
    -239,   205,   205,   126,    88,   155,  -239,    20,  -239,  -239,
      17,  -239,   205,    41,  -239,  -239,  -239,  -239,  -239,   129,
    -239,    88,   144,    16,   114,  -239,  -239,   205,   205,   671,
     205,   205,   205,   205,   205,   205,   205,   205,   205,   205,
     205,   205,   205,   205,   205,   205,  -239,   147,    34,  -239,
    -239,    16,  -239,   205,    16,  -239,  -239,  -239,   205,  -239,
     575,  -239,    41,  -239,  -239,   130,   133,  -239,    88,   671,
    -239,  -239,   138,  -239,    16,   413,    56,    56,    56,    56,
     689,   708,    56,    56,   113,   113,   101,   101,   101,   101,
    -239,  -239,  -239,  -239,    16,  -239,  -239,  -239,  -239,   205,
     671,  -239,    88,   136,  -239,  -239,  -239,    16,    88,   671,
    -239,    16,  -239
  };

  const unsigned char
  parser::yydefact_[] =
  {
       8,     7,     0,     0,     1,    21,    22,    26,    27,    28,
      29,    30,    31,    32,    33,    34,    35,    36,    37,    38,
      39,    40,    41,    42,     0,     0,     0,     0,     0,     0,
       0,     0,    55,    56,    57,    58,    59,    60,    61,    62,
       0,     0,     0,     8,     6,     0,    44,    43,    50,    53,
      54,     0,    64,    65,     0,    16,    20,    17,    10,    11,
       9,    18,    19,    15,    12,    13,    14,     0,   137,   138,
      23,    24,    25,     0,     0,     0,     0,     0,   124,   134,
     135,   136,   129,   133,   162,   112,   141,   132,   143,     0,
      44,     0,     0,     0,     0,     0,    44,     0,     0,     0,
       0,     0,    96,   128,   126,     0,   125,     2,     4,     0,
       5,   140,     0,    45,     0,   140,     0,     0,    69,    63,
       0,     0,     0,     0,   181,     0,   163,   165,   146,   144,
       0,   145,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
     118,    69,     0,     0,   120,     0,     0,     0,    44,     0,
       0,     0,     0,    80,    91,     0,    94,    49,     3,     0,
     139,     0,    48,     0,     0,    51,    52,   140,     0,   106,
       0,     0,    95,    67,    74,   131,     8,     8,     8,     8,
       8,     8,     8,     8,     8,     8,     8,     8,     8,     8,
       8,     8,   130,     0,   142,   156,   157,   158,   159,   160,
     161,   152,   153,   147,   148,   149,   150,   151,   154,   155,
       0,     6,     0,     0,     0,   116,   119,    67,     0,   121,
      73,     0,     0,     0,    83,     0,    86,     0,    84,    76,
       0,    81,     0,     0,    92,   111,   127,    47,    46,     0,
      68,     0,   105,     0,     0,    78,    93,   140,     0,   106,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,   123,     4,     0,    99,
     100,     0,   117,     0,     0,   122,    72,    71,     0,    82,
       0,    87,     0,    77,    90,    88,     0,   102,   104,     0,
     109,    79,     0,    66,     0,   164,   175,   176,   177,   178,
     179,   180,   171,   172,   166,   167,   168,   169,   170,   173,
     174,    97,    98,   114,     0,   115,    70,    85,    75,     0,
     106,   101,     0,     0,   107,   113,    89,     0,   103,   106,
     110,     0,   108
  };

  const short int
  parser::yypgoto_[] =
  {
    -239,  -239,    73,   176,   -40,   -28,    -3,   -18,  -239,  -239,
    -239,  -239,    78,  -239,   -24,   174,  -239,   -25,   202,  -239,
    -239,  -239,    49,  -239,   -60,  -239,  -155,  -239,   -77,  -140,
     -80,  -100,  -239,  -238,  -239,     3,  -239,  -239,  -239,  -239,
    -239,  -239,  -239,  -239,    66,    86,   -99,   -16,  -239,  -239,
     653,  -239,  -239,   542
  };

  const short int
  parser::yydefgoto_[] =
  {
      -1,     2,    43,     3,    44,    45,    78,    47,    79,    80,
      81,    48,    49,    50,    51,    52,    53,    54,    55,    56,
      57,   240,   162,    58,   236,   237,   163,   165,   102,    59,
     150,   298,   252,   253,    60,    82,    62,   225,   226,    63,
      64,    65,    66,    83,    67,    84,   169,   170,    86,    87,
      88,   125,   126,   127
  };

  const short int
  parser::yytable_[] =
  {
      46,    92,    91,   108,    98,    97,    61,   241,    85,    89,
       5,    94,    99,   101,   154,   109,   174,   105,   116,     1,
     238,   304,     6,    90,   166,   255,    96,   160,     4,   177,
     122,   161,   243,   117,   244,   178,   179,   103,    90,   103,
      46,   111,   112,   257,   182,     6,    61,  -128,   118,   258,
     259,    90,    32,    33,    34,    35,    36,    37,    38,    39,
     130,   142,   143,   144,   145,   146,   147,   148,   152,    95,
     110,   113,   280,   110,   159,   229,   114,   115,   249,   155,
      40,   149,   164,   100,   290,   292,   291,   293,   151,    90,
     173,     5,   337,    42,   157,   158,   171,   123,   168,   301,
     322,   341,   180,   181,   256,   133,   104,   184,   106,   221,
     156,    90,   153,    90,    90,   100,   203,   134,   135,   183,
     167,   222,   112,   136,   137,   138,   139,   195,   196,   197,
     198,   199,   200,   201,   168,   235,   234,   228,   111,   132,
     230,   223,   224,   239,   282,   185,    46,   202,   285,   227,
     238,   186,    61,   147,   148,   251,   233,    90,   302,   124,
     128,   129,   250,   131,   140,   141,   142,   143,   144,   145,
     146,   147,   148,   300,   283,   284,    90,   200,   201,   160,
     277,   144,   145,   146,   147,   148,   197,   198,   199,   200,
     201,   178,   278,   231,   175,   176,   232,   279,   245,   242,
     246,   323,   247,   248,   325,   276,   121,   281,     5,   258,
     288,   296,   299,   321,   329,   286,   287,    46,   330,   107,
     333,   339,   220,    61,   334,   295,   294,   119,    93,   254,
     327,   289,   338,     0,     0,   251,     0,    68,    69,    70,
      71,    72,   303,     0,   335,     0,     0,     0,   297,     0,
       0,     0,     0,     0,     0,     0,    90,   340,     0,     0,
       0,   342,     0,     0,     0,   235,   234,   324,     0,     0,
      73,     0,   326,     0,   328,   332,     0,    74,    75,     0,
       0,     0,     0,    76,     0,     0,     0,    90,     0,     0,
       0,    77,     0,     0,     0,   331,    90,     0,     0,     0,
       0,     0,     0,     0,     0,     0,   251,     0,     0,     0,
       0,     0,     0,   336,     0,   251,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,    90,     0,   297,
       0,     0,     0,     0,     0,   331,    90,     0,     0,     0,
       0,     0,     0,     0,     0,     0,   124,   124,   124,   124,
     124,   124,   124,   124,   124,   124,   124,   124,   124,   124,
     124,   124,   260,   261,   262,   263,   264,   265,   266,   267,
     268,   269,   270,   271,   272,   273,   274,   275,     5,     6,
       7,     8,     9,    10,    11,    12,    13,    14,    15,    16,
      17,    18,    19,    20,    21,    22,    23,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,    24,    25,     0,     0,    26,    27,    28,     0,
       0,     0,     0,    29,     0,    30,    31,    32,    33,    34,
      35,    36,    37,    38,    39,   187,   188,     0,     0,     1,
       0,   189,   190,   191,   192,     0,     0,     0,    40,     0,
       0,     0,     0,    41,     0,     0,     0,     0,     0,     0,
       0,    42,     5,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    21,    22,
      23,     0,   193,   194,   195,   196,   197,   198,   199,   200,
     201,     0,     0,     0,     0,     0,    24,    25,     0,     0,
      26,    27,    28,     0,     0,     0,     0,    29,     0,    30,
      31,    32,    33,    34,    35,    36,    37,    38,    39,   134,
     135,     0,     0,     0,     0,   136,   137,     0,   139,     0,
       0,     0,    40,     0,     0,     0,     0,    41,     0,     0,
       0,     0,     0,     0,     0,    42,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    22,    23,     0,   140,   141,   142,   143,
     144,   145,   146,   147,   148,     0,     0,     0,     5,     6,
       7,     8,     9,    10,    11,    12,    13,    14,    15,    16,
      17,    18,    19,    20,    21,    22,    23,     0,     5,     6,
       7,     8,     9,    10,    11,    12,    13,    14,    15,    16,
      17,    18,    19,    20,    21,    22,    23,     0,     0,     0,
       0,    41,     0,     0,     0,   172,    31,    32,    33,    34,
      35,    36,    37,    38,    39,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,   120,   121,     0,     0,     0,
       0,     0,     0,    41,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    22,    23,    41,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    22,    23,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,    32,    33,    34,    35,    36,    37,    38,
      39,   187,   188,     0,     0,     0,     0,   189,   190,     0,
     192,   134,   135,     0,     0,     0,     0,   136,   137,    41,
     187,   188,     0,     0,     0,     0,   189,   190,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,    41,
       0,     0,     0,     0,     0,     0,     0,     0,   193,   194,
     195,   196,   197,   198,   199,   200,   201,     0,   140,   141,
     142,   143,   144,   145,   146,   147,   148,   193,   194,   195,
     196,   197,   198,   199,   200,   201,   204,   205,   206,   207,
     208,   209,   210,   211,   212,   213,   214,   215,   216,   217,
     218,   219,   305,   306,   307,   308,   309,   310,   311,   312,
     313,   314,   315,   316,   317,   318,   319,   320
  };

  const short int
  parser::yycheck_[] =
  {
       3,    26,    26,    43,    29,    29,     3,   162,    24,    25,
       3,    27,    30,    31,    94,    43,   115,    41,    68,    64,
     160,   259,     4,    26,   101,   180,    29,    65,     0,    78,
      54,    69,    64,    83,    66,    84,    85,    40,    41,    42,
      43,    78,    79,    78,   121,     4,    43,    84,    51,    84,
      85,    54,    52,    53,    54,    55,    56,    57,    58,    59,
      76,    71,    72,    73,    74,    75,    76,    77,    92,    42,
      64,    73,    66,    64,    98,   155,    78,    79,   177,    95,
      73,    65,   100,    65,    64,    68,    66,    70,    91,    92,
     114,     3,   330,    86,    97,    98,   112,    84,    64,   254,
      66,   339,   120,   121,   181,    68,    40,   123,    42,   149,
      84,   114,    44,   116,   117,    65,   132,    22,    23,   122,
      82,   149,    79,    28,    29,    30,    31,    71,    72,    73,
      74,    75,    76,    77,    64,   160,   160,   153,    78,    79,
     156,    39,    40,   161,   224,    66,   149,    82,   228,   152,
     290,    68,   149,    76,    77,   179,   159,   160,   257,    73,
      74,    75,   178,    77,    69,    70,    71,    72,    73,    74,
      75,    76,    77,   253,    39,    40,   179,    76,    77,    65,
     220,    73,    74,    75,    76,    77,    73,    74,    75,    76,
      77,    84,   220,    84,   116,   117,    84,    66,    82,    84,
      81,   281,    82,    81,   284,    81,    51,   223,     3,    84,
      84,    82,    68,    66,    84,   231,   232,   220,    85,    43,
      82,    85,   149,   220,   304,   243,   242,    53,    26,   180,
     290,   234,   332,    -1,    -1,   259,    -1,    32,    33,    34,
      35,    36,   258,    -1,   324,    -1,    -1,    -1,   251,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,   259,   337,    -1,    -1,
      -1,   341,    -1,    -1,    -1,   290,   290,   283,    -1,    -1,
      65,    -1,   288,    -1,   292,   299,    -1,    72,    73,    -1,
      -1,    -1,    -1,    78,    -1,    -1,    -1,   290,    -1,    -1,
      -1,    86,    -1,    -1,    -1,   298,   299,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,   330,    -1,    -1,    -1,
      -1,    -1,    -1,   329,    -1,   339,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,   330,    -1,   332,
      -1,    -1,    -1,    -1,    -1,   338,   339,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,   260,   261,   262,   263,
     264,   265,   266,   267,   268,   269,   270,   271,   272,   273,
     274,   275,   186,   187,   188,   189,   190,   191,   192,   193,
     194,   195,   196,   197,   198,   199,   200,   201,     3,     4,
       5,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    37,    38,    -1,    -1,    41,    42,    43,    -1,
      -1,    -1,    -1,    48,    -1,    50,    51,    52,    53,    54,
      55,    56,    57,    58,    59,    22,    23,    -1,    -1,    64,
      -1,    28,    29,    30,    31,    -1,    -1,    -1,    73,    -1,
      -1,    -1,    -1,    78,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    86,     3,     4,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    -1,    69,    70,    71,    72,    73,    74,    75,    76,
      77,    -1,    -1,    -1,    -1,    -1,    37,    38,    -1,    -1,
      41,    42,    43,    -1,    -1,    -1,    -1,    48,    -1,    50,
      51,    52,    53,    54,    55,    56,    57,    58,    59,    22,
      23,    -1,    -1,    -1,    -1,    28,    29,    -1,    31,    -1,
      -1,    -1,    73,    -1,    -1,    -1,    -1,    78,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    86,     3,     4,     5,     6,
       7,     8,     9,    10,    11,    12,    13,    14,    15,    16,
      17,    18,    19,    20,    21,    -1,    69,    70,    71,    72,
      73,    74,    75,    76,    77,    -1,    -1,    -1,     3,     4,
       5,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    -1,     3,     4,
       5,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    -1,    -1,    -1,
      -1,    78,    -1,    -1,    -1,    82,    51,    52,    53,    54,
      55,    56,    57,    58,    59,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    50,    51,    -1,    -1,    -1,
      -1,    -1,    -1,    78,     3,     4,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    78,     3,     4,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    52,    53,    54,    55,    56,    57,    58,
      59,    22,    23,    -1,    -1,    -1,    -1,    28,    29,    -1,
      31,    22,    23,    -1,    -1,    -1,    -1,    28,    29,    78,
      22,    23,    -1,    -1,    -1,    -1,    28,    29,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    78,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    69,    70,
      71,    72,    73,    74,    75,    76,    77,    -1,    69,    70,
      71,    72,    73,    74,    75,    76,    77,    69,    70,    71,
      72,    73,    74,    75,    76,    77,   133,   134,   135,   136,
     137,   138,   139,   140,   141,   142,   143,   144,   145,   146,
     147,   148,   260,   261,   262,   263,   264,   265,   266,   267,
     268,   269,   270,   271,   272,   273,   274,   275
  };

  const unsigned char
  parser::yystos_[] =
  {
       0,    64,    88,    90,     0,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    37,    38,    41,    42,    43,    48,
      50,    51,    52,    53,    54,    55,    56,    57,    58,    59,
      73,    78,    86,    89,    91,    92,    93,    94,    98,    99,
     100,   101,   102,   103,   104,   105,   106,   107,   110,   116,
     121,   122,   123,   126,   127,   128,   129,   131,    32,    33,
      34,    35,    36,    65,    72,    73,    78,    86,    93,    95,
      96,    97,   122,   130,   132,   134,   135,   136,   137,   134,
      93,   101,   104,   105,   134,    42,    93,   101,   104,    94,
      65,    94,   115,    93,   131,   101,   131,    90,    91,    92,
      64,    78,    79,    73,    78,    79,    68,    83,    93,   102,
      50,    51,   101,    84,   132,   138,   139,   140,   132,   132,
     134,   132,    79,    68,    22,    23,    28,    29,    30,    31,
      69,    70,    71,    72,    73,    74,    75,    76,    77,    65,
     117,    93,   101,    44,   117,   134,    84,    93,    93,   101,
      65,    69,   109,   113,    94,   114,   115,    82,    64,   133,
     134,   134,    82,   101,   133,    99,    99,    78,    84,    85,
      94,    94,   115,    93,   134,    66,    68,    22,    23,    28,
      29,    30,    31,    69,    70,    71,    72,    73,    74,    75,
      76,    77,    82,   134,   137,   137,   137,   137,   137,   137,
     137,   137,   137,   137,   137,   137,   137,   137,   137,   137,
      89,    91,    92,    39,    40,   124,   125,    93,   134,   117,
     134,    84,    84,    93,   101,   104,   111,   112,   116,    94,
     108,   113,    84,    64,    66,    82,    81,    82,    81,   133,
     134,   101,   119,   120,   109,   113,   115,    78,    84,    85,
      90,    90,    90,    90,    90,    90,    90,    90,    90,    90,
      90,    90,    90,    90,    90,    90,    81,    91,    92,    66,
      66,   134,   117,    39,    40,   117,   134,   134,    84,    93,
      64,    66,    68,    70,   134,    94,    82,    93,   118,    68,
     117,   113,   133,   134,   120,   140,   140,   140,   140,   140,
     140,   140,   140,   140,   140,   140,   140,   140,   140,   140,
     140,    66,    66,   117,   134,   117,   134,   111,    94,    84,
      85,    93,   101,    82,   117,   117,   134,   120,   118,    85,
     117,   120,   117
  };

  const unsigned char
  parser::yyr1_[] =
  {
       0,    87,    88,    89,    89,    89,    89,    90,    90,    91,
      91,    91,    91,    91,    91,    91,    92,    92,    92,    92,
      92,    93,    94,    95,    96,    97,    98,    98,    98,    98,
      98,    98,    98,    98,    98,    98,    98,    98,    98,    98,
      98,    98,    98,    98,    98,    99,    99,    99,    99,    99,
      99,   100,   100,   100,   101,   102,   102,   102,   102,   102,
     102,   102,   102,   103,   103,   104,   105,   105,   105,   105,
     106,   106,   106,   106,   107,   108,   108,   109,   110,   110,
     110,   110,   111,   111,   111,   112,   112,   113,   114,   114,
     114,   114,   115,   116,   116,   116,   116,   117,   117,   117,
     117,   118,   118,   119,   119,   120,   120,   121,   121,   121,
     121,   122,   123,   124,   124,   125,   125,   125,   125,   126,
     127,   128,   129,   130,   130,   131,   131,   131,   131,   132,
     132,   132,   132,   132,   132,   132,   132,   132,   132,   133,
     133,   134,   135,   135,   136,   136,   136,   137,   137,   137,
     137,   137,   137,   137,   137,   137,   137,   137,   137,   137,
     137,   137,   137,   138,   139,   139,   140,   140,   140,   140,
     140,   140,   140,   140,   140,   140,   140,   140,   140,   140,
     140,   140
  };

  const unsigned char
  parser::yyr2_[] =
  {
       0,     2,     3,     3,     2,     2,     1,     1,     0,     1,
       1,     1,     1,     1,     1,     1,     1,     1,     1,     1,
       1,     1,     1,     1,     1,     1,     1,     1,     1,     1,
       1,     1,     1,     1,     1,     1,     1,     1,     1,     1,
       1,     1,     1,     1,     1,     2,     4,     4,     3,     3,
       1,     3,     3,     1,     1,     1,     1,     1,     1,     1,
       1,     1,     1,     2,     1,     1,     5,     3,     4,     2,
       6,     5,     5,     4,     3,     3,     1,     3,     4,     5,
       3,     4,     2,     1,     1,     3,     1,     3,     3,     5,
       3,     1,     3,     4,     3,     3,     2,     4,     4,     3,
       3,     2,     1,     4,     2,     1,     0,     6,     9,     5,
       8,     4,     2,     4,     3,     3,     1,     2,     0,     4,
       3,     4,     5,     4,     1,     2,     2,     4,     1,     1,
       3,     3,     1,     1,     1,     1,     1,     1,     1,     1,
       0,     1,     3,     1,     2,     2,     2,     3,     3,     3,
       3,     3,     3,     3,     3,     3,     3,     3,     3,     3,
       3,     3,     1,     1,     4,     1,     4,     4,     4,     4,
       4,     4,     4,     4,     4,     4,     4,     4,     4,     4,
       4,     1
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
  "'.'", "'('", "'['", "HIGH", "']'", "')'", "'|'", "'='", "':'", "'&'",
  "$accept", "top_level_stmt_list", "stmt_list", "maybe_newline",
  "no_nl_stmt", "nl_stmt", "ident", "usertype", "intlit", "fltlit",
  "strlit", "lit_type", "type", "type_expr_", "type_expr", "modifier",
  "modifier_list_", "modifier_list", "var_decl", "let_binding",
  "var_assign", "usertype_list", "generic", "data_decl", "type_decl",
  "type_decl_list", "type_decl_block", "val_init_list", "enum_block",
  "enum_decl", "block", "ident_list", "params", "maybe_params", "fn_decl",
  "fn_call", "ret_stmt", "elif_list", "maybe_elif_list", "if_stmt",
  "while_loop", "do_while_loop", "for_loop", "var", "ref_val", "val",
  "maybe_expr", "expr", "expr_list", "unary_op", "expr_p", "nl_expr",
  "nl_expr_list", "expr_block_p", YY_NULLPTR
  };

#if YYDEBUG
  const unsigned short int
  parser::yyrline_[] =
  {
       0,   105,   105,   108,   109,   110,   111,   114,   115,   122,
     123,   124,   125,   126,   127,   128,   132,   133,   134,   135,
     136,   139,   142,   145,   148,   151,   154,   155,   156,   157,
     158,   159,   160,   161,   162,   163,   164,   165,   166,   167,
     168,   169,   170,   171,   172,   175,   176,   177,   178,   179,
     180,   183,   184,   185,   188,   191,   192,   193,   194,   195,
     196,   197,   198,   201,   202,   205,   209,   210,   211,   212,
     215,   216,   217,   218,   222,   225,   226,   229,   232,   233,
     234,   235,   238,   239,   240,   243,   244,   247,   251,   252,
     253,   254,   257,   260,   261,   262,   263,   266,   267,   268,
     269,   272,   273,   281,   282,   285,   286,   289,   290,   291,
     292,   295,   298,   301,   302,   305,   306,   307,   308,   311,
     314,   317,   320,   323,   324,   327,   328,   329,   330,   333,
     334,   335,   336,   337,   338,   339,   340,   341,   342,   346,
     347,   350,   352,   353,   356,   357,   358,   361,   362,   363,
     364,   365,   366,   367,   368,   369,   370,   371,   372,   373,
     374,   375,   376,   381,   384,   385,   388,   389,   390,   391,
     392,   393,   394,   395,   396,   397,   398,   399,   400,   401,
     402,   403
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
       2,     2,     2,     2,     2,     2,     2,    75,    86,     2,
      78,    82,    73,    71,    68,    72,    77,    74,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,    85,     2,
      69,    84,    70,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,    79,     2,    81,    76,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,    83,     2,     2,     2,     2,     2,
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
      65,    66,    67,    80
    };
    const unsigned int user_token_number_max_ = 323;
    const token_number_type undef_token_ = 2;

    if (static_cast<int>(t) <= yyeof_)
      return yyeof_;
    else if (static_cast<unsigned int> (t) <= user_token_number_max_)
      return translate_table[t];
    else
      return undef_token_;
  }


} // yy
#line 2311 "src/parser.cpp" // lalr1.cc:1167
#line 405 "src/syntax.y" // lalr1.cc:1168


void yy::parser::error(const location& loc, const string& msg){
    ante::error(msg.c_str(), yylexer->fileName, yylexer->getRow(), yylexer->getCol());
}

#endif
