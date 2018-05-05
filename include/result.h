#ifndef AN_RESULT_H
#define AN_RESULT_H

#include <iostream>

namespace ante {

    constexpr size_t cmax(size_t a, size_t b){
        return a > b ? a : b;
    }

    /** Holds either the templated value or an error string */
    template<typename T, typename E>
    class Result {
    private:
        static constexpr size_t size = cmax(sizeof(T), sizeof(E));
        static constexpr size_t align = cmax(alignof(T), alignof(E));

        using Box = typename std::aligned_storage<size, align>::type;

        bool isVal;
        Box valOrErr;

    public:
        Result(bool b, T v) : isVal(b) {
            ::new (&valOrErr) T(v);
        }

        Result(bool b, E v) : isVal(b) {
            ::new (&valOrErr) E(v);
        }

        Result(T t) : isVal(true) {
            ::new (&valOrErr) T(t);
        }

        Result(E e) : isVal(false) {
            ::new (&valOrErr) E(e);
        }

        //Result(const Result<T, E> &r) : isVal(r.isVal), valOrErr(r) {}

        Result(Result<T, E> &&r) : isVal(r.isVal) {
            if(isVal) ::new (&valOrErr) T((*reinterpret_cast<T*>(&r.valOrErr)));
            else ::new (&valOrErr) E(move(*reinterpret_cast<E*>(&r.valOrErr)));
        }

        Result<T, E>& operator=(const Result<T, E>& rhs){
            if(rhs.isVal){
                isVal = true;
                auto t = rhs.getVal();
                valOrErr = *reinterpret_cast<Box*>(&t);
            }else{
                isVal = false;
                auto e = rhs.getErr();
                valOrErr = *reinterpret_cast<Box*>(&e);
            }
            return *this;
        }

        ~Result(){
            if(isVal) reinterpret_cast<T*>(&valOrErr)->T::~T();
            else reinterpret_cast<E*>(&valOrErr)->E::~E();
        }

        bool operator==(const Result<T, E>& rhs){
            if(rhs.isVal != isVal) return false;

            if(isVal) return getVal() == rhs.getVal();
            else return getErr() == rhs.getErr();
        }

        explicit operator bool() const{
            return isVal;
        }

        bool operator !() const{
            return !isVal;
        }

        T getVal() const{
            if(isVal)
                return *reinterpret_cast<const T*>(&valOrErr);

            std::cerr << "getVal() called on Result without a value!" << std::endl;
            exit(1);
        }

        E getErr() const {
            if(!isVal)
                return *reinterpret_cast<const E*>(&valOrErr);

            std::cerr << "getVal() called on Result without a value!" << std::endl;
            exit(1);
        }
    };

    //separate named 'constructors' in case T = E
    template<typename T, typename E>
    Result<T,E> success(T val){
        return Result<T,E>(true, val);
    }

    template<typename T, typename E>
    Result<T,E> failure(E errorMsg){
        return Result<T,E>(false, errorMsg);
    }
}

#endif
