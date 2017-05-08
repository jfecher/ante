#ifndef AN_TYPES_H
#define AN_TYPES_H

#include "compiler.h"

TypedValue* typeCheckWithImplicitCasts(Compiler *c, TypedValue *arg, TypeNode *ty);

TypeNode* deepCopyTypeNode(const TypeNode *n);
string typeNodeToStr(const TypeNode *t);
lazy_str typeNodeToColoredStr(const TypeNode *t);
lazy_str typeNodeToColoredStr(const unique_ptr<TypeNode>& tn);

//conversions
Type* typeTagToLlvmType(TypeTag tagTy, LLVMContext &c, string typeName = "");
TypeTag llvmTypeToTypeTag(Type *t);
string llvmTypeToStr(Type *ty);
string typeTagToStr(TypeTag ty);
bool llvmTypeEq(Type *l, Type *r);
        
void bindGenericToType(TypeNode *tn, const vector<pair<string, unique_ptr<TypeNode>>> &bindings);
void bindGenericToType(TypeNode *tn, const vector<unique_ptr<TypeNode>> &bindings);

char getBitWidthOfTypeTag(const TypeTag tagTy);
bool isNumericTypeTag(const TypeTag ty);
bool isIntTypeTag(const TypeTag ty);
bool isFPTypeTag(const TypeTag tt);
bool isUnsignedTypeTag(const TypeTag tagTy);

#endif
