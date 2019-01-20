#include "antevalue.h"
#include "types.h"
#include "uniontag.h"

using namespace std;
using namespace llvm;

namespace ante {
    float f32_from_f16(uint16_t f) {
        return f;
    }

    TypedValue convertTupleToTypedValue(Compiler *c, AnteValue const& arg, AnAggregateType *tn){
        if(tn->extTys.empty()){
            return c->getVoidLiteral();
        }

        auto elems = vecOf<Constant*>(tn->extTys.size());
        auto elemTys = vecOf<Type*>(tn->extTys.size());
        auto anElemTys = vecOf<AnType*>(tn->extTys.size());

        map<unsigned, Value*> nonConstants;
        size_t offset = 0;

        for(unsigned i = 0; i < tn->extTys.size(); i++){
            char* elem = (char*)arg.asRawData() + offset;
            AnteValue elemTup{(void*)elem, tn->extTys[i]};
            TypedValue tval = elemTup.asTypedValue(c);

            if(Constant *elem = dyn_cast<Constant>(tval.val)){
                elems.push_back(elem);
            }else{
                nonConstants[i] = tval.val;
                elems.push_back(UndefValue::get(tval.getType()));
            }

            auto res = tn->extTys[i]->getSizeInBits(c);
            if(!res){
                error("Unknown/Unimplemented TypeTag " + typeTagToStr(tn->typeTag), unknownLoc());
            }

            offset += res.getVal() / 8;
            elemTys.push_back(tval.getType());
            anElemTys.push_back(tval.type);
        }

        //Create the constant tuple with undef values in place for the non-constant values
        Value* tuple = ConstantStruct::get((StructType*)c->anTypeToLlvmType(tn), elems);

        //Insert each pathogen value into the tuple individually
        for(const auto &p : nonConstants){
            tuple = c->builder.CreateInsertValue(tuple, p.second, p.first);
        }

        return TypedValue(tuple, tn);
    }


    /**
     * Finds and returns the last stored value from a LoadInst
     * of a mutable variable.
     */
    TypedValue findLastStore(Compiler *c, TypedValue const& tv){
        //mutable pointer passed, find last store
        if(LoadInst *si = dyn_cast<LoadInst>(tv.val)){
            for(auto *u : si->getPointerOperand()->users()){
                if(StoreInst *si = dyn_cast<StoreInst>(u)){
                    Value *vo = si->getValueOperand();

                    if(BitCastInst *be = dyn_cast<BitCastInst>(vo)){
                        for(auto *u : be->users()){
                            if(StoreInst *si = dyn_cast<StoreInst>(u)){
                                return {si->getValueOperand(), tv.type};
                            }
                        }
                    }
                    return {vo, tv.type};
                }
            }
        }else{
            return tv;
        }

        error("Cannot find last store to mutable variable during translation", unknownLoc());
        return {};
    }


    /**
     * Converts an AnteValue into a typedValue.  If the type
     * cannot be converted or an error occurs, c's error flag
     * is set and a void literal is returned.
     */
    TypedValue AnteValue::asTypedValue(Compiler *c) const{
        switch(type->typeTag){
            case TT_I8:              return TypedValue(c->builder.getInt8( *(uint8_t*) data), type);
            case TT_I16:             return TypedValue(c->builder.getInt16(*(uint16_t*)data), type);
            case TT_I32:             return TypedValue(c->builder.getInt32(*(uint32_t*)data), type);
            case TT_I64:             return TypedValue(c->builder.getInt64(*(uint64_t*)data), type);
            case TT_U8:              return TypedValue(c->builder.getInt8( *(uint8_t*) data), type);
            case TT_U16:             return TypedValue(c->builder.getInt16(*(uint16_t*)data), type);
            case TT_U32:             return TypedValue(c->builder.getInt32(*(uint32_t*)data), type);
            case TT_U64:             return TypedValue(c->builder.getInt64(*(uint64_t*)data), type);
            case TT_Isz:             return TypedValue(c->builder.getIntN(AN_USZ_SIZE, *(size_t*) data), type);
            case TT_Usz:             return TypedValue(c->builder.getIntN(AN_USZ_SIZE, *(size_t*) data), type);
            case TT_C8:              return TypedValue(c->builder.getInt8( *(uint8_t*) data), type);
            case TT_C32:             return TypedValue(c->builder.getInt32(*(uint32_t*)data), type);
            case TT_F16:             return TypedValue(ConstantFP::get(*c->ctxt, APFloat(f32_from_f16(*(uint16_t*)data))), type);
            case TT_F32:             return TypedValue(ConstantFP::get(*c->ctxt, APFloat(*(float*)data)), type);
            case TT_F64:             return TypedValue(ConstantFP::get(*c->ctxt, APFloat(*(double*)data)), type);
            case TT_Bool:            return TypedValue(c->builder.getInt1(*(uint8_t*)data), type);
            case TT_Array:           break;
            case TT_Ptr: {
                auto *cint = c->builder.getIntN(AN_USZ_SIZE, *(size_t*)data);
                auto *ty = c->anTypeToLlvmType(type);
                return TypedValue(c->builder.CreateIntToPtr(cint, ty), type);
            }
            case TT_Data:
            case TT_Tuple:
                return convertTupleToTypedValue(c, *this, try_cast<AnAggregateType>(type));
            case TT_TypeVar:
            case TT_Function:
            case TT_TaggedUnion:
            case TT_MetaFunction:
            case TT_FunctionList:
            case TT_Type:
            case TT_Trait:
                break;
            case TT_Void:
                return c->getVoidLiteral();
        }

        error("Unknown/Unimplemented TypeTag " + typeTagToStr(type->typeTag), unknownLoc());
        return {};
    }

