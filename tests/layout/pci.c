#include <exec/nodes.h>
#include <expansion/pci.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
/* sys::MinNode */
A(sizeof(struct MinNode) == 8);
/* sys::PCICapability (after #pragma pack(): natural alignment) */
A(sizeof(struct PCICapability) == 20);
A(offsetof(struct PCICapability, Type) == 8);
A(offsetof(struct PCICapability, Size) == 12);
A(offsetof(struct PCICapability, CapOffset) == 16);
/* sys::PCIResourceRange (pack(2)) */
A(sizeof(struct PCIResourceRange) == 30);
A(offsetof(struct PCIResourceRange, BaseAddress) == 14);
A(offsetof(struct PCIResourceRange, Size) == 18);
A(offsetof(struct PCIResourceRange, Physical) == 22);
A(offsetof(struct PCIResourceRange, Flags) == 26);
