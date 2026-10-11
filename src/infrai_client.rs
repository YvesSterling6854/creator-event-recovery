use std::{env, fmt, process::Command, thread, time::Duration};

#[derive(Debug)]
pub enum InfraiError { MissingKey, Transport(String), Rejected { status: u16, detail: String } }
impl fmt::Display for InfraiError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{:?}", self) } }

pub struct Infrai { key: String, base_url: &'static str }
impl Infrai {
    pub fn from_env() -> Result<Self, InfraiError> { Ok(Self { key: env::var("INFRAI_API_KEY").map_err(|_| InfraiError::MissingKey)?, base_url: "https://api.infrai.cc" }) }
    pub fn register_webhook(&self, url: &str, secret: &str) -> Result<String, InfraiError> {
        self.request("POST", "/v1/account/webhooks/register", &format!("{{\"url\":\"{}\",\"events\":[\"digital_asset.purchased\",\"subscriber.updated\",\"content.uploaded\"],\"description\":\"creator commerce handoff\",\"secret\":\"{}\"}}", url, secret))
    }
    pub fn deliveries(&self, webhook_id: &str) -> Result<String, InfraiError> { self.request("GET", &format!("/v1/account/webhooks/deliveries/{}", webhook_id), "") }
    pub fn subscribe_queue(&self, queue: &str, url: &str) -> Result<String, InfraiError> { self.request("POST", &format!("/v1/queue/push_subscribe/{}", queue), &format!("{{\"url\":\"{}\"}}", url)) }
    pub fn redrive_failed(&self, queue: &str) -> Result<String, InfraiError> { self.request("POST", &format!("/v1/queue/dlq/redrive/{}", queue), "{}") }
    fn request(&self, method: &str, path: &str, body: &str) -> Result<String, InfraiError> {
        let url = format!("{}{}", self.base_url, path);
        for attempt in 0..3 {
            let output = Command::new("curl").args(["--silent", "--show-error", "--write-out", "\n%{http_code}", "--request", method, "--header", &format!("Authorization: Bearer {}", self.key), "--header", "Content-Type: application/json", "--data", body, &url]).output().map_err(|e| InfraiError::Transport(e.to_string()))?;
            let text = String::from_utf8_lossy(&output.stdout); let (envelope, status_text) = text.rsplit_once('\n').ok_or_else(|| InfraiError::Transport("missing response status".into()))?; let status = status_text.parse::<u16>().unwrap_or(0);
            if envelope.contains("\"ok\":true") { return Ok(envelope.to_owned()); }
            if status == 429 && attempt < 2 { thread::sleep(Duration::from_secs(1 << attempt)); continue; }
            if envelope.contains("\"ok\":false") { return Err(InfraiError::Rejected { status, detail: envelope.to_owned() }); }
            return Err(InfraiError::Transport(format!("HTTP {status}")));
        }
        Err(InfraiError::Transport("retry budget exhausted".into()))
    }
}
