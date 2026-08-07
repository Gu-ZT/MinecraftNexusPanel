use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use nexus_domain::DownloadManifest;
use reqwest::Client;
use reqwest::redirect::Policy;
use rustls::crypto::ring;
use sha2::Digest;
use sha2::Sha256;
use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::select;
use url::Url;

use crate::DownloadError;
use crate::DownloadTask;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);

/// 负责 HTTPS 下载、大小/摘要校验和本地产物缓存的管理器。
///
/// 下载地址拒绝凭据和非 HTTPS 方案；缓存只有在同时匹配清单大小和 SHA-256
/// 时才会复用，失败或取消会清理临时文件。
#[derive(Clone)]
pub struct DownloadManager {
    cache_directory: PathBuf,
    client: Client,
}

impl DownloadManager {
    /// 创建下载管理器并初始化下载缓存目录配置。
    pub fn new(data_directory: &Path) -> Result<Self, DownloadError> {
        let _ = ring::default_provider().install_default();
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(DOWNLOAD_TIMEOUT)
            .redirect(Policy::none())
            .build()
            .map_err(DownloadError::Client)?;

        Ok(Self {
            cache_directory: data_directory.join("downloads"),
            client,
        })
    }

    /// 下载并校验清单指定的产物，或返回已验证的缓存路径。
    ///
    /// 返回路径位于 Core 数据目录的下载缓存中；任务取消、平台不匹配、大小或
    /// 摘要校验失败时不会留下可被复用的部分文件。
    pub async fn download(
        &self,
        task: &DownloadTask,
        manifest: &DownloadManifest,
    ) -> Result<PathBuf, DownloadError> {
        if task.is_cancelled() {
            return Err(DownloadError::Cancelled);
        }
        validate_target(manifest)?;
        let url = parse_download_url(manifest.url())?;
        let artifact_path = self.cache_directory.join(manifest.sha256().as_str());
        if cache_matches(&artifact_path, manifest).await? {
            return Ok(artifact_path);
        }
        if task.is_cancelled() {
            return Err(DownloadError::Cancelled);
        }

        remove_if_exists(&artifact_path).await?;
        fs::create_dir_all(&self.cache_directory)
            .await
            .map_err(|source| DownloadError::Storage {
                operation: "create",
                path: self.cache_directory.clone(),
                source,
            })?;
        let temporary_path = self.cache_directory.join(format!("{}.partial", task.id()));
        remove_if_exists(&temporary_path).await?;

        let result = self
            .download_to_path(task, manifest, url, &temporary_path)
            .await;
        if result.is_err() {
            let _ = fs::remove_file(&temporary_path).await;
            return result.map(|_| artifact_path);
        }

        fs::rename(&temporary_path, &artifact_path)
            .await
            .map_err(|source| DownloadError::Storage {
                operation: "finalize",
                path: artifact_path.clone(),
                source,
            })?;

        Ok(artifact_path)
    }

    async fn download_to_path(
        &self,
        task: &DownloadTask,
        manifest: &DownloadManifest,
        url: Url,
        path: &Path,
    ) -> Result<(), DownloadError> {
        let mut cancellation = task.subscribe_cancellation();
        if *cancellation.borrow() {
            return Err(DownloadError::Cancelled);
        }

        let request = self.client.get(url).send();
        tokio::pin!(request);
        let mut response = select! {
            changed = cancellation.changed() => {
                match changed {
                    Ok(()) if *cancellation.borrow() => return Err(DownloadError::Cancelled),
                    Ok(()) | Err(_) => return Err(DownloadError::Cancelled),
                }
            }
            response = &mut request => response.map_err(DownloadError::Request)?,
        }
        .error_for_status()
        .map_err(DownloadError::Request)?;
        if let Some(content_length) = response.content_length() {
            if content_length != manifest.size_bytes() {
                return Err(DownloadError::ContentLengthMismatch {
                    expected_bytes: manifest.size_bytes(),
                    actual_bytes: content_length,
                });
            }
        }

        let mut file = File::create(path)
            .await
            .map_err(|source| DownloadError::Storage {
                operation: "create",
                path: path.to_path_buf(),
                source,
            })?;
        let mut hasher = Sha256::new();
        let mut downloaded_bytes = 0_u64;

        loop {
            let chunk = select! {
                changed = cancellation.changed() => {
                    match changed {
                        Ok(()) if *cancellation.borrow() => return Err(DownloadError::Cancelled),
                        Ok(()) | Err(_) => continue,
                    }
                }
                chunk = response.chunk() => chunk.map_err(DownloadError::Request)?,
            };
            let Some(chunk) = chunk else {
                break;
            };
            let chunk_bytes =
                u64::try_from(chunk.len()).map_err(|_| DownloadError::SizeMismatch {
                    expected_bytes: manifest.size_bytes(),
                    actual_bytes: u64::MAX,
                })?;
            downloaded_bytes =
                downloaded_bytes
                    .checked_add(chunk_bytes)
                    .ok_or(DownloadError::SizeMismatch {
                        expected_bytes: manifest.size_bytes(),
                        actual_bytes: u64::MAX,
                    })?;
            if downloaded_bytes > manifest.size_bytes() {
                return Err(DownloadError::SizeMismatch {
                    expected_bytes: manifest.size_bytes(),
                    actual_bytes: downloaded_bytes,
                });
            }

            file.write_all(&chunk)
                .await
                .map_err(|source| DownloadError::Storage {
                    operation: "write",
                    path: path.to_path_buf(),
                    source,
                })?;
            hasher.update(&chunk);
        }
        file.flush()
            .await
            .map_err(|source| DownloadError::Storage {
                operation: "flush",
                path: path.to_path_buf(),
                source,
            })?;

        verify_download(manifest, downloaded_bytes, sha256_hex(hasher.finalize()))
    }
}

