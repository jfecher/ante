#ifndef AN_SUBSTITUTINGVISITOR_H
#define AN_SUBSTITUTINGVISITOR_H

#include "parser.h"
#include "unification.h"

namespace ante {
    
    struct SubstitutingVisitor : public NodeVisitor {
        DECLARE_NODE_VISIT_METHODS();

        SubstitutingVisitor(Substitutions const& s)
            : substitutions{s}{}

        static void substituteIntoAst(parser::Node *ast, Substitutions const& subs){
            SubstitutingVisitor v{subs};
            ast->accept(v);
        }

        private:
        Substitutions const& substitutions;
    };
}


#endif /* end of include guard: AN_SUBSTITUTINGVISITOR_H */
