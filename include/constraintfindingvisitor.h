#ifndef AN_CONSTRAINTFINDINGVISITOR_H
#define AN_CONSTRAINTFINDINGVISITOR_H

#include "parser.h"
#include "antype.h"

namespace ante {
    
    struct ConstraintFindingVisitor : public NodeVisitor {
        ConstraintFindingVisitor(){}

        DECLARE_NODE_VISIT_METHODS();

        std::list<std::pair<AnType*, AnType*>> getConstraints() const;

        private:
            std::list<std::pair<AnType*, AnType*>> constraints;
    };
}


#endif /* end of include guard: AN_CONSTRAINTFINDINGVISITOR_H */
