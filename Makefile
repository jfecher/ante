vpath %.c src
vpath %.h include
vpath %.d obj

WARNINGS  := -Wall -Wpedantic -Wsign-compare
#LLVMFLAGS := `llvm-config --cppflags --libs Core BitWriter Passes Target --ldflags --system-libs`
LLVMFLAGS := `llvm-config --cppflags --libs All --ldflags --system-libs`

#                              v These macros are required when compiling with clang
CPPFLAGS  := -g -O2 -std=c++11 -D__STDC_CONSTANT_MACROS -D__STDC_LIMIT_MACROS $(WARNINGS)

PARSERSRC := src/parser.cpp
YACCFLAGS := -Lc++ -o$(PARSERSRC) --defines=include/yyparser.h

SRCDIRS  := src
SRCFILES := $(shell find $(SRCDIRS) -type f -name "*.cpp")

OBJFILES := $(patsubst src/%.cpp,obj/%.o,$(SRCFILES))
DEPFILES := $(OBJFILES:.o=.d)

.PHONY: ante new clean
.DEFAULT: ante

ante: obj obj/parser.o $(OBJFILES)
	@echo Linking...
	@$(CXX) $(OBJFILES) $(CPPFLAGS) $(LLVMFLAGS) -o ante

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
	-@$(RM) obj/*.o obj/*.d
