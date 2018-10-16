#ifndef AN_CONSTRAINTFINDINGVISITOR_H
#define AN_CONSTRAINTFINDINGVISITOR_H

#include "parser.h"
#include "antype.h"
#include <tuple>

namespace ante {
    
    struct ConstraintFindingVisitor : public NodeVisitor {
        ConstraintFindingVisitor(){}

        DECLARE_NODE_VISIT_METHODS();

        std::list<std::tuple<AnType*, AnType*, LOC_TY&>> getConstraints() const;

        private:
            std::list<std::tuple<AnType*, AnType*, LOC_TY&>> constraints;
    };
}


#endif /* end of include guard: AN_CONSTRAINTFINDINGVISITOR_H */
