use serde_json::Value;
use sysinfo::{System, SystemExt, ProcessExt};

pub fn choose_channel(data: Value) -> Option<String> {
    let keys = ["rc_default", "rc_live", "rc_beta"];
  
    for key in keys {
        if let Value::String(path) = &data[key] {
            return Some(path.to_string());
        }
    }
  
    None
}

pub fn is_running() -> bool {
    let sys = System::new_all();
    sys.processes().values().find(|p| p.name() == "RiotClientServices.exe").is_some()
}