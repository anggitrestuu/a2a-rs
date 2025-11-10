use a2a_client::{
    components::{create_sse_stream, MessageView, TaskView},
    WebA2AClient,
};
use a2a_rs::{
    domain::{ListTasksParams, TaskState, TaskStatusUpdateEvent},
    services::AsyncA2AClient,
};
use anyhow;
use askama::Template;
use askama_axum::IntoResponse;
use axum::{
    extract::{Multipart, Path, Query, State},
    response::Response as AxumResponse,
    routing::{get, post},
    Form, Router,
};
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tracing::{error, info, warn};
use uuid::Uuid;

struct AppState {
    client: Arc<WebA2AClient>,
    webhook_token: String,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    agent_url: String,
}

#[derive(Template)]
#[template(path = "chat.html")]
struct ChatTemplate {
    task_id: String,
    messages: Vec<MessageView>,
    task_state: Option<String>,
}

#[derive(Template)]
#[template(path = "tasks.html")]
struct TasksTemplate {
    tasks: Vec<TaskView>,
    filter_state: Option<String>,
    total_count: usize,
}

#[derive(Template)]
#[template(path = "expense-form.html")]
struct ExpenseFormTemplate {
    category: Option<String>,
}

// TaskView and MessageView are now imported from a2a_client

// Form structs for endpoints

#[derive(Deserialize)]
struct NewChatForm {
    #[allow(dead_code)]
    agent_url: String,
}

#[derive(Deserialize)]
struct TasksQuery {
    state: Option<String>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct ExpenseQuery {
    #[serde(rename = "type")]
    expense_type: Option<String>,
}

#[derive(Deserialize)]
struct ExpenseSubmitForm {
    category: String,
    amount: String,
    date: String,
    vendor: Option<String>,
    description: String,
    project_code: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Separate URLs for HTTP and WebSocket
    let http_url = std::env::var("AGENT_HTTP_URL")
        .or_else(|_| std::env::var("AGENT_URL"))
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    let ws_url =
        std::env::var("AGENT_WS_URL").unwrap_or_else(|_| "ws://localhost:8081".to_string());

    // Check if we should use WebSocket or HTTP client
    let use_websocket = std::env::var("USE_WEBSOCKET")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    let client = if use_websocket {
        info!("Using WebSocket client for subscriptions at {}", ws_url);
        info!("Using HTTP client for API calls at {}", http_url);
        WebA2AClient::new_with_websocket(http_url, ws_url)
    } else {
        info!("Using HTTP client at {}", http_url);
        WebA2AClient::new_http(http_url)
    };

    // Generate or load webhook authentication token
    let webhook_token = std::env::var("WEBHOOK_TOKEN").unwrap_or_else(|_| {
        let token = format!("wh_{}", Uuid::new_v4().simple());
        info!("Generated webhook token: {}", token);
        token
    });

