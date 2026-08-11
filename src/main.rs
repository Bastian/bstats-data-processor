use std::sync::Arc;

use actix_web::{App, HttpServer, web};
use data_processor::chart_buffer::{ChartBuffer, ChartFlusher, flush_loop};
use data_processor::{routes, util::redis::get_redis_cluster_pool};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .unwrap();

    let pool = get_redis_cluster_pool().await;

    let chart_buffer = Arc::new(ChartBuffer::new());
    let flusher = ChartFlusher::new().expect("invalid REDIS_CLUSTER__URLS");
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let flush_task = tokio::spawn(flush_loop(chart_buffer.clone(), flusher, shutdown_rx));

    let chart_buffer = web::Data::from(chart_buffer);
    let mut http_server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(chart_buffer.clone())
            .service(routes::submit_data::submit_data)
            .service(routes::legacy::legacy_submit_data)
    });

    if let Ok(workers) = std::env::var("WORKERS") {
        http_server = http_server.workers(workers.parse().unwrap());
    }

    let result = http_server.bind((host, port))?.run().await;

    // Flush what the last requests buffered before the process exits,
    // also when the server stopped with an error
    let _ = shutdown_tx.send(());
    let _ = flush_task.await;
    result
}
