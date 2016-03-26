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
#line 114 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[1].value));}
#line 610 "src/parser.cpp" // lalr1.cc:859
    break;

  case 4:
#line 115 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[1].value), (yystack_[0].value));}
#line 616 "src/parser.cpp" // lalr1.cc:859
    break;

  case 5:
#line 116 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[1].value));}
#line 622 "src/parser.cpp" // lalr1.cc:859
    break;

  case 6:
#line 117 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 628 "src/parser.cpp" // lalr1.cc:859
    break;

  case 21:
#line 145 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (Node*)lextxt;}
#line 634 "src/parser.cpp" // lalr1.cc:859
    break;

  case 22:
#line 148 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (Node*)lextxt;}
#line 640 "src/parser.cpp" // lalr1.cc:859
    break;

  case 23:
#line 151 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkIntLitNode(lextxt);}
#line 646 "src/parser.cpp" // lalr1.cc:859
    break;

  case 24:
#line 154 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFltLitNode(lextxt);}
#line 652 "src/parser.cpp" // lalr1.cc:859
    break;

  case 25:
#line 157 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkStrLitNode(lextxt);}
#line 658 "src/parser.cpp" // lalr1.cc:859
    break;

  case 26:
#line 160 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_I8,  (char*)"");}
#line 664 "src/parser.cpp" // lalr1.cc:859
    break;

  case 27:
#line 161 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_I16, (char*)"");}
#line 670 "src/parser.cpp" // lalr1.cc:859
    break;

  case 28:
#line 162 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_I32, (char*)"");}
#line 676 "src/parser.cpp" // lalr1.cc:859
    break;

  case 29:
#line 163 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_I64, (char*)"");}
#line 682 "src/parser.cpp" // lalr1.cc:859
    break;

  case 30:
#line 164 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_U8,  (char*)"");}
#line 688 "src/parser.cpp" // lalr1.cc:859
    break;

  case 31:
#line 165 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_U16, (char*)"");}
#line 694 "src/parser.cpp" // lalr1.cc:859
    break;

  case 32:
#line 166 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_U32, (char*)"");}
#line 700 "src/parser.cpp" // lalr1.cc:859
    break;

  case 33:
#line 167 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_U64, (char*)"");}
#line 706 "src/parser.cpp" // lalr1.cc:859
    break;

  case 34:
#line 168 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_Isz, (char*)"");}
#line 712 "src/parser.cpp" // lalr1.cc:859
    break;

  case 35:
#line 169 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_Usz, (char*)"");}
#line 718 "src/parser.cpp" // lalr1.cc:859
    break;

  case 36:
#line 170 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_F16, (char*)"");}
#line 724 "src/parser.cpp" // lalr1.cc:859
    break;

  case 37:
#line 171 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_F32, (char*)"");}
#line 730 "src/parser.cpp" // lalr1.cc:859
    break;

  case 38:
#line 172 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_F64, (char*)"");}
#line 736 "src/parser.cpp" // lalr1.cc:859
    break;

  case 39:
#line 173 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_C8,  (char*)"");}
#line 742 "src/parser.cpp" // lalr1.cc:859
    break;

  case 40:
#line 174 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_C32, (char*)"");}
#line 748 "src/parser.cpp" // lalr1.cc:859
    break;

  case 41:
#line 175 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_Bool, (char*)"");}
#line 754 "src/parser.cpp" // lalr1.cc:859
    break;

  case 42:
#line 176 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_Void, (char*)"");}
#line 760 "src/parser.cpp" // lalr1.cc:859
    break;

  case 43:
#line 177 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_Data, (char*)(yystack_[0].value));}
#line 766 "src/parser.cpp" // lalr1.cc:859
    break;

  case 44:
#line 178 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_TypeVar, (char*)(yystack_[0].value));}
#line 772 "src/parser.cpp" // lalr1.cc:859
    break;

  case 45:
#line 181 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_Ptr,  (char*)"", (yystack_[1].value));}
#line 778 "src/parser.cpp" // lalr1.cc:859
    break;

  case 46:
#line 182 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_Array,(char*)"", (yystack_[3].value));}
#line 784 "src/parser.cpp" // lalr1.cc:859
    break;

  case 47:
#line 183 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_Func, (char*)"", (yystack_[3].value));}
#line 790 "src/parser.cpp" // lalr1.cc:859
    break;

  case 48:
#line 184 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTypeNode(TT_Func, (char*)"", (yystack_[2].value));}
#line 796 "src/parser.cpp" // lalr1.cc:859
    break;

  case 49:
#line 185 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 802 "src/parser.cpp" // lalr1.cc:859
    break;

  case 50:
#line 186 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 808 "src/parser.cpp" // lalr1.cc:859
    break;

  case 51:
#line 189 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 814 "src/parser.cpp" // lalr1.cc:859
    break;

  case 53:
#line 191 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 820 "src/parser.cpp" // lalr1.cc:859
    break;

  case 54:
#line 194 "src/syntax.y" // lalr1.cc:859
    {Node* tmp = getRoot(); 
                        if(tmp == (yystack_[0].value)){//singular type, first type in list equals the last
                            (yylhs.value) = tmp;
                        }else{ //tuple type
                            (yylhs.value) = mkTypeNode(TT_Tuple, (char*)"", tmp);
                        }
                       }
#line 832 "src/parser.cpp" // lalr1.cc:859
    break;

  case 55:
#line 203 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pub);}
#line 838 "src/parser.cpp" // lalr1.cc:859
    break;

  case 56:
#line 204 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pri);}
#line 844 "src/parser.cpp" // lalr1.cc:859
    break;

  case 57:
#line 205 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pro);}
#line 850 "src/parser.cpp" // lalr1.cc:859
    break;

  case 58:
#line 206 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Raw);}
#line 856 "src/parser.cpp" // lalr1.cc:859
    break;

  case 59:
#line 207 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Const);}
#line 862 "src/parser.cpp" // lalr1.cc:859
    break;

  case 60:
#line 208 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Ext);}
#line 868 "src/parser.cpp" // lalr1.cc:859
    break;

  case 61:
#line 209 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Noinit);}
#line 874 "src/parser.cpp" // lalr1.cc:859
    break;

  case 62:
#line 210 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkModNode(Tok_Pathogen);}
#line 880 "src/parser.cpp" // lalr1.cc:859
    break;

  case 63:
#line 213 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[1].value), (yystack_[0].value));}
#line 886 "src/parser.cpp" // lalr1.cc:859
    break;

  case 64:
#line 214 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 892 "src/parser.cpp" // lalr1.cc:859
    break;

  case 65:
#line 217 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 898 "src/parser.cpp" // lalr1.cc:859
    break;

  case 66:
#line 221 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[2].value), (yystack_[4].value), (yystack_[3].value), (yystack_[0].value));}
#line 904 "src/parser.cpp" // lalr1.cc:859
    break;

  case 67:
#line 222 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[0].value), (yystack_[2].value), (yystack_[1].value),  0);}
#line 910 "src/parser.cpp" // lalr1.cc:859
    break;

  case 68:
