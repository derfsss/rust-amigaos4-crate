/*
 * kick_glue.c - rusttest.device, a kickstart module written in Rust.
 *
 * The file carries two Residents, found by exec when the kickstart is
 * scanned:
 *
 *   KickTags.cold   RTF_NATIVE | RTF_COLDSTART | RTF_AUTOINIT, NT_DEVICE.
 *                Exec creates the device during the cold start and calls
 *                devInit(), which hands the boot details to Rust.
 *
 *   KickTags.after  RTF_NATIVE | RTF_AFTERDOS. Runs once DOS is up: Rust
 *                reports again, then opens rusttest.device and reads from
 *                it with DoIO to show the module is live.
 *
 * Everything is printed with DebugPrintF to the debug output (serial).
 * The COLDSTART tag is the first Resident in the file: both tags sit in one
 * object (KickTags) and this object is linked first.
 */

#include <proto/exec.h>
#include <exec/types.h>
#include <exec/resident.h>
#include <exec/devices.h>
#include <exec/execbase.h>
#include <exec/exectags.h>
#include <exec/io.h>
#include <exec/errors.h>
#include <dos/dos.h>   /* BPTR */

#define DEVNAME    "rusttest.device"
#define DEVVER     1
#define DEVREV     0
#define DEVPRI     0
#define VSTRING    "rusttest.device 1.0 (03.11.2026)"
#define AFTERPRI   -100

struct ExecIFace *IExec = NULL;

struct MyDevice {
    struct Device devNode;
    uint16        pad;
    BPTR          segList;
};

/* What the glue tells Rust about the moment it runs (kick_test::KickInfo). */
struct KickInfo {
    CONST_STRPTR phase;          /* "COLDSTART" or "AFTERDOS" */
    CONST_STRPTR task_name;      /* the task running the init */
    APTR         resident;       /* the Resident being initialised */
    uint32       rt_flags;
    int32        rt_pri;
    uint32       rt_version;
    APTR         base;           /* device base (COLDSTART) or NULL */
    uint32       seglist;
    uint32       exec_version;
    uint32       exec_revision;
    uint32       avail_mem;      /* AvailMem(MEMF_ANY) */
    uint32       dos_resident_version;   /* rt_Version of dos.library's Resident, 0 if none */
    CONST_STRPTR cpu_model;      /* GCIT_ModelString */
};

extern int32 rust_kick_coldstart(const struct KickInfo *info);
extern void  rust_kick_afterdos(const struct KickInfo *info);
extern int8  rust_dev_open(struct IORequest *ior, uint32 unit, uint32 flags);
extern void  rust_dev_close(struct IORequest *ior);
extern void  rust_dev_begin_io(struct IORequest *ior);

/* Both Residents in one object, COLDSTART first (defined at the end). */
struct KickTags {
    struct Resident cold;
    struct Resident after;
};
extern const struct KickTags KickTags;

static void fill_info(struct KickInfo *ki, CONST_STRPTR phase,
                      const struct Resident *rt, APTR base, BPTR seglist)
{
    struct ExecBase *sysbase = (struct ExecBase *)IExec->Data.LibBase;
    struct Task *me = IExec->FindTask(NULL);
    struct Resident *dosrt = IExec->FindResident("dos.library");
    CONST_STRPTR model = NULL;

    IExec->GetCPUInfoTags(GCIT_ModelString, &model, TAG_DONE);

    ki->phase       = phase;
    ki->task_name   = (me && me->tc_Node.ln_Name) ? me->tc_Node.ln_Name : (CONST_STRPTR)"?";
    ki->resident    = (APTR)rt;
    ki->rt_flags    = rt->rt_Flags;
    ki->rt_pri      = rt->rt_Pri;
    ki->rt_version  = rt->rt_Version;
    ki->base        = base;
    ki->seglist     = (uint32)seglist;
    ki->exec_version  = sysbase->LibNode.lib_Version;
    ki->exec_revision = sysbase->LibNode.lib_Revision;
    ki->avail_mem   = IExec->AvailMem(MEMF_ANY);
    ki->dos_resident_version = dosrt ? dosrt->rt_Version : 0;
    ki->cpu_model   = model ? model : (CONST_STRPTR)"?";
}

/* ---------------------------------------------------------------- device */

static uint32 _manager_Obtain(struct Interface *self)
{
    return self->Data.RefCount++;
}

static uint32 _manager_Release(struct Interface *self)
{
    return self->Data.RefCount--;
}

static struct MyDevice *devInit(struct MyDevice *devBase, BPTR segList,
                                struct ExecIFace *iexec)
{
    struct KickInfo ki;

    IExec = iexec;

    devBase->devNode.dd_Library.lib_Node.ln_Type = NT_DEVICE;
    devBase->devNode.dd_Library.lib_Node.ln_Pri  = DEVPRI;
    devBase->devNode.dd_Library.lib_Node.ln_Name = (STRPTR)DEVNAME;
    devBase->devNode.dd_Library.lib_Flags        = LIBF_SUMUSED | LIBF_CHANGED;
    devBase->devNode.dd_Library.lib_Version      = DEVVER;
    devBase->devNode.dd_Library.lib_Revision     = DEVREV;
    devBase->devNode.dd_Library.lib_IdString     = (STRPTR)VSTRING;
    devBase->segList                             = segList;

