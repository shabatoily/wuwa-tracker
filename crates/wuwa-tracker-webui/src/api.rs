//! 실행 환경에 따라 Tauri IPC 또는 HTTP API를 선택하는 transport 계층입니다.

use crate::types::*;
use gloo_net::http::Request;
use js_sys::{Array, Reflect, Uint8Array};
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::{prelude::*, JsCast};

const DEV_API_HOST: &str = "http://localhost:3000";

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(
        js_namespace = ["window", "__TAURI__", "core"],
        js_name = invoke,
        catch
    )]
    async fn invoke_with_args(command: &str, args: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(
        js_namespace = ["window", "__TAURI__", "core"],
        js_name = invoke,
        catch
    )]
    async fn invoke_without_args(command: &str) -> Result<JsValue, JsValue>;
}

pub async fn fetch_config(lang: &str) -> Result<ConfigResponse, String> {
    if is_tauri() {
        invoke("get_config", &serde_json::json!({"lang": lang})).await
    } else {
        get(&format!("/api/config?lang={lang}")).await
    }
}

pub async fn fetch_players() -> Result<PlayersResponse, String> {
    if is_tauri() {
        invoke_no_args("list_players").await
    } else {
        get("/api/players").await
    }
}

pub async fn fetch_stats(player_id: &str, lang: &str) -> Result<StatsResponse, String> {
    if is_tauri() {
        invoke("get_stats", &PlayerArgs { player_id, lang }).await
    } else {
        get(&format!("/api/stats/{player_id}?lang={lang}")).await
    }
}

pub async fn scan_path(path: &str) -> Result<ScanResponse, String> {
    if is_tauri() {
        invoke("scan_url", &PathArgs { path }).await
    } else {
        post("/api/scan", &PathArgs { path }).await
    }
}

pub async fn track_url(url: &str, lang: &str) -> Result<StatsResponse, String> {
    if is_tauri() {
        invoke("track_url", &UrlArgs { url, lang }).await
    } else {
        post(&format!("/api/track?lang={lang}"), &UrlArgs { url, lang }).await
    }
}

pub async fn upload_json(data: &serde_json::Value, lang: &str) -> Result<StatsResponse, String> {
    if is_tauri() {
        invoke("upload_json", &UploadArgs { data, lang }).await
    } else {
        post(&format!("/api/upload?lang={lang}"), data).await
    }
}

pub async fn export_report(player_id: &str, format: &str, lang: &str) -> Result<(), String> {
    if is_tauri() {
        let response: ExportResponse = invoke(
            "export_report",
            &ExportArgs {
                player_id,
                format,
                lang,
            },
        )
        .await?;
        download_bytes(
            &response.content,
            &response.content_type,
            &response.filename,
        )
    } else {
        let response = Request::get(&api_url(&format!(
            "/api/export/{player_id}?format={format}&lang={lang}"
        )))
        .send()
        .await
        .map_err(display_error)?;
        if !response.ok() {
            return Err(format!("report export failed: {}", response.status()));
        }
        let bytes = response.binary().await.map_err(display_error)?;
        download_bytes(
            &bytes,
            response
                .headers()
                .get("content-type")
                .as_deref()
                .unwrap_or("application/octet-stream"),
            &format!("report_{player_id}.{format}"),
        )
    }
}

async fn get<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let response = Request::get(&api_url(path))
        .send()
        .await
        .map_err(display_error)?;
    response.json().await.map_err(display_error)
}

async fn post<T: DeserializeOwned, B: Serialize + ?Sized>(
    path: &str,
    body: &B,
) -> Result<T, String> {
    let body = serde_json::to_string(body).map_err(display_error)?;
    let response = Request::post(&api_url(path))
        .header("Content-Type", "application/json")
        .body(body)
        .map_err(display_error)?
        .send()
        .await
        .map_err(display_error)?;
    response.json().await.map_err(display_error)
}

async fn invoke<T: DeserializeOwned>(command: &str, args: &impl Serialize) -> Result<T, String> {
    let args = serde_wasm_bindgen::to_value(args).map_err(display_error)?;
    let value = invoke_with_args(command, args).await.map_err(js_error)?;
    serde_wasm_bindgen::from_value(value).map_err(display_error)
}

async fn invoke_no_args<T: DeserializeOwned>(command: &str) -> Result<T, String> {
    let value = invoke_without_args(command).await.map_err(js_error)?;
    serde_wasm_bindgen::from_value(value).map_err(display_error)
}

fn is_tauri() -> bool {
    Reflect::get(
        &web_sys::window().expect("window").into(),
        &JsValue::from_str("__TAURI__"),
    )
    .map(|value| value.is_object())
    .unwrap_or(false)
}

fn api_url(path: &str) -> String {
    // Trunk 개발 서버에서는 정적 파일과 API가 서로 다른 origin에서 실행됩니다.
    if is_trunk_dev_server() {
        format!("{DEV_API_HOST}{path}")
    } else {
        path.to_string()
    }
}

fn is_trunk_dev_server() -> bool {
    web_sys::window()
        .and_then(|window| window.location().port().ok())
        .map(|port| port == "8080" || port == "1420")
        .unwrap_or(false)
}

fn download_bytes(content: &[u8], content_type: &str, filename: &str) -> Result<(), String> {
    let bytes = Uint8Array::from(content);
    let parts = Array::new();
    parts.push(&bytes.buffer());
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(content_type);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options)
        .map_err(js_error)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(js_error)?;
    let document = web_sys::window()
        .expect("window")
        .document()
        .expect("document");
    let anchor = document
        .create_element("a")
        .map_err(js_error)?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|_| "failed to create download link".to_string())?;
    anchor.set_href(&url);
    anchor.set_download(filename);
    anchor.click();
    web_sys::Url::revoke_object_url(&url).map_err(js_error)
}

fn js_error(error: JsValue) -> String {
    error
        .as_string()
        .unwrap_or_else(|| format!("JavaScript error: {error:?}"))
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
