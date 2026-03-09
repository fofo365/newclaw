// Example: Inter-Agent Messaging
// Demonstrates how agents communicate through the message queue

use newclaw::{
    communication::{
        message::{AgentId, InterAgentMessage, MessagePayload},
        queue::{MessageQueue, QueueConfig},
    },
    core::context::ContextManager,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🤖 Inter-Agent Messaging Demo\n");

    // 1. Create message queue
    println!("✓ Creating message queue...");
    let config = QueueConfig {
        max_size: 1000,
        processing_timeout_ms: 5000,
    };
    let queue = MessageQueue::new(config);

    // 2. Create context manager
    println!("✓ Creating context manager...");
    let ctx_manager = ContextManager::new();

    // 3. Setup agents
    let agent1 = AgentId("agent-alice".to_string());
    let agent2 = AgentId("agent-bob".to_string());

    println!("✓ Created agents: {:?}, {:?}", agent1, agent2);

    // 4. Create isolated contexts for each agent
    let ctx1 = ctx_manager.create_context(&agent1.0).await?;
    let ctx2 = ctx_manager.create_context(&agent2.0).await?;

    println!("✓ Created isolated contexts\n");

    // 5. Agent 1 sends message to Agent 2
    println!("📤 Agent {:?} sending message to {:?}", agent1, agent2);
    let msg1 = InterAgentMessage {
        from: agent1.clone(),
        to: agent2.clone(),
        payload: MessagePayload::Text("Hello Bob!".to_string()),
        timestamp: chrono::Utc::now(),
    };
    queue.enqueue(msg1).await?;
    println!("   ✓ Message enqueued\n");

    // 6. Agent 2 receives and processes message
    println!("📥 Agent {:?} receiving message...", agent2);
    let received = queue.dequeue().await?;
    println!("   ✓ Received: {:?}", received.payload);

    // 7. Agent 2 replies
    println!("\n📤 Agent {:?} sending reply to {:?}", agent2, agent1);
    let msg2 = InterAgentMessage {
        from: agent2.clone(),
        to: agent1.clone(),
        payload: MessagePayload::Json(serde_json::json!({
            "status": "received",
            "reply": "Hello Alice!"
        })),
        timestamp: chrono::Utc::now(),
    };
    queue.enqueue(msg2).await?;
    println!("   ✓ Reply enqueued\n");

    // 8. Demonstrate context isolation
    println!("🧠 Testing context isolation...");
    ctx_manager.set_data(&ctx1, "state", "waiting_for_response").await?;
    ctx_manager.set_data(&ctx2, "state", "processing_request").await?;

    let state1 = ctx_manager.get_data(&ctx1, "state").await?;
    let state2 = ctx_manager.get_data(&ctx2, "state").await?;

    println!("   Agent 1 state: {:?}", state1);
    println!("   Agent 2 state: {:?}", state2);
    println!("   ✓ Contexts are isolated\n");

    println!("✅ Demo completed successfully!");
    Ok(())
}
