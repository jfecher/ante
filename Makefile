vpath %.c src
vpath %.h include
vpath %.d obj

WARNINGS := -Wall
CFLAGS   := -g -O2 -lgmp -std=c11 $(WARNINGS)

PROJDIRS := src include
SRCFILES := $(shell find $(PROJDIRS) -type f -name "*.c")

OBJFILES := $(patsubst src/%.c,obj/%.o,$(SRCFILES))
DEPFILES := $(SRCFILES:.c=.d)

-include $(DEPFILES)

.PHONY: all clean

all: zy

zy: $(OBJFILES)
	-@$(CC) $(CFLAGS) -o zy $?

$(OBJFILES): | obj

obj: 
	@mkdir -p $@

obj/%.o: %.c Makefile
	-@$(CC) $(CFLAGS) -MMD -MP -Iinclude -c $< -o $@

clean:
	-@$(RM) obj/*.o obj/*.d zy
