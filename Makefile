vpath %.c src
vpath %.h include
vpath %.d obj

WARNINGS := -Wall
CFLAGS   := -g -O3 -std=c99 $(WARNINGS)

PROJDIRS := src include
SRCFILES := $(shell find $(PROJDIRS) -type f -name "*.c")

OBJFILES := $(patsubst src/%.c,obj/%.o,$(SRCFILES))
DEPFILES := $(SRCFILES:.c=.d)

-include $(DEPFILES)

.PHONY: all obj clean

all: zy

zy: $(OBJFILES)
	-$(CC) $(CFLAGS) -o zy $?

$(OBJFILES): | obj

obj: 
	@mkdir -p $@

obj/%.o: %.c Makefile
	-$(CC) $(CFLAGS) -MMD -MP -Iinclude -c $< -o $@

clean:
	-@$(RM) obj/*.o obj/*.d zy
