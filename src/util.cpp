#include "util.h"
#include "compiler.h"

namespace ante {
    void print(parser::Node *n){
        PrintingVisitor::print(n);
        puts("");
    }

    void print(std::shared_ptr<parser::Node> const& n){
        print(n.get());
    }

    void print(std::unique_ptr<parser::Node> const& n){
        print(n.get());
    }

    std::ostream& operator<<(std::ostream &out, parser::Node &n){
        out << std::flush;
        PrintingVisitor::print(&n);
        out << std::flush;
        return out;
    }
}
