vpath %.c src
vpath %.h include
vpath %.d obj

WARNINGS := -Wall
CFLAGS   := -g -O2 -lgmp -std=c11 $(WARNINGS)

CPPFLAGS := -g -O2 -std=c++11 $(WARNINGS)

PROJDIRS := src include

CSRCFILES := $(shell find $(PROJDIRS) -type f -name "*.c")
CPPSRCFILES := $(shell find $(PROJDIRS) -type f -name "*.cpp")

OBJFILES := $(patsubst src/%.c,obj/%.o,$(SRCFILES))
DEPFILES := $(SRCFILES:.c=.d)

-include $(DEPFILES)

.PHONY: all clean zy

zy: $(OBJFILES) obj/compiler.o
	-@$(CC) $(CFLAGS) -o zy $?

new: clean zy

$(OBJFILES): | obj

obj: 
	@mkdir -p $@

obj/compiler.o: src/compiler.cpp compiler.h
	-@$(CC) $(CPPFLAGS) -MMD -MP -Iinclude -c $< -o $@

obj/%.o: %.c Makefile
	-@$(CC) $(CFLAGS) -MMD -MP -Iinclude -c $< -o $@

clean:
	-@$(RM) obj/*.o obj/*.d zy