async fn cache_matches(path: &Path, manifest: &DownloadManifest) -> Result<bool, DownloadError> {
    let mut file = match File::open(path).await {
        Ok(file) => file,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(source) => {
            return Err(DownloadError::Storage {
                operation: "read",
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let mut hasher = Sha256::new();
    let mut actual_bytes = 0_u64;
    let mut buffer = [0_u8; 32 * 1024];
    loop {
        let bytes_read = file
            .read(&mut buffer)
            .await
            .map_err(|source| DownloadError::Storage {
                operation: "read",
                path: path.to_path_buf(),
                source,
            })?;
        if bytes_read == 0 {
            break;
        }
        actual_bytes = actual_bytes
            .checked_add(
                u64::try_from(bytes_read).map_err(|_| DownloadError::SizeMismatch {
                    expected_bytes: manifest.size_bytes(),
                    actual_bytes: u64::MAX,
                })?,
            )
            .ok_or(DownloadError::SizeMismatch {
                expected_bytes: manifest.size_bytes(),
                actual_bytes: u64::MAX,
            })?;
        if actual_bytes > manifest.size_bytes() {
            return Ok(false);
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(verify_download(manifest, actual_bytes, sha256_hex(hasher.finalize())).is_ok())
}

async fn remove_if_exists(path: &Path) -> Result<(), DownloadError> {
    match fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(DownloadError::Storage {
            operation: "remove",
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn parse_download_url(value: &str) -> Result<Url, DownloadError> {
    let url = Url::parse(value).map_err(|_| DownloadError::InvalidUrl {
        url: value.to_owned(),
    })?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err(DownloadError::UrlContainsCredentials);
    }
    if url.scheme() != "https" {
        #[cfg(not(test))]
        return Err(DownloadError::InsecureUrl);
        #[cfg(test)]
        // Unit tests use a loopback HTTP server; production accepts HTTPS only.
        if url.scheme() != "http" {
            return Err(DownloadError::InsecureUrl);
        }
    }
    if url.host_str().is_none() {
        return Err(DownloadError::InvalidUrl {
            url: value.to_owned(),
        });
    }

    Ok(url)
}

fn validate_target(manifest: &DownloadManifest) -> Result<(), DownloadError> {
    if !manifest.platform().is_current() {
        return Err(DownloadError::UnsupportedPlatform {
            platform: manifest.platform(),
        });
    }
    if !manifest.architecture().is_current() {
        return Err(DownloadError::UnsupportedArchitecture {
            architecture: manifest.architecture(),
        });
    }

    Ok(())
}

fn verify_download(
    manifest: &DownloadManifest,
    actual_bytes: u64,
    actual_sha256: String,
) -> Result<(), DownloadError> {
    if actual_bytes != manifest.size_bytes() {
        return Err(DownloadError::SizeMismatch {
            expected_bytes: manifest.size_bytes(),
            actual_bytes,
        });
    }
    if actual_sha256 != manifest.sha256().as_str() {
        return Err(DownloadError::Sha256Mismatch {
            expected: manifest.sha256().as_str().to_owned(),
            actual: actual_sha256,
        });
    }

    Ok(())
}

fn sha256_hex(digest: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let digest = digest.as_ref();
    let mut value = String::with_capacity(digest.len() * 2);
    for byte in digest {
        value.push(char::from(HEX[usize::from(*byte >> 4)]));
        value.push(char::from(HEX[usize::from(*byte & 0x0f)]));
    }

    value
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use nexus_domain::DownloadArchitecture;
    use nexus_domain::DownloadPlatform;
    use nexus_domain::Sha256Digest;
    use sha2::Digest;
    use sha2::Sha256;
    use tempfile::tempdir;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;
    use tokio::time::Duration;
    use tokio::time::sleep;
    use tokio::time::timeout;

    use super::DownloadError;
    use super::DownloadManager;
    use crate::DownloadTask;

    #[tokio::test]
    async fn downloads_and_reuses_a_verified_cached_artifact() {
        let body = b"trusted artifact".to_vec();
        let address = serve_once(body.clone()).await;
        let data_directory = tempdir().expect("temporary data directory is created");
        let manager =
            DownloadManager::new(data_directory.path()).expect("download manager is created");
        let manifest = manifest(format!("http://{address}/artifact"), &body);

        let first = manager
            .download(&DownloadTask::new(), &manifest)
            .await
            .expect("artifact downloads");
        let second = manager
            .download(&DownloadTask::new(), &manifest)
            .await
            .expect("verified cache is reused");

        assert_eq!(first, second);
        assert_eq!(
            tokio::fs::read(first).await.expect("artifact is readable"),
            body
        );
    }

    #[tokio::test]
    async fn removes_partial_artifacts_when_a_task_is_cancelled() {
        let body = b"waiting artifact".to_vec();
        let (address, release) = serve_after_release(body.clone()).await;
        let data_directory = tempdir().expect("temporary data directory is created");
        let manager =
            DownloadManager::new(data_directory.path()).expect("download manager is created");
        let manifest = manifest(format!("http://{address}/artifact"), &body);
        let task = DownloadTask::new();
        let task_id = task.id();
        let download = manager.download(&task, &manifest);
        tokio::pin!(download);

        tokio::select! {
            result = &mut download => panic!("download completed before cancellation: {result:?}"),
            () = sleep(Duration::from_millis(20)) => {}
        }
        task.cancel();
        let error = timeout(Duration::from_secs(1), &mut download)
            .await
            .expect("cancellation interrupts download")
            .expect_err("cancelled download fails");
        release.send(()).expect("server is released");

        assert!(matches!(error, DownloadError::Cancelled));
        assert!(
            !data_directory
                .path()
                .join("downloads")
                .join(format!("{task_id}.partial"))
                .exists()
        );
    }

    #[tokio::test]
    async fn removes_partial_artifacts_when_the_digest_does_not_match() {
        let expected_body = b"expected body".to_vec();
        let actual_body = b"tampered body".to_vec();
        let address = serve_once(actual_body).await;
        let data_directory = tempdir().expect("temporary data directory is created");
        let manager =
            DownloadManager::new(data_directory.path()).expect("download manager is created");
        let manifest = manifest(format!("http://{address}/artifact"), &expected_body);
        let task = DownloadTask::new();

        let error = manager
            .download(&task, &manifest)
            .await
            .expect_err("digest mismatch rejects the artifact");

        assert!(matches!(error, DownloadError::Sha256Mismatch { .. }));
        assert!(
            !data_directory
                .path()
                .join("downloads")
                .join(format!("{}.partial", task.id()))
                .exists()
        );
    }

    #[test]
    fn rejects_a_manifest_for_a_different_platform() {
        let wrong_platform = match DownloadPlatform::current().expect("test platform is supported")
        {
            DownloadPlatform::Any => unreachable!("current platform is never universal"),
            DownloadPlatform::Windows => DownloadPlatform::Linux,
            DownloadPlatform::Linux | DownloadPlatform::Macos => DownloadPlatform::Windows,
        };
        let manifest = nexus_domain::DownloadManifest::new(
            "https://example.invalid/artifact".to_owned(),
            0,
            Sha256Digest::from_hex(
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            )
            .expect("empty file digest is valid"),
            wrong_platform,
            DownloadArchitecture::current().expect("test architecture is supported"),
        );

        assert!(matches!(
            super::validate_target(&manifest),
            Err(DownloadError::UnsupportedPlatform { .. })
        ));
    }

    fn manifest(url: String, body: &[u8]) -> nexus_domain::DownloadManifest {
        nexus_domain::DownloadManifest::new(
            url,
            u64::try_from(body.len()).expect("test artifact has valid size"),
            Sha256Digest::from_hex(&super::sha256_hex(Sha256::digest(body)))
                .expect("test digest is valid"),
            DownloadPlatform::current().expect("test platform is supported"),
            DownloadArchitecture::current().expect("test architecture is supported"),
        )
    }

    async fn serve_once(body: Vec<u8>) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test HTTP listener binds");
        let address = listener.local_addr().expect("listener has an address");
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("client connects");
            read_request(&mut stream).await;
            write_response(&mut stream, &body).await;
        });

        address
    }

    async fn serve_after_release(body: Vec<u8>) -> (SocketAddr, oneshot::Sender<()>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test HTTP listener binds");
        let address = listener.local_addr().expect("listener has an address");
        let (release, released) = oneshot::channel();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("client connects");
            read_request(&mut stream).await;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("response headers are written");
            let _ = released.await;
            let _ = stream.write_all(&body).await;
        });

        (address, release)
    }

    async fn read_request(stream: &mut tokio::net::TcpStream) {
        let mut buffer = [0_u8; 1024];
        let _ = stream.read(&mut buffer).await.expect("request is readable");
    }

    async fn write_response(stream: &mut tokio::net::TcpStream, body: &[u8]) {
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .await
            .expect("response headers are written");
        stream
            .write_all(body)
            .await
            .expect("response body is written");
    }
}
