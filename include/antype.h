#ifndef AN_TYPE_H
#define AN_TYPE_H

#include <string>
#include <vector>
#include <memory>

#include <llvm/IR/Module.h>

#include "tokens.h"
#include "parser.h"

namespace ante {
    struct Compiler;
    struct UnionTag;
    struct Trait;

    class AnModifier;
    class AnAggregateType;
    class AnArrayType;
    class AnPtrType;
    class AnTypeVarType;
    class AnDataType;
    class AnFunctionType;
    class AnTypeContainer;

    //Most primitive types
    class AnType {
        friend AnTypeContainer;

    protected:
        AnType(TypeTag id, bool ig, AnModifier *m) :
            typeTag(id), isGeneric(ig), mods(m){}
        //virtual ~AnType() = delete;

    public:
        TypeTag typeTag;
        bool isGeneric;
        AnModifier *mods;

        bool hasModifier(TokenType m) const;
        virtual AnType* addModifier(TokenType m);
        virtual AnType* setModifier(AnModifier *m);

        size_t getSizeInBits(Compiler *c, std::string *incompleteType = nullptr, bool force = false) const;

        void dump() const;

        AnType* getFunctionReturnType() const;

        static AnType* getPrimitive(TypeTag tag, AnModifier *mod = nullptr);
        static AnType* getI8();
        static AnType* getI16();
        static AnType* getI32();
        static AnType* getI64();
        static AnType* getIsz();
        static AnType* getU8();
        static AnType* getU16();
        static AnType* getU32();
        static AnType* getU64();
        static AnType* getUsz();
        static AnType* getF16();
        static AnType* getF32();
        static AnType* getF64();
        static AnType* getBool();
        static AnType* getVoid();
        static AnPtrType* getPtr(AnType*);
        static AnDataType* getDataType(std::string name);
        static AnArrayType* getArray(AnType*, size_t len = 0);
        static AnTypeVarType* getTypeVar(std::string name);
        static AnFunctionType* getFunction(AnType *r, const std::vector<AnType*>);
        static AnAggregateType* getAggregate(TypeTag t, const std::vector<AnType*>);
    };

    bool isGeneric(const std::vector<AnType*> &vec);

    //Type modifiers
    class AnModifier {
        protected:
        AnModifier(std::vector<TokenType> mods) :
            modifiers(mods){}

        public:
        std::vector<TokenType> modifiers;
        std::vector<std::unique_ptr<PreProcNode>> compilerDirectives;
        
        static AnModifier* get(std::vector<TokenType> modifiers);
    };

    //Tuples
    class AnAggregateType : public AnType {
        protected:
        AnAggregateType(TypeTag ty, const std::vector<AnType*> exts, AnModifier *m) :
                AnType(ty, ante::isGeneric(exts), m), extTys(exts) {}

        public:
        std::vector<AnType*> extTys;

        static AnAggregateType* get(TypeTag t, std::vector<AnType*> types, AnModifier *m = nullptr);

        /** @brief Get a function type. */
        static AnAggregateType* get(AnType* retty, NamedValNode* params, AnModifier *m = nullptr);
        
        AnAggregateType* addModifier(TokenType m) override;
        AnAggregateType* setModifier(AnModifier *m) override;
        
        static bool classof(const AnType *t){
            return t->typeTag == TT_Tuple or t->typeTag == TT_Function
                or t->typeTag == TT_Data;
        }
    };

    //Arrays, both sized and not
    class AnArrayType : public AnType {
        protected:
        AnArrayType(AnType* ext, size_t l, AnModifier *m) :
            AnType(TT_Array, ext->isGeneric, m), extTy(ext), len(l) {}

        public:
        AnType *extTy;
        
        /** @brief Length of the array type.  0 if not specified */
        size_t len;
        
        static AnArrayType* get(AnType*, size_t len = 0, AnModifier *m = nullptr);
        
        AnArrayType* addModifier(TokenType m) override;
        AnArrayType* setModifier(AnModifier *m) override;

        static bool classof(const AnType *t){
            return t->typeTag == TT_Array;
        }
    };
    
    class AnPtrType : public AnType {
        protected:
        AnPtrType(AnType* ext, AnModifier *m) :
            AnType(TT_Ptr, ext->isGeneric, m), extTy(ext){}

        public:
        AnType *extTy;

        static AnPtrType* get(AnType* l, AnModifier *m = nullptr);
        
        AnPtrType* addModifier(TokenType m) override;
        AnPtrType* setModifier(AnModifier *m) override;
        
        static bool classof(const AnType *t){
            return t->typeTag == TT_Ptr;
        }
    };

    //Typevars
    class AnTypeVarType : public AnType {
        protected:
        AnTypeVarType(std::string &n, AnModifier *m) :
            AnType(TT_TypeVar, true, m), name(n){}

        public:
        std::string name;

        static AnTypeVarType* get(std::string name, AnModifier *m = nullptr);
        
        AnTypeVarType* addModifier(TokenType m) override;
        AnTypeVarType* setModifier(AnModifier *m) override;
        
        static bool classof(const AnType *t){
            return t->typeTag == TT_TypeVar;
        }
    };

    class AnFunctionType : public AnAggregateType {
        protected:
        AnFunctionType(AnType *ret, std::vector<AnType*> elems, bool isMetaFunction, AnModifier *m) :
            AnAggregateType(isMetaFunction ? TT_MetaFunction : TT_Function, elems, m), retTy(ret){}

        public:
        AnType *retTy;

