use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashMap,
    env,
    fs,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex as AsyncMutex;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeFile;
use webbrowser;

#[derive(Clone)]
struct AppState {
    client: Client,
    firebase_config: FirebaseConfig,
    auth_data: Arc<AsyncMutex<Option<AuthData>>>,
    auth_state_path: PathBuf,
}

#[derive(Clone)]
struct FirebaseConfig {
    api_key: String,
    project_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct AuthData {
    refresh_token: String,
    id_token: String,
    expires_at: u64,
}

#[derive(Serialize)]
struct TokenResponse {
    access_token: String,
}

const APP_BASE_URL: &str = "https://app.cline.bot";

#[tokio::main]
async fn main() {
    let firebase_config = FirebaseConfig {
        api_key: "AIzaSyC5rx59Xt8UgwdU3PCfzUF7vCwmp9-K2vk".to_string(),
        project_id: "cline-prod".to_string(),
    };

    let client = Client::new();

    // Determine auth state path from environment variable or default to current dir
    let auth_state_dir = env::var("AUTH_STATE_DIR").unwrap_or_else(|_| ".".to_string());
    let mut auth_state_path = PathBuf::from(auth_state_dir);
    fs::create_dir_all(&auth_state_path)
        .unwrap_or_else(|e| panic!("Failed to create auth state directory: {}", e));
    auth_state_path.push("auth_state.json");
    
    // Load initial state from file
    let initial_auth_data: Option<AuthData> = fs::read_to_string(&auth_state_path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok());

    if initial_auth_data.is_some() {
        println!("✅ Loaded persisted login state from {}", auth_state_path.display());
    }

    let auth_data = Arc::new(AsyncMutex::new(initial_auth_data.clone()));

    let app_state = AppState {
        client,
        firebase_config,
        auth_data,
        auth_state_path,
    };

    let app = Router::new()
        .route("/login", get(login_handler))
        .route_service("/handle-auth.html", ServeFile::new("handle-auth.html"))
        .route("/callback", post(callback_handler))
        .route("/token", get(token_handler))
        .route("/raw-token", get(raw_token_handler))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let host_binding = env::var("HOST_BINDING").unwrap_or_else(|_| "127.0.0.1".to_string());
    let addr = format!("{}:8888", host_binding);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("🚀 Standalone Auth Service (Rust) listening on http://{}", addr);
    println!("\n--- Instructions ---");
    println!("1. To start, navigate to http://localhost:8888/login in your browser.");
    println!("2. Complete the login process on the provider's page.");
    println!("3. You will be automatically redirected, and the service will handle the rest.");
    println!("4. Once the browser shows \"Authentication Successful!\", the service is ready.");
    println!("5. Your application can get the bearer token from:");
    println!("   - JSON format: http://localhost:8888/token");
    println!("   - Plain text: http://localhost:8888/raw-token");
    println!("--------------------\n");

    // In local mode, if no persisted state is found, automatically trigger login.
    if env::var("CONTAINER_MODE").map_or(false, |v| v == "true") == false && initial_auth_data.is_none() {
        println!("ℹ️ No previous login state found. Automatically opening login page...");
        if let Err(e) = webbrowser::open("http://localhost:8888/login") {
            println!("⚠️ Failed to open browser automatically: {}. Please open it manually.", e);
        }
    }

    axum::serve(listener, app).await.unwrap();
}

async fn login_handler(State(_state): State<AppState>) -> impl IntoResponse {
    let callback_url = format!("http://localhost:8888/handle-auth.html");
    let auth_url = format!(
        "{}/auth?callback_url={}",
        APP_BASE_URL,
        urlencoding::encode(&callback_url)
    );

    // Check for container mode via environment variable
    let is_container_mode = env::var("CONTAINER_MODE").map_or(false, |v| v == "true");

    if is_container_mode {
        // In container mode, return the URL for the user to open manually.
        (
            StatusCode::OK,
            Html(format!(
                r#"
                <html>
                <head><title>Login Required</title></head>
                <body>
                    <h2>Login Required</h2>
                    <p>Please open the following URL in your browser to log in:</p>
                    <p><a href="{0}" target="_blank">{0}</a></p>
                </body>
                </html>
                "#,
                auth_url
            )),
        )
            .into_response()
    } else {
        // In local mode, try to open the browser automatically.
        if let Err(e) = webbrowser::open(&auth_url) {
            println!("⚠️ Failed to open browser: {}", e);
            println!("Please manually navigate to: {}", auth_url);
        }

        Html(format!(
            r#"
            <html>
            <body>
            <h2>Login Initiated</h2>
            <p>Please check your browser to complete the login process.</p>
            <p>If the browser didn't open automatically, click here:</p>
            <a href="{}" target="_blank">Open Login Page</a>
            </body>
            </html>
            "#,
            auth_url
        ))
        .into_response()
    }
}

async fn callback_handler(
    State(state): State<AppState>,
    axum::extract::Form(params): axum::extract::Form<HashMap<String, String>>,
) -> impl IntoResponse {
    if let Some(refresh_token) = params.get("refreshToken") {
        println!("Received refresh token from client-side Firebase auth flow.");

        let auth_data = AuthData {
            refresh_token: refresh_token.clone(),
            id_token: "".to_string(),
            expires_at: 0,
        };

        if let Ok(file_content) = serde_json::to_string_pretty(&auth_data) {
            if fs::write(&state.auth_state_path, file_content).is_ok() {
                println!("✅ Persisted login state to {}", state.auth_state_path.display());
            } else {
                println!("⚠️ Failed to persist login state to file.");
            }
        }

        let mut auth_data_lock = state.auth_data.lock().await;
        *auth_data_lock = Some(auth_data);

        (StatusCode::OK, Json(json!({ "message": "Refresh token stored successfully." })))
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "refreshToken not found in the request body." })),
        )
    }
}

