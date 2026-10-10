//! Talking to a language model through the OpenAI-compatible chat API, which local
//! servers (Ollama, LM Studio) and most providers offer. Tandem ships no key of its
//! own: the player brings a local model or their own account.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// A provider the player can pick, with what Tandem knows about it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
    pub base_url: &'static str,
    /// Runs on the player's computer: free, private, slower.
    pub local: bool,
    pub needs_key: bool,
    /// Where to get a key or the software.
    pub help_url: &'static str,
    /// Requests sent at once by default.
    pub concurrency: u8,
}

pub const PRESETS: &[Preset] = &[
    Preset {
        id: "ollama",
        label: "Ollama",
        base_url: "http://localhost:11434/v1",
        local: true,
        needs_key: false,
        help_url: "https://ollama.com/download",
        concurrency: 1,
    },
    Preset {
        id: "lmstudio",
        label: "LM Studio",
        base_url: "http://localhost:1234/v1",
        local: true,
        needs_key: false,
        help_url: "https://lmstudio.ai",
        concurrency: 1,
    },
    Preset {
        id: "openai",
        label: "OpenAI",
        base_url: "https://api.openai.com/v1",
        local: false,
        needs_key: true,
        help_url: "https://platform.openai.com/api-keys",
        concurrency: 4,
    },
    Preset {
        id: "anthropic",
        label: "Anthropic (Claude)",
        base_url: "https://api.anthropic.com/v1",
        local: false,
        needs_key: true,
        help_url: "https://console.anthropic.com/settings/keys",
        concurrency: 4,
    },
    Preset {
        id: "mistral",
        label: "Mistral AI",
        base_url: "https://api.mistral.ai/v1",
        local: false,
        needs_key: true,
        help_url: "https://console.mistral.ai/api-keys",
        concurrency: 4,
    },
    Preset {
        id: "gemini",
        label: "Google Gemini",
        base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
        local: false,
        needs_key: true,
        help_url: "https://aistudio.google.com/apikey",
        concurrency: 4,
    },
    Preset {
        id: "openrouter",
        label: "OpenRouter",
        base_url: "https://openrouter.ai/api/v1",
        local: false,
        needs_key: true,
        help_url: "https://openrouter.ai/keys",
        concurrency: 4,
    },
    Preset {
        id: "deepseek",
        label: "DeepSeek",
        base_url: "https://api.deepseek.com/v1",
        local: false,
        needs_key: true,
        help_url: "https://platform.deepseek.com/api_keys",
        concurrency: 4,
    },
    Preset {
        id: "custom",
        label: "Autre (compatible OpenAI)",
        base_url: "",
        local: false,
        needs_key: false,
        help_url: "",
        concurrency: 2,
    },
];

pub fn preset(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.id == id)
}

/// The provider the player configured (the key is stored apart, see `secrets`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub preset: String,
    pub base_url: String,
    pub model: String,
    pub concurrency: u8,
}

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    model: String,
    key: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Completion {
    pub text: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Deserialize)]
struct Usage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
}

#[derive(Deserialize)]
struct ModelList {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

impl Client {
    pub fn new(
        http: reqwest::Client,
        config: &ProviderConfig,
        key: Option<String>,
    ) -> Result<Self> {
        let base_url = config.base_url.trim().trim_end_matches('/').to_owned();
        if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
            return Err(Error::Translation(
                "Adresse du service de traduction invalide.".into(),
            ));
        }
        Ok(Self {
            http,
            base_url,
            model: config.model.trim().to_owned(),
            key: key.filter(|k| !k.trim().is_empty()),
        })
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    fn request(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let builder = match &self.key {
            Some(key) => builder.bearer_auth(key).header("x-api-key", key),
            None => builder,
        };
        // Anthropic asks for a version header; other services ignore it.
        builder.header("anthropic-version", "2023-06-01")
    }

    /// Models the service offers, sorted.
    pub async fn models(&self) -> Result<Vec<String>> {
        let response = self
            .request(self.http.get(format!("{}/models", self.base_url)))
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .map_err(connection_error)?;
        let response = check(response).await?;
        let list: ModelList = response
            .json()
            .await
            .map_err(|e| Error::Translation(format!("Réponse inattendue du service : {e}")))?;
        let mut models: Vec<String> = list.data.into_iter().map(|m| m.id).collect();
        models.sort();
        Ok(models)
    }

    /// One chat completion, asking for JSON. Local models can take minutes on a large
    /// batch, hence the long timeout.
    pub async fn complete(&self, system: &str, user: &str, json: bool) -> Result<Completion> {
        if self.model.is_empty() {
            return Err(Error::Translation(
                "Choisis un modèle dans les réglages de traduction.".into(),
            ));
        }
        let mut body = serde_json::json!({
            "model": self.model,
            "temperature": 0.2,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user },
            ],
        });
        if json {
            body["response_format"] = serde_json::json!({ "type": "json_object" });
        }
        let response = self
            .request(
                self.http
                    .post(format!("{}/chat/completions", self.base_url)),
            )
            .timeout(Duration::from_secs(600))
            .json(&body)
            .send()
            .await
            .map_err(connection_error)?;
        let response = check(response).await?;
        let chat: ChatResponse = response
            .json()
            .await
            .map_err(|e| Error::Translation(format!("Réponse inattendue du service : {e}")))?;
        let usage = chat.usage.unwrap_or(Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
        });
        Ok(Completion {
            text: chat
                .choices
                .into_iter()
                .next()
                .and_then(|c| c.message.content)
                .unwrap_or_default(),
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
        })
    }
}

