extern crate http;
extern crate crypto;
extern crate chrono;
extern crate uuid;
extern crate url;

mod credential;

use crypto::sha2::Sha256;
use crypto::digest::Digest;
use crate::credential::Credential;
use http::Request;
use http::header::HeaderValue;
use chrono::prelude::*;
use uuid::Uuid;
use url::percent_encoding::{utf8_percent_encode, EncodeSet};

static EMPTY_STRING_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
static LONG_DATE_FORMAT_STR: &str = "%Y%m%dT%H%M%SZ";
static DATE_HEADER: &str = "x-jdcloud-date";
static NONCE_HEADER: &str = "x-jdcloud-nonce";

pub struct JdcloudSigner {
    credential: Credential,
    service_name: String,
    region: String,
}

impl JdcloudSigner {
    pub fn new(credential: Credential, service_name: String, region: String) -> JdcloudSigner {
        JdcloudSigner {
            credential,
            service_name,
            region
        }
    }

    pub fn sign_request(&self, request: &Request<String>) -> Result<Request<String>, &str> {
        if !self.credential.is_valid() {
            panic!("invalid credential");
        }
        let payload_hash = self.compute_payload_hash(request);
        let utc: DateTime<Utc> = Utc::now();
        let utc = utc.format(LONG_DATE_FORMAT_STR).to_string();

        let mut res = Request::builder();
        res.header(DATE_HEADER, HeaderValue::from_str(&utc).unwrap());
        res.header(NONCE_HEADER, Uuid::new_v4().to_hyphenated().to_string());
        // string dateHeaderValue = now.ToGmtString(LONG_DATE_FORMAT_STR);


        Ok(Request::builder().body("".to_string()).unwrap())
    }

    fn compute_payload_hash(&self, request: &Request<String>) -> String {
        if request.body().is_empty() {
            EMPTY_STRING_SHA256.to_string()
        } else {
            let mut hasher = Sha256::new();
            hasher.input_str(request.body());
            hasher.result_str()
        }
    }
}

fn should_sign_header(header: &str) -> bool {
    return !(header.eq_ignore_ascii_case("user-agent") || header.eq_ignore_ascii_case("authorization"))
}

fn make_cananical_request_str(request: &Request<String>) -> String {
    let mut res: String = "".to_owned();
    res.push_str(request.method().as_str());
    res.push('\n');
    res.push_str(request.uri().path());
    res.push('\n');
    res.push_str(&make_cananical_query_str(request));
    res.push('\n');
    res.push_str(&make_cananical_header_str(request));
    res.push('\n');
    res
}

fn make_cananical_header_str(request: &Request<String>) -> String {
    let mut header_names = Vec::new();
    for header_name in request.headers().into_iter() {
        header_names.push(header_name);
    }
    header_names.sort_by(|a, b|{
        a.0.as_str().partial_cmp(b.0.as_str()).unwrap()
    });
    let mut res: String = "".to_owned();
    for x in header_names {
        res.push_str(x.0.as_str());
        res.push(':');
        res.push_str(x.1.to_str().unwrap().trim_matches(' '));
        res.push('\n');
    }
    res
}

fn make_cananical_query_str(request: &Request<String>) -> String {
    let query = request.uri().query();
    let query = match query {
        None => "",
        Some(q) => q
    };
    let query = url::form_urlencoded::parse(query.as_bytes());
    let mut vec = Vec::new();
    for q in query {
        vec.push((q.0.to_string(), q.1.to_string()));
    }
    vec.sort_by(|a, b| {
        if a.0 == b.0 {
            a.1.partial_cmp(&b.1).unwrap()
        } else {
            a.0.partial_cmp(&b.0).unwrap()
        }
    });
    let mut res: String = "".to_owned();
    let mut first = true;
    for x in vec {
        if !first {
            res.push('&');
        }
        first = false;
        res.push_str(&utf8_percent_encode(&x.0, Aws4QueryItemEncodeSet).to_string());
        res.push('=');
        res.push_str(&utf8_percent_encode(&x.1, Aws4QueryItemEncodeSet).to_string());
    }
    res
}

#[derive(Copy, Clone, Debug)]
struct Aws4QueryItemEncodeSet;

