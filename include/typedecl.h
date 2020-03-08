#ifndef AN_TYPEDECL_H
#define AN_TYPEDECL_H

#include <llvm/IR/Type.h>
#include "antype.h"

namespace ante {
    /**
     * Contains additional information about a user-declared type.
     * Unlike AnDataType, there is only 1 TypeDecl for each declared type.
     * When using functions like toLlvmType, an AnDataType needs to be
     * provided to know which generic instance of the type we are in.
     *
     * Types in the type system only contain information needed for type checking,
     * they do not contain e.g. fields since data types do not check fields when
     * checking if two types are equal (they check name and type args). TypeDecls
     * store the additional information needed for fields, union tag values, etc.
     */
    class TypeDecl {
    public:
        AnType *type;
        LOC_TY &loc;

        bool isUnionType;
        bool isAlias;

        AnType *aliasedType;

        mutable std::unordered_map<TypeArgs, llvm::Type*> variantTypes;

        TypeDecl(AnType *type, LOC_TY &loc) : type{type}, loc{loc}, isUnionType{false}, isAlias{false}{}

        std::vector<std::string> fields;

        void addField(std::string const& name, AnType *type);

        std::vector<AnType*> getBoundFieldTypes(const AnDataType *dt) const;
        std::vector<AnType*>& getUnboundFieldTypes();
        std::vector<AnType*> getLargestVariantFieldTypes(Compiler *c, const AnDataType *dt) const;

        llvm::Type* toLlvmType(Compiler *c, const AnDataType *type) const;
        Result<size_t, std::string> getSizeInBits(Compiler *c, std::string const& incompleteType, const AnDataType *type) const;
        size_t getTagIndex(std::string const& tag) const;
        llvm::Value* getTagValue(Compiler *c, const AnDataType *type, std::string const& variantName, std::vector<TypedValue> const& args) const;
        const AnType* getLargestVariant(Compiler *c, const AnDataType *type) const;
        size_t getFieldIndex(std::string const& field) const;

    private:
        std::vector<AnType*> fieldTypes;
    };
}

#endif
