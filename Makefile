vpath %.c src
vpath %.h include
vpath %.d obj

WARNINGS  := -Wall -Wpedantic -Wsign-compare

#Required for ubuntu and other distros with outdated llvm packages
LLVMCFG := $(shell if command -v llvm-config-5.0 >/dev/null 2>&1; then echo 'llvm-config-5.0'; else echo 'llvm-config'; fi)
LLVMFLAGS := `$(LLVMCFG) --cflags --cppflags --libs Core mcjit interpreter native BitWriter Passes Target --ldflags --system-libs` -lffi

# Change this to change the location of the stdlib
# Expects the stdlib/*.an to be located in this dirirectory
ANLIBDIR := "\"$(shell pwd)/stdlib/\""


LIBFILES := $(shell find stdlib -type f -name "*.an")

CPPFLAGS  := -g -std=c++11 `$(LLVMCFG) --cflags --cppflags` -O0 $(WARNINGS)

PARSERSRC := src/parser.cpp
YACCFLAGS := -Lc++ -o$(PARSERSRC) --defines=include/yyparser.h

SRCDIRS  := src
SRCFILES := $(shell find $(SRCDIRS) -type f -name "*.cpp")

OBJFILES := $(patsubst src/%.cpp,obj/%.o,$(SRCFILES))

DEPFILES := $(OBJFILES:.o=.d)

#If src/parser.cpp is still present, remove it from objfiles so as to not double-compile it
OBJFILES := $(patsubst obj/parser.o,,$(OBJFILES))

ANSRCFILES := $(shell find $(SRCDIRS) -type f -name "*.an")
ANOBJFILES := $(patsubst src/%.an,obj/%.ao,$(ANSRCFILES))

ITESTFILES := $(shell find 'tests/integration' -maxdepth 1 -type f -name "*.an")
UTESTFILES := $(shell find 'tests/unit' -maxdepth 1 -type f -name "*.cpp")

UOBJFILES := $(patsubst tests/unit/%.cpp,obj/unit/%.o,$(UTESTFILES))

.PHONY: new clean stdlib
.DEFAULT: ante

ante: obj obj/parser.o $(OBJFILES) $(ANOBJFILES)
	@if [ ! -e obj/f16.ao ]; then $(MAKE) bootante; fi
	@echo Linking...
	@$(CXX) obj/parser.o $(OBJFILES) $(ANOBJFILES) $(LLVMFLAGS) -o ante


run: ante
	./ante


bootante: obj obj/parser.o $(OBJFILES) $(ANOBJFILES)
	@echo Bootstrapping f16.ao...
	@echo Compiling argtuple.o...
	@$(CXX) -DAN_LIB_DIR=$(ANLIBDIR) -DF16_BOOT $(CPPFLAGS) -MMD -MP -Iinclude -c src/argtuple.cpp -o obj/argtuple.o
	@echo Compiling compiler.o...
	@$(CXX) -DAN_LIB_DIR=$(ANLIBDIR) -DF16_BOOT $(CPPFLAGS) -MMD -MP -Iinclude -c src/compiler.cpp -o obj/compiler.o
	@echo Linking bootante...
	@$(CXX) obj/parser.o $(OBJFILES) $(LLVMFLAGS) -o bootante
	@./bootante -lib -c src/f16.an -o obj/f16.ao
	@rm obj/operator.o obj/compiler.o
	@$(MAKE) obj/operator.o obj/compiler.o


new: clean ante

#create the obj folder if it is not present
obj:
	@mkdir -p obj

obj/unit:
	@mkdir -p obj/unit

debug_parser:
	@echo Generating parser.output file...
	$(YACC) $(YACCFLAGS) -v src/syntax.y


obj/%.o: src/%.cpp Makefile | obj
	@echo Compiling $@...
	@$(CXX) -DAN_LIB_DIR=$(ANLIBDIR) $(CPPFLAGS) -MMD -MP -Iinclude -c $< -o $@

obj/%.ao: src/%.an Makefile | obj
	@if command -v ./ante >/dev/null 2>&1; then \
	     echo Compiling $@...; \
	     ./ante -lib -c $< -o $@;\
	 fi

obj/parser.o: src/syntax.y Makefile
	@echo Generating parser...
	@$(YACC) $(YACCFLAGS) src/syntax.y
	@-mv src/*.hh include
	@$(CXX) -DAN_LIB_DIR=$(ANLIBDIR) $(CPPFLAGS) -MMD -MP -Iinclude -c $(PARSERSRC) -o $@


test: unittest integrationtest



obj/unit/%.o: tests/unit/%.cpp Makefile | obj/unit
	@echo Compiling unittest $@...
	@$(CXX) -DAN_LIB_DIR=$(ANLIBDIR) $(CPPFLAGS) -MMD -MP -Iinclude -c $< -o $@


unittest: ante $(UOBJFILES)
	@echo Linking unittests...
	@mv obj/ante.o obj/ante.o.tmp
	@$(CXX) -DAN_LIB_DIR=$(ANLIBDIR) -DNO_MAIN $(CPPFLAGS) -MMD -MP -Iinclude -c src/ante.cpp -o obj/ante.o
	@$(CXX) obj/parser.o $(UOBJFILES) $(OBJFILES) $(ANOBJFILES) $(LLVMFLAGS) -o unittest
	@mv obj/ante.o.tmp obj/ante.o
	@./unittest


integrationtest:
	@ERRC=0;                                                                  \
	for file in $(ITESTFILES); do                                             \
		./ante -check $$file;                                                 \
		if [ $$? -ne 0 ]; then                                                \
		    echo "Failed to compile $$file";                                  \
		    ERRC=1;                                                           \
		fi;                                                                   \
	done;                                                                     \
	exit $$ERRC


#remove all intermediate files
clean:
	-@$(RM) obj/*.o obj/unit/*.o obj/*.d include/*.hh include/yyparser.h src/parser.cpp
