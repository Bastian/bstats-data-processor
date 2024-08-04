use crate::helper::test_environment::TestEnvironment;
use data_processor::software::find_all;

#[tokio::test]
async fn test_find_all() {
    let test_environment = TestEnvironment::with_data().await;

    let mut con = test_environment.redis_multiplexed_connection().await;

    let software: Vec<data_processor::software::Software> = find_all(&mut con).await.unwrap();
    assert_eq!(software.len(), test_environment.software().len());
    assert_eq!(software[0].name, "Bukkit / Spigot");
    assert_eq!(
        software[0].default_charts[0].title,
        "Servers using %plugin.name%"
    );
    assert_eq!(software[0].hide_in_plugin_list, false);

    // In an empty environment, no data should be returned
    let empty_test_environment = TestEnvironment::empty().await;
    let mut con = empty_test_environment.redis_multiplexed_connection().await;

    let software: Vec<data_processor::software::Software> = find_all(&mut con).await.unwrap();
    assert_eq!(software.len(), 0);
}

#[tokio::test]
async fn test_find_by_url() {
    let test_environment = TestEnvironment::with_data().await;
    let mut con = test_environment.redis_multiplexed_connection().await;

    let software = data_processor::software::find_by_url(&mut con, "bukkit")
        .await
        .unwrap();
    assert_eq!(software.unwrap().name, "Bukkit / Spigot");
}
