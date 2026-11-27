use ocaf_runtime::architect_https_source_fetch::{
    parse_response, PinnedSourceRoute, SourceFetchError,
};
use sha2::{Digest, Sha256};
use std::net::Ipv4Addr;

const NOW: u64 = 1_785_000_000;

fn route() -> PinnedSourceRoute {
    PinnedSourceRoute {
        tenant: "tenant_a".into(),
        hostname: "example.com".into(),
        pinned_ipv4: Ipv4Addr::new(104, 20, 23, 154),
        path: "/research?q=latest".into(),
        max_body_bytes: 32 * 1024,
        timeout_seconds: 8,
    }
}

#[test]
fn pinned_route_rejects_ssrf_special_addresses_injected_host_and_path() {
    let mut config = route();
    config.validate().unwrap();
    for ip in [
        Ipv4Addr::new(127, 0, 0, 1),
        Ipv4Addr::new(10, 0, 0, 1),
        Ipv4Addr::new(172, 16, 0, 1),
        Ipv4Addr::new(169, 254, 169, 254),
        Ipv4Addr::new(100, 64, 0, 1),
        Ipv4Addr::new(192, 168, 0, 1),
        Ipv4Addr::new(0, 0, 0, 0),
        Ipv4Addr::new(198, 51, 100, 42),
        Ipv4Addr::new(203, 0, 113, 1),
        Ipv4Addr::new(255, 255, 255, 255),
    ] {
        config.pinned_ipv4 = ip;
        assert_eq!(
            config.validate(),
            Err(SourceFetchError::InvalidRoute),
            "{ip}"
        );
    }
    config = route();
    for host in [
        "localhost",
        "169.254.169.254",
        "evil.com\r\nHost: internal",
        "foo..com",
        "local.localhost",
        "UPPER.LOCALHOST",
        "source.local",
        "-bad.com",
        "example.com:443",
    ] {
        config.hostname = host.into();
        assert_eq!(
            config.validate(),
            Err(SourceFetchError::InvalidRoute),
            "{host}"
        );
    }
    config = route();
    for path in [
        "http://internal/",
        "//internal/path",
        "/good\r\nHost: evil",
        "/good#fragment",
        "/safe\\escape",
        "",
    ] {
        config.path = path.into();
        assert_eq!(
            config.validate(),
            Err(SourceFetchError::InvalidRoute),
            "{path}"
        );
    }
}

#[test]
fn exact_https_text_response_binds_real_content_bytes_and_sha() {
    let response=b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nhello";
    let result =
        parse_response(response, "https://example.com/research?q=latest", NOW, 4096).unwrap();
    assert_eq!(result.body, b"hello");
    assert_eq!(result.source.url, "https://example.com/research?q=latest");
    assert_eq!(result.source.retrieved_unix, NOW);
    assert_eq!(
        result.source.content_sha256,
        format!("{:x}", Sha256::digest(b"hello"))
    );
    assert_eq!(result.status, 200);
}

#[test]
fn response_parser_rejects_redirect_compression_chunking_and_mismatched_length() {
    for response in [
        "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1/\r\nContent-Length: 0\r\n\r\n",
        "HTTP/1.1 200 OK\r\nContent-Length: 5\r\nContent-Encoding: gzip\r\n\r\nhello",
        "HTTP/1.1 200 OK\r\nContent-Length: 5\r\nContent-Length: 5\r\n\r\nhello",
        "HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\nhello",
        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: 5\r\n\r\nhello",
        "HTTP/2 200 OK\r\nContent-Length: 5\r\n\r\nhello",
    ] {
        assert_eq!(parse_response(response.as_bytes(),"https://example.com/",NOW,4096),
            Err(SourceFetchError::Response),"{response}");
    }
}

#[test]
fn response_parser_enforces_bounded_bytes_and_date() {
    let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\n0123456789";
    assert_eq!(
        parse_response(raw, "https://example.com/", NOW, 9),
        Err(SourceFetchError::TooLarge)
    );
    assert_eq!(
        parse_response(raw, "https://example.com/", 0, 4096),
        Err(SourceFetchError::InvalidRoute)
    );
    let mut r = route();
    r.timeout_seconds = 0;
    assert_eq!(r.validate(), Err(SourceFetchError::InvalidRoute));
    r = route();
    r.max_body_bytes = 0;
    assert_eq!(r.validate(), Err(SourceFetchError::InvalidRoute));
}

#[test]
fn parser_rejects_header_smuggling_and_unexpected_trailing_bytes() {
    for raw in [
        b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nTransfer-Encoding: identity\r\n\r\na".as_slice(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n X: value\r\n\r\na".as_slice(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\nab".as_slice(),
        b"HTTP/1.1 200 OK\nContent-Length: 1\n\nx".as_slice(),
    ] {
        assert_eq!(
            parse_response(raw, "https://example.com/", NOW, 4096),
            Err(SourceFetchError::Response)
        );
    }
}

#[test]
fn valid_bounded_http_chunked_text_is_decoded_and_hashed_not_rejected() {
    let raw=b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n";
    let fetched = parse_response(raw, "https://example.com/", NOW, 4096).unwrap();
    assert_eq!(fetched.body, b"hello world");
    assert_eq!(
        fetched.source.content_sha256,
        format!("{:x}", Sha256::digest(b"hello world"))
    );
}

#[test]
fn malformed_chunked_and_content_length_smuggling_fail_closed() {
    for raw in [
       "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Length: 5\r\n\r\n5\r\nhello\r\n0\r\n\r\n",
       "HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip, chunked\r\n\r\n0\r\n\r\n",
       "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhell\r\n0\r\n\r\n",
       "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX-Extra: bad\r\n\r\n",
       "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\nextra",
       "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
    ] {
        assert_eq!(parse_response(raw.as_bytes(),"https://example.com/",NOW,4096),
            Err(SourceFetchError::Response),"{raw}");
    }
}
