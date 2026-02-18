use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct GeminiClient {
    api_key: String,
    client: Client,
}

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    temperature: f32,
    top_p: f32,
    top_k: i32,
    max_output_tokens: i32,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: ResponseContent,
}

#[derive(Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize)]
struct ResponsePart {
    text: String,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
        }
    }

    pub async fn generate_commit_message(&self, diff: &str, branch: &str) -> Result<String> {
        let prompt = format!(
            "You are an expert developer. Generate a short, concise, professional conventional commit message based on the following git diff and branch name. 
            Follow the format: <type>(<scope>): <description>
            Do not include any conversational filler, markdown blocks, or explanations. Just the message.

            Branch: {branch}

            Diff:
{diff}"
        );

        self.generate_content(prompt, 100).await
    }

    pub async fn explain_rebase_conflict(&self, diff: &str) -> Result<String> {
        let prompt = format!(
            "You are an expert developer. A git rebase has failed due to conflicts. 
            Analyze the following diff which contains conflict markers (<<<<<<<, =======, >>>>>>>).
            Explain in plain English why the conflict happened and suggest how to resolve it.
            Be concise and professional. Do not used markdown formatting in your response.

            Conflict Diff:
{diff}"
        );

        self.generate_content(prompt, 500).await
    }

    async fn generate_content(&self, prompt: String, max_tokens: i32) -> Result<String> {
        let url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent";

        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part { text: prompt }],
            }],
            generation_config: Some(GenerationConfig {
                temperature: 0.2,
                top_p: 0.8,
                top_k: 40,
                max_output_tokens: max_tokens,
            }),
        };

        let response = self
            .client
            .post(url)
            .header("x-goog-api-key", &self.api_key)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Gemini API")?;

        if !response.status().is_success() {
            let error_body = response.text().await?;
            return Err(anyhow::anyhow!("Gemini API error: {error_body}"));
        }

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .context("Failed to parse Gemini API response")?;

        let text = gemini_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.trim().to_string())
            .context("No response text found in Gemini response")?;

        Ok(text)
    }
}
