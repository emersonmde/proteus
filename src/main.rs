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
use tokio::time::Instant;
use tracing::info;

const AWS_REGION: &str = "us-east-1";
const MODEL_ID: &str = "anthropic.claude-3-5-sonnet-20240620-v1:0";

const WEBSITE_CATEGORIES: [&str; 100] = [
    "Tech News Portal",
    "Weather Forecast Dashboard",
    "Literary Magazine",
    "Cryptocurrency Exchange",
    "Indie Game Showcase",
    "Minimalist Lifestyle Blog",
    "Virtual Art Gallery",
    "Scientific Research Database",
    "Gourmet Recipe Collection",
    "Cyberpunk-themed E-commerce",
    "Environmental Conservation NGO",
    "Interactive Children's Storybook",
    "Futuristic Smart Home Control Panel",
    "Retro Arcade Game Leaderboard",
    "Luxury Watch Boutique",
    "Space Exploration News Site",
    "Mindfulness and Meditation App",
    "Sustainable Fashion Marketplace",
    "Graphic Novel Reader",
    "Artificial Intelligence Chatbot Interface",
    "Virtual Reality Travel Experience",
    "Experimental Music Composition Tool",
    "Citizen Journalism Platform",
    "Genealogy and Family Tree Builder",
    "Quantum Computing Educational Resource",
    "Artisanal Coffee Roaster",
    "Vintage Photograph Restoration Service",
    "Urban Gardening Community",
    "Blockchain Voting System",
    "Handcrafted Furniture Showcase",
    "Exotic Pet Care Information Center",
    "Fantasy Sports League Manager",
    "Collaborative Fiction Writing Platform",
    "Augmented Reality Art Installation Guide",
    "Sustainable Energy Solutions Marketplace",
    "Historical Reenactment Society",
    "Rare Book Collector's Network",
    "Underwater Photography Portfolio",
    "Personalized Nutrition Plan Generator",
    "Autonomous Vehicle News and Reviews",
    "Interactive Music Theory Tutor",
    "Minimalist Productivity Tool Suite",
    "Bespoke Tailoring Service",
    "Drone Racing League",
    "Foraging and Wild Edibles Guide",
    "Competitive Esports Team Profile",
    "Traditional Craft Preservation Society",
    "Tiny House Design and Living Blog",
    "Asteroid Mining Company",
    "Bioluminescent Organism Database",
    "Virtual Fashion Show Platform",
    "Neurofeedback Meditation Tracker",
    "Architectural Acoustics Consultancy",
    "Holographic Display Art Gallery",
    "Extinct Language Learning Resource",
    "Eco-friendly Packaging Design Showcase",
    "Competitive Rubik's Cube Solving Community",
    "Biohacking and Human Augmentation Forum",
    "Generative Art Creation Tool",
    "Sustainable Urban Planning Simulator",
    "Microgravity Experiment Database",
    "Artisanal Cheese Aging Tracker",
    "Cryptozoology Evidence Archive",
    "Futuristic Transportation Concept Showcase",
    "Interactive Periodic Table Explorer",
    "Experimental Theater Production Platform",
    "Bonsai Cultivation Masterclass",
    "Synthetic Biology Design Tool",
    "Psychedelic Art Therapy Resource",
    "Ancient Civilization Mystery Solver",
    "Extreme Weather Photography Gallery",
    "Fermentation Process Monitoring App",
    "Origami Design and Sharing Platform",
    "Ethical Hacking Tutorial Series",
    "Particle Physics Visualization Tool",
    "Retro Computing Emulator Collection",
    "Urban Exploration Safety Guide",
    "Collaborative Music Remix Platform",
    "Insect-based Cuisine Recipe Blog",
    "Sustainable Architecture Portfolio",
    "Time Capsule Creation Service",
    "Geocaching Adventure Planner",
    "Competitive Drone Obstacle Course",
    "Minimalist Watch Face Designer",
    "Fractal Art Generator",
    "Sensory Deprivation Float Center Locator",
    "Abandoned Places Photography Tour",
    "Acoustic Levitation Experiment Guide",
    "Bioluminescent Landscaping Service",
    "Sustainable Tiny House Community",
    "Exoplanet Discovery News Aggregator",
    "Interactive Optical Illusion Gallery",
    "Molecular Gastronomy Technique Database",
    "Autonomous Robot Building Competition",
    "Experimental Typography Showcase",
    "Lucid Dreaming Journal and Guide",
    "Underwater Cave Mapping Project",
    "Customizable Hologram Message Creator",
    "Zero-Waste Lifestyle Community",
    "Experimental Musical Instrument Maker",
];

const WEBSITE_GENERATION_PROMPT: &str = r#"As a senior UI designer at a top tech company like Apple or Google, create a visually stunning and highly functional HTML webpage for a "{{CATEGORY}}" website. Your design should showcase cutting-edge web design principles, exceptional user experience, and innovative visual elements, all tailored specifically to the {{CATEGORY}} niche.

Requirements:

1. Output a complete, self-contained HTML document with inline CSS. Start with <!DOCTYPE html> and end with </html>. Include no explanations or additional text.

2. Use semantic HTML5 structure with meticulous attention to accessibility (ARIA attributes, proper heading hierarchy, meaningful alt text).

