//! `yana-rt os credential models <provider>`: the provider's own model list,
//! fetched with the stored key, so an app can show it without holding the key.
//!
//! The address and header style are fixed here, redirects are not followed, the
//! body is size limited, and a body that repeats the key is refused. What is
//! printed is the provider's JSON; each app reads the shape it already knows.

use super::{account_for, clean_key, inside_agent_command, KeyStore, NativeKeyStore};
use anyhow::{bail, Result};

/// Far above any provider's list (OpenRouter's is the largest, under 1 MB).
const MAX_LIST_BYTES: u64 = 4_000_000;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Auth {
    Bearer,
    Anthropic,
    Google,
}

/// The same table both apps used when they held the key (Studio's
/// `model-discovery.cjs`, the chat app's `ModelCatalog.swift`).
const ENDPOINTS: &[(&str, &str, Auth)] = &[
    ("anthropic", "https://api.anthropic.com/v1/models?limit=1000", Auth::Anthropic),
    ("openai", "https://api.openai.com/v1/models", Auth::Bearer),
    ("gemini", "https://generativelanguage.googleapis.com/v1beta/models?pageSize=1000", Auth::Google),
    ("groq", "https://api.groq.com/openai/v1/models", Auth::Bearer),
    ("deepseek", "https://api.deepseek.com/models", Auth::Bearer),
    ("openrouter", "https://openrouter.ai/api/v1/models", Auth::Bearer),
    ("xai", "https://api.x.ai/v1/language-models", Auth::Bearer),
    ("novita", "https://api.novita.ai/openai/v1/models", Auth::Bearer),
    ("nvidia", "https://integrate.api.nvidia.com/v1/models", Auth::Bearer),
    ("minimax", "https://api.minimaxi.com/v1/models", Auth::Bearer),
    ("glm", "https://open.bigmodel.cn/api/paas/v4/models", Auth::Bearer),
    ("huggingface", "https://router.huggingface.co/v1/models", Auth::Bearer),
    ("kimi", "https://api.moonshot.ai/v1/models", Auth::Bearer),
];

fn endpoint(provider: &str) -> Option<(&'static str, Auth)> {
    ENDPOINTS.iter().find(|(name, _, _)| *name == provider).map(|(_, url, auth)| (*url, *auth))
}

fn fetch(url: &str, auth: Auth, key: &str) -> Result<String> {
    let agent = crate::model::provider::build_agent();
    let request = agent.get(url).header("accept", "application/json");
    let request = match auth {
        Auth::Bearer => request.header("authorization", format!("Bearer {key}")),
        Auth::Anthropic => request.header("x-api-key", key).header("anthropic-version", "2023-06-01"),
        Auth::Google => request.header("x-goog-api-key", key),
    };
    // Neither the transport error nor the response body is passed on: either could repeat the request.
    let Ok(mut response) = request.call() else {
        bail!("the provider could not be reached");
    };
    if !response.status().is_success() {
        bail!("HTTP {}", response.status().as_u16());
    }
    let Ok(body) = response.body_mut().with_config().limit(MAX_LIST_BYTES).read_to_string() else {
        bail!("the model list could not be read, or is larger than {MAX_LIST_BYTES} bytes");
    };
    if body.contains(key) {
        bail!("the provider's answer repeats the key, so it is not printed");
    }
    Ok(body)
}

fn list(provider: &str, from_agent: bool, store: &dyn KeyStore) -> Result<String> {
    super::refuse_agent(from_agent)?;
    let account = account_for(provider)?;
    let selected = crate::model::catalog::try_select_provider(provider).map_err(anyhow::Error::msg)?;
    let Some((url, auth)) = endpoint(selected.name()) else {
        bail!("provider '{}' has no model list here; type the model ID", selected.name());
    };
    let Some(key) = store.get(&account)? else {
        bail!("no key is stored for '{}'", selected.name());
    };
    fetch(url, auth, &clean_key(&key)?)
}

pub(crate) fn cmd_models(provider: &str, extra: &[String]) -> Result<()> {
    super::refuse_extra(extra)?;
    if std::env::var_os(super::OPT_OUT_ENV).is_some() {
        anyhow::bail!("{} is set, so stored keys are not used", super::OPT_OUT_ENV);
    }
    println!("{}", list(provider, inside_agent_command(), &NativeKeyStore)?);
    Ok(())
}

#[cfg(test)]
mod tests;
