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
#line 115 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[1].value));}
#line 644 "src/parser.cpp" // lalr1.cc:859
    break;

  case 4:
#line 116 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[1].value), (yystack_[0].value));}
#line 650 "src/parser.cpp" // lalr1.cc:859
    break;

  case 5:
#line 117 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[1].value));}
#line 656 "src/parser.cpp" // lalr1.cc:859
    break;

  case 6:
#line 118 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 662 "src/parser.cpp" // lalr1.cc:859
    break;

  case 21:
#line 146 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (Node*)lextxt;}
#line 668 "src/parser.cpp" // lalr1.cc:859
    break;

  case 22:
#line 149 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (Node*)lextxt;}
#line 674 "src/parser.cpp" // lalr1.cc:859
    break;

  case 23:
#line 152 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkIntLitNode(lextxt);}
#line 680 "src/parser.cpp" // lalr1.cc:859
    break;

  case 24:
#line 155 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFltLitNode(lextxt);}
#line 686 "src/parser.cpp" // lalr1.cc:859
    break;

  case 25:
#line 158 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkStrLitNode(lextxt);}
#line 692 "src/parser.cpp" // lalr1.cc:859
    break;

  case 26:
#line 161 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I8,  (char*)"");}
#line 698 "src/parser.cpp" // lalr1.cc:859
    break;

  case 27:
#line 162 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I16, (char*)"");}
#line 704 "src/parser.cpp" // lalr1.cc:859
    break;

  case 28:
#line 163 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I32, (char*)"");}
#line 710 "src/parser.cpp" // lalr1.cc:859
    break;

  case 29:
#line 164 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I64, (char*)"");}
#line 716 "src/parser.cpp" // lalr1.cc:859
    break;

  case 30:
#line 165 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U8,  (char*)"");}
#line 722 "src/parser.cpp" // lalr1.cc:859
    break;

  case 31:
#line 166 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U16, (char*)"");}
#line 728 "src/parser.cpp" // lalr1.cc:859
    break;

  case 32:
#line 167 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U32, (char*)"");}
#line 734 "src/parser.cpp" // lalr1.cc:859
    break;

  case 33:
#line 168 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U64, (char*)"");}
#line 740 "src/parser.cpp" // lalr1.cc:859
    break;

  case 34:
#line 169 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Isz, (char*)"");}
#line 746 "src/parser.cpp" // lalr1.cc:859
    break;

  case 35:
#line 170 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Usz, (char*)"");}
#line 752 "src/parser.cpp" // lalr1.cc:859
    break;

  case 36:
#line 171 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_F16, (char*)"");}
#line 758 "src/parser.cpp" // lalr1.cc:859
    break;

  case 37:
#line 172 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_F32, (char*)"");}
#line 764 "src/parser.cpp" // lalr1.cc:859
    break;

  case 38:
#line 173 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_F64, (char*)"");}
#line 770 "src/parser.cpp" // lalr1.cc:859
    break;

  case 39:
#line 174 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_C8,  (char*)"");}
#line 776 "src/parser.cpp" // lalr1.cc:859
    break;

  case 40:
#line 175 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_C32, (char*)"");}
#line 782 "src/parser.cpp" // lalr1.cc:859
    break;

  case 41:
#line 176 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Bool, (char*)"");}
#line 788 "src/parser.cpp" // lalr1.cc:859
    break;

  case 42:
#line 177 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Void, (char*)"");}
#line 794 "src/parser.cpp" // lalr1.cc:859
    break;

  case 43:
#line 178 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_UserType, (char*)(yystack_[0].value));}
#line 800 "src/parser.cpp" // lalr1.cc:859
    break;

  case 44:
#line 179 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Ident, (char*)(yystack_[0].value));}
#line 806 "src/parser.cpp" // lalr1.cc:859
    break;

  case 45:
#line 182 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('*', (char*)"", (yystack_[1].value));}
#line 812 "src/parser.cpp" // lalr1.cc:859
    break;

  case 46:
#line 183 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('[', (char*)"", (yystack_[3].value));}
#line 818 "src/parser.cpp" // lalr1.cc:859
    break;

  case 47:
#line 184 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('(', (char*)"", (yystack_[3].value));}
#line 824 "src/parser.cpp" // lalr1.cc:859
    break;

  case 48:
#line 185 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('(', (char*)"", (yystack_[2].value));}
#line 830 "src/parser.cpp" // lalr1.cc:859
    break;

  case 49:
#line 186 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_UserType, (char*)"", (yystack_[1].value));}
#line 836 "src/parser.cpp" // lalr1.cc:859
    break;

  case 50:
#line 187 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 842 "src/parser.cpp" // lalr1.cc:859
    break;

  case 51:
#line 190 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 848 "src/parser.cpp" // lalr1.cc:859
    break;

  case 53:
#line 192 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 854 "src/parser.cpp" // lalr1.cc:859
    break;

  case 54:
#line 195 "src/syntax.y" // lalr1.cc:859
    {Node* tmp = getRoot(); 
                        if(tmp == (yystack_[0].value)){//singular type, first type in list equals the last
                            (yylhs.value) = tmp;
                        }else{ //tuple type
                            (yylhs.value) = mkTypeNode(Tok_UserType, (char*)"", tmp);
                        }
                       }
#line 866 "src/parser.cpp" // lalr1.cc:859
    break;

  case 55:
#line 204 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pub);}
#line 872 "src/parser.cpp" // lalr1.cc:859
    break;

  case 56:
#line 205 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pri);}
#line 878 "src/parser.cpp" // lalr1.cc:859
    break;

  case 57:
#line 206 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pro);}
#line 884 "src/parser.cpp" // lalr1.cc:859
    break;

  case 58:
#line 207 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Raw);}
#line 890 "src/parser.cpp" // lalr1.cc:859
    break;

  case 59:
#line 208 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Const);}
#line 896 "src/parser.cpp" // lalr1.cc:859
    break;

  case 60:
#line 209 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Ext);}
#line 902 "src/parser.cpp" // lalr1.cc:859
    break;

  case 61:
#line 210 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Noinit);}
#line 908 "src/parser.cpp" // lalr1.cc:859
    break;

  case 62:
#line 211 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pathogen);}
#line 914 "src/parser.cpp" // lalr1.cc:859
    break;

  case 63:
#line 214 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[1].value), (yystack_[0].value));}
#line 920 "src/parser.cpp" // lalr1.cc:859
    break;

  case 64:
#line 215 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 926 "src/parser.cpp" // lalr1.cc:859
    break;

  case 65:
