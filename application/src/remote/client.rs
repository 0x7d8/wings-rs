use crate::server::{
    activity::ApiActivity, installation::InstallationScript, permissions::Permissions,
};
use axum::http::HeaderMap;

#[repr(C)]
pub struct Client {
    pub(super) config: crate::config::RemoteQuery,

    pub(super) client: reqwest::Client,
    pub(super) url: String,
}

impl Client {
    pub fn new(config: &crate::config::InnerConfig, ignore_certificate_errors: bool) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            "User-Agent",
            format!(
                "Pterodactyl Wings/v{} (id:{})",
                crate::VERSION,
                config.token_id
            )
            .parse()
            .unwrap(),
        );
        headers.insert(
            "Accept",
            "application/vnd.pterodactyl.v1+json".parse().unwrap(),
        );
        headers.insert(
            "Authorization",
            format!("Bearer {}.{}", config.token_id, config.token)
                .parse()
                .unwrap(),
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .danger_accept_invalid_certs(ignore_certificate_errors)
            .default_headers(headers)
            .build()
            .unwrap();

        Self {
            config: config.remote_query.clone(),
            client,
            url: format!("{}/api/remote", config.remote.trim_end_matches('/')),
        }
    }

    #[tracing::instrument(skip(self, password))]
    pub async fn get_sftp_auth(
        &self,
        r#type: super::AuthenticationType,
        username: &str,
        password: &str,
    ) -> Result<(uuid::Uuid, uuid::Uuid, Permissions), reqwest::Error> {
        tracing::debug!("getting sftp auth");
        super::get_sftp_auth(self, r#type, username, password).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn send_activity(&self, activity: Vec<ApiActivity>) -> Result<(), reqwest::Error> {
        tracing::debug!("sending {} activity to remote", activity.len());
        super::send_activity(self, activity).await
    }

    pub async fn servers(&self) -> Result<Vec<super::servers::RawServer>, reqwest::Error> {
        tracing::info!("fetching all servers from remote");

        let mut servers = Vec::new();

        let mut page = 1;
        loop {
            tracing::info!("fetching page {} of servers", page);
            let (new_servers, pagination) = super::servers::get_servers_paged(self, page).await?;
            servers.extend(new_servers);

            if pagination.current_page >= pagination.last_page {
                break;
            }

            page += 1;
        }

        tracing::info!("fetched {} servers from remote", servers.len());

        Ok(servers)
    }

    pub async fn server(
        &self,
        uuid: uuid::Uuid,
    ) -> Result<super::servers::RawServer, reqwest::Error> {
        super::servers::get_server(self, uuid).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn server_install_script(
        &self,
        uuid: uuid::Uuid,
    ) -> Result<InstallationScript, reqwest::Error> {
        tracing::info!("fetching server install script");
        super::servers::get_server_install_script(self, uuid).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_server_install(
        &self,
        uuid: uuid::Uuid,
        successful: bool,
        reinstalled: bool,
    ) -> Result<(), reqwest::Error> {
        tracing::info!("setting server install status");
        super::servers::set_server_install(self, uuid, successful, reinstalled).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_server_transfer(
        &self,
        uuid: uuid::Uuid,
        successful: bool,
    ) -> Result<(), reqwest::Error> {
        tracing::info!("setting server transfer status");
        super::servers::set_server_transfer(self, uuid, successful).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_backup_status(
        &self,
        uuid: uuid::Uuid,
        data: &super::backups::RawServerBackup,
    ) -> Result<(), reqwest::Error> {
        tracing::info!("setting backup status");
        super::backups::set_backup_status(self, uuid, data).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_backup_restore_status(
        &self,
        uuid: uuid::Uuid,
        successful: bool,
    ) -> Result<(), reqwest::Error> {
        tracing::info!("setting backup restore status");
        super::backups::set_backup_restore_status(self, uuid, successful).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn backup_upload_urls(
        &self,
        uuid: uuid::Uuid,
        size: u64,
    ) -> Result<(u64, Vec<String>), reqwest::Error> {
        tracing::info!("getting backup upload urls");
        super::backups::backup_upload_urls(self, uuid, size).await
    }
}
