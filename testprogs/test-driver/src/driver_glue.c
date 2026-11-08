/*
 * driver_glue.c - entry point for test-driver.
 *
 * test-driver has no C runtime: it is linked with -nostartfiles
 * -nodefaultlibs -lgcc. The system calls _start() with ExecBase; the glue
 * sets IExec (which the amigaos4 crate reads, and which ExecAllocator needs
 * before the first allocation) and runs the Rust tests.
 */

#define __NOLIBBASE__
#define __NOGLOBALIFACE__
#include <proto/exec.h>
#include <exec/types.h>

struct ExecIFace *IExec = NULL;

extern int32 rust_handler_main(struct ExecBase *sysbase);

int32 _start(STRPTR argstring __attribute__((unused)),
             int32 arglen __attribute__((unused)),
             struct ExecBase *sysbase)
{
    IExec = (struct ExecIFace *)sysbase->MainInterface;
    IExec->Obtain();

    int32 ret = rust_handler_main(sysbase);

    IExec->Release();
    return ret;
}
