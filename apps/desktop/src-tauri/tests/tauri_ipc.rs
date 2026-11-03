use serde_json::{json, Value};
use tanzeel_desktop_lib::{configure_app, AppState};
use tanzeel_storage::Storage;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{
    get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY,
};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewWindow};

fn invoke(webview: &WebviewWindow<MockRuntime>, cmd: &str, body: Value) -> Result<Value, Value> {
    get_ipc_response(
        webview,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: if cfg!(any(windows, target_os = "android")) {
                "http://tauri.localhost"
            } else {
                "tauri://localhost"
            }
            .parse()
            .expect("test URL should be valid"),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|response| {
        response
            .deserialize::<Value>()
            .expect("IPC response should contain JSON")
    })
}

fn app_with_in_memory_storage() -> App<MockRuntime> {
    let storage = Storage::open_in_memory().expect("in-memory storage should open");
    configure_app(mock_builder().manage(AppState::new(storage)))
        .build(mock_context(noop_assets()))
        .expect("mock Tauri app should build")
}

fn webview_for(app: &App<MockRuntime>) -> WebviewWindow<MockRuntime> {
    tauri::WebviewWindowBuilder::new(app, "main", Default::default())
        .build()
        .expect("mock webview should build")
}

#[test]
fn settings_round_trip_through_tauri_ipc_and_sqlite() {
    let app = app_with_in_memory_storage();
    let webview = webview_for(&app);

    let defaults =
        invoke(&webview, "get_settings", json!({})).expect("defaults should load through IPC");
    assert_eq!(
        defaults,
        json!({ "language": "ar", "theme": "system", "downloadDir": "" })
    );

    let settings = json!({
        "language": "en",
        "theme": "dark",
        "downloadDir": "C:\\Users\\example\\Downloads"
    });
    let saved = invoke(&webview, "save_settings", json!({ "settings": settings }))
        .expect("settings should save through IPC");
    assert_eq!(saved, Value::Null);

    let loaded =
        invoke(&webview, "get_settings", json!({})).expect("settings should load through IPC");
    assert_eq!(loaded, settings);
}

#[test]
fn invalid_settings_are_rejected_through_tauri_ipc() {
    let app = app_with_in_memory_storage();
    let webview = webview_for(&app);

    let error = invoke(
        &webview,
        "save_settings",
        json!({
            "settings": {
                "language": "ar",
                "theme": "system",
                "downloadDir": "Downloads"
            }
        }),
    )
    .expect_err("relative download directory should be rejected through IPC");

    assert!(
        error.to_string().contains("absolute"),
        "unexpected IPC error: {error}"
    );
}