async fn token_handler(State(state): State<AppState>) -> impl IntoResponse {
    let mut auth_data_lock = state.auth_data.lock().await;

    if let Some(auth_data) = &mut *auth_data_lock {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if auth_data.id_token.is_empty() || auth_data.expires_at <= now + 300 {
            println!("ID token is missing or expiring. Refreshing...");

            match refresh_token(&state.client, &state.firebase_config.api_key, &auth_data.refresh_token).await {
                Ok(new_token_data) => {
                    auth_data.id_token = new_token_data.id_token.clone();
                    auth_data.expires_at = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        + new_token_data.expires_in.parse::<u64>().unwrap_or(3600);
                    println!("Firebase ID token refreshed successfully.");

                    if let Ok(file_content) = serde_json::to_string_pretty(&auth_data) {
                        if fs::write(&state.auth_state_path, file_content).is_ok() {
                            println!("✅ Persisted refreshed login state to {}", state.auth_state_path.display());
                        } else {
                            println!("⚠️ Failed to persist refreshed login state to file.");
                        }
                    }
                }
                Err(e) => {
                    println!("Error refreshing Firebase ID token: {}", e);
                    *auth_data_lock = None;
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "Failed to refresh token. Please re-authenticate via /login." })),
                    ).into_response();
                }
            }
        }

        (
            StatusCode::OK,
            Json(TokenResponse {
                access_token: auth_data.id_token.clone(),
            }),
        ).into_response()
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "Not authenticated. Please complete the /login flow first." })),
        ).into_response()
    }
}

async fn raw_token_handler(State(state): State<AppState>) -> impl IntoResponse {
    let mut auth_data_lock = state.auth_data.lock().await;

    if let Some(auth_data) = &mut *auth_data_lock {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if auth_data.id_token.is_empty() || auth_data.expires_at <= now + 300 {
            println!("ID token is missing or expiring. Refreshing...");

            match refresh_token(&state.client, &state.firebase_config.api_key, &auth_data.refresh_token).await {
                Ok(new_token_data) => {
                    auth_data.id_token = new_token_data.id_token.clone();
                    auth_data.expires_at = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        + new_token_data.expires_in.parse::<u64>().unwrap_or(3600);
                    println!("Firebase ID token refreshed successfully.");

                    if let Ok(file_content) = serde_json::to_string_pretty(&auth_data) {
                        if fs::write(&state.auth_state_path, file_content).is_ok() {
                            println!("✅ Persisted refreshed login state to {}", state.auth_state_path.display());
                        } else {
                            println!("⚠️ Failed to persist refreshed login state to file.");
                        }
                    }
                }
                Err(e) => {
                    println!("Error refreshing Firebase ID token: {}", e);
                    *auth_data_lock = None;
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to refresh token. Please re-authenticate via /login.\n".to_string(),
                    );
                }
            }
        }

        (StatusCode::OK, auth_data.id_token.clone())
    } else {
        (
            StatusCode::UNAUTHORIZED,
            "Not authenticated. Please complete the /login flow first.\n".to_string(),
        )
    }
}

async fn refresh_token(
    client: &Client,
    api_key: &str,
    refresh_token: &str,
) -> Result<TokenRefreshResponse, Box<dyn std::error::Error + Send + Sync>> {
    let refresh_url = format!("https://securetoken.googleapis.com/v1/token?key={}", api_key);

    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];

    let response = client
        .post(&refresh_url)
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("Failed to refresh token: {}", error_text).into());
    }

    let token_data: TokenRefreshResponse = response.json().await?;
    Ok(token_data)
}

#[derive(Deserialize)]
struct TokenRefreshResponse {
    id_token: String,
    expires_in: String,
}
