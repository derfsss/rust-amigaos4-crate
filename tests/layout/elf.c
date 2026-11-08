#include <libraries/elf.h>
#include <utility/hooks.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
/* elf.rs */
A(sizeof(struct Elf32_SymbolQuery) == 36 && offsetof(struct Elf32_SymbolQuery, Name) == 4 && offsetof(struct Elf32_SymbolQuery, NameLength) == 8);
A(offsetof(struct Elf32_SymbolQuery, Value) == 12 && offsetof(struct Elf32_SymbolQuery, Found) == 16 && sizeof(BOOL) == 2 && offsetof(struct Elf32_SymbolQuery, Sym) == 20);
A(sizeof(Elf32_Sym) == 16 && offsetof(Elf32_Sym, st_value) == 4 && offsetof(Elf32_Sym, st_size) == 8 && offsetof(Elf32_Sym, st_info) == 12 && offsetof(Elf32_Sym, st_shndx) == 14);
A(sizeof(Elf32_Shdr) == 40 && offsetof(Elf32_Shdr, sh_type) == 4 && offsetof(Elf32_Shdr, sh_offset) == 16 && offsetof(Elf32_Shdr, sh_size) == 20 && offsetof(Elf32_Shdr, sh_entsize) == 36);
A(sizeof(Elf32_Ehdr) == 52 && offsetof(Elf32_Ehdr, e_type) == 16 && offsetof(Elf32_Ehdr, e_machine) == 18 && offsetof(Elf32_Ehdr, e_entry) == 24);
A(offsetof(Elf32_Ehdr, e_shnum) == 48 && offsetof(Elf32_Ehdr, e_shstrndx) == 50);
A(offsetof(struct SymbolMsg, Sym) == 0 && offsetof(struct SymbolMsg, AbsValue) == 4 && offsetof(struct SymbolMsg, Name) == 8);
A(offsetof(struct Hook, h_Entry) == 8 && offsetof(struct Hook, h_SubEntry) == 12 && offsetof(struct Hook, h_Data) == 16 && sizeof(struct Hook) == 20);
A(ELF32_SQ_BYNAME == 1 && ELF32_SQ_BYVALUE == 2 && SST_Load == TAG_USER + 1 && GST_SectionIndex == TAG_USER + 1 && GST_Load == TAG_USER + 3);
A(EAT_NumSections == TAG_USER + 1 && EAT_ElfHeader == TAG_USER + 4 && EAT_SectionStringTable == TAG_USER + 5);
A(OET_Filename == TAG_USER + 1 && OET_MemoryAddr == TAG_USER + 3 && CET_CloseAll == TAG_USER + 4 && SHT_NOBITS == 8);
/* SharedObject (src/elf.rs). */
#include <dos/dos.h>
#include <dos/dostags.h>
A(OET_ElfHandle == TAG_USER + 4 && CET_ReClose == TAG_USER + 5 && ELF32_RTLD_GLOBAL == 1 && ELF32_NO_ERROR == 0);
A(GSLI_ElfHandle == TAG_USER + 4000 + 4 && GPSLF_RUN == (1 << 4));
