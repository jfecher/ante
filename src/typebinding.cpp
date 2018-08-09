#include "typebinding.h"
#include "antype.h"

namespace ante {
    std::ostream& operator<<(std::ostream &o, ante::TypeBinding const& b){
        if(b.isNominalBinding())
            return o << b.getTypeVarName() << " -> " << anTypeToStr(b.getBinding());
        else
            return o << b.getIndex() << " -> " << anTypeToStr(b.getBinding());
    }

    std::ostream& operator<<(std::ostream& o, GenericTypeParam const& p){
        o << p.typeVarName;
        if(!p.isNominalBinding())
            o << " (pos " << p.pos << ')';
        return o;
    }

    bool ante::TypeBinding::matches(GenericTypeParam const& gtp) const {
        return this->param == gtp;
    }

    const AnDataType* getParentTypeOrSelf(const AnDataType* dt){
        if(!dt) return dt;

        if(dt->parentUnionType)
            dt = dt->parentUnionType;

        if(dt->unboundType)
            return dt->unboundType;
        else
            return dt;
    }
}
