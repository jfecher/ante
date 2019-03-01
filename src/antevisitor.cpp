#include "antevalue.h"
#include "compiler.h"
#include "parser.h"

namespace ante {
    
    bool AnteVisitor::isDeclaredInternally(std::string const& var) const {
        for(auto &scope : varTable)
            if(scope.find(var) != scope.end())
                return true;
        return false;
    }

    void AnteVisitor::visitExternalDecl(std::string const& name, AnType* type, parser::Node *decl){
        inAnteExpr = false;
        dependencies.emplace_back(name, type, decl);
        decl->accept(*this);
        inAnteExpr = true;
    }

    void AnteVisitor::declare(std::string const& var){
        if(inAnteExpr)
            varTable.back().insert(var);
    }

    void AnteVisitor::newScope(){
        varTable.emplace_back();
    }

    void AnteVisitor::endScope(){
        varTable.pop_back();
    }

    void AnteVisitor::visit(parser::RootNode *n){
        for(auto &m : n->extensions)
            m->accept(*this);
        for(auto &m : n->funcs)
            m->accept(*this);
        for(auto &m : n->main){
            m->accept(*this);
        }
    }

    void AnteVisitor::visit(parser::IntLitNode *n){
        //nothing to do
    }

    void AnteVisitor::visit(parser::FltLitNode *n){
        //nothing to do
    }

    void AnteVisitor::visit(parser::BoolLitNode *n){
        //nothing to do
    }

    void AnteVisitor::visit(parser::CharLitNode *n){
        //nothing to do
    }

    void AnteVisitor::visit(parser::ArrayNode *n){
        for(auto &m : n->exprs)
            m->accept(*this);
    }

    void AnteVisitor::visit(parser::TupleNode *n){
        for(auto &m : n->exprs)
            m->accept(*this);
    }

    void AnteVisitor::visit(parser::UnOpNode *n){
        n->rval->accept(*this);
    }

    void AnteVisitor::visit(parser::BinOpNode *n){
        n->lval->accept(*this);
        if(n->op != '.')
            n->rval->accept(*this);
    }

    void AnteVisitor::visit(parser::SeqNode *n){
        for(auto &m : n->sequence)
            m->accept(*this);
    }

    void AnteVisitor::visit(parser::BlockNode *n){
        newScope();
        n->block->accept(*this);
        endScope();
    }

    void AnteVisitor::visit(parser::ModNode *n){
        if(n->expr)
            n->expr->accept(*this);
    }

    void AnteVisitor::visit(parser::TypeNode *n){
        //nothing to do
    }

    void AnteVisitor::visit(parser::TypeCastNode *n){
        n->rval->accept(*this);
    }

    void AnteVisitor::visit(parser::RetNode *n){
        n->expr->accept(*this);
    }

    void AnteVisitor::visit(parser::NamedValNode *n){
        //nothing to do
    }

    void AnteVisitor::visit(parser::VarNode *n){
        if(implicitDeclare){
            declare(n->name);
            return;
        }

        if(!isDeclaredInternally(n->name)){
            auto *v = c->lookup(n->name);

            if(!v){
                auto& fnlist = c->getFunctionList(n->name);
                if(!fnlist.empty())
                    return;

                if(fnlist.empty())
                    c->compErr("Use of undeclared variable " + n->name + " in ante expression", n->loc);
            }

            switch(v->assignments.back().assignmentType){
                case Assignment::ForLoop:
                    if(!v->tval.type->hasModifier(Tok_Ante)){
                        c->compErr("Cannot evaluate a non-ante for-loop binding during compile-time.  Prefix the for loop with 'ante' to evaluate it in compile-time",
                                n->loc);
                    }
                    break;
                case Assignment::Parameter:
                    if(!v->tval.type->hasModifier(Tok_Ante)){
                        c->compErr("Cannot evaluate a non-ante parameter during compile-time.  Mark the parameter's type with 'ante' to take in the parameter during compile-time", n->loc);
                    }
                    break;
                case Assignment::TypeVar:
                    break;
                case Assignment::Normal:
                    if(v->tval.type->hasModifier(Tok_Mut) && !v->tval.type->hasModifier(Tok_Ante)){
                        c->compErr("Cannot evaluate a mutable variable during compile-time.  Use 'ante mut' in its declaration instead if you wish to evaluate it.", n->loc);
                    }else if(v->assignments.back().assignmentExpr){
                        visitExternalDecl(n->name, v->tval.type, v->assignments.back().assignmentExpr);
                    }else{
                        c->compErr("Cannot find last assignment to variable used in ante expression.", n->loc);
                    }
                    break;
            }
        }
    }

    void AnteVisitor::visit(parser::GlobalNode *n){
        for(auto &m : n->vars){
            m->accept(*this);
            declare(m->name);
        }
    }

    void AnteVisitor::visit(parser::StrLitNode *n){
        //nothing to do
    }

    void AnteVisitor::visit(parser::VarAssignNode *n){
        n->expr->accept(*this);

        if(n->modifiers.empty()){
            //assignment
            n->ref_expr->accept(*this);
        }else{
            //declaration
            if(parser::VarNode *vn = dynamic_cast<parser::VarNode*>(n->ref_expr)){
                declare(vn->name);
            }else{
                c->compErr("Pattern-declarations currently unimplemented in ante expressions", n->ref_expr->loc);
            }
        }
    }

    void AnteVisitor::visit(parser::ExtNode *n){
        for(auto fn : *n->methods)
            fn->accept(*this);
    }

    void AnteVisitor::visit(parser::ImportNode *n){
        n->expr->accept(*this);
    }

    void AnteVisitor::visit(parser::JumpNode *n){
        if(n->expr)
            n->expr->accept(*this);
    }

    void AnteVisitor::visit(parser::WhileNode *n){
        n->condition->accept(*this);
        n->child->accept(*this);
    }

    void AnteVisitor::visit(parser::ForNode *n){
        declare(n->var);
        n->range->accept(*this);
        n->child->accept(*this);
    }

    void AnteVisitor::visit(parser::MatchBranchNode *n){
        implicitDeclare = true;
        n->pattern->accept(*this);
        implicitDeclare = false;
        n->branch->accept(*this);
    }

    void AnteVisitor::visit(parser::MatchNode *n){
        n->expr->accept(*this);
        for(auto &m : n->branches)
            m->accept(*this);
    }

    void AnteVisitor::visit(parser::IfNode *n){
        n->condition->accept(*this);
        n->thenN->accept(*this);
        if(n->elseN)
            n->elseN->accept(*this);
    }

    void AnteVisitor::visit(parser::FuncDeclNode *n){
        declare(n->name);
    }

    void AnteVisitor::visit(parser::DataDeclNode *n){
        //nothing to do
    }

    void AnteVisitor::visit(parser::TraitNode *n){
        //nothing to do
    }
}
