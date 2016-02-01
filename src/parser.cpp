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
#include "yyparser.h"

extern int yylex(...);

void yyerror(const char *msg);

#define YYERROR_VERBOSE


#line 58 "src/parser.cpp" // lalr1.cc:404

# ifndef YY_NULLPTR
#  if defined __cplusplus && 201103L <= __cplusplus
#   define YY_NULLPTR nullptr
#  else
#   define YY_NULLPTR 0
#  endif
# endif

#include "yyparser.h"

// User implementation prologue.

#line 72 "src/parser.cpp" // lalr1.cc:412


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
#line 139 "src/parser.cpp" // lalr1.cc:479

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
#line 97 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 564 "src/parser.cpp" // lalr1.cc:859
    break;

  case 4:
#line 98 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 570 "src/parser.cpp" // lalr1.cc:859
    break;

  case 7:
#line 105 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 576 "src/parser.cpp" // lalr1.cc:859
    break;

  case 8:
#line 106 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 582 "src/parser.cpp" // lalr1.cc:859
    break;

  case 9:
#line 107 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 588 "src/parser.cpp" // lalr1.cc:859
    break;

  case 10:
#line 108 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 594 "src/parser.cpp" // lalr1.cc:859
    break;

  case 11:
#line 109 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 600 "src/parser.cpp" // lalr1.cc:859
    break;

  case 12:
#line 110 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 606 "src/parser.cpp" // lalr1.cc:859
    break;

  case 13:
#line 111 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 612 "src/parser.cpp" // lalr1.cc:859
    break;

  case 14:
#line 112 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 618 "src/parser.cpp" // lalr1.cc:859
    break;

  case 15:
#line 113 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 624 "src/parser.cpp" // lalr1.cc:859
    break;

  case 16:
#line 114 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 630 "src/parser.cpp" // lalr1.cc:859
    break;

  case 17:
#line 115 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 636 "src/parser.cpp" // lalr1.cc:859
    break;

  case 18:
#line 118 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (Node*)lextxt;}
#line 642 "src/parser.cpp" // lalr1.cc:859
    break;

  case 19:
#line 121 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (Node*)lextxt;}
#line 648 "src/parser.cpp" // lalr1.cc:859
    break;

  case 20:
#line 124 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkIntLitNode(lextxt);}
#line 654 "src/parser.cpp" // lalr1.cc:859
    break;

  case 21:
#line 127 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFltLitNode(lextxt);}
#line 660 "src/parser.cpp" // lalr1.cc:859
    break;

  case 22:
#line 130 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkStrLitNode(lextxt);}
#line 666 "src/parser.cpp" // lalr1.cc:859
    break;

  case 23:
#line 133 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I8,  (char*)"");}
#line 672 "src/parser.cpp" // lalr1.cc:859
    break;

  case 24:
#line 134 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I16, (char*)"");}
#line 678 "src/parser.cpp" // lalr1.cc:859
    break;

  case 25:
#line 135 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I32, (char*)"");}
#line 684 "src/parser.cpp" // lalr1.cc:859
    break;

  case 26:
#line 136 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_I64, (char*)"");}
#line 690 "src/parser.cpp" // lalr1.cc:859
    break;

  case 27:
#line 137 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U8,  (char*)"");}
#line 696 "src/parser.cpp" // lalr1.cc:859
    break;

  case 28:
#line 138 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U16, (char*)"");}
#line 702 "src/parser.cpp" // lalr1.cc:859
    break;

  case 29:
#line 139 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U32, (char*)"");}
#line 708 "src/parser.cpp" // lalr1.cc:859
    break;

  case 30:
#line 140 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_U64, (char*)"");}
#line 714 "src/parser.cpp" // lalr1.cc:859
    break;

  case 31:
#line 141 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Isz, (char*)"");}
#line 720 "src/parser.cpp" // lalr1.cc:859
    break;

  case 32:
#line 142 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Usz, (char*)"");}
#line 726 "src/parser.cpp" // lalr1.cc:859
    break;

  case 33:
#line 143 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_F16, (char*)"");}
#line 732 "src/parser.cpp" // lalr1.cc:859
    break;

  case 34:
#line 144 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_F32, (char*)"");}
#line 738 "src/parser.cpp" // lalr1.cc:859
    break;

  case 35:
#line 145 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_F64, (char*)"");}
#line 744 "src/parser.cpp" // lalr1.cc:859
    break;

  case 36:
#line 146 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_C8,  (char*)"");}
#line 750 "src/parser.cpp" // lalr1.cc:859
    break;

  case 37:
