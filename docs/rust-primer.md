# Rust primer for Squish

You don't need to know Rust to run or ship this app, but this page explains how the Rust half is wired so you can read and change it with confidence.

## The big picture

```
Vue (webview)  ──invoke("process_images", …)──▶  Rust command  ──▶  pipeline  ──▶  files on disk
      ▲                                              │
      └──────── Channel<ProcessEvent> (progress) ◀───┘
```

The webview never touches image bytes. It sends **paths + settings** and receives small JSON events.

## Where things live (`src-tauri/src/`)

| File          | Role                                                                                                                                                      |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `main.rs`     | One line: calls `squish_lib::run()`. Leave it alone.                                                                                                      |
| `lib.rs`      | Builds the Tauri app: registers plugins, shared state and **every command** in `generate_handler![…]`.                                                    |
| `commands.rs` | The functions JavaScript can call (`#[tauri::command]`). Thin: they validate input and hand off to `pipeline`.                                            |
| `types.rs`    | Structs/enums that cross the JS ↔ Rust boundary. Mirrors `app/types/squish.ts`.                                                                           |
| `error.rs`    | `AppError`. Any `Result<T, AppError>` that fails becomes a string in JS.                                                                                  |
| `pipeline/`   | The actual work: `decode.rs`, one file per codec, and `mod.rs` which ties them together. Has no Tauri code, so it is unit-tested with plain `cargo test`. |

## Adding a command (the 4-step recipe)

1. Write `#[tauri::command] pub async fn my_command(arg: String) -> Result<Thing, AppError>` in `commands.rs`.
2. Add `commands::my_command` to `generate_handler![…]` in `lib.rs` — **forgetting this is the #1 "command not found" cause**.
3. Call it from Vue: `await invoke<Thing>("my_command", { arg: "x" })`. Argument names are **camelCase in JS, snake_case in Rust** (`on_event` ↔ `onEvent`).
4. If it touches a plugin (dialog, opener, fs…), add the permission string to `src-tauri/capabilities/default.json`.

## Rust concepts you'll meet here

- **`Result<T, E>` and `?`** — functions that can fail return a `Result`. `?` means "return the error to my caller if this failed".
- **Ownership / `&`** — `String` owns text, `&str` borrows it. Async commands must take owned types (`String`, `Vec<String>`), not borrowed ones.
- **`async` + `spawn_blocking`** — heavy CPU work (encoding) runs on a blocking thread pool so the UI never freezes.
- **`rayon`** — `par_iter()` runs a loop body across all CPU cores.
- **`Arc<AtomicBool>`** — a thread-safe flag; used for the Cancel button.
- **`#[derive(Serialize, Deserialize)]`** — auto-generates JSON conversion for a struct. `#[serde(rename_all = "camelCase")]` matches JS naming.

## Day-to-day commands (run in `src-tauri/`)

```bash
cargo test                              # run the pipeline tests
cargo clippy --all-targets -- -D warnings   # lints; fix what it asks
cargo doc --open                        # browse docs for every dependency, offline
cargo add <crate>                       # add a dependency
```

## Reading compiler errors

Rust errors are long but structured: the first line is _what_, the `-->` line is _where_, and the `help:` lines usually contain the fix. Fix the **first** error first — later ones are often fallout.

## Where to look things up

- Crate APIs: <https://docs.rs> (e.g. `docs.rs/image`, `docs.rs/oxipng`)
- Tauri v2: <https://v2.tauri.app>
- This project registers a Rust docs MCP server in `.mcp.json` (`rust-docs`, backed by docs.rs), so Claude Code can look up crate APIs directly.
