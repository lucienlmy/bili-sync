use core::str;
use std::io::SeekFrom;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, bail, ensure};
use async_tempfile::TempFile;
use futures::TryStreamExt;
use futures::StreamExt;
use reqwest::{Method, StatusCode, header};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::fs::{self};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::task::JoinSet;
use tokio_util::io::StreamReader;

use crate::bilibili::{Client, ErrorForStatusExt};
use crate::config::{ARGS, ConcurrentDownloadLimit};

pub struct Downloader {
    client: Client,
}

impl Downloader {
    // Downloader 使用带有默认 Header 的 Client 构建
    // 拿到 url 后下载文件不需要任何 cookie 作为身份凭证
    // 但如果不设置默认 Header，下载时会遇到 403 Forbidden 错误
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn fetch(&self, url: &str, path: &Path, concurrent_download: &ConcurrentDownloadLimit, kind: Option<&str>) -> Result<()> {
        let mut temp_file = TempFile::new().await?;
        self.fetch_internal(url, &mut temp_file, false, concurrent_download, Some(path), kind)
            .await?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let existed = fs::metadata(path).await.is_ok();
        fs::copy(temp_file.file_path(), path).await?;
        info!("【文件层】保存文件：{} ({})", path.display(), if existed { "overwrite" } else { "create" });
        // temp_file 的 drop 需要 std::fs::remove_file
        // 如果交由 rust 自动执行虽然逻辑正确但会略微阻塞异步上下文
        // 尽量主动调用，保证正常执行的情况下文件清除操作由 spawn_blocking 在专门线程中完成
        temp_file.drop_async().await;
        Ok(())
    }

    pub async fn multi_fetch(
        &self,
        urls: &[&str],
        path: &Path,
        concurrent_download: &ConcurrentDownloadLimit,
        kind: Option<&str>,
    ) -> Result<()> {
        let temp_file = self.multi_fetch_internal(urls, Some(path), true, concurrent_download, kind).await?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let existed = fs::metadata(path).await.is_ok();
        fs::copy(temp_file.file_path(), path).await?;
        info!("【文件层】保存文件：{} ({})", path.display(), if existed { "overwrite" } else { "create" });
        temp_file.drop_async().await;
        Ok(())
    }