    let state = AppState {
        client: Arc::new(client),
        webhook_token,
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/tasks", get(tasks_page))
        .route("/expense/new", get(expense_form))
        .route("/expense/submit", post(submit_expense))
        .route("/chat/new", post(new_chat))
        .route("/chat/:task_id", get(chat_page))
        .route("/chat/:task_id/send", post(send_message))
        .route("/chat/:task_id/cancel", post(cancel_task))
        .route("/chat/:task_id/stream", get(stream_task))
        .route("/webhook/push-notification", post(handle_push_notification))
        .nest_service("/static", ServeDir::new("static"))
        .layer(CorsLayer::permissive())
        .with_state(Arc::new(state));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

async fn index() -> impl IntoResponse {
    let agent_url = std::env::var("AGENT_HTTP_URL")
        .or_else(|_| std::env::var("AGENT_URL"))
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    IndexTemplate { agent_url }
}

async fn expense_form(Query(query): Query<ExpenseQuery>) -> impl IntoResponse {
    ExpenseFormTemplate {
        category: query.expense_type,
    }
}

async fn submit_expense(
    State(state): State<Arc<AppState>>,
    Form(form): Form<ExpenseSubmitForm>,
) -> Result<AxumResponse, AppError> {
    use a2a_rs::domain::{Message, Part, Role};

    // Create a new task ID for this expense
    let task_id = Uuid::new_v4().to_string();

    // Format the expense details as a structured message
    let expense_details = format!(
        "I need to submit an expense reimbursement:\n\n\
        Category: {}\n\
        Amount: ${}\n\
        Date: {}\n\
        {}\
        Description: {}\n\
        {}",
        form.category,
        form.amount,
        form.date,
        form.vendor
            .as_ref()
            .map(|v| format!("Vendor: {}\n", v))
            .unwrap_or_default(),
        form.description,
        form.project_code
            .as_ref()
            .map(|p| format!("Project/Cost Center: {}\n", p))
            .unwrap_or_default()
    );

    let message = Message {
        role: Role::User,
        parts: vec![Part::text(expense_details)],
        metadata: None,
        reference_task_ids: None,
        message_id: Uuid::new_v4().to_string(),
        task_id: Some(task_id.clone()),
        context_id: None,
        extensions: None,
        kind: "message".to_string(),
    };

    // Send the initial expense request to create the task
    let response = state
        .client
        .http
        .send_task_message(&task_id, &message, None, Some(50))
        .await
        .map_err(|e| AppError(anyhow::anyhow!("Failed to submit expense: {}", e)))?;

    info!(
        "Expense submitted for task {}, response state: {:?}",
        task_id, response.status.state
    );

    // Register push notification for this task
    use a2a_rs::domain::{PushNotificationConfig, TaskPushNotificationConfig};

    let push_config = TaskPushNotificationConfig {
        task_id: task_id.clone(),
        push_notification_config: PushNotificationConfig {
            id: None,
            url: "http://localhost:3000/webhook/push-notification".to_string(),
            token: Some(state.webhook_token.clone()),
            authentication: None,
        },
    };

    match state
        .client
        .http
        .set_task_push_notification(&push_config)
        .await
    {
        Ok(_) => info!(
            "Push notification registered for expense task {} with authentication",
            task_id
        ),
        Err(e) => warn!("Failed to register push notification: {}", e),
    }

    // Wait to ensure task is persisted before redirect
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Redirect to the chat page to see the agent's response
    Ok(axum::response::Redirect::to(&format!("/chat/{}", task_id)).into_response())
}

async fn new_chat(
    State(_state): State<Arc<AppState>>,
    Form(_form): Form<NewChatForm>,
) -> Result<AxumResponse, AppError> {
    // Create a new task
    let task_id = Uuid::new_v4().to_string();

    // Redirect to the chat page
    Ok(axum::response::Redirect::to(&format!("/chat/{}", task_id)).into_response())
}

async fn tasks_page(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TasksQuery>,
) -> Result<impl IntoResponse, AppError> {
    let mut params = ListTasksParams::default();

    // Apply state filter if provided
    if let Some(state_str) = &query.state {
        if let Ok(state) = serde_json::from_str::<TaskState>(&format!("\"{}\"", state_str)) {
            params.status = Some(state);
        }
    }

    // Apply limit
    params.page_size = query.limit.map(|l| l as i32).or(Some(50));

    // Use HTTP client for all API operations
    let result = state
        .client
        .http
        .list_tasks(&params)
        .await
        .map_err(|e| AppError(anyhow::anyhow!("Failed to list tasks: {}", e)))?;

    let tasks: Vec<TaskView> = result.tasks.into_iter().map(TaskView::from_task).collect();

    let template = TasksTemplate {
        tasks,
        filter_state: query.state,
        total_count: result.total_size as usize,
    };

    Ok(template)
}

async fn chat_page(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    // Try to get existing task messages (use HTTP client)
    // Retry a few times in case of race condition with task creation
    let mut retry_count = 0;
    let max_retries = 3;

    let (messages, task_state) = loop {
        match state.client.http.get_task(&task_id, Some(50)).await {
            Ok(task) => {
                info!(
                    "Retrieved task {} with {} history items",
                    task_id,
                    task.history.as_ref().map(|h| h.len()).unwrap_or(0)
                );

                let state = Some(format!("{:?}", task.status.state));
                let messages = task
                    .history
                    .unwrap_or_default()
                    .into_iter()
                    .map(MessageView::from_message_with_json_parsing)
                    .collect();
                break (messages, state);
            }
            Err(e) => {
                retry_count += 1;
                if retry_count >= max_retries {
                    warn!(
                        "Failed to get task {} after {} retries: {}",
                        task_id, max_retries, e
                    );
                    // New task or not found, no messages yet
                    break (vec![], None);
                }
                // Wait a bit and retry
                info!(
                    "Task {} not found, retrying ({}/{})",
                    task_id, retry_count, max_retries
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
        }
    };

    let template = ChatTemplate {
        task_id,
        messages,
        task_state,
    };
    Ok(template)
}

async fn send_message(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<AxumResponse, AppError> {
    use a2a_rs::domain::{FileContent, Message, Part, Role};

    let mut task_id = String::new();
    let mut message_text = String::new();
    let mut parts = Vec::new();

    // Process multipart form data
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError(anyhow::anyhow!("Failed to read multipart field: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "task_id" => {
                task_id = field
                    .text()
                    .await
                    .map_err(|e| AppError(anyhow::anyhow!("Failed to read task_id: {}", e)))?;
            }
            "message" => {
                message_text = field
                    .text()
                    .await
                    .map_err(|e| AppError(anyhow::anyhow!("Failed to read message: {}", e)))?;
            }
            "receipt" => {
                // Extract file data
                let file_name = field.file_name().map(|s| s.to_string());
                let content_type = field.content_type().map(|s| s.to_string());
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError(anyhow::anyhow!("Failed to read file: {}", e)))?;

                if !data.is_empty() {
                    info!(
                        "Received file upload: name={:?}, type={:?}, size={} bytes",
                        file_name,
                        content_type,
                        data.len()
                    );

                    // Encode bytes as base64
                    use base64::Engine;
                    let base64_data = base64::engine::general_purpose::STANDARD.encode(&data);

                    // Create a file part
                    let file_part = Part::File {
                        file: FileContent {
                            bytes: Some(base64_data),
                            uri: None,
                            name: file_name,
                            mime_type: content_type,
                        },
                        metadata: None,
                    };
                    parts.push(file_part);
                }
            }
            _ => {
                warn!("Unknown form field: {}", name);
            }
        }
    }

    // Add text message as a part
    if !message_text.is_empty() {
        parts.insert(0, Part::text(message_text));
    }

    if task_id.is_empty() {
        return Err(AppError(anyhow::anyhow!("Missing task_id")));
    }

    if parts.is_empty() {
        return Err(AppError(anyhow::anyhow!("Message cannot be empty")));
    }

    let message = Message {
        role: Role::User,
        parts,
        metadata: None,
        reference_task_ids: None,
        message_id: Uuid::new_v4().to_string(),
        task_id: Some(task_id.clone()),
        context_id: None,
        extensions: None,
        kind: "message".to_string(),
    };

    // Send message using HTTP client
    let response = state
        .client
        .http
        .send_task_message(&task_id, &message, None, Some(50))
        .await
        .map_err(|e| AppError(anyhow::anyhow!("Failed to send message: {}", e)))?;

    info!(
        "Message sent successfully for task {}, response has {} history items",
        task_id,
        response.history.as_ref().map(|h| h.len()).unwrap_or(0)
    );

    // Register push notification for this task to get notified when agent responds
    use a2a_rs::domain::{PushNotificationConfig, TaskPushNotificationConfig};

    let push_config = TaskPushNotificationConfig {
        task_id: task_id.clone(),
        push_notification_config: PushNotificationConfig {
            id: None,
            url: "http://localhost:3000/webhook/push-notification".to_string(),
            token: Some(state.webhook_token.clone()),
            authentication: None,
        },
    };

    // Try to register push notification (don't fail if it doesn't work)
    match state
        .client
        .http
        .set_task_push_notification(&push_config)
        .await
    {
        Ok(_) => info!(
            "Push notification registered for task {} with authentication",
            task_id
        ),
        Err(e) => warn!(
            "Failed to register push notification for task {}: {}",
            task_id, e
        ),
    }

    // Wait a bit longer to ensure task is persisted
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Redirect back to the chat page
    Ok(axum::response::Redirect::to(&format!("/chat/{}", task_id)).into_response())
}

async fn cancel_task(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> Result<AxumResponse, AppError> {
    state
        .client
        .http
        .cancel_task(&task_id)
        .await
        .map_err(|e| AppError(anyhow::anyhow!("Failed to cancel task: {}", e)))?;

    // Redirect back to the chat page
    Ok(axum::response::Redirect::to(&format!("/chat/{}", task_id)).into_response())
}

async fn stream_task(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> axum::response::sse::Sse<
    impl futures::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>,
> {
    // Use the generic streaming component from a2a-client
    create_sse_stream(state.client.clone(), task_id)
}

async fn handle_push_notification(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::Json(event): axum::Json<TaskStatusUpdateEvent>,
) -> Result<AxumResponse, AppError> {
    // Verify authentication token
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let authenticated = match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header[7..]; // Skip "Bearer "
            token == state.webhook_token
        }
        _ => false,
    };

    if !authenticated {
        warn!(
            "Unauthorized push notification attempt for task {}",
            event.task_id
        );
        return Err(AppError(anyhow::anyhow!("Unauthorized")));
    }

    info!(
        "✅ Authenticated push notification for task {}: state={:?}",
        event.task_id, event.status.state
    );

    // Log the event - in a real app, you might:
    // - Store it in a database
    // - Trigger browser notifications
    // - Update a cache
    // - Forward to connected WebSocket clients

    Ok(axum::response::Json(serde_json::json!({
        "status": "received",
        "task_id": event.task_id,
        "authenticated": true
    }))
    .into_response())
}

#[derive(Debug)]
struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> AxumResponse {
        error!("Application error: {}", self.0);
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Internal server error: {}", self.0),
        )
            .into_response()
    }
}