#line 223 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[2].value), 0,  (yystack_[3].value), (yystack_[0].value));}
#line 916 "src/parser.cpp" // lalr1.cc:859
    break;

  case 69:
#line 224 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarDeclNode((char*)(yystack_[0].value), 0,  (yystack_[1].value),  0);}
#line 922 "src/parser.cpp" // lalr1.cc:859
    break;

  case 70:
#line 227 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[3].value), (yystack_[4].value), (yystack_[3].value), (yystack_[0].value));}
#line 928 "src/parser.cpp" // lalr1.cc:859
    break;

  case 71:
#line 228 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[3].value), (yystack_[3].value), 0,  (yystack_[0].value));}
#line 934 "src/parser.cpp" // lalr1.cc:859
    break;

  case 72:
#line 229 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[2].value), 0,  (yystack_[3].value), (yystack_[0].value));}
#line 940 "src/parser.cpp" // lalr1.cc:859
    break;

  case 73:
#line 230 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkLetBindingNode((char*)(yystack_[2].value), 0,  0,  (yystack_[0].value));}
#line 946 "src/parser.cpp" // lalr1.cc:859
    break;

  case 74:
#line 234 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarAssignNode((yystack_[2].value), (yystack_[0].value));}
#line 952 "src/parser.cpp" // lalr1.cc:859
    break;

  case 75:
#line 235 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarAssignNode((yystack_[2].value), mkBinOpNode('+', mkUnOpNode('*', (yystack_[2].value)), (yystack_[0].value)));}
#line 958 "src/parser.cpp" // lalr1.cc:859
    break;

  case 76:
#line 236 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarAssignNode((yystack_[2].value), mkBinOpNode('-', mkUnOpNode('*', (yystack_[2].value)), (yystack_[0].value)));}
#line 964 "src/parser.cpp" // lalr1.cc:859
    break;

  case 77:
#line 237 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarAssignNode((yystack_[2].value), mkBinOpNode('*', mkUnOpNode('*', (yystack_[2].value)), (yystack_[0].value)));}
#line 970 "src/parser.cpp" // lalr1.cc:859
    break;

  case 78:
#line 238 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarAssignNode((yystack_[2].value), mkBinOpNode('/', mkUnOpNode('*', (yystack_[2].value)), (yystack_[0].value)));}
#line 976 "src/parser.cpp" // lalr1.cc:859
    break;

  case 79:
#line 241 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 982 "src/parser.cpp" // lalr1.cc:859
    break;

  case 80:
#line 242 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 988 "src/parser.cpp" // lalr1.cc:859
    break;

  case 81:
#line 245 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 994 "src/parser.cpp" // lalr1.cc:859
    break;

  case 82:
#line 248 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[1].value), (yystack_[0].value));}
#line 1000 "src/parser.cpp" // lalr1.cc:859
    break;

  case 83:
#line 249 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[2].value), (yystack_[0].value));}
#line 1006 "src/parser.cpp" // lalr1.cc:859
    break;

  case 84:
#line 250 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[1].value), (yystack_[0].value));}
#line 1012 "src/parser.cpp" // lalr1.cc:859
    break;

  case 85:
#line 251 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkDataDeclNode((char*)(yystack_[2].value), (yystack_[0].value));}
#line 1018 "src/parser.cpp" // lalr1.cc:859
    break;

  case 86:
#line 254 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkNamedValNode(mkVarNode((char*)(yystack_[0].value)), (yystack_[1].value));}
#line 1024 "src/parser.cpp" // lalr1.cc:859
    break;

  case 87:
#line 255 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkNamedValNode(0, (yystack_[0].value));}
#line 1030 "src/parser.cpp" // lalr1.cc:859
    break;

  case 89:
#line 259 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 1036 "src/parser.cpp" // lalr1.cc:859
    break;

  case 90:
#line 260 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 1042 "src/parser.cpp" // lalr1.cc:859
    break;

  case 91:
#line 263 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1048 "src/parser.cpp" // lalr1.cc:859
    break;

  case 97:
#line 276 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1054 "src/parser.cpp" // lalr1.cc:859
    break;

  case 98:
#line 277 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1060 "src/parser.cpp" // lalr1.cc:859
    break;

  case 99:
#line 278 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1066 "src/parser.cpp" // lalr1.cc:859
    break;

  case 100:
#line 279 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1072 "src/parser.cpp" // lalr1.cc:859
    break;

  case 101:
#line 282 "src/syntax.y" // lalr1.cc:859
    {setNext((yystack_[2].value), (yystack_[1].value)); (yylhs.value) = getRoot();}
#line 1078 "src/parser.cpp" // lalr1.cc:859
    break;

  case 102:
#line 283 "src/syntax.y" // lalr1.cc:859
    {setNext((yystack_[2].value), (yystack_[1].value)); (yylhs.value) = getRoot();}
#line 1084 "src/parser.cpp" // lalr1.cc:859
    break;

  case 103:
#line 284 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1090 "src/parser.cpp" // lalr1.cc:859
    break;

  case 104:
#line 285 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1096 "src/parser.cpp" // lalr1.cc:859
    break;

  case 105:
#line 288 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[1].value), mkVarNode((char*)(yystack_[0].value)));}
#line 1102 "src/parser.cpp" // lalr1.cc:859
    break;

  case 106:
#line 289 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkVarNode((char*)(yystack_[0].value)));}
#line 1108 "src/parser.cpp" // lalr1.cc:859
    break;

  case 107:
#line 297 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[3].value), mkNamedValNode(getRoot(), (yystack_[1].value)));}
#line 1114 "src/parser.cpp" // lalr1.cc:859
    break;

  case 108:
#line 298 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkNamedValNode(getRoot(), (yystack_[1].value)));}
#line 1120 "src/parser.cpp" // lalr1.cc:859
    break;

  case 109:
#line 301 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1126 "src/parser.cpp" // lalr1.cc:859
    break;

  case 110:
#line 302 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1132 "src/parser.cpp" // lalr1.cc:859
    break;

  case 111:
#line 305 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[3].value), (yystack_[5].value), (yystack_[4].value), (yystack_[1].value), (yystack_[0].value));}
#line 1138 "src/parser.cpp" // lalr1.cc:859
    break;

  case 112:
#line 306 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[6].value), (yystack_[8].value), (yystack_[7].value), (yystack_[1].value), (yystack_[0].value));}
#line 1144 "src/parser.cpp" // lalr1.cc:859
    break;

  case 113:
#line 307 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[3].value), 0,  (yystack_[4].value), (yystack_[1].value), (yystack_[0].value));}
#line 1150 "src/parser.cpp" // lalr1.cc:859
    break;

  case 114:
#line 308 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncDeclNode((char*)(yystack_[6].value), 0,  (yystack_[7].value), (yystack_[1].value), (yystack_[0].value));}
#line 1156 "src/parser.cpp" // lalr1.cc:859
    break;

  case 115:
