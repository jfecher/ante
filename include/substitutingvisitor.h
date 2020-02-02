#ifndef AN_SUBSTITUTINGVISITOR_H
#define AN_SUBSTITUTINGVISITOR_H

#include "parser.h"
#include "module.h"
#include "unification.h"
#include <vector>

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
        std::vector<llvm::StringMap<const AnTypeVarType*>> typevarsInScope;

        void checkTypeClassConstraints(AnFunctionType *t, LOC_TY &loc);
        bool delayTraitCheck(TraitImpl *impl) const;
        bool inScope(llvm::StringRef typevar) const;
    };
}


#endif /* end of include guard: AN_SUBSTITUTINGVISITOR_H */
