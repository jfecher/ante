#ifndef ANTE_TARGET_H
#define ANTE_TARGET_H

//Triple: Arch, Vendor, OS, Env=0

/*
 *  Evaluate several macros to determine host OS and architecture.
 */
#if defined __FreeBSD__ || defined __NetBSD__ || defined __OpenBSD__ || \
        defined __bsdi__ || defined __DragonFly__ || \
        (defined __FreeBSD_kernel__ && __GLIBC__)

#  define AN_NATIVE_OS "FreeBSD"
#  ifndef AN_LIB_DIR
#    define AN_LIB_DIR "/usr/include/ante/"
#  endif
#
#  define AN_EXEC_STR "./"
#endif


#if defined __gnu_linux__ || defined __linux__ || defined __CYGWIN__
#  define AN_NATIVE_OS "linux"
#  ifndef AN_LIB_DIR
#    define AN_LIB_DIR "/usr/include/ante/"
#  endif
#  define AN_EXEC_STR "./"
#endif


//          MAC OS 9             MAC OS 9              MAC OS X
#if defined macintosh || defined Macintosh || (defined __APPLE__ && \
        defined __MACH__)

#  define AN_NATIVE_OS "darwin"
#  define AN_NATIVE_VENDOR "apple"
#  ifndef AN_LIB_DIR
#    define AN_LIB_DIR "/usr/include/ante/"
#  endif
#  define AN_EXEC_STR "./"
#endif


/*
 *  _WIN32 is defined for both 32-bit and 64-bit windows environments,
 *  so the check for _WIN32 must come before the check for _WIN64
 */
#ifdef _WIN32
#  define AN_NATIVE_OS "win32"
#  define AN_NATIVE_VENDOR "pc"
#  ifndef AN_LIB_DIR
#    define AN_LIB_DIR "C:\\Program Files (x86)\\Ante\\"
#  endif
#  define AN_EXEC_STR ".\\"
#  define AN_LINKER "C:\\MinGW\\bin\\gcc.exe"
#endif



//Determine Vendor
//TODO: implement

//Determine Arch
#if defined __amd64__ || defined __amd64 || defined __x86_64__ || \
    defined __x86_64 || defined _M_X64 || defined _M_AMD64

#  define AN_NATIVE_ARCH "x86_64"
#endif

#if defined i386 || defined __i386__ || defined __i686__ || defined _M_I86 || \
    defined _M_IX86 || defined __i386 || defined __i386__ || defined __i586__

#ifdef _WIN64
#  define AN_NATIVE_ARCH "x86_64"
#else
#  define AN_NATIVE_ARCH "i686"
#endif

#endif


#if defined __arm__ || defined __TARGET_ARCH_ARM || defined _ARM || \
    defined _M_ARM || defined _M_ARMT || defined __arm

#  define AN_NATIVE_ARCH "arm"
#endif


#if defined __mips__ || defined __mips || defined __MIPS__
#  define AN_NATIVE_ARCH "mips"
#endif


#if defined __ppc__ || defined _M_PPC
#  if defined __ppc64 || defined __PPC64__
#    define AN_NATIVE_ARCH "ppc64"
#  else
#    define AN_NATIVE_ARCH "ppc"
#  endif
#endif


#if defined __sparc__ || defined __sparc
#  if defined __sparc_v9__ || defined __sparcv9
#    define AN_NATIVE_ARCH "sparcv9"
#  else
#    define AN_NATIVE_ARCH "sparc"
#  endif
#endif


//if any of the above macros are not defined, mark them as so.
#ifndef AN_NATIVE_OS
#  define AN_NATIVE_OS "UnknownOS"
#  ifndef AN_LIB_DIR
#    define AN_LIB_DIR "/usr/include/ante/"
#  endif
#endif

#ifndef AN_NATIVE_VENDOR
#  define AN_NATIVE_VENDOR "UnknownVendor"
#endif

#ifndef AN_NATIVE_ARCH
#  define AN_NATIVE_ARCH "UnknownArch"
#endif

#ifndef AN_LINKER
#  define AN_LINKER "gcc"
#endif

#ifndef AN_EXEC_STR
#  define AN_EXEC_STR "./"
#endif


#ifndef AN_TARGET_TRIPLE
#  define AN_TARGET_TRIPLE AN_NATIVE_ARCH "-" AN_NATIVE_VENDOR "-" AN_NATIVE_OS
#endif


#ifndef _WIN32
#  define AN_CONSOLE_COLOR_RED "\033[;31m"
#  define AN_CONSOLE_RESET "\033[;m"
#  define AN_CONSOLE_ITALICS "\033[;3m"
#  define AN_CONSOLE_BOLD "\033[;1m"
#else
#  define AN_CONSOLE_COLOR_RED win_console_color::red
#  define AN_CONSOLE_RESET win_console_color::white
#  define AN_CONSOLE_ITALICS ""
#  define AN_CONSOLE_BOLD ""

#define WIN32_LEAN_AND_MEAN
#include <windows.h>

//thanks to Eklavya Sharma: http://www.cplusplus.com/articles/2ywTURfi/
namespace ante {
	enum win_console_color {
		black = 0, darkblue = 1, darkgreen = 2, darkcyan = 3, darkred = 4, darkmagenta = 5, darkyellow = 6, darkwhite = 7,
		gray = 8,      blue = 9,     green = 10,    cyan = 11,    red = 12,    magenta = 13,    yellow = 14,    white = 15
	};
}

win_console_color getBackgroundColor();
void setcolor(win_console_color foreColor, win_console_color backColor);
std::ostream& operator<<(std::ostream& os, win_console_color color);

#endif

#endif
