//! The exec and dos constants used by `task`, `signal` and `sync` against
//! the SDK's exec/exectags.h, dos/dos.h and utility/tagitem.h. Skips,
//! saying so, without an SDK at `sdk/` or `AMIGA_SDK`.

use std::fs;
mod common;
use common::{definition, eval, include_dir};

use amigaos4::sys;

#[test]
fn exec_constants_match_the_sdk() {
    let Some(inc) = include_dir() else {
        eprintln!("[skip] AmigaOS 4 SDK not found at sdk/ (set AMIGA_SDK to override).");
        return;
    };
    let read = |p: &str| fs::read_to_string(inc.join(p)).unwrap_or_else(|e| panic!("{}: {}", p, e));
    let exectags = read("exec/exectags.h");
    let dos = read("dos/dos.h");
    let tagitem = read("utility/tagitem.h");
    let ports = read("exec/ports.h");
    let nodes = read("exec/nodes.h");
    let memory = read("exec/memory.h");
    let debug = read("exec/debug.h");
    let semaphores = read("exec/semaphores.h");
    let texts = [
        exectags.as_str(), dos.as_str(), tagitem.as_str(), ports.as_str(), nodes.as_str(), memory.as_str(), debug.as_str(),
        semaphores.as_str(),
    ];

    let checks: Vec<(&str, i64)> = vec![
        ("ASOT_SEMAPHORE", sys::ASOT_SEMAPHORE as i64),
        ("ASOT_MUTEX", sys::ASOT_MUTEX as i64),
        ("ASO_NoTrack", sys::ASO_NOTRACK as i64),
        ("ASOMUTEX_Recursive", sys::ASOMUTEX_RECURSIVE as i64),
        ("ASOSEM_Size", sys::ASOSEM_SIZE as i64),
        ("ASOSEM_Name", sys::ASOSEM_NAME as i64),
        ("ASOSEM_Pri", sys::ASOSEM_PRI as i64),
        ("ASOSEM_Public", sys::ASOSEM_PUBLIC as i64),
        ("ASOSEM_CopyName", sys::ASOSEM_COPYNAME as i64),
        ("AT_Param1", sys::AT_PARAM1 as i64),
        ("AT_Param2", sys::AT_PARAM2 as i64),
        ("AT_Param3", sys::AT_PARAM3 as i64),
        ("AT_Param4", sys::AT_PARAM4 as i64),
        ("CT_LockStack", sys::CT_LOCK_STACK as i64),
        ("GTATAG_IsValidTask", sys::GTATAG_IS_VALID_TASK as i64),
        ("GSITAG_SchedulerState", sys::GSITAG_SCHEDULER_STATE as i64),
        ("GSITAG_SchedulerStateGlobal", sys::GSITAG_SCHEDULER_STATE_GLOBAL as i64),
        ("GSITAG_InterruptState", sys::GSITAG_INTERRUPT_STATE as i64),
        ("GSITAG_InterruptStateGlobal", sys::GSITAG_INTERRUPT_STATE_GLOBAL as i64),
        ("GSITAG_ProcessingInterrupt", sys::GSITAG_PROCESSING_INTERRUPT as i64),
        ("GSISTATE_ENABLE", sys::GSISTATE_ENABLE as i64),
        ("GSISTATE_DISABLE", sys::GSISTATE_DISABLE as i64),
        ("STA_Core", sys::STA_CORE as i64),
        ("STACORE_CLEAR", sys::STACORE_CLEAR as i64),
        ("STACORE_CURRENT", sys::STACORE_CURRENT as i64),
        ("GCIT_NumberOfCPUs", sys::GCIT_NUMBER_OF_CPUS as i64),
        ("GCIT_CacheLineSize", sys::GCIT_CACHE_LINE_SIZE as i64),
        ("SIGBREAKF_CTRL_C", sys::SIGBREAKF_CTRL_C as i64),
        ("SIGBREAKF_CTRL_D", sys::SIGBREAKF_CTRL_D as i64),
        ("SIGBREAKF_CTRL_E", sys::SIGBREAKF_CTRL_E as i64),
        ("SIGBREAKF_CTRL_F", sys::SIGBREAKF_CTRL_F as i64),
        ("ASOT_IOREQUEST", sys::ASOT_IOREQUEST as i64),
        ("ASOT_HOOK", sys::ASOT_HOOK as i64),
        ("ASOT_LIST", sys::ASOT_LIST as i64),
        ("ASOT_NODE", sys::ASOT_NODE as i64),
        ("ASOT_PORT", sys::ASOT_PORT as i64),
        ("ASOT_MESSAGE", sys::ASOT_MESSAGE as i64),
        ("ASOT_MEMPOOL", sys::ASOT_MEMPOOL as i64),
        ("ASOT_ITEMPOOL", sys::ASOT_ITEMPOOL as i64),
        ("ASOT_RMAP", sys::ASOT_RMAP as i64),
        ("ASOIOR_Size", sys::ASOIOR_SIZE as i64),
        ("ASOIOR_ReplyPort", sys::ASOIOR_REPLYPORT as i64),
        ("ASOIOR_Duplicate", sys::ASOIOR_DUPLICATE as i64),
        ("ASOHOOK_Size", sys::ASOHOOK_SIZE as i64),
        ("ASOHOOK_Entry", sys::ASOHOOK_ENTRY as i64),
        ("ASOHOOK_Subentry", sys::ASOHOOK_SUBENTRY as i64),
        ("ASOHOOK_Data", sys::ASOHOOK_DATA as i64),
        ("ASOLIST_Size", sys::ASOLIST_SIZE as i64),
        ("ASOLIST_Type", sys::ASOLIST_TYPE as i64),
        ("ASOLIST_Min", sys::ASOLIST_MIN as i64),
        ("ASONODE_Size", sys::ASONODE_SIZE as i64),
        ("ASONODE_Min", sys::ASONODE_MIN as i64),
        ("ASONODE_Type", sys::ASONODE_TYPE as i64),
        ("ASONODE_Pri", sys::ASONODE_PRI as i64),
        ("ASONODE_Name", sys::ASONODE_NAME as i64),
        ("ASONODE_CopyName", sys::ASONODE_COPYNAME as i64),
        ("ASOPORT_Size", sys::ASOPORT_SIZE as i64),
        ("ASOPORT_AllocSig", sys::ASOPORT_ALLOCSIG as i64),
        ("ASOPORT_Action", sys::ASOPORT_ACTION as i64),
        ("ASOPORT_Pri", sys::ASOPORT_PRI as i64),
        ("ASOPORT_Name", sys::ASOPORT_NAME as i64),
        ("ASOPORT_Signal", sys::ASOPORT_SIGNAL as i64),
        ("ASOPORT_Target", sys::ASOPORT_TARGET as i64),
        ("ASOPORT_Public", sys::ASOPORT_PUBLIC as i64),
        ("ASOPORT_CopyName", sys::ASOPORT_COPYNAME as i64),
        ("ASOMSG_Size", sys::ASOMSG_SIZE as i64),
        ("ASOMSG_ReplyPort", sys::ASOMSG_REPLYPORT as i64),
        ("ASOMSG_Length", sys::ASOMSG_LENGTH as i64),
        ("ASOMSG_Name", sys::ASOMSG_NAME as i64),
        ("ASOPOOL_MFlags", sys::ASOPOOL_MFLAGS as i64),
        ("ASOPOOL_Puddle", sys::ASOPOOL_PUDDLE as i64),
        ("ASOPOOL_Threshold", sys::ASOPOOL_THRESHOLD as i64),
        ("ASOPOOL_Protected", sys::ASOPOOL_PROTECTED as i64),
        ("ASOPOOL_Name", sys::ASOPOOL_NAME as i64),
        ("ASOPOOL_CopyName", sys::ASOPOOL_COPYNAME as i64),
        ("ASOPOOL_LockMem", sys::ASOPOOL_LOCKMEM as i64),
        ("ASOITEM_MFlags", sys::ASOITEM_MFLAGS as i64),
        ("ASOITEM_ItemSize", sys::ASOITEM_ITEMSIZE as i64),
        ("ASOITEM_BatchItems", sys::ASOITEM_BATCHITEMS as i64),
        ("ASOITEM_MaxItems", sys::ASOITEM_MAXITEMS as i64),
        ("ASOITEM_GCPolicy", sys::ASOITEM_GCPOLICY as i64),
        ("ASOITEM_GCParameter", sys::ASOITEM_GCPARAMETER as i64),
        ("ASOITEM_Protected", sys::ASOITEM_PROTECTED as i64),
        ("ITEMGC_NONE", sys::ITEMGC_NONE as i64),
        ("ITEMGC_AFTERCOUNT", sys::ITEMGC_AFTERCOUNT as i64),
        ("ASORMAP_Base", sys::ASORMAP_BASE as i64),
        ("ASORMAP_Size", sys::ASORMAP_SIZE as i64),
        ("ASORMAP_Quantum", sys::ASORMAP_QUANTUM as i64),
        ("ASORMAP_CacheMax", sys::ASORMAP_CACHEMAX as i64),
        ("PA_SIGNAL", sys::PA_SIGNAL as i64),
        ("PA_SOFTINT", sys::PA_SOFTINT as i64),
        ("PA_IGNORE", sys::PA_IGNORE as i64),
        ("NT_UNKNOWN", sys::NT_UNKNOWN as i64),
        ("NT_MESSAGE", sys::NT_MESSAGE as i64),
        ("NT_FREEMSG", sys::NT_FREEMSG as i64),
        ("NT_REPLYMSG", sys::NT_REPLYMSG as i64),
        ("ANMT_CheckSum", sys::ANMT_CHECKSUM as i64),
        ("ANMT_Error", sys::ANMT_ERROR as i64),
        ("ANMERROR_NOERROR", sys::ANMERROR_NOERROR as i64),
        ("ANMERROR_NOMEMORY", sys::ANMERROR_NOMEMORY as i64),
        ("ANMERROR_DUPLICATENAME", sys::ANMERROR_DUPLICATENAME as i64),
        ("ANMERROR_PARAMETER", sys::ANMERROR_PARAMETER as i64),
        ("SNMF_NAMESPACES_ONLY", sys::SNMF_NAMESPACES_ONLY as i64),
        ("NSR_StackSize", sys::NSR_STACKSIZE as i64),
        ("NSR_MinStackSize", sys::NSR_MINSTACKSIZE as i64),
        ("NSR_LockStack", sys::NSR_LOCKSTACK as i64),
        ("NSR_Arg1", sys::NSR_ARG1 as i64),
        ("GCIT_Family", sys::GCIT_FAMILY as i64),
        ("GCIT_Model", sys::GCIT_MODEL as i64),
        ("GCIT_ModelString", sys::GCIT_MODELSTRING as i64),
        ("GCIT_Version", sys::GCIT_VERSION as i64),
        ("GCIT_VersionString", sys::GCIT_VERSIONSTRING as i64),
        ("GCIT_FrontsideSpeed", sys::GCIT_FRONTSIDESPEED as i64),
        ("GCIT_ProcessorSpeed", sys::GCIT_PROCESSORSPEED as i64),
        ("GCIT_L1CacheSize", sys::GCIT_L1CACHESIZE as i64),
        ("GCIT_L2CacheSize", sys::GCIT_L2CACHESIZE as i64),
        ("GCIT_L3CacheSize", sys::GCIT_L3CACHESIZE as i64),
        ("GCIT_VectorUnit", sys::GCIT_VECTORUNIT as i64),
        ("GCIT_Extensions", sys::GCIT_EXTENSIONS as i64),
        ("GCIT_CPUPageSize", sys::GCIT_CPUPAGESIZE as i64),
        ("GCIT_ExecPageSize", sys::GCIT_EXECPAGESIZE as i64),
        ("GCIT_CurrentCPUNum", sys::GCIT_CURRENTCPUNUM as i64),
        ("GCIT_TimeBaseSpeed", sys::GCIT_TIMEBASE_SPEED as i64),
        ("STACK_FRAME_DECODED", sys::STACK_FRAME_DECODED as i64),
        ("DEBUG_SYMBOL_MODULE", sys::DEBUG_SYMBOL_MODULE as i64),
        ("DEBUG_SYMBOL_MODULE_STABS", sys::DEBUG_SYMBOL_MODULE_STABS as i64),
        ("SM_SHARED", sys::SM_SHARED as i64),
        ("SM_EXCLUSIVE", sys::SM_EXCLUSIVE as i64),
        ("MEMHF_RECYCLE", sys::MEMHF_RECYCLE as i64),
        ("MEM_DID_NOTHING", sys::MEM_DID_NOTHING as i64),
        ("MEM_ALL_DONE", sys::MEM_ALL_DONE as i64),
        ("MEM_TRY_AGAIN", sys::MEM_TRY_AGAIN as i64),
        ("SDONA_Task", sys::SDONA_TASK as i64),
        ("SDONA_SignalMask", sys::SDONA_SIGNALMASK as i64),
    ];
    let mut bad = Vec::new();
    for (name, ours) in &checks {
        let def = definition(&texts, name).unwrap_or_else(|| panic!("{} not found in the SDK", name));
        let sdk = eval(&texts, def, 0).unwrap_or_else(|| panic!("{}: cannot evaluate {:?}", name, def));
        if sdk != *ours {
            bad.push(format!("{}: SDK {} ours {}", name, sdk, ours));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
    assert!(checks.len() >= 31 + 95, "the check list was emptied");
}
