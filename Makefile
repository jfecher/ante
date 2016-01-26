vpath %.c src
vpath %.h include
vpath %.d obj

WARNINGS  := -Wall -Wpedantic
LLVMFLAGS := `llvm-config --cppflags --libs Core BitWriter Passes --ldflags --system-libs`
CPPFLAGS  := -g -O2 -std=c++11 $(WARNINGS) $(LLVMFLAGS)
YACCFLAGS := -Lc -osrc/parser.c

SRCDIRS  := src
SRCFILES := $(shell find $(SRCDIRS) -type f -name "*.cpp")

OBJFILES := $(patsubst src/%.cpp,obj/%.o,$(SRCFILES))
DEPFILES := $(OBJFILES:.o=.d)

.PHONY: ante new clean
.DEFAULT: ante

ante: obj/parser.o $(OBJFILES)
	@echo Linking...               # | do not move!
	@							   # | for some reason llvm requires
	@						       # | its flags to be right before the -o
	@							   # V and after each object file
	@$(CXX) $(OBJFILES) obj/parser.o $(CPPFLAGS) -o ante

new: clean ante

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
	@$(CXX) $(CPPFLAGS) -MMD -MP -Iinclude -c src/parser.c -o $@

clean:
	-@$(RM) obj/*.o obj/*.d ante
