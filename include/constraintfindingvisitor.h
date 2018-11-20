#ifndef AN_CONSTRAINTFINDINGVISITOR_H
#define AN_CONSTRAINTFINDINGVISITOR_H

#include "parser.h"
#include "antype.h"
#include "unification.h"
#include <tuple>

namespace ante {
    
    struct ConstraintFindingVisitor : public NodeVisitor {
        ConstraintFindingVisitor(){}

        DECLARE_NODE_VISIT_METHODS();

        UnificationList getConstraints() const;

        private:
            UnificationList constraints;

            void addConstraint(AnType *a, AnType *b, LOC_TY &loc);

            /** Searches type for typeclasses, removes them from the type, and adds them
             * as a separate constraint.  Returns a new type with type classes removed. */
            AnType* handleTypeClassConstraints(AnType *t, LOC_TY const& loc);
    };
}


#endif /* end of include guard: AN_CONSTRAINTFINDINGVISITOR_H */