fn connection_error(err: reqwest::Error) -> Error {
    if err.is_timeout() {
        Error::Translation("Le service de traduction ne répond pas (délai dépassé).".into())
    } else if err.is_connect() {
        Error::Translation("Impossible de joindre le service de traduction. S'il est local (Ollama, LM Studio), vérifie qu'il est lancé.".into())
    } else {
        Error::Translation(format!("Erreur réseau : {err}"))
    }
}

/// Turns an error status into a readable message, with the service's own explanation.
async fn check(response: reqwest::Response) -> Result<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    let detail = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| {
            v.pointer("/error/message")
                .or_else(|| v.pointer("/error"))
                .or_else(|| v.pointer("/message"))
                .and_then(|m| m.as_str().map(str::to_owned))
        })
        .unwrap_or_else(|| body.chars().take(200).collect());
    let message = match status.as_u16() {
        401 | 403 => "Clé API refusée par le service.".to_owned(),
        404 => "Modèle ou adresse introuvable chez le service.".to_owned(),
        429 => "Trop de requêtes ou crédit épuisé chez le service.".to_owned(),
        code => format!("Le service a répondu {code}."),
    };
    Err(Error::Translation(if detail.trim().is_empty() {
        message
    } else {
        format!("{message} ({})", detail.trim())
    }))
}

/// Whether an error is worth retrying after a pause.
pub fn is_transient(err: &Error) -> bool {
    match err {
        Error::Translation(message) => {
            message.starts_with("Trop de requêtes")
                || message.contains("délai dépassé")
                || message.contains("répondu 5")
        }
        _ => false,
    }
}

/// Instructions for translating a batch of game texts.
pub fn system_prompt(language: &str) -> String {
    format!(
        "You are a professional translator of Minecraft mods and modpacks. Translate every value of the JSON object \
         the user sends from English into {language}.\n\
         Rules:\n\
         - Reply with only a JSON object that has exactly the same keys. No comments, no code fences.\n\
         - The keys identify the texts in the game (for example block.mod.copper_gear or a quest number): use them \
           as hints of what each text is, but never translate or change them.\n\
         - Every token such as ⟦0⟧ stands for a game code: keep each one exactly once and unchanged, placed where the \
           grammar of {language} needs it.\n\
         - Follow the glossary strictly: it holds the official {language} names used by Minecraft and the player.\n\
         - Keep mod names, brand names and proper nouns (characters, places, factions) unless the glossary translates them.\n\
         - Use the capitalization conventions of {language} Minecraft for item and block names, and keep the \
           grammatical number of the English (singular stays singular).\n\
         - Translate similar names the same way: a series of blocks or items must read as one consistent family.\n\
         - Keep the meaning, tone and length: these texts appear in the game's interface, tooltips and quest books.\n\
         - Keep key bindings and technical words players know in English (RPM, FE, NBT, JEI) as they are."
    )
}

/// The user message: context, the glossary terms this batch uses, then the texts.
pub fn user_prompt(context: &str, glossary: &[(&str, &str)], texts: &[(String, String)]) -> String {
    let mut out = String::new();
    if !context.is_empty() {
        out.push_str(&format!("Source: {context}\n"));
    }
    if !glossary.is_empty() {
        out.push_str("Glossary:\n");
        for (term, translation) in glossary {
            out.push_str(&format!("- {term} = {translation}\n"));
        }
    }
    let object: serde_json::Map<String, serde_json::Value> = texts
        .iter()
        .map(|(id, text)| (id.clone(), serde_json::Value::String(text.clone())))
        .collect();
    out.push_str("\nJSON to translate:\n");
    out.push_str(&serde_json::to_string_pretty(&object).unwrap_or_default());
    out
}

/// The JSON object in a model's answer, tolerating code fences and chatter around it.
pub fn parse_answer(text: &str) -> std::collections::HashMap<String, String> {
    let start = text.find('{');
    let end = text.rfind('}');
    let (Some(start), Some(end)) = (start, end) else {
        return Default::default();
    };
    if end <= start {
        return Default::default();
    }
    let Ok(serde_json::Value::Object(map)) =
        serde_json::from_str::<serde_json::Value>(&text[start..=end])
    else {
        return Default::default();
    };
    map.into_iter()
        .filter_map(|(k, v)| match v {
            serde_json::Value::String(s) => Some((k, s)),
            serde_json::Value::Array(lines) => {
                let joined: Vec<String> = lines
                    .into_iter()
                    .filter_map(|l| l.as_str().map(str::to_owned))
                    .collect();
                (!joined.is_empty()).then(|| (k, joined.join("\n")))
            }
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_answers_with_noise() {
        let answer =
            "Sure! Here it is:\n```json\n{\"1\": \"Presse\", \"2\": 3, \"3\": [\"a\", \"b\"]}\n```";
        let parsed = parse_answer(answer);
        assert_eq!(parsed.get("1").map(String::as_str), Some("Presse"));
        assert_eq!(parsed.get("3").map(String::as_str), Some("a\nb"));
        assert!(!parsed.contains_key("2"));
        assert!(parse_answer("no json").is_empty());
        assert!(parse_answer("} {").is_empty());
    }

    #[test]
    fn builds_prompts() {
        let user = user_prompt(
            "Create",
            &[("Crafting Table", "Établi")],
            &[("1".into(), "Hold ⟦0⟧".into())],
        );
        assert!(user.starts_with("Source: Create\nGlossary:\n- Crafting Table = Établi\n"));
        assert!(user.contains("\"1\": \"Hold ⟦0⟧\""));
        assert!(system_prompt("French (France)").contains("into French (France)"));
    }

    #[test]
    fn rejects_bad_urls() {
        let config = ProviderConfig {
            base_url: "localhost:11434".into(),
            ..Default::default()
        };
        assert!(Client::new(reqwest::Client::new(), &config, None).is_err());
        assert!(preset("ollama").is_some_and(|p| p.local));
    }
}
