#include <libraries/locale.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
/* locale.rs reads these struct Locale fields at fixed offsets */
A(offsetof(struct Locale, loc_LanguageName) == 4);
A(offsetof(struct Locale, loc_CodeSet) == 52);
A(offsetof(struct Locale, loc_CountryCode) == 56);
A(offsetof(struct Locale, loc_GMTOffset) == 64);
/* constants */
A(OC_BuiltInLanguage == TAG_USER + 0x90001);
A(OC_Version == TAG_USER + 0x90003);
A(OC_Language == TAG_USER + 0x90004);
A(SC_ASCII == 0 && SC_COLLATE1 == 1 && SC_COLLATE2 == 2);
A(DAY_1 == 1 && ABDAY_1 == 8 && MON_1 == 15 && ABMON_1 == 27);
A(YESSTR == 39 && NOSTR == 40 && AM_STR == 41 && PM_STR == 42);
