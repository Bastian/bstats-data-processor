use crate::helper::test_environment::TestEnvironment;
use data_processor::service::{find_all, find_by_software_url_and_name, Service};

#[tokio::test]
async fn test_find_all() {
    let test_environment = TestEnvironment::with_data().await;

    let mut con = test_environment.redis_multiplexed_connection().await;

    let services: Vec<Service> = find_all(&mut con).await.unwrap();
    // The default test environment has two services
    assert_eq!(services.len(), test_environment.software().len());
    assert_eq!(services[0].name, "_bukkit_");
    assert_eq!(services[0].global, true);

    // In an empty environment, no data should be returned
    let empty_test_environment: TestEnvironment = TestEnvironment::empty().await;
    let mut con = empty_test_environment.redis_multiplexed_connection().await;

    let services: Vec<Service> = find_all(&mut con).await.unwrap();
    assert_eq!(services.len(), 0);
}

#[tokio::test]
async fn test_find_by_software_url_and_name() {
    let test_environment = TestEnvironment::with_data().await;
    let mut con: redis::aio::MultiplexedConnection =
        test_environment.redis_multiplexed_connection().await;

    let service = find_by_software_url_and_name(&mut con, "bukkit", "_bukkit_")
        .await
        .unwrap();
    assert_eq!(service.unwrap().name, "_bukkit_");
}
