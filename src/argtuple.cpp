#include "argtuple.h"
#include "types.h"

using namespace std;
using namespace llvm;

namespace ante {
    //ante function to convert between IEEE half and IEEE single
    //since c++ does not support an IEEE half value
#ifndef F16_BOOT
    extern "C" float f32_from_f16(uint16_t f);
#else
    float f32_from_f16(uint16_t f) {
        return f;
    }
#endif

    /**
     * Converts an ArgTuple into a typedValue.  If the type
     * cannot be converted or an error occurs, c's error flag
     * is set and a void literal is returned.
     */
    TypedValue convertToTypedValue(Compiler *c, ArgTuple &arg, AnType *tn){
        auto *data = arg.asRawData();
        switch(tn->typeTag){
            case TT_I8:              return TypedValue(c->builder.getInt8( *(uint8_t*) data), tn);
            case TT_I16:             return TypedValue(c->builder.getInt16(*(uint16_t*)data), tn);
            case TT_I32:             return TypedValue(c->builder.getInt32(*(uint32_t*)data), tn);
            case TT_I64:             return TypedValue(c->builder.getInt64(*(uint64_t*)data), tn);
            case TT_U8:              return TypedValue(c->builder.getInt8( *(uint8_t*) data), tn);
            case TT_U16:             return TypedValue(c->builder.getInt16(*(uint16_t*)data), tn);
            case TT_U32:             return TypedValue(c->builder.getInt32(*(uint32_t*)data), tn);
            case TT_U64:             return TypedValue(c->builder.getInt64(*(uint64_t*)data), tn);
            case TT_Isz:             return TypedValue(c->builder.getIntN( *(size_t*)  data, AN_USZ_SIZE), tn);
            case TT_Usz:             return TypedValue(c->builder.getIntN( *(size_t*)  data, AN_USZ_SIZE), tn);
            case TT_C8:              return TypedValue(c->builder.getInt8( *(uint8_t*) data), tn);
            case TT_C32:             return TypedValue(c->builder.getInt32(*(uint32_t*)data), tn);
            case TT_F16:             return TypedValue(ConstantFP::get(*c->ctxt, APFloat(f32_from_f16(*(uint16_t*)data))), tn);
            case TT_F32:             return TypedValue(ConstantFP::get(*c->ctxt, APFloat(*(float*)data)), tn);
            case TT_F64:             return TypedValue(ConstantFP::get(*c->ctxt, APFloat(*(double*)data)), tn);
            case TT_Bool:            return TypedValue(c->builder.getInt1(*(uint8_t*)data), tn);
            case TT_Tuple:           break;
            case TT_Array:           break;
            case TT_Ptr: {
                auto *cint = c->builder.getIntN(AN_USZ_SIZE, *(size_t*)data);
                auto *ty = c->anTypeToLlvmType(tn);
                return TypedValue(c->builder.CreateIntToPtr(cint, ty), tn);
            }
            case TT_Data:
            case TT_TypeVar:
            case TT_Function:
            case TT_TaggedUnion:
            case TT_MetaFunction:
            case TT_FunctionList:
            case TT_Type:
                break;
            case TT_Void:
                return c->getVoidLiteral();
        }

        c->errFlag = true;
        cerr << "ArgTuple: Unknown/Unimplemented TypeTag " << typeTagToStr(tn->typeTag) << endl;
        return c->getVoidLiteral();
    }
    
    
    /**
     * Stores a pointer value of a constant pointer type
     */
    void ArgTuple::storePtr(Compiler *c, TypedValue &tv){
        auto *ptrty = (AnPtrType*)tv.type;

        if(GlobalVariable *gv = dyn_cast<GlobalVariable>(tv.val)){
            Value *v = gv->getInitializer();
            if(ConstantDataArray *cda = dyn_cast<ConstantDataArray>(v)){
                char *cstr = strdup(cda->getAsString().str().c_str());
                data = cstr;
            }else{
                TypedValue tv = {v, ptrty->extTy};
                void **oldData = (void**)data;
                data = *oldData;
                storeValue(c, tv);
                *oldData = data;
                data = oldData;
            }
        }else if(ConstantExpr *ce = dyn_cast<ConstantExpr>(tv.val)){
            Instruction *in = ce->getAsInstruction();
            if(GetElementPtrInst *gep = dyn_cast<GetElementPtrInst>(in)){
                auto ptr = TypedValue(gep->getPointerOperand(), ptrty->extTy);
                storePtr(c, ptr);
            }
        }else{
            cerr << "error: unknown type given to getConstPtr, dumping\n";
            c->errFlag = true;
            tv.dump();
        }
    }
    
    
    void ArgTuple::storeTuple(Compiler *c, TypedValue &tup){
        //size_t i = 0;
        //for(auto *ty : ((AnAggregateType*)tup.type)->extTys){
        //    Value *extract = c->builder.CreateExtractValue(tup.val, i);
        //    auto field = TypedValue(extract, ty);
        //    ret.AggregateVal.push_back(typedValueToGenericValue(c, field));
        //    i++;
        //}
    }


