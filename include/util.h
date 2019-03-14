#ifndef AN_UTIL_H
#define AN_UTIL_H

#include "error.h"

namespace ante {
    template<typename F>
    void tryTo(F f){
        try{
            f();
        }catch(CtError){}
    }

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
}

#endif