        static AnFunctionType* get(AnType *retTy, const std::vector<AnType*> elems, bool isMetaFunction = false, AnModifier *m = nullptr);
        static AnFunctionType* get(Compiler *c, AnType* retty, NamedValNode* params, bool isMetaFunction = false, AnModifier *m = nullptr);
        
        AnFunctionType* addModifier(TokenType m) override;
        AnFunctionType* setModifier(AnModifier *m) override;

        static bool classof(const AnType *t){
            return t->typeTag == TT_Function or t->typeTag == TT_MetaFunction;
        }
    };

    //User type declarations
    class AnDataType : public AnAggregateType {

        protected:
        AnDataType(std::string &n, const std::vector<AnType*> elems, bool isUnion, AnModifier *m) :
            AnAggregateType(isUnion ? TT_TaggedUnion : TT_Data, elems, m), name(n), fields(), tags(),
            traitImpls(), unboundType(0), variants(), parentUnionType(0), boundGenerics(), llvmType(0){}

        public:
        std::string name;

        /** @brief Names of each field. */
        std::vector<std::string> fields;
        std::vector<std::shared_ptr<UnionTag>> tags;
        std::vector<std::shared_ptr<Trait>> traitImpls;

        /** @brief If this type is a bound version (eg. Maybe<i32>) of some generic
         * type (eg. Maybe<'t>), unboundType will point to the generic type. */
        AnDataType *unboundType;

        /**
         * @brief If bound versions of generic types are stored in the variants field
         * so they may all be accessed and updated if the definition of their generic parent
         * type is changed.
         */
        std::vector<AnDataType*> variants;

        /** @brief The parent union type of this type if it is a union tag */
        AnDataType *parentUnionType;

        std::vector<AnTypeVarType*> generics;
        std::vector<std::pair<std::string, AnType*>> boundGenerics;

        /** @brief Types are lazily translated into their llvm::Type counterpart to better support
        * generics and prevent the need of forward-decls */
        llvm::Type* llvmType;

        static AnDataType* get(std::string name, AnModifier *m = nullptr);
        static AnDataType* getVariant(Compiler *c, const std::string &name, const std::vector<std::pair<std::string, AnType*>> &boundTys, AnModifier *m = nullptr);
        static AnDataType* getVariant(Compiler *c, AnDataType *unboundType, const std::vector<std::pair<std::string, AnType*>> &boundTys, AnModifier *m);
        static AnDataType* getOrCreate(std::string name, std::vector<AnType*> &elems, bool isUnion, AnModifier *m = nullptr);
        static AnDataType* getOrCreate(const AnDataType *dt, AnModifier *m = nullptr);
        static AnDataType* create(std::string name, std::vector<AnType*> elems, bool isUnion, const std::vector<AnTypeVarType*> &generics, AnModifier *m = nullptr);

        AnDataType* addModifier(TokenType m) override;
        AnDataType* setModifier(AnModifier *m) override;

        /** @brief returns true if this type is a bound variant of the generic type dt */
        bool isVariantOf(const AnDataType *dt) const;

        static bool classof(const AnType *t){
            return t->typeTag == TT_Data or t->typeTag == TT_TaggedUnion;
        }

        /**
        * @param field Name of the field to search for
        *
        * @return The index of the field on success, -1 on failure
        */
        int getFieldIndex(std::string &field) const {
            for(unsigned int i = 0; i < fields.size(); i++)
                if(field == fields[i])
                    return i;
            return -1;
        }

        bool isStub() const {
            return extTys.empty();
        }
        
        /**
        * @return True if this DataType is actually a tag type
        */
        bool isUnionTag() const {
            return parentUnionType;
        }

        /**
        * @brief Gets the name of the parent union type
        *
        * Will fail if this DataType is not a union tag and contains no fields.
        * Use isUnionTag before calling this function if unsure.
        *
        * @return The name of the DataType containing this UnionTag
        */
        //std::string getParentUnionName() const {
        //    return parentUnionType->name;
        //}

        /**
        * @brief Returns the UnionTag value of a tag within the union type.
        *
        * This function assumes the tag is within the type. The 0 returned
        * on failure is indistinguishable from a tag of value 0 and will be
        * changed to an exception at a later date.
        *
        * @param name Name of the tag to search for
        *
        * @return the value of the tag found, or 0 on failure
        */
        unsigned short getTagVal(std::string &name);
    };

    class AnTypeContainer {
        friend AnType;
        friend AnModifier;
        friend AnAggregateType;
        friend AnArrayType;
        friend AnPtrType;
        friend AnTypeVarType;
        friend AnFunctionType;
        friend AnDataType;

        std::map<TypeTag,       std::unique_ptr<AnType>> primitiveTypes;
        std::map<std::string,   std::unique_ptr<AnType>> otherTypes;
        std::map<std::string,   std::unique_ptr<AnModifier>> modifiers;
        std::map<const AnType*, std::unique_ptr<AnPtrType>> ptrTypes;
        std::map<std::string,   std::unique_ptr<AnArrayType>> arrayTypes;
        std::map<std::string,   std::unique_ptr<AnTypeVarType>> typeVarTypes;
        std::map<std::string,   std::unique_ptr<AnAggregateType>> aggregateTypes;
        std::map<std::string,   std::unique_ptr<AnFunctionType>> functionTypes;
        std::map<std::string,   std::unique_ptr<AnDataType>> declaredTypes;

    public:
        AnTypeContainer();
        ~AnTypeContainer() = default;

        void clearDeclaredTypes(){
            declaredTypes.clear();
        }
    };
}

#endif
