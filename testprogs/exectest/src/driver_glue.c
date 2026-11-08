/*
 * driver_glue.c - entry point for exectest.
 *
 * exectest has no C runtime (-nostartfiles -nodefaultlibs -lgcc). _start()
 * gets ExecBase from the system, sets IExec, opens dos.library's and
 * utility.library's main interfaces (Shell output; CallHookPkt in the hook
 * test), and passes the argument string to Rust.
 */

#define __NOLIBBASE__
#define __NOGLOBALIFACE__
#include <proto/exec.h>
#include <proto/dos.h>
#include <proto/utility.h>
#include <exec/types.h>

/* Read by the amigaos4 crate (sys::IExec, sys::IDOS, sys::IUtility). */
struct ExecIFace    *IExec    = NULL;
struct DOSIFace     *IDOS     = NULL;
struct UtilityIFace *IUtility = NULL;

extern int32 rust_main(STRPTR args, int32 arglen);

int32 _start(STRPTR argstring, int32 arglen, struct ExecBase *sysbase)
{
    struct Library *DOSBase, *UtilityBase;
    int32 ret = RETURN_FAIL;

    IExec = (struct ExecIFace *)sysbase->MainInterface;
    IExec->Obtain();

    DOSBase = IExec->OpenLibrary("dos.library", 52);
    UtilityBase = IExec->OpenLibrary("utility.library", 52);
    if (DOSBase && UtilityBase) {
        IDOS = (struct DOSIFace *)IExec->GetInterface(DOSBase, "main", 1, NULL);
        IUtility = (struct UtilityIFace *)IExec->GetInterface(UtilityBase, "main", 1, NULL);
        if (IDOS && IUtility) {
            ret = rust_main(argstring, arglen);
        }
        if (IUtility) IExec->DropInterface((struct Interface *)IUtility);
        if (IDOS) IExec->DropInterface((struct Interface *)IDOS);
    }
    if (UtilityBase) IExec->CloseLibrary(UtilityBase);
    if (DOSBase) IExec->CloseLibrary(DOSBase);

    IExec->Release();
    return ret;
}
