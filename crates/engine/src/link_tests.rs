use engine::links::{self, Link};
use std::time::Duration;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};
fn link(url: String) -> Link {
    Link {
        file: "languages/rust.yaml".into(),
        field: "links.website".into(),
        url,
    }
}

#[tokio::test]
async fn duplicate_url_is_fetched_once_and_reported_for_each_field() {
    let server = MockServer::start().await;
    for verb in ["HEAD", "GET"] {
        Mock::given(method(verb))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&server)
            .await;
    }
    let first = link(server.uri());
    let mut second = first.clone();
    second.field = "links.docs".into();
    let report = links::check_with_client(
        vec![first, second],
        Duration::from_secs(10),
        engine::http::Client::test_local(&server.uri(), Duration::from_secs(10), Duration::ZERO)
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(report.checked, 2);
    assert_eq!(report.problems.len(), 2);
    assert_eq!(report.problems[0].field, "links.docs");
    assert_eq!(report.problems[1].field, "links.website");
}

#[tokio::test]
async fn head_fallback_and_broken_links_report_file_and_field() {
    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/ok"))
        .respond_with(ResponseTemplate::new(405))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ok"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(path("/broken"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let report = links::check_with_client(
        vec![
            link(format!("{}/ok", server.uri())),
            link(format!("{}/broken", server.uri())),
        ],
        Duration::from_secs(10),
        engine::http::Client::test_local(&server.uri(), Duration::from_secs(10), Duration::ZERO)
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(report.checked, 2);
    assert_eq!(report.problems.len(), 1);
    assert_eq!(report.problems[0].file, "languages/rust.yaml");
    assert_eq!(report.problems[0].field, "links.website");
    assert_eq!(report.problems[0].message, "HTTP 404");
}

#[tokio::test]
async fn refuses_private_redirect_and_reports_timeout() {
    let server = MockServer::start().await;
    let target = MockServer::start().await;
    Mock::given(method("HEAD"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&target)
        .await;
    Mock::given(path("/redirect"))
        .respond_with(
            ResponseTemplate::new(302)
                .insert_header("Location", target.uri().replace("127.0.0.1", "localhost")),
        )
        .mount(&server)
        .await;
    let report = links::check_with_client(
        vec![link(format!("{}/redirect", server.uri()))],
        Duration::from_secs(10),
        engine::http::Client::test_local(&server.uri(), Duration::from_secs(10), Duration::ZERO)
            .unwrap(),
    )
    .await
    .unwrap();
    assert!(
        report.problems[0]
            .message
            .contains("egress destination refused")
    );
    Mock::given(path("/slow"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .mount(&server)
        .await;
    let report = links::check_with_client(
        vec![link(format!("{}/slow", server.uri()))],
        Duration::from_millis(300),
        engine::http::Client::test_local(&server.uri(), Duration::from_millis(300), Duration::ZERO)
            .unwrap(),
    )
    .await
    .unwrap();
    assert!(report.problems[0].message.contains("timeout"));
}
