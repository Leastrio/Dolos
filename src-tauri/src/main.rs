#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod http_proxy;
mod tcp_proxy;
mod utils;

use std::{error::Error, env, fs};

use tokio::{sync::OnceCell, process::Command};

pub static RIOT_CLIENT_PATH: OnceCell<String> = OnceCell::const_new();

type DolosResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

fn main() {
  // TODO: Check if riot client is running at boot, close if so
  // TODO: Close riot client if dolos closes
  // utils::is_running();
  let file_path = format!(
    "{}\\Riot Games\\RiotClientInstalls.json",
    env::var("ProgramData").expect("[Dolos] [Main] Could not find program data folder")
  );
  let data = serde_json::from_str(&String::from_utf8_lossy(&fs::read(file_path).expect("[Dolos] [Main] Could not read riot installs config"))).expect("[Dolos] [Main] Could not parse riot installs config");
  RIOT_CLIENT_PATH.set(utils::choose_channel(data).unwrap()).expect("[Dolos] [Main] Could not set RIOT_CLIENT_PATH");

  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![launch_game])
    .setup(|_app| {
      tauri::async_runtime::spawn(async {
        tokio::spawn(tcp_proxy::proxy_tcp_chat());
        tokio::spawn(http_proxy::listen_http());
      });
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("[Dolos] [Main] error while running tauri application");
}

#[tauri::command]
fn launch_game(game: &str) -> () {
  Command::new(RIOT_CLIENT_PATH.get().unwrap())
        .arg(format!("--client-config-url=http://127.0.0.1:{}", http_proxy::HTTP_PORT.get().unwrap()))
        .arg(format!("--launch-product={}", game))
        .arg("--launch-patchline=live")
        .spawn()
        .expect("[Dolos] [Main] Could not launch riot client!");
}