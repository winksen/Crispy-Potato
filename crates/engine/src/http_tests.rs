use engine::http::{Client, Error};
use std::time::{Duration, Instant};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::method};

#[tokio::test]
async fn source_requests_require_allowlisted_https_hosts() {
    let client = Client::new().unwrap();
    for url in [
        "http://api.github.com/repos/example/test",
        "https://example.com/",
        "https://api.github.com:8443/",
    ] {
        assert!(matches!(client.get(url).await, Err(Error::Blocked)));
    }
    let links = Client::for_links(Duration::from_secs(1)).unwrap();
    for url in [
        "http://127.0.0.1/",
        "http://169.254.169.254/",
        "http://[::1]/",
        "http://[::ffff:127.0.0.1]/",
    ] {
        assert!(matches!(links.get(url).await, Err(Error::Blocked)));
    }
}

#[tokio::test]
async fn redirects_cannot_leave_test_origin_for_internal_addresses() {
    let server = MockServer::start().await;
    let client = Client::test_local(&server.uri(), Duration::from_secs(2), Duration::ZERO).unwrap();
    for target in [
        "http://127.0.0.1:1/",
        "http://169.254.169.254/latest/meta-data",
        "http://[::1]/",
    ] {
        server.reset().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(302).insert_header("Location", target))
            .expect(1)
            .mount(&server)
            .await;
        assert!(matches!(
            client.get(&server.uri()).await,
            Err(Error::Blocked)
        ));
    }
}

#[tokio::test]
async fn timeout_releases_concurrency_slot_before_retry_delay() {
    // A distinct loopback host keeps this test independent of other mock clients.
    let listener = std::net::TcpListener::bind("127.0.0.9:0").unwrap();
    let server = MockServer::builder().listener(listener).start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(1)))
        .mount(&server)
        .await;
    let host = engine::limits::host("127.0.0.9").await;
    let occupied = host.acquire().await.unwrap();
    let client = Client::test_local(
        &server.uri(),
        Duration::from_millis(40),
        Duration::from_secs(2),
    )
    .unwrap();
    let url = server.uri();
    let task = tokio::spawn(async move { client.get(&url).await });
    tokio::time::timeout(Duration::from_secs(2), async {
        while server.received_requests().await.unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(80)).await;
    let available = tokio::time::timeout(Duration::from_millis(400), host.acquire()).await;
    task.abort();
    assert!(
        available.is_ok(),
        "timeout held its slot through retry sleep"
    );
    drop(available);
    drop(occupied);
}

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
    let client = Client::test_local(&server.uri(), Duration::from_secs(2), Duration::ZERO).unwrap();
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
    let client = Client::test_local(&server.uri(), Duration::from_secs(2), Duration::ZERO).unwrap();
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
        Client::test_local(&server.uri(), Duration::from_secs(30), Duration::ZERO)
            .unwrap()
            .get(&server.uri())
            .await,
        Err(Error::TooLarge)
    ));
    server.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(1)))
        .expect(3)
        .mount(&server)
        .await;
    assert!(matches!(
        Client::test_local(&server.uri(), Duration::from_millis(30), Duration::ZERO)
            .unwrap()
            .get(&server.uri())
            .await,
        Err(Error::Timeout)
    ));
}
