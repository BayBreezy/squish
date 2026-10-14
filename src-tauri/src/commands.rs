use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use tauri::{State, ipc::Channel};

use crate::{
    error::AppError,
    pipeline,
    types::{InspectResult, ProcessEvent, Settings},
};

/// Shared flag used to stop a running batch.
#[derive(Default)]
pub struct ProcessingState {
    pub cancel: Arc<AtomicBool>,
}

fn join_err(e: tauri::Error) -> AppError {
    AppError::Encode(e.to_string())
}

/// Validate dropped/selected paths and read basic info (folders are expanded).
#[tauri::command]
pub async fn inspect_files(paths: Vec<String>) -> Result<InspectResult, AppError> {
    tauri::async_runtime::spawn_blocking(move || pipeline::inspect(&paths))
        .await
        .map_err(join_err)
}

/// Compress/convert every file, streaming progress over `on_event`.
#[tauri::command]
pub async fn process_images(
    state: State<'_, ProcessingState>,
    paths: Vec<String>,
    settings: Settings,
    on_event: Channel<ProcessEvent>,
) -> Result<(), AppError> {
    let cancel = state.cancel.clone();
    cancel.store(false, Ordering::Relaxed);
    let settings = settings.sanitized();
    let files: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();

    let cancelled = tauri::async_runtime::spawn_blocking({
        let on_event = on_event.clone();
        move || {
            pipeline::run_batch(&files, &settings, &cancel, |event| {
                let _ = on_event.send(event);
            })
        }
    })
    .await
    .map_err(join_err)?;

    let _ = on_event.send(ProcessEvent::Finished { cancelled });
    Ok(())
}

/// Ask the running batch to stop after the files currently being encoded.
#[tauri::command]
pub fn cancel_processing(state: State<'_, ProcessingState>) {
    state.cancel.store(true, Ordering::Relaxed);
}

/// File bytes for the before/after viewer (see `pipeline::preview_bytes`).
#[tauri::command]
pub async fn read_preview(path: String, max_px: u32) -> Result<tauri::ipc::Response, AppError> {
    let bytes = tauri::async_runtime::spawn_blocking(move || {
        pipeline::preview_bytes(&PathBuf::from(path), max_px.clamp(256, 6000))
    })
    .await
    .map_err(join_err)??;
    Ok(tauri::ipc::Response::new(bytes))
}

/// JPEG preview bytes for the file list.
#[tauri::command]
pub async fn make_thumbnail(path: String, max_px: u32) -> Result<tauri::ipc::Response, AppError> {
    let bytes = tauri::async_runtime::spawn_blocking(move || {
        pipeline::thumbnail(&PathBuf::from(path), max_px.clamp(16, 1024))
    })
    .await
    .map_err(join_err)??;
    Ok(tauri::ipc::Response::new(bytes))
}
