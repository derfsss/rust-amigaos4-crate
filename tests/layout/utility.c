#include <utility/date.h>
#include <utility/message_digest.h>
#include <utility/random.h>
#include <utility/name.h>
#include <utility/tagitem.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
/* sys::ClockData */
A(sizeof(struct ClockData) == 14);
A(offsetof(struct ClockData, sec) == 0);
A(offsetof(struct ClockData, min) == 2);
A(offsetof(struct ClockData, hour) == 4);
A(offsetof(struct ClockData, mday) == 6);
A(offsetof(struct ClockData, month) == 8);
A(offsetof(struct ClockData, year) == 10);
A(offsetof(struct ClockData, wday) == 12);
/* sys::MessageDigest_SHA */
A(sizeof(struct MessageDigest_SHA) == 348);
A(offsetof(struct MessageDigest_SHA, mdsha_Code) == 0);
/* sys::RandomState */
A(sizeof(struct RandomState) == 8);
A(offsetof(struct RandomState, rs_Low) == 4);
/* sys::NamedObject */
A(sizeof(struct NamedObject) == 4);
/* sys::TagItem */
A(sizeof(struct TagItem) == 8);
