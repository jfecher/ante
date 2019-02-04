#ifndef AN_UNIFICATION_H
#define AN_UNIFICATION_H

#include "antype.h"
#include <tuple>
#include <variant>

namespace ante {
    using Substitutions = std::list<std::pair<AnType*, AnType*>>;

    class UnificationConstraint {
        using EqConstraint = std::pair<AnType*, AnType*>;
        using TypeClassConstraint = AnTraitType*;

        std::variant<EqConstraint, TypeClassConstraint> constraint;

        public:
            const LOC_TY &loc;

            /** Eq constructor, enforce a = b */
            UnificationConstraint(AnType *a, AnType *b, LOC_TY const& loc)
                : constraint{EqConstraint{a, b}}, loc{loc}{}

            /** Typeclass constructor, enforce impl typeclass args exists */
            UnificationConstraint(AnTraitType *typeclass, LOC_TY const& loc)
                : constraint{TypeClassConstraint{typeclass}}, loc{loc}{}

            bool isEqConstraint() const noexcept {
                return std::holds_alternative<EqConstraint>(constraint);
            }

            EqConstraint asEqConstraint() const {
                return std::get<EqConstraint>(constraint);
            }

            TypeClassConstraint asTypeClassConstraint() const {
                return std::get<TypeClassConstraint>(constraint);
            }
    };


    using UnificationList = std::list<UnificationConstraint>;

    /** Substitute all instances of a given type subType in t with u.
     * Returns a new substituted type or t if subType was not contained within */
    AnType* substitute(AnType *u, AnType *subType, AnType *t);

    Substitutions unify(UnificationList const& list);

    AnType* applySubstitutions(Substitutions const& substitutions, AnType *t);

    AnTypeVarType* nextTypeVar();

    AnType* copyWithNewTypeVars(AnType *t, std::unordered_map<std::string, AnTypeVarType*> &map);

    template<typename T>
    std::vector<T*> copyWithNewTypeVars(std::vector<T*> tys, std::unordered_map<std::string, AnTypeVarType*> &map);

    AnType* copyWithNewTypeVars(AnType *t);

    AnFunctionType* removeDuplicateTypeClassConstraints(AnFunctionType *t);
    void checkTypeClassImplExists(AnFunctionType *ft);
}

#endif /* end of include guard: AN_UNIFICATION_H */