#line 218 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 932 "src/parser.cpp" // lalr1.cc:859
    break;

  case 66:
#line 222 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[2].value), (yystack_[4].value), (yystack_[3].value), (yystack_[0].value));}
#line 938 "src/parser.cpp" // lalr1.cc:859
    break;

  case 67:
#line 223 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[0].value), (yystack_[2].value), (yystack_[1].value),  0);}
#line 944 "src/parser.cpp" // lalr1.cc:859
    break;

  case 68:
#line 224 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[2].value), 0,  (yystack_[3].value), (yystack_[0].value));}
#line 950 "src/parser.cpp" // lalr1.cc:859
    break;

  case 69:
#line 225 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[0].value), 0,  (yystack_[1].value),  0);}
#line 956 "src/parser.cpp" // lalr1.cc:859
    break;

  case 70:
#line 228 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[3].value), (yystack_[4].value), (yystack_[3].value), (yystack_[0].value));}
#line 962 "src/parser.cpp" // lalr1.cc:859
    break;

  case 71:
#line 229 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[3].value), (yystack_[3].value), 0,  (yystack_[0].value));}
#line 968 "src/parser.cpp" // lalr1.cc:859
    break;

  case 72:
#line 230 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[2].value), 0,  (yystack_[3].value), (yystack_[0].value));}
#line 974 "src/parser.cpp" // lalr1.cc:859
    break;

  case 73:
#line 231 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[2].value), 0,  0,  (yystack_[0].value));}
#line 980 "src/parser.cpp" // lalr1.cc:859
    break;

  case 74:
#line 235 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarAssignNode((yystack_[2].value), (yystack_[0].value));}
#line 986 "src/parser.cpp" // lalr1.cc:859
    break;

  case 75:
#line 236 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarAssignNode((yystack_[2].value), mkBinOpNode('+', mkUnOpNode('*', (yystack_[2].value)), (yystack_[0].value)));}
#line 992 "src/parser.cpp" // lalr1.cc:859
    break;

  case 76:
#line 237 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarAssignNode((yystack_[2].value), mkBinOpNode('-', mkUnOpNode('*', (yystack_[2].value)), (yystack_[0].value)));}
#line 998 "src/parser.cpp" // lalr1.cc:859
    break;

  case 77:
#line 238 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarAssignNode((yystack_[2].value), mkBinOpNode('*', mkUnOpNode('*', (yystack_[2].value)), (yystack_[0].value)));}
#line 1004 "src/parser.cpp" // lalr1.cc:859
    break;

  case 78:
#line 239 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarAssignNode((yystack_[2].value), mkBinOpNode('/', mkUnOpNode('*', (yystack_[2].value)), (yystack_[0].value)));}
#line 1010 "src/parser.cpp" // lalr1.cc:859
    break;

  case 79:
#line 242 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 1016 "src/parser.cpp" // lalr1.cc:859
    break;

  case 80:
#line 243 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 1022 "src/parser.cpp" // lalr1.cc:859
    break;

  case 81:
#line 246 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1028 "src/parser.cpp" // lalr1.cc:859
    break;

  case 82:
#line 249 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[1].value), (yystack_[0].value));}
#line 1034 "src/parser.cpp" // lalr1.cc:859
    break;

  case 83:
#line 250 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[2].value), (yystack_[0].value));}
#line 1040 "src/parser.cpp" // lalr1.cc:859
    break;

  case 84:
#line 251 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[1].value), (yystack_[0].value));}
#line 1046 "src/parser.cpp" // lalr1.cc:859
    break;

  case 85:
#line 252 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[2].value), (yystack_[0].value));}
#line 1052 "src/parser.cpp" // lalr1.cc:859
    break;

  case 86:
#line 255 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkNamedValNode(mkVarNode((char*)(yystack_[0].value)), (yystack_[1].value));}
#line 1058 "src/parser.cpp" // lalr1.cc:859
    break;

  case 87:
#line 256 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkNamedValNode(0, (yystack_[0].value));}
#line 1064 "src/parser.cpp" // lalr1.cc:859
    break;

  case 89:
#line 260 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 1070 "src/parser.cpp" // lalr1.cc:859
    break;

  case 90:
#line 261 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 1076 "src/parser.cpp" // lalr1.cc:859
    break;

  case 91:
#line 264 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1082 "src/parser.cpp" // lalr1.cc:859
    break;

  case 97:
#line 277 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1088 "src/parser.cpp" // lalr1.cc:859
    break;

  case 98:
#line 278 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1094 "src/parser.cpp" // lalr1.cc:859
    break;

  case 99:
#line 279 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1100 "src/parser.cpp" // lalr1.cc:859
    break;

  case 100:
#line 280 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1106 "src/parser.cpp" // lalr1.cc:859
    break;

  case 101:
#line 283 "src/syntax.y" // lalr1.cc:859
    {setNext((yystack_[2].value), (yystack_[1].value)); (yylhs.value) = getRoot();}
#line 1112 "src/parser.cpp" // lalr1.cc:859
    break;

  case 102:
#line 284 "src/syntax.y" // lalr1.cc:859
    {setNext((yystack_[2].value), (yystack_[1].value)); (yylhs.value) = getRoot();}
#line 1118 "src/parser.cpp" // lalr1.cc:859
    break;

  case 103:
#line 285 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1124 "src/parser.cpp" // lalr1.cc:859
    break;

  case 104:
#line 286 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1130 "src/parser.cpp" // lalr1.cc:859
    break;

  case 105:
#line 289 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[1].value), mkVarNode((char*)(yystack_[0].value)));}
#line 1136 "src/parser.cpp" // lalr1.cc:859
    break;

  case 106:
#line 290 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkVarNode((char*)(yystack_[0].value)));}
#line 1142 "src/parser.cpp" // lalr1.cc:859
    break;

  case 107:
#line 298 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[3].value), mkNamedValNode(getRoot(), (yystack_[1].value)));}
#line 1148 "src/parser.cpp" // lalr1.cc:859
    break;

  case 108:
#line 299 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkNamedValNode(getRoot(), (yystack_[1].value)));}
#line 1154 "src/parser.cpp" // lalr1.cc:859
    break;

  case 109:
#line 302 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1160 "src/parser.cpp" // lalr1.cc:859
    break;

  case 110:
#line 303 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1166 "src/parser.cpp" // lalr1.cc:859
    break;

  case 111:
#line 306 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[3].value), (yystack_[5].value), (yystack_[4].value), (yystack_[1].value), (yystack_[0].value));}
#line 1172 "src/parser.cpp" // lalr1.cc:859
    break;

  case 112:
#line 307 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[6].value), (yystack_[8].value), (yystack_[7].value), (yystack_[1].value), (yystack_[0].value));}
#line 1178 "src/parser.cpp" // lalr1.cc:859
    break;

  case 113:
#line 308 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[3].value), 0,  (yystack_[4].value), (yystack_[1].value), (yystack_[0].value));}
#line 1184 "src/parser.cpp" // lalr1.cc:859
    break;

  case 114:
#line 309 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[6].value), 0,  (yystack_[7].value), (yystack_[1].value), (yystack_[0].value));}
#line 1190 "src/parser.cpp" // lalr1.cc:859
    break;

  case 115:
#line 312 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncCallNode((char*)(yystack_[1].value), (yystack_[0].value));}
#line 1196 "src/parser.cpp" // lalr1.cc:859
    break;

  case 116:
#line 315 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkRetNode((yystack_[0].value));}
#line 1202 "src/parser.cpp" // lalr1.cc:859
    break;

  case 117:
#line 318 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setElse((IfNode*)(yystack_[3].value), (IfNode*)mkIfNode((yystack_[1].value), (yystack_[0].value)));}
#line 1208 "src/parser.cpp" // lalr1.cc:859
    break;

  case 118:
#line 319 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkIfNode((yystack_[1].value), (yystack_[0].value)));}
#line 1214 "src/parser.cpp" // lalr1.cc:859
    break;

  case 119:
#line 322 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setElse((IfNode*)(yystack_[2].value), (IfNode*)mkIfNode(NULL, (yystack_[0].value)));}
#line 1220 "src/parser.cpp" // lalr1.cc:859
    break;

  case 120:
#line 323 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1226 "src/parser.cpp" // lalr1.cc:859
    break;

  case 121:
#line 324 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkIfNode(NULL, (yystack_[0].value)));}
#line 1232 "src/parser.cpp" // lalr1.cc:859
    break;

  case 122:
#line 325 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(NULL);}
#line 1238 "src/parser.cpp" // lalr1.cc:859
    break;

  case 123:
#line 328 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkIfNode((yystack_[2].value), (yystack_[1].value), (IfNode*)getRoot());}
#line 1244 "src/parser.cpp" // lalr1.cc:859
    break;

  case 124:
#line 331 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1250 "src/parser.cpp" // lalr1.cc:859
    break;

  case 125:
#line 334 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1256 "src/parser.cpp" // lalr1.cc:859
    break;

  case 126:
#line 337 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1262 "src/parser.cpp" // lalr1.cc:859
    break;

  case 127:
#line 340 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarNode((char*)(yystack_[0].value));}
#line 1268 "src/parser.cpp" // lalr1.cc:859
    break;

  case 128:
#line 343 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('&', (yystack_[0].value));}
#line 1274 "src/parser.cpp" // lalr1.cc:859
    break;

  case 129:
#line 344 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('*', (yystack_[0].value));}
#line 1280 "src/parser.cpp" // lalr1.cc:859
    break;

  case 131:
#line 346 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkRefVarNode((char*)(yystack_[0].value));}
#line 1286 "src/parser.cpp" // lalr1.cc:859
    break;

  case 132:
#line 349 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1292 "src/parser.cpp" // lalr1.cc:859
    break;

  case 133:
#line 350 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1298 "src/parser.cpp" // lalr1.cc:859
    break;

  case 134:
#line 351 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1304 "src/parser.cpp" // lalr1.cc:859
    break;

  case 135:
#line 352 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1310 "src/parser.cpp" // lalr1.cc:859
    break;

  case 136:
#line 353 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1316 "src/parser.cpp" // lalr1.cc:859
    break;

  case 137:
#line 354 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1322 "src/parser.cpp" // lalr1.cc:859
    break;

  case 138:
#line 355 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1328 "src/parser.cpp" // lalr1.cc:859
    break;

  case 139:
#line 356 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1334 "src/parser.cpp" // lalr1.cc:859
    break;

  case 140:
#line 357 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1340 "src/parser.cpp" // lalr1.cc:859
    break;

  case 141:
#line 358 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1346 "src/parser.cpp" // lalr1.cc:859
    break;

  case 142:
#line 359 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBoolLitNode(1);}
#line 1352 "src/parser.cpp" // lalr1.cc:859
    break;

  case 143:
#line 360 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBoolLitNode(0);}
#line 1358 "src/parser.cpp" // lalr1.cc:859
    break;

  case 144:
#line 363 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTupleNode((yystack_[1].value));}
#line 1364 "src/parser.cpp" // lalr1.cc:859
    break;

  case 145:
#line 364 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTupleNode(0);}
#line 1370 "src/parser.cpp" // lalr1.cc:859
    break;

  case 146:
#line 367 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkArrayNode((yystack_[1].value));}
#line 1376 "src/parser.cpp" // lalr1.cc:859
    break;

  case 147:
#line 368 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkArrayNode(0);}
#line 1382 "src/parser.cpp" // lalr1.cc:859
    break;

  case 148:
#line 371 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1388 "src/parser.cpp" // lalr1.cc:859
    break;

  case 149:
#line 372 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1394 "src/parser.cpp" // lalr1.cc:859
    break;

  case 150:
#line 375 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1400 "src/parser.cpp" // lalr1.cc:859
    break;

  case 151:
#line 378 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 1406 "src/parser.cpp" // lalr1.cc:859
    break;

  case 152:
#line 379 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 1412 "src/parser.cpp" // lalr1.cc:859
    break;

  case 153:
#line 384 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('*', (yystack_[0].value));}
#line 1418 "src/parser.cpp" // lalr1.cc:859
    break;

  case 154:
#line 385 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('&', (yystack_[0].value));}
#line 1424 "src/parser.cpp" // lalr1.cc:859
    break;

  case 155:
#line 386 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('-', (yystack_[0].value));}
#line 1430 "src/parser.cpp" // lalr1.cc:859
    break;

  case 156:
#line 389 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Where, (yystack_[4].value), mkLetBindingNode((char*)(yystack_[2].value), 0, 0, (yystack_[0].value)));}
#line 1436 "src/parser.cpp" // lalr1.cc:859
    break;

  case 157:
#line 390 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1442 "src/parser.cpp" // lalr1.cc:859
    break;

  case 158:
#line 393 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('+', (yystack_[2].value), (yystack_[0].value));}
#line 1448 "src/parser.cpp" // lalr1.cc:859
    break;

  case 159:
