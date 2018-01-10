#ifndef AN_RESULT_H
#define AN_RESULT_H

#include <iostream>

namespace ante {

    /** Holds either the templated value or an error string */
    template<typename T, typename E>
    class Result {
    private:
        bool isVal;

        union U {
            T val;
            E error;

            U(){}
            U(T v) : val(v){}
            U(E e) : error(e){}
            ~U(){}
        } valOrErr;

    public:
        Result(bool b, T v) : isVal(b), valOrErr(v){}
        Result(bool b, E v) : isVal(b), valOrErr(v){}

        Result(T v) : isVal(true), valOrErr(v) {}
        Result(E e) : isVal(false), valOrErr(e) {}

        Result(Result<T, E> &r) : isVal(r.isVal){
            if(r) valOrErr.val = r.getVal();
            else valOrErr.error = r.getErr();
        }

        Result(Result<T, E> &&r) : isVal(r.isVal){
            if(r) valOrErr.val = r.getVal();
            else valOrErr.error = r.getErr();
            r.isVal = false;
            r.valOrErr.error = "";
        }

        Result<T, E>& operator=(const Result<T, E>& rhs){
            if(rhs.isVal){
                isVal = true;
                valOrErr.val = rhs.getVal();
            }else{
                isVal = false;
                valOrErr.error = rhs.getErr();
            }
            return *this;
        }

        ~Result(){
            //if(isVal){
            //    if(!std::is_trivially_destructible<T>())
            //        valOrErr.val::T::~T();
            //}else{
            //    if(!std::is_trivially_destructible<E>())
            //        valOrErr.error::E::~E();
            //}
        }

        explicit operator bool() const{
            return isVal;
        }

        bool operator !() const{
            return !isVal;
        }

        T getVal() const{
            if(isVal)
                return valOrErr.val;
            
            std::cerr << "getVal() called on Result without a value!" << std::endl;
            exit(1);
        }

        std::string getErr() const {
            if(!isVal)
                return valOrErr.error;
            
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
