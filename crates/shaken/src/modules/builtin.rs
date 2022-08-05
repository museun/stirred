use std::{path::PathBuf, sync::Arc};
use tokio::time::Instant;

use crate::{
    error::ErrorExt, help::HelpRegistry, twitch::HelixClient, Binding, FormatTime, Request,
    Response, SharedState, SystemTime,
};

pub struct Builtin {
    uptime: Instant,
}

impl Builtin {
    pub async fn create(_: SharedState) -> anyhow::Result<Binding<Self>> {
        Ok(Binding::create(Self {
            uptime: Instant::now(),
        })
        .bind("!hello", "sends a greeting", Self::hello)?
        .bind("!time", "gives the current streamer time", Self::time)?
        .bind("!theme", "gets the current VsCode theme", Self::theme)?
        .bind_this(
            "!bot-uptime",
            "gets the uptime for the bot",
            Self::bot_uptime,
        )?
        .bind("!uptime", "gets the uptime for the stream", Self::uptime)?
        .bind(
            "!help <cmd?>",
            "list all commands, or looks up a specific command",
            Self::help,
        )?
        .listen(Self::say_hello))
    }

    async fn bot_uptime(self: Arc<Self>, req: Request) -> anyhow::Result<Response> {
        let uptime = self.uptime.elapsed().as_readable_time();
        req.say(format!("I've been running for: {uptime}")).ok()
    }

    async fn hello(req: Request) -> anyhow::Result<Response> {
        req.say(format!("hello, {}!", req.sender)).ok()
    }

    async fn say_hello(req: Request) -> anyhow::Result<Response> {
        let data = req.data().trim_end_matches(['!', '?', '.']);
        check!(matches!(data, s if s.eq_ignore_ascii_case("hello")));
        req.say(format!("hello, {}.", req.sender)).ok()
    }

    async fn time(req: Request) -> anyhow::Result<Response> {
        let f = time::format_description::parse("[hour]:[minute]:[second]")?;
        let now = time::OffsetDateTime::now_local()?.format(&f)?;
        req.say(format!("current time: {now}")).ok()
    }

    async fn theme(req: Request) -> anyhow::Result<Response> {
        let data = Self::read_settings_json().await.dont_care()?;
        let current = what_theme::get_current_theme_from(&data).dont_care()?;

        let data = Self::read_extension_cache().await.dont_care()?;
        let settings = what_theme::VsCodeSettings::new_from(&data).dont_care()?;

        match settings.find_theme(&current) {
            Some(theme) => req.say(theme.to_string()).ok(),
            None => req.problem("I can't figure that out").ok(),
        }
    }

    async fn uptime(req: Request) -> anyhow::Result<Response> {
        let client = req.state.get::<HelixClient>().await;

        if let Ok(room) = req.streamer_name() {
            if let [stream] = &*client.get_streams([room]).await? {
                let uptime = (SystemTime::now() - stream.started_at).as_readable_time();
                let resp = format!("stream has been running for: {uptime}");
                return req.say(resp).ok();
            };
        }

        req.problem("I don't know").ok()
    }

    const MAX_COMMANDS_PER_LINE: usize = 20;

    async fn help(req: Request) -> anyhow::Result<Response> {
        let help = req.state.get::<HelpRegistry>().await;

        // TODO track where the command came from
        match req.args.get("cmd") {
            Ok(cmd) => match help.lookup(cmd) {
                Some((usage, desc)) => req
                    .say(format!("usage: {usage}"))
                    .say(format!("description: {desc}"))
                    .ok(),
                None => req.problem(format!("I couldn't find {cmd}")).ok(),
            },
            Err(..) => Self::format_all_commands(&help, Self::MAX_COMMANDS_PER_LINE).ok(),
        }
    }

    async fn read_settings_json() -> anyhow::Result<String> {
        #[cfg(test)]
        fn path() -> anyhow::Result<PathBuf> {
            Ok(crate::testing::inputs_dir().join("what_theme_settings.json"))
        }
        #[cfg(not(test))]
        fn path() -> anyhow::Result<PathBuf> {
            Ok(what_theme::VsCodeSettings::settings_json_path()?)
        }

        Ok(tokio::fs::read_to_string(path()?).await?)
    }

