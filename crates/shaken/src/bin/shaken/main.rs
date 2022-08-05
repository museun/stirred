use std::sync::Arc;

use shaken::{
    config,
    error::DontCare,
    help::HelpRegistry,
    irc,
    modules::{AnotherViewer, Builtin, Crates, Spotify, UserDefined},
    twitch::{data::EmoteMap, HelixClient, OAuth},
    Arguments, Callable, Request, ResponseKind, SharedState, State,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    simple_env_load::load_env_from([".dev.env", ".env"]);
    alto_logger::TermLogger::new(
        alto_logger::Options::default()
            .with_time(alto_logger::TimeConfig::relative_now())
            .with_style(alto_logger::StyleConfig::MultiLine),
    )?
    .init()?;

    let config = config::Config::load_from_env()?;

    let twitch_oauth = {
        let config::Twitch {
            client_id,
            client_secret,
        } = &config.twitch;
        log::trace!("getting twitch oauth tokens");
        OAuth::create(client_id, client_secret).await?
    };

    let twitch_client = HelixClient::new(
        twitch_oauth.get_client_id(), //
        twitch_oauth.get_bearer_token(),
    );

    log::trace!("getting the twitch global emotes");
    let (_, global) = twitch_client.get_global_emotes().await?;
    let iter = global.iter().map(|c| (&*c.id, &*c.name));
    let emote_map = EmoteMap::default().with_emotes(iter);

    // TODO reconnect if forcibly closed because twitch uses a sharded irc network
    log::info!("connecting to twitch irc");
    let (identity, mut conn) = irc::connect(
        irc::TWITCH_NO_TLS,
        irc::Registration {
            name: &config.irc.name,
            pass: &config.irc.pass,
        },
    )
    .await?;
    log::info!("connected");

    let mut state = State::default();
    state.insert(identity);
    state.insert(config);
    state.insert(twitch_oauth);
    state.insert(twitch_client);
    state.insert(emote_map);

    let state = SharedState::new(state);

    log::debug!("creating handlers");
    let handlers = [
        Builtin::create(state.clone()).await?.into_callable(), //
        Crates::create(state.clone()).await?.into_callable(),
        Spotify::create(state.clone()).await?.into_callable(),
        AnotherViewer::create(state.clone()).await?.into_callable(),
        UserDefined::create(state.clone()).await?.into_callable(),
    ];
    log::debug!("created handlers");

    state.insert(HelpRegistry::create_from(&handlers)).await;

    log::info!("joining #museun");
    conn.join_channel("#museun").await?;
    while let Ok(pm) = conn.read_message().await {
        log::debug!("<- {pm}");
        let req = Request::from_pm(state.clone(), pm);

        log::trace!("calling handlers");
        let resp = match handlers.call(req.clone()).await {
            Ok(resp) => resp,
            Err(err) if err.is::<DontCare>() => continue,
            Err(err) => {
                log::warn!("a handler returned an error: {err}");
                continue;
            }
        };

        for resp in resp.kind {
            handle_response(&mut conn, resp, &req).await?
        }

        log::debug!("waiting for next message");
    }

    log::warn!("end of main loop");
    Ok(())
}

async fn handle_response(
    conn: &mut irc::Conn,
    resp: ResponseKind,
    req: &Request,
) -> anyhow::Result<()> {
    use ResponseKind::*;
    let out = match resp {
        Say(data) => data,
        Reply(data) => format!("{}: {}", &req.sender, &data).into_boxed_str(),
        Problem(data) => format!("{}: {}", &req.sender, &data).into_boxed_str(),
    };
    conn.privmsg(&req.target, &out).await
}
