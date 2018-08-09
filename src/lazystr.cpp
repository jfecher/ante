#include "lazystr.h"
using namespace ante;
using namespace std;

namespace ante {
    extern bool colored_output;

    ostream& operator<<(ostream& os, lazy_str str){
        if(colored_output)
            os << str.fmt << str.s << AN_CONSOLE_RESET;
        else
            os << str.s;
        return os;
    }

    ostream& operator<<(ostream& os, lazy_printer& lp){
        for(auto& str : lp.strs){
            os << str;
        }
        return os;
    }

    lazy_printer operator+(lazy_printer lp, lazy_str ls){
        lp.strs.push_back(ls);
        return lp;
    }

    lazy_printer operator+(lazy_str ls, lazy_printer lp){
        lp.strs.push_front(ls);
        return lp;
    }
}


lazy_str::lazy_str(string const& str) : s(str), fmt(AN_CONSOLE_RESET){}

lazy_str::lazy_str(string const& str, AN_COLOR_TYPE fg) : s(str), fmt(fg){}

lazy_str::lazy_str(const char* str) : s(str), fmt(AN_CONSOLE_RESET){}

lazy_printer::lazy_printer(const char* str){
    strs.push_back(str);
}

lazy_printer::lazy_printer(string str){
    strs.push_back(str);
}


#ifdef _WIN32
win_console_color getBackgroundColor() {
	CONSOLE_SCREEN_BUFFER_INFO csbi;
	GetConsoleScreenBufferInfo(GetStdHandle(STD_OUTPUT_HANDLE), &csbi);
	int a = csbi.wAttributes;
	return (win_console_color)((a / 16) % 16);
}

void setcolor(win_console_color foreColor, win_console_color backColor) {
	int fc = foreColor % 16;
	int bc = backColor % 16;

	unsigned short wAttr = ((unsigned)backColor << 4) | (unsigned)foreColor;
	SetConsoleTextAttribute(GetStdHandle(STD_OUTPUT_HANDLE), wAttr);
}

std::ostream& operator<<(std::ostream& os, win_console_color color) {
	os.flush();
	setcolor(color, getBackgroundColor());
	return os;
}
#endif
