/// gateway — Web 控制台（axum + WebSocket + Three.js 3D 脑模型）
use axum::{
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router, Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

// 编译时嵌入 web 静态资源
const INDEX_HTML: &str = include_str!("../web/index.html");
const STYLE_CSS: &str = include_str!("../web/style.css");
const APP_JS: &str = include_str!("../web/app.js");

struct AppState {
    home: String,
    tx: broadcast::Sender<String>,
}

#[derive(Deserialize)]
struct EngramQuery {
    layer: Option<String>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct RecallRequest {
    query: String,
    top_k: Option<usize>,
}

#[derive(Deserialize)]
struct GateRequest {
    message: String,
}

/// 统一处理 spawn_blocking 结果
fn unwrap_res(res: Result<Result<Value, String>, tokio::task::JoinError>) -> Json<Value> {
    match res {
        Ok(Ok(v)) => Json(v),
        Ok(Err(e)) => Json(json!({"error": e})),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

pub async fn run_gateway(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let home = std::env::var("HIPPOCAMPUS_HOME").unwrap_or_else(|_| {
        std::env::var("HOME")
            .map(|h| format!("{}/.hippocampus", h))
            .unwrap_or_else(|_| "./.hippocampus".to_string())
    });

    let (tx, _) = broadcast::channel(100);
    let state = Arc::new(AppState { home, tx });

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/index.html", get(serve_index))
        .route("/style.css", get(serve_css))
        .route("/app.js", get(serve_js))
        .route("/api/stats", get(api_stats))
        .route("/api/brain/status", get(api_brain_status))
        .route("/api/engrams", get(api_engrams))
        .route("/api/recall", post(api_recall))
        .route("/api/gate", post(api_gate))
        .route("/api/gate/execute", post(api_gate_execute))
        .route("/api/notify", post(api_notify))
        .route("/api/events", get(api_events))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    eprintln!("🧠 Hippocampus Gateway running on http://localhost:{}", port);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// --- 静态文件 ---

async fn serve_index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn serve_css() -> Response {
    (
        [(axum::http::header::CONTENT_TYPE, "text/css")],
        STYLE_CSS,
    )
        .into_response()
}

async fn serve_js() -> Response {
    (
        [(axum::http::header::CONTENT_TYPE, "application/javascript")],
        APP_JS,
    )
        .into_response()
}

// --- API ---

async fn api_stats(State(state): State<Arc<AppState>>) -> Json<Value> {
    let home = state.home.clone();
    let res = tokio::task::spawn_blocking(move || -> Result<Value, String> {
        let hippo = crate::Hippocampus::new(&home).map_err(|e| e.to_string())?;
        let stats = hippo.stats();
        Ok(json!({
            "status": "ok",
            "total_engrams": stats.total,
            "by_layer": stats.by_layer,
            "avg_access_count": stats.avg_access_count,
            "avg_importance": stats.avg_importance,
        }))
    })
    .await;
    unwrap_res(res)
}

async fn api_brain_status(State(state): State<Arc<AppState>>) -> Json<Value> {
    let home = state.home.clone();
    let res = tokio::task::spawn_blocking(move || -> Value {
        let path = std::path::Path::new(&home).join("last_gate.json");
        if let Ok(data) = std::fs::read_to_string(&path) {
            serde_json::from_str(&data).unwrap_or_else(|_| json!({"status": "no_data"}))
        } else {
            json!({
                "status": "no_data",
                "components": {
                    "amygdala": {"score": 0.0, "reason": "等待评估"},
                    "hippocampus": {"score": 0.0, "reason": "等待评估"},
                    "prefrontal": {"score": 0.0, "reason": "等待评估"},
                    "temporal": {"score": 0.0, "reason": "等待评估"},
                }
            })
        }
    })
    .await;
    match res {
        Ok(v) => Json(v),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

async fn api_engrams(
    axum::extract::Query(params): axum::extract::Query<EngramQuery>,
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let home = state.home.clone();
    let layer = params.layer.unwrap_or_else(|| "L1".to_string());
    let limit = params.limit.unwrap_or(20);

    let res = tokio::task::spawn_blocking(move || -> Result<Value, String> {
        let hippo = crate::Hippocampus::new(&home).map_err(|e| e.to_string())?;
        let engrams = hippo.store.read_layer(&layer).map_err(|e| e.to_string())?;
        let list: Vec<Value> = engrams
            .iter()
            .rev()
            .take(limit)
            .map(|e| {
                json!({
                    "id": e.id,
                    "content": e.content.chars().take(200).collect::<String>(),
                    "importance": e.importance,
                    "emotion": e.emotion,
                    "layer": e.layer,
                    "tags": e.tags,
                    "created_at": e.created_at,
                    "access_count": e.access_count,
                })
            })
            .collect();
        Ok(json!({
            "status": "ok",
            "layer": layer,
            "count": list.len(),
            "engrams": list,
        }))
    })
    .await;
    unwrap_res(res)
}

async fn api_recall(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RecallRequest>,
) -> Json<Value> {
    let home = state.home.clone();
    let query = body.query;
    let top_k = body.top_k.unwrap_or(5);

    let res = tokio::task::spawn_blocking(move || -> Result<Value, String> {
        let hippo = crate::Hippocampus::new(&home).map_err(|e| e.to_string())?;
        let results = hippo.recall(&query, top_k, 0.01, true, None, None);
        Ok(json!({
            "status": "ok",
            "results": results,
        }))
    })
    .await;
    unwrap_res(res)
}

async fn api_gate(
    State(state): State<Arc<AppState>>,
    Json(body): Json<GateRequest>,
) -> Json<Value> {
    let home = state.home.clone();
    let tx = state.tx.clone();
    let message = body.message;

    let res = tokio::task::spawn_blocking(move || -> Result<Value, String> {
        let hippo = crate::Hippocampus::new(&home).map_err(|e| e.to_string())?;
        let decision = hippo.should_remember(&message);

        let result = json!({
            "status": "ok",
            "should_remember": decision.should_remember,
            "importance": decision.importance,
            "emotion": decision.emotion,
            "emotion_score": decision.emotion_score,
            "decision_score": decision.decision_score,
            "reason": decision.reason,
            "tags": decision.tags,
            "components": {
                "amygdala": {"score": decision.components.amygdala.score, "reason": decision.components.amygdala.reason},
                "hippocampus": {"score": decision.components.hippocampus.score, "reason": decision.components.hippocampus.reason},
                "prefrontal": {"score": decision.components.prefrontal.score, "reason": decision.components.prefrontal.reason},
                "temporal": {"score": decision.components.temporal.score, "reason": decision.components.temporal.reason},
            },
        });

        // 保存 last_gate.json
        let gate_path = std::path::Path::new(&home).join("last_gate.json");
        let _ = std::fs::write(&gate_path, serde_json::to_string_pretty(&result).unwrap_or_default());

        // 广播事件
        let event = json!({
            "type": "gate",
            "timestamp": now_ts(),
            "should_remember": decision.should_remember,
            "decision_score": decision.decision_score,
            "components": {
                "amygdala": decision.components.amygdala.score,
                "hippocampus": decision.components.hippocampus.score,
                "prefrontal": decision.components.prefrontal.score,
                "temporal": decision.components.temporal.score,
            },
        });
        let _ = tx.send(event.to_string());

        Ok(result)
    })
    .await;
    unwrap_res(res)
}

async fn api_gate_execute(
    State(state): State<Arc<AppState>>,
    Json(body): Json<GateRequest>,
) -> Json<Value> {
    let home = state.home.clone();
    let tx = state.tx.clone();
    let message = body.message;

    let res = tokio::task::spawn_blocking(move || -> Result<Value, String> {
        let mut hippo = crate::Hippocampus::new(&home).map_err(|e| e.to_string())?;
        let decision = hippo
            .auto_remember(&message, "gateway", None, false)
            .map_err(|e| e.to_string())?;

        let result = json!({
            "status": "ok",
            "should_remember": decision.should_remember,
            "importance": decision.importance,
            "emotion": decision.emotion,
            "emotion_score": decision.emotion_score,
            "decision_score": decision.decision_score,
            "reason": decision.reason,
            "tags": decision.tags,
            "written": true,
            "components": {
                "amygdala": {"score": decision.components.amygdala.score, "reason": decision.components.amygdala.reason},
                "hippocampus": {"score": decision.components.hippocampus.score, "reason": decision.components.hippocampus.reason},
                "prefrontal": {"score": decision.components.prefrontal.score, "reason": decision.components.prefrontal.reason},
                "temporal": {"score": decision.components.temporal.score, "reason": decision.components.temporal.reason},
            },
        });

        // 保存 last_gate.json
        let gate_path = std::path::Path::new(&home).join("last_gate.json");
        let _ = std::fs::write(&gate_path, serde_json::to_string_pretty(&result).unwrap_or_default());

        // 广播事件
        let msg_preview: String = message.chars().take(100).collect();
        let event = json!({
            "type": "gate_execute",
            "timestamp": now_ts(),
            "should_remember": decision.should_remember,
            "decision_score": decision.decision_score,
            "message_preview": msg_preview,
            "components": {
                "amygdala": decision.components.amygdala.score,
                "hippocampus": decision.components.hippocampus.score,
                "prefrontal": decision.components.prefrontal.score,
                "temporal": decision.components.temporal.score,
            },
        });
        let _ = tx.send(event.to_string());

        Ok(result)
    })
    .await;
    unwrap_res(res)
}

async fn api_notify(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    let tx = state.tx.clone();
    let _ = tx.send(payload.to_string());
    Json(json!({"status": "ok", "message": "Notification broadcasted"}))
}

// --- WebSocket ---

async fn api_events(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.tx.subscribe();

    // 发送初始状态 — 使用 spawn_blocking 避免 Send 问题
    let home = state.home.clone();
    let init_msg = tokio::task::spawn_blocking(move || -> Option<String> {
        let hippo = crate::Hippocampus::new(&home).ok()?;
        let stats = hippo.stats();
        Some(json!({
            "type": "init",
            "total": stats.total,
            "by_layer": stats.by_layer,
        }).to_string())
    }).await;

    if let Ok(Some(msg)) = init_msg {
        let _ = socket.send(Message::Text(msg.into())).await;
    }

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(text) => {
                        if socket.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        let msg = json!({"type": "lagged", "missed": n}).to_string();
                        if socket.send(Message::Text(msg.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                if socket.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }
        }
    }
}

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
