use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    error::ErrorExt,
    persist::{Json, PersistExt},
    util::Quote,
    Binding, Request, Response, SharedState,
};

mod state;
use self::state::{Command, UserDefinedState};

pub struct UserDefined {
    state: Mutex<UserDefinedState>,
}

impl UserDefined {
    pub async fn create(_: SharedState) -> anyhow::Result<Binding<Self>> {
        // TODO move this to the configuration
        let state = UserDefinedState::load_from_file::<Json>(&"user_defined.json")
            .await
            .unwrap_or_default();

        Binding::create(Self {
            state: Mutex::new(state),
        })
        .bind_this("!add <name> <body..>", "adds a new command", Self::add)?
        .bind_this(
            "!update <name> <body..>",
            "updates an existing command",
            Self::update,
        )?
        .bind_this("!remove <name>", "removes a command", Self::remove)?
        .bind_this("!alias <from> <to>", "aliases a command", Self::alias)?
        .bind_this(
            "!commands",
            "lists all user-defined commands",
            Self::list_commands,
        )?
        .listen_this(Self::lookup)
    }

    async fn add(self: Arc<Self>, req: Request) -> anyhow::Result<Response> {
        req.require_elevation()?;

        let name = Self::validate_command(req.args.get("name")?).map(Quote::Single)?;
        let body = req.args.get("body").map(Quote::Single)?;
        anyhow::ensure!(!body.is_empty(), "the command body cannot be empty");

        let mut state = self.state.lock().await;
        ensure!(
            state.insert(Command::new(name, body, &*req.sender)),
            "{name} already exists"
        );
        Self::sync(&state).await.dont_care()?;

        req.reply(format!("created {name} -> {body}")).ok()
    }

    async fn remove(self: Arc<Self>, req: Request) -> anyhow::Result<Response> {
        req.require_elevation()?;

        let name = Self::validate_command(req.args.get("name")?).map(Quote::Single)?;

        let mut state = self.state.lock().await;
        ensure!(state.remove(&name), "{name} wasn't found");
        Self::sync(&state).await.dont_care()?;

        req.reply(format!("removed: {name}")).ok()
    }

    async fn update(self: Arc<Self>, req: Request) -> anyhow::Result<Response> {
        req.require_elevation()?;

        let name = Self::validate_command(req.args.get("name")?).map(Quote::Single)?;
        let body = req.args.get("body").map(Quote::Single)?;
        anyhow::ensure!(!body.is_empty(), "the command body cannot be empty");

        let mut state = self.state.lock().await;
        ensure!(
            state.update(&name, |cmd| cmd.body = body.inner().to_string()),
            "{name} doesn't exists"
        );
        Self::sync(&state).await.dont_care()?;

        req.reply(format!("updated {name} -> {body}")).ok()
    }

    async fn alias(self: Arc<Self>, req: Request) -> anyhow::Result<Response> {
        req.require_elevation()?;

        let from = Self::validate_command(req.args.get("from")?).map(Quote::Single)?;
        let to = Self::validate_command(req.args.get("to")?).map(Quote::Single)?;

        let mut state = self.state.lock().await;
        ensure!(state.has(&from), "{from} was not found");
        ensure!(state.alias(&from, &to), "{to} already exits");
        Self::sync(&state).await.dont_care()?;

        req.reply(format!("aliased {from} to {to}")).ok()
    }

    async fn lookup(self: Arc<Self>, req: Request) -> anyhow::Result<Response> {
        let data = req.data();
        if let Some(cmd) = data.split_ascii_whitespace().next() {
            let mut state = self.state.lock().await;
            if state.has(cmd) {
                state.update(cmd, |cmd| cmd.uses += 1);
                let cmd = state.get_by_name(cmd).expect("cmd should exist");
                return req.say(&*cmd.body).ok();
            }
        }
        Response::nothing()
    }

    async fn list_commands(self: Arc<Self>, _: Request) -> anyhow::Result<Response> {
        let (mut resp, line) = self
            .state
            .lock()
            .await
            .get_all()
            .map(|c| &*c.name)
            .enumerate()
            .fold(
                (Response::empty(), String::new()),
                |(mut resp, mut out), (i, cmd)| {
                    if i > 0 && i % 20 == 0 {
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
        resp.ok()
    }

    #[cfg(not(test))]
    async fn sync(state: &UserDefinedState) -> anyhow::Result<()> {
        // TODO move this to the configuration
        state.save_to_file::<Json>(&"user_defined.json").await
    }

    #[cfg(test)]
    async fn sync(_: &UserDefinedState) -> anyhow::Result<()> {
        Ok(())
    }

    fn validate_command(name: &str) -> anyhow::Result<&str> {
        anyhow::ensure!(name.starts_with('!'), "you must prefix commands with !");
        anyhow::ensure!(name.len() > 1, "the command name cannot be empty");
        Ok(name)
    }
}

#[cfg(test)]
mod tests;