#line 394 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('-', (yystack_[2].value), (yystack_[0].value));}
#line 1454 "src/parser.cpp" // lalr1.cc:859
    break;

  case 160:
#line 395 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('*', (yystack_[2].value), (yystack_[0].value));}
#line 1460 "src/parser.cpp" // lalr1.cc:859
    break;

  case 161:
#line 396 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('/', (yystack_[2].value), (yystack_[0].value));}
#line 1466 "src/parser.cpp" // lalr1.cc:859
    break;

  case 162:
#line 397 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('%', (yystack_[2].value), (yystack_[0].value));}
#line 1472 "src/parser.cpp" // lalr1.cc:859
    break;

  case 163:
#line 398 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('<', (yystack_[2].value), (yystack_[0].value));}
#line 1478 "src/parser.cpp" // lalr1.cc:859
    break;

  case 164:
#line 399 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('>', (yystack_[2].value), (yystack_[0].value));}
#line 1484 "src/parser.cpp" // lalr1.cc:859
    break;

  case 165:
#line 400 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('^', (yystack_[2].value), (yystack_[0].value));}
#line 1490 "src/parser.cpp" // lalr1.cc:859
    break;

  case 166:
#line 401 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('.', (yystack_[2].value), (yystack_[0].value));}
#line 1496 "src/parser.cpp" // lalr1.cc:859
    break;

  case 167:
#line 402 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('[', (yystack_[3].value), (yystack_[1].value));}
#line 1502 "src/parser.cpp" // lalr1.cc:859
    break;

  case 168:
#line 403 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Eq, (yystack_[2].value), (yystack_[0].value));}
#line 1508 "src/parser.cpp" // lalr1.cc:859
    break;

  case 169:
#line 404 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_NotEq, (yystack_[2].value), (yystack_[0].value));}
#line 1514 "src/parser.cpp" // lalr1.cc:859
    break;

  case 170:
#line 405 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_GrtrEq, (yystack_[2].value), (yystack_[0].value));}
#line 1520 "src/parser.cpp" // lalr1.cc:859
    break;

  case 171:
#line 406 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_LesrEq, (yystack_[2].value), (yystack_[0].value));}
#line 1526 "src/parser.cpp" // lalr1.cc:859
    break;

  case 172:
#line 407 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Or, (yystack_[2].value), (yystack_[0].value));}
#line 1532 "src/parser.cpp" // lalr1.cc:859
    break;

  case 173:
#line 408 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_And, (yystack_[2].value), (yystack_[0].value));}
#line 1538 "src/parser.cpp" // lalr1.cc:859
    break;

  case 174:
#line 409 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1544 "src/parser.cpp" // lalr1.cc:859
    break;

  case 175:
#line 414 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1550 "src/parser.cpp" // lalr1.cc:859
    break;

  case 176:
#line 417 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[3].value), (yystack_[0].value));}
#line 1556 "src/parser.cpp" // lalr1.cc:859
    break;

  case 177:
#line 418 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 1562 "src/parser.cpp" // lalr1.cc:859
    break;

  case 178:
#line 421 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('+', (yystack_[3].value), (yystack_[0].value));}
#line 1568 "src/parser.cpp" // lalr1.cc:859
    break;

  case 179:
#line 422 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('-', (yystack_[3].value), (yystack_[0].value));}
#line 1574 "src/parser.cpp" // lalr1.cc:859
    break;

  case 180:
#line 423 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('*', (yystack_[3].value), (yystack_[0].value));}
#line 1580 "src/parser.cpp" // lalr1.cc:859
    break;

  case 181:
#line 424 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('/', (yystack_[3].value), (yystack_[0].value));}
#line 1586 "src/parser.cpp" // lalr1.cc:859
    break;

  case 182:
#line 425 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('%', (yystack_[3].value), (yystack_[0].value));}
#line 1592 "src/parser.cpp" // lalr1.cc:859
    break;

  case 183:
#line 426 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('<', (yystack_[3].value), (yystack_[0].value));}
#line 1598 "src/parser.cpp" // lalr1.cc:859
    break;

  case 184:
#line 427 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('>', (yystack_[3].value), (yystack_[0].value));}
#line 1604 "src/parser.cpp" // lalr1.cc:859
    break;

  case 185:
#line 428 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('^', (yystack_[3].value), (yystack_[0].value));}
#line 1610 "src/parser.cpp" // lalr1.cc:859
    break;

  case 186:
#line 429 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('.', (yystack_[3].value), (yystack_[0].value));}
#line 1616 "src/parser.cpp" // lalr1.cc:859
    break;

  case 187:
#line 430 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Eq, (yystack_[3].value), (yystack_[0].value));}
#line 1622 "src/parser.cpp" // lalr1.cc:859
    break;

  case 188:
#line 431 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_NotEq, (yystack_[3].value), (yystack_[0].value));}
#line 1628 "src/parser.cpp" // lalr1.cc:859
    break;

  case 189:
#line 432 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_GrtrEq, (yystack_[3].value), (yystack_[0].value));}
#line 1634 "src/parser.cpp" // lalr1.cc:859
    break;

  case 190:
#line 433 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_LesrEq, (yystack_[3].value), (yystack_[0].value));}
#line 1640 "src/parser.cpp" // lalr1.cc:859
    break;

  case 191:
#line 434 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Or, (yystack_[3].value), (yystack_[0].value));}
#line 1646 "src/parser.cpp" // lalr1.cc:859
    break;

  case 192:
#line 435 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_And, (yystack_[3].value), (yystack_[0].value));}
#line 1652 "src/parser.cpp" // lalr1.cc:859
    break;

  case 193:
#line 436 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1658 "src/parser.cpp" // lalr1.cc:859
    break;


