use data_processor::software::find_all;
use test_environment::TestEnvironment;

mod bound_redis_client;
mod test_environment;

#[tokio::test]
async fn test_find_all() {
    let test_environment = TestEnvironment::with_data().await;

    let mut con = test_environment.redis_multiplexed_connection().await;

    let software: Vec<data_processor::software::Software> = find_all(&mut con).await.unwrap();
    // The default test environment has two software entries
    assert_eq!(software.len(), test_environment.software().len());

    // In an empty environment, no data should be returned
    let empty_test_environment = TestEnvironment::empty().await;
    let mut con = empty_test_environment.redis_multiplexed_connection().await;

    let software: Vec<data_processor::software::Software> = find_all(&mut con).await.unwrap();
    assert_eq!(software.len(), 0);
}
