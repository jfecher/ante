#ifndef AN_UNIFICATION_H
#define AN_UNIFICATION_H

#include "antype.h"
#include <tuple>

namespace ante {
    using Substitutions = std::list<std::pair<AnType*, AnType*>>;

    using UnificationList = std::list<std::tuple<AnType*, AnType*, LOC_TY&>>;

    /** Substitute all instances of a given type subType in t with u.
     * Returns a new substituted type or t if subType was not contained within */
    AnType* substitute(AnType *u, AnType *subType, AnType *t);

    Substitutions unify(UnificationList const& list);

    AnType* applySubstitutions(Substitutions const& substitutions, AnType *t);

    AnTypeVarType* nextTypeVar();

    AnType* copyWithNewTypeVars(AnType *t, std::unordered_map<std::string, AnTypeVarType*> &map);

    std::vector<AnType*> copyWithNewTypeVars(std::vector<AnType*> tys, std::unordered_map<std::string, AnTypeVarType*> &map);

    AnType* copyWithNewTypeVars(AnType *t);
}

#endif /* end of include guard: AN_UNIFICATION_H */
