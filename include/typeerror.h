#ifndef AN_TYPEERROR_H
#define AN_TYPEERROR_H

#include "lazystr.h"
#include "antype.h"

namespace ante {
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

    public:
        TypeError(ante::lazy_printer const& msg) : encoded_msg{msg}{};
        TypeError(ante::lazy_str const& msg) : encoded_msg{msg}{};
        TypeError(std::string const& msg) : encoded_msg{msg}{};
        TypeError(const char* msg) : encoded_msg{msg}{};
        TypeError() = default;

        /**
        * Substitute each type for $1 and $2 respectively
        * and return the resulting lazy_printer error message
        */
        ante::lazy_printer decode(const AnType *a, const AnType *b) const;
    };
}

#endif