#line 311 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkFuncCallNode((char*)(yystack_[1].value), (yystack_[0].value));}
#line 1162 "src/parser.cpp" // lalr1.cc:859
    break;

  case 116:
#line 314 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkRetNode((yystack_[0].value));}
#line 1168 "src/parser.cpp" // lalr1.cc:859
    break;

  case 117:
#line 317 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setElse((IfNode*)(yystack_[3].value), (IfNode*)mkIfNode((yystack_[1].value), (yystack_[0].value)));}
#line 1174 "src/parser.cpp" // lalr1.cc:859
    break;

  case 118:
#line 318 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkIfNode((yystack_[1].value), (yystack_[0].value)));}
#line 1180 "src/parser.cpp" // lalr1.cc:859
    break;

  case 119:
#line 321 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setElse((IfNode*)(yystack_[2].value), (IfNode*)mkIfNode(NULL, (yystack_[0].value)));}
#line 1186 "src/parser.cpp" // lalr1.cc:859
    break;

  case 120:
#line 322 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1192 "src/parser.cpp" // lalr1.cc:859
    break;

  case 121:
#line 323 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(mkIfNode(NULL, (yystack_[0].value)));}
#line 1198 "src/parser.cpp" // lalr1.cc:859
    break;

  case 122:
#line 324 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot(NULL);}
#line 1204 "src/parser.cpp" // lalr1.cc:859
    break;

  case 123:
#line 327 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkIfNode((yystack_[2].value), (yystack_[1].value), (IfNode*)getRoot());}
#line 1210 "src/parser.cpp" // lalr1.cc:859
    break;

  case 124:
#line 330 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1216 "src/parser.cpp" // lalr1.cc:859
    break;

  case 125:
#line 333 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1222 "src/parser.cpp" // lalr1.cc:859
    break;

  case 126:
#line 336 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1228 "src/parser.cpp" // lalr1.cc:859
    break;

  case 127:
#line 339 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkVarNode((char*)(yystack_[0].value));}
#line 1234 "src/parser.cpp" // lalr1.cc:859
    break;

  case 128:
#line 342 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('&', (yystack_[0].value));}
#line 1240 "src/parser.cpp" // lalr1.cc:859
    break;

  case 129:
#line 343 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('*', (yystack_[0].value));}
#line 1246 "src/parser.cpp" // lalr1.cc:859
    break;

  case 130:
#line 344 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('[', mkRefVarNode((char*)(yystack_[3].value)), (yystack_[1].value));}
#line 1252 "src/parser.cpp" // lalr1.cc:859
    break;

  case 131:
#line 345 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkRefVarNode((char*)(yystack_[0].value));}
#line 1258 "src/parser.cpp" // lalr1.cc:859
    break;

  case 132:
#line 348 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1264 "src/parser.cpp" // lalr1.cc:859
    break;

  case 133:
#line 349 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1270 "src/parser.cpp" // lalr1.cc:859
    break;

  case 134:
#line 350 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1276 "src/parser.cpp" // lalr1.cc:859
    break;

  case 135:
#line 351 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1282 "src/parser.cpp" // lalr1.cc:859
    break;

  case 136:
#line 352 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[1].value);}
#line 1288 "src/parser.cpp" // lalr1.cc:859
    break;

  case 137:
#line 353 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1294 "src/parser.cpp" // lalr1.cc:859
    break;

  case 138:
#line 354 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1300 "src/parser.cpp" // lalr1.cc:859
    break;

  case 139:
#line 355 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1306 "src/parser.cpp" // lalr1.cc:859
    break;

  case 140:
#line 356 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1312 "src/parser.cpp" // lalr1.cc:859
    break;

  case 141:
#line 357 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1318 "src/parser.cpp" // lalr1.cc:859
    break;

  case 142:
#line 358 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBoolLitNode(1);}
#line 1324 "src/parser.cpp" // lalr1.cc:859
    break;

  case 143:
#line 359 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBoolLitNode(0);}
#line 1330 "src/parser.cpp" // lalr1.cc:859
    break;

  case 144:
#line 362 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTupleNode((yystack_[1].value));}
#line 1336 "src/parser.cpp" // lalr1.cc:859
    break;

  case 145:
#line 363 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkTupleNode(0);}
#line 1342 "src/parser.cpp" // lalr1.cc:859
    break;

  case 146:
#line 366 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkArrayNode((yystack_[1].value));}
#line 1348 "src/parser.cpp" // lalr1.cc:859
    break;

  case 147:
#line 367 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkArrayNode(0);}
#line 1354 "src/parser.cpp" // lalr1.cc:859
    break;

  case 148:
#line 370 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1360 "src/parser.cpp" // lalr1.cc:859
    break;

  case 149:
#line 371 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = NULL;}
#line 1366 "src/parser.cpp" // lalr1.cc:859
    break;

  case 150:
#line 374 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1372 "src/parser.cpp" // lalr1.cc:859
    break;

  case 151:
#line 377 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[2].value), (yystack_[0].value));}
#line 1378 "src/parser.cpp" // lalr1.cc:859
    break;

  case 152:
#line 378 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 1384 "src/parser.cpp" // lalr1.cc:859
    break;

  case 153:
#line 383 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('*', (yystack_[0].value));}
#line 1390 "src/parser.cpp" // lalr1.cc:859
    break;

  case 154:
#line 384 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('&', (yystack_[0].value));}
#line 1396 "src/parser.cpp" // lalr1.cc:859
    break;

  case 155:
#line 385 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkUnOpNode('-', (yystack_[0].value));}
#line 1402 "src/parser.cpp" // lalr1.cc:859
    break;

  case 156:
#line 388 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1408 "src/parser.cpp" // lalr1.cc:859
    break;

  case 157:
#line 391 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('+', (yystack_[2].value), (yystack_[0].value));}
#line 1414 "src/parser.cpp" // lalr1.cc:859
    break;

  case 158:
#line 392 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('-', (yystack_[2].value), (yystack_[0].value));}
#line 1420 "src/parser.cpp" // lalr1.cc:859
    break;

  case 159:
#line 393 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('*', (yystack_[2].value), (yystack_[0].value));}
#line 1426 "src/parser.cpp" // lalr1.cc:859
    break;

  case 160:
#line 394 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('/', (yystack_[2].value), (yystack_[0].value));}
#line 1432 "src/parser.cpp" // lalr1.cc:859
    break;

  case 161:
#line 395 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('%', (yystack_[2].value), (yystack_[0].value));}
#line 1438 "src/parser.cpp" // lalr1.cc:859
    break;

  case 162:
#line 396 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('<', (yystack_[2].value), (yystack_[0].value));}
#line 1444 "src/parser.cpp" // lalr1.cc:859
    break;

  case 163:
#line 397 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('>', (yystack_[2].value), (yystack_[0].value));}
#line 1450 "src/parser.cpp" // lalr1.cc:859
    break;

  case 164:
#line 398 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('^', (yystack_[2].value), (yystack_[0].value));}
#line 1456 "src/parser.cpp" // lalr1.cc:859
    break;

  case 165:
