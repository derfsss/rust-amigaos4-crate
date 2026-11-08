/*
 * driver_glue.c - entry point for panictest.
 *
 * panictest has no C runtime (-nostartfiles -nodefaultlibs -lgcc). _start()
 * gets ExecBase from the system, sets IExec, opens dos.library's main
 * interface (Shell output) and passes the argument string to Rust.
 *
 * IUtility, IExpansion, IPCI and ITimer are declared but left NULL on purpose: the
 * crate's safe modules then open those interfaces themselves, which is the
 * path panictest checks.
 */

#define __NOLIBBASE__
#define __NOGLOBALIFACE__

#include <proto/exec.h>
#include <proto/dos.h>
#include <proto/utility.h>
#include <proto/expansion.h>
#include <proto/timer.h>
#include <proto/locale.h>
#include <proto/application.h>
#include <proto/graphics.h>
#include <proto/intuition.h>
#include <proto/gadtools.h>
#include <proto/layers.h>
#include <proto/wb.h>
#include <proto/icon.h>
#include <proto/asl.h>
#include <proto/datatypes.h>
#include <proto/console.h>
#include <proto/elf.h>
#include <proto/amigainput.h>
#include <exec/types.h>

/* Read by the amigaos4 crate. */
struct ExecIFace      *IExec      = NULL;
struct DOSIFace       *IDOS       = NULL;
struct UtilityIFace   *IUtility   = NULL;
struct ExpansionIFace *IExpansion = NULL;
struct PCIIFace       *IPCI       = NULL;
struct TimerIFace     *ITimer     = NULL;
struct LocaleIFace    *ILocale    = NULL;
struct ApplicationIFace *IApplication = NULL;
struct PrefsObjectsIFace *IPrefsObjects = NULL;
struct GraphicsIFace  *IGraphics  = NULL;
struct IntuitionIFace *IIntuition = NULL;
struct GadToolsIFace  *IGadTools  = NULL;
struct LayersIFace    *ILayers    = NULL;
struct WorkbenchIFace *IWorkbench = NULL;
struct IconIFace      *IIcon      = NULL;
struct AslIFace       *IAsl       = NULL;
struct DataTypesIFace *IDataTypes = NULL;
struct ConsoleIFace   *IConsole   = NULL;
struct ElfIFace       *IElf       = NULL;
struct AIN_IFace      *IAmigaInput = NULL;

extern int32 rust_main(STRPTR args, int32 arglen);

int32 _start(STRPTR argstring, int32 arglen, struct ExecBase *sysbase)
{
    struct Library *DOSBase;
    int32 ret = RETURN_FAIL;

    IExec = (struct ExecIFace *)sysbase->MainInterface;
    IExec->Obtain();

    DOSBase = IExec->OpenLibrary("dos.library", 52);
    if (DOSBase) {
        IDOS = (struct DOSIFace *)IExec->GetInterface(DOSBase, "main", 1, NULL);
        if (IDOS) {
            ret = rust_main(argstring, arglen);
            IExec->DropInterface((struct Interface *)IDOS);
        }
        IExec->CloseLibrary(DOSBase);
    }

    IExec->Release();
    return ret;
}