    void AnteValue::allocAndStoreValue(Compiler *c, TypedValue const& tv){
        auto size = tv.type->getSizeInBits(c);
        if(!size){
            error(size.getErr(), unknownLoc());
        }
        data = malloc(size.getVal() / 8);
        storeValue(c, tv);
    }


    /**
     * Stores a pointer value of a constant pointer type
     */
    void AnteValue::storePtr(Compiler *c, TypedValue const &tv){
        auto *ptrty = try_cast<AnPtrType>(tv.type);

        if(ConstantExpr *ce = dyn_cast<ConstantExpr>(tv.val)){
            Instruction *in = ce->getAsInstruction();
            auto ptr = TypedValue(in, ptrty);
            storePtr(c, ptr);
        }else if(GlobalVariable *gv = dyn_cast<GlobalVariable>(tv.val)){
            Value *v = gv->getInitializer();
            if(ConstantDataArray *cda = dyn_cast<ConstantDataArray>(v)){
                char *cstr = strdup(cda->getAsString().str().c_str());
                *(void**)data = cstr;
            }else{
                TypedValue tv = {v, ptrty->extTy};
                void **oldData = (void**)data;
                data = *oldData;
                allocAndStoreValue(c, tv);
                *oldData = data;
                data = oldData;
            }
        }else if(BitCastInst *be = dyn_cast<BitCastInst>(tv.val)){
            //there should be stores in this bitcast if it was of malloc
            for(auto *u : be->users()){
                if(StoreInst *si = dyn_cast<StoreInst>(u)){
                    //Its possible this is a store of the same type if the pointer is mutable,
                    //we want what is stored within only
                    if(si->getValueOperand()->getType() == tv.val->getType()->getPointerElementType()){
                        TypedValue elem = {si->getValueOperand(), ptrty->extTy};
                        void **data_ptr = (void**)data;
                        allocAndStoreValue(c, elem);
                        *data_ptr = data;
                        data = data_ptr;
                        return;
                    }
                }
            }
        }else if(dyn_cast<LoadInst>(tv.val)){
            auto ptr = findLastStore(c, tv);
            storePtr(c, ptr);

        }else if(PtrToIntInst *ptii = dyn_cast<PtrToIntInst>(tv.val)){
            auto ptr = ptii->getOperand(0);
            storePtr(c, {ptr, tv.type});
        }else if(IntToPtrInst *itpi = dyn_cast<IntToPtrInst>(tv.val)){
            auto ptr = itpi->getOperand(0);

            if(ConstantInt *addr = dyn_cast<ConstantInt>(ptr)){
                *(void**)data = (void*)addr->getZExtValue();
            }else{
                storePtr(c, {ptr, tv.type});
            }
        }else if(BinaryOperator *bi = dyn_cast<BinaryOperator>(tv.val)){
            if(bi->getOpcode() == BinaryOperator::BinaryOps::Add && dyn_cast<ConstantInt>(bi->getOperand(1))){
                TypedValue ptr{bi->getOperand(0), tv.type};
                storePtr(c, ptr);
                *(char**)data += cast<ConstantInt>(bi->getOperand(1))->getZExtValue();
            }
        }else{
            tv.dump();
            error("unknown value given to getConstPtr of type: " + anTypeToColoredStr(tv.type), unknownLoc());
        }
    }


