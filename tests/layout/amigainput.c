#include <amigainput/amigainput.h>
#include <stddef.h>
#define A(x) _Static_assert(x, #x)
/* input.rs */
A(sizeof(AIN_Device) == 28 && offsetof(AIN_Device, Type) == 4 && offsetof(AIN_Device, SubType) == 8 && offsetof(AIN_Device, NumButtons) == 12);
A(offsetof(AIN_Device, NumAxes) == 16 && offsetof(AIN_Device, NumHats) == 20 && offsetof(AIN_Device, DeviceID) == 24);
A(AINQ_NUMAXES == 1 && AINQ_NUMBUTTONS == 2 && AINQ_NUMHATS == 3 && AINQ_POLLED == 7 && AINQ_AXIS_SYMMETRIC == 8 && AINQ_SUBCLASS == 9 && AINQ_DRIVERVERSION == 10);
A(AINQ_AXIS_CLASS == 11 && AINQ_AXIS_OFFSET == 12 && AINQ_BUTTON_OFFSET == 13 && AINQ_HAT_OFFSET == 14 && AINQ_LAYOUT_CLASS == 15 && AINQ_CONNECTED == 18 && AINQ_UNITS == 19);
A(AINDT_MOUSE == 1 && AINDT_KEYBOARD == 2 && AINDT_JOYSTICK == 3 && AINDP_ACTIVE == 1 && AINERR_DOUBLEOBTAIN == 11 && AINERR_UNKNOWN_DEVICEID == 3);
