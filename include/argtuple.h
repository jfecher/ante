#ifndef AN_ARGTUPLE_H
#define AN_ARGTUPLE_H

#include <unordered_set>
#include <vector>
#include <nodevisitor.h>
#include <llvm/ADT/StringMap.h>
#include "typedvalue.h"
#include "error.h"
#include "parser.h"

namespace ante {
    struct Compiler;

    /** A data structure for translating between runtime
     * values and TypedValues while jitting. */
    class ArgTuple {

        public:
            /** Converts a tuple pointer into a TypedValue */
            TypedValue asTypedValue(Compiler *c) const;

            /** Returns the contained data without any conversions. */
            void* asRawData() const { return data; }

            /** Return the AnType* of the contained value. */
            AnType *getType() const { return type; }

            template<typename T>
            T castTo() const {
                if(!data)
                    throw new CompilationError("Uninitialized data in cast");
                return *(T*)data;
            }

            /** Dump the contained value to the given stream. */
            void print(Compiler *c, std::ostream &os = std::cout) const;

            /**
             * Constructs an ArgTuple from the given TypedValue arguments.
             * - Can throw if the given expressions aren't able to be evaluated
             *   during compile-time (eg. are mut, loop bindings, or a parameter).
             */
            ArgTuple(Compiler *c, std::vector<TypedValue> const& val,
                    std::vector<std::unique_ptr<parser::Node>> const& exprs);

            /**
             * Constructs an ArgTuple of a single value from the given argument.
             * - Can throw if the given expressions aren't able to be evaluated
             *   during compile-time (eg. are mut, loop bindings, or a parameter).
             */
            ArgTuple(Compiler *c, TypedValue const& val, std::unique_ptr<parser::Node> const& expr);

            ArgTuple(Compiler *c, TypedValue const& val, parser::Node *expr);

            /** Construct an ArgTuple using the given pre-initialized data. */
            ArgTuple(void *d, AnType *t) : data(d), type(t){}

            /** Constructs an empty ArgTuple representing a void literal. */
            ArgTuple() = default;


        private:
            /** Pointer to a tuple of the given data types */
            void *data;

            /** The type and value of this data. */
            AnType *type;

            /** Stores pointer value of a constant pointer type */
            void storePtr(Compiler *c, TypedValue const& tv);

            /** Allocates space then calls storeValue */
            void allocAndStoreValue(Compiler *c, TypedValue const& tv);

            /** Stores a tuple value in data */
            void storeTuple(Compiler *c, TypedValue const& tup);

            /** Stores a primitive integer, character, or boolean. */
            void storeInt(Compiler *c, TypedValue const& tv);

            /** Stores a primitive floating-point value. */
            void storeFloat(Compiler *c, TypedValue const& tv);

            /**
             * Converts the given TypedValue into its corresponding
             * value in c++ and stores it in this->data
             */
            void storeValue(Compiler *c, TypedValue const& tv);

            /** Implementation of print function. */
            void printCtVal(Compiler *c, std::ostream &os) const;

            /** Implementation of print function. */
            void printTupleOrData(Compiler *c, std::ostream &os) const;

            /** Implementation of print function. */
            void printUnion(Compiler *c, std::ostream &os) const;

    }__attribute__((__packed__));

    /**
     * Ensures the contents of an expression is able to be evaluated
     * during compile-time, throwing an exception if it is not.
     *
     * An expression is not able to be evaluated during compile-time iff:
     * - It uses a previously-declared mut variable, that is not ante mut
     * - It uses a previously-declared function parameter not marked ante
     * - It uses a previously-declared loop binding not marked ante
     */
    struct AnteVisitor : public NodeVisitor {
        Compiler *c;

        /** Pseudo var table used for tracking which variables are declared inside and
         * which are declared outisde the given expression. */
        std::vector<std::unordered_set<std::string>> varTable;

        /** External bindings (the minimal environment) the expression needs to run */
        std::vector<std::pair<std::string, parser::Node*>> dependencies;

        /** True if we are inside the ante expr and not backtracing a dependency */
        bool inAnteExpr;

        /** True if all identifiers should be implicitly declared (ie. in a match pattern) */
        bool implicitDeclare;

        DECLARE_NODE_VISIT_METHODS();

        AnteVisitor(Compiler *cc) : c{cc}, inAnteExpr{true}, implicitDeclare{false}{
            varTable.emplace_back();
        }
        AnteVisitor() = delete;

        /** Throw an error if an expr is not a well-typed ante expression */
        static void validate(Compiler *c, parser::Node *n){
            AnteVisitor v{c};
            n->accept(v);
        }

        bool isDeclaredInternally(std::string const& var) const;

        /** Visit a declaration external to the ante expression. */
        void visitExternalDecl(std::string const& name, parser::Node *decl);

        void declare(std::string const& var);

        void newScope();

        void endScope();
    };
}

#endif
