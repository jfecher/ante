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

            U(){ memset(this, 0, sizeof(U)); }
            U(T v) : val(v){}
            U(E e) : error(e){}
            ~U(){}

            bool initialized() const {
                for(unsigned i = 0; i < sizeof(U); i++){
                    if(((char*)this)[i]) return false;
                }
                return true;
            }

            void deleteVal(){
                if(initialized() && !std::is_trivially_destructible<T>())
                    val.T::~T();
            }

            void deleteErr(){
                if(initialized() && !std::is_trivially_destructible<E>())
                    error.E::~E();
            }
        } valOrErr;

    public:
        Result(bool b, T v) : isVal(b), valOrErr(v){}
        Result(bool b, E v) : isVal(b), valOrErr(v){}

        Result(T v) : isVal(true), valOrErr(v) {}
        Result(E e) : isVal(false), valOrErr(e) {}

        Result(const Result<T, E> &r) : isVal(r.isVal) {
            if(r.isVal) valOrErr.val = r.valOrErr.val;
            else valOrErr.error = r.valOrErr.error;
        }

        Result(Result<T, E> &&r) : isVal(r.isVal) {
            if(r.isVal) valOrErr.val = (r.valOrErr.val);
            else valOrErr.error = move(r.valOrErr.error);
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
            if(isVal) valOrErr.deleteVal();
            else valOrErr.deleteErr();
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
                return valOrErr.val;
            
            std::cerr << "getVal() called on Result without a value!" << std::endl;
            exit(1);
        }

        E getErr() const {
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
