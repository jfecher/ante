#ifndef AN_TYPEBINDING_H
#define AN_TYPEBINDING_H

#include <ostream>
#include <string>

namespace ante {
    class AnType;
    class AnDataType;
            
    std::string anTypeToStr(const AnType*);

    const AnDataType* getParentTypeOrSelf(const AnDataType*);

    /**
     * Represents a generic parameter to a type.
     *
     * Can be a nominal parameter (bound by name)
     * or a structural one (bound by position).
     *
     * Example: 't => nominal
     * Example: Maybe _ => structural (pos 0 of Maybe)
     *
     * Note on structural bindings:
     * when a user declares type T 'a = ...
     * and later uses T i32 or some other variant,
     * we do NOT want to bind 'a to i32 as this would
     * conflict with other typechecks.  Consider the function:
     *
     * fun foo: 'a x =
     *     let y = T 58
     *     x
     *
     * foo "bar"
     *
     * In the call to foo "bar", 'a is bound to Str but
     * in the body (without structural bindings) T 'a would
     * try to be bound to i32 but this is a type error since
     * 'a already equals Str.  This is why positional bindings
     * are used for generic datatypes with unspecified
     * type parameters like T.
     */
    struct GenericTypeParam {
        /** Name of a typevar in a nominal binding. */
        std::string typeVarName;

        /** The unbound/curried datatype in a structural binding. */
        const AnDataType *dt;

        /** The index to be bound in a structural binding. */
        size_t pos;

        /** Constructor for nominal generic parameter. */
        GenericTypeParam(std::string const& name)
            : typeVarName(name), dt(nullptr){
            
            if(name.empty())
                puts("!!! GenericTypeParam(string name): name cannot be empty !!!");
        }

        /** Constructor for structural generic parameter. */
        GenericTypeParam(std::string const& name, const AnDataType *t, size_t p)
            : typeVarName(name), dt(getParentTypeOrSelf(t)), pos(p){}

        bool isNominalBinding() const { return !dt; }

        bool operator==(GenericTypeParam const& r) const {
            if(isNominalBinding()){
                return typeVarName == r.typeVarName;
            }else{
                return pos == r.pos && dt == r.dt;
            }
        }
    };

    std::ostream& operator<<(std::ostream& o, GenericTypeParam const& p);

    /**
     * Represents a singular type binding, eg 't to i32.
     * 
     * Type bindings can be by typevar name, like the above
     * example, or by their positioning in a generic type,
     * eg binding Maybe to Maybe i32 would require binding
     * the first argument of Maybe to i32 while ignoring the
     * name the typevar was declared with.
     *
     * This keeps code such as the following working:
     * 
     * type Maybe 't = | Some 't | None
     *
     * ('t, Maybe) = (i32, Maybe Str) //=> true
     *
     * Despite Maybe being declared as Maybe 't, the
     * two 't are separate and are treated as such.
     */
    class TypeBinding {
        /** The generic type parameter to bind to. */
        GenericTypeParam param;

        /** The type to be bound to. */
        AnType *boundType;

        public:
        TypeBinding(std::string const& name, AnType *binding)
            : param{name}, boundType{binding}{}

        TypeBinding(std::string const& name, const AnDataType *parentTy,
                size_t idx, AnType *binding)
            : param{name, parentTy, idx}, boundType{binding}{}

        bool isNominalBinding() const { return param.isNominalBinding(); }

        /* Return true if the given generic parameter matches the internal one. */
        bool matches(GenericTypeParam const& param) const;

        AnType* getBinding() const { return boundType; }
        
        void setBinding(AnType *t) { boundType = t; }

        /** Only valid for nominal matches. */
        const std::string& getTypeVarName() const { return param.typeVarName; }

        /** Only valid for structural matches. */
        const AnDataType* getDataType() const { return param.dt; }

        const GenericTypeParam& getGenericTypeParam() const { return param; }

        /** Only valid for structural matches. */
        size_t getIndex() const { return param.pos; }

        /** Note: only returns true if the two bindings are exactly equal.
         * That is, the boundTypes of both bindings must be exactly the same,
         * unlike in typeEq which allows type modifiers to differ. */
        bool operator==(TypeBinding const& r) const {
            bool nominal = this->isNominalBinding();
            if(r.isNominalBinding() != nominal)
                return false;

            return param == r.param && boundType == r.boundType;
        }
    };

    std::ostream& operator<<(std::ostream &o, ante::TypeBinding const& b);
}

#endif
