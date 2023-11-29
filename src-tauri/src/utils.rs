use serde_json::Value;
use sysinfo::{System, SystemExt, ProcessExt, PidExt};
use winapi::um::{processthreadsapi::{OpenProcess, TerminateProcess}, winnt::PROCESS_ALL_ACCESS, handleapi::CloseHandle, errhandlingapi::GetLastError};

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

pub fn get_pids() -> Vec<(u32, String)> {
    let sys = System::new_all();
    sys.processes().iter().filter_map(|(pid, process)| {
        if PROCESS_NAMES.contains(&process.name()) {
            Some((pid.as_u32(), process.name().to_string()))
        } else {
            None
        }
    }).collect::<Vec<(u32, String)>>()
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