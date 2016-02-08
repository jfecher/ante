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
#line 147 "src/parser.cpp" // lalr1.cc:479

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
  parser::syntax_error::syntax_error (const std::string& m)
    : std::runtime_error (m)
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
  {
    value = other.value;
  }


  template <typename Base>
  inline
  parser::basic_symbol<Base>::basic_symbol (typename Base::kind_type t, const semantic_type& v)
    : Base (t)
    , value (v)
  {}


  /// Constructor for valueless symbols.
  template <typename Base>
  inline
  parser::basic_symbol<Base>::basic_symbol (typename Base::kind_type t)
    : Base (t)
    , value ()
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
    : super_type (s)
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
        << ' ' << yytname_[yytype] << " (";
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
            yyla.type = yytranslate_ (yylex (&yyla.value));
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


      // Perform the reduction.
      YY_REDUCE_PRINT (yyn);
      try
        {
          switch (yyn)
            {
  case 3:
#line 106 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 610 "src/parser.cpp" // lalr1.cc:859
    break;

  case 4:
#line 107 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 616 "src/parser.cpp" // lalr1.cc:859
    break;

  case 7:
#line 114 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 622 "src/parser.cpp" // lalr1.cc:859
    break;

  case 8:
#line 115 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 628 "src/parser.cpp" // lalr1.cc:859
    break;

  case 9:
#line 116 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 634 "src/parser.cpp" // lalr1.cc:859
    break;

  case 10:
#line 117 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 640 "src/parser.cpp" // lalr1.cc:859
    break;

  case 11:
#line 118 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 646 "src/parser.cpp" // lalr1.cc:859
    break;

  case 12:
#line 119 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 652 "src/parser.cpp" // lalr1.cc:859
    break;

  case 13:
#line 120 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 658 "src/parser.cpp" // lalr1.cc:859
    break;

  case 14:
#line 121 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 664 "src/parser.cpp" // lalr1.cc:859
    break;

  case 15:
#line 122 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 670 "src/parser.cpp" // lalr1.cc:859
    break;

  case 16:
#line 123 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 676 "src/parser.cpp" // lalr1.cc:859
    break;

  case 17:
#line 124 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 682 "src/parser.cpp" // lalr1.cc:859
    break;

  case 18:
#line 127 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (Node*)lextxt;}
#line 688 "src/parser.cpp" // lalr1.cc:859
    break;

  case 19:
#line 130 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (Node*)lextxt;}
#line 694 "src/parser.cpp" // lalr1.cc:859
    break;

  case 20:
#line 133 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkIntLitNode(lextxt);}
#line 700 "src/parser.cpp" // lalr1.cc:859
    break;

  case 21:
#line 136 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFltLitNode(lextxt);}
#line 706 "src/parser.cpp" // lalr1.cc:859
    break;

  case 22:
#line 139 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkStrLitNode(lextxt);}
#line 712 "src/parser.cpp" // lalr1.cc:859
    break;

  case 23:
#line 142 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I8,  (char*)"");}
#line 718 "src/parser.cpp" // lalr1.cc:859
    break;

  case 24:
#line 143 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I16, (char*)"");}
#line 724 "src/parser.cpp" // lalr1.cc:859
    break;

  case 25:
#line 144 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I32, (char*)"");}
#line 730 "src/parser.cpp" // lalr1.cc:859
    break;

  case 26:
#line 145 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I64, (char*)"");}
#line 736 "src/parser.cpp" // lalr1.cc:859
    break;

  case 27:
#line 146 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U8,  (char*)"");}
#line 742 "src/parser.cpp" // lalr1.cc:859
    break;

  case 28:
#line 147 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U16, (char*)"");}
#line 748 "src/parser.cpp" // lalr1.cc:859
    break;

  case 29:
#line 148 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U32, (char*)"");}
#line 754 "src/parser.cpp" // lalr1.cc:859
    break;

  case 30:
#line 149 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U64, (char*)"");}
#line 760 "src/parser.cpp" // lalr1.cc:859
    break;

  case 31:
#line 150 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Isz, (char*)"");}
#line 766 "src/parser.cpp" // lalr1.cc:859
    break;

  case 32:
#line 151 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Usz, (char*)"");}
#line 772 "src/parser.cpp" // lalr1.cc:859
    break;

  case 33:
#line 152 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_F16, (char*)"");}
#line 778 "src/parser.cpp" // lalr1.cc:859
    break;

  case 34:
#line 153 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_F32, (char*)"");}
#line 784 "src/parser.cpp" // lalr1.cc:859
    break;

  case 35:
#line 154 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_F64, (char*)"");}
#line 790 "src/parser.cpp" // lalr1.cc:859
    break;

  case 36:
#line 155 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_C8,  (char*)"");}
#line 796 "src/parser.cpp" // lalr1.cc:859
    break;

  case 37:
#line 156 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_C32, (char*)"");}
#line 802 "src/parser.cpp" // lalr1.cc:859
    break;

  case 38:
#line 157 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Bool, (char*)"");}
#line 808 "src/parser.cpp" // lalr1.cc:859
    break;

  case 39:
#line 158 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Void, (char*)"");}
#line 814 "src/parser.cpp" // lalr1.cc:859
    break;

  case 40:
#line 159 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_UserType, (char*)(yystack_[0].value));}
#line 820 "src/parser.cpp" // lalr1.cc:859
    break;

  case 41:
#line 160 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Ident, (char*)(yystack_[0].value));}
#line 826 "src/parser.cpp" // lalr1.cc:859
    break;

  case 42:
#line 163 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('*', (char*)"", (yystack_[1].value));}
#line 832 "src/parser.cpp" // lalr1.cc:859
    break;

  case 43:
#line 164 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('[', (char*)"", (yystack_[3].value));}
#line 838 "src/parser.cpp" // lalr1.cc:859
    break;

  case 44:
#line 165 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('(', (char*)"", (yystack_[3].value));}
#line 844 "src/parser.cpp" // lalr1.cc:859
    break;

  case 45:
#line 166 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode('(', (char*)"", (yystack_[2].value));}
#line 850 "src/parser.cpp" // lalr1.cc:859
    break;

  case 46:
#line 167 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[2].value);}
#line 856 "src/parser.cpp" // lalr1.cc:859
    break;

  case 47:
#line 168 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 862 "src/parser.cpp" // lalr1.cc:859
    break;

  case 48:
#line 171 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 868 "src/parser.cpp" // lalr1.cc:859
    break;

  case 50:
#line 173 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 874 "src/parser.cpp" // lalr1.cc:859
    break;

  case 58:
#line 185 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[1].value), (yystack_[0].value));}
#line 880 "src/parser.cpp" // lalr1.cc:859
    break;

  case 59:
#line 186 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 886 "src/parser.cpp" // lalr1.cc:859
    break;

  case 60:
#line 189 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 892 "src/parser.cpp" // lalr1.cc:859
    break;

  case 61:
#line 190 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 898 "src/parser.cpp" // lalr1.cc:859
    break;

  case 62:
#line 193 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[2].value), (yystack_[3].value), (yystack_[0].value));}
#line 904 "src/parser.cpp" // lalr1.cc:859
    break;

  case 63:
#line 194 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[0].value), (yystack_[1].value), 0);}
#line 910 "src/parser.cpp" // lalr1.cc:859
    break;

  case 64:
#line 198 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarAssignNode((yystack_[2].value), (yystack_[0].value));}
#line 916 "src/parser.cpp" // lalr1.cc:859
    break;

  case 65:
#line 201 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 922 "src/parser.cpp" // lalr1.cc:859
    break;

  case 66:
#line 202 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 928 "src/parser.cpp" // lalr1.cc:859
    break;

  case 67:
#line 205 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 934 "src/parser.cpp" // lalr1.cc:859
    break;

  case 68:
#line 208 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[1].value), (yystack_[0].value));}
#line 940 "src/parser.cpp" // lalr1.cc:859
    break;

  case 69:
#line 209 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[2].value), (yystack_[0].value));}
#line 946 "src/parser.cpp" // lalr1.cc:859
    break;

  case 70:
#line 210 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[1].value), (yystack_[0].value));}
#line 952 "src/parser.cpp" // lalr1.cc:859
    break;

  case 71:
#line 211 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[2].value), (yystack_[0].value));}
#line 958 "src/parser.cpp" // lalr1.cc:859
    break;

  case 83:
#line 236 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 964 "src/parser.cpp" // lalr1.cc:859
    break;

  case 84:
#line 237 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 970 "src/parser.cpp" // lalr1.cc:859
    break;

  case 85:
#line 238 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 976 "src/parser.cpp" // lalr1.cc:859
    break;

  case 86:
#line 239 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 982 "src/parser.cpp" // lalr1.cc:859
    break;

  case 87:
#line 242 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 988 "src/parser.cpp" // lalr1.cc:859
    break;

  case 88:
#line 245 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[3].value), mkNamedValNode((char*)(yystack_[0].value), (yystack_[1].value)));}
#line 994 "src/parser.cpp" // lalr1.cc:859
    break;

  case 89:
#line 246 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkNamedValNode((char*)(yystack_[0].value), (yystack_[1].value)));}
#line 1000 "src/parser.cpp" // lalr1.cc:859
    break;

  case 90:
#line 249 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1006 "src/parser.cpp" // lalr1.cc:859
    break;

  case 91:
#line 250 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1012 "src/parser.cpp" // lalr1.cc:859
    break;

  case 92:
#line 253 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[3].value), (yystack_[4].value), (yystack_[1].value), (yystack_[0].value));}
#line 1018 "src/parser.cpp" // lalr1.cc:859
    break;

  case 93:
#line 254 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[6].value), (yystack_[7].value), (yystack_[1].value), (yystack_[0].value));}
#line 1024 "src/parser.cpp" // lalr1.cc:859
    break;

  case 94:
#line 257 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncCallNode((char*)(yystack_[3].value), (yystack_[1].value));}
#line 1030 "src/parser.cpp" // lalr1.cc:859
    break;

  case 95:
#line 260 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkRetNode((yystack_[0].value));}
#line 1036 "src/parser.cpp" // lalr1.cc:859
    break;

  case 96:
#line 263 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setElse((IfNode*)(yystack_[3].value), (IfNode*)mkIfNode((yystack_[1].value), (yystack_[0].value)));}
#line 1042 "src/parser.cpp" // lalr1.cc:859
    break;

  case 97:
#line 264 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkIfNode((yystack_[1].value), (yystack_[0].value)));}
#line 1048 "src/parser.cpp" // lalr1.cc:859
    break;

  case 98:
#line 267 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setElse((IfNode*)(yystack_[2].value), (IfNode*)mkIfNode(NULL, (yystack_[0].value)));}
#line 1054 "src/parser.cpp" // lalr1.cc:859
    break;

  case 99:
#line 268 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1060 "src/parser.cpp" // lalr1.cc:859
    break;

  case 100:
#line 269 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkIfNode(NULL, (yystack_[0].value)));}
#line 1066 "src/parser.cpp" // lalr1.cc:859
    break;

  case 101:
#line 270 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(NULL);}
#line 1072 "src/parser.cpp" // lalr1.cc:859
    break;

  case 102:
#line 273 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkIfNode((yystack_[2].value), (yystack_[1].value), (IfNode*)getRoot());}
#line 1078 "src/parser.cpp" // lalr1.cc:859
    break;

  case 103:
#line 276 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1084 "src/parser.cpp" // lalr1.cc:859
    break;

  case 104:
#line 279 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1090 "src/parser.cpp" // lalr1.cc:859
    break;

  case 105:
#line 282 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1096 "src/parser.cpp" // lalr1.cc:859
    break;

  case 106:
#line 285 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarNode((char*)(yystack_[3].value));}
#line 1102 "src/parser.cpp" // lalr1.cc:859
    break;

  case 107:
#line 286 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarNode((char*)(yystack_[0].value));}
#line 1108 "src/parser.cpp" // lalr1.cc:859
    break;

  case 108:
#line 289 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1114 "src/parser.cpp" // lalr1.cc:859
    break;

  case 109:
#line 290 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1120 "src/parser.cpp" // lalr1.cc:859
    break;

  case 110:
#line 291 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1126 "src/parser.cpp" // lalr1.cc:859
    break;

  case 111:
#line 292 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1132 "src/parser.cpp" // lalr1.cc:859
    break;

  case 112:
#line 293 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1138 "src/parser.cpp" // lalr1.cc:859
    break;

  case 113:
#line 294 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1144 "src/parser.cpp" // lalr1.cc:859
    break;

  case 114:
#line 295 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1150 "src/parser.cpp" // lalr1.cc:859
    break;

  case 115:
#line 296 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBoolLitNode(1);}
#line 1156 "src/parser.cpp" // lalr1.cc:859
    break;

  case 116:
#line 297 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBoolLitNode(0);}
#line 1162 "src/parser.cpp" // lalr1.cc:859
    break;

  case 117:
#line 300 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1168 "src/parser.cpp" // lalr1.cc:859
    break;

  case 118:
#line 301 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1174 "src/parser.cpp" // lalr1.cc:859
    break;

  case 119:
#line 304 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1180 "src/parser.cpp" // lalr1.cc:859
    break;

  case 120:
#line 306 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 1186 "src/parser.cpp" // lalr1.cc:859
    break;

  case 121:
#line 307 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 1192 "src/parser.cpp" // lalr1.cc:859
    break;

  case 122:
#line 311 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('+', (yystack_[2].value), (yystack_[0].value));}
#line 1198 "src/parser.cpp" // lalr1.cc:859
    break;

  case 123:
#line 312 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('-', (yystack_[2].value), (yystack_[0].value));}
#line 1204 "src/parser.cpp" // lalr1.cc:859
    break;

  case 124:
#line 313 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('*', (yystack_[2].value), (yystack_[0].value));}
#line 1210 "src/parser.cpp" // lalr1.cc:859
    break;

  case 125:
#line 314 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('/', (yystack_[2].value), (yystack_[0].value));}
#line 1216 "src/parser.cpp" // lalr1.cc:859
    break;

  case 126:
#line 315 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('%', (yystack_[2].value), (yystack_[0].value));}
#line 1222 "src/parser.cpp" // lalr1.cc:859
    break;

  case 127:
#line 316 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('<', (yystack_[2].value), (yystack_[0].value));}
#line 1228 "src/parser.cpp" // lalr1.cc:859
    break;

  case 128:
#line 317 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('>', (yystack_[2].value), (yystack_[0].value));}
#line 1234 "src/parser.cpp" // lalr1.cc:859
    break;

  case 129:
#line 318 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('^', (yystack_[2].value), (yystack_[0].value));}
#line 1240 "src/parser.cpp" // lalr1.cc:859
    break;

  case 130:
#line 319 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('.', (yystack_[2].value), (yystack_[0].value));}
#line 1246 "src/parser.cpp" // lalr1.cc:859
    break;

  case 131:
#line 320 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Eq, (yystack_[2].value), (yystack_[0].value));}
#line 1252 "src/parser.cpp" // lalr1.cc:859
    break;

  case 132:
#line 321 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_NotEq, (yystack_[2].value), (yystack_[0].value));}
#line 1258 "src/parser.cpp" // lalr1.cc:859
    break;

  case 133:
#line 322 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_GrtrEq, (yystack_[2].value), (yystack_[0].value));}
#line 1264 "src/parser.cpp" // lalr1.cc:859
    break;

  case 134:
#line 323 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_LesrEq, (yystack_[2].value), (yystack_[0].value));}
#line 1270 "src/parser.cpp" // lalr1.cc:859
    break;

  case 135:
#line 324 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Or, (yystack_[2].value), (yystack_[0].value));}
#line 1276 "src/parser.cpp" // lalr1.cc:859
    break;

  case 136:
#line 325 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_And, (yystack_[2].value), (yystack_[0].value));}
#line 1282 "src/parser.cpp" // lalr1.cc:859
    break;

  case 137:
#line 326 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1288 "src/parser.cpp" // lalr1.cc:859
    break;

  case 138:
#line 330 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1294 "src/parser.cpp" // lalr1.cc:859
    break;

  case 139:
#line 333 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[3].value), (yystack_[0].value));}
#line 1300 "src/parser.cpp" // lalr1.cc:859
    break;

  case 140:
#line 334 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 1306 "src/parser.cpp" // lalr1.cc:859
    break;

  case 141:
#line 335 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1312 "src/parser.cpp" // lalr1.cc:859
    break;

  case 142:
#line 338 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('+', (yystack_[3].value), (yystack_[0].value));}
#line 1318 "src/parser.cpp" // lalr1.cc:859
    break;

  case 143:
#line 339 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('-', (yystack_[3].value), (yystack_[0].value));}
#line 1324 "src/parser.cpp" // lalr1.cc:859
    break;

  case 144:
#line 340 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('*', (yystack_[3].value), (yystack_[0].value));}
#line 1330 "src/parser.cpp" // lalr1.cc:859
    break;

  case 145:
#line 341 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('/', (yystack_[3].value), (yystack_[0].value));}
#line 1336 "src/parser.cpp" // lalr1.cc:859
    break;

  case 146:
#line 342 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('%', (yystack_[3].value), (yystack_[0].value));}
#line 1342 "src/parser.cpp" // lalr1.cc:859
    break;

  case 147:
#line 343 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('<', (yystack_[3].value), (yystack_[0].value));}
#line 1348 "src/parser.cpp" // lalr1.cc:859
    break;

  case 148:
#line 344 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('>', (yystack_[3].value), (yystack_[0].value));}
#line 1354 "src/parser.cpp" // lalr1.cc:859
    break;

  case 149:
#line 345 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('^', (yystack_[3].value), (yystack_[0].value));}
#line 1360 "src/parser.cpp" // lalr1.cc:859
    break;

  case 150:
#line 346 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('.', (yystack_[3].value), (yystack_[0].value));}
#line 1366 "src/parser.cpp" // lalr1.cc:859
    break;

  case 151:
#line 347 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Eq, (yystack_[3].value), (yystack_[0].value));}
#line 1372 "src/parser.cpp" // lalr1.cc:859
    break;

  case 152:
#line 348 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_NotEq, (yystack_[3].value), (yystack_[0].value));}
#line 1378 "src/parser.cpp" // lalr1.cc:859
    break;

  case 153:
#line 349 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_GrtrEq, (yystack_[3].value), (yystack_[0].value));}
#line 1384 "src/parser.cpp" // lalr1.cc:859
    break;

  case 154:
#line 350 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_LesrEq, (yystack_[3].value), (yystack_[0].value));}
#line 1390 "src/parser.cpp" // lalr1.cc:859
    break;

  case 155:
#line 351 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Or, (yystack_[3].value), (yystack_[0].value));}
#line 1396 "src/parser.cpp" // lalr1.cc:859
    break;

  case 156:
#line 352 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_And, (yystack_[3].value), (yystack_[0].value));}
#line 1402 "src/parser.cpp" // lalr1.cc:859
    break;

  case 157:
#line 353 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1408 "src/parser.cpp" // lalr1.cc:859
    break;


#line 1412 "src/parser.cpp" // lalr1.cc:859
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
        error (yysyntax_error_ (yystack_[0].state, yyla));
      }


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

          yy_destroy_ ("Error: popping", yystack_[0]);
          yypop_ ();
          YY_STACK_PRINT ();
        }


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
    error (yyexc.what());
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


  const short int parser::yypact_ninf_ = -131;

  const signed char parser::yytable_ninf_ = -108;

  const short int
  parser::yypact_[] =
  {
     -47,  -131,    37,   336,  -131,  -131,  -131,  -131,  -131,  -131,
    -131,  -131,  -131,  -131,  -131,  -131,  -131,  -131,  -131,  -131,
    -131,  -131,  -131,  -131,    17,    17,   511,    17,   -15,    69,
       8,  -131,  -131,  -131,  -131,  -131,  -131,  -131,   530,   -47,
    -131,    25,  -131,  -131,   -38,   -44,  -131,   410,    67,  -131,
    -131,  -131,  -131,  -131,  -131,  -131,  -131,  -131,  -131,  -131,
      -5,  -131,  -131,  -131,  -131,  -131,    17,    17,    28,  -131,
    -131,  -131,  -131,  -131,  -131,  -131,    13,   547,   -15,  -131,
     511,    67,    38,   -15,   336,    70,    48,    69,    72,  -131,
     -48,   336,    17,    17,  -131,   305,    17,   530,   530,    69,
       8,   -44,  -131,    27,    17,  -131,    54,    68,   560,    52,
      17,    17,    17,    17,    17,    17,    17,    17,    17,    17,
      17,    17,    17,    17,    17,    17,     3,    83,    17,  -131,
     -34,    17,   437,    69,   100,  -131,    86,    55,  -131,  -131,
    -131,    87,  -131,    90,  -131,   -39,    91,   -38,   -38,    48,
      72,  -131,    17,    17,   530,  -131,  -131,  -131,   -47,   -47,
     -47,   -47,   -47,   -47,   -47,   -47,   -47,   -47,   -47,   -47,
     -47,   -47,   -47,   -47,  -131,   547,    -6,    -6,    -6,    -6,
     570,   634,    -6,    -6,    66,    66,    78,    78,    78,    78,
      96,    17,   -15,   116,  -131,   -15,  -131,   336,  -131,    18,
       4,  -131,    85,  -131,  -131,    82,  -131,    17,    69,  -131,
    -131,  -131,  -131,  -131,   100,  -131,  -131,    93,  -131,    18,
      99,   -15,    17,    17,    17,    17,    17,    17,    17,    17,
      17,    17,    17,    17,    17,    17,    17,    17,   -15,  -131,
      17,   -15,  -131,  -131,   437,  -131,    69,  -131,  -131,    92,
    -131,    94,  -131,   530,  -131,   547,    53,    53,    53,    53,
     580,   643,    53,    53,    71,    71,    84,    84,    84,    84,
     102,  -131,   -15,  -131,  -131,  -131,    17,   530,    18,  -131,
    -131,   -15,  -131,  -131
  };

  const unsigned char
  parser::yydefact_[] =
  {
       6,     5,     0,     0,     1,    18,    19,    23,    24,    25,
      26,    27,    28,    29,    30,    31,    32,    33,    34,    35,
      36,    37,    38,    39,     0,     0,     0,     0,     0,     0,
       0,    51,    52,    53,    54,    55,    56,    57,     0,     6,
       4,    41,    40,    47,    50,    61,    59,     0,     0,     7,
       8,    11,    17,     9,    10,    12,    16,    13,    14,    15,
       0,   115,   116,    20,    21,    22,     0,     0,   107,   112,
     113,   114,   108,   111,   137,    95,   119,   121,     0,    41,
       0,     0,     0,     0,     0,     0,     0,     0,     0,    86,
       0,     2,   118,     0,    42,     0,   118,     0,     0,     0,
       0,    60,    58,    63,     0,   157,     0,   138,   140,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,   101,    63,     0,   103,
       6,     0,     0,     0,     0,    70,    81,     0,    84,    46,
       3,     0,   117,     0,    45,     0,     0,    48,    49,     0,
       0,    85,   118,     0,    91,    64,   110,   141,     6,     6,
       6,     6,     6,     6,     6,     6,     6,     6,     6,     6,
       6,     6,     6,     6,   109,   120,   131,   132,   133,   134,
     135,   136,   127,   128,   122,   123,   124,   125,   126,   129,
     130,     0,     0,    99,   102,     0,    87,     0,   104,    73,
       0,    76,     0,    74,    66,     0,    71,     0,     0,    82,
      94,   106,    44,    43,     0,    68,    83,     0,    62,     0,
      90,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,   100,
       0,     0,   105,    72,     0,    77,     0,    67,    80,    78,
      69,     0,    89,     0,    92,   139,   151,   152,   153,   154,
     155,   156,   147,   148,   142,   143,   144,   145,   146,   149,
     150,    97,     0,    98,    75,    65,     0,    91,     0,    96,
      79,     0,    88,    93
  };

  const short int
  parser::yypgoto_[] =
  {
    -131,  -131,    97,   118,   -81,    -3,   -13,  -131,  -131,  -131,
    -131,    63,   -33,   -46,   -19,   148,   149,  -131,  -131,    29,
    -131,   -64,  -131,  -123,  -131,   -75,  -130,   -59,  -131,   -94,
    -131,     5,  -131,  -131,  -131,  -131,  -131,  -131,  -131,     6,
      34,   -67,   -21,  -131,   182,  -131,  -131,   135
  };

  const short int
  parser::yydefgoto_[] =
  {
      -1,     2,    39,     3,    40,    68,    42,    69,    70,    71,
      43,    44,    45,    46,    47,    48,    49,    50,   205,   134,
      51,   201,   202,   135,   137,    89,    52,    85,   220,   221,
      53,    72,    55,   193,   194,    56,    57,    58,    59,    73,
      74,   141,   142,    76,    77,   106,   107,   108
  };

  const short int
  parser::yytable_[] =
  {
      41,   102,   203,    75,    78,    90,    83,    80,    54,    60,
     140,   206,     6,   138,   101,     1,    86,    88,    97,   126,
       5,     5,    97,    79,   129,   151,   215,    97,     1,   146,
     196,   139,    98,    94,   102,    79,    98,     4,    95,    96,
     212,    98,   191,   192,    79,   103,   109,   101,    84,    61,
      62,    63,    64,    65,   100,    31,    32,    33,    34,    35,
      36,    37,   145,   119,   120,   121,   122,   123,   124,   125,
       5,    87,   143,     6,   136,   216,   104,    79,   127,   110,
      66,    41,   128,   155,    97,   217,   149,   150,    41,    54,
      60,   250,    79,    67,    79,    79,    54,    60,    98,   199,
     105,    92,    93,   152,    92,    93,  -107,   195,   153,   154,
     198,   132,   131,   200,   203,   133,   140,   208,   156,   209,
     204,   219,   167,   168,   169,   170,   171,   172,   173,    79,
     157,   174,   218,   239,   158,    87,   242,   121,   122,   123,
     124,   125,   169,   170,   171,   172,   173,   244,   246,   245,
     247,    79,   124,   125,   102,   240,   241,    91,   172,   173,
     147,   148,   254,   132,   153,   253,   210,   207,   211,   213,
     238,   125,   251,   276,    81,    82,   277,   173,   214,   271,
     274,   130,   273,   281,     0,     0,   248,     0,     0,     0,
       0,     0,     0,     0,    41,   249,   243,     0,     0,     0,
       0,     0,    54,    60,     0,     0,     0,     0,     0,     0,
       0,   199,     0,   279,     0,     0,   252,     0,     0,   272,
     278,     0,   283,     0,     0,   200,     0,     0,     0,     0,
       0,     0,     0,   275,     0,     0,     0,     0,     0,     0,
       0,    79,     0,     0,   219,     0,     0,     0,   197,     0,
      79,     0,     0,     0,     0,   280,     0,   105,   105,   105,
     105,   105,   105,   105,   105,   105,   105,   105,   105,   105,
     105,   105,     0,     0,    79,   282,   222,   223,   224,   225,
     226,   227,   228,   229,   230,   231,   232,   233,   234,   235,
     236,   237,   175,   176,   177,   178,   179,   180,   181,   182,
     183,   184,   185,   186,   187,   188,   189,   190,     5,     6,
       7,     8,     9,    10,    11,    12,    13,    14,    15,    16,
      17,    18,    19,    20,    21,    22,    23,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     5,
       6,     7,     8,     9,    10,    11,    12,    13,    14,    15,
      16,    17,    18,    19,    20,    21,    22,    23,   256,   257,
     258,   259,   260,   261,   262,   263,   264,   265,   266,   267,
     268,   269,   270,    24,    25,     0,     0,    26,    27,    28,
       0,    38,     0,     0,   144,    29,    30,    31,    32,    33,
      34,    35,    36,    37,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,   255,     0,     0,     0,     0,     0,
       0,     0,    38,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
      22,    23,     0,     0,     0,     0,     0,     0,     0,     0,
       5,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    22,    23,    99,
     100,    31,    32,    33,    34,    35,    36,    37,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,    38,    30,    31,    32,
      33,    34,    35,    36,    37,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,    38,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    22,    23,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
      22,    23,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,    31,    32,    33,    34,    35,    36,    37,   111,
     112,     0,     0,     0,     0,   113,   114,   115,   116,     0,
       0,     0,   159,   160,     0,     0,     0,    38,   161,   162,
     163,   164,   111,   112,     0,     0,     0,     0,   113,   114,
       0,   116,   159,   160,     0,     0,    38,     0,   161,   162,
       0,   164,     0,     0,   117,   118,   119,   120,   121,   122,
     123,   124,   125,     0,     0,     0,     0,   165,   166,   167,
     168,   169,   170,   171,   172,   173,     0,   117,   118,   119,
     120,   121,   122,   123,   124,   125,     0,   165,   166,   167,
     168,   169,   170,   171,   172,   173,   111,   112,     0,     0,
       0,     0,   113,   114,     0,   159,   160,     0,     0,     0,
       0,   161,   162,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,   117,   118,   119,   120,   121,   122,   123,   124,   125,
     165,   166,   167,   168,   169,   170,   171,   172,   173
  };

  const short int
  parser::yycheck_[] =
  {
       3,    47,   132,    24,    25,    38,    27,    26,     3,     3,
      91,   134,     4,    88,    47,    62,    29,    30,    66,    78,
       3,     3,    66,    26,    83,   100,   149,    66,    62,    96,
      64,    79,    80,    71,    80,    38,    80,     0,    76,    77,
      79,    80,    39,    40,    47,    48,    67,    80,    63,    32,
      33,    34,    35,    36,    50,    51,    52,    53,    54,    55,
      56,    57,    95,    69,    70,    71,    72,    73,    74,    75,
       3,    63,    93,     4,    87,   150,    81,    80,    81,    66,
      63,    84,    44,   104,    66,   152,    99,   100,    91,    84,
      84,   214,    95,    76,    97,    98,    91,    91,    80,   132,
      66,    76,    77,    76,    76,    77,    81,   128,    81,    82,
     131,    63,    42,   132,   244,    67,   197,    62,    64,    64,
     133,   154,    69,    70,    71,    72,    73,    74,    75,   132,
      62,    79,   153,   192,    66,    63,   195,    71,    72,    73,
      74,    75,    71,    72,    73,    74,    75,    62,    66,    64,
      68,   154,    74,    75,   200,    39,    40,    39,    74,    75,
      97,    98,   221,    63,    81,    66,    79,    81,    78,    78,
     191,    75,    79,    81,    26,    26,    82,    75,   149,   238,
     244,    84,   241,   277,    -1,    -1,   207,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,   197,   208,   199,    -1,    -1,    -1,
      -1,    -1,   197,   197,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,   244,    -1,   272,    -1,    -1,   219,    -1,    -1,   240,
     253,    -1,   281,    -1,    -1,   244,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,   246,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,   244,    -1,    -1,   277,    -1,    -1,    -1,   130,    -1,
     253,    -1,    -1,    -1,    -1,   276,    -1,   223,   224,   225,
     226,   227,   228,   229,   230,   231,   232,   233,   234,   235,
     236,   237,    -1,    -1,   277,   278,   158,   159,   160,   161,
     162,   163,   164,   165,   166,   167,   168,   169,   170,   171,
     172,   173,   110,   111,   112,   113,   114,   115,   116,   117,
     118,   119,   120,   121,   122,   123,   124,   125,     3,     4,
       5,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,     3,
       4,     5,     6,     7,     8,     9,    10,    11,    12,    13,
      14,    15,    16,    17,    18,    19,    20,    21,   223,   224,
     225,   226,   227,   228,   229,   230,   231,   232,   233,   234,
     235,   236,   237,    37,    38,    -1,    -1,    41,    42,    43,
      -1,    76,    -1,    -1,    79,    49,    50,    51,    52,    53,
      54,    55,    56,    57,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,   222,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    76,     3,     4,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
       3,     4,     5,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    21,    49,
      50,    51,    52,    53,    54,    55,    56,    57,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    76,    50,    51,    52,
      53,    54,    55,    56,    57,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    76,     3,     4,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,     3,     4,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    51,    52,    53,    54,    55,    56,    57,    22,
      23,    -1,    -1,    -1,    -1,    28,    29,    30,    31,    -1,
      -1,    -1,    22,    23,    -1,    -1,    -1,    76,    28,    29,
      30,    31,    22,    23,    -1,    -1,    -1,    -1,    28,    29,
      -1,    31,    22,    23,    -1,    -1,    76,    -1,    28,    29,
      -1,    31,    -1,    -1,    67,    68,    69,    70,    71,    72,
      73,    74,    75,    -1,    -1,    -1,    -1,    67,    68,    69,
      70,    71,    72,    73,    74,    75,    -1,    67,    68,    69,
      70,    71,    72,    73,    74,    75,    -1,    67,    68,    69,
      70,    71,    72,    73,    74,    75,    22,    23,    -1,    -1,
      -1,    -1,    28,    29,    -1,    22,    23,    -1,    -1,    -1,
      -1,    28,    29,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    67,    68,    69,    70,    71,    72,    73,    74,    75,
      67,    68,    69,    70,    71,    72,    73,    74,    75
  };

  const unsigned char
  parser::yystos_[] =
  {
       0,    62,    84,    86,     0,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    37,    38,    41,    42,    43,    49,
      50,    51,    52,    53,    54,    55,    56,    57,    76,    85,
      87,    88,    89,    93,    94,    95,    96,    97,    98,    99,
     100,   103,   109,   113,   114,   115,   118,   119,   120,   121,
     122,    32,    33,    34,    35,    36,    63,    76,    88,    90,
      91,    92,   114,   122,   123,   125,   126,   127,   125,    88,
      97,    98,    99,   125,    63,   110,    89,    63,    89,   108,
      95,    86,    76,    77,    71,    76,    77,    66,    80,    49,
      50,    95,    96,    88,    81,   123,   128,   129,   130,   125,
      66,    22,    23,    28,    29,    30,    31,    67,    68,    69,
      70,    71,    72,    73,    74,    75,   110,    88,    44,   110,
      85,    42,    63,    67,   102,   106,    89,   107,   108,    79,
      87,   124,   125,   125,    79,    95,   124,    94,    94,    89,
      89,   108,    76,    81,    82,   125,    64,    62,    66,    22,
      23,    28,    29,    30,    31,    67,    68,    69,    70,    71,
      72,    73,    74,    75,    79,   127,   127,   127,   127,   127,
     127,   127,   127,   127,   127,   127,   127,   127,   127,   127,
     127,    39,    40,   116,   117,   125,    64,    86,   125,    95,
      97,   104,   105,   109,    89,   101,   106,    81,    62,    64,
      79,    78,    79,    78,   102,   106,   108,   124,   125,    95,
     111,   112,    86,    86,    86,    86,    86,    86,    86,    86,
      86,    86,    86,    86,    86,    86,    86,    86,   125,   110,
      39,    40,   110,    88,    62,    64,    66,    68,   125,    89,
     106,    79,    88,    66,   110,   127,   130,   130,   130,   130,
     130,   130,   130,   130,   130,   130,   130,   130,   130,   130,
     130,   110,   125,   110,   104,    89,    81,    82,    95,   110,
     125,   112,    88,   110
  };

  const unsigned char
  parser::yyr1_[] =
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
     123,   123,   123,   123,   123,   123,   123,   124,   124,   125,
     126,   126,   127,   127,   127,   127,   127,   127,   127,   127,
     127,   127,   127,   127,   127,   127,   127,   127,   128,   129,
     129,   129,   130,   130,   130,   130,   130,   130,   130,   130,
     130,   130,   130,   130,   130,   130,   130,   130
  };

  const unsigned char
  parser::yyr2_[] =
  {
       0,     2,     3,     3,     1,     1,     0,     1,     1,     1,
       1,     1,     1,     1,     1,     1,     1,     1,     1,     1,
       1,     1,     1,     1,     1,     1,     1,     1,     1,     1,
       1,     1,     1,     1,     1,     1,     1,     1,     1,     1,
       1,     1,     2,     4,     4,     3,     3,     1,     3,     3,
       1,     1,     1,     1,     1,     1,     1,     1,     2,     1,
       2,     1,     4,     2,     3,     3,     1,     3,     4,     5,
       3,     4,     2,     1,     1,     3,     1,     3,     3,     5,
       3,     1,     3,     4,     3,     3,     2,     3,     4,     2,
       1,     0,     5,     8,     4,     2,     4,     3,     3,     1,
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
  "var", "val", "maybe_expr", "expr", "expr_list", "expr_p", "nl_expr",
  "nl_expr_list", "nl_expr_p", YY_NULLPTR
  };

