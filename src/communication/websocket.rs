// WebSocket Communication Module
use super::message::{AgentId, InterAgentMessage};
use anyhow::{anyhow, Result};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{accept_async, WebSocketStream};
use tokio_stream::wrappers::UnboundedReceiverStream;

/// WebSocket server
pub struct WebSocketServer {
    addr: SocketAddr,
    clients: Arc<RwLock<HashMap<AgentId, mpsc::UnboundedSender<Message>>>>,
    message_tx: mpsc::UnboundedSender<ServerMessage>,
}

/// Server message for internal routing
#[derive(Debug, Clone)]
pub enum ServerMessage {
    Connected { agent_id: AgentId },
    Disconnected { agent_id: AgentId },
    Message { from: AgentId, msg: InterAgentMessage },
}

impl WebSocketServer {
    /// Create a new WebSocket server
    pub fn new(addr: SocketAddr) -> Self {
        let (message_tx, _) = mpsc::unbounded_channel();
        Self {
            addr,
            clients: Arc::new(RwLock::new(HashMap::new())),
            message_tx,
        }
    }
    
    /// Get the message sender for external use
    pub fn get_message_sender(&self) -> mpsc::UnboundedSender<ServerMessage> {
        self.message_tx.clone()
    }

    /// Start the server
    pub async fn start(&self) -> Result<mpsc::UnboundedReceiver<ServerMessage>> {
        let listener = TcpListener::bind(&self.addr).await?;
        let clients = self.clients.clone();
        let (message_tx, message_rx) = mpsc::unbounded_channel();

        tracing::info!("WebSocket server listening on {}", self.addr);

        tokio::spawn(async move {
            while let Ok((stream, addr)) = listener.accept().await {
                let clients = clients.clone();
                let message_tx = message_tx.clone();

                tokio::spawn(async move {
                    if let Ok(ws_stream) = accept_async(stream).await {
                        tracing::debug!("WebSocket connection from {}", addr);
                        
                        if let Err(e) = Self::handle_connection(ws_stream, clients, message_tx).await {
                            tracing::error!("Error handling WebSocket connection: {}", e);
                        }
                    }
                });
            }
        });

        Ok(message_rx)
    }

    /// Handle a WebSocket connection
    async fn handle_connection(
        ws_stream: WebSocketStream<tokio::net::TcpStream>,
        clients: Arc<RwLock<HashMap<AgentId, mpsc::UnboundedSender<Message>>>>,
        message_tx: mpsc::UnboundedSender<ServerMessage>,
    ) -> Result<()> {
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let (client_tx, client_rx) = mpsc::unbounded_channel();
        
        let mut agent_id: Option<AgentId> = None;
        let mut client_rx = UnboundedReceiverStream::new(client_rx);

        // Wait for handshake
        if let Some(msg_result) = ws_receiver.next().await {
            if let Ok(Message::Text(text)) = msg_result {
                if let Ok(handshake) = serde_json::from_str::<Handshake>(&text) {
                    agent_id = Some(handshake.agent_id.clone());
                    
                    // Register client
                    clients.write().await.insert(handshake.agent_id.clone(), client_tx.clone());
                    
                    let _ = message_tx.send(ServerMessage::Connected {
                        agent_id: handshake.agent_id.clone(),
                    });

                    // Send acknowledgment
                    let ack = HandshakeAck {
                        status: "ok".to_string(),
                        message: "Connected successfully".to_string(),
                    };
                    let ack_msg = Message::Text(serde_json::to_string(&ack)?);
                    ws_sender.send(ack_msg).await?;
                }
            }
        }

        let agent_id = match agent_id {
            Some(id) => id,
            None => return Err(anyhow!("No handshake received")),
        };

        // Create a channel for ping/pong responses
        let (pong_tx, mut pong_rx) = mpsc::unbounded_channel::<Message>();

        // Spawn task to send messages to client
        let send_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(msg) = client_rx.next() => {
                        if ws_sender.send(msg).await.is_err() {
                            break;
                        }
                    }
                    Some(pong_msg) = pong_rx.recv() => {
                        if ws_sender.send(pong_msg).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        // Receive messages from client
        while let Some(msg_result) = ws_receiver.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    if let Ok(msg) = serde_json::from_str::<InterAgentMessage>(&text) {
                        let _ = message_tx.send(ServerMessage::Message {
                            from: agent_id.clone(),
                            msg,
                        });
                    }
                }
                Ok(Message::Ping(data)) => {
                    let _ = pong_tx.send(Message::Pong(data));
                }
                Ok(Message::Close(_)) => {
                    break;
                }
                Err(e) => {
                    tracing::error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        // Cleanup
        clients.write().await.remove(&agent_id);
        let _ = message_tx.send(ServerMessage::Disconnected { agent_id });
        send_task.abort();

        Ok(())
    }

    /// Send a message to a specific agent
    pub async fn send_to(&self, agent_id: &AgentId, msg: InterAgentMessage) -> Result<()> {
        let clients = self.clients.read().await;
        if let Some(sender) = clients.get(agent_id) {
            let text = serde_json::to_string(&msg)?;
            sender.send(Message::Text(text))?;
            Ok(())
        } else {
            Err(anyhow!("Agent {} not connected", agent_id))
        }
    }

    /// Broadcast a message to all connected agents
    pub async fn broadcast(&self, msg: InterAgentMessage) -> Result<()> {
        let clients = self.clients.read().await;
        let text = serde_json::to_string(&msg)?;
        
        for sender in clients.values() {
            let _ = sender.send(Message::Text(text.clone()));
        }
        
        Ok(())
    }

    /// Get list of connected agents
    pub async fn connected_agents(&self) -> Vec<AgentId> {
        self.clients.read().await.keys().cloned().collect()
    }
}

/// Handshake message
#[derive(Debug, Serialize, Deserialize)]
pub struct Handshake {
    pub agent_id: AgentId,
    pub protocol_version: String,
}

/// Handshake acknowledgment
#[derive(Debug, Serialize, Deserialize)]
pub struct HandshakeAck {
    pub status: String,
    pub message: String,
}

/// WebSocket client
pub struct WebSocketClient {
    url: String,
    agent_id: AgentId,
    sender: mpsc::UnboundedSender<Message>,
    receiver: mpsc::UnboundedReceiver<InterAgentMessage>,
}

impl WebSocketClient {
    /// Get the agent ID
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }
    
