#ifndef AN_TRAIT_H
#define AN_TRAIT_H

#include "funcdecl.h"
#include "antype.h"

namespace ante {
    class AnTypeVarType;

    /** Abstract base class for TraitDecl, TraitImpl */
    struct TraitBase {
        std::string name;
        TypeArgs typeArgs;

        TraitBase(std::string const& name, TypeArgs const& typeArgs)
          : name{name}, typeArgs{typeArgs}{}
    };

    struct TraitDecl : public TraitBase {
        std::vector<std::shared_ptr<FuncDecl>> funcs;

        TraitDecl(std::string const& name, TypeArgs const& typeArgs)
          : TraitBase(name, typeArgs){}
    };

    /**
     * @brief Holds the name of a trait and the functions needed to implement it
     */
    struct TraitImpl : public TraitBase {
        TraitImpl(std::string name, TypeArgs const& tArgs)
                : TraitBase(name, tArgs) {}

        TraitImpl(TraitDecl *decl, TypeArgs const& tArgs)
                : TraitBase(decl->name, tArgs), decl{decl} {}

        /** Pointer to the ExtNode of where this trait instance is
         *  implemented or nullptr if it is not implemented. */
        parser::ExtNode *impl = nullptr;

        TraitDecl *decl = nullptr;

        ~TraitImpl() = default;

        bool implemented() const noexcept {
            return impl;
        }

        const std::string& getName() const noexcept {
            return name;
        }

        bool operator==(TraitImpl const& r){
            return name == r.name && typeArgs == r.typeArgs;
        }

        bool hasTrivialImpl() const;
    };

    TraitImpl* toTrait(parser::TypeNode *tn, Module *m);
    std::string traitToStr(const TraitImpl *trait);
    lazy_str traitToColoredStr(const TraitImpl *trait);
}


#endif /* end of include guard: AN_TRAIT_H */
