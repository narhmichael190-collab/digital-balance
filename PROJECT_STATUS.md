[package]
name = "digital-balance"
version = "1.3.4"
description = "Digital Balance Windows productivity app"
authors = ["Digital Balance"]
edition = "2021"

[lib]
name = "digital_balance_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2" }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-autostart = "2"
tauri-plugin-deep-link = "2"
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["clock", "serde"] }
once_cell = "1"
url = "2"
windows = { version = "0.61", features = [
  "Win32_Foundation",
  "Win32_UI_WindowsAndMessaging",
  "Win32_System_Threading",
  "Win32_UI_Accessibility"
] }
