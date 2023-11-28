fn main() {
  if cfg!(target_os = "windows") {
    tauri_build::build()
  } else {
    println!("Error: This application can only be used and compiled for Windows.");
    std::process::exit(1);
  }
}
