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
    };
}


#endif /* end of include guard: AN_CONSTRAINTFINDINGVISITOR_H */
