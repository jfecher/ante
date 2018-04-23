#include "compiler.h"
#include "anyvalue.h"
#include "types.h"

using namespace ante;
using namespace ante::parser;
using namespace std;

void InterpretingVisitor::visit(RootNode *n){

}


void InterpretingVisitor::visit(IntLitNode *n){
    val = AnyValue(atoi(n->val.c_str()), AnType::getPrimitive(n->type));
}


void InterpretingVisitor::visit(FltLitNode *n){
    val = AnyValue(atof(n->val.c_str()), AnType::getPrimitive(n->type));
}


void InterpretingVisitor::visit(BoolLitNode *n){
    val = AnyValue(n->val, AnType::getBool());
}


void InterpretingVisitor::visit(CharLitNode *n){
    val = AnyValue(n->val, AnType::getPrimitive(TT_C8));
}


void InterpretingVisitor::visit(ArrayNode *n){
    vector<AnyValue> arr;
    AnType *elemTy = n->exprs.empty() ? AnType::getVoid() : nullptr;

    int i = 1;
    for(auto& n : n->exprs){
        n->accept(*this);

        arr.push_back(val);

        if(!elemTy){
            elemTy = val.type;
        }else{
            if(!c->typeEq(val.type, elemTy))
                c->compErr("Element " + to_string(i) + "'s type " + anTypeToColoredStr(val.type) +
                    " does not match the first element's type of " + anTypeToColoredStr(elemTy), n->loc);
        }
        i++;
    }

    val = AnyValue(arr, AnArrayType::get(elemTy, n->exprs.size()));
}


void InterpretingVisitor::visit(TupleNode *n){
    vector<AnyValue> vals;
    vals.reserve(n->exprs.size());

    vector<AnType*> types;
    types.reserve(n->exprs.size());

    for(auto &e : n->exprs){
        e->accept(*this);
        vals.push_back(val);
        types.push_back(val.type);
    }

    AnType *t = AnAggregateType::get(TT_Tuple, types);
    val = AnyValue(vals, t);
}


void InterpretingVisitor::visit(UnOpNode *n){
    n->rval->accept(*this);

    switch(n->op){
        case '@': //pointer dereference
            if(val.type->typeTag != TT_Ptr){
                c->compErr("Cannot dereference non-pointer type " + anTypeToColoredStr(val.type), n->loc);
            }

            this->val = AnyValue(*(void**)val.val, ((AnPtrType*)val.type)->extTy);
            return;
        case '&': //address-of
            // Casting val.val to (int*) should select the generic constructor that allocates values
            this->val = AnyValue((int*)val.val, ((AnPtrType*)val.type)->extTy);
            return;
        case '-': //negation
            this->val = AnyValue(-*(size_t*)val.val, val.type);
            return;
        case Tok_Not:
            if(val.type->typeTag != TT_Bool)
                c->compErr("Unary not operator not overloaded for type " + anTypeToColoredStr(val.type), n->loc);

            this->val = AnyValue(!*(bool*)val.val, val.type);
            return;
        case Tok_New:
            //the 'new' keyword in ante creates a reference to any existing value
            this->val = AnyValue((int*)val.val, ((AnPtrType*)val.type)->extTy);
            return;
    }

    c->compErr("Unknown unary operator " + Lexer::getTokStr(n->op), n->loc);
}


void InterpretingVisitor::visit(BinOpNode *n){

}


void InterpretingVisitor::visit(SeqNode *n){

}


void InterpretingVisitor::visit(BlockNode *n){

}


void InterpretingVisitor::visit(ModNode *n){

}


void InterpretingVisitor::visit(TypeNode *n){

}


void InterpretingVisitor::visit(TypeCastNode *n){

}


void InterpretingVisitor::visit(RetNode *n){

}


void InterpretingVisitor::visit(NamedValNode *n){

}


void InterpretingVisitor::visit(VarNode *n){

}


void InterpretingVisitor::visit(GlobalNode *n){

}


void InterpretingVisitor::visit(StrLitNode *n){

}


void InterpretingVisitor::visit(VarDeclNode *n){

}


void InterpretingVisitor::visit(VarAssignNode *n){

}


void InterpretingVisitor::visit(ExtNode *n){

}


void InterpretingVisitor::visit(ImportNode *n){

}


void InterpretingVisitor::visit(JumpNode *n){

}


void InterpretingVisitor::visit(WhileNode *n){

}


void InterpretingVisitor::visit(ForNode *n){

}


void InterpretingVisitor::visit(MatchBranchNode *n){

}


void InterpretingVisitor::visit(MatchNode *n){

}


void InterpretingVisitor::visit(IfNode *n){

}


void InterpretingVisitor::visit(FuncDeclNode *n){

}


void InterpretingVisitor::visit(DataDeclNode *n){

}


void InterpretingVisitor::visit(TraitNode *n){

}