    void AnteValue::storeTuple(Compiler *c, TypedValue const& tup){
        auto *sty = try_cast<AnAggregateType>(tup.type);
        if(ConstantStruct *ca = dyn_cast<ConstantStruct>(tup.val)){
            void *orig_data = this->data;
            for(size_t i = 0; i < ca->getNumOperands(); i++){
                Value *elem = ca->getAggregateElement(i);
                AnType *ty = sty->extTys[i];
                auto field = TypedValue(elem, ty);
                storeValue(c, field);

                auto size = ty->getSizeInBits(c);
                if(!size){
                    error("storeTuple: " + size.getErr(), unknownLoc());
                }
                data = (char*)data + size.getVal() / 8;
            }
            data = orig_data;
        }else{
            //single-value "tuple"
            AnType *ty = sty->extTys[0];
            auto field = TypedValue(tup.val, ty);
            storeValue(c, field);
        }
    }


    void AnteValue::storeInt(Compiler *c, TypedValue const& tv){
        Value *v = tv.val;
        if(auto *e = dyn_cast<SExtInst>(tv.val)){
            v = e->getOperand(0);
        }

        auto *ci = dyn_cast<ConstantInt>(v);

        if(!ci){
            v->print(dbgs());
            error("Cannot convert non-constant integer", unknownLoc());
        }

        switch(tv.type->typeTag){
            case TT_I8:   *(uint8_t*) data = ci->getSExtValue(); return;
            case TT_U8:
            case TT_C8:
            case TT_Bool: *(uint8_t*) data = ci->getZExtValue(); return;
            case TT_I16:  *(uint16_t*)data = ci->getSExtValue(); return;
            case TT_U16:  *(uint16_t*)data = ci->getZExtValue(); return;
            case TT_I32:  *(uint32_t*)data = ci->getSExtValue(); return;
            case TT_U32:  *(uint32_t*)data = ci->getZExtValue(); return;
            case TT_C32:  *(uint32_t*)data = ci->getZExtValue(); return;
            case TT_I64:  *(uint64_t*)data = ci->getSExtValue(); return;
            case TT_U64:  *(uint64_t*)data = ci->getZExtValue(); return;
            case TT_Isz:  *(size_t*)  data = ci->getSExtValue(); return;
            case TT_Usz:  *(size_t*)  data = ci->getZExtValue(); return;
            default: return;
        }
    }


    void AnteValue::storeFloat(Compiler *c, TypedValue const& tv){
        auto *cf = dyn_cast<ConstantFP>(tv.val);
        if(!cf){
            error("Cannot convert non-constant floating point value", unknownLoc());
        }

        switch(tv.type->typeTag){
            case TT_F16: *(float*)   data = cf->getValueAPF().convertToFloat(); return;
            case TT_F32: *(float*)   data = cf->getValueAPF().convertToFloat(); return;
            case TT_F64: *(double*)  data = cf->getValueAPF().convertToDouble(); return;
            default: return;
        }
    }


    void AnteValue::storeValue(Compiler *c, TypedValue const& tv){
        switch(tv.type->typeTag){
            case TT_I8: case TT_U8: case TT_C8: case TT_Bool:
            case TT_I16: case TT_U16:
            case TT_I32: case TT_U32: case TT_C32:
            case TT_I64: case TT_U64: case TT_Isz: case TT_Usz:
                storeInt(c, tv);
                return;
            case TT_F16:
            case TT_F32:
            case TT_F64:
                storeFloat(c, tv);
                return;
            case TT_Ptr:
            case TT_Function:
            case TT_MetaFunction:
            case TT_Array: storePtr(c, tv); return;
            case TT_TypeVar: {
                //TODO: re-add
                //auto *tvt = try_cast<AnTypeVarType>(tv.type);
                //auto *var = c->lookup(tvt->name);
                //if(!var){
                //    c->errFlag = true;
                //    cerr << "Lookup for typevar " + tvt->name + " failed\n";
                //}

                //auto *type = extractTypeValue(var->tval);
                auto boundTv = TypedValue(tv.val, type);
                storeValue(c, boundTv);
                return;
            }
            case TT_Tuple:
            case TT_TaggedUnion:
            case TT_Data:
                storeTuple(c, tv);
                return;
            case TT_Type:
                *(void**)data = extractTypeValue(tv);
                return;
            case TT_FunctionList:
            case TT_Trait:
                break;
            case TT_Void:
                return;
        }

        error("Compile-time function argument must be constant", unknownLoc());
    }

    void AnteValue::printUnion(Compiler *c, std::ostream &os) const{
        auto *dt = try_cast<AnSumType>(type);
        char tag = castTo<char>();

        auto &tagty = dt->tags[tag];
        AnteValue((char*)data + 1, tagty).printTupleOrData(c, os);
    }

