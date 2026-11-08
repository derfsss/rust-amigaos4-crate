//! The dos.library constants the `dos` module uses, against the SDK's
//! dos/dos.h, dos/dostags.h, dos/errors.h, dos/var.h and exec/nodes.h.
//! Skips, saying so, without an SDK at `sdk/` or `AMIGA_SDK`.

use std::fs;
mod common;
use common::{definition, eval, include_dir};

use amigaos4::sys;

#[test]
fn dos_constants_match_the_sdk() {
    let Some(inc) = include_dir() else {
        eprintln!("[skip] AmigaOS 4 SDK not found at sdk/ (set AMIGA_SDK to override).");
        return;
    };
    let read = |p: &str| fs::read_to_string(inc.join(p)).unwrap_or_else(|e| panic!("{}: {}", p, e));
    let dos = read("dos/dos.h");
    let tags = read("dos/dostags.h");
    let errors = read("dos/errors.h");
    let var = read("dos/var.h");
    let rdargs = read("dos/rdargs.h");
    let notify = read("dos/notify.h");
    let dosext = read("dos/dosextens.h");
    let datetime = read("dos/datetime.h");
    let nodes = read("exec/nodes.h");
    let tagitem = read("utility/tagitem.h");
    let texts = [dos.as_str(), tags.as_str(), errors.as_str(), var.as_str(), rdargs.as_str(), notify.as_str(), dosext.as_str(), datetime.as_str(), nodes.as_str(), tagitem.as_str()];

    let checks: Vec<(&str, i64)> = vec![
        ("MODE_OLDFILE", sys::MODE_OLDFILE as i64),
        ("MODE_NEWFILE", sys::MODE_NEWFILE as i64),
        ("MODE_READWRITE", sys::MODE_READWRITE as i64),
        ("OFFSET_BEGINNING", sys::OFFSET_BEGINNING as i64),
        ("OFFSET_CURRENT", sys::OFFSET_CURRENT as i64),
        ("OFFSET_END", sys::OFFSET_END as i64),
        ("SHARED_LOCK", sys::SHARED_LOCK as i64),
        ("EXCLUSIVE_LOCK", sys::EXCLUSIVE_LOCK as i64),
        ("LOCK_SAME", sys::LOCK_SAME as i64),
        ("LOCK_SAME_VOLUME", sys::LOCK_SAME_VOLUME as i64),
        ("LOCK_DIFFERENT", sys::LOCK_DIFFERENT as i64),
        ("MAX_DOS_NAME", sys::MAX_DOS_NAME as i64),
        ("MAX_DOS_PATH", sys::MAX_DOS_PATH as i64),
        ("DOS_EXAMINEDATA", sys::DOS_EXAMINEDATA as i64),
        ("EX_StringNameInput", sys::EX_STRING_NAME_INPUT as i64),
        ("EX_FileHandleInput", sys::EX_FILE_HANDLE_INPUT as i64),
        ("EX_LockInput", sys::EX_LOCK_INPUT as i64),
        ("EX_DataFields", sys::EX_DATA_FIELDS as i64),
        ("EX_MatchString", sys::EX_MATCH_STRING as i64),
        ("EXF_ALL", sys::EXF_ALL as i64),
        ("FSO_TYPE_SOFTLINK", sys::FSO_TYPE_SOFTLINK as i64),
        ("FSO_TYPE_FILE", sys::FSO_TYPE_FILE as i64),
        ("FSO_TYPE_DIRECTORY", sys::FSO_TYPE_DIRECTORY as i64),
        ("FSO_TYPE_PIPE", sys::FSO_TYPE_PIPE as i64),
        ("FSO_TYPE_SOCKET", sys::FSO_TYPE_SOCKET as i64),
        ("FSO_TYPE_MASK", sys::FSO_TYPE_MASK as i64),
        ("FSOF_LINK", sys::FSOF_LINK as i64),
        ("FSOF_ALTLINK", sys::FSOF_ALTLINK as i64),
        ("FSOF_EMULATED", sys::FSOF_EMULATED as i64),
        ("ERROR_LINE_TOO_LONG", sys::ERROR_LINE_TOO_LONG as i64),
        ("ERROR_OBJECT_IN_USE", sys::ERROR_OBJECT_IN_USE as i64),
        ("ERROR_OBJECT_EXISTS", sys::ERROR_OBJECT_EXISTS as i64),
        ("ERROR_DIR_NOT_FOUND", sys::ERROR_DIR_NOT_FOUND as i64),
        ("ERROR_OBJECT_NOT_FOUND", sys::ERROR_OBJECT_NOT_FOUND as i64),
        ("ERROR_OBJECT_WRONG_TYPE", sys::ERROR_OBJECT_WRONG_TYPE as i64),
        ("ERROR_DIRECTORY_NOT_EMPTY", sys::ERROR_DIRECTORY_NOT_EMPTY as i64),
        ("ERROR_NO_MORE_ENTRIES", sys::ERROR_NO_MORE_ENTRIES as i64),
        ("ERROR_NOT_IMPLEMENTED", sys::ERROR_NOT_IMPLEMENTED as i64),
        ("ERROR_BREAK", sys::ERROR_BREAK as i64),
        ("LV_VAR", sys::LV_VAR as i64),
        ("GVF_GLOBAL_ONLY", sys::GVF_GLOBAL_ONLY as i64),
        ("GVF_LOCAL_ONLY", sys::GVF_LOCAL_ONLY as i64),
        ("GVF_BINARY_VAR", sys::GVF_BINARY_VAR as i64),
        ("GVF_DONT_NULL_TERM", sys::GVF_DONT_NULL_TERM as i64),
        ("NT_TASK", sys::NT_TASK as i64),
        ("NT_PROCESS", sys::NT_PROCESS as i64),
        ("DOS_INFODATA", sys::DOS_INFODATA as i64),
        ("DOS_NOTIFYREQUEST", sys::DOS_NOTIFYREQUEST as i64),
        ("DOS_VOLUMELIST", sys::DOS_VOLUMELIST as i64),
        ("ADO_Type", sys::ADO_TYPE as i64),
        ("ADO_AddColon", sys::ADO_ADD_COLON as i64),
        ("ADO_NotifyName", sys::ADO_NOTIFY_NAME as i64),
        ("ADO_NotifyMethod", sys::ADO_NOTIFY_METHOD as i64),
        ("ADO_NotifyTask", sys::ADO_NOTIFY_TASK as i64),
        ("ADO_NotifySignalNumber", sys::ADO_NOTIFY_SIGNAL_NUMBER as i64),
        ("NRF_SEND_SIGNAL", sys::NRF_SEND_SIGNAL as i64),
        ("LDF_DEVICES", sys::LDF_DEVICES as i64),
        ("LDF_VOLUMES", sys::LDF_VOLUMES as i64),
        ("LDF_ASSIGNS", sys::LDF_ASSIGNS as i64),
        ("GDI_StringNameInput", sys::GDI_STRING_NAME_INPUT as i64),
        ("GDI_InfoData", sys::GDI_INFO_DATA as i64),
        ("ID_DISKSTATE_WRITE_PROTECTED", sys::ID_DISKSTATE_WRITE_PROTECTED as i64),
        ("ID_DISKSTATE_VALIDATING", sys::ID_DISKSTATE_VALIDATING as i64),
        ("ID_DISKSTATE_VALIDATED", sys::ID_DISKSTATE_VALIDATED as i64),
        ("SYS_Input", sys::SYS_INPUT as i64),
        ("SYS_Output", sys::SYS_OUTPUT as i64),
        ("FORMAT_DOS", sys::FORMAT_DOS as i64),
        ("FORMAT_INT", sys::FORMAT_INT as i64),
        ("FORMAT_USA", sys::FORMAT_USA as i64),
        ("FORMAT_CDN", sys::FORMAT_CDN as i64),
        ("FORMAT_DEF", sys::FORMAT_DEF as i64),
        ("FORMAT_ISO", sys::FORMAT_ISO as i64),
        ("DTF_SUBST", sys::DTF_SUBST as i64),
        ("DTF_FUTURE", sys::DTF_FUTURE as i64),
        ("DOS_RDARGS", sys::DOS_RDARGS as i64),
        ("RDAF_STDIN", sys::RDAF_STDIN as i64),
        ("RDAF_NOALLOC", sys::RDAF_NOALLOC as i64),
        ("RDAF_NOPROMPT", sys::RDAF_NOPROMPT as i64),
        ("ERROR_BAD_TEMPLATE", sys::ERROR_BAD_TEMPLATE as i64),
        ("ERROR_BAD_NUMBER", sys::ERROR_BAD_NUMBER as i64),
        ("ERROR_REQUIRED_ARG_MISSING", sys::ERROR_REQUIRED_ARG_MISSING as i64),
        ("ERROR_KEY_NEEDS_ARG", sys::ERROR_KEY_NEEDS_ARG as i64),
        ("ERROR_TOO_MANY_ARGS", sys::ERROR_TOO_MANY_ARGS as i64),
        ("NP_Start", sys::NP_START as i64),
        ("NP_Input", sys::NP_INPUT as i64),
        ("NP_Output", sys::NP_OUTPUT as i64),
        ("NP_CloseInput", sys::NP_CLOSE_INPUT as i64),
        ("NP_CloseOutput", sys::NP_CLOSE_OUTPUT as i64),
        ("NP_CurrentDir", sys::NP_CURRENT_DIR as i64),
        ("NP_StackSize", sys::NP_STACK_SIZE as i64),
        ("NP_Name", sys::NP_NAME as i64),
        ("NP_Priority", sys::NP_PRIORITY as i64),
        ("NP_UserData", sys::NP_USER_DATA as i64),
        ("NP_Child", sys::NP_CHILD as i64),
        ("GPID_PROCESS", sys::GPID_PROCESS as i64),
        ("ERROR_INVALID_COMPONENT_NAME", sys::ERROR_INVALID_COMPONENT_NAME as i64),
        ("NV_ADD_NOTIFY", sys::NV_ADD_NOTIFY as i64),
        ("NV_REMOVE_NOTIFY_HOOK", sys::NV_REMOVE_NOTIFY_HOOK as i64),
        ("GPID_PARENT", sys::GPID_PARENT as i64),
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
    assert!(checks.len() >= 96, "the check list was emptied");
}