3. Implement a sophisticated, unique design system tailored to the {{CATEGORY}}:
   - Curate a an expertly crafted color palette using CSS custom properties that resonates with the {{CATEGORY}} audience
   - Pair typography to enhance readability and convey the personality of a {{CATEGORY}} website
   - Create a responsive layout using advanced Flexbox and CSS Grid techniques, optimized for typical {{CATEGORY}} content
   - Include purposeful micro-animations and transitions that enhance the {{CATEGORY}} user experience
   - Incorporate at least three advanced CSS techniques (e.g., CSS shapes, backdrop-filter, clip-path, CSS masks, custom properties) in ways that complement the {{CATEGORY}} theme

4. Design an innovative and unique navbar that reflects the {{CATEGORY}}:
   - Breaks away from conventional styles while maintaining usability and relevance to {{CATEGORY}} navigation needs
   - Integrates seamlessly with the overall {{CATEGORY}} design
   - Utilizes creative interaction patterns that make sense for {{CATEGORY}} users
   - Adapts intelligently across different device sizes, considering how {{CATEGORY}} users might access the site
   - Employs cutting-edge CSS effects that enhance the {{CATEGORY}} brand

5. Expertly apply current design trends in the context of {{CATEGORY}}:
   - Use neumorphism, glassmorphism, or other cutting-edge effects where appropriate for the {{CATEGORY}}
   - Implement thoughtful microinteractions to guide user behavior in the {{CATEGORY}} context
   - Use typography as a central design element that captures the essence of {{CATEGORY}}
   - Create asymmetrical layouts that maintain visual balance while showcasing {{CATEGORY}} content
   - Consider both light and dark mode experiences that suit {{CATEGORY}} user preferences
   - Incorporate advanced scrolling effects (parallax, reveal animations) that enhance the {{CATEGORY}} narrative
   - Balance aesthetics and functionality perfectly for a {{CATEGORY}} website

6. Use Lorem Picsum for all images, styled appropriately for {{CATEGORY}}:
   - https://picsum.photos/seed/[RANDOM_SEED]/width/height
   Apply creative CSS treatments (e.g., masks, blend modes, filters) to elevate the design and align with {{CATEGORY}} aesthetics.

7. Craft a professional, innovative, and unique appearance that's perfect for {{CATEGORY}}:
   - Use whitespace strategically to create visual hierarchy suited to {{CATEGORY}} content
   - Design custom background patterns or subtle textures that add depth and relate to {{CATEGORY}}
   - Create innovative hover and focus states that delight {{CATEGORY}} users
   - Push the boundaries of web design while maintaining intuitive usability for the {{CATEGORY}} audience

8. Include engaging, brand-appropriate content for the {{CATEGORY}}:
   - Write persuasive and concise copy reflecting UX writing best practices and {{CATEGORY}} terminology
   - Craft headings and CTAs demonstrating conversion optimization understanding in the {{CATEGORY}} context
   - Use realistic placeholder content that showcases the design's potential and feels authentic to {{CATEGORY}}

9. Ensure efficient code practices to keep the HTML file size reasonable without compromising design quality or {{CATEGORY}}-specific content.

10. Incorporate innovative structural elements that serve the {{CATEGORY}} purpose:
    - Design an engaging hero section that immediately communicates the {{CATEGORY}} value proposition
    - Organize content to tell a compelling story about the {{CATEGORY}} brand or product
    - Thoughtfully integrate the navbar with other page elements for a cohesive {{CATEGORY}} experience

Remember, you are a top-tier UI designer creating a {{CATEGORY}} website. Your webpage should not only be visually stunning but also demonstrate a deep understanding of user-centered design principles, current industry best practices, and innovative approaches to web design, all perfectly tailored to {{CATEGORY}}. Craft every element with intention, ensuring the final product is worthy of a leading tech company's portfolio and perfectly suited to the {{CATEGORY}} niche.

Generate only the HTML and inline CSS for this {{CATEGORY}} webpage, starting with <!DOCTYPE html> and ending with </html>. Do not include any explanations or additional text."#;

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
    let category = WEBSITE_CATEGORIES.choose(&mut rand::thread_rng()).unwrap();
    info!("Starting webpage generation for category: {category}");

    let webpage_generation_start = Instant::now();
    let final_prompt = WEBSITE_GENERATION_PROMPT.replace("{{CATEGORY}}", category);
    let webpage_content = invoke_bedrock(client, final_prompt.to_string()).await?;

    // Remove any potential non-HTML content
    let html_start = webpage_content.find("<!DOCTYPE html>").unwrap_or(0);
    let html_end = webpage_content
        .rfind("</html>")
        .map(|i| i + 7)
        .unwrap_or(webpage_content.len());
    let cleaned_content = &webpage_content[html_start..html_end];
    let webpage_generation_duration = webpage_generation_start.elapsed();

    info!(
        category = %category,
        webpage_generation_time = ?webpage_generation_duration,
        "Webpage generation complete"
    );
    Ok(cleaned_content.to_string())
}
async fn invoke_bedrock(client: &Client, prompt: String) -> Result<String, BedrockConverseError> {
    let response = client
        .converse()
        .model_id(MODEL_ID)
        .inference_config(InferenceConfiguration::builder().temperature(1.0).build())
        .messages(
            aws_sdk_bedrockruntime::types::Message::builder()
                .role(ConversationRole::User)
                .content(ContentBlock::Text(prompt))
                .build()
                .map_err(|_| "failed to build message")?,
        )
        .send()
        .await;

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
    let start_time = Instant::now();
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

    let total_duration = start_time.elapsed();
    info!(
        request_time = ?total_duration,
        "Finished handling request"
    );
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
    info!("Server running on http://localhost:3000");

    serve(listener, app).await?;

    Ok(())
}
