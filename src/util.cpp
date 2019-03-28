#include "util.h"
#include "compiler.h"

namespace ante {
    void show(parser::Node *n){
        PrintingVisitor::print(n);
        puts("");
    }

    void show(std::shared_ptr<parser::Node> const& n){
        show(n.get());
    }

    void show(std::unique_ptr<parser::Node> const& n){
        show(n.get());
    }

    std::ostream& operator<<(std::ostream &out, parser::Node &n){
        out << std::flush;
        PrintingVisitor::print(&n);
        out << std::flush;
        return out;
    }

    std::ostream& operator<<(std::ostream &out, AnType &n){
        return out << anTypeToColoredStr(&n);
    }
}
