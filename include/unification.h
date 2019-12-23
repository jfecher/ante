#ifndef AN_UNIFICATION_H
#define AN_UNIFICATION_H

#include "antype.h"
#include "typeerror.h"
#include <tuple>

namespace ante {
    using Substitutions = std::list<std::pair<AnType*, AnType*>>;

    class UnificationConstraint {
        using EqConstraint = std::pair<AnType*, AnType*>;
        using TypeClassConstraint = TraitImpl*;

        union U {
            EqConstraint eqConstraint;
            TypeClassConstraint typeClassConstraint;

            U(AnType *a, AnType *b) : eqConstraint{a, b}{}
            U(TraitImpl *tc) : typeClassConstraint{tc}{}
        } u;

        bool eqConstraint;

        public:
            TypeError error;

            /** Eq constructor, enforce a = b */
            UnificationConstraint(AnType *a, AnType *b, TypeError const& err)
                : u{a, b}, eqConstraint{true}, error{err}{}

            /** Typeclass constructor, enforce impl typeclass args exists */
            UnificationConstraint(TraitImpl *typeclass, TypeError const& err)
                : u{typeclass}, eqConstraint{false}, error{err}{}

            bool isEqConstraint() const noexcept {
                return eqConstraint;
            }

            EqConstraint asEqConstraint() const {
                return u.eqConstraint;
            }

            TypeClassConstraint asTypeClassConstraint() const {
                return u.typeClassConstraint;
            }
    };


    using UnificationList = std::list<UnificationConstraint>;

    /** Substitute all instances of a given type subType in t with u.
     * Returns a new substituted type or t if subType was not contained within */
    AnType* substitute(AnType *u, AnType *subType, AnType *t, int recursionLimit = 10000);

    Substitutions unify(UnificationList const& list);
    Substitutions unifyOne(AnType *t1, AnType *t2, TypeError const& errMsg);

    AnType* applySubstitutions(Substitutions const& substitutions, AnType *t);
    TraitImpl* applySubstitutions(Substitutions const& substitutions, TraitImpl *t);

    AnTypeVarType* nextTypeVar();

    bool hasTypeVarNotInMap(const AnType *t, llvm::StringMap<const AnTypeVarType*> &map);

    AnType* copyWithNewTypeVars(AnType *t, std::unordered_map<std::string, AnTypeVarType*> &map);

    llvm::StringMap<const AnTypeVarType*> getAllContainedTypeVars(const AnType *t);

    void getAllContainedTypeVarsHelper(const AnType *t, llvm::StringMap<const AnTypeVarType*> &map);

    template<typename T>
    std::vector<T*> copyWithNewTypeVars(std::vector<T*> tys, std::unordered_map<std::string, AnTypeVarType*> &map);

    AnType* copyWithNewTypeVars(AnType *t);

    /** Remove any duplicate type class constraints and any constraints that are known to exist. */
    AnFunctionType* cleanTypeClassConstraints(AnFunctionType *t);
}

#endif /* end of include guard: AN_UNIFICATION_H */