#line 1662 "src/parser.cpp" // lalr1.cc:859
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


  const short int parser::yypact_ninf_ = -258;

  const signed char parser::yytable_ninf_ = -45;

  const short int
  parser::yypact_[] =
  {
     -43,  -258,    24,   483,  -258,  -258,  -258,  -258,  -258,  -258,
    -258,  -258,  -258,  -258,  -258,  -258,  -258,  -258,  -258,  -258,
    -258,  -258,  -258,  -258,   478,   478,   586,   478,     0,   586,
      41,     7,  -258,  -258,  -258,  -258,  -258,  -258,  -258,  -258,
      25,   712,    25,   399,  -258,   -15,    40,  -258,  -258,   -46,
     -36,    55,  -258,   246,   663,  -258,  -258,  -258,  -258,  -258,
    -258,  -258,  -258,  -258,  -258,  -258,  -258,    28,  -258,  -258,
    -258,  -258,  -258,   478,   478,   478,   245,   257,   478,     4,
    -258,  -258,  -258,  -258,  -258,  -258,  -258,  -258,  -258,    35,
     723,   -30,  -258,    55,   712,    44,   -30,   478,    15,    55,
     712,    13,    41,    50,  -258,    38,  -258,    43,  -258,  -258,
    -258,    60,  -258,   245,   478,  -258,  -258,   682,   478,   712,
     712,   -11,  -258,    41,     7,    55,   478,   478,   478,   478,
     478,  -258,    64,    65,   759,  -258,  -258,  -258,    66,    79,
     -42,  -258,    68,    35,  -258,    55,   478,   478,   478,   478,
     478,   478,   478,   478,   478,   478,   478,   478,   478,   478,
     478,   478,   483,    70,    71,    55,   478,  -258,   -30,   478,
      74,    80,    55,   567,    41,   111,  -258,    88,    20,  -258,
    -258,  -258,   -41,  -258,   109,    98,    35,   -46,   -46,   478,
     478,   712,    13,    50,  -258,    -8,    35,    35,    35,    35,
      35,  -258,   -43,   -43,   -43,   -43,   -43,   -43,   -43,   -43,
     -43,   -43,   -43,   -43,   -43,   -43,   -43,   -43,  -258,   478,
    -258,  -258,   101,   -10,   -10,   -10,   -10,   735,   747,   -10,
     -10,    52,    52,   -20,   -20,   -20,   -20,   121,   -38,   483,
     136,    23,   478,   124,   128,  -258,   125,   -30,  -258,    35,
     478,   478,   126,    55,   161,  -258,    97,  -258,  -258,    73,
    -258,   478,    41,  -258,  -258,  -258,  -258,   137,    35,    55,
     145,   124,   111,  -258,  -258,   478,   478,   712,   478,   478,
     478,   478,   478,   478,   478,   478,   478,   478,   478,   478,
     478,   478,   478,   478,    35,   478,  -258,   150,   107,  -258,
    -258,   -30,  -258,   478,   124,  -258,    35,    35,   478,  -258,
     567,  -258,    41,  -258,    35,   151,   149,  -258,    55,   712,
    -258,  -258,   162,    35,   124,   759,   302,   302,   302,   302,
     815,   123,   302,   302,    62,    62,   102,   102,   102,   102,
    -258,   723,  -258,  -258,  -258,   -30,  -258,    35,  -258,  -258,
     478,   712,  -258,    55,   154,  -258,  -258,    35,   124,    55,
     712,  -258,   124,  -258
  };

  const unsigned char
  parser::yydefact_[] =
  {
       8,     7,     0,     0,     1,    21,    22,    26,    27,    28,
      29,    30,    31,    32,    33,    34,    35,    36,    37,    38,
      39,    40,    41,    42,     0,     0,     0,     0,     0,     0,
       0,     0,    55,    56,    57,    58,    59,    60,    61,    62,
       0,     0,     0,     8,     6,     0,   131,    43,    50,    53,
      54,     0,    64,    65,     0,    16,    20,    17,    10,    11,
       9,    18,    19,    15,    12,    13,    14,     0,   142,   143,
      23,    24,    25,     0,     0,     0,     0,     0,     0,   127,
     139,   140,   141,   132,   138,   174,   134,   135,   137,   116,
     157,     0,    44,     0,     0,     0,     0,     0,    44,     0,
       0,     0,     0,     0,   100,   131,   129,     0,   128,     2,
       4,     0,     5,     0,     0,   115,    45,     0,   149,     0,
       0,    69,    63,     0,     0,     0,     0,     0,     0,     0,
       0,   193,     0,   175,   177,   155,   153,   145,     0,   150,
     152,   147,     0,   152,   154,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,   122,    69,     0,     0,   124,     0,     0,
       0,    44,     0,     0,     0,     0,    84,    95,     0,    98,
      49,     3,     0,    48,     0,     0,   148,    51,    52,   149,
       0,   110,     0,     0,    99,    67,    75,    76,    77,    78,
      74,   136,     8,     8,     8,     8,     8,     8,     8,     8,
       8,     8,     8,     8,     8,     8,     8,     8,   144,     0,
     133,   146,     0,   168,   169,   170,   171,   172,   173,   163,
     164,   158,   159,   160,   161,   162,   165,   166,     0,     0,
       6,     0,     0,     0,   120,   123,    67,     0,   125,    73,
       0,     0,     0,    87,     0,    90,     0,    88,    80,     0,
      85,     0,     0,    96,   130,    47,    46,     0,    68,     0,
     109,     0,     0,    82,    97,   149,     0,   110,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,   151,     0,   167,     4,     0,   103,
     104,     0,   121,     0,     0,   126,    72,    71,     0,    86,
       0,    91,     0,    81,    94,    92,     0,   106,   108,     0,
     113,    83,     0,    66,     0,   176,   187,   188,   189,   190,
     191,   192,   183,   184,   178,   179,   180,   181,   182,   185,
     186,   156,   101,   102,   118,     0,   119,    70,    89,    79,
       0,   110,   105,     0,     0,   111,   117,    93,     0,   107,
     110,   114,     0,   112
  };

  const short int
  parser::yypgoto_[] =
  {
    -258,  -258,    82,    16,   -31,   -28,    -3,   -17,  -258,  -258,
    -258,  -258,   -40,  -258,   -25,   188,  -258,   -24,   221,  -258,
    -258,  -258,    57,  -258,   -59,  -258,  -167,  -258,   -93,  -170,
     -87,  -100,  -258,  -257,  -258,     3,  -258,  -258,  -258,  -258,
    -258,  -258,  -258,  -258,   135,   108,   -29,  -258,  -182,   177,
    -258,  -258,    77,   212,  -258,  -258,   369
  };

  const short int
  parser::yydefgoto_[] =
  {
      -1,     2,    43,     3,    44,    45,    79,    47,    80,    81,
      82,    48,    49,    50,    51,    52,    53,    54,    55,    56,
      57,   259,   175,    58,   255,   256,   176,   178,   104,    59,
     163,   318,   270,   271,    60,    83,    62,   244,   245,    63,
      64,    65,    66,    84,    67,    85,    86,    87,   185,   138,
     139,    88,   186,    90,   132,   133,   134
  };

  const short int
  parser::yytable_[] =
  {
      46,    93,    94,   257,    99,   100,    61,   267,   260,   167,
     179,     6,   110,   101,   103,   111,   107,   115,   145,   145,
     324,     1,   145,    92,     4,   273,    98,   116,     5,   125,
     145,   194,   119,   117,   118,   162,   220,   105,    92,   105,
      46,   264,    97,   -44,   296,     6,    61,   120,   121,   112,
     115,    92,   126,   127,   128,   129,   159,   160,     5,   109,
     161,   154,   155,   156,   157,   158,   159,   160,   189,   165,
     161,   275,   102,   190,   191,   172,   276,   277,   173,   187,
     188,   248,   174,   113,   262,   177,   263,   112,   166,   300,
     164,    92,   184,   322,   358,   145,   170,   171,    40,   169,
     274,    89,    91,   362,    96,   321,   192,   193,   -44,   242,
     243,    42,   130,   -44,    92,   102,    92,    92,   114,   113,
     114,   180,   195,   -44,   181,   156,   157,   158,   159,   160,
     201,   240,   161,   202,   241,   213,   214,   215,   216,   217,
     257,   312,   222,   313,   218,   203,   204,   219,   253,   254,
     221,   205,   206,   140,   143,   190,   302,   258,   250,    46,
     305,   310,   246,   311,   251,    61,   269,   303,   304,   252,
      92,   181,   261,   343,   168,   106,   173,   108,   216,   217,
     266,   131,   135,   136,   320,   295,   144,   265,    92,   162,
     143,   182,   209,   210,   211,   212,   213,   214,   215,   216,
     217,   161,   299,   196,   197,   198,   199,   200,   297,   276,
     308,   298,   124,   319,   344,   316,   342,   346,   278,   279,
     280,   281,   282,   283,   284,   285,   286,   287,   288,   289,
     290,   291,   292,   293,   351,   350,    46,   355,   238,   360,
     354,   122,    61,   247,   239,   315,   249,    95,     5,   272,
     309,   348,   269,   359,   142,     0,     0,     0,   356,     0,
       5,     0,     0,     0,     0,     0,   317,   268,     0,     0,
       0,   361,     0,     0,    92,   363,     0,    68,    69,    70,
      71,    72,     0,     0,     0,   253,   254,     0,     0,    68,
      69,    70,    71,    72,   353,   349,   294,     0,    32,    33,
      34,    35,    36,    37,    38,    39,     0,    92,     0,     0,
      73,     0,     0,     0,     0,   352,    92,    74,    75,   301,
       0,     0,    73,   137,    76,    77,   269,   306,   307,    74,
      75,    78,     0,     0,     0,   269,    76,    77,   314,   141,
       0,     0,     0,    78,     0,     0,     0,     0,    92,     0,
     317,     0,     0,   323,     0,     0,   352,    92,   223,   224,
     225,   226,   227,   228,   229,   230,   231,   232,   233,   234,
     235,   236,   237,   211,   212,   213,   214,   215,   216,   217,
     345,     0,     0,     0,     0,   347,   131,   131,   131,   131,
     131,   131,   131,   131,   131,   131,   131,   131,   131,   131,
     131,   131,     5,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    21,    22,
      23,     0,     0,     0,     0,     0,     0,   357,     0,     0,
       0,     0,     0,     0,     0,     0,    24,    25,     0,     0,
      26,    27,    28,     0,     0,     0,     0,    29,     0,    30,
      31,    32,    33,    34,    35,    36,    37,    38,    39,     0,
       0,     0,     0,     1,     0,     0,     0,     0,     0,     0,
       0,     0,    40,     0,     0,     0,     0,     0,    41,     0,
       0,     5,     0,     0,     0,    42,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    22,    23,     0,     0,   341,     0,     0,
      68,    69,    70,    71,    72,     0,     0,     0,     0,     0,
      24,    25,     0,     0,    26,    27,    28,     0,     0,     0,
       0,    29,     0,    30,    31,    32,    33,    34,    35,    36,
      37,    38,    39,    73,     0,     0,     0,     0,     0,     0,
      74,    75,     0,     0,     0,     0,    40,    76,    77,     0,
       0,     0,    41,     0,    78,     0,     0,     0,     0,    42,
       5,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    22,    23,     5,
       6,     7,     8,     9,    10,    11,    12,    13,    14,    15,
      16,    17,    18,    19,    20,    21,    22,    23,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,    31,    32,
      33,    34,    35,    36,    37,    38,    39,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,    32,    33,
      34,    35,    36,    37,    38,    39,    41,   325,   326,   327,
     328,   329,   330,   331,   332,   333,   334,   335,   336,   337,
     338,   339,   340,     0,     0,    41,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    22,    23,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    22,    23,     0,     0,     0,     0,     0,     0,
       0,     0,     0,   123,   124,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    22,    23,     0,     0,     0,     0,     0,     0,
       0,     0,    41,     0,     0,   146,   147,     0,     0,     0,
       0,   148,   149,   150,   151,     0,     0,   146,   147,     0,
     183,    41,     0,   148,   149,     0,   151,     0,     0,   146,
     147,     0,     0,     0,     0,   148,   149,     0,     0,     0,
       0,   203,   204,     0,     0,     0,     0,   205,   206,   207,
     208,    41,   152,   153,   154,   155,   156,   157,   158,   159,
     160,     0,     0,   161,   152,   153,   154,   155,   156,   157,
     158,   159,   160,     0,     0,   161,   152,   153,   154,   155,
     156,   157,   158,   159,   160,     0,     0,   161,   209,   210,
     211,   212,   213,   214,   215,   216,   217,   203,   204,     0,
       0,     0,     0,   205,   206,     0,   208,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,   209,   210,   211,   212,   213,   214,
     215,   216,   217
  };

  const short int
  parser::yycheck_[] =
  {
       3,    26,    26,   173,    29,    29,     3,   189,   175,    96,
     103,     4,    43,    30,    31,    43,    41,    46,    60,    60,
     277,    64,    60,    26,     0,   192,    29,    73,     3,    54,
      60,   124,    68,    79,    80,    65,    78,    40,    41,    42,
      43,    82,    42,     3,    82,     4,    43,    83,    51,    64,
      79,    54,    24,    25,    26,    27,    76,    77,     3,    43,
      80,    71,    72,    73,    74,    75,    76,    77,    79,    94,
      80,    79,    65,    84,    85,   100,    84,    85,    65,   119,
     120,   168,    69,    79,    64,   102,    66,    64,    44,    66,
      93,    94,   117,   275,   351,    60,    99,   100,    73,    84,
     193,    24,    25,   360,    27,   272,   123,   124,    68,    39,
      40,    86,    84,    73,   117,    65,   119,   120,    80,    79,
      80,    78,   125,    83,    64,    73,    74,    75,    76,    77,
      66,   162,    80,    68,   162,    73,    74,    75,    76,    77,
     310,    68,   145,    70,    78,    22,    23,    68,   173,   173,
      82,    28,    29,    76,    77,    84,   243,   174,    84,   162,
     247,    64,   165,    66,    84,   162,   191,    39,    40,   172,
     173,    64,    84,    66,    97,    40,    65,    42,    76,    77,
      82,    73,    74,    75,   271,    84,    78,    78,   191,    65,
     113,   114,    69,    70,    71,    72,    73,    74,    75,    76,
      77,    80,    66,   126,   127,   128,   129,   130,   239,    84,
      84,   239,    51,    68,   301,    78,    66,   304,   202,   203,
     204,   205,   206,   207,   208,   209,   210,   211,   212,   213,
     214,   215,   216,   217,    85,    84,   239,   324,   161,    85,
      78,    53,   239,   166,   162,   262,   169,    26,     3,   192,
     253,   310,   277,   353,    77,    -1,    -1,    -1,   345,    -1,
       3,    -1,    -1,    -1,    -1,    -1,   269,   190,    -1,    -1,
      -1,   358,    -1,    -1,   277,   362,    -1,    32,    33,    34,
      35,    36,    -1,    -1,    -1,   310,   310,    -1,    -1,    32,
      33,    34,    35,    36,   319,   312,   219,    -1,    52,    53,
      54,    55,    56,    57,    58,    59,    -1,   310,    -1,    -1,
      65,    -1,    -1,    -1,    -1,   318,   319,    72,    73,   242,
      -1,    -1,    65,    78,    79,    80,   351,   250,   251,    72,
      73,    86,    -1,    -1,    -1,   360,    79,    80,   261,    82,
      -1,    -1,    -1,    86,    -1,    -1,    -1,    -1,   351,    -1,
     353,    -1,    -1,   276,    -1,    -1,   359,   360,   146,   147,
     148,   149,   150,   151,   152,   153,   154,   155,   156,   157,
     158,   159,   160,    71,    72,    73,    74,    75,    76,    77,
     303,    -1,    -1,    -1,    -1,   308,   278,   279,   280,   281,
     282,   283,   284,   285,   286,   287,   288,   289,   290,   291,
     292,   293,     3,     4,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    -1,    -1,    -1,    -1,    -1,    -1,   350,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    37,    38,    -1,    -1,
      41,    42,    43,    -1,    -1,    -1,    -1,    48,    -1,    50,
      51,    52,    53,    54,    55,    56,    57,    58,    59,    -1,
      -1,    -1,    -1,    64,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    73,    -1,    -1,    -1,    -1,    -1,    79,    -1,
      -1,     3,    -1,    -1,    -1,    86,     3,     4,     5,     6,
       7,     8,     9,    10,    11,    12,    13,    14,    15,    16,
      17,    18,    19,    20,    21,    -1,    -1,   295,    -1,    -1,
      32,    33,    34,    35,    36,    -1,    -1,    -1,    -1,    -1,
      37,    38,    -1,    -1,    41,    42,    43,    -1,    -1,    -1,
      -1,    48,    -1,    50,    51,    52,    53,    54,    55,    56,
      57,    58,    59,    65,    -1,    -1,    -1,    -1,    -1,    -1,
      72,    73,    -1,    -1,    -1,    -1,    73,    79,    80,    -1,
      -1,    -1,    79,    -1,    86,    -1,    -1,    -1,    -1,    86,
       3,     4,     5,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    21,     3,
       4,     5,     6,     7,     8,     9,    10,    11,    12,    13,
      14,    15,    16,    17,    18,    19,    20,    21,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    51,    52,
      53,    54,    55,    56,    57,    58,    59,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    52,    53,
      54,    55,    56,    57,    58,    59,    79,   278,   279,   280,
     281,   282,   283,   284,   285,   286,   287,   288,   289,   290,
     291,   292,   293,    -1,    -1,    79,     3,     4,     5,     6,
       7,     8,     9,    10,    11,    12,    13,    14,    15,    16,
      17,    18,    19,    20,    21,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    50,    51,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    79,    -1,    -1,    22,    23,    -1,    -1,    -1,
      -1,    28,    29,    30,    31,    -1,    -1,    22,    23,    -1,
      78,    79,    -1,    28,    29,    -1,    31,    -1,    -1,    22,
      23,    -1,    -1,    -1,    -1,    28,    29,    -1,    -1,    -1,
      -1,    22,    23,    -1,    -1,    -1,    -1,    28,    29,    30,
      31,    79,    69,    70,    71,    72,    73,    74,    75,    76,
      77,    -1,    -1,    80,    69,    70,    71,    72,    73,    74,
      75,    76,    77,    -1,    -1,    80,    69,    70,    71,    72,
      73,    74,    75,    76,    77,    -1,    -1,    80,    69,    70,
      71,    72,    73,    74,    75,    76,    77,    22,    23,    -1,
      -1,    -1,    -1,    28,    29,    -1,    31,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    69,    70,    71,    72,    73,    74,
      75,    76,    77
  };

  const unsigned char
  parser::yystos_[] =
  {
       0,    64,    88,    90,     0,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    37,    38,    41,    42,    43,    48,
      50,    51,    52,    53,    54,    55,    56,    57,    58,    59,
      73,    79,    86,    89,    91,    92,    93,    94,    98,    99,
     100,   101,   102,   103,   104,   105,   106,   107,   110,   116,
     121,   122,   123,   126,   127,   128,   129,   131,    32,    33,
      34,    35,    36,    65,    72,    73,    79,    80,    86,    93,
      95,    96,    97,   122,   130,   132,   133,   134,   138,   139,
     140,   139,    93,   101,   104,   105,   139,    42,    93,   101,
     104,    94,    65,    94,   115,    93,   131,   101,   131,    90,
      91,    92,    64,    79,    80,   133,    73,    79,    80,    68,
      83,    93,   102,    50,    51,   101,    24,    25,    26,    27,
      84,   132,   141,   142,   143,   132,   132,    78,   136,   137,
     139,    82,   136,   139,   132,    60,    22,    23,    28,    29,
      30,    31,    69,    70,    71,    72,    73,    74,    75,    76,
      77,    80,    65,   117,    93,   101,    44,   117,   139,    84,
      93,    93,   101,    65,    69,   109,   113,    94,   114,   115,
      78,    64,   139,    78,   101,   135,   139,    99,    99,    79,
      84,    85,    94,    94,   115,    93,   139,   139,   139,   139,
     139,    66,    68,    22,    23,    28,    29,    30,    31,    69,
      70,    71,    72,    73,    74,    75,    76,    77,    78,    68,
      78,    82,    93,   140,   140,   140,   140,   140,   140,   140,
     140,   140,   140,   140,   140,   140,   140,   140,   139,    89,
      91,    92,    39,    40,   124,   125,    93,   139,   117,   139,
      84,    84,    93,   101,   104,   111,   112,   116,    94,   108,
     113,    84,    64,    66,    82,    78,    82,   135,   139,   101,
     119,   120,   109,   113,   115,    79,    84,    85,    90,    90,
      90,    90,    90,    90,    90,    90,    90,    90,    90,    90,
      90,    90,    90,    90,   139,    84,    82,    91,    92,    66,
      66,   139,   117,    39,    40,   117,   139,   139,    84,    93,
      64,    66,    68,    70,   139,    94,    78,    93,   118,    68,
     117,   113,   135,   139,   120,   143,   143,   143,   143,   143,
     143,   143,   143,   143,   143,   143,   143,   143,   143,   143,
     143,   140,    66,    66,   117,   139,   117,   139,   111,    94,
      84,    85,    93,   101,    78,   117,   117,   139,   120,   118,
      85,   117,   120,   117
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
     106,   106,   106,   106,   107,   107,   107,   107,   107,   108,
     108,   109,   110,   110,   110,   110,   111,   111,   111,   112,
     112,   113,   114,   114,   114,   114,   115,   116,   116,   116,
     116,   117,   117,   117,   117,   118,   118,   119,   119,   120,
     120,   121,   121,   121,   121,   122,   123,   124,   124,   125,
     125,   125,   125,   126,   127,   128,   129,   130,   131,   131,
     131,   131,   132,   132,   132,   132,   132,   132,   132,   132,
     132,   132,   132,   132,   133,   133,   134,   134,   135,   135,
     136,   137,   137,   138,   138,   138,   139,   139,   140,   140,
     140,   140,   140,   140,   140,   140,   140,   140,   140,   140,
     140,   140,   140,   140,   140,   141,   142,   142,   143,   143,
     143,   143,   143,   143,   143,   143,   143,   143,   143,   143,
     143,   143,   143,   143
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
       6,     5,     5,     4,     3,     3,     3,     3,     3,     3,
       1,     3,     4,     5,     3,     4,     2,     1,     1,     3,
       1,     3,     3,     5,     3,     1,     3,     4,     3,     3,
       2,     4,     4,     3,     3,     2,     1,     4,     2,     1,
       0,     6,     9,     5,     8,     2,     2,     4,     3,     3,
       1,     2,     0,     4,     3,     4,     5,     1,     2,     2,
       4,     1,     1,     3,     1,     1,     3,     1,     1,     1,
       1,     1,     1,     1,     3,     2,     3,     2,     1,     0,
       1,     3,     1,     2,     2,     2,     5,     1,     3,     3,
       3,     3,     3,     3,     3,     3,     3,     4,     3,     3,
       3,     3,     3,     3,     1,     1,     4,     1,     4,     4,
       4,     4,     4,     4,     4,     4,     4,     4,     4,     4,
       4,     4,     4,     1
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
  "'.'", "')'", "'('", "'['", "HIGH", "']'", "'|'", "'='", "':'", "'&'",
  "$accept", "top_level_stmt_list", "stmt_list", "maybe_newline",
  "no_nl_stmt", "nl_stmt", "ident", "usertype", "intlit", "fltlit",
  "strlit", "lit_type", "type", "type_expr_", "type_expr", "modifier",
  "modifier_list_", "modifier_list", "var_decl", "let_binding",
  "var_assign", "usertype_list", "generic", "data_decl", "type_decl",
  "type_decl_list", "type_decl_block", "val_init_list", "enum_block",
  "enum_decl", "block", "ident_list", "params", "maybe_params", "fn_decl",
  "fn_call", "ret_stmt", "elif_list", "maybe_elif_list", "if_stmt",
  "while_loop", "do_while_loop", "for_loop", "var", "ref_val", "val",
  "tuple", "array", "maybe_expr", "expr_list", "expr_list_p", "unary_op",
  "expr", "binop", "nl_expr", "nl_expr_list", "expr_block_p", YY_NULLPTR
  };

