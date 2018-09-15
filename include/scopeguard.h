#ifndef AN_SCOPEGUARD_H
#define AN_SCOPEGUARD_H

#include <functional>
#include <list>

// While gcc/clang allow WARN_UNUSED to be used after a function's
// return type, MSVC mandates it be used before the type.
#if defined(__GNUC__) && (__GNUC__ >= 4)
#  define WARN_UNUSED __attribute__((warn_unused_result))
#elif defined(_MSC_VER) && (_MSC_VER >= 1700)
#  define WARN_UNUSED _Check_return_
#else
#  define WARN_UNUSED
#endif

namespace ante {

    /**
     * Perform a given action when this class is destructed.
     *
     * NOTE: If this class is used without assigning it to a variable,
     * its destructor will be run immediately and thus be useless.
     */
    template<typename T>
    class ScopeGuard {
        public:
            /** Named constructor so we can use WARN_UNUSED */
            WARN_UNUSED static ScopeGuard<T> guard(T &&fn){
                return {std::forward<T>(fn)};
            }

            ScopeGuard(ScopeGuard<T>&) = delete;

            ~ScopeGuard(){
                try{ f(); }
                catch(...){}
            }

        private:
            ScopeGuard(T &&defer_fn) :
                f{std::forward<T>(defer_fn)} {}

            T f;
    };

    /**
     * Temporarily set a variable to a given value.
     * The variable is reset to its original value on destruction.
     *
     * NOTE: If this class is used without assigning it to a variable,
     * its destructor will be run immediately and thus be useless.
     */
    template<typename T>
    class TemporarilySet {
        public:
            /** Named constructor so we can use WARN_UNUSED */
            WARN_UNUSED static TemporarilySet<T> set(T &var, T const& newVal){
                return {var, newVal};
            }

            ~TemporarilySet(){
                var = oldVal;
            }

            T getOldVal() const noexcept {
                return oldVal;
            }
        
        private:
            TemporarilySet(T &v, T const& newVal)
                    : var{v}, oldVal{v} {
                var = newVal;
            }

            T &var;
            T oldVal;
    };

    /** Auxilary functions for better class-type inference when using lambdas */
    template<typename T>
    WARN_UNUSED ScopeGuard<T> defer(T &&f){
        return ScopeGuard<T>::guard(std::forward<T>(f));
    }

    template<typename T>
    WARN_UNUSED TemporarilySet<T> tmpSet(T &var, T const& val){
        return TemporarilySet<T>::set(var, val);
    }

/** Auxilary functions for automatically naming the return values */
#define CONCAT_(x, y) x ## y
#define CONCAT(x, y) CONCAT_(x, y)
#define DEFER(f) auto CONCAT(__defer_, __line__) = ante::defer([&](){ f })
#define TMP_SET(var, val) auto CONCAT(__tmpset_, __line__) = ante::tmpSet((var), (val))
}

#endif