    /// Get the URL
    pub fn url(&self) -> &str {
        &self.url
    }
    
    /// Connect to a WebSocket server
    pub async fn connect(url: String, agent_id: AgentId) -> Result<Self> {
        let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await?;
        let (ws_sender, ws_receiver) = ws_stream.split();

        // Send handshake
        let handshake = Handshake {
            agent_id: agent_id.clone(),
            protocol_version: "1.0".to_string(),
        };
        let handshake_msg = Message::Text(serde_json::to_string(&handshake)?);
        let mut ws_sender = ws_sender;
        ws_sender.send(handshake_msg).await?;

        // Wait for acknowledgment
        let mut ws_receiver = ws_receiver;
        if let Some(msg_result) = ws_receiver.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    let ack: HandshakeAck = serde_json::from_str(&text)?;
                    if ack.status != "ok" {
                        return Err(anyhow!("Handshake failed: {}", ack.message));
                    }
                }
                _ => return Err(anyhow!("Invalid handshake response")),
            }
        } else {
            return Err(anyhow!("No handshake response"));
        }

        // Create channels
        let (sender, mut send_rx) = mpsc::unbounded_channel();
        let (recv_tx, receiver) = mpsc::unbounded_channel();

        // Spawn send task
        tokio::spawn(async move {
            while let Some(msg) = send_rx.recv().await {
                if ws_sender.send(msg).await.is_err() {
                    break;
                }
            }
        });

        // Spawn receive task
        tokio::spawn(async move {
            while let Some(msg_result) = ws_receiver.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        if let Ok(msg) = serde_json::from_str::<InterAgentMessage>(&text) {
                            if recv_tx.send(msg).is_err() {
                                break;
                            }
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Err(_) => break,
                    _ => {}
                }
            }
        });

        Ok(Self {
            url,
            agent_id,
            sender,
            receiver,
        })
    }

    /// Send a message
    pub async fn send(&mut self, msg: InterAgentMessage) -> Result<()> {
        let text = serde_json::to_string(&msg)?;
        self.sender.send(Message::Text(text))?;
        Ok(())
    }

    /// Receive a message
    pub async fn receive(&mut self) -> Result<InterAgentMessage> {
        self.receiver
            .recv()
            .await
            .ok_or_else(|| anyhow!("Connection closed"))
    }

    /// Try to receive a message without blocking
    pub fn try_receive(&mut self) -> Option<InterAgentMessage> {
        self.receiver.try_recv().ok()
    }

    /// Send heartbeat
    pub async fn heartbeat(&mut self) -> Result<()> {
        self.sender.send(Message::Ping(vec![]))?;
        Ok(())
    }

    /// Close connection
    pub async fn close(&mut self) -> Result<()> {
        self.sender.send(Message::Close(None))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handshake_serialization() {
        let handshake = Handshake {
            agent_id: "test-agent".to_string(),
            protocol_version: "1.0".to_string(),
        };

        let json = serde_json::to_string(&handshake).unwrap();
        let deserialized: Handshake = serde_json::from_str(&json).unwrap();

        assert_eq!(handshake.agent_id, deserialized.agent_id);
    }

    #[tokio::test]
    async fn test_server_creation() {
        let server = WebSocketServer::new("127.0.0.1:18080".parse().unwrap());
        assert_eq!(server.addr.port(), 18080);
    }
}
