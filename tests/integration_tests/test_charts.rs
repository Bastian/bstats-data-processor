use crate::helper::test_environment::TestEnvironment;
use data_processor::charts;

#[tokio::test]
async fn test_find_by_id() {
    let test_environment = TestEnvironment::with_data().await;
    let mut con = test_environment.redis_connection().await;

    let chart = charts::find_by_id(&mut con, 1).await;
    assert_eq!(chart.unwrap().unwrap().id_custom, "servers");
}

#[tokio::test]
async fn test_find_by_ids() {
    let test_environment = TestEnvironment::with_data().await;
    let mut con = test_environment.redis_connection().await;

    let charts: std::collections::HashMap<u64, Option<charts::Chart>> =
        charts::find_by_ids(&mut con, vec![1, 2]).await.unwrap();

    assert_eq!(
        charts.get(&1).unwrap().as_ref().unwrap().id_custom.clone(),
        "servers"
    );

    assert_eq!(
        charts.get(&2).unwrap().as_ref().unwrap().id_custom.clone(),
        "players"
    );
}
