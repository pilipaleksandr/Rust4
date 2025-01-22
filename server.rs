use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string_pretty};
use std::{collections::HashMap, fs, sync::{Arc, Mutex}};
use tokio::sync::broadcast;
use warp::ws::{Message, WebSocket};
use warp::Filter;

const USERS_FILE: &str = "users.json";
const MESSAGES_FILE: &str = "messages.json";

#[derive(Serialize, Deserialize, Clone)]
struct User {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct ChatMessage {
    username: String,
    text: String,
    timestamp: String,
}

type Users = Arc<Mutex<HashMap<String, User>>>;
type Messages = Arc<Mutex<Vec<ChatMessage>>>;

#[tokio::main]
async fn main() {
    let users = Arc::new(Mutex::new(load_users()));
    let messages = Arc::new(Mutex::new(load_messages()));
    let (tx, _rx) = broadcast::channel::<ChatMessage>(100);

    // WebSocket endpoint
    let chat_route = warp::path("ws")
        .and(warp::ws())
        .and(with_users(users.clone()))
        .and(with_messages(messages.clone()))
        .and(with_broadcast(tx.clone()))
        .map(|ws: warp::ws::Ws, users, messages, tx| {
            ws.on_upgrade(move |socket| handle_connection(socket, users, messages, tx))
        });

    println!("WebSocket-сервер запущен на порту 3000");
    warp::serve(chat_route).run(([0, 0, 0, 0], 3000)).await;
}

// Завантаження користувачів
fn load_users() -> HashMap<String, User> {
    if let Ok(data) = fs::read_to_string(USERS_FILE) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        HashMap::new()
    }
}

// Завантаження повідомлень
fn load_messages() -> Vec<ChatMessage> {
    if let Ok(data) = fs::read_to_string(MESSAGES_FILE) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    }
}

// Збереження повідомлень
fn save_messages(messages: &Vec<ChatMessage>) {
    if let Ok(data) = to_string_pretty(&messages) {
        fs::write(MESSAGES_FILE, data).expect("Ошибка записи сообщений");
    }
}

// Обробка з'єднань WebSocket
async fn handle_connection(
    ws: WebSocket,
    users: Users,
    messages: Messages,
    tx: broadcast::Sender<ChatMessage>,
) {
    let (user_ws_tx, mut user_ws_rx) = ws.split();
    let mut broadcast_rx = tx.subscribe();

    tokio::spawn(async move {
        while let Ok(message) = broadcast_rx.recv().await {
            if let Ok(msg) = serde_json::to_string(&message) {
                let _ = user_ws_tx.send(Message::text(msg)).await;
            }
        }
    });

    while let Some(Ok(message)) = user_ws_rx.next().await {
        if let Ok(text) = message.to_str() {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(text) {
                match data["type"].as_str() {
                    Some("auth") => {
                        let username = data["username"].as_str().unwrap_or("").to_string();
                        let password = data["password"].as_str().unwrap_or("").to_string();

                        let user = users.lock().unwrap().get(&username);
                        if let Some(user) = user {
                            if user.password == password {
                                let auth_message = json!({
                                    "type": "auth",
                                    "status": "success",
                                    "username": username
                                });
                                let _ = user_ws_tx.send(Message::text(auth_message.to_string())).await;
                            } else {
                                let error_message = json!({
                                    "type": "auth",
                                    "status": "error",
                                    "message": "Неверные логин или пароль"
                                });
                                let _ = user_ws_tx.send(Message::text(error_message.to_string())).await;
                            }
                        }
                    }
                    Some("message") => {
                        let new_message = ChatMessage {
                            username: data["username"].as_str().unwrap_or("").to_string(),
                            text: data["text"].as_str().unwrap_or("").to_string(),
                            timestamp: chrono::Utc::now().to_string(),
                        };
                        messages.lock().unwrap().push(new_message.clone());
                        save_messages(&messages.lock().unwrap());
                        let _ = tx.send(new_message);
                    }
                    _ => {}
                }
            }
        }
    }
}

// Фільтри для передачі даних у хендлери
fn with_users(users: Users) -> impl Filter<Extract = (Users,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || users.clone())
}

fn with_messages(messages: Messages) -> impl Filter<Extract = (Messages,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || messages.clone())
}

fn with_broadcast(tx: broadcast::Sender<ChatMessage>) -> impl Filter<Extract = (broadcast::Sender<ChatMessage>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || tx.clone())
}
