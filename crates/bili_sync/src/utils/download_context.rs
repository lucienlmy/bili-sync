use sea_orm::DatabaseConnection;

use crate::adapter::VideoSourceEnum;
use crate::bilibili::{BiliClient, FilterOption};
use crate::config::Config;
use crate::config::SkipOption;
use crate::downloader::Downloader;

#[derive(Clone)]
pub struct DownloadContext<'a> {
    pub bili_client: &'a BiliClient,
    pub video_source: &'a VideoSourceEnum,
    pub template: &'a handlebars::Handlebars<'a>,
    pub connection: &'a DatabaseConnection,
    pub downloader: &'a Downloader,
    pub config: &'a Config,
    pub skip_option: Option<SkipOption>,
    pub filter_option: &'a FilterOption,
}

impl<'a> DownloadContext<'a> {
    pub fn new(
        bili_client: &'a BiliClient,
        video_source: &'a VideoSourceEnum,
        template: &'a handlebars::Handlebars<'a>,
        connection: &'a DatabaseConnection,
        downloader: &'a Downloader,
        config: &'a Config,
        filter_option: &'a FilterOption,
    ) -> Self {
        let skip_option = video_source
            .skip_option()
            .and_then(|s| serde_json::from_str(s).ok());

        Self {
            bili_client,
            video_source,
            template,
            connection,
            downloader,
            config,
            skip_option,
            filter_option,
        }
    }

    pub fn get_skip_option(&self) -> &SkipOption {
        self.skip_option.as_ref().unwrap_or(&self.config.skip_option)
    }
}
