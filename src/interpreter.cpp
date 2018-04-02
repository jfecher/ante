#include "compiler.h"
#include "anyvalue.h"
#include "types.h"

using namespace ante;
using namespace ante::parser;
using namespace std;

#ifdef hi_there

AnyValue RootNode::eval(Compiler *c){

}


AnyValue IntLitNode::eval(Compiler *c){
    return AnyValue(atoi(val.c_str()), AnType::getPrimitive(type));
}


AnyValue FltLitNode::eval(Compiler *c){
    return AnyValue(atof(val.c_str()), AnType::getPrimitive(type));
}


AnyValue BoolLitNode::eval(Compiler *c){
    return AnyValue(val, AnType::getBool());
}


AnyValue CharLitNode::eval(Compiler *c){
    return AnyValue(val, AnType::getPrimitive(TT_C8));
}


AnyValue ArrayNode::eval(Compiler *c){
    vector<AnyValue> arr;
    AnType *elemTy = exprs.empty() ? AnType::getVoid() : nullptr;

    int i = 1;
    for(auto& n : exprs){
        auto val = n->eval(c);

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

    return AnyValue(arr, AnArrayType::get(elemTy, exprs.size()));
}


AnyValue TupleNode::eval(Compiler *c){
    vector<AnyValue> vals;
    vals.reserve(exprs.size());

    vector<AnType*> types;
    types.reserve(exprs.size());

    for(auto &e : exprs){
        auto val = e->eval(c);
        vals.push_back(val);
        types.push_back(val.type);
    }

    AnType *t = AnAggregateType::get(TT_Tuple, types);
    return AnyValue(vals, t);
}


AnyValue UnOpNode::eval(Compiler *c){

}


AnyValue BinOpNode::eval(Compiler *c){

}


AnyValue SeqNode::eval(Compiler *c){

}


AnyValue BlockNode::eval(Compiler *c){

}


AnyValue ModNode::eval(Compiler *c){

}


AnyValue TypeNode::eval(Compiler *c){

}


AnyValue TypeCastNode::eval(Compiler *c){

}


AnyValue RetNode::eval(Compiler *c){

}


AnyValue NamedValNode::eval(Compiler *c){

}


AnyValue VarNode::eval(Compiler *c){

}


AnyValue GlobalNode::eval(Compiler *c){

}


AnyValue StrLitNode::eval(Compiler *c){

}


AnyValue LetBindingNode::eval(Compiler *c){

}


AnyValue VarDeclNode::eval(Compiler *c){

}


AnyValue VarAssignNode::eval(Compiler *c){

}


AnyValue ExtNode::eval(Compiler *c){

}


AnyValue ImportNode::eval(Compiler *c){

}


AnyValue JumpNode::eval(Compiler *c){

}


AnyValue WhileNode::eval(Compiler *c){

}


AnyValue ForNode::eval(Compiler *c){

}


AnyValue MatchBranchNode::eval(Compiler *c){

}


AnyValue MatchNode::eval(Compiler *c){

}


AnyValue IfNode::eval(Compiler *c){

}


AnyValue FuncDeclNode::eval(Compiler *c){

}


AnyValue DataDeclNode::eval(Compiler *c){

}


AnyValue TraitNode::eval(Compiler *c){

}

#endif
