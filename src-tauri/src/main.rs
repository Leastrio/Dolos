#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod http_proxy;
mod tcp_proxy;
mod utils;
mod sys_tray;
mod state;

use std::{error::Error, env, fs};

use tauri::Manager;
use tokio::{sync::OnceCell, process::Command};
use tauri::api::notification::Notification;

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
    .manage(state::DolosState::default())
    .invoke_handler(tauri::generate_handler![launch_game, mark_shutdown])
    .on_page_load(move |window, _| {
      if open_pids_c.is_some() {
        window.eval("showRiotClientPopup()").unwrap();
        window.eval(&format!("gamesToClose = {}", open_pids_c.iter().count())).unwrap();
      }
    })
    .system_tray(sys_tray::create_tray())
    .on_system_tray_event(sys_tray::handle_event)
    .setup(|app| {

      let handle = app.handle();
      handle.once_global("closeClients", move |_| {
        if let Some(pids) = open_pids {
          for (pid, name) in pids {
            utils::kill_process(pid, name)
          }
        }
      });
      tauri::async_runtime::spawn(tcp_proxy::proxy_tcp_chat());
      tauri::async_runtime::spawn(http_proxy::listen_http());

      Ok(())
    })
    .build(tauri::generate_context!())
    .expect("[Dolos] [Main] error while running tauri application")
    .run(|app_handle, event| match event {
      tauri::RunEvent::ExitRequested { api, .. } => {
        if app_handle.state::<state::DolosState>().shutdown.load(std::sync::atomic::Ordering::Relaxed) {
          app_handle.exit(0);
        } else {
          api.prevent_exit();
          let _ = Notification::new(&app_handle.config().tauri.bundle.identifier)
            .title("Dolos is running")
            .body("Dolos is running in the background! View the tray icon for more options.")
            .show();
        }
      }
      _ => {}
    });
}

#[tauri::command]
async fn launch_game(app: tauri::AppHandle, game: String) -> () {
  Command::new(RIOT_CLIENT_PATH.get().unwrap())
        .arg(format!("--client-config-url=http://127.0.0.1:{}", http_proxy::HTTP_PORT.get().unwrap()))
        .arg(format!("--launch-product={}", game))
        .arg("--launch-patchline=live")
        .spawn()
        .expect("[Dolos] [Main] Could not launch riot client!");

  tokio::time::sleep(tokio::time::Duration::from_millis(2500)).await;
  app.get_window("main").unwrap().close().unwrap();
}

#[tauri::command]
fn mark_shutdown(state: tauri::State<state::DolosState>) {
  state.shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
}