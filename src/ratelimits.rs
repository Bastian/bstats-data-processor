use redis::{aio::ConnectionLike, AsyncCommands};

pub async fn is_ratelimited<C: ConnectionLike + AsyncCommands>(
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
    return Ok(false);
}

async fn _is_ratelimited<C: ConnectionLike + AsyncCommands>(
    con: &mut C,
    identifier: &str,
    software_url: &str,
    max_requests_per_ip: u16,
    tms2000: i64,
) -> Result<bool, redis::RedisError> {
    let key = format!("ratelimit:{}:{}:{}", identifier, software_url, tms2000);

    let request_count: Vec<u16> = redis::pipe()
        .atomic()
        .incr(&key, 1)
        .expire(&key, 60 * 31)
        .ignore()
        .query_async(con)
        .await?;

    return Ok(*request_count.get(0).unwrap() > max_requests_per_ip);
}