    async fn read_extension_cache() -> anyhow::Result<String> {
        #[cfg(test)]
        fn path() -> anyhow::Result<PathBuf> {
            Ok(crate::testing::inputs_dir().join("what_theme_extension_user.json"))
        }
        #[cfg(not(test))]
        fn path() -> anyhow::Result<PathBuf> {
            Ok(what_theme::VsCodeSettings::extension_user_cache_path()?)
        }

        Ok(tokio::fs::read_to_string(path()?).await?)
    }

    fn format_all_commands(help: &HelpRegistry, max: usize) -> Response {
        let (mut resp, line) = help.get_all_commands().enumerate().fold(
            (Response::empty(), String::new()),
            |(mut resp, mut out), (i, cmd)| {
                if i > 0 && i % max == 0 {
                    resp = resp.say(std::mem::take(&mut out))
                }
                if !out.is_empty() {
                    out.push(' ')
                }
                out.push_str(cmd);
                (resp, out)
            },
        );
        if !line.is_empty() {
            resp = resp.say(line)
        }
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{insta_settings, Mock, MockServer, MockTwitch};

    #[tokio::test]
    async fn hello() {
        let _g = insta_settings("hello");
        let mut mock = Builtin::create.mock().await;
        mock.send_message("!hello").await;
        insta::assert_yaml_snapshot!(mock.get_response());
    }

    #[tokio::test]
    async fn say_hello() {
        let _g = insta_settings("say_hello");
        let mut mock = Builtin::create.mock().await;
        for msg in ["hello", "hello.", "hello?", "hello#"] {
            mock.send_message(msg).await;
            insta::assert_yaml_snapshot!(mock.get_response());
        }
    }

    #[tokio::test]
    async fn list_commands() {
        let _g = insta_settings("list_commands");
        let mut mock = Builtin::create.mock().await;
        mock.send_message("!commands").await;
        insta::assert_yaml_snapshot!(mock.get_response());
    }

    #[tokio::test]
    async fn help() {
        let _g = insta_settings("help");
        let mut mock = Builtin::create.mock().await;
        for msg in ["!help", "!help !help", "!help !foo"] {
            mock.send_message(msg).await;
            insta::assert_yaml_snapshot!(mock.get_response());
        }
    }

    #[tokio::test]
    async fn time() {
        let _g = insta_settings("time");
        let mut mock = Builtin::create.mock().await;
        mock.send_message("!time").await;
        insta::with_settings!({filters => vec![
            (r#"\d{2}:\d{2}:\d{2}"#, "[time]")
        ]}, {
            insta::assert_yaml_snapshot!(mock.get_response());
        })
    }

    #[tokio::test]
    async fn theme() {
        let _g = insta_settings("theme");
        let mut mock = Builtin::create.mock().await;
        mock.send_message("!theme").await;
        insta::assert_yaml_snapshot!(mock.get_response());
    }

    #[tokio::test]
    async fn bot_uptime() {
        let _g = insta_settings("bot_uptime");
        tokio::time::pause();
        let mut mock = Builtin::create.mock().await;
        tokio::time::advance(std::time::Duration::from_secs(12345)).await;
        mock.send_message("!bot-uptime").await;
        insta::assert_yaml_snapshot!(mock.get_response());
    }

    #[tokio::test]
    async fn uptime() {
        let server = MockTwitch::start_mock_get_streams(
            &MockServer::read_input_data("twitch_get_stream.json").await,
        )
        .await;

        let _g = insta_settings("uptime");
        tokio::time::pause();

        let state = crate::State::default().with(HelixClient::new_with_ep(
            server.address().to_string(),
            "hunter2",
            "hunter2",
        ));

        let mut mock = Builtin::create.mock_with_state(state).await;
        mock.send_message("!uptime").await;
        insta::assert_yaml_snapshot!(mock.get_response());
    }
}
