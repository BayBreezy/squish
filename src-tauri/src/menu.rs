//! Native application menu with a customised "About Squish" panel.
//!
//! macOS only shows name, version, copyright, credits and icon in its About
//! panel; the remaining fields (authors, comments, license) are used on
//! Windows and Linux.

use tauri::{
  AppHandle, Runtime,
  menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu},
};

/// Menu item id for "How to Use Squish"; handled in `lib.rs`.
pub const HELP_MENU_ID: &str = "how-to";
/// Event the frontend listens for to open the help sheet.
pub const HELP_EVENT: &str = "squish://show-help";

/// File-menu actions, sent to the frontend as the payload of [`ACTION_EVENT`].
pub const ACTION_ADD_IMAGES: &str = "add-images";
pub const ACTION_ADD_FOLDER: &str = "add-folder";
pub const ACTION_PROCESS: &str = "process";
pub const ACTION_CLEAR: &str = "clear";
pub const ACTION_SHORTCUTS: &str = "shortcuts";
/// Event carrying one of the `ACTION_*` ids as its payload.
pub const ACTION_EVENT: &str = "squish://menu-action";

/// True for menu item ids that are forwarded to the frontend as actions.
pub fn is_action(id: &str) -> bool {
  matches!(
    id,
    ACTION_ADD_IMAGES | ACTION_ADD_FOLDER | ACTION_PROCESS | ACTION_CLEAR | ACTION_SHORTCUTS
  )
}

const COPYRIGHT: &str = "© 2026 Behon Baker";
const DESCRIPTION: &str = "Compress and convert images locally.";
const CREDITS: &str = "Compress and convert PNG, JPEG, WebP and AVIF images — entirely on your \
computer. Nothing is uploaded.\n\n\
Built with Tauri, Nuxt, UI Thing and Rust.\n\
Powered by mozjpeg, libwebp, oxipng, imagequant and rav1e.\n\n\
Licensed under GPL-3.0-or-later.";

fn about_metadata<R: Runtime>(app: &AppHandle<R>) -> AboutMetadata<'_> {
  let pkg = app.package_info();
  AboutMetadata {
    name: Some(pkg.name.clone()),
    version: Some(pkg.version.to_string()),
    copyright: Some(COPYRIGHT.into()),
    credits: Some(CREDITS.into()),
    comments: Some(DESCRIPTION.into()),
    authors: Some(vec!["Behon Baker".into()]),
    license: Some("GPL-3.0-or-later".into()),
    icon: app.default_window_icon().cloned(),
    ..Default::default()
  }
}

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
  let name = app.package_info().name.clone();
  let about = PredefinedMenuItem::about(
    app,
    Some(&format!("About {name}")),
    Some(about_metadata(app)),
  )?;
  let separator = PredefinedMenuItem::separator(app)?;
  let shortcuts_item = MenuItem::with_id(
    app,
    ACTION_SHORTCUTS,
    "Keyboard Shortcuts",
    true,
    // Not ⇧⌘/ : macOS reserves that for "open the Help menu search".
    Some("CmdOrCtrl+K"),
  )?;
  let help_item = MenuItem::with_id(
    app,
    HELP_MENU_ID,
    format!("How to Use {name}"),
    true,
    Some("CmdOrCtrl+/"),
  )?;

  let add_images = MenuItem::with_id(
    app,
    ACTION_ADD_IMAGES,
    "Add Images…",
    true,
    Some("CmdOrCtrl+O"),
  )?;
  let add_folder = MenuItem::with_id(
    app,
    ACTION_ADD_FOLDER,
    "Add Folder…",
    true,
    Some("CmdOrCtrl+Shift+O"),
  )?;
  let process = MenuItem::with_id(
    app,
    ACTION_PROCESS,
    format!("Start {name}"),
    true,
    Some("CmdOrCtrl+Enter"),
  )?;
  // No accelerator: ⌘⌫ already means "delete line" inside the text fields.
  let clear = MenuItem::with_id(app, ACTION_CLEAR, "Clear List", true, None::<&str>)?;

  #[cfg(target_os = "macos")]
  let app_menu = Submenu::with_items(
    app,
    &name,
    true,
    &[
      &about,
      &separator,
      &PredefinedMenuItem::services(app, None)?,
      &separator,
      &PredefinedMenuItem::hide(app, None)?,
      &PredefinedMenuItem::hide_others(app, None)?,
      &PredefinedMenuItem::show_all(app, None)?,
      &separator,
      &PredefinedMenuItem::quit(app, None)?,
    ],
  )?;

  let mut file_items: Vec<&dyn tauri::menu::IsMenuItem<R>> =
    vec![&add_images, &add_folder, &separator, &process, &clear, &separator];
  let close_window = PredefinedMenuItem::close_window(app, None)?;
  file_items.push(&close_window);
  #[cfg(not(target_os = "macos"))]
  let quit = PredefinedMenuItem::quit(app, None)?;
  #[cfg(not(target_os = "macos"))]
  file_items.push(&quit);
  let file = Submenu::with_items(app, "File", true, &file_items)?;

  // Standard edit shortcuts (⌘C/⌘V…) only work in text fields when the items exist.
  let edit = Submenu::with_items(
    app,
    "Edit",
    true,
    &[
      &PredefinedMenuItem::undo(app, None)?,
      &PredefinedMenuItem::redo(app, None)?,
      &separator,
      &PredefinedMenuItem::cut(app, None)?,
      &PredefinedMenuItem::copy(app, None)?,
      &PredefinedMenuItem::paste(app, None)?,
      &PredefinedMenuItem::select_all(app, None)?,
    ],
  )?;

  let window = Submenu::with_items(
    app,
    "Window",
    true,
    &[
      &PredefinedMenuItem::minimize(app, None)?,
      &PredefinedMenuItem::maximize(app, None)?,
      &separator,
      &PredefinedMenuItem::close_window(app, None)?,
    ],
  )?;

  #[cfg(target_os = "macos")]
  {
    let help = Submenu::with_items(app, "Help", true, &[&help_item, &shortcuts_item])?;
    // Lets macOS add its built-in menu search box to this menu.
    help.set_as_help_menu_for_nsapp()?;
    Menu::with_items(app, &[&app_menu, &file, &edit, &window, &help])
  }

  #[cfg(not(target_os = "macos"))]
  {
    let help = Submenu::with_items(
      app,
      "Help",
      true,
      &[&help_item, &shortcuts_item, &separator, &about],
    )?;
    Menu::with_items(app, &[&file, &edit, &window, &help])
  }
}
