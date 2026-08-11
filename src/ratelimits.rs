use redis::AsyncCommands;

pub async fn is_ratelimited<C: AsyncCommands>(
    con: &mut C,
    software_url: &str,
    max_requests_per_ip: u16,
    server_uuid: &str,
    ip: &str,
    service_id: u32,
    tms2000: i64,
) -> Result<bool, redis::RedisError> {
    if _is_ratelimited(
        con,
        &format!("{}#{}", service_id, server_uuid),
        software_url,
        1,
        tms2000,
    )
    .await?
    {
        return Ok(true);
    }
    if _is_ratelimited(
        con,
        &format!("{}#{}", service_id, ip),
        software_url,
        max_requests_per_ip,
        tms2000,
    )
    .await?
    {
        return Ok(true);
    }
    Ok(false)
}

async fn _is_ratelimited<C: AsyncCommands>(
    con: &mut C,
    identifier: &str,
    software_url: &str,
    max_requests_per_ip: u16,
    tms2000: i64,
) -> Result<bool, redis::RedisError> {
    let key = format!("ratelimit:{}.{}.{}", identifier, software_url, tms2000);

    let request_count: Vec<u16> = redis::pipe()
        .incr(&key, 1)
        .expire(&key, 60 * 31)
        .ignore()
        .query_async(con)
        .await?;

    Ok(*request_count.first().unwrap() > max_requests_per_ip)
}

#[cfg(all(test, feature = "integration-tests"))]
mod integration_tests {
    use super::*;
    use crate::test_support::test_environment::TestEnvironment;

    #[tokio::test]
    async fn test_check_ratelimits() {
        let test_environment = TestEnvironment::empty().await;

        let mut con = test_environment.redis_connection().await;

        let software_url = "bukkit";
        let max_requests_per_ip = 3;
        let tms2000: i64 = 1337;

        // The first request should not be ratelimited
        assert!(
            !is_ratelimited(
                &mut con,
                software_url,
                max_requests_per_ip,
                "server-uuid-1",
                "127.0.0.1",
                1,
                tms2000
            )
            .await
            .unwrap()
        );

        // A second request from the same server uuid and for the same service
        // should be ratelimited
        assert!(
            is_ratelimited(
                &mut con,
                software_url,
                max_requests_per_ip,
                "server-uuid-1",
                "127.0.0.1",
                1,
                tms2000
            )
            .await
            .unwrap()
        );

        // However, a request from a different server uuid should not be ratelimited
        assert!(
            !is_ratelimited(
                &mut con,
                software_url,
                max_requests_per_ip,
                "server-uuid-2",
                "127.0.0.1",
                1,
                tms2000
            )
            .await
            .unwrap()
        );

        // We are now at 2 successful requests. Since the limit is 3, the next request
        // should not be ratelimited
        assert!(
            !is_ratelimited(
                &mut con,
                software_url,
                max_requests_per_ip,
                "server-uuid-3",
                "127.0.0.1",
                1,
                tms2000
            )
            .await
            .unwrap()
        );

        // We are now at 3 successful requests. Now the next request should be ratelimited
        assert!(
            is_ratelimited(
                &mut con,
                software_url,
                max_requests_per_ip,
                "server-uuid-4",
                "127.0.0.1",
                1,
                tms2000
            )
            .await
            .unwrap()
        );

        // But for a different ip and server uuid, the request should not be ratelimited
        assert!(
            !is_ratelimited(
                &mut con,
                software_url,
                max_requests_per_ip,
                "server-uuid-5",
                "127.0.0.2",
                1,
                tms2000
            )
            .await
            .unwrap()
        );

        // Also, every service has its own ratelimit, so a request for a different service
        // should not be ratelimited
        assert!(
            !is_ratelimited(
                &mut con,
                software_url,
                max_requests_per_ip,
                "server-uuid-4",
                "127.0.0.1",
                2,
                tms2000
            )
            .await
            .unwrap()
        );
    }
}
