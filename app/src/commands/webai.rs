use crate::error::{AppError, AppResult};
use aineer_webai::WebAiEngine;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebAiProviderInfo {
    pub id: String,
    pub name: String,
    pub models: Vec<WebAiModelInfo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebAiModelInfo {
    pub id: String,
    pub name: String,
    pub default: bool,
}

#[tauri::command]
pub async fn webai_list_providers() -> AppResult<Vec<WebAiProviderInfo>> {
    let handle =
        crate::app_handle().ok_or_else(|| AppError::Gateway("app handle not available".into()))?;
    let engine = WebAiEngine::new(handle.clone());
    let providers = engine
        .list_providers()
        .into_iter()
        .map(|p| {
            let models = engine
                .list_models(&p.id)
                .into_iter()
                .map(|m| WebAiModelInfo {
                    id: m.id,
                    name: m.name,
                    default: m.default,
                })
                .collect();
            WebAiProviderInfo {
                id: p.id,
                name: p.name,
                models,
            }
        })
        .collect();
    Ok(providers)
}

#[tauri::command]
pub async fn webai_start_auth(provider_id: String) -> AppResult<String> {
    let handle =
        crate::app_handle().ok_or_else(|| AppError::Gateway("app handle not available".into()))?;
    let engine = WebAiEngine::new(handle.clone());
    let providers = engine.list_providers();
    let config = providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| AppError::Gateway(format!("unknown provider: {provider_id}")))?;

    let creds = aineer_webai::webauth::start_webauth(handle, config)
        .await
        .map_err(|e| AppError::Gateway(format!("webauth failed: {e}")))?;

    Ok(creds.provider_id)
}

#[tauri::command]
pub async fn webai_list_authenticated() -> AppResult<Vec<String>> {
    Ok(aineer_webai::webauth::list_authenticated())
}

#[tauri::command]
pub async fn webai_logout(provider_id: String) -> AppResult<()> {
    aineer_webai::webauth::logout(&provider_id)
        .map_err(|e| AppError::Gateway(format!("logout failed: {e}")))?;
    Ok(())
}
