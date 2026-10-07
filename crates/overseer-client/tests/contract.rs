use overseer_client::{
    ClientError, Conversation, OverseerClient, RuntimeRegistryStatus, SystemLoad,
};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

async fn mock(body: &str, status: &str) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let body = body.to_owned();
    let status = status.to_owned();
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut chunk = [0; 4096];
        loop {
            let count = socket.read(&mut chunk).await.unwrap();
            if count == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..count]);
            if let Some(end) = request.windows(4).position(|x| x == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]).to_lowercase();
                let length = headers
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("content-length:")
                            .and_then(|v| v.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if request.len() >= end + 4 + length {
                    break;
                }
            }
        }
        socket
            .write_all(
                format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        String::from_utf8(request).unwrap()
    });
    (format!("http://{address}"), task)
}
#[tokio::test]
async fn exact_ids_and_auth_over_http() {
    let (url, task) = mock("9223372036854775806", "200 OK").await;
    let client = OverseerClient::new(&url).unwrap().with_token("test-token");
    assert_eq!(
        client.create_conversation("SDK").await.unwrap(),
        9223372036854775806
    );
    let request = task.await.unwrap();
    assert!(request.starts_with("POST /api/conversations "));
    assert!(request
        .to_lowercase()
        .contains("authorization: bearer test-token"));
}
#[tokio::test]
async fn orchestration_returns_plain_text() {
    let (url, task) = mock("plain answer", "200 OK").await;
    assert_eq!(
        OverseerClient::new(&url)
            .unwrap()
            .orchestrate(9223372036854775806, "hello")
            .await
            .unwrap(),
        "plain answer"
    );
    assert!(task.await.unwrap().contains("9223372036854775806"));
}
#[tokio::test]
async fn patch_errors_keep_server_body() {
    let (url, task) = mock("credential missing", "404 Not Found").await;
    let error = OverseerClient::new(&url)
        .unwrap()
        .update_credential(1, None, Some("value"))
        .await
        .unwrap_err();
    assert!(matches!(error,ClientError::Api {body,..} if body == "credential missing"));
    task.await.unwrap();
}
#[tokio::test]
async fn json_extension_and_origin_validation() {
    assert!(OverseerClient::new("file:///tmp/data").is_err());
    assert!(OverseerClient::new("http://user:pass@localhost:8080").is_err());
    let (url, task) = mock("{\"result\":true}", "200 OK").await;
    let client = OverseerClient::new(&url)
        .unwrap()
        .with_timeout(Duration::from_secs(1))
        .unwrap();
    assert!(client
        .request_json(reqwest::Method::GET, "//example.invalid", None)
        .await
        .is_err());
    assert!(client
        .request_json(reqwest::Method::GET, "/\\\\example.invalid", None)
        .await
        .is_err());
    assert_eq!(
        client
            .request_json(reqwest::Method::GET, "/extra", None)
            .await
            .unwrap()["result"],
        true
    );
    task.await.unwrap();
}
#[test]
fn response_models_match_server() {
    let conversation: Conversation = serde_json::from_str(
        r#"{"id":9223372036854775806,"title":"SDK","created_at":"2026-10-05T00:00:00"}"#,
    )
    .unwrap();
    assert_eq!(conversation.id, 9223372036854775806);
    let load: SystemLoad = serde_json::from_str(r#"{"label":"System Load","usage":0}"#).unwrap();
    assert_eq!(load.usage, 0);
    let registry: RuntimeRegistryStatus =
        serde_json::from_str(r#"{"status":"live","refreshed_at":null,"error":null}"#).unwrap();
    assert_eq!(registry.status, "live");
}