    pub async fn multi_fetch_and_merge(
        &self,
        video_urls: &[&str],
        audio_urls: &[&str],
        path: &Path,
        concurrent_download: &ConcurrentDownloadLimit,
        kind: Option<&str>,
    ) -> Result<()> {
        let (video_temp_file, audio_temp_file) = tokio::try_join!(
            self.multi_fetch_internal(video_urls, Some(path), true, concurrent_download, kind),
            self.multi_fetch_internal(audio_urls, Some(path), true, concurrent_download, kind)
        )?;
        let final_temp_file = TempFile::new().await?;
        let output = Command::new(ARGS.ffmpeg_path.as_deref().unwrap_or("ffmpeg"))
            .args([
                "-i",
                video_temp_file.file_path().to_string_lossy().as_ref(),
                "-i",
                audio_temp_file.file_path().to_string_lossy().as_ref(),
                "-c",
                "copy",
                "-strict",
                "unofficial",
                "-f",
                "mp4",
                "-y",
                final_temp_file.file_path().to_string_lossy().as_ref(),
            ])
            .output()
            .await
            .context("failed to run ffmpeg")?;
        if !output.status.success() {
            bail!("ffmpeg error: {}", str::from_utf8(&output.stderr).unwrap_or("unknown"));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let existed = fs::metadata(path).await.is_ok();
        fs::copy(final_temp_file.file_path(), path).await?;
        info!("【文件层】保存文件：{} ({})", path.display(), if existed { "overwrite" } else { "create" });
        tokio::join!(
            video_temp_file.drop_async(),
            audio_temp_file.drop_async(),
            final_temp_file.drop_async()
        );
        Ok(())
    }

    async fn multi_fetch_internal(
        &self,
        urls: &[&str],
        target: Option<&Path>,
        is_stream: bool,
        concurrent_download: &ConcurrentDownloadLimit,
        kind: Option<&str>,
    ) -> Result<TempFile> {
        if urls.is_empty() {
            bail!("no urls provided");
        }
        let mut temp_file = TempFile::new().await?;
        for (idx, url) in urls.iter().enumerate() {
            match self
                .fetch_internal(url, &mut temp_file, is_stream, concurrent_download, target, kind)
                .await
            {
                Ok(_) => return Ok(temp_file),
                Err(e) => {
                    if idx == urls.len() - 1 {
                        temp_file.drop_async().await;
                        return Err(e).with_context(|| format!("failed to download file from all {} urls", urls.len()));
                    }
                    temp_file.set_len(0).await?;
                    temp_file.rewind().await?;
                }
            }
        }
        unreachable!()
    }

    async fn fetch_internal(
        &self,
        url: &str,
        file: &mut TempFile,
        is_stream: bool,
        concurrent_download: &ConcurrentDownloadLimit,
        target: Option<&Path>,
        kind: Option<&str>,
    ) -> Result<()> {
        if concurrent_download.enable {
            self.fetch_parallel(url, file, is_stream, concurrent_download, target, kind).await
        } else {
            self.fetch_serial(url, file, target, kind).await
        }
    }

    async fn fetch_serial(&self, url: &str, file: &mut TempFile, target: Option<&Path>, kind: Option<&str>) -> Result<()> {
        let resp = self
            .client
            .request(Method::GET, url, None)
            .send()
            .await?
            .error_for_status_ext()?;
        let expected = resp.header_content_length();
        let mut stream = resp.bytes_stream();
        let mut received: u64 = 0;
        // progress tracking
        let total = expected;
        let downloaded = Arc::new(AtomicU64::new(0));
        let last_percent = Arc::new(AtomicU64::new(0));
        // 0: none, 1: start logged, 2: 50% logged, 3: finished logged
        let last_phase = Arc::new(AtomicU64::new(0));
        // friendly id: prefer target file name, fallback to temp file name
        let id_display = target
            .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
            .or_else(|| file.file_path().file_name().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_else(|| file.file_path().to_string_lossy().into_owned());
        // file type: prefer explicit kind, fallback to target/temp extension
        let file_type = kind
            .map(|s| s.to_string())
            .or_else(|| {
                target
                    .and_then(|p| p.extension().map(|s| s.to_string_lossy().into_owned()))
                    .or_else(|| file.file_path().extension().map(|s| s.to_string_lossy().into_owned()))
            })
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| "UNKNOWN".to_string());
        // full path, but simplify LOCALAPPDATA on Windows to keep logs concise
        let mut full_path_display = target
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| file.file_path().to_string_lossy().into_owned());
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            if full_path_display.starts_with(&local) {
                full_path_display = full_path_display.replacen(&local, "<winLocalAppData>", 1);
            }
        }
        if let Some(total_size) = total {
            if last_phase.compare_exchange(0, 1, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
                info!("【文件层进度】file=\"{}\" done=0/{} (0%) path=\"{}\"", id_display, total_size, full_path_display);
            }
        }

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(std::io::Error::other)?;
            file.write_all(&chunk).await?;
            received += chunk.len() as u64;
            downloaded.fetch_add(chunk.len() as u64, Ordering::Relaxed);
            if let Some(total_size) = total {
                let percent = downloaded.load(Ordering::Relaxed) * 100 / total_size;
                let old = last_percent.load(Ordering::Relaxed);
                if percent > old {
                    if last_percent.compare_exchange(old, percent, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
                        // Only log three times: start(0%), 50% (first time >=50), finish(100%)
                        if percent == 100 {
                            if last_phase.compare_exchange(0, 3, Ordering::Relaxed, Ordering::Relaxed).is_ok() || last_phase.compare_exchange(1, 3, Ordering::Relaxed, Ordering::Relaxed).is_ok() || last_phase.compare_exchange(2, 3, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
                                info!("【文件层进度】type=\"{}\" file=\"{}\" done={}/{} ({}%) path=\"{}\"", file_type, id_display, downloaded.load(Ordering::Relaxed), total_size, percent, full_path_display);
                            }
                        } else if percent >= 50 {
                            if last_phase.compare_exchange(1, 2, Ordering::Relaxed, Ordering::Relaxed).is_ok() || last_phase.compare_exchange(0, 2, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
                                info!("【文件层进度】type=\"{}\" file=\"{}\" done={}/{} ({}%) path=\"{}\"", file_type, id_display, downloaded.load(Ordering::Relaxed), total_size, percent, full_path_display);
                            }
                        }
                    }
                }
            }
        }
        file.flush().await?;
        if let Some(expected) = expected {
            ensure!(
                received == expected,
                "downloaded bytes mismatch: expected {}, got {}",
                expected,
                received
            );
        }
        Ok(())
    }

    async fn fetch_parallel(
        &self,
        url: &str,
        file: &mut TempFile,
        is_stream: bool,
        concurrent_download: &ConcurrentDownloadLimit,
        target: Option<&Path>,
        kind: Option<&str>,
    ) -> Result<()> {
        let (concurrency, threshold) = (concurrent_download.concurrency, concurrent_download.threshold);
        let file_size = if is_stream {
            // B 站视频、音频流存在 HEAD 为 404 但 GET 正常的情况，此处假设支持分块，直接使用携带 Range 头的 GET 请求探测
            let resp = self
                .client
                .request(Method::GET, url, None)
                .header(header::RANGE, "bytes=0-0")
                .send()
                .await?
                .error_for_status_ext()?;
            if resp.status() != StatusCode::PARTIAL_CONTENT {
                return self.fetch_serial(url, file, target, kind).await;
            }
            resp.header_file_size()
        } else {
            // 对于普通文件，直接使用常规的 HEAD 请求探测
            let resp = self
                .client
                .request(Method::HEAD, url, None)
                .send()
                .await?
                .error_for_status_ext()?;
            if resp
                .headers()
                .get(header::ACCEPT_RANGES)
                // https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Accept-Ranges#none
                .is_none_or(|v| v.to_str().unwrap_or_default() == "none")
            {
                return self.fetch_serial(url, file, target, kind).await;
            }
            resp.header_content_length()
        };
        let Some(file_size) = file_size else {
            return self.fetch_serial(url, file, target, kind).await;
        };
        let chunk_size = file_size / concurrency as u64;
        if chunk_size < threshold {
            return self.fetch_serial(url, file, target, kind).await;
        }
        file.set_len(file_size).await?;
        let mut tasks = JoinSet::new();
        let url = Arc::new(url.to_string());
        // shared counters for progress reporting
        let downloaded = Arc::new(AtomicU64::new(0));
        let last_percent = Arc::new(AtomicU64::new(0));
        // 0: none, 1: start logged, 2: 50% logged, 3: finished logged
        let last_phase = Arc::new(AtomicU64::new(0));
        for i in 0..concurrency {
            let start = i as u64 * chunk_size;
            let end = if i == concurrency - 1 {
                file_size
            } else {
                start + chunk_size
            } - 1;
            let (url_clone, client_clone) = (url.clone(), self.client.clone());
            let mut file_clone = file.open_rw().await?;
            let downloaded_cl = downloaded.clone();
            let last_percent_cl = last_percent.clone();
            let last_phase_cl = last_phase.clone();
            let file_path_buf = file_clone.file_path().to_path_buf();
                let target_cl = target.map(|p| p.to_path_buf());
                let kind_cl = kind.map(|s| s.to_string());
            tasks.spawn(async move {
                file_clone.seek(SeekFrom::Start(start)).await?;
                let range_header = format!("bytes={}-{}", start, end);
                let resp = client_clone
                    .request(Method::GET, &url_clone, None)
                    .header(header::RANGE, &range_header)
                    .send()
                    .await?
                    .error_for_status_ext()?;
                if let Some(content_length) = resp.header_content_length() {
                    ensure!(
                        content_length == end - start + 1,
                        "content length mismatch: expected {}, got {}",
                        end - start + 1,
                        content_length
                    );
                }
                let mut stream = resp.bytes_stream();
                let mut part_received: u64 = 0;
                // friendly id for logs: prefer target when available
                let id_display = target_cl
                    .as_ref()
                    .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
                    .or_else(|| file_path_buf.file_name().map(|s| s.to_string_lossy().into_owned()))
                    .unwrap_or_else(|| file_path_buf.to_string_lossy().into_owned());
                // file type: prefer explicit kind, fallback to target/temp extension
                let file_type = kind_cl
                    .as_deref()
                    .map(|s| s.to_string())
                    .or_else(|| {
                        target_cl
                            .as_ref()
                            .and_then(|p| p.extension().map(|s| s.to_string_lossy().into_owned()))
                            .or_else(|| file_path_buf.extension().map(|s| s.to_string_lossy().into_owned()))
                    })
                    .map(|s| s.to_uppercase())
                    .unwrap_or_else(|| "UNKNOWN".to_string());
                // full path, simplify LOCALAPPDATA on Windows
                let mut full_path_display = target_cl
                    .as_ref()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|| file_path_buf.to_string_lossy().into_owned());
                if let Ok(local) = std::env::var("LOCALAPPDATA") {
                    if full_path_display.starts_with(&local) {
                        full_path_display = full_path_display.replacen(&local, "<winLocalAppData>", 1);
                    }
                }
                while let Some(chunk) = stream.next().await {
                    let chunk = chunk.map_err(std::io::Error::other)?;
                    file_clone.write_all(&chunk).await?;
                    part_received += chunk.len() as u64;
                    // update global counter and maybe log progress
                    downloaded_cl.fetch_add(chunk.len() as u64, Ordering::Relaxed);
                    let percent = downloaded_cl.load(Ordering::Relaxed) * 100 / file_size;
                    let old = last_percent_cl.load(Ordering::Relaxed);
                    if percent > old && last_percent_cl.compare_exchange(old, percent, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
                        // Only log three times: start(0%), 50% (first time >=50), finish(100%)
                        if percent == 100 {
                            if last_phase_cl.compare_exchange(0, 3, Ordering::Relaxed, Ordering::Relaxed).is_ok() || last_phase_cl.compare_exchange(1, 3, Ordering::Relaxed, Ordering::Relaxed).is_ok() || last_phase_cl.compare_exchange(2, 3, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
                                info!("【文件层进度】type=\"{}\" file=\"{}\" done={}/{} ({}%) path=\"{}\"", file_type, id_display, downloaded_cl.load(Ordering::Relaxed), file_size, percent, full_path_display);
                            }
                        } else if percent >= 50 {
                            if last_phase_cl.compare_exchange(1, 2, Ordering::Relaxed, Ordering::Relaxed).is_ok() || last_phase_cl.compare_exchange(0, 2, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
                                info!("【文件层进度】type=\"{}\" file=\"{}\" done={}/{} ({}%) path=\"{}\"", file_type, id_display, downloaded_cl.load(Ordering::Relaxed), file_size, percent, full_path_display);
                            }
                        }
                    }
                }
                file_clone.flush().await?;
                ensure!(
                    part_received == end - start + 1,
                    "downloaded bytes mismatch: expected {}, got {}",
                    end - start + 1,
                    part_received,
                );
                Ok(())
            });
        }
        while let Some(res) = tasks.join_next().await {
            res??;
        }
        Ok(())
    }
}

