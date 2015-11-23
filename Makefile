vpath %.c src
vpath %.h include
vpath %.d obj

WARNINGS := -Wall
LLVMFLAGS := `llvm-config --cppflags --libs all --ldflags --system-libs`

CPPFLAGS := -g -O2 -std=c++11 $(WARNINGS) $(LLVMFLAGS)

SRCDIRS := src

SRCFILES := $(shell find $(SRCDIRS) -type f -name "*.cpp")

OBJFILES := $(patsubst src/%.cpp,obj/%.o,$(SRCFILES))

DEPFILES := $(OBJFILES:.o=.d)

-include $(DEPFILES)

.PHONY: all clean zy

zy: $(OBJFILES)
	$(CXX) $? $(CPPFLAGS) -o zy

new: clean zy

#$(OBJFILES): | obj

obj: 
	@mkdir -p $@

obj/%.o: src/%.cpp Makefile
	$(CXX) $(CPPFLAGS) -MMD -MP -Iinclude -c $< -o $@

clean:
	-@$(RM) obj/*.o obj/*.d zy
