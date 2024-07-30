use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::types::InferenceConfiguration;
use aws_sdk_bedrockruntime::{
    operation::converse::{ConverseError, ConverseOutput},
    types::{ContentBlock, ConversationRole},
    Client,
};
use axum::{extract::State, response::Html, routing::get, serve, Router};
use rand::prelude::IndexedRandom;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, RwLock};
use tracing::info;

const AWS_REGION: &str = "us-east-1";
const MODEL_ID: &str = "anthropic.claude-3-5-sonnet-20240620-v1:0";

const WEBSITE_CATEGORIES: [&str; 20] = [
    "Tech Blog",
    "News Website",
    "Travel Website",
    "E-commerce Store",
    "Personal Portfolio",
    "Restaurant Website",
    "Fitness Blog",
    "Photography Portfolio",
    "Corporate Website",
    "Educational Platform",
    "Music Streaming Service",
    "Real Estate Listings",
    "Fashion Blog",
    "Social Media Dashboard",
    "Online Marketplace",
    "Gaming Community",
    "Recipe Blog",
    "Nonprofit Organization",
    "Event Planning Service",
    "Job Board",
];

const WEBPAGE_GENERATION_PROMPT: &str = r#"Task: Generate a complete, random webpage with HTML, CSS, and JavaScript that exemplifies modern web design trends and best practices.

Context: You are a creative web developer and designer tasked with creating a unique, visually stunning, fully functional webpage. The webpage should be for the following website type, showcasing contemporary design aesthetics: "{{CATEGORY}}"

Instructions:
1. Create a webpage for the given website type with the following components:
   a. HTML structure:
      - Use semantic HTML5 tags
      - Ensure proper nesting and organization of elements
      - Implement accessibility best practices (ARIA attributes where appropriate)

   b. CSS styling:
      - Use inline CSS for this exercise, but structure it as if it were in a separate file
      - Implement a modern, cohesive design system including:
        * A harmonious color palette (consider using CSS variables for colors)
        * Typography: Use a combination of web-safe and Google Fonts for varied, attractive typography
        * Responsive layout using Flexbox and/or CSS Grid
        * Subtle animations and transitions for interactive elements
        * Implement at least one advanced CSS technique (e.g., CSS shapes, backdrop-filter, custom properties)
      - Use modern CSS features like:
        * Custom properties (CSS variables)
        * Calc() for dynamic calculations
        * Media queries for responsiveness
        * CSS Grid for complex layouts
      - Incorporate current design trends such as:
        * Neumorphism or glassmorphism effects
        * Microinteractions
        * Bold typography
        * Asymmetrical layouts
      - Ensure the design is fully responsive and looks good on mobile, tablet, and desktop

   c. JavaScript functionality:
      - Add meaningful interactivity relevant to the website type
      - Implement at least two dynamic elements (e.g., smooth scrolling, lazy loading images, dynamic content updates)
      - Use modern JavaScript (ES6+) features and best practices

2. Ensure the webpage looks professional and cutting-edge:
   - Apply principles of visual hierarchy and whitespace
   - Use high-quality, relevant images (use Lorem Picsum for image URLs, but style them appropriately)
   - Implement subtle background patterns or gradients where appropriate
   - Add thoughtful microinteractions and hover effects

3. Include realistic placeholder content that is relevant to the website type:
   - Generate engaging, contextually appropriate text for all content areas
   - Create compelling headings, subheadings, and calls-to-action
   - Use plausible names, titles, and descriptions
   - Adapt content style and tone to match the nature of the website type

4. Optimize for performance and user experience:
   - Use efficient CSS selectors
   - Implement lazy loading for images
   - Ensure the website is keyboard navigable
   - Add appropriate meta tags for SEO

Output: Provide the complete HTML document, including all inline CSS and JavaScript, ready to be rendered by a web browser. The output should contain only the HTML code, with no additional explanations or comments outside the code itself. Ensure the design is modern, stylish, and tailored to the given website type."#;

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
    let category = WEBSITE_CATEGORIES.choose(&mut rand::thread_rng()).unwrap();
    let prompt = WEBPAGE_GENERATION_PROMPT.replace("{{CATEGORY}}", category);

    let response = client
        .converse()
        .model_id(MODEL_ID)
        .inference_config(InferenceConfiguration::builder().temperature(0.8).build())
        .messages(
            aws_sdk_bedrockruntime::types::Message::builder()
                .role(ConversationRole::User)
                .content(ContentBlock::Text(prompt))
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