#line 399 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('.', (yystack_[2].value), (yystack_[0].value));}
#line 1462 "src/parser.cpp" // lalr1.cc:859
    break;

  case 166:
#line 400 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(';', (yystack_[3].value), (yystack_[0].value));}
#line 1468 "src/parser.cpp" // lalr1.cc:859
    break;

  case 167:
#line 401 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('[', (yystack_[3].value), (yystack_[1].value));}
#line 1474 "src/parser.cpp" // lalr1.cc:859
    break;

  case 168:
#line 402 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Where, (yystack_[4].value), mkLetBindingNode((char*)(yystack_[2].value), 0, 0, (yystack_[0].value)));}
#line 1480 "src/parser.cpp" // lalr1.cc:859
    break;

  case 169:
#line 403 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Eq, (yystack_[2].value), (yystack_[0].value));}
#line 1486 "src/parser.cpp" // lalr1.cc:859
    break;

  case 170:
#line 404 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_NotEq, (yystack_[2].value), (yystack_[0].value));}
#line 1492 "src/parser.cpp" // lalr1.cc:859
    break;

  case 171:
#line 405 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_GrtrEq, (yystack_[2].value), (yystack_[0].value));}
#line 1498 "src/parser.cpp" // lalr1.cc:859
    break;

  case 172:
#line 406 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_LesrEq, (yystack_[2].value), (yystack_[0].value));}
#line 1504 "src/parser.cpp" // lalr1.cc:859
    break;

  case 173:
#line 407 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Or, (yystack_[2].value), (yystack_[0].value));}
#line 1510 "src/parser.cpp" // lalr1.cc:859
    break;

  case 174:
#line 408 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_And, (yystack_[2].value), (yystack_[0].value));}
#line 1516 "src/parser.cpp" // lalr1.cc:859
    break;

  case 175:
#line 409 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1522 "src/parser.cpp" // lalr1.cc:859
    break;

  case 176:
#line 414 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = getRoot();}
#line 1528 "src/parser.cpp" // lalr1.cc:859
    break;

  case 177:
#line 417 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setNext((yystack_[3].value), (yystack_[0].value));}
#line 1534 "src/parser.cpp" // lalr1.cc:859
    break;

  case 178:
#line 418 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = setRoot((yystack_[0].value));}
#line 1540 "src/parser.cpp" // lalr1.cc:859
    break;

  case 179:
#line 421 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('+', (yystack_[3].value), (yystack_[0].value));}
#line 1546 "src/parser.cpp" // lalr1.cc:859
    break;

  case 180:
#line 422 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('-', (yystack_[3].value), (yystack_[0].value));}
#line 1552 "src/parser.cpp" // lalr1.cc:859
    break;

  case 181:
#line 423 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('*', (yystack_[3].value), (yystack_[0].value));}
#line 1558 "src/parser.cpp" // lalr1.cc:859
    break;

  case 182:
#line 424 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('/', (yystack_[3].value), (yystack_[0].value));}
#line 1564 "src/parser.cpp" // lalr1.cc:859
    break;

  case 183:
#line 425 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('%', (yystack_[3].value), (yystack_[0].value));}
#line 1570 "src/parser.cpp" // lalr1.cc:859
    break;

  case 184:
#line 426 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('<', (yystack_[3].value), (yystack_[0].value));}
#line 1576 "src/parser.cpp" // lalr1.cc:859
    break;

  case 185:
#line 427 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('>', (yystack_[3].value), (yystack_[0].value));}
#line 1582 "src/parser.cpp" // lalr1.cc:859
    break;

  case 186:
#line 428 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('^', (yystack_[3].value), (yystack_[0].value));}
#line 1588 "src/parser.cpp" // lalr1.cc:859
    break;

  case 187:
#line 429 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('.', (yystack_[3].value), (yystack_[0].value));}
#line 1594 "src/parser.cpp" // lalr1.cc:859
    break;

  case 188:
#line 430 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(';', (yystack_[3].value), (yystack_[0].value));}
#line 1600 "src/parser.cpp" // lalr1.cc:859
    break;

  case 189:
#line 431 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode('[', (yystack_[4].value), (yystack_[2].value));}
#line 1606 "src/parser.cpp" // lalr1.cc:859
    break;

  case 190:
#line 432 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Where, (yystack_[4].value), mkLetBindingNode((char*)(yystack_[2].value), 0, 0, (yystack_[0].value)));}
#line 1612 "src/parser.cpp" // lalr1.cc:859
    break;

  case 191:
#line 433 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Eq, (yystack_[3].value), (yystack_[0].value));}
#line 1618 "src/parser.cpp" // lalr1.cc:859
    break;

  case 192:
#line 434 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_NotEq, (yystack_[3].value), (yystack_[0].value));}
#line 1624 "src/parser.cpp" // lalr1.cc:859
    break;

  case 193:
#line 435 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_GrtrEq, (yystack_[3].value), (yystack_[0].value));}
#line 1630 "src/parser.cpp" // lalr1.cc:859
    break;

  case 194:
#line 436 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_LesrEq, (yystack_[3].value), (yystack_[0].value));}
#line 1636 "src/parser.cpp" // lalr1.cc:859
    break;

  case 195:
#line 437 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_Or, (yystack_[3].value), (yystack_[0].value));}
#line 1642 "src/parser.cpp" // lalr1.cc:859
    break;

  case 196:
#line 438 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = mkBinOpNode(Tok_And, (yystack_[3].value), (yystack_[0].value));}
#line 1648 "src/parser.cpp" // lalr1.cc:859
    break;

  case 197:
#line 439 "src/syntax.y" // lalr1.cc:859
    {(yylhs.value) = (yystack_[0].value);}
#line 1654 "src/parser.cpp" // lalr1.cc:859
    break;