    void ArgTuple::storeValue(Compiler *c, TypedValue &tv){
        auto *ci = dyn_cast<ConstantInt>(tv.val);
        auto *cf = dyn_cast<ConstantFP>(tv.val);

        TypeTag tt = tv.type->typeTag;
        switch(tt){
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
            case TT_F16:  *(float*)   data = cf->getValueAPF().convertToFloat(); return;
            case TT_F32:  *(float*)   data = cf->getValueAPF().convertToFloat(); return;
            case TT_F64:  *(double*)  data = cf->getValueAPF().convertToDouble(); return;
            case TT_Ptr:
            case TT_Array: storePtr(c, tv); return;
            case TT_Tuple: storeTuple(c, tv); return;
            case TT_TypeVar: {
                auto *tvt = (AnTypeVarType*)tv.type;
                auto *var = c->lookup(tvt->name);
                if(!var){
                    cerr << AN_ERR_COLOR << "error: " << AN_CONSOLE_RESET << "Lookup for typevar "+tvt->name+" failed";
                    c->errFlag = true;
                    return;
                }

                auto *type = extractTypeValue(var->tval);
                auto boundTv = TypedValue(tv.val, type);
                storeValue(c, boundTv);
                return;
            }
            case TT_Data:
            case TT_Function:
            case TT_TaggedUnion:
            case TT_MetaFunction:
            case TT_FunctionList:
            case TT_Type:
            case TT_Void:
                break;
        }

        cerr << AN_ERR_COLOR << "error: " << AN_CONSOLE_RESET << "Compile-time function argument must be constant.\n";
        c->errFlag = true;
    }


    /*
    *  Converts a TypedValue to an llvm GenericValue
    *  - Assumes the Value* within the TypedValue is a Constant*
    */
    ArgTuple::ArgTuple(Compiler *c, vector<TypedValue> &tvals)
            : data(nullptr){

        size_t size = 0;
        vector<AnType*> types;
        for(auto &tv : tvals){
            size_t elemSize = tv.type->getSizeInBits(c) / 8;

            void *dataBegin = realloc(data, size + elemSize);
            data = (char*)dataBegin + size;

            types.push_back(tv.type);
            storeValue(c, tv);
            data = dataBegin;
            size += elemSize;
        }
    }


    ArgTuple::ArgTuple(Compiler *c, TypedValue &val)
            : data(nullptr), tval(val){

        storeValue(c, val);
    }


    /**
     * Constructs an ArgTuple using the given pre-initialized data.
     */
    ArgTuple::ArgTuple(Compiler *c, void *d, AnType *t) : data(d){
        this->tval = convertToTypedValue(c, *this, t);
    }
}
