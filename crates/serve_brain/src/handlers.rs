use std::path::PathBuf;

use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, Json,
};
use markov::Brain;

use crate::{messaging, request, response, spawn_brain, state::State, Messaging};

pub async fn generate(
    Path(name): Path<String>,
    req: Option<Json<request::Generate>>,
    state: Extension<State>,
) -> Response {
    let brain = match state.try_get(&name).await {
        Some(brain) => brain,
        None => return make_error(404, format!("cannot find {name}")),
    };

    let opts = req.map(|Json(data)| data).unwrap_or_default();
    let resp = brain.send(messaging::Request::Generate { opts }).await;
    match resp {
        messaging::Response::Generated { data } => json(response::Generate { data }),
        messaging::Response::Error { error } => make_error(503, error),
        _ => StatusCode::OK.into_response(),
    }
}

pub async fn train(
    Path(name): Path<String>,
    Json(request::Train { data }): Json<request::Train>,
    state: Extension<State>,
) -> impl IntoResponse {
    use messaging::{Request::*, Response::*};

    let brain = match state.try_get(&name).await {
        Some(brain) => brain,
        None => return make_error(404, format!("cannot find {name}")),
    };

    if let Error { error } = brain.send(Train { data }).await {
        return make_error(401, error);
    }

    if let Error { error } = brain.send(Save).await {
        return make_error(503, error);
    }

    StatusCode::OK.into_response()
}

// TODO 'scope' the path to the configured directory
pub async fn create(
    Path(name): Path<String>,
    Json(request::Create { path, depth }): Json<request::Create>,
    state: Extension<State>,
) -> impl IntoResponse {
    use messaging::{Request::*, Response::*};

    if let Some(..) = state.try_get(&name).await {
        return make_error(401, format!("{name} already exists"));
    };

    let brain = Brain::new(name.clone(), depth.unwrap());
    let out = spawn_brain(brain, path.clone());
    state.brains.lock().await.insert(
        PathBuf::from(path)
            .file_stem()
            .expect("valid path") // TODO not this
            .to_string_lossy()
            .to_string(),
        Messaging::new(out),
    );

    let brain = state.try_get(&name).await.unwrap();
    match brain.send(ForceSave).await {
        Error { error } => make_error(503, format!("cannot save {name}: {error}")),
        _ => StatusCode::OK.into_response(),
    }
}

fn json<T: serde::Serialize + 'static + Send>(data: T) -> Response {
    Json(data).into_response()
}

fn make_error(code: u16, err: impl ToString + Send) -> Response {
    let status_code = StatusCode::from_u16(code).expect("valid status code");
    let json = json(response::Error {
        msg: err.to_string(),
    });
    (status_code, json).into_response()
}
