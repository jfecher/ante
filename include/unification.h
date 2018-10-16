#ifndef AN_UNIFICATION_H
#define AN_UNIFICATION_H

#include "antype.h"
#include <tuple>

namespace ante {
    using Substitutions = std::list<std::pair<std::string, AnType*>>;

    /** Substitute all instances of AnTypeVar(name) in t with u.
     * Returns a new substituted type or t if name was not contained within */
    AnType* substitute(AnType *u, std::string const& name, AnType *t);

    Substitutions unify(std::list<std::tuple<AnType*, AnType*, LOC_TY&>>& list);

    AnType* applySubstitutions(Substitutions const& substitutions, AnType *t);

    AnTypeVarType* nextTypeVar();

    AnType* copyWithNewTypeVars(AnType *t, std::unordered_map<std::string, AnTypeVarType*> &map);

    std::vector<AnType*> copyWithNewTypeVars(std::vector<AnType*> tys, std::unordered_map<std::string, AnTypeVarType*> &map);

    AnType* copyWithNewTypeVars(AnType *t);
}

#endif /* end of include guard: AN_UNIFICATION_H */
