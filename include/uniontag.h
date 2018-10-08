#ifndef AN_UNIONTAG_H
#define AN_UNIONTAG_H

#include "antype.h"

namespace ante {
    /**
    * @brief An individual tag of a tagged union along with the types it corresponds to
    */
    struct UnionTag {
        std::string name;
        AnDataType *ty;
        AnDataType *parent;
        unsigned short tag;

        UnionTag(std::string &n, AnDataType *tyn, AnDataType *p, unsigned short t) :
            name(n), ty(tyn), parent(p), tag(t){}
    };
}


#endif /* end of include guard: AN_UNIONTAG_H */
