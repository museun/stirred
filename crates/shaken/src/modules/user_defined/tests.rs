use super::*;
use crate::testing::{insta_settings, Mock};

#[tokio::test]
async fn add() {
    let _g = insta_settings("add");

    let mut mock = UserDefined::create.mock().await;
    // normal users can't do this
    mock.send_message("!add !foo bar").await;
    insta::assert_yaml_snapshot!("unauthorized", mock.get_response());

    let mut mock = mock.with_broadcaster();

    mock.send_message("!add").await;
    insta::assert_yaml_snapshot!("help", mock.get_response());

    mock.send_message("!add foo bar").await;
    insta::assert_yaml_snapshot!("missing leader", mock.get_response());

    mock.send_message("!add !foo").await;
    insta::assert_yaml_snapshot!("missing body", mock.get_response());

    mock.send_message("!add !foo bar").await;
    insta::assert_yaml_snapshot!("success add", mock.get_response());

    mock.send_message("!add !foo bar").await;
    insta::assert_yaml_snapshot!("duplicate", mock.get_response());
}

#[tokio::test]
async fn update() {
    let _g = insta_settings("update");

    let mut mock = UserDefined::create.mock().await;
    // normal users can't do this
    mock.send_message("!update !foo bar").await;
    insta::assert_yaml_snapshot!("unauthorized", mock.get_response());

    let mut mock = mock.with_broadcaster();
    mock.send_message("!add !foo bar").await;
    mock.get_response();

    mock.send_message("!update").await;
    insta::assert_yaml_snapshot!("help", mock.get_response());

    mock.send_message("!update foo bar").await;
    insta::assert_yaml_snapshot!("missing leader", mock.get_response());

    mock.send_message("!update !foo").await;
    insta::assert_yaml_snapshot!("no body", mock.get_response());

    mock.send_message("!update !foo bar").await;
    insta::assert_yaml_snapshot!("success", mock.get_response());
}

#[tokio::test]
async fn remove() {
    let _g = insta_settings("remove");

    let mut mock = UserDefined::create.mock().await;
    mock.send_message("!remove !foo").await;
    insta::assert_yaml_snapshot!("unauthorized", mock.get_response());

    let mut mock = mock.with_broadcaster();
    mock.send_message("!add !foo bar").await;
    mock.get_response();

    mock.send_message("!remove").await;
    insta::assert_yaml_snapshot!("help", mock.get_response());

    // this one is wrong
    mock.send_message("!remove foo").await;
    insta::assert_yaml_snapshot!("missing leader", mock.get_response());

    mock.send_message("!remove !foo").await;
    insta::assert_yaml_snapshot!("success", mock.get_response());

    mock.send_message("!remove !foo").await;
    insta::assert_yaml_snapshot!("not found", mock.get_response());
}

#[tokio::test]
async fn alias() {
    let _g = insta_settings("alias");

    let mut mock = UserDefined::create.mock().await;
    mock.send_message("!alias !foo !bar").await;
    insta::assert_yaml_snapshot!("unauthorized", mock.get_response());

    let mut mock = mock.with_broadcaster();
    mock.send_message("!add !foo bar").await;
    mock.get_response();

    mock.send_message("!alias !foo !bar").await;
    insta::assert_yaml_snapshot!("alias success", mock.get_response());

    mock.send_message("!alias !baz !bar").await;
    insta::assert_yaml_snapshot!("not found", mock.get_response());

    mock.send_message("!alias !foo !bar").await;
    insta::assert_yaml_snapshot!("already exists", mock.get_response());
}

#[tokio::test]
async fn lookup() {
    let _g = insta_settings("alias");

    let mut mock = UserDefined::create.mock().await.with_broadcaster();

    mock.send_message("!add !foo bar").await;
    mock.get_response();

    mock.send_message("!foo").await;
    insta::assert_yaml_snapshot!("does !foo", mock.get_response());

    mock.send_message("!bar").await;
    insta::assert_yaml_snapshot!("doesnt !bar", mock.get_response());
}
