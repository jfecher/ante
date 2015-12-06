vpath %.c src
vpath %.h include
vpath %.d obj

WARNINGS := -Wall
LLVMFLAGS := `llvm-config --cppflags --libs all --ldflags --system-libs`
CPPFLAGS := -g -O2 -std=c++11 $(WARNINGS) $(LLVMFLAGS)
YACCFLAGS := -Lc -osrc/parser.cpp

SRCDIRS := src
SRCFILES := $(shell find $(SRCDIRS) -type f -name "*.cpp")

OBJFILES := $(patsubst src/%.cpp,obj/%.o,$(SRCFILES))
DEPFILES := $(OBJFILES:.o=.d)

.PHONY: ante new clean
.DEFAULT: ante

ante: $(OBJFILES) obj/parser.o
	@echo Linking...
	@$(CXX) $(OBJFILES) obj/parser.o $(CPPFLAGS) -o ante

new: clean ante

obj: 
	@mkdir -p obj

obj/%.o: src/%.cpp Makefile | obj
	@echo Compiling $@...
	@$(CXX) $(CPPFLAGS) -MMD -MP -Iinclude -c $< -o $@

obj/parser.o: src/syntax.y
	@echo Generating parser...
	@$(YACC) $(YACCFLAGS) src/syntax.y
	@$(CC) -g -O2 -MMD -MP -Iinclude -c src/parser.cpp -o $@

clean:
	-@$(RM) obj/*.o obj/*.d ante
