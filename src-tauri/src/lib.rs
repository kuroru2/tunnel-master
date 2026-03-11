pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter("tunnel_master=debug")
        .init();

    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
