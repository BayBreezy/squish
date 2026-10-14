use tauri::Emitter;

mod commands;
mod error;
mod menu;
mod pipeline;
mod types;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let builder = tauri::Builder::default();

  // Remember window size, position and maximised state between launches.
  // This must be registered on the builder, not inside `setup`: Tauri creates
  // the windows from tauri.conf.json *before* running `setup`, so a plugin
  // added there never sees the main window and restores nothing.
  #[cfg(desktop)]
  let builder = builder.plugin(tauri_plugin_window_state::Builder::default().build());

  builder
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_opener::init())
    .manage(commands::ProcessingState::default())
    .invoke_handler(tauri::generate_handler![
      commands::inspect_files,
      commands::process_images,
      commands::cancel_processing,
      commands::make_thumbnail,
      commands::read_preview,
    ])
    .on_menu_event(|app, event| {
      let id = event.id().as_ref();
      if id == menu::HELP_MENU_ID {
        let _ = app.emit(menu::HELP_EVENT, ());
      } else if menu::is_action(id) {
        let _ = app.emit(menu::ACTION_EVENT, id);
      }
    })
    .setup(|app| {
      app.set_menu(menu::build(app.handle())?)?;
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running Squish");
}
