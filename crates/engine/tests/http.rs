use engine::http::{Client, Error};
use std::time::{Duration, Instant};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

#[tokio::test]
async fn retries_503_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;
    let client = Client::with_timeout(Duration::from_secs(2), Duration::ZERO).unwrap();
    assert_eq!(client.get(&server.uri()).await.unwrap().body, b"ok");
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn stops_after_three_attempts_and_honors_retry_after() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "1"))
        .expect(3)
        .mount(&server)
        .await;
    let client = Client::with_timeout(Duration::from_secs(2), Duration::ZERO).unwrap();
    let start = Instant::now();
    assert_eq!(client.get(&server.uri()).await.unwrap().status, 429);
    assert!(start.elapsed() >= Duration::from_secs(2));
}

#[tokio::test]
async fn rejects_25_mib_and_retries_timeouts() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 25 * 1024 * 1024]))
        .mount(&server)
        .await;
    assert!(matches!(
        Client::new().unwrap().get(&server.uri()).await,
        Err(Error::TooLarge)
    ));
    server.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(1)))
        .expect(3)
        .mount(&server)
        .await;
    assert!(matches!(
        Client::with_timeout(Duration::from_millis(30), Duration::ZERO)
            .unwrap()
            .get(&server.uri())
            .await,
        Err(Error::Timeout)
    ));
}
