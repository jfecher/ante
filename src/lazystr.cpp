#include "lazystr.h"
using namespace ante;
using namespace std;

namespace ante {
    extern bool colored_output;

    ostream& operator<<(ostream& os, lazy_str const& str) {
        if (colored_output)
            os << str.fmt << str.s << AN_CONSOLE_RESET;
        else
            os << str.s;
        return os;
    }

    ostream& operator<<(ostream& os, lazy_printer const& lp) {
        for (auto& str : lp.strs) {
            os << str;
        }
        return os;
    }

    lazy_printer operator+(lazy_printer const& lp, lazy_str const& ls) {
        lazy_printer ret = lp;
        ret.strs.push_back(ls);
        return ret;
    }

    lazy_printer operator+(lazy_str const& ls, lazy_printer const& lp) {
        lazy_printer ret = lp;
        ret.strs.push_front(ls);
        return ret;
    }

    lazy_printer operator+(lazy_str const& l, lazy_str const& r){
        lazy_printer lp;
        lp.strs.push_back(l);
        lp.strs.push_back(r);
        return lp;
    }

    lazy_printer operator+(lazy_printer const& l, lazy_printer const& r){
        lazy_printer ret = l;
        for(lazy_str const& str : r.strs){
            ret.strs.push_back(str);
        }
        return ret;
    }

    lazy_printer operator+(lazy_printer const& l, char r){
        lazy_printer ret = l;
        ret.strs.emplace_back(r);
        return ret;
    }

    lazy_printer& operator+=(lazy_printer &l, lazy_printer const& r) {
        for(auto &str : r.strs){
            l.strs.push_back(str);
        }
        return l;
    }

    lazy_str::lazy_str(string const& str) : s(str), fmt(AN_CONSOLE_RESET) {}

    lazy_str::lazy_str(string const& str, AN_COLOR_TYPE fg) : s(str), fmt(fg) {}

    lazy_str::lazy_str(const char* str) : s(str), fmt(AN_CONSOLE_RESET) {}

    lazy_str::lazy_str(char c) : s(1, c), fmt(AN_CONSOLE_RESET) {}

    lazy_printer::lazy_printer(const char* str) {
        strs.push_back(str);
    }

    lazy_printer::lazy_printer(string const& str) {
        strs.push_back(str);
    }

    lazy_printer::lazy_printer(lazy_str const& str) {
        strs.push_back(str);
    }

    lazy_printer::lazy_printer(char c) {
        strs.emplace_back(c);
    }

    lazy_printer::lazy_printer(lazy_printer const& r) {
        strs = r.strs;
    }


#ifdef _WIN32
    win_console_color getBackgroundColor() {
        CONSOLE_SCREEN_BUFFER_INFO csbi;
        GetConsoleScreenBufferInfo(GetStdHandle(STD_OUTPUT_HANDLE), &csbi);
        int a = csbi.wAttributes;
        return (win_console_color)((a / 16) % 16);
    }

    void setcolor(win_console_color foreColor, win_console_color backColor) {
        int fc = (int)foreColor % 16;
        int bc = (int)backColor % 16;

        unsigned short wAttr = ((unsigned)backColor << 4) | (unsigned)foreColor;
        SetConsoleTextAttribute(GetStdHandle(STD_OUTPUT_HANDLE), wAttr);
    }

    std::ostream& operator<<(std::ostream& os, win_console_color color) {
        if (colored_output) {
            os.flush();
            setcolor(color, getBackgroundColor());
        }
        return os;
    }
#endif
}
