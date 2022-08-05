use super::*;
use crate::{
    testing::{insta_settings, Mock, MockServer},
    State,
};
use wiremock::matchers::*;

#[tokio::test]
async fn lookup() {
    let server = MockServer::mock_get(
        |mock| {
            mock.and(query_param("page", "1"))
                .and(query_param("per_page", "1"))
                .and(query_param("q", "twitchchat"))
        },
        &MockServer::read_input_data("crates_io_lookup.json").await,
    )
    .await;

    let _g = insta_settings("crates");
    let mut mock = Crates::create
        .mock_with_state(State::default().with(CratesClient(
            CratesClient::http(),
            Some(server.address().to_string()),
        )))
        .await;

    for msg in ["!crate twitchchat", "!crates twitchchat"] {
        mock.send_message(msg).await;
        insta::assert_yaml_snapshot!(mock.get_response());
    }
}
