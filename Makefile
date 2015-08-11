WARNINGS := -Wall
CFLAGS := -g $(WARNINGS)

PROJDIRS := src include
AUXFILES := Makefile README.md LICENSE

SRCFILES := $(shell find $(PROJDIRS) -type f -name "*.c")
HDRFILES := $(shell find $(PROJDIRS) -type f -name "*.h")

OBJFILES := $(patsubst %.c,%.o,$(SRCFILES))
DEPFILES := $(patsubst %.c,%.d,$(SRCFILES))

ALLFILES := $(SRCFILES) $(HDRFILES) $(AUXFILES)

-include $(DEPFILES)

.PHONY: all clean

all: zy

zy: $(OBJFILES)
	@$(CC) $(CFLAGS) -o zy $?
	@mv $(OBJFILES) obj
	-@$(RM) $(DEPFILES)

%.o: %.c Makefile
	@$(CC) $(CFLAGS) -MMD -MP -Iinclude -c $< -o $@

clean:
	-@$(RM) $(wildcard $(OBJFILES) $(DEPFILES) zy)


