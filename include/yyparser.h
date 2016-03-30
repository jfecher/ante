// A Bison parser, made by GNU Bison 3.0.4.

// Skeleton interface for Bison GLR parsers in C++

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

// C++ GLR parser skeleton written by Akim Demaille.

#ifndef YY_YY_INCLUDE_YYPARSER_H_INCLUDED
# define YY_YY_INCLUDE_YYPARSER_H_INCLUDED


#include <stdexcept>
#include <string>
#include <iostream>


/* Debug traces.  */
#ifndef YYDEBUG
# define YYDEBUG 0
#endif


namespace yy {
#line 52 "include/yyparser.h" // glr.cc:329


  /// A Bison parser.
  class parser
  {
  public:
#ifndef YYSTYPE
    /// Symbol semantic values.
    typedef int semantic_type;
#else
    typedef YYSTYPE semantic_type;
#endif

    /// Syntax errors thrown from user actions.
    struct syntax_error : std::runtime_error
    {
      syntax_error (const std::string& m);
    };

    /// Tokens.
    struct token
    {
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
        Let = 303,
        Match = 304,
        Data = 305,
        Enum = 306,
        Pub = 307,
        Pri = 308,
        Pro = 309,
        Raw = 310,
        Const = 311,
        Ext = 312,
        Noinit = 313,
        Pathogen = 314,
        Where = 315,
        Infect = 316,
        Cleanse = 317,
        Ct = 318,
        Newline = 319,
        Indent = 320,
        Unindent = 321,
        LOW = 322,
        HIGH = 323
      };
    };

    /// (External) token type, as returned by yylex.
    typedef token::yytokentype token_type;

    /// Symbol type: an internal symbol number.
    typedef int symbol_number_type;

    /// The symbol type number to denote an empty symbol.
    enum { empty_symbol = -2 };

    /// Internal symbol number for tokens (subsumed by symbol_number_type).
    typedef unsigned char token_number_type;

    /// A complete symbol.
    ///
    /// Expects its Base type to provide access to the symbol type
    /// via type_get().
    ///
    /// Provide access to semantic value.
    template <typename Base>
    struct basic_symbol : Base
    {
      /// Alias to Base.
      typedef Base super_type;

      /// Default constructor.
      basic_symbol ();

      /// Copy constructor.
      basic_symbol (const basic_symbol& other);

      /// Constructor for valueless symbols.
      basic_symbol (typename Base::kind_type t);

      /// Constructor for symbols with semantic value.
      basic_symbol (typename Base::kind_type t,
                    const semantic_type& v);

      /// Destroy the symbol.
      ~basic_symbol ();

      /// Destroy contents, and record that is empty.
      void clear ();

      /// Whether empty.
      bool empty () const;

      /// Destructive move, \a s is emptied into this.
      void move (basic_symbol& s);

      /// The semantic value.
      semantic_type value;

    private:
      /// Assignment operator.
      basic_symbol& operator= (const basic_symbol& other);
    };

    /// Type access provider for token (enum) based symbols.
    struct by_type
    {
      /// Default constructor.
      by_type ();

      /// Copy constructor.
      by_type (const by_type& other);

      /// The symbol type as needed by the constructor.
      typedef token_type kind_type;

      /// Constructor from (external) token numbers.
      by_type (kind_type t);

      /// Record that this symbol is empty.
      void clear ();

      /// Steal the symbol type from \a that.
      void move (by_type& that);

      /// The (internal) type number (corresponding to \a type).
      /// \a empty when empty.
      symbol_number_type type_get () const;

      /// The token.
      token_type token () const;

      /// The symbol type.
      /// \a empty_symbol when empty.
      /// An int, not token_number_type, to be able to store empty_symbol.
      int type;
    };

    /// "External" symbols: returned by the scanner.
    typedef basic_symbol<by_type> symbol_type;



    /// Build a parser object.
    parser ();
    virtual ~parser ();

    /// Parse.
    /// \returns  0 iff parsing succeeded.
    virtual int parse ();

    /// The current debugging stream.
    std::ostream& debug_stream () const;
    /// Set the current debugging stream.
    void set_debug_stream (std::ostream &);

    /// Type for debugging levels.
    typedef int debug_level_type;
    /// The current debugging level.
    debug_level_type debug_level () const;
    /// Set the current debugging level.
    void set_debug_level (debug_level_type l);

  public:
    /// Report a syntax error.
    /// \param msg    a description of the syntax error.
    virtual void error (const std::string& msg);

# if YYDEBUG
  public:
    /// \brief Report a symbol value on the debug stream.
    /// \param yytype       The token type.
    /// \param yyvaluep     Its semantic value.
    virtual void yy_symbol_value_print_ (int yytype,
                                         const semantic_type* yyvaluep);
    /// \brief Report a symbol on the debug stream.
    /// \param yytype       The token type.
    /// \param yyvaluep     Its semantic value.
    virtual void yy_symbol_print_ (int yytype,
                                   const semantic_type* yyvaluep);
  private:
    // Debugging.
    std::ostream* yycdebug_;
#endif


  };



#ifndef YYSTYPE
# define YYSTYPE yy::parser::semantic_type
#endif
#ifndef YYLTYPE
# define YYLTYPE yy::parser::location_type
#endif


} // yy
#line 298 "include/yyparser.h" // glr.cc:329


#endif // !YY_YY_INCLUDE_YYPARSER_H_INCLUDED