#line 1658 "src/parser.cpp" // lalr1.cc:859
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


  const short int parser::yypact_ninf_ = -270;

  const signed char parser::yytable_ninf_ = -45;

  const short int
  parser::yypact_[] =
  {
     -42,  -270,    28,   536,  -270,  -270,  -270,  -270,  -270,  -270,
    -270,  -270,  -270,  -270,  -270,  -270,  -270,  -270,  -270,  -270,
    -270,  -270,  -270,  -270,   410,   410,   640,   410,    -7,   640,
      38,     8,  -270,  -270,  -270,  -270,  -270,  -270,  -270,  -270,
      11,   767,    11,   451,  -270,   -19,   121,  -270,  -270,   -56,
     -48,    50,  -270,   252,   718,  -270,  -270,  -270,  -270,  -270,
    -270,  -270,  -270,  -270,  -270,  -270,  -270,    33,  -270,  -270,
    -270,  -270,  -270,   410,   410,   410,   107,   446,   410,   -12,
    -270,  -270,  -270,  -270,  -270,  -270,  -270,  -270,  -270,  -270,
     797,    41,  -270,    50,   767,    27,    41,   410,    24,    50,
     767,   -21,    38,    56,  -270,    34,  -270,    44,  -270,  -270,
    -270,    66,  -270,   107,   410,  -270,  -270,   737,   410,   767,
     767,   -39,  -270,    38,     8,    50,   410,   410,   410,   410,
     410,  -270,    65,    68,   854,  -270,  -270,  -270,    59,    75,
      67,  -270,    62,  -270,  -270,   410,   410,   410,   410,   410,
     410,    50,   -42,   410,   410,   410,   410,   410,   410,   410,
     410,   410,   410,   536,    73,    70,    50,   410,  -270,    41,
     410,    71,    72,    50,   621,    38,    88,  -270,    76,   -47,
    -270,  -270,  -270,    79,  -270,    95,    94,  -270,   -56,   -56,
     410,   410,   767,   -21,    56,  -270,    -8,  -270,  -270,  -270,
    -270,  -270,  -270,   -42,   -42,   -42,   -42,   -42,   -42,   -42,
      50,   -42,   -42,   -42,   -42,   -42,   -42,   -42,   -42,   -42,
     -42,   410,  -270,   410,  -270,  -270,   -11,   -11,   -11,   -11,
     942,   344,    93,   410,   -11,   -11,   162,   162,     3,     3,
       3,     3,    77,    96,   536,   116,   -10,   410,    41,    87,
    -270,    98,    41,  -270,  -270,   410,   410,    99,    50,   134,
    -270,    10,  -270,  -270,    17,  -270,   410,    38,  -270,  -270,
    -270,  -270,   112,  -270,    50,   123,    41,    88,  -270,  -270,
     410,   410,   767,   410,   410,   410,   410,   410,   410,   410,
     108,   410,   410,   410,   410,   410,   410,   410,   410,   410,
     410,   778,  -270,   410,   873,  -270,   130,    23,  -270,  -270,
      41,  -270,   410,    41,  -270,  -270,  -270,   410,  -270,   621,
    -270,    38,  -270,  -270,   113,   114,  -270,    50,   767,  -270,
    -270,   120,  -270,    41,   854,   173,   173,   173,   173,   954,
     494,   410,   930,   173,   173,   199,   199,    30,    30,    30,
      30,   122,   -42,   873,  -270,  -270,  -270,    41,  -270,  -270,
    -270,  -270,   410,   767,  -270,    50,   131,  -270,   930,  -270,
    -270,  -270,    41,    50,   767,  -270,    41,  -270
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
     139,   140,   141,   132,   138,   175,   134,   135,   137,   116,
     156,     0,    44,     0,     0,     0,     0,     0,    44,     0,
       0,     0,     0,     0,   100,   131,   129,     0,   128,     2,
       4,     0,     5,     0,     0,   115,    45,     0,   149,     0,
       0,    69,    63,     0,     0,     0,     0,     0,     0,     0,
       0,   197,     0,   176,   178,   155,   153,   145,     0,   150,
     152,   147,     0,   152,   154,     0,     0,     0,     0,     0,
       0,     0,     8,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,   122,    69,     0,     0,   124,     0,
       0,     0,    44,     0,     0,     0,     0,    84,    95,     0,
      98,    49,     3,     0,    48,     0,     0,   148,    51,    52,
     149,     0,   110,     0,     0,    99,    67,    75,    76,    77,
      78,    74,   136,     8,     8,     8,     8,     8,     8,     8,
       0,     8,     8,     8,     8,     8,     8,     8,     8,     8,
       8,     0,   144,     0,   133,   146,   169,   170,   171,   172,
     173,   174,     0,     0,   162,   163,   157,   158,   159,   160,
     161,   164,   165,     0,     0,     6,     0,     0,     0,   120,
     123,    67,     0,   125,    73,     0,     0,     0,    87,     0,
      90,     0,    88,    80,     0,    85,     0,     0,    96,   130,
      47,    46,     0,    68,     0,   109,     0,     0,    82,    97,
     149,     0,   110,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,   151,     0,   166,   167,     4,     0,   103,   104,
       0,   121,     0,     0,   126,    72,    71,     0,    86,     0,
      91,     0,    81,    94,    92,     0,   106,   108,     0,   113,
      83,     0,    66,     0,   177,   191,   192,   193,   194,   195,
     196,     0,   188,   184,   185,   179,   180,   181,   182,   183,
     186,   187,     8,   168,   101,   102,   118,     0,   119,    70,
      89,    79,     0,   110,   105,     0,     0,   111,   190,   189,
     117,    93,     0,   107,   110,   114,     0,   112
  };

  const short int
  parser::yypgoto_[] =
  {
    -270,  -270,    43,     7,   -16,    -9,    -3,   -23,  -270,  -270,
    -270,  -270,     9,  -270,   -25,   156,  -270,   -24,   204,  -270,
    -270,  -270,    40,  -270,   -88,  -270,  -173,  -270,   -92,  -168,
     -44,  -131,  -270,  -269,  -270,    12,  -270,  -270,  -270,  -270,
    -270,  -270,  -270,  -270,    53,    91,   -36,  -270,  -181,   165,
    -270,  -270,     6,   197,  -270,  -270,   109
  };

  const short int
  parser::yydefgoto_[] =
  {
      -1,     2,    43,     3,    44,    45,    79,    47,    80,    81,
      82,    48,    49,    50,    51,    52,    53,    54,    55,    56,
      57,   264,   176,    58,   260,   261,   177,   179,   104,    59,
     164,   327,   275,   276,    60,    83,    62,   249,   250,    63,
      64,    65,    66,    84,    67,    85,    86,    87,   186,   138,
     139,    88,   187,    90,   132,   133,   134
  };

  const short int
  parser::yytable_[] =
  {
      46,    93,    94,   265,    99,   100,   262,   101,   103,   272,
     115,   180,     6,   333,     5,    61,   107,   267,   116,   268,
     278,   119,     1,    92,   117,   118,    98,   110,     4,   125,
      89,    91,   195,    96,   111,    97,   120,   105,    92,   105,
      46,   190,     6,   115,   174,   112,   191,   192,   121,   175,
     109,    92,   168,     5,   112,    61,   309,   126,   127,   128,
     129,   155,   156,   157,   158,   159,   160,   161,   113,   166,
     162,   167,   280,   102,   319,   173,   320,   281,   282,   178,
     160,   161,   140,   143,   162,    40,   321,   182,   322,   355,
     165,    92,   185,   106,   372,   108,   171,   172,    42,   331,
     193,   194,   279,   169,   330,   376,   163,   219,   220,   170,
       5,   221,   247,   248,    92,   114,    92,    92,   130,   143,
     183,   102,   196,   181,   -44,   253,   312,   313,   188,   189,
     182,   202,   197,   198,   199,   200,   201,   203,   222,    68,
      69,    70,    71,    72,   223,   225,   224,   245,   232,   258,
     259,   262,   263,   174,   246,   191,   255,   256,   162,   233,
      46,   266,   269,   251,   131,   135,   136,   274,   243,   144,
     257,    92,    73,   252,   270,    61,   254,   271,   303,   305,
      74,    75,   308,   281,   317,   124,   137,    76,    77,    92,
     -44,   325,   328,   341,    78,   -44,   354,   273,   362,   366,
     363,   113,   114,   221,   311,   -44,   244,   290,   314,   122,
     283,   284,   285,   286,   287,   288,   289,   374,   291,   292,
     293,   294,   295,   296,   297,   298,   299,   300,   306,   302,
      95,   360,   329,   277,   373,   307,   157,   158,   159,   160,
     161,    46,   142,   162,   324,   214,   215,   216,   217,   218,
     219,   220,     0,   310,   221,   318,    61,   274,     0,     0,
       0,   315,   316,     0,     0,     0,   356,     0,     0,   358,
       0,   326,   323,   216,   217,   218,   219,   220,     0,    92,
     221,     0,     0,     0,     0,     0,     0,   332,     0,   367,
       0,     0,     0,     0,   258,   259,     0,     0,   361,     0,
       0,     0,     0,   365,    32,    33,    34,    35,    36,    37,
      38,    39,   131,   370,     0,     0,    92,     0,   357,     0,
       0,     0,     0,   359,   364,    92,     0,     0,   375,     0,
     301,     0,   377,     0,     0,     0,     0,     0,   274,     0,
       0,     0,   226,   227,   228,   229,   230,   231,     0,   274,
     234,   235,   236,   237,   238,   239,   240,   241,   242,   369,
      92,     0,   326,     0,     0,     0,   145,   146,   371,     0,
     364,    92,   147,   148,   131,   131,   131,   131,   131,   131,
     131,     0,   131,   131,   131,   131,   131,   131,   131,   131,
     131,   131,   334,   335,   336,   337,   338,   339,   340,     0,
     342,   343,   344,   345,   346,   347,   348,   349,   350,   351,
       0,     0,     0,     5,   153,   154,   155,   156,   157,   158,
     159,   160,   161,     0,     0,   162,     0,     0,     0,     0,
     304,     0,   131,     0,     0,     0,     0,     0,     0,     0,
       0,     0,    68,    69,    70,    71,    72,     0,     0,     5,
     368,     0,     0,     0,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    22,    23,     0,     0,    73,     0,     0,    68,    69,
      70,    71,    72,    74,    75,     0,     0,     0,    24,    25,
      76,    77,    26,    27,    28,     0,     0,    78,     0,    29,
     353,    30,    31,    32,    33,    34,    35,    36,    37,    38,
      39,    73,     0,     0,     0,     1,   204,   205,     0,    74,
      75,     0,   206,   207,     0,    40,    76,    77,     0,   141,
       0,    41,     0,    78,     0,     0,     0,     0,    42,     5,
       6,     7,     8,     9,    10,    11,    12,    13,    14,    15,
      16,    17,    18,    19,    20,    21,    22,    23,     0,     0,
       0,     0,     0,     0,   212,   213,   214,   215,   216,   217,
     218,   219,   220,    24,    25,   221,     0,    26,    27,    28,
       0,     0,     0,     0,    29,     0,    30,    31,    32,    33,
      34,    35,    36,    37,    38,    39,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
      40,     0,     0,     0,     0,     0,    41,     0,     0,     0,
       0,     0,     0,    42,     5,     6,     7,     8,     9,    10,
      11,    12,    13,    14,    15,    16,    17,    18,    19,    20,
      21,    22,    23,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
      22,    23,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,    31,    32,    33,    34,    35,    36,    37,    38,
      39,     0,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,    32,    33,    34,    35,    36,    37,    38,    39,
       0,    41,     0,     0,     0,     0,     0,     0,     0,     0,
       0,     0,     0,     0,     0,     0,     0,     0,     0,     0,
      41,     5,     6,     7,     8,     9,    10,    11,    12,    13,
      14,    15,    16,    17,    18,    19,    20,    21,    22,    23,
       5,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    22,    23,     0,
       0,     0,     0,     0,     0,     0,     0,     0,   123,   124,
       5,     6,     7,     8,     9,    10,    11,    12,    13,    14,
      15,    16,    17,    18,    19,    20,    21,    22,    23,     0,
       0,     0,     0,     0,     0,     0,     0,     0,    41,     0,
     204,   205,     0,     0,     0,     0,   206,   207,   208,   209,
       0,     0,     0,     0,     0,     0,   184,    41,     0,   145,
     146,     0,     0,     0,     0,   147,   148,   149,   150,     0,
       0,     0,     0,     0,     0,     0,     0,     0,   210,     0,
       0,     0,     0,     0,     0,     0,   211,    41,   212,   213,
     214,   215,   216,   217,   218,   219,   220,   151,     0,   221,
       0,   352,     0,     0,     0,   152,     0,   153,   154,   155,
     156,   157,   158,   159,   160,   161,   204,   205,   162,     0,
       0,     0,   206,   207,   208,   209,     0,     0,     0,     0,
       0,     0,     0,     0,     0,   145,   146,     0,     0,     0,
       0,   147,   148,   149,   150,     0,     0,     0,     0,     0,
       0,     0,     0,     0,   210,     0,     0,     0,     0,     0,
       0,     0,   211,     0,   212,   213,   214,   215,   216,   217,
     218,   219,   220,   151,     0,   221,     0,     0,     0,     0,
       0,     0,     0,   153,   154,   155,   156,   157,   158,   159,
     160,   161,   204,   205,   162,     0,     0,     0,   206,   207,
     208,   209,     0,     0,   145,   146,     0,     0,     0,     0,
     147,   148,     0,   150,     0,     0,   204,   205,     0,     0,
       0,     0,   206,   207,     0,   209,     0,     0,     0,     0,
     210,     0,     0,     0,     0,     0,     0,     0,     0,     0,
     212,   213,   214,   215,   216,   217,   218,   219,   220,     0,
       0,   221,   153,   154,   155,   156,   157,   158,   159,   160,
     161,     0,     0,   162,   212,   213,   214,   215,   216,   217,
     218,   219,   220,     0,     0,   221
  };

  const short int
  parser::yycheck_[] =
  {
       3,    26,    26,   176,    29,    29,   174,    30,    31,   190,
      46,   103,     4,   282,     3,     3,    41,    64,    74,    66,
     193,    69,    64,    26,    80,    81,    29,    43,     0,    54,
      24,    25,   124,    27,    43,    42,    84,    40,    41,    42,
      43,    80,     4,    79,    65,    64,    85,    86,    51,    70,
      43,    54,    96,     3,    64,    43,    66,    24,    25,    26,
      27,    72,    73,    74,    75,    76,    77,    78,    80,    94,
      81,    44,    80,    65,    64,   100,    66,    85,    86,   102,
      77,    78,    76,    77,    81,    74,    69,    64,    71,    66,
      93,    94,   117,    40,   363,    42,    99,   100,    87,   280,
     123,   124,   194,    97,   277,   374,    65,    77,    78,    85,
       3,    81,    39,    40,   117,    81,   119,   120,    85,   113,
     114,    65,   125,    79,     3,   169,    39,    40,   119,   120,
      64,    66,   126,   127,   128,   129,   130,    69,    79,    32,
      33,    34,    35,    36,    69,    83,    79,   163,   151,   174,
     174,   319,   175,    65,   163,    85,    85,    85,    81,   152,
     163,    85,    83,   166,    73,    74,    75,   192,   162,    78,
     173,   174,    65,   167,    79,   163,   170,    83,    85,    83,
      73,    74,    66,    85,    85,    51,    79,    80,    81,   192,
      69,    79,    69,    85,    87,    74,    66,   191,    85,    79,
      86,    80,    81,    81,   248,    84,   163,   210,   252,    53,
     203,   204,   205,   206,   207,   208,   209,    86,   211,   212,
     213,   214,   215,   216,   217,   218,   219,   220,   244,   223,
      26,   319,   276,   193,   365,   244,    74,    75,    76,    77,
      78,   244,    77,    81,   267,    72,    73,    74,    75,    76,
      77,    78,    -1,   247,    81,   258,   244,   282,    -1,    -1,
      -1,   255,   256,    -1,    -1,    -1,   310,    -1,    -1,   313,
      -1,   274,   266,    74,    75,    76,    77,    78,    -1,   282,
      81,    -1,    -1,    -1,    -1,    -1,    -1,   281,    -1,   333,
      -1,    -1,    -1,    -1,   319,   319,    -1,    -1,   321,    -1,
      -1,    -1,    -1,   328,    52,    53,    54,    55,    56,    57,
      58,    59,   221,   357,    -1,    -1,   319,    -1,   312,    -1,
      -1,    -1,    -1,   317,   327,   328,    -1,    -1,   372,    -1,
     221,    -1,   376,    -1,    -1,    -1,    -1,    -1,   363,    -1,
      -1,    -1,   145,   146,   147,   148,   149,   150,    -1,   374,
     153,   154,   155,   156,   157,   158,   159,   160,   161,   352,
     363,    -1,   365,    -1,    -1,    -1,    22,    23,   362,    -1,
     373,   374,    28,    29,   283,   284,   285,   286,   287,   288,
     289,    -1,   291,   292,   293,   294,   295,   296,   297,   298,
     299,   300,   283,   284,   285,   286,   287,   288,   289,    -1,
     291,   292,   293,   294,   295,   296,   297,   298,   299,   300,
      -1,    -1,    -1,     3,    70,    71,    72,    73,    74,    75,
      76,    77,    78,    -1,    -1,    81,    -1,    -1,    -1,    -1,
     233,    -1,   341,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    32,    33,    34,    35,    36,    -1,    -1,     3,
     341,    -1,    -1,    -1,     3,     4,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,    -1,    -1,    65,    -1,    -1,    32,    33,
      34,    35,    36,    73,    74,    -1,    -1,    -1,    37,    38,
      80,    81,    41,    42,    43,    -1,    -1,    87,    -1,    48,
     303,    50,    51,    52,    53,    54,    55,    56,    57,    58,
      59,    65,    -1,    -1,    -1,    64,    22,    23,    -1,    73,
      74,    -1,    28,    29,    -1,    74,    80,    81,    -1,    83,
      -1,    80,    -1,    87,    -1,    -1,    -1,    -1,    87,     3,
       4,     5,     6,     7,     8,     9,    10,    11,    12,    13,
      14,    15,    16,    17,    18,    19,    20,    21,    -1,    -1,
      -1,    -1,    -1,    -1,    70,    71,    72,    73,    74,    75,
      76,    77,    78,    37,    38,    81,    -1,    41,    42,    43,
      -1,    -1,    -1,    -1,    48,    -1,    50,    51,    52,    53,
      54,    55,    56,    57,    58,    59,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      74,    -1,    -1,    -1,    -1,    -1,    80,    -1,    -1,    -1,
      -1,    -1,    -1,    87,     3,     4,     5,     6,     7,     8,
       9,    10,    11,    12,    13,    14,    15,    16,    17,    18,
      19,    20,    21,     3,     4,     5,     6,     7,     8,     9,
      10,    11,    12,    13,    14,    15,    16,    17,    18,    19,
      20,    21,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    51,    52,    53,    54,    55,    56,    57,    58,
      59,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    52,    53,    54,    55,    56,    57,    58,    59,
      -1,    80,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      80,     3,     4,     5,     6,     7,     8,     9,    10,    11,
      12,    13,    14,    15,    16,    17,    18,    19,    20,    21,
       3,     4,     5,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    21,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    50,    51,
       3,     4,     5,     6,     7,     8,     9,    10,    11,    12,
      13,    14,    15,    16,    17,    18,    19,    20,    21,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    80,    -1,
      22,    23,    -1,    -1,    -1,    -1,    28,    29,    30,    31,
      -1,    -1,    -1,    -1,    -1,    -1,    79,    80,    -1,    22,
      23,    -1,    -1,    -1,    -1,    28,    29,    30,    31,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    60,    -1,
      -1,    -1,    -1,    -1,    -1,    -1,    68,    80,    70,    71,
      72,    73,    74,    75,    76,    77,    78,    60,    -1,    81,
      -1,    83,    -1,    -1,    -1,    68,    -1,    70,    71,    72,
      73,    74,    75,    76,    77,    78,    22,    23,    81,    -1,
      -1,    -1,    28,    29,    30,    31,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    -1,    22,    23,    -1,    -1,    -1,
      -1,    28,    29,    30,    31,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    -1,    60,    -1,    -1,    -1,    -1,    -1,
      -1,    -1,    68,    -1,    70,    71,    72,    73,    74,    75,
      76,    77,    78,    60,    -1,    81,    -1,    -1,    -1,    -1,
      -1,    -1,    -1,    70,    71,    72,    73,    74,    75,    76,
      77,    78,    22,    23,    81,    -1,    -1,    -1,    28,    29,
      30,    31,    -1,    -1,    22,    23,    -1,    -1,    -1,    -1,
      28,    29,    -1,    31,    -1,    -1,    22,    23,    -1,    -1,
      -1,    -1,    28,    29,    -1,    31,    -1,    -1,    -1,    -1,
      60,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,    -1,
      70,    71,    72,    73,    74,    75,    76,    77,    78,    -1,
      -1,    81,    70,    71,    72,    73,    74,    75,    76,    77,
      78,    -1,    -1,    81,    70,    71,    72,    73,    74,    75,
      76,    77,    78,    -1,    -1,    81
  };

  const unsigned char
  parser::yystos_[] =
  {
       0,    64,    89,    91,     0,     3,     4,     5,     6,     7,
       8,     9,    10,    11,    12,    13,    14,    15,    16,    17,
      18,    19,    20,    21,    37,    38,    41,    42,    43,    48,
      50,    51,    52,    53,    54,    55,    56,    57,    58,    59,
      74,    80,    87,    90,    92,    93,    94,    95,    99,   100,
     101,   102,   103,   104,   105,   106,   107,   108,   111,   117,
     122,   123,   124,   127,   128,   129,   130,   132,    32,    33,
      34,    35,    36,    65,    73,    74,    80,    81,    87,    94,
      96,    97,    98,   123,   131,   133,   134,   135,   139,   140,
     141,   140,    94,   102,   105,   106,   140,    42,    94,   102,
     105,    95,    65,    95,   116,    94,   132,   102,   132,    91,
      92,    93,    64,    80,    81,   134,    74,    80,    81,    69,
      84,    94,   103,    50,    51,   102,    24,    25,    26,    27,
      85,   133,   142,   143,   144,   133,   133,    79,   137,   138,
     140,    83,   137,   140,   133,    22,    23,    28,    29,    30,
      31,    60,    68,    70,    71,    72,    73,    74,    75,    76,
      77,    78,    81,    65,   118,    94,   102,    44,   118,   140,
      85,    94,    94,   102,    65,    70,   110,   114,    95,   115,
     116,    79,    64,   140,    79,   102,   136,   140,   100,   100,
      80,    85,    86,    95,    95,   116,    94,   140,   140,   140,
     140,   140,    66,    69,    22,    23,    28,    29,    30,    31,
      60,    68,    70,    71,    72,    73,    74,    75,    76,    77,
      78,    81,    79,    69,    79,    83,   141,   141,   141,   141,
     141,   141,    94,    91,   141,   141,   141,   141,   141,   141,
     141,   141,   141,   140,    90,    92,    93,    39,    40,   125,
     126,    94,   140,   118,   140,    85,    85,    94,   102,   105,
     112,   113,   117,    95,   109,   114,    85,    64,    66,    83,
      79,    83,   136,   140,   102,   120,   121,   110,   114,   116,
      80,    85,    86,    91,    91,    91,    91,    91,    91,    91,
      94,    91,    91,    91,    91,    91,    91,    91,    91,    91,
      91,   144,   140,    85,   141,    83,    92,    93,    66,    66,
     140,   118,    39,    40,   118,   140,   140,    85,    94,    64,
      66,    69,    71,   140,    95,    79,    94,   119,    69,   118,
     114,   136,   140,   121,   144,   144,   144,   144,   144,   144,
     144,    85,   144,   144,   144,   144,   144,   144,   144,   144,
     144,   144,    83,   141,    66,    66,   118,   140,   118,   140,
     112,    95,    85,    86,    94,   102,    79,   118,   144,    91,
     118,   140,   121,   119,    86,   118,   121,   118
  };

  const unsigned char
  parser::yyr1_[] =
  {
       0,    88,    89,    90,    90,    90,    90,    91,    91,    92,
      92,    92,    92,    92,    92,    92,    93,    93,    93,    93,
      93,    94,    95,    96,    97,    98,    99,    99,    99,    99,
      99,    99,    99,    99,    99,    99,    99,    99,    99,    99,
      99,    99,    99,    99,    99,   100,   100,   100,   100,   100,
     100,   101,   101,   101,   102,   103,   103,   103,   103,   103,
     103,   103,   103,   104,   104,   105,   106,   106,   106,   106,
     107,   107,   107,   107,   108,   108,   108,   108,   108,   109,
     109,   110,   111,   111,   111,   111,   112,   112,   112,   113,
     113,   114,   115,   115,   115,   115,   116,   117,   117,   117,
     117,   118,   118,   118,   118,   119,   119,   120,   120,   121,
     121,   122,   122,   122,   122,   123,   124,   125,   125,   126,
     126,   126,   126,   127,   128,   129,   130,   131,   132,   132,
     132,   132,   133,   133,   133,   133,   133,   133,   133,   133,
     133,   133,   133,   133,   134,   134,   135,   135,   136,   136,
     137,   138,   138,   139,   139,   139,   140,   141,   141,   141,
     141,   141,   141,   141,   141,   141,   141,   141,   141,   141,
     141,   141,   141,   141,   141,   141,   142,   143,   143,   144,
     144,   144,   144,   144,   144,   144,   144,   144,   144,   144,
     144,   144,   144,   144,   144,   144,   144,   144
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
       1,     3,     1,     2,     2,     2,     1,     3,     3,     3,
       3,     3,     3,     3,     3,     3,     4,     4,     5,     3,
       3,     3,     3,     3,     3,     1,     1,     4,     1,     4,
       4,     4,     4,     4,     4,     4,     4,     4,     4,     5,
       5,     4,     4,     4,     4,     4,     4,     1
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
  "LOW", "';'", "','", "'<'", "'>'", "'+'", "'-'", "'*'", "'/'", "'%'",
  "'^'", "'.'", "')'", "'('", "'['", "HIGH", "']'", "'|'", "'='", "':'",
  "'&'", "$accept", "top_level_stmt_list", "stmt_list", "maybe_newline",
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
       0,   111,   111,   114,   115,   116,   117,   120,   121,   128,
     129,   130,   131,   132,   133,   134,   138,   139,   140,   141,
     142,   145,   148,   151,   154,   157,   160,   161,   162,   163,
     164,   165,   166,   167,   168,   169,   170,   171,   172,   173,
     174,   175,   176,   177,   178,   181,   182,   183,   184,   185,
     186,   189,   190,   191,   194,   203,   204,   205,   206,   207,
     208,   209,   210,   213,   214,   217,   221,   222,   223,   224,
     227,   228,   229,   230,   234,   235,   236,   237,   238,   241,
     242,   245,   248,   249,   250,   251,   254,   255,   256,   259,
     260,   263,   267,   268,   269,   270,   273,   276,   277,   278,
     279,   282,   283,   284,   285,   288,   289,   297,   298,   301,
     302,   305,   306,   307,   308,   311,   314,   317,   318,   321,
     322,   323,   324,   327,   330,   333,   336,   339,   342,   343,
     344,   345,   348,   349,   350,   351,   352,   353,   354,   355,
     356,   357,   358,   359,   362,   363,   366,   367,   370,   371,
     374,   377,   378,   383,   384,   385,   388,   391,   392,   393,
     394,   395,   396,   397,   398,   399,   400,   401,   402,   403,
     404,   405,   406,   407,   408,   409,   414,   417,   418,   421,
     422,   423,   424,   425,   426,   427,   428,   429,   430,   431,
     432,   433,   434,   435,   436,   437,   438,   439
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
       2,     2,     2,     2,     2,     2,     2,    76,    87,     2,
      80,    79,    74,    72,    69,    73,    78,    75,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,    86,    68,
      70,    85,    71,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,    81,     2,    83,    77,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,     2,     2,     2,     2,     2,     2,
       2,     2,     2,     2,    84,     2,     2,     2,     2,     2,
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
      65,    66,    67,    82
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
#line 2472 "src/parser.cpp" // lalr1.cc:1167
#line 441 "src/syntax.y" // lalr1.cc:1168


/* location parser error
void yy::parser::error(const location& loc, const string& msg){
    ante::error(msg.c_str(), yylexer->fileName, yylexer->getRow(), yylexer->getCol());
} */

void yy::parser::error(const string& msg){
    ante::error(msg.c_str(), yylexer->fileName, yylexer->getRow(), yylexer->getCol());
}

#endif
