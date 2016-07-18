vpath %.c src
vpath %.h include
vpath %.d obj

WARNINGS  := -Wall -Wpedantic -Wsign-compare
#LLVMFLAGS := `llvm-config --cppflags --libs Core BitWriter Passes Target --ldflags --system-libs`
LLVMFLAGS := `llvm-config --cppflags --libs All --ldflags --system-libs`

LIBDIR := /usr/include/ante
LIBFILES := $(shell find stdlib -type f -name "*.an")

#                              v These macros are required when compiling with clang
CPPFLAGS  := -g -O2 -std=c++11 -D__STDC_CONSTANT_MACROS -D__STDC_LIMIT_MACROS $(WARNINGS)

PARSERSRC := src/parser.cpp
YACCFLAGS := -Lc++ -o$(PARSERSRC) --defines=include/yyparser.h

SRCDIRS  := src
SRCFILES := $(shell find $(SRCDIRS) -type f -name "*.cpp")

OBJFILES := $(patsubst src/%.cpp,obj/%.o,$(SRCFILES))

#If src/parser.cpp is still present, remove it from objfiles so as to not double-compile it
OBJFILES := $(patsubst obj/parser.o,,$(OBJFILES))

DEPFILES := $(OBJFILES:.o=.d)

.PHONY: ante new clean stdlib
.DEFAULT: ante

ante: stdlib obj obj/parser.o $(OBJFILES)
	@echo Linking...
	@$(CXX) obj/parser.o $(OBJFILES) $(CPPFLAGS) $(LLVMFLAGS) -o ante

#export the stdlib to /usr/share/Ante
#this is the only part that requires root permissions
stdlib: $(LIBFILES) Makefile
	@if [ `id -u` -eq 0 ]; then                                  \
	    echo 'Exporting stdlib to $(LIBDIR)...';                 \
	    mkdir -p $(LIBDIR);                                      \
	    cp stdlib/*.an $(LIBDIR);                                \
	 else                                                        \
	    echo 'Must run with root permissions to export stdlib!'; \
		echo 'To export stdlib run:';                            \
		echo '';                                                 \
		echo '$$ sudo make stdlib';                               \
		echo '';                                                 \
	 fi

new: clean ante

#create the obj folder if it is not present
obj: 
	@mkdir -p obj

debug_parser:
	@echo Generating parser.output file...
	@$(YACC) $(YACCFLAGS) -v src/syntax.y


obj/%.o: src/%.cpp Makefile | obj
	@echo Compiling $@...
	@$(CXX) $(CPPFLAGS) -MMD -MP -Iinclude -c $< -o $@

obj/parser.o: src/syntax.y Makefile
	@echo Generating parser...
	@$(YACC) $(YACCFLAGS) src/syntax.y
	@-mv src/*.hh include
	@$(CXX) $(CPPFLAGS) -MMD -MP -Iinclude -c $(PARSERSRC) -o $@

#remove all intermediate files
clean:
	-@$(RM) obj/*.o obj/*.d include/*.hh include/yyparser.h src/parser.cpp
