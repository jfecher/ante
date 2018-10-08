#ifndef AN_TYPECHECKRESULT_H
#define AN_TYPECHECKRESULT_H

#include <memory>
#include <vector>
#include "typebinding.h"

namespace ante {
    /**
    * @brief The result of a type check
    *
    * Can be one of three states: Failure, Success,
    * or SuccessWithTypeVars.
    *
    * SuccessWithTypeVars indicates the typecheck is only a
    * success if a typevar within is bound to a particular type.
    * For example the check of 't* and i32* would return this status.
    * Whenever SuccessWithTypeVars is set, the bindings field contains
    * the specific bindings that should be bound to the typevar term.
    */
    struct TypeCheckResult {
        enum Result { Failure, Success, SuccessWithTypeVars };

        //box internals for faster passing by value and easier ownership transfer
        struct Internals {
            Result res;
            unsigned int matches;

            /* typevar mappings, eg. 't to i32 */
            std::vector<TypeBinding> bindings;

            Internals() : res{Success}, matches{0}, bindings{}{}
        };

        std::shared_ptr<Internals> box;

        TypeCheckResult& successIf(bool b);
        TypeCheckResult& successIf(Result r);
        TypeCheckResult& success();
        TypeCheckResult& success(size_t matches);
        TypeCheckResult& successWithTypeVars();
        TypeCheckResult& failure();

        bool failed();

        bool operator!() const { return box->res == Failure; }
        explicit operator bool() const { return box->res == Success || box->res == SuccessWithTypeVars; }
        Internals* operator->() const { return box.get(); }

        TypeCheckResult() : box(new Internals()){}
        TypeCheckResult(const TypeCheckResult &r)  : box(r.box){}
        //TypeCheckResult(TypeCheckResult &&r)  : box(move(r.box)){}
    };
}

#endif /* end of include guard: AN_TYPECHECKRESULT_H */
