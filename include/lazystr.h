#ifndef AN_LAZYSTR_H
#define AN_LAZYSTR_H

#include <string>
#include <ostream>
#include <list>


//define colors for windows and other OS
#ifndef _WIN32
#  define AN_CONSOLE_RESET "\033[;m"
#  define AN_CONSOLE_ITALICS "\033[;3m"
#  define AN_CONSOLE_BOLD "\033[;1m"

#  define AN_ERR_COLOR "\033[;31m"  //red
#  define AN_WARN_COLOR "\033[;33m"  //yellow
#  define AN_NOTE_COLOR "\033[;35m"  //purple
#  define AN_TYPE_COLOR "\033[;34m" //cyan
#  define AN_COLOR_TYPE const char*

//older versions of windows do not understand escape sequences, use winAPI instead
#else
#  define AN_CONSOLE_RESET win_console_color::darkwhite
#  define AN_CONSOLE_ITALICS ""
#  define AN_CONSOLE_BOLD ""

#  define AN_ERR_COLOR win_console_color::red
#  define AN_WARN_COLOR win_console_color::yellow
#  define AN_NOTE_COLOR win_console_color::magenta
#  define AN_TYPE_COLOR win_console_color::cyan
#  define AN_COLOR_TYPE win_console_color

#include <windows.h>

//thanks to Eklavya Sharma: http://www.cplusplus.com/articles/2ywTURfi/
namespace ante {
	enum win_console_color {
		black = 0, darkblue = 1, darkgreen = 2, darkcyan = 3, darkred = 4, darkmagenta = 5, darkyellow = 6, darkwhite = 7,
		gray = 8,      blue = 9,     green = 10,    cyan = 11,    red = 12,    magenta = 13,    yellow = 14,    white = 15
	};
}

ante::win_console_color getBackgroundColor();

//NOTE: this function sets the console's foreground and background color as it cannot
//      be contained within the string itself on windows, thus necessitating the lazy_str
void setcolor(ante::win_console_color foreColor, ante::win_console_color backColor);
std::ostream& operator<<(std::ostream& os, ante::win_console_color color);

#endif


//define a basic lazy_str type to contain the string
//to print and the OS dependent color/formatting to print it in
namespace ante {
    struct lazy_str {
        std::string s;
        AN_COLOR_TYPE fmt;

        lazy_str(const char* str);
        lazy_str(std::string str);
    };
}

std::ostream& operator<<(std::ostream& os, ante::lazy_str str);


//due to each string's coloring lazy_strs cannot be concatenated, so
//define a wrapper class that can
namespace ante {
    struct lazy_printer {
        std::list<lazy_str> strs;

        lazy_printer(const char* str);
        lazy_printer(std::string str);
    };
}

ante::lazy_printer operator+(ante::lazy_printer, ante::lazy_str);
ante::lazy_printer operator+(ante::lazy_str, ante::lazy_printer);

std::ostream& operator<<(std::ostream&, ante::lazy_printer&);

#endif
