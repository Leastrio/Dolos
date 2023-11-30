use std::ffi::CStr;
use serde_json::Value;
use winapi::um::{processthreadsapi::{OpenProcess, TerminateProcess}, winnt::PROCESS_ALL_ACCESS, handleapi::{CloseHandle, INVALID_HANDLE_VALUE}, errhandlingapi::GetLastError, tlhelp32::{TH32CS_SNAPPROCESS, PROCESSENTRY32, Process32First, Process32Next}};
use winapi::um::tlhelp32::CreateToolhelp32Snapshot;

const PROCESS_NAMES: [&str; 3] = ["RiotClientServices.exe", "LeagueClient.exe", "VALORANT-Win64-Shipping.exe"];

pub fn choose_channel(data: Value) -> Option<String> {
    let keys = ["rc_default", "rc_live", "rc_beta"];
  
    for key in keys {
        if let Value::String(path) = &data[key] {
            return Some(path.to_string());
        }
    }
  
    None
}

pub fn get_pids() -> Option<Vec<(u32, String)>> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            eprintln!("[DOLOS] Error creating snapshot of processes");
            return None;
        }

        let mut pids: Vec<(u32, String)> = vec![];
        let mut pe: PROCESSENTRY32 = std::mem::zeroed();
        pe.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

        if Process32First(snapshot, &mut pe) == 0 {
            eprintln!("[DOLOS] Error getting process snapshot");
            CloseHandle(snapshot);
            return None;
        }

        while Process32Next(snapshot, &mut pe) != 0 {
            let process_name = CStr::from_ptr(pe.szExeFile.as_ptr()).to_string_lossy();
            
            if PROCESS_NAMES.contains(&process_name.as_ref()) {
                pids.push((pe.th32ProcessID, process_name.to_string()));
            }
        }
        CloseHandle(snapshot);
        if pids.is_empty() {
            None
        } else {
            Some(pids)
        }
    }
}

pub fn kill_process(pid: u32, name: String) {
    unsafe {
        let process_handle = OpenProcess(PROCESS_ALL_ACCESS, 0, pid);
        if process_handle.is_null() {
            eprintln!("[DOLOS] Failed to open handle to {}. Error code: {}", name, GetLastError());
        }

        if TerminateProcess(process_handle, 0) == 0 {
            eprintln!("[DOLOS] Failed to terminate {}. Error code: {}", name, GetLastError())
        } else {
            println!("[DOLOS] Successfully terminated {}", name);
        };
        CloseHandle(process_handle);
    }
}