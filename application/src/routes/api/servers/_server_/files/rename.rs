use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod put {
    use crate::routes::{ApiError, api::servers::_server_::GetServer};
    use axum::http::StatusCode;
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct RenameFile {
        pub to: String,
        pub from: String,
    }

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        #[serde(default)]
        pub root: String,

        #[schema(inline)]
        pub files: Vec<RenameFile>,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        renamed: usize,
    }

    #[utoipa::path(put, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = NOT_FOUND, body = inline(ApiError)),
    ), request_body = inline(Payload))]
    pub async fn route(
        server: GetServer,
        axum::Json(data): axum::Json<Payload>,
    ) -> (StatusCode, axum::Json<serde_json::Value>) {
        let root = match server.filesystem.safe_path(&data.root).await {
            Some(path) => path,
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    axum::Json(ApiError::new("root not found").to_json()),
                );
            }
        };

        let metadata = root.symlink_metadata();
        if !metadata.map(|m| m.is_dir()).unwrap_or(true) {
            return (
                StatusCode::EXPECTATION_FAILED,
                axum::Json(ApiError::new("root is not a directory").to_json()),
            );
        }

        let mut renamed_count = 0;
        for file in data.files {
            let from = crate::server::filesystem::Filesystem::resolve_path(&root.join(file.from));
            if !server.filesystem.is_safe_path(&from).await || from == root {
                continue;
            }
            let to = crate::server::filesystem::Filesystem::resolve_path(&root.join(file.to));
            if !server.filesystem.is_safe_path(&to).await || to == root {
                continue;
            }

            if from == to {
                continue;
            }

            let from_metadata = match tokio::fs::symlink_metadata(&from).await {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };

            if to.exists()
                || server
                    .filesystem
                    .is_ignored(&from, from_metadata.is_dir())
                    .await
                || server
                    .filesystem
                    .is_ignored(&to, from_metadata.is_dir())
                    .await
            {
                continue;
            }

            if server.filesystem.rename_path(&from, &to).await.is_ok() {
                renamed_count += 1;
            }
        }

        (
            StatusCode::OK,
            axum::Json(
                serde_json::to_value(&Response {
                    renamed: renamed_count,
                })
                .unwrap(),
            ),
        )
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(put::route))
        .with_state(state.clone())
}
