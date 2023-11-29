#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod http_proxy;
mod tcp_proxy;
mod utils;

use std::{error::Error, env, fs};

use tauri::Manager;
use tokio::{sync::OnceCell, process::Command};

pub static RIOT_CLIENT_PATH: OnceCell<String> = OnceCell::const_new();

type DolosResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

fn main() {
  let file_path = format!(
    "{}\\Riot Games\\RiotClientInstalls.json",
    env::var("ProgramData").expect("[Dolos] [Main] Could not find program data folder")
  );
  let data = serde_json::from_str(&String::from_utf8_lossy(&fs::read(file_path).expect("[Dolos] [Main] Could not read riot installs config"))).expect("[Dolos] [Main] Could not parse riot installs config");
  RIOT_CLIENT_PATH.set(utils::choose_channel(data).unwrap()).expect("[Dolos] [Main] Could not set RIOT_CLIENT_PATH");

  let open_pids = utils::get_pids();
  let open_pids_c = open_pids.clone();

  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![launch_game])
    .on_page_load(move |window, _| {
      if !open_pids_c.is_empty() {
        window.eval("showRiotClientPopup()").unwrap();
        window.eval(&format!("gamesToClose = {}", open_pids_c.iter().count())).unwrap();
      }
    })
    .setup(|app| {

      let handle = app.handle();
      handle.once_global("closeClients", move |_| {
        for (pid, name) in open_pids {
          utils::kill_process(pid, name)
        }
      });

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
fn launch_game(game: &str) {
  Command::new(RIOT_CLIENT_PATH.get().unwrap())
        .arg(format!("--client-config-url=http://127.0.0.1:{}", http_proxy::HTTP_PORT.get().unwrap()))
        .arg(format!("--launch-product={}", game))
        .arg("--launch-patchline=live")
        .spawn()
        .expect("[Dolos] [Main] Could not launch riot client!");
}