/// reqwest.content_length() 居然指的是 body_size 而非 content-length header，没办法自己实现一下
/// https://github.com/seanmonstar/reqwest/issues/1814
trait ResponseExt {
    /// 获取 Content-Length 头的值
    fn header_content_length(&self) -> Option<u64>;
    /// 获取 Content-Range 头中的文件总大小部分
    fn header_file_size(&self) -> Option<u64>;
}

impl ResponseExt for reqwest::Response {
    fn header_content_length(&self) -> Option<u64> {
        self.headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
    }

    fn header_file_size(&self) -> Option<u64> {
        self.headers()
            .get(header::CONTENT_RANGE)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| {
                // Content-Range: bytes 0-0/800946
                s.rsplit_once('/')
            })
            .and_then(|(_, size_str)| size_str.parse::<u64>().ok())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anyhow::Result;

    use crate::bilibili::{BestStream, BiliClient, Video};
    use crate::config::VersionedConfig;
    use crate::database::setup_database;
    use crate::downloader::Downloader;

    #[ignore = "only for manual test"]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_parse_and_download_video() -> Result<()> {
        VersionedConfig::init_for_test(&setup_database(Path::new("./test.sqlite")).await?).await?;
        let config = VersionedConfig::get().read();
        let client = BiliClient::new();
        let video = Video::new(&client, "BV1QJmaYKEv4", &config.credential);
        let pages = video.get_pages().await.expect("failed to get pages");
        let first_page = pages.into_iter().next().expect("no page found");
        let mut page_analyzer = video
            .get_page_analyzer(&first_page)
            .await
            .expect("failed to get page analyzer");
        let json_info = serde_json::to_string_pretty(&page_analyzer.info)?;
        tokio::fs::write("./debug_playurl.json", json_info).await?;
        let best_stream = page_analyzer
            .best_stream(&config.filter_option)
            .expect("failed to get best stream");
        let BestStream::VideoAudio {
            video,
            audio: Some(audio),
        } = best_stream
        else {
            panic!("best stream is not video & audio");
        };
        dbg!(&video);
        dbg!(&audio);
        let downloader = Downloader::new(client.client);
        downloader
            .multi_fetch_and_merge(
                &video.urls(true),
                &audio.urls(true),
                Path::new("./output.mp4"),
                &config.concurrent_limit.download,
            )
            .await
            .expect("failed to download video");
        Ok(())
    }
}
