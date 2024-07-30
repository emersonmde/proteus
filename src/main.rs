use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::{
    operation::converse::{ConverseError, ConverseOutput},
    types::{ContentBlock, ConversationRole},
    Client,
};
use axum::{extract::State, response::Html, routing::get, serve, Router};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, RwLock};
use tracing::info;

const AWS_REGION: &str = "us-east-1";
const MODEL_ID: &str = "anthropic.claude-3-5-sonnet-20240620-v1:0";

struct AppState {
    buffers: [String; 2],
    current_buffer: usize,
    client: Client,
    is_generating: Arc<Mutex<()>>,
}

#[derive(Debug)]
struct BedrockConverseError(String);

impl std::fmt::Display for BedrockConverseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Can't invoke '{}'. Reason: {}", MODEL_ID, self.0)
    }
}

impl std::error::Error for BedrockConverseError {}

impl From<&str> for BedrockConverseError {
    fn from(value: &str) -> Self {
        BedrockConverseError(value.to_string())
    }
}

impl From<&ConverseError> for BedrockConverseError {
    fn from(value: &ConverseError) -> Self {
        BedrockConverseError::from(match value {
            ConverseError::ModelTimeoutException(_) => "Model took too long",
            ConverseError::ModelNotReadyException(_) => "Model is not ready",
            _ => "Unknown",
        })
    }
}

async fn generate_webpage(client: &Client) -> Result<String, BedrockConverseError> {
    info!("Starting webpage generation");
    let prompt = "Generate a random webpage complete with HTML, inline CSS, and inline JavaScript \
    just as you would find it on the web. You response should be just the website HTML as it would \
    be returned from a webserver.";

    let response = client
        .converse()
        .model_id(MODEL_ID)
        .messages(
            aws_sdk_bedrockruntime::types::Message::builder()
                .role(ConversationRole::User)
                .content(ContentBlock::Text(prompt.to_string()))
                .build()
                .map_err(|_| "failed to build message")?,
        )
        .send()
        .await;

    info!("Finished webpage generation");
    match response {
        Ok(output) => get_converse_output_text(output),
        Err(e) => Err(e
            .as_service_error()
            .map(BedrockConverseError::from)
            .unwrap_or_else(|| BedrockConverseError("Unknown service error".into()))),
    }
}

fn get_converse_output_text(output: ConverseOutput) -> Result<String, BedrockConverseError> {
    let text = output
        .output()
        .ok_or("no output")?
        .as_message()
        .map_err(|_| "output not a message")?
        .content()
        .first()
        .ok_or("no content in message")?
        .as_text()
        .map_err(|_| "content is not text")?
        .to_string();
    Ok(text)
}

async fn serve_webpage(State(state): State<Arc<RwLock<AppState>>>) -> Html<String> {
    info!("Handling new request");
    let current_buffer;
    {
        let app_state = state.read().await;
        current_buffer = app_state.current_buffer;

        let state_clone = Arc::clone(&state);
        let is_generating_clone = Arc::clone(&app_state.is_generating);
        tokio::spawn(async move {
            if let Ok(_lock) = is_generating_clone.try_lock() {
                let new_content = generate_webpage(&state_clone.read().await.client)
                    .await
                    .unwrap_or_else(|e| {
                        eprintln!("Error generating webpage: {}", e);
                        "Error generating content".to_string()
                    });

                let mut app_state = state_clone.write().await;
                app_state.buffers[1 - current_buffer] = new_content;
                app_state.current_buffer = 1 - current_buffer;
            };
        });
    }

    info!("Finished handling request");
    Html(state.read().await.buffers[current_buffer].clone())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .region(AWS_REGION)
        .load()
        .await;
    let client = Client::new(&sdk_config);

    let initial_content = generate_webpage(&client).await?;
    let app_state = Arc::new(RwLock::new(AppState {
        buffers: [initial_content, String::new()],
        current_buffer: 0,
        client,
        is_generating: Arc::new(Mutex::new(())),
    }));

    let app = Router::new()
        .route("/", get(serve_webpage))
        .with_state(app_state);

    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    println!("Server running on http://localhost:3000");

    serve(listener, app).await?;

    Ok(())
}
