use tauri::{SystemTray, CustomMenuItem, SystemTrayMenu,
    // SystemTrayMenuItem, 
    AppHandle, SystemTrayEvent,
    // SystemTraySubmenu
};

pub fn create_tray() -> SystemTray {
    // let status = SystemTraySubmenu::new("Status", SystemTrayMenu::new().add_item(CustomMenuItem::new("Online".to_string(), "Online").selected()));
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let open = CustomMenuItem::new("open".to_string(), "Open");
    let menu = SystemTrayMenu::new()
        // .add_submenu(status)
        // .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(open)
        .add_item(quit);
    SystemTray::new().with_menu(menu)
}

pub fn handle_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::MenuItemClick {  id, .. } => {
            match id.as_ref() {
                "open" => {
                    tauri::WindowBuilder::from_config(app, app.config().tauri.windows.get(0).unwrap().clone()).build().unwrap();
                },
                "quit" => {
                    app.exit(0);
                },
                _ => {}
            }
        }
        _ => {}
    }
}