    void AnteValue::printTupleOrData(Compiler *c, std::ostream &os) const{
        auto *dt = try_cast<AnDataType>(type);
        if(dt){
            if(dt->typeTag == TT_TaggedUnion){
                printUnion(c, os);
                return;
            }else if(dt->name == "Str"){
                os << '"' << castTo<string>() << '"';
                return;
            }
        }
        os << '(';
        auto *agg = try_cast<AnAggregateType>(type);
        if(!agg){
            cerr << "printTupleOrData called on non-aggregate type\n";
            throw CtError();
        }

        char* dataptr = (char*)data;
        for(auto &ty : agg->extTys){
            auto size = ty->getSizeInBits(c);
            AnteValue(dataptr, ty).printCtVal(c, os);
            if(&ty != &agg->extTys.back()){
                cout << ", ";
            }
            dataptr += size.getVal() / 8;
        }
        putchar(')');
    }


    void AnteValue::printCtVal(Compiler *c, std::ostream &os) const{
        switch(type->typeTag){
            case TT_I8:  os << castTo<int8_t>(); break;
            case TT_I16: os << castTo<int16_t>(); break;
            case TT_I32: os << castTo<int32_t>(); break;
            case TT_I64: os << castTo<int64_t>(); break;
            case TT_Isz: os << castTo<signed long>(); break;
            case TT_U8:  os << castTo<uint8_t>(); break;
            case TT_U16: os << castTo<uint16_t>(); break;
            case TT_U32: os << castTo<uint32_t>(); break;
            case TT_U64: os << castTo<uint64_t>(); break;
            case TT_Usz: os << castTo<size_t>(); break;
            case TT_C8:  os << '\'' << castTo<char>() << '\''; break;
            case TT_C32: os << '\'' << castTo<wchar_t>() << '\''; break;  /** TODO: wchar_t is 16 bits on windows, not 32 */
            case TT_Bool: os << (castTo<bool>() ? "true" : "false"); break;
            case TT_F16: os << castTo<float>(); break;
            case TT_F32: os << castTo<float>(); break;
            case TT_F64: os << castTo<double>(); break;
            case TT_Ptr:
                if(try_cast<AnPtrType>(type)->extTy->typeTag == TT_C8){
                    os << '"' << castTo<char*>() << '"';
                }else{
                    os << castTo<void*>() << " -> ";
                    AnteValue(*(void**)data, try_cast<AnPtrType>(type)->extTy).printCtVal(c, os);
                }
                break;
            case TT_Array:
                os << " [...]";
                break;
            case TT_TaggedUnion:
            case TT_Data:
            case TT_Tuple:
                printTupleOrData(c, os);
                break;
            case TT_Type: cout << anTypeToStr(castTo<AnType*>()); break;
            case TT_Function: os << "fun @ " << castTo<void*>(); break;
            case TT_MetaFunction: os << "compiler-api function\n"; break;
            case TT_FunctionList: os << "function list\n"; break;
            case TT_Trait: os << "trait\n"; break;
            case TT_TypeVar: os << "?"; break; //compile-time value with unknown type, something went wrong.
            case TT_Void: os << "()"; break;
        }
    }


    void AnteValue::print(Compiler *c, std::ostream &os) const {
        cerr << anTypeToColoredStr(type) << ' ';
        printCtVal(c, os);
        puts("");
    }


    /*
    *  Converts a TypedValue to an llvm GenericValue
    *  - Assumes the Value* within the TypedValue is a Constant*
    */
    AnteValue::AnteValue(Compiler *c, vector<TypedValue> const& tvals,
            vector<unique_ptr<parser::Node>> const& exprs)
            : data(nullptr){

        for(auto &n : exprs){
            AnteVisitor::validate(c, n.get());
        }

        size_t size = 0;
        for(auto &tv : tvals){
            auto elemSize = tv.type->getSizeInBits(c);
            if(!elemSize){
                error("AnteValue: sizeerror: " + elemSize.getErr(), unknownLoc());
            }

            elemSize = elemSize.getVal() / 8;

            //we're reallocating the data manually here for the tuple
            //so storeValue must be used instead of allocAndStore
            void *dataBegin = realloc(data, size + elemSize.getVal());
            data = (char*)dataBegin + size;

            if(tv.type->hasModifier(Tok_Mut)){
                storeValue(c, findLastStore(c, tv));
            }else{
                storeValue(c, tv);
            }

            data = dataBegin;
            size += elemSize.getVal();
        }
    }

    AnteValue::AnteValue(Compiler *c, TypedValue const& val, unique_ptr<parser::Node> const& expr){
        *this = AnteValue(c, val, expr.get());
    }

    AnteValue::AnteValue(Compiler *c, TypedValue const& val, parser::Node *expr)
            : data(nullptr), type(val.type){

        AnteVisitor::validate(c, expr);

        if(val.type->hasModifier(Tok_Mut)){
            allocAndStoreValue(c, findLastStore(c, val));
        }else{
            allocAndStoreValue(c, val);
        }
    }
}
