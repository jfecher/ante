#ifndef AN_NODEVISITOR_H
#define AN_NODEVISITOR_H

namespace ante {
    namespace parser {
        struct RootNode;
        struct IntLitNode;
        struct FltLitNode;
        struct BoolLitNode;
        struct CharLitNode;
        struct ArrayNode;
        struct TupleNode;
        struct UnOpNode;
        struct BinOpNode;
        struct SeqNode;
        struct BlockNode;
        struct ModNode;
        struct TypeNode;
        struct TypeCastNode;
        struct RetNode;
        struct NamedValNode;
        struct VarNode;
        struct GlobalNode;
        struct StrLitNode;
        struct VarAssignNode;
        struct ExtNode;
        struct ImportNode;
        struct JumpNode;
        struct WhileNode;
        struct ForNode;
        struct MatchBranchNode;
        struct MatchNode;
        struct IfNode;
        struct FuncDeclNode;
        struct DataDeclNode;
        struct TraitNode;
    }

    struct Compiler;

    struct NodeVisitor {
        virtual void visit(parser::RootNode*) = 0;
        virtual void visit(parser::IntLitNode*) = 0;
        virtual void visit(parser::FltLitNode*) = 0;
        virtual void visit(parser::BoolLitNode*) = 0;
        virtual void visit(parser::CharLitNode*) = 0;
        virtual void visit(parser::ArrayNode*) = 0;
        virtual void visit(parser::TupleNode*) = 0;
        virtual void visit(parser::UnOpNode*) = 0;
        virtual void visit(parser::BinOpNode*) = 0;
        virtual void visit(parser::SeqNode*) = 0;
        virtual void visit(parser::BlockNode*) = 0;
        virtual void visit(parser::ModNode*) = 0;
        virtual void visit(parser::TypeNode*) = 0;
        virtual void visit(parser::TypeCastNode*) = 0;
        virtual void visit(parser::RetNode*) = 0;
        virtual void visit(parser::NamedValNode*) = 0;
        virtual void visit(parser::VarNode*) = 0;
        virtual void visit(parser::GlobalNode*) = 0;
        virtual void visit(parser::StrLitNode*) = 0;
        virtual void visit(parser::VarAssignNode*) = 0;
        virtual void visit(parser::ExtNode*) = 0;
        virtual void visit(parser::ImportNode*) = 0;
        virtual void visit(parser::JumpNode*) = 0;
        virtual void visit(parser::WhileNode*) = 0;
        virtual void visit(parser::ForNode*) = 0;
        virtual void visit(parser::MatchBranchNode*) = 0;
        virtual void visit(parser::MatchNode*) = 0;
        virtual void visit(parser::IfNode*) = 0;
        virtual void visit(parser::FuncDeclNode*) = 0;
        virtual void visit(parser::DataDeclNode*) = 0;
        virtual void visit(parser::TraitNode*) = 0;
    };


#define DECLARE_NODE_VISIT_METHODS()       \
    void visit(parser::RootNode*);         \
    void visit(parser::IntLitNode*);       \
    void visit(parser::FltLitNode*);       \
    void visit(parser::BoolLitNode*);      \
    void visit(parser::CharLitNode*);      \
    void visit(parser::ArrayNode*);        \
    void visit(parser::TupleNode*);        \
    void visit(parser::UnOpNode*);         \
    void visit(parser::BinOpNode*);        \
    void visit(parser::SeqNode*);          \
    void visit(parser::BlockNode*);        \
    void visit(parser::ModNode*);          \
    void visit(parser::TypeNode*);         \
    void visit(parser::TypeCastNode*);     \
    void visit(parser::RetNode*);          \
    void visit(parser::NamedValNode*);     \
    void visit(parser::VarNode*);          \
    void visit(parser::GlobalNode*);       \
    void visit(parser::StrLitNode*);       \
    void visit(parser::VarAssignNode*);    \
    void visit(parser::ExtNode*);          \
    void visit(parser::ImportNode*);       \
    void visit(parser::JumpNode*);         \
    void visit(parser::WhileNode*);        \
    void visit(parser::ForNode*);          \
    void visit(parser::MatchBranchNode*);  \
    void visit(parser::MatchNode*);        \
    void visit(parser::IfNode*);           \
    void visit(parser::FuncDeclNode*);     \
    void visit(parser::DataDeclNode*);     \
    void visit(parser::TraitNode*)
}

#endif
