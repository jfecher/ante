#ifndef AN_TYPEERROR_H
#define AN_TYPEERROR_H

#include "lazystr.h"
#include "parser.h"  // for LOC_TY

namespace ante {
    class AnType;

    /**
    * A wrapper for lazy_printer that holds an error message where
    * types that are not yet known can be substituted at a later stage
    * (usually unification).
    * 
    * The decode method, and this type by extension, supports the
    * substitution of up to two types via string replacement for the
    * substrings $1 and $2.
    */
    class TypeError {
        ante::lazy_printer encoded_msg;

        /**
        * Substitute each type for $1 and $2 respectively
        * and return the resulting lazy_printer error message
        */
        ante::lazy_printer decode(const AnType *a, const AnType *b) const;

    public:
        LOC_TY const& loc;

        TypeError(ante::lazy_printer const& msg, LOC_TY const& loc) : encoded_msg{msg}, loc{loc}{};
        TypeError(ante::lazy_str const& msg, LOC_TY const& loc) : encoded_msg{msg}, loc{loc}{};
        TypeError(std::string const& msg, LOC_TY const& loc) : encoded_msg{msg}, loc{loc}{};
        TypeError(const char* msg, LOC_TY const& loc) : encoded_msg{msg}, loc{loc}{};
        TypeError(LOC_TY const& loc) : encoded_msg{}, loc{loc}{}

        /**
         * Show the contained error via ante::showError with the given mismatched types.
         * NOTE: All errors issued will be ErrorType::Error.
         *       Warnings/Notes will need to be issued manually.
         */
        void show(const AnType *a, const AnType *b) const;
    };

    /**
    * Increment a string, say 'a to 'b then 'c ... 'z 'aa 'ab and so on
    */
    std::string nextLetter(std::string cur);

    /**
    * Replace numbered typevars, eg '1382 with proper names starting with 'a.
    * Keep track of already-seen typevars with map.
    */
    AnType* sanitize(AnType *t);

    /**
    * Convert two AnType* to two string-like object usable in error messages.
    * The types are compared as they are converted and have their differences
    * highlighted in AN_ERR_COLOR, and everything else in AN_TYPE_COLOR.
    */
    std::pair<lazy_printer, lazy_printer>
    anTypesToErrorStrs(const AnType *t1, const AnType *t2);
}

#endif
