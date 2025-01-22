#[macro_use]
extern crate serde_json;

use warp::ws::{Message, WebSocket};
use warp::Filter;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::fs;
use chrono::Utc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use futures::{StreamExt, SinkExt};

const USERS_FILE: &str = "users.json";
const MESSAGES_FILE: &str = "messages.json";

#[derive(Serialize, Deserialize, Clone)]
struct User {
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
type Connections = Arc<Mutex<Vec<mpsc::UnboundedSender<Message>>>>;

#[tokio::main]
async fn main() {
    let users = Arc::new(Mutex::new(load_users()));
    let messages = Arc::new(Mutex::new(load_messages()));
    let connections = Arc::new(Mutex::new(Vec::new()));

    let chat_route = warp::path("ws")
        .and(warp::ws())
        .and(with_users(users.clone()))
        .and(with_messages(messages.clone()))
        .and(with_connections(connections.clone()))
        .map(|ws: warp::ws::Ws, users, messages, connections| {
            ws.on_upgrade(move |socket| handle_connection(socket, users, messages, connections))
        });

    println!("WebSocket-сервер запущен на порту 3000");
    warp::serve(chat_route).run(([0, 0, 0, 0], 3000)).await;
}

fn load_users() -> HashMap<String, User> {
    match fs::read_to_string(USERS_FILE) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

fn load_messages() -> Vec<ChatMessage> {
    match fs::read_to_string(MESSAGES_FILE) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save_messages(messages: &Vec<ChatMessage>) {
    if let Ok(data) = serde_json::to_string_pretty(messages) {
        fs::write(MESSAGES_FILE, data).expect("Ошибка записи сообщений");
    }
}

async fn handle_connection(
    ws: WebSocket,
    users: Users,
    messages: Messages,
    connections: Connections,
) {
    let (mut user_ws_tx, mut user_ws_rx) = ws.split(); 
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    {
        connections.lock().unwrap().push(tx.clone());
    }

    let mut rx_stream = UnboundedReceiverStream::new(rx);

    
    tokio::spawn(async move {
        while let Some(message) = rx_stream.next().await {
            if let Err(e) = user_ws_tx.send(message).await {
                eprintln!("Ошибка отправки сообщения: {}", e);
                break;
            }
        }
    });

    
    while let Some(Ok(message)) = user_ws_rx.next().await {
        if let Ok(text) = message.to_str() {
            println!("Получено сообщение: {}", text);
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(text) {
                match data["type"].as_str() {
                    Some("auth") => {
                        let username = data["username"].as_str().unwrap_or("").to_string();
                        let password = data["password"].as_str().unwrap_or("").to_string();

                        let user = {
                            let users_guard = users.lock().unwrap();
                            users_guard.get(&username).cloned()
                        };

                        if let Some(user) = user {
                            if user.password == password {
                                let auth_message = json!({
                                    "type": "auth",
                                    "status": "success",
                                    "username": username
                                });
                                let _ = tx.send(Message::text(auth_message.to_string()));
                            } else {
                                let error_message = json!({
                                    "type": "auth",
                                    "status": "error",
                                    "message": "Неверный пароль"
                                });
                                let _ = tx.send(Message::text(error_message.to_string()));
                            }
                        } else {
                            let error_message = json!({
                                "type": "auth",
                                "status": "error",
                                "message": "Пользователь не найден"
                            });
                            let _ = tx.send(Message::text(error_message.to_string()));
                        }
                    }
                    Some("message") => {
                        let new_message = ChatMessage {
                            username: data["username"].as_str().unwrap_or("").to_string(),
                            text: data["text"].as_str().unwrap_or("").to_string(),
                            timestamp: Utc::now().to_rfc3339(),
                        };

                        {
                            let mut msgs = messages.lock().unwrap();
                            msgs.push(new_message.clone());
                            save_messages(&msgs);
                        }

                        let broadcast_message = json!({
                            "type": "message",
                            "message": new_message
                        });

                        let connections_guard = connections.lock().unwrap();
                        for conn in connections_guard.iter() {
                            let _ = conn.send(Message::text(broadcast_message.to_string()));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    println!("Клиент отключился");
    connections.lock().unwrap().retain(|conn| !conn.is_closed());
}


fn with_users(users: Users) -> impl Filter<Extract = (Users,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || users.clone())
}

fn with_messages(messages: Messages) -> impl Filter<Extract = (Messages,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || messages.clone())
}

fn with_connections(
    connections: Connections,
) -> impl Filter<Extract = (Connections,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || connections.clone())
}