#if YYDEBUG
  const unsigned short int
  parser::yyrline_[] =
  {
       0,   103,   103,   106,   107,   110,   111,   114,   115,   116,
     117,   118,   119,   120,   121,   122,   123,   124,   127,   130,
     133,   136,   139,   142,   143,   144,   145,   146,   147,   148,
     149,   150,   151,   152,   153,   154,   155,   156,   157,   158,
     159,   160,   163,   164,   165,   166,   167,   168,   171,   172,
     173,   176,   177,   178,   179,   180,   181,   182,   185,   186,
     189,   190,   193,   194,   198,   201,   202,   205,   208,   209,
     210,   211,   214,   215,   216,   219,   220,   223,   227,   228,
     229,   230,   233,   236,   237,   238,   239,   242,   245,   246,
     249,   250,   253,   254,   257,   260,   263,   264,   267,   268,
     269,   270,   273,   276,   279,   282,   285,   286,   289,   290,
     291,   292,   293,   294,   295,   296,   297,   300,   301,   304,
     306,   307,   311,   312,   313,   314,   315,   316,   317,   318,
     319,   320,   321,   322,   323,   324,   325,   326,   330,   333,
     334,   335,   338,   339,   340,   341,   342,   343,   344,   345,
     346,   347,   348,   349,   350,   351,   352,   353
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
    const unsigned int user_token_number_max_ = 320;
    const token_number_type undef_token_ = 2;

    if (static_cast<int>(t) <= yyeof_)
      return yyeof_;
    else if (static_cast<unsigned int> (t) <= user_token_number_max_)
      return translate_table[t];
    else
      return undef_token_;
  }


} // yy
#line 2119 "src/parser.cpp" // lalr1.cc:1167
#line 356 "src/syntax.y" // lalr1.cc:1168


void yy::parser::error(const string& msg){
    ante::error(msg.c_str(), yylexer->fileName, yylexer->getRow(), yylexer->getCol());
}

#endif
