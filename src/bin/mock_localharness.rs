use antigravity_sdk_rust::proto::localharness::OutputConfig;
use futures_util::{SinkExt, StreamExt};
use prost::Message;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Read InputConfig (length prefix + payload) from stdin
    let mut stdin = tokio::io::stdin();
    let mut len_bytes = [0u8; 4];
    if stdin.read_exact(&mut len_bytes).await.is_err() {
        std::process::exit(1);
    }
    let length = u32::from_le_bytes(len_bytes) as usize;
    let mut input_buf = vec![0u8; length];
    stdin.read_exact(&mut input_buf).await?;

    // 2. Bind TCP listener to random port on localhost
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    // 3. Write OutputConfig to stdout
    let output_config = OutputConfig {
        port: Some(i32::from(port)),
        api_key: Some("mock_api_key".to_string()),
    };
    let mut output_buf = Vec::new();
    output_config.encode(&mut output_buf)?;

    let mut stdout = tokio::io::stdout();
    let size = output_buf.len() as u32;
    stdout.write_all(&size.to_le_bytes()).await?;
    stdout.write_all(&output_buf).await?;
    stdout.flush().await?;

    // 4. Accept a TCP connection and upgrade to WebSocket
    let (stream, _) = listener.accept().await?;
    let mut ws_stream = accept_async(stream).await?;

    // 5. Read client config message (InitializeConversationEvent / HarnessConfig)
    if let Some(msg_res) = ws_stream.next().await {
        let _ = msg_res?;
    }

    // 6. Send trajectoryStateUpdate (RUNNING) to signal the turn is active
    let traj_running = serde_json::json!({
        "trajectoryStateUpdate": {
            "trajectoryId": "test_traj",
            "state": "STATE_RUNNING"
        }
    });
    ws_stream
        .send(WsMessage::Text(traj_running.to_string()))
        .await?;

    // 7. Send the step updates
    let step1 = serde_json::json!({
        "stepUpdate": {
            "stepIndex": 1,
            "cascadeId": "test_traj",
            "trajectoryId": "test_traj",
            "text": "Hello from mock harness!",
            "textDelta": "Hello from mock harness!",
            "state": "STATE_ACTIVE",
            "source": "SOURCE_MODEL",
            "target": "TARGET_USER"
        }
    });
    ws_stream.send(WsMessage::Text(step1.to_string())).await?;

    tokio::time::sleep(Duration::from_millis(100)).await;

    let step2 = serde_json::json!({
        "stepUpdate": {
            "stepIndex": 2,
            "cascadeId": "test_traj",
            "trajectoryId": "test_traj",
            "text": "Hello from mock harness!How can I help you today?",
            "textDelta": "How can I help you today?",
            "state": "STATE_DONE",
            "source": "SOURCE_MODEL",
            "target": "TARGET_USER",
            "finish": {
                "outputString": "\"done\""
            }
        }
    });
    ws_stream.send(WsMessage::Text(step2.to_string())).await?;

    // 8. Send trajectoryStateUpdate (IDLE) to signal the turn is complete
    let traj_idle = serde_json::json!({
        "trajectoryStateUpdate": {
            "trajectoryId": "test_traj",
            "state": "STATE_IDLE"
        }
    });
    ws_stream
        .send(WsMessage::Text(traj_idle.to_string()))
        .await?;

    // 9. Keep reading until client disconnects or we get terminated
    while let Some(msg_res) = ws_stream.next().await {
        if msg_res.is_err() {
            break;
        }
    }

    Ok(())
}
