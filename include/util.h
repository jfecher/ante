#ifndef AN_UTIL_H
#define AN_UTIL_H

#include "error.h"

namespace ante {
    template<typename F>
    void tryTo(F f){
        try{
            f();
        }catch(CtError){}
    }
}

#endif
