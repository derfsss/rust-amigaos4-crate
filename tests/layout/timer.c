#include <devices/timer.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
/* sys::TimeVal, sys::EClockVal, timer.rs TIME_REQUEST_SIZE */
A(sizeof(struct TimeVal) == 8);
A(offsetof(struct TimeVal, Microseconds) == 4);
A(sizeof(struct EClockVal) == 8);
A(offsetof(struct EClockVal, ev_lo) == 4);
A(sizeof(struct TimeRequest) == 40);
A(offsetof(struct TimeRequest, Time) == 32);
A(TR_ADDREQUEST == 9);
A(UNIT_MICROHZ == 0 && UNIT_VBLANK == 1 && UNIT_ECLOCK == 2 && UNIT_WAITUNTIL == 3);