#if YYDEBUG
  const unsigned short int
  parser::yyrline_[] =
  {
       0,   112,   112,   115,   116,   117,   118,   121,   122,   129,
     130,   131,   132,   133,   134,   135,   139,   140,   141,   142,
     143,   146,   149,   152,   155,   158,   161,   162,   163,   164,
     165,   166,   167,   168,   169,   170,   171,   172,   173,   174,
     175,   176,   177,   178,   179,   182,   183,   184,   185,   186,
     187,   190,   191,   192,   195,   204,   205,   206,   207,   208,
     209,   210,   211,   214,   215,   218,   222,   223,   224,   225,
     228,   229,   230,   231,   235,   236,   237,   238,   239,   242,
     243,   246,   249,   250,   251,   252,   255,   256,   257,   260,
     261,   264,   268,   269,   270,   271,   274,   277,   278,   279,
     280,   283,   284,   285,   286,   289,   290,   298,   299,   302,
     303,   306,   307,   308,   309,   312,   315,   318,   319,   322,
     323,   324,   325,   328,   331,   334,   337,   340,   343,   344,
     345,   346,   349,   350,   351,   352,   353,   354,   355,   356,
     357,   358,   359,   360,   363,   364,   367,   368,   371,   372,
     375,   378,   379,   384,   385,   386,   389,   390,   393,   394,
     395,   396,   397,   398,   399,   400,   401,   402,   403,   404,
     405,   406,   407,   408,   409,   414,   417,   418,   421,   422,
     423,   424,   425,   426,   427,   428,   429,   430,   431,   432,
     433,   434,   435,   436
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
      79,    78,    73,    71,    68,    72,    77,    74,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,    85,     2,
      69,    84,    70,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,    80,     2,    82,    76,     2,     2,     2,     2,     2,
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
      65,    66,    67,    81
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
#line 2450 "src/parser.cpp" // lalr1.cc:1167
#line 438 "src/syntax.y" // lalr1.cc:1168


void yy::parser::error(const location& loc, const string& msg){
    ante::error(msg.c_str(), yylexer->fileName, yylexer->getRow(), yylexer->getCol());
}

#endif
