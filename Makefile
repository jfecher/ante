vpath %.c src
vpath %.h include
vpath %.d obj

WARNINGS  := -Wall -Wpedantic -Wsign-compare

LLVMCFG := $(shell if command -v llvm-config-4.0; then echo 'llvm-config-4.0'; else echo 'llvm-config'; fi)
LLVMFLAGS := `$(LLVMCFG) --cppflags --cflags --link-static --libs Core mcjit interpreter native BitWriter Passes Target --ldflags --system-libs` -lffi

LIBDIR := /usr/include/ante
LIBFILES := $(shell find stdlib -type f -name "*.an")

#                              v These macros are required when compiling with clang
CPPFLAGS  := -g -O0 -std=c++11 `$(LLVMCFG) --cppflags --cflags` $(WARNINGS)

PARSERSRC := src/parser.cpp
YACCFLAGS := -Lc++ -o$(PARSERSRC) --defines=include/yyparser.h

SRCDIRS  := src
SRCFILES := $(shell find $(SRCDIRS) -type f -name "*.cpp")

OBJFILES := $(patsubst src/%.cpp,obj/%.o,$(SRCFILES))

ANSRCFILES := $(shell find $(SRCDIRS) -type f -name "*.an")
ANOBJFILES := $(patsubst src/%.an,obj/%.ao,$(ANSRCFILES))

#If src/parser.cpp is still present, remove it from objfiles so as to not double-compile it
OBJFILES := $(patsubst obj/parser.o,,$(OBJFILES))

DEPFILES := $(OBJFILES:.o=.d)

.PHONY: new clean stdlib
.DEFAULT: ante

ante: obj obj/parser.o $(OBJFILES) $(ANOBJFILES)
	@echo Linking...
	$(CXX) obj/parser.o $(OBJFILES) $(ANOBJFILES) $(CPPFLAGS) $(LLVMFLAGS) -o ante

#export the stdlib to /usr/include/ante
#this is the only part that requires root permissions
stdlib: $(LIBFILES) Makefile
	@if [ `id -u` -eq 0 ]; then                                                      \
	    echo 'Exporting $< to $(LIBDIR)...';                                         \
	    mkdir -p $(LIBDIR);                                                          \
	    cp stdlib/*.an $(LIBDIR);                                                    \
	 else                                                                            \
	    printf '\033[;31mMust run with root permissions to export stdlib!\033[;m\n'; \
		echo 'To export stdlib run:';                                                \
		echo -e '\n$$ sudo make stdlib\n';                                           \
		exit 1;                                                                      \
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
	$(CXX) $(CPPFLAGS) -MMD -MP -Iinclude -c $< -o $@

obj/%.ao: src/%.an Makefile | obj
	@if command -v ante > /dev/null; then \
	     echo Compiling $@...; \
	     ante -lib -c $< -o $@;\
	 fi

obj/parser.o: src/syntax.y Makefile
	@echo Generating parser...
	@$(YACC) $(YACCFLAGS) src/syntax.y
	@-mv src/*.hh include
	@$(CXX) $(CPPFLAGS) -MMD -MP -Iinclude -c $(PARSERSRC) -o $@

#remove all intermediate files
clean:
	-@$(RM) obj/*.o obj/*.d include/*.hh include/yyparser.h src/parser.cpp