#line 147 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_C32, (char*)"");}
#line 756 "src/parser.cpp" // lalr1.cc:859
    break;

  case 38:
#line 148 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Bool, (char*)"");}
#line 762 "src/parser.cpp" // lalr1.cc:859
    break;

  case 39:
#line 149 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Void, (char*)"");}
#line 768 "src/parser.cpp" // lalr1.cc:859
    break;

  case 40:
#line 150 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_UserType, (char*)(yystack_[0].value));}
#line 774 "src/parser.cpp" // lalr1.cc:859
    break;

  case 41:
#line 151 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(Tok_Ident, (char*)(yystack_[0].value));}
#line 780 "src/parser.cpp" // lalr1.cc:859
    break;

  case 47:
#line 159 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 786 "src/parser.cpp" // lalr1.cc:859
    break;

  case 50:
#line 164 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 792 "src/parser.cpp" // lalr1.cc:859
    break;

  case 60:
#line 180 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 798 "src/parser.cpp" // lalr1.cc:859
    break;

  case 61:
#line 181 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 804 "src/parser.cpp" // lalr1.cc:859
    break;

  case 62:
#line 184 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[2].value), (yystack_[3].value), (yystack_[0].value));}
#line 810 "src/parser.cpp" // lalr1.cc:859
    break;

  case 63:
#line 185 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[0].value), (yystack_[1].value), 0);}
#line 816 "src/parser.cpp" // lalr1.cc:859
    break;

  case 64:
#line 189 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarAssignNode((yystack_[2].value), (yystack_[0].value));}
#line 822 "src/parser.cpp" // lalr1.cc:859
    break;

  case 65:
#line 192 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 828 "src/parser.cpp" // lalr1.cc:859
    break;

  case 66:
#line 193 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 834 "src/parser.cpp" // lalr1.cc:859
    break;

  case 67:
#line 196 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 840 "src/parser.cpp" // lalr1.cc:859
    break;

  case 68:
#line 199 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[1].value), (yystack_[0].value));}
#line 846 "src/parser.cpp" // lalr1.cc:859
    break;

  case 69:
#line 200 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[2].value), (yystack_[0].value));}
#line 852 "src/parser.cpp" // lalr1.cc:859
    break;

  case 70:
#line 201 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[1].value), (yystack_[0].value));}
#line 858 "src/parser.cpp" // lalr1.cc:859
    break;

  case 71:
#line 202 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[2].value), (yystack_[0].value));}
#line 864 "src/parser.cpp" // lalr1.cc:859
    break;

  case 83:
#line 227 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 870 "src/parser.cpp" // lalr1.cc:859
    break;

  case 84:
#line 228 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 876 "src/parser.cpp" // lalr1.cc:859
    break;

  case 85:
#line 229 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 882 "src/parser.cpp" // lalr1.cc:859
    break;

  case 86:
#line 230 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 888 "src/parser.cpp" // lalr1.cc:859
    break;

  case 87:
#line 233 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 894 "src/parser.cpp" // lalr1.cc:859
    break;

  case 88:
#line 236 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[3].value), mkNamedValNode((char*)(yystack_[0].value), (yystack_[1].value)));}
#line 900 "src/parser.cpp" // lalr1.cc:859
    break;

  case 89:
#line 237 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkNamedValNode((char*)(yystack_[0].value), (yystack_[1].value)));}
#line 906 "src/parser.cpp" // lalr1.cc:859
    break;

  case 90:
#line 240 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 912 "src/parser.cpp" // lalr1.cc:859
    break;

  case 91:
#line 241 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 918 "src/parser.cpp" // lalr1.cc:859
    break;

  case 92:
#line 244 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[3].value), (yystack_[4].value), (yystack_[1].value), (yystack_[0].value));}
#line 924 "src/parser.cpp" // lalr1.cc:859
    break;

  case 93:
#line 245 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[6].value), (yystack_[7].value), (yystack_[1].value), (yystack_[0].value));}
#line 930 "src/parser.cpp" // lalr1.cc:859
    break;

  case 94:
#line 248 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncCallNode((char*)(yystack_[3].value), (yystack_[1].value));}
#line 936 "src/parser.cpp" // lalr1.cc:859
    break;

  case 95:
#line 251 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkRetNode((yystack_[0].value));}
#line 942 "src/parser.cpp" // lalr1.cc:859
    break;

  case 96:
#line 254 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setElse((IfNode*)(yystack_[3].value), (IfNode*)mkIfNode((yystack_[1].value), (yystack_[0].value)));}
#line 948 "src/parser.cpp" // lalr1.cc:859
    break;

  case 97:
