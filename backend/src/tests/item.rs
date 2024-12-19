use std::sync::Arc;

use axum::http::StatusCode;
use lambda_http::{tower::ServiceExt, Error};
use ulid::Ulid;

use crate::{
    create_service,
    models::item::Item,
    state::AppState,
    tests::{
        build_request, parse_resp,
        seller::{add_test_item, clean_item, test_seller_login},
    },
};

#[tokio::test]
async fn test_get_item() -> Result<(), Error> {
    let state = Arc::new(AppState::new().await?);

    let user_info = test_seller_login(state.clone()).await?;

    let name = format!("TestItme_{}", Ulid::new());

    let item_ref = add_test_item(state.clone(), &user_info, &name)
        .await
        .unwrap();
    let service = create_service(state.clone()).await?;

    let uri = format!("/v1/item/{}/{}", &user_info.id, item_ref.id.to_string());
    let req = build_request::<()>("GET", &uri, &user_info.token, None)?;
    let resp = service.oneshot(req).await?;

    assert_eq!(resp.status(), StatusCode::OK);

    let item: Item = parse_resp(resp).await?;

    assert_eq!(&item.name, &name);
    assert_eq!(item.id, item_ref.id);

    clean_item(state, user_info.id, item_ref.id).await?;
    Ok(())
}
