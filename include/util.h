#ifndef AN_UTIL_H
#define AN_UTIL_H

#include "error.h"
#include "compiler.h"
#include <memory>

namespace ante {
    template<typename F>
    void tryTo(F f){
        try{
            f();
        }catch(CtError){}
    }

    std::ostream& operator<<(std::ostream &out, parser::Node &n);

    /** @brief Create a vector with a capacity of at least cap elements. */
    template<typename T> std::vector<T> vecOf(size_t cap){
        std::vector<T> vec;
        vec.reserve(cap);
        return vec;
    }

    /** Perform an element count in linear time for lists or similar data structures. */
    template<typename T>
    size_t count(T& collection){
        size_t i = 0;
        for(auto &_unused : collection){
            i++;
        }
        return i;
    }

    /** Given [a, b, ..., z], f: return [f(a), f(b), ..., f(z)] */
    template<typename T, typename F,
        typename U = typename std::decay<typename std::result_of<F&(typename std::vector<T>::const_reference)>::type>::type>
    std::vector<U> applyToAll(std::vector<T> const& vec, F f){
        std::vector<U> result;
        result.reserve(vec.size());
        for(const auto& elem : vec){
            result.emplace_back(f(elem));
        }
        return result;
    }

    /**
     * Similar to applyToAll, but works on any iterable object and
     * as a result cannot efficiently reserve() the resulting vector.
     */
    template<typename T, typename F,
        typename U = typename std::result_of<F&(typename T::const_reference)>::type>
    std::vector<U> collect(T const& iterable, F f){
        std::vector<U> result;
        for(const auto& elem : iterable){
            result.emplace_back(f(elem));
        }
        return result;
    }

    template<typename T, typename E>
    typename T::const_iterator find(T const& collection, E const& elem){
        auto it = collection.cbegin();
        auto end = collection.cend();
        for(; it != end; ++it){
            if(*it == elem){
                return it;
            }
        }
        return end;
    }

    template<typename T, typename F>
    typename T::const_iterator find_if(T const& collection, F fn){
        auto it = collection.cbegin();
        auto end = collection.cend();
        for(; it != end; ++it){
            if(fn(*it)){
                return it;
            }
        }
        return end;
    }

    template<typename T, typename F>
    bool remove_if(T& collection, F fn){
        auto it = collection.begin();
        auto end = collection.end();
        for(; it != end; ++it){
            if(fn(*it)){
                collection.erase(it);
                return true;
            }
        }
        return false;
    }

    template<typename E, typename T>
    bool in(E const& elem, T const& collection){
        return ante::find(collection, elem) != collection.cend();
    }

    template<typename F, typename T>
    bool any(T const& collection, F fn){
        return ante::find_if(collection, fn) != collection.cend();
    }

    template<typename T>
    std::ostream& operator<<(std::ostream &o, std::vector<T> const& vec){
        o << '[';
        for(auto &elem : vec){
            o << elem;
            if(&elem != &vec.back())
                o << elem << ", ";
        }
        o << ']';
    }

    void print(parser::Node *n);
    void print(std::shared_ptr<parser::Node> const& n);
    void print(std::unique_ptr<parser::Node> const& n);
}

#endif
