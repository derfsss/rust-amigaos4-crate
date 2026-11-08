/*
 * freestanding_mem.c - memcpy, memmove, memset, memcmp, bcmp and strlen for
 * code linked without a C runtime (-nostartfiles -nodefaultlibs).
 * (LLVM turns a loop that looks for a NUL byte into a strlen call.)
 *
 * Rust's core library calls these symbols. In a clib4 program they come from
 * clib4. A driver, library or resource has no C library, and the
 * powerpc-unknown-amigaos sysroot does not export them from
 * compiler_builtins, so they are provided here.
 *
 * They are weak: if another object in the link defines them, that one wins.
 *
 * Compile with -ffreestanding -fno-builtin -fno-tree-loop-distribute-patterns
 * so GCC does not turn these loops back into calls to themselves.
 *
 * memcpy, memmove (forward) and memset move 32-bit words once the
 * destination is aligned. A source at another alignment is read as aligned
 * words and shifted into place, so no access is ever unaligned (the e5500
 * and others trap on some unaligned accesses, and the kernel's emulation of
 * them is slow). Every word read holds at least one byte of the source, so
 * nothing past the source's last aligned word is touched. The byte loops
 * these replace ran at about 1/10 of CopyMemQuick's speed and were most of
 * a TCP/IP stack's receive time.
 * tests/host/memtest.c checks every length and alignment against the C
 * library on the (little-endian) host; the big-endian shift is checked on
 * the target by testprogs/exectest's memcpy check.
 */

#include <stddef.h>
#include <stdint.h>

/* Whatever flags a Makefile passes: never turn a loop here into a call. */
#if defined(__GNUC__) && !defined(__clang__)
#pragma GCC optimize("no-tree-loop-distribute-patterns")
#endif

typedef uint32_t __attribute__((may_alias)) mword;

#if defined(__BYTE_ORDER__) && __BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__
#define MERGE(a, b, lo, hi) (((a) >> (lo)) | ((b) << (hi)))
#else
#define MERGE(a, b, lo, hi) (((a) << (lo)) | ((b) >> (hi)))
#endif

/* Forward copy; also right for an overlap with dst below src. */
static void copy_forward(unsigned char *d, const unsigned char *s, size_t n)
{
    if (n >= 16) {
        while ((uintptr_t)d & 3) {
            *d++ = *s++;
            n--;
        }
        mword *dw = (mword *)d;
        unsigned off = (uintptr_t)s & 3;
        if (off == 0) {
            const mword *sw = (const mword *)s;
            while (n >= 16) {
                mword a = sw[0], b = sw[1], c = sw[2], e = sw[3];
                dw[0] = a;
                dw[1] = b;
                dw[2] = c;
                dw[3] = e;
                sw += 4;
                dw += 4;
                n -= 16;
            }
            while (n >= 4) {
                *dw++ = *sw++;
                n -= 4;
            }
            s = (const unsigned char *)sw;
        } else {
            unsigned lo = off * 8, hi = 32 - lo;
            const mword *sw = (const mword *)(s - off);
            mword w0 = *sw++;
            while (n >= 4) {
                mword w1 = *sw++;
                *dw++ = MERGE(w0, w1, lo, hi);
                w0 = w1;
                n -= 4;
            }
            s = (const unsigned char *)sw - 4 + off;
        }
        d = (unsigned char *)dw;
    }
    while (n--)
        *d++ = *s++;
}

__attribute__((weak)) void *memcpy(void *dst, const void *src, size_t n)
{
    copy_forward(dst, src, n);
    return dst;
}

__attribute__((weak)) void *memmove(void *dst, const void *src, size_t n)
{
    unsigned char *d = dst;
    const unsigned char *s = src;
    if (d == s || n == 0)
        return dst;
    if (d < s || d >= s + n) {
        copy_forward(d, s, n);
    } else {
        d += n;
        s += n;
        while (n--)
            *--d = *--s;
    }
    return dst;
}

__attribute__((weak)) void *memset(void *dst, int c, size_t n)
{
    unsigned char *d = dst;
    unsigned char b = (unsigned char)c;
    if (n >= 16) {
        while ((uintptr_t)d & 3) {
            *d++ = b;
            n--;
        }
        mword w = (mword)b * 0x01010101u;
        mword *dw = (mword *)d;
        while (n >= 4) {
            *dw++ = w;
            n -= 4;
        }
        d = (unsigned char *)dw;
    }
    while (n--)
        *d++ = b;
    return dst;
}

__attribute__((weak)) int memcmp(const void *a, const void *b, size_t n)
{
    const unsigned char *p = a;
    const unsigned char *q = b;
    for (; n; n--, p++, q++) {
        if (*p != *q)
            return (int)*p - (int)*q;
    }
    return 0;
}

__attribute__((weak)) int bcmp(const void *a, const void *b, size_t n)
{
    return memcmp(a, b, n);
}

__attribute__((weak)) size_t strlen(const char *s)
{
    const char *p = s;
    while (*p)
        p++;
    return (size_t)(p - s);
}
