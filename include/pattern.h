#ifndef AN_PATTERN_H
#define AN_PATTERN_H

#include "parser.h"
#include "compiler.h"

namespace ante {
    void handlePattern(CompilingVisitor &cv, parser::MatchNode *n, parser::Node *pattern,
            llvm::BasicBlock *jmpOnFail, TypedValue valToMatch);


    class Pattern {
        /**
        * The type of pattern currently held.
        *
        * Can be any literal type, TT_TypeVar, TT_Tuple, or TT_Data.
        * Both tuples and product types are interpreted as TT_Tuple,
        * with sum types tagged as TT_Data. TT_TypeVar is treated
        * as a stand in for any pattern.
        */
        TypeTag type;

        /**
         * True if the current pattern/group of patterns can match this case.
         *
         * If every matched is true, the match expression is exhaustive.
         */
        bool matched;

        /**
         * The name of this union variant if it has one, "" otherwise.
         */
        std::string name;

        /**
         * The child patterns of the current pattern.
         * This contains each sub-pattern of a tuple, or each variant of a union.
         */
        std::vector<Pattern> children;

    public:
        Pattern(TypeTag type) : type{type}, matched{false}{}

        /**
         * Attempt to overwrite this pattern with the other pattern.
         * This will raise an error if the current pattern is not a TT_TypeVar
         * and the two patterns do not match.
         */
        void overwrite(Pattern const& other, LOC_TY &loc);

        /** set matched to true */
        void setMatched();

        /** Can this pattern always be matched? */
        bool irrefutable() const;

        /** Get the child at the given index */
        Pattern& getChild(size_t idx);

        size_t numChildren() const { return children.size(); }

        /** Return a string representation of an unrepresented case */
        lazy_printer constructMissedCase() const;

        static Pattern getFillerPattern();

        static Pattern fromType(const AnType *t);

        static Pattern fromSumType(const AnSumType *t);

        static Pattern fromTuple(std::vector<AnType*> const& types);
    };
}

#endif
