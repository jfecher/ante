#ifndef AN_SUBSTITUTINGVISITOR_H
#define AN_SUBSTITUTINGVISITOR_H

#include "parser.h"
#include "module.h"
#include "unification.h"

namespace ante {
    
    struct SubstitutingVisitor : public NodeVisitor {
        DECLARE_NODE_VISIT_METHODS();

        SubstitutingVisitor(Substitutions const& s, Module *module)
            : substitutions{s}, module{module}{}

        static void substituteIntoAst(parser::Node *ast, Substitutions const& subs, Module *module){
            SubstitutingVisitor v{subs, module};
            ast->accept(v);
        }

        private:
        Substitutions const& substitutions;
        Module *module;
    };
}


#endif /* end of include guard: AN_SUBSTITUTINGVISITOR_H */