impl EncodeSet for Aws4QueryItemEncodeSet {
    #[inline]
    fn contains(&self, c: u8) -> bool {
        !(('A' as u8 <= c && c <= 'Z' as u8)
            || ('a' as u8 <= c && c <= 'z' as u8)
            || ('0' as u8 <= c && c <= '9' as u8)
            || c == '-' as u8
            || c == '_' as u8
            || c == '.' as u8
            || c == '~' as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::header::{CONTENT_TYPE, USER_AGENT};

    #[test]
    fn test_new() {
        let c = Credential::new("ak".to_string(), "sk".to_string());
        let s = JdcloudSigner::new(c, "service_name".to_string(), "cn-north-1".to_string());
        let mut req = Request::builder();
        let req = req.uri("http://www.jdcloud-api.com/v1/regions/cn-north-1/instances?pageNumber=2&pageSize=10")
            .method("GET")
            .header(CONTENT_TYPE, "application/json")
            .header(USER_AGENT, "JdcloudSdkRust/0.0.1 vm/0.7.4")
            .body("".to_string())
            .unwrap();
        let req = s.sign_request(&req);
    }

    #[test]
    fn test_should_sign_header() {
        let should_not_sign_headers = ["user-agent", "User-Agent", "Authorization", "authorization"];
        let should_sign_headers = ["X-hello", "Content-Length", "User"];
        for header in should_not_sign_headers.iter() {
            assert!(!should_sign_header(header))
        }
        for header in should_sign_headers.iter() {
            assert!(should_sign_header(header))
        }
    }

    #[test]
    fn test_make_cananical_request_str() {
        let req = Request::builder().method("GET").body("".to_string()).unwrap();
        assert_eq!(make_cananical_request_str(&req), "GET\n/\n\n\n");
        let req = Request::builder().method("POST").body("".to_string()).unwrap();
        assert_eq!(make_cananical_request_str(&req), "POST\n/\n\n\n");
        let req = Request::builder().method("GET").uri("/helloworld").body("".to_string()).unwrap();
        assert_eq!(make_cananical_request_str(&req), "GET\n/helloworld\n\n\n");
        let req = Request::builder().method("GET").uri("/hello%20world").body("".to_string()).unwrap();
        assert_eq!(make_cananical_request_str(&req), "GET\n/hello%20world\n\n\n");
        let req = Request::builder().method("GET").uri("/Hello%20world").body("".to_string()).unwrap();
        assert_eq!(make_cananical_request_str(&req), "GET\n/Hello%20world\n\n\n");
        let req = Request::builder().method("GET").uri("/Hello%20world?").body("".to_string()).unwrap();
        assert_eq!(make_cananical_request_str(&req), "GET\n/Hello%20world\n\n\n");
        let req = Request::builder().method("GET").uri("/Hello%20world?a=1").body("".to_string()).unwrap();
        assert_eq!(make_cananical_request_str(&req), "GET\n/Hello%20world\na=1\n\n");
    }

    #[test]
    fn test_make_cananical_header_str() {
        let req = Request::builder().method("GET").body("".to_string()).unwrap();
        assert_eq!(make_cananical_header_str(&req), "");

        let single_header_tcs = vec![
            ("Hello", "World", "hello:World\n"),
            ("Hello", "\"World\"", "hello:\"World\"\n"),
            ("Hello", " World ", "hello:World\n"),
            ("Hello", "  World  ", "hello:World\n"),
            ("Hello", "  World", "hello:World\n"),
            ("Hello", "World  ", "hello:World\n"),
            ("Hello", "", "hello:\n"),
            ("Hello", "  ", "hello:\n"),
            ("Hello", "  \t", "hello:\t\n"),
        ];

        for tc in single_header_tcs {
            let req = Request::builder().method("GET")
                .header(tc.0, tc.1)
                .body("".to_string()).unwrap();
            assert_eq!(make_cananical_header_str(&req), tc.2);
        }

        let multi_header_cases = vec![
            (vec![("Hello", "World")], "hello:World\n"),
            (vec![("Hello", "World"), ("A", "B")], "a:B\nhello:World\n"),
            (vec![("A", "A"), ("B", "B")], "a:A\nb:B\n"),
            (vec![("B", "B"), ("A", "A")], "a:A\nb:B\n"),
        ];
        for tc in multi_header_cases {
            let mut req = Request::builder();
            req.method("GET");
            for x in tc.0 {
                req.header(x.0, x.1);
            }
            let req = req.body("".to_string()).unwrap();
            assert_eq!(make_cananical_header_str(&req), tc.1);
        }
    }

    #[test]
    fn test_make_cananical_query_str() {
        let req = Request::builder().method("GET").body("".to_string()).unwrap();
        assert_eq!(make_cananical_query_str(&req), "");
        let testcases = vec![
            ("/", ""),
            ("/?", ""),
            ("/?a=1", "a=1"),
            ("/?a=1#bcd", "a=1"),
            ("/?a=1&b=1", "a=1&b=1"),
            ("/?a&b", "a=&b="),
            ("/?a=&b", "a=&b="),
            ("/?a&b=", "a=&b="),
            ("/?a=&b=", "a=&b="),
            ("/?b&a", "a=&b="),
            ("/?b&a&B&A", "A=&B=&a=&b="),
            ("/?a=-_.~", "a=-_.~"),
            ("/?a=/", "a=%2F"),
            ("/?a=%", "a=%25"),
            ("/?a=%2", "a=%252"),
            ("/?a=%00", "a=%00"),
            ("/?a=%ff", "a=%EF%BF%BD"),
            ("/?a=%0g", "a=%250g"),
            ("/?a=%fg", "a=%25fg"),
            ("/?b&a=%e4%b8%ad", "a=%E4%B8%AD&b="),
            ("/?b&a=%e4%b8", "a=%EF%BF%BD&b="),
            ("/?b&a=%2f%25%20", "a=%2F%25%20&b="),
            ("/?b&a=%2f%25%20", "a=%2F%25%20&b="),
            ("/?b&a=+++", "a=%20%20%20&b="),
            ("/?a=2&a=1", "a=1&a=2"),
            ("/?a=1&a=1", "a=1&a=1"),
        ];
        for tc in testcases {
            let req = Request::builder().method("GET").uri(tc.0).body("".to_string()).unwrap();
            assert_eq!(make_cananical_query_str(&req), tc.1);
        }
    }
}