    fill_info(&ki, "COLDSTART", &KickTags.cold, devBase, segList);
    if (rust_kick_coldstart(&ki) != 0)
        return NULL;

    return devBase;
}

static void devOpen(struct Interface *self, struct IORequest *ior,
                    uint32 unit, uint32 flags)
{
    struct MyDevice *devBase = (struct MyDevice *)self->Data.LibBase;
    int8 err = rust_dev_open(ior, unit, flags);

    ior->io_Error = err;
    if (err == 0) {
        devBase->devNode.dd_Library.lib_OpenCnt++;
        devBase->devNode.dd_Library.lib_Flags &= ~LIBF_DELEXP;
        ior->io_Device = (struct Device *)devBase;
        ior->io_Message.mn_Node.ln_Type = NT_REPLYMSG;
    } else {
        ior->io_Device = NULL;
        ior->io_Unit   = NULL;
    }
}

static APTR devClose(struct Interface *self, struct IORequest *ior)
{
    struct MyDevice *devBase = (struct MyDevice *)self->Data.LibBase;

    rust_dev_close(ior);
    ior->io_Device = NULL;
    ior->io_Unit   = NULL;
    devBase->devNode.dd_Library.lib_OpenCnt--;
    return NULL;   /* a kickstart module is never unloaded */
}

static APTR devExpunge(struct Interface *self)
{
    struct MyDevice *devBase = (struct MyDevice *)self->Data.LibBase;

    devBase->devNode.dd_Library.lib_Flags |= LIBF_DELEXP;
    return NULL;   /* a kickstart module is never unloaded */
}

static void devBeginIO(struct Interface *self __attribute__((unused)),
                       struct IORequest *ior)
{
    rust_dev_begin_io(ior);
}

static void devAbortIO(struct Interface *self __attribute__((unused)),
                       struct IORequest *ior __attribute__((unused)))
{
    /* Every request completes inside BeginIO. */
}

static void *devManagerVectors[] = {
    (void *)_manager_Obtain,
    (void *)_manager_Release,
    (void *)0,
    (void *)0,
    (void *)devOpen,
    (void *)devClose,
    (void *)devExpunge,
    (void *)0,
    (void *)devBeginIO,
    (void *)devAbortIO,
    (void *)(APTR)-1,
};

static struct TagItem devManagerTags[] = {
    { MIT_Name,        (uint32)"__device" },
    { MIT_VectorTable, (uint32)devManagerVectors },
    { MIT_Version,     1 },
    { MIT_DataSize,    0 },
    { TAG_DONE,        0 }
};

static uint32 devInterfaces[] = {
    (uint32)devManagerTags,
    0
};

static struct TagItem devCreateTags[] = {
    { CLT_DataSize,   (uint32)sizeof(struct MyDevice) },
    { CLT_Interfaces, (uint32)devInterfaces },
    { CLT_InitFunc,   (uint32)devInit },
    { TAG_DONE,       0 }
};

/* ------------------------------------------------------------- AFTERDOS */

static APTR afterDosInit(APTR dummy __attribute__((unused)), BPTR seglist,
                         struct ExecBase *sysbase)
{
    struct KickInfo ki;

    IExec = (struct ExecIFace *)sysbase->MainInterface;
    fill_info(&ki, "AFTERDOS", &KickTags.after, NULL, seglist);
    rust_kick_afterdos(&ki);
    return NULL;
}

/* ------------------------------------------------------------ Residents */

static const char devName[]       = DEVNAME;
static const char devVString[]    = VSTRING;
static const char afterName[]     = "rusttest.afterdos";

/* One object holds both tags, so the COLDSTART tag is first by layout, not
 * by whatever order the compiler emits separate globals in. */
const struct KickTags KickTags __attribute__((used, aligned(4), section(".rodata"))) = {
    {
        RTC_MATCHWORD,
        (struct Resident *)&KickTags.cold,
        (APTR)(&KickTags.cold + 1),
        RTF_NATIVE | RTF_COLDSTART | RTF_AUTOINIT,
        DEVVER,
        NT_DEVICE,
        DEVPRI,
        (STRPTR)devName,
        (STRPTR)devVString,
        (APTR)devCreateTags
    },
    {
        RTC_MATCHWORD,
        (struct Resident *)&KickTags.after,
        (APTR)(&KickTags.after + 1),
        RTF_NATIVE | RTF_AFTERDOS,
        DEVVER,
        NT_UNKNOWN,
        AFTERPRI,
        (STRPTR)afterName,
        (STRPTR)devVString,
        (APTR)afterDosInit
    }
};

/* Not a program: running it from the Shell does nothing. */
int32 _start(void)
{
    return RETURN_FAIL;
}
