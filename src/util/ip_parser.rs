use actix_web::{error, HttpRequest};

/// Get the IP address of the client making the request.

pub fn get_ip(request: &HttpRequest) -> Result<String, error::Error> {
    let behind_cloudflare =
        std::env::var("BEHIND_CLOUDFLARE_PROXY").unwrap_or(String::from("false")) == "true";
    let behind_proxy = std::env::var("BEHIND_PROXY").unwrap_or(String::from("false")) == "true";

    if behind_cloudflare {
        if let Some(header) = request.headers().get("cf-connecting-ip") {
            return Ok(header.to_str().unwrap().to_string());
        }
    }

    if behind_proxy {
        return Ok(request
            .connection_info()
            .realip_remote_addr()
            .unwrap()
            .to_string());
    }

    Ok(request.peer_addr().unwrap().ip().to_string())
}

#[cfg(test)]
mod tests {
    use actix_web::{http::header, test};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use super::*;

    #[tokio::test]
    async fn test_get_ip() {
        let req = test::TestRequest::get()
            .peer_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 1111))
            .insert_header((
                header::HeaderName::from_static("cf-connecting-ip"),
                header::HeaderValue::from_static("2.2.2.2"),
            ))
            .insert_header((
                header::HeaderName::from_static("x-forwarded-for"),
                header::HeaderValue::from_static("3.3.3.3"),
            ))
            .insert_header((
                header::HeaderName::from_static("forwarded"),
                header::HeaderValue::from_static("for=4.4.4.4"),
            ))
            .to_http_request();

        // Should not use proxy ips when not behind Cloudflare
        let ip = get_ip(&req).unwrap();
        assert_eq!(ip, "1.1.1.1");

        // Should use Cloudflare header when behind Cloudflare
        std::env::set_var("BEHIND_CLOUDFLARE_PROXY", "true");
        let ip = get_ip(&req).unwrap();
        assert_eq!(ip, "2.2.2.2");

        // Should use proxy ip when behind proxy
        std::env::set_var("BEHIND_PROXY", "true");
        std::env::set_var("BEHIND_CLOUDFLARE_PROXY", "false");

        let ip = get_ip(&req).unwrap();
        assert_eq!(ip, "4.4.4.4");
    }
}
