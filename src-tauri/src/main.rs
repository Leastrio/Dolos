#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  tauri::Builder::default()
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

// mod http_proxy;
// mod tcp_proxy;

// use serde_json::Value;
// use std::{env, error::Error, process::Command, fs};

// type DolosResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

// #[tokio::main]
// async fn main() -> DolosResult<()> {
//     tokio::spawn(tcp_proxy::proxy_tcp_chat());
//     let file_path = format!(
//         "{}\\Riot Games\\RiotClientInstalls.json",
//         env::var("ProgramData")?
//     );
//     let data: Value = serde_json::from_str(&String::from_utf8_lossy(&fs::read(file_path)?))?;
//     let path = choose_channel(data).unwrap();

//     let config_address = http_proxy::listen_http().await?;

//     Command::new(path)
//         .arg(format!("--client-config-url=http://{}", config_address))
//         .arg("--launch-product=league_of_legends")
//         .arg("--launch-patchline=live")
//         .spawn()?;

//     loop {}
// }

// pub fn choose_channel(data: Value) -> Option<String> {
//   let keys = ["rc_default", "rc_live", "rc_beta"];

//   for key in keys {
//       if let Value::String(path) = &data[key] {
//           return Some(path.to_string());
//       }
//   }

//   None
// }