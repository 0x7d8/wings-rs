use super::State;
use utoipa_axum::{router::OpenApiRouter, routes};

mod get {
    use crate::routes::{ApiError, api::servers::_server_::GetServer};
    use axum::{extract::Query, http::StatusCode};
    use serde::Deserialize;
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct Params {
        #[serde(default)]
        pub directory: String,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = Vec<crate::models::DirectoryEntry>),
        (status = NOT_FOUND, body = inline(ApiError)),
        (status = EXPECTATION_FAILED, body = inline(ApiError)),
    ), params(
        (
            "directory" = String, Query,
            description = "The directory to list files from",
        ),
    ))]
    pub async fn route(
        server: GetServer,
        Query(data): Query<Params>,
    ) -> (StatusCode, axum::Json<serde_json::Value>) {
        let mut entries = Vec::new();

        let path = match server.filesystem.safe_path(&data.directory).await {
            Some(path) => path,
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    axum::Json(ApiError::new("path not found").to_json()),
                );
            }
        };

        let metadata = tokio::fs::symlink_metadata(&path).await;
        if let Ok(metadata) = metadata {
            if !metadata.is_dir() || server.filesystem.is_ignored(&path, metadata.is_dir()).await {
                return (
                    StatusCode::EXPECTATION_FAILED,
                    axum::Json(ApiError::new("path not a directory").to_json()),
                );
            }
        } else {
            return (
                StatusCode::NOT_FOUND,
                axum::Json(ApiError::new("path not found").to_json()),
            );
        }

        let mut directory = tokio::fs::read_dir(path).await.unwrap();
        while let Ok(Some(entry)) = directory.next_entry().await {
            let path = entry.path();
            let metadata = match entry.metadata().await {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };

            if server.filesystem.is_ignored(&path, metadata.is_dir()).await {
                continue;
            }

            entries.push(server.filesystem.to_api_entry(path, metadata).await);
        }

        entries.sort_by(|a, b| {
            if a.directory && !b.directory {
                std::cmp::Ordering::Less
            } else if !a.directory && b.directory {
                std::cmp::Ordering::Greater
            } else {
                a.name.cmp(&b.name)
            }
        });

        (
            StatusCode::OK,
            axum::Json(serde_json::to_value(&entries).unwrap()),
        )
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .with_state(state.clone())
}