#line 255 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkIfNode((yystack_[1].value), (yystack_[0].value)));}
#line 954 "src/parser.cpp" // lalr1.cc:859
    break;

  case 98:
#line 258 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setElse((IfNode*)(yystack_[2].value), (IfNode*)mkIfNode(NULL, (yystack_[0].value)));}
#line 960 "src/parser.cpp" // lalr1.cc:859
    break;

  case 99:
#line 259 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 966 "src/parser.cpp" // lalr1.cc:859
    break;

  case 100:
#line 260 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkIfNode(NULL, (yystack_[0].value)));}
#line 972 "src/parser.cpp" // lalr1.cc:859
    break;

  case 101:
#line 261 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(NULL);}
#line 978 "src/parser.cpp" // lalr1.cc:859
    break;

  case 102:
#line 264 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkIfNode((yystack_[2].value), (yystack_[1].value), (IfNode*)getRoot());}
#line 984 "src/parser.cpp" // lalr1.cc:859
    break;

  case 103:
#line 267 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 990 "src/parser.cpp" // lalr1.cc:859
    break;

  case 104:
#line 270 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 996 "src/parser.cpp" // lalr1.cc:859
    break;

  case 105:
#line 273 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1002 "src/parser.cpp" // lalr1.cc:859
    break;

  case 106:
#line 276 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarNode((char*)(yystack_[3].value));}
#line 1008 "src/parser.cpp" // lalr1.cc:859
    break;

  case 107:
#line 277 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarNode((char*)(yystack_[0].value));}
#line 1014 "src/parser.cpp" // lalr1.cc:859
    break;

  case 108:
#line 280 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1020 "src/parser.cpp" // lalr1.cc:859
    break;

  case 109:
#line 281 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1026 "src/parser.cpp" // lalr1.cc:859
    break;

  case 110:
#line 282 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1032 "src/parser.cpp" // lalr1.cc:859
    break;

  case 111:
#line 283 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1038 "src/parser.cpp" // lalr1.cc:859
    break;

  case 112:
#line 284 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1044 "src/parser.cpp" // lalr1.cc:859
    break;

  case 113:
#line 285 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1050 "src/parser.cpp" // lalr1.cc:859
    break;

  case 114:
#line 286 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBoolLitNode(1);}
#line 1056 "src/parser.cpp" // lalr1.cc:859
    break;

  case 115:
#line 287 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBoolLitNode(0);}
#line 1062 "src/parser.cpp" // lalr1.cc:859
    break;

  case 116:
#line 290 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1068 "src/parser.cpp" // lalr1.cc:859
    break;

  case 117:
#line 291 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1074 "src/parser.cpp" // lalr1.cc:859
    break;

  case 118:
#line 294 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1080 "src/parser.cpp" // lalr1.cc:859
    break;

  case 119:
#line 296 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 1086 "src/parser.cpp" // lalr1.cc:859
    break;

  case 120:
#line 297 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 1092 "src/parser.cpp" // lalr1.cc:859
    break;

  case 121:
#line 300 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('+', (yystack_[2].value), (yystack_[0].value));}
#line 1098 "src/parser.cpp" // lalr1.cc:859
    break;

  case 122:
#line 301 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('-', (yystack_[2].value), (yystack_[0].value));}
#line 1104 "src/parser.cpp" // lalr1.cc:859
    break;

  case 123:
#line 302 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('*', (yystack_[2].value), (yystack_[0].value));}
#line 1110 "src/parser.cpp" // lalr1.cc:859
    break;

  case 124:
#line 303 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('/', (yystack_[2].value), (yystack_[0].value));}
#line 1116 "src/parser.cpp" // lalr1.cc:859
    break;

  case 125:
#line 304 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('%', (yystack_[2].value), (yystack_[0].value));}
#line 1122 "src/parser.cpp" // lalr1.cc:859
    break;

  case 126:
#line 305 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('<', (yystack_[2].value), (yystack_[0].value));}
#line 1128 "src/parser.cpp" // lalr1.cc:859
    break;

  case 127:
#line 306 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('>', (yystack_[2].value), (yystack_[0].value));}
#line 1134 "src/parser.cpp" // lalr1.cc:859
    break;

  case 128:
#line 307 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('^', (yystack_[2].value), (yystack_[0].value));}
#line 1140 "src/parser.cpp" // lalr1.cc:859
    break;

  case 129:
#line 308 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('.', (yystack_[2].value), (yystack_[0].value));}
#line 1146 "src/parser.cpp" // lalr1.cc:859
    break;

  case 130:
#line 309 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Eq, (yystack_[2].value), (yystack_[0].value));}
#line 1152 "src/parser.cpp" // lalr1.cc:859
    break;

  case 131:
#line 310 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_NotEq, (yystack_[2].value), (yystack_[0].value));}
#line 1158 "src/parser.cpp" // lalr1.cc:859
    break;

  case 132:
#line 311 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_GrtrEq, (yystack_[2].value), (yystack_[0].value));}
#line 1164 "src/parser.cpp" // lalr1.cc:859
    break;

  case 133:
#line 312 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_LesrEq, (yystack_[2].value), (yystack_[0].value));}
#line 1170 "src/parser.cpp" // lalr1.cc:859
    break;

  case 134:
#line 313 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Or, (yystack_[2].value), (yystack_[0].value));}
#line 1176 "src/parser.cpp" // lalr1.cc:859
    break;

  case 135:
#line 314 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_And, (yystack_[2].value), (yystack_[0].value));}
#line 1182 "src/parser.cpp" // lalr1.cc:859
    break;

  case 136:
#line 315 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1188 "src/parser.cpp" // lalr1.cc:859
    break;


#line 1192 "src/parser.cpp" // lalr1.cc:859
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
  parser::yysyntax_error_ (state_type, const symbol_type&) const
  {
    return YY_("syntax error");
  }


  const signed char parser::yypact_ninf_ = -118;

  const signed char parser::yytable_ninf_ = -108;

  const short int
  parser::yypact_[] =
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

  const unsigned char
  parser::yydefact_[] =
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

  const short int
  parser::yypgoto_[] =
  {
    -118,  -118,    82,   -33,   -76,    -3,   -13,  -118,  -118,  -118,
    -118,    40,   -37,   -45,   -23,   134,   135,  -118,  -118,    19,
    -118,   -39,  -118,  -117,  -118,   -67,  -108,   -64,  -118,   -53,
    -118,     5,  -118,  -118,  -118,  -118,  -118,  -118,  -118,     6,
    -118,   -80,   -20,  -118,   202
  };

  const short int
  parser::yydefgoto_[] =
  {
      -1,     2,    39,     3,    40,    67,    42,    68,    69,    70,
      43,    44,    45,    46,    47,    48,    49,    50,   182,   129,
      51,   178,   179,   130,   132,    88,    52,    84,   197,   198,
      53,    71,    55,   170,   171,    56,    57,    58,    59,    72,
      73,   136,   137,    75,    76
  };

  const short int
  parser::yytable_[] =
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

  const short int
  parser::yycheck_[] =
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

  const unsigned char
  parser::yystos_[] =
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
     123,   123,   123,   123,   123,   123,   124,   124,   125,   126,
     126,   127,   127,   127,   127,   127,   127,   127,   127,   127,
     127,   127,   127,   127,   127,   127,   127
  };

  const unsigned char
  parser::yyr2_[] =
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


#if YYDEBUG
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
  "var", "val", "maybe_expr", "expr", "expr_list", "expr_p", YY_NULLPTR
  };


  const unsigned short int
  parser::yyrline_[] =
  {
       0,    94,    94,    97,    98,   101,   102,   105,   106,   107,
     108,   109,   110,   111,   112,   113,   114,   115,   118,   121,
     124,   127,   130,   133,   134,   135,   136,   137,   138,   139,
     140,   141,   142,   143,   144,   145,   146,   147,   148,   149,
     150,   151,   154,   155,   156,   157,   158,   159,   162,   163,
     164,   167,   168,   169,   170,   171,   172,   173,   176,   177,
     180,   181,   184,   185,   189,   192,   193,   196,   199,   200,
     201,   202,   205,   206,   207,   210,   211,   214,   218,   219,
     220,   221,   224,   227,   228,   229,   230,   233,   236,   237,
     240,   241,   244,   245,   248,   251,   254,   255,   258,   259,
     260,   261,   264,   267,   270,   273,   276,   277,   280,   281,
     282,   283,   284,   285,   286,   287,   290,   291,   294,   296,
     297,   300,   301,   302,   303,   304,   305,   306,   307,   308,
     309,   310,   311,   312,   313,   314,   315
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
#line 1751 "src/parser.cpp" // lalr1.cc:1167
#line 318 "src/syntax.y" // lalr1.cc:1168


void yy::parser::error(const string& msg){
    cerr << msg << endl;
}

#endif
