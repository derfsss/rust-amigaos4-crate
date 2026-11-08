/*
 * driver_glue.c - entry point for imgview.
 *
 * No C runtime (-nostartfiles -nodefaultlibs -lgcc): _start() takes
 * ExecBase, sets IExec, opens dos.library's interface and runs the Rust
 * program. A Workbench start is handled in Rust (workbench::WbStartup):
 * the start-up message is taken first and replied as rust_main returns.
 *
 * The other interfaces stay NULL: the crate opens what it uses itself.
 */

#define __NOLIBBASE__
#define __NOGLOBALIFACE__

#include <proto/exec.h>
#include <proto/dos.h>
#include <proto/utility.h>
#include <proto/graphics.h>
#include <proto/intuition.h>
#include <proto/wb.h>
#include <proto/datatypes.h>
#include <proto/application.h>
#include <proto/asl.h>
#include <proto/icon.h>
#include <exec/types.h>

/* Read by the amigaos4 crate. */
struct ExecIFace         *IExec         = NULL;
struct DOSIFace          *IDOS          = NULL;
struct UtilityIFace      *IUtility      = NULL;
struct GraphicsIFace     *IGraphics     = NULL;
struct IntuitionIFace    *IIntuition    = NULL;
struct WorkbenchIFace    *IWorkbench    = NULL;
struct DataTypesIFace    *IDataTypes    = NULL;
struct ApplicationIFace  *IApplication  = NULL;
struct PrefsObjectsIFace *IPrefsObjects = NULL;
struct AslIFace          *IAsl          = NULL;
struct IconIFace         *IIcon         = NULL;

/* The version string Version and the Workbench read. Global and named to
 * the linker (-u in the Makefile): --gc-sections dropped it as unreferenced. */
const char imgview_version[] __attribute__((used)) = "$VER: imgview 0.4 (5.11.2026)";

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
