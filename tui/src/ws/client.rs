use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

use crate::ws::protocol::{ClientHello, IncomingMessage, OutgoingMessage, ServerHello, SessionSummary};

#[derive(Debug, Clone)]
pub enum WsEvent {
    Connected {
        #[allow(dead_code)]
        client_id: String,
        daemon_version: String,
        sessions: Vec<SessionSummary>,
    },
    Message(IncomingMessage),
    Disconnected,
    Error(String),
}

pub struct WsClient {
    tx: mpsc::Sender<OutgoingMessage>,
    client_id: String,
}

impl WsClient {
    pub async fn connect(uri: &str, event_tx: mpsc::Sender<WsEvent>) -> Result<Self> {
        let client_id = Uuid::new_v4().to_string();
        let (ws_stream, _) = connect_async(uri).await?;
        let (mut write, mut read) = ws_stream.split();

        // Send ClientHello immediately after connection
        let hello = ClientHello::new(client_id.clone());
        let hello_json = serde_json::to_string(&hello)?;
        tracing::debug!("WS sending hello: {}", &hello_json);
        write.send(Message::Text(hello_json)).await?;

        // Wait for ServerHello before proceeding
        let server_hello: ServerHello = loop {
            match read.next().await {
                Some(Ok(Message::Text(text))) => {
                    tracing::debug!("WS received: {}", &text[..text.len().min(200)]);
                    // Try to parse as ServerHello
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                        if value.get("type").and_then(|v| v.as_str()) == Some("hello_ack") {
                            match serde_json::from_str::<ServerHello>(&text) {
                                Ok(hello) => break hello,
                                Err(e) => {
                                    tracing::warn!("Failed to parse ServerHello: {}", e);
                                }
                            }
                        }
                    }
                }
                Some(Ok(Message::Close(_))) => {
                    return Err(anyhow::anyhow!("Connection closed before receiving ServerHello"));
                }
                Some(Err(e)) => {
                    return Err(anyhow::anyhow!("WebSocket error before receiving ServerHello: {}", e));
                }
                None => {
                    return Err(anyhow::anyhow!("Connection ended before receiving ServerHello"));
                }
                _ => {}
            }
        };

        tracing::info!("Connected to daemon v{}", server_hello.daemon_version);

        let (tx, mut rx) = mpsc::channel::<OutgoingMessage>(32);

        // Spawn task to handle outgoing messages
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let json = match serde_json::to_string(&msg) {
                    Ok(j) => j,
                    Err(e) => {
                        tracing::error!("Failed to serialize outgoing message: {}", e);
                        continue;
                    }
                };
                tracing::debug!("WS sending: {}", &json);
                if write.send(Message::Text(json)).await.is_err() {
                    tracing::error!("Failed to send WS message");
                    break;
                }
            }
        });

        // Spawn task to handle incoming messages
        let event_tx_clone = event_tx.clone();
        tokio::spawn(async move {
            while let Some(result) = read.next().await {
                match result {
                    Ok(Message::Text(text)) => {
                        tracing::debug!("WS received: {}", &text[..text.len().min(200)]);

                        // Parse as IncomingMessage
                        match serde_json::from_str::<IncomingMessage>(&text) {
                            Ok(msg) => {
                                tracing::debug!("Parsed message: {:?}", msg);
                                let _ = event_tx_clone.send(WsEvent::Message(msg)).await;
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse WS message: {} - {}", e, &text[..text.len().min(200)]);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        let _ = event_tx_clone.send(WsEvent::Disconnected).await;
                        break;
                    }
                    Ok(Message::Ping(_)) => {}
                    Ok(_) => {}
                    Err(e) => {
                        let _ = event_tx_clone
                            .send(WsEvent::Error(e.to_string()))
                            .await;
                        let _ = event_tx_clone.send(WsEvent::Disconnected).await;
                        break;
                    }
                }
            }
        });

        // Send connected event with server hello data
        let _ = event_tx
            .send(WsEvent::Connected {
                client_id: client_id.clone(),
                daemon_version: server_hello.daemon_version,
                sessions: server_hello.sessions,
            })
            .await;

        Ok(Self { tx, client_id })
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub async fn send(&self, msg: OutgoingMessage) -> Result<()> {
        self.tx
            .send(msg)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))
    }
}
