# Makefile — Link Rust staticlib + C glue into an AmigaOS 4 shared library
# No CRT, no clib4. Library mode: -nostartfiles -nodefaultlibs -lgcc
# Run inside Docker: walkero/amigagccondocker:os4-gcc15-3.6.1

TARGET    = hello.library
CC        = ppc-amigaos-gcc

# SDK include paths
SDK_INC   = /opt/ppc-amigaos/ppc-amigaos/SDK/include/include_h

# C glue sources
LIB_GLUE_SRC    = src/library_glue.c
SYS_GLUE_SRC    = ../../glue/amiga_glue.c
MEM_GLUE_SRC    = ../../glue/freestanding_mem.c

# C glue objects
LIB_GLUE_OBJ    = build/library_glue.o
SYS_GLUE_OBJ    = build/sys_glue.o
MEM_GLUE_OBJ    = build/freestanding_mem.o

# Rust output (staticlib built on host)
RUST_LIB  = target/powerpc-unknown-amigaos/release/libhello_library.a

# Compiler flags
CFLAGS    = -O2 -Wall -D__AMIGAOS4__ -U__USE_INLINE__ -I$(SDK_INC) \
            -fno-tree-loop-distribute-patterns -ffunction-sections -fdata-sections

# Linker flags — library mode
# --undefined=RomTag forces the Resident struct to be retained as a GC root
# (otherwise --gc-sections drops it — nothing statically references the RomTag,
# AmigaOS finds it by scanning memory for RTC_MATCHWORD).
LDFLAGS   = -nostartfiles -nodefaultlibs -Wl,-z,noexecstack -Wl,--gc-sections -Wl,--undefined=RomTag
LIBS      = -lgcc

.PHONY: all clean

all: build $(TARGET)

build:
	mkdir -p build

$(LIB_GLUE_OBJ): $(LIB_GLUE_SRC)
	$(CC) $(CFLAGS) -c $< -o $@

$(MEM_GLUE_OBJ): $(MEM_GLUE_SRC)
	$(CC) $(CFLAGS) -ffreestanding -fno-builtin -c $< -o $@

$(SYS_GLUE_OBJ): $(SYS_GLUE_SRC)
	$(CC) $(CFLAGS) -c $< -o $@

$(TARGET): $(LIB_GLUE_OBJ) $(SYS_GLUE_OBJ) $(MEM_GLUE_OBJ) $(RUST_LIB)
	$(CC) $(LDFLAGS) -o $@ $(LIB_GLUE_OBJ) $(SYS_GLUE_OBJ) $(MEM_GLUE_OBJ) $(RUST_LIB) $(LIBS)
	ppc-amigaos-strip $@

clean:
	rm -rf build $(TARGET)
