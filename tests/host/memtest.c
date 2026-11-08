/*
 * memtest.c - glue/freestanding_mem.c against the host C library: every
 * length 0..300 at every source and destination alignment 0..7, memmove
 * overlaps both ways, memset. On the host:
 *   gcc -O2 -fno-builtin -fno-tree-loop-distribute-patterns tests/host/memtest.c && ./a.out
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define memcpy fm_memcpy
#define memmove fm_memmove
#define memset fm_memset
#define memcmp fm_memcmp
#define bcmp fm_bcmp
#define strlen fm_strlen
#include "../../glue/freestanding_mem.c"
#undef memcpy
#undef memmove
#undef memset
#undef memcmp
#undef bcmp
#undef strlen

#define N 400
static unsigned char src[N + 16], a[N + 16], b[N + 16];

int main(void)
{
    long checks = 0, bad = 0;
    for (int i = 0; i < N + 16; i++)
        src[i] = (unsigned char)(i * 7 + 3);
    for (int so = 0; so < 8; so++)
        for (int doff = 0; doff < 8; doff++)
            for (int n = 0; n <= 300; n++) {
                memset(a, 0xEE, sizeof a);
                memset(b, 0xEE, sizeof b);
                void *r = fm_memcpy(a + doff, src + so, n);
                memcpy(b + doff, src + so, n);
                checks++;
                if (r != a + doff || memcmp(a, b, sizeof a)) {
                    if (bad++ < 5)
                        printf("memcpy wrong: src+%d dst+%d n=%d\n", so, doff, n);
                }
            }
    /* memmove: overlapping, both directions, every shift up to 9. */
    for (int shift = -9; shift <= 9; shift++)
        for (int at = 9; at < 17; at++)
            for (int n = 0; n <= 300; n++) {
                memcpy(a, src, sizeof a);
                memcpy(b, src, sizeof b);
                fm_memmove(a + at + shift, a + at, n);
                memmove(b + at + shift, b + at, n);
                checks++;
                if (memcmp(a, b, sizeof a)) {
                    if (bad++ < 10)
                        printf("memmove wrong: shift %d at %d n=%d\n", shift, at, n);
                }
            }
    for (int doff = 0; doff < 8; doff++)
        for (int n = 0; n <= 300; n++) {
            memset(a, 0xEE, sizeof a);
            memset(b, 0xEE, sizeof b);
            fm_memset(a + doff, 0x5A, n);
            memset(b + doff, 0x5A, n);
            checks++;
            if (memcmp(a, b, sizeof a)) {
                if (bad++ < 15)
                    printf("memset wrong: dst+%d n=%d\n", doff, n);
            }
        }
    printf("memtest: %ld checks, %ld wrong\n", checks, bad);
    return bad != 0 || checks == 0;
}
