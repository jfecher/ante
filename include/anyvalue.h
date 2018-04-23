#ifndef AN_ANYVALUE_H
#define AN_ANYVALUE_H

#include "antype.h"

namespace ante {

    struct AnyValue {
        void *val;
        AnType *type;

        explicit operator bool() const {
            return type;
        }
        
        AnyValue() : val(nullptr), type(nullptr){}

        template<typename T>
        AnyValue(T const& v, AnType *ty) : type(ty){
            val = new T(move(v));
        }

        template<typename T>
        AnyValue(T &v, AnType *ty) : type(ty){
            val = new T(v);
        }

        template<typename T>
        AnyValue(T &&v, AnType *ty) : type(ty){
            val = new T(v);
        }
    };
}

#endif
