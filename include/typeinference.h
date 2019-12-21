#ifndef AN_TYPEINFERENCEVISITOR_H
#define AN_TYPEINFERENCEVISITOR_H

#include <chrono>

#include "parser.h"
#include "antype.h"
#include "constraintfindingvisitor.h"
#include "unification.h"
#include "substitutingvisitor.h"
#include "util.h"

namespace ante {
    AnType* applySubstitutions(Substitutions const& substitutions, AnType *t);

    /**
     * Perform type inference on a parse tree.
     * This consists of several steps:
     * 1. Annotate tree with placeholder types  (TypeInferenceVisitor)
     * 2. Recurse again to find a list of constraints  (ConstrintFindingVisitor)
     * 3. Perform unification
     * 4. Substitute any yet-unresolved types  (SubstitutingVisitor)
     */
    struct TypeInferenceVisitor : public NodeVisitor {
        Module *module;

        TypeInferenceVisitor(Module *module) : module{module}{}

        /** Infer types of all expressions in parse tree and
        * mutate the ast with the inferred types. */
        static void infer(parser::Node *n, Module *module){
            using namespace std::chrono;
            auto tistart = high_resolution_clock::now();
            TypeInferenceVisitor step1{module};
            n->accept(step1);

            auto cfstart = high_resolution_clock::now();
            ConstraintFindingVisitor step2{module};
            n->accept(step2);

            auto unifystart = high_resolution_clock::now();
            auto constraints = step2.getConstraints();
            auto substitutions = unify(constraints);

            auto substart = high_resolution_clock::now();
            SubstitutingVisitor::substituteIntoAst(n, substitutions, module);
            auto end = high_resolution_clock::now();

            std::cout << "Module: " << module->name << '\n';
            std::cout << "Type inference: " << duration_cast<milliseconds>(end - tistart).count() << "ms\n";
            std::cout << "    Initialization:     " << duration_cast<milliseconds>(cfstart - tistart).count() << "ms\n";
            std::cout << "    Constraint finding: " << duration_cast<milliseconds>(unifystart - cfstart).count() << "ms\n";
            std::cout << "    Unification:        " << duration_cast<milliseconds>(substart - unifystart).count() << "ms\n";
            std::cout << "    Substitution:       " << duration_cast<milliseconds>(end - substart).count() << "ms\n";
        }


        static void infer(std::unique_ptr<parser::Node> &n, Module *module){
            return infer(n.get(), module);
        }

        static void infer(std::shared_ptr<parser::Node> &n, Module *module){
            return infer(n.get(), module);
        }

        DECLARE_NODE_VISIT_METHODS();
    };
}

#endif
