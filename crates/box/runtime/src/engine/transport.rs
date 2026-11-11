use std::future::Future;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{watch, OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinSet;

use crate::file_lock::FileLock;
use crate::host_sockets::current_unix_uid;
use crate::pool::client::{read_frame_with_timeout, write_frame};

use super::{
    EngineCommand, EngineError, EngineErrorCode, EngineReply, EngineReplyValue, EngineRequest,
    EngineResult, LocalEngine,
};

pub(super) const MAX_CONTROL_FRAME_BYTES: usize = crate::pool::client::MAX_FRAME_SIZE;
pub(super) const CONTROL_TIMEOUT: Duration = Duration::from_secs(30);

/// Stable home-scoped endpoint that also fits Unix socket limits for long homes.
pub fn default_engine_endpoint(home: &std::path::Path) -> EngineResult<PathBuf> {
    use sha2::{Digest, Sha256};
    use std::os::unix::ffi::OsStrExt;
    let home = std::path::absolute(home)?;
    let digest = format!("{:x}", Sha256::digest(home.as_os_str().as_bytes()));
    let parent = if cfg!(target_os = "macos") {
        "/private/tmp"
    } else {
        "/tmp"
    };
    // SHA-256 hex is 64 ASCII bytes; 128 bits keep the path within sun_path.
    Ok(PathBuf::from(parent)
        .join(format!(
            "kbox-engine-{}-{}",
            current_unix_uid(),
            &digest[..32]
        ))
        .join("control.sock"))
}

/// Same-user local control endpoint; stopping the server leaves accepted intent recoverable.
pub async fn serve_engine(
    path: PathBuf,
    engine: Arc<LocalEngine>,
    shutdown: impl Future<Output = ()> + Send,
) -> EngineResult<()> {
    let socket = tokio::task::spawn_blocking(move || SocketOwner::bind(path))
        .await
        .map_err(|_| EngineError::new(EngineErrorCode::Internal, "socket worker failed"))??;
    let listener = UnixListener::from_std(socket.listener.try_clone()?)?;
    let (stop, stopped) = watch::channel(false);
    let mut workers = JoinSet::new();
    workers.spawn(engine.clone().run(stopped.clone()));
    if engine.prewarm.is_some() {
        workers.spawn(engine.clone().run_prewarm(stopped));
    }
    let mut clients = JoinSet::new();
    let permits = Arc::new(Semaphore::new(128));
    tokio::pin!(shutdown);
    let result = loop {
        tokio::select! {
            biased;
            _ = &mut shutdown => break Ok(()),
            Some(result) = workers.join_next() => break result.unwrap_or_else(|_| Err(EngineError::new(
                EngineErrorCode::Internal, "engine reconciliation failed"))),
            Some(_) = clients.join_next(), if !clients.is_empty() => {},
            incoming = listener.accept() => {
                let (stream, _) = match incoming { Ok(incoming) => incoming, Err(error) => break Err(error.into()) };
                if !stream.peer_cred().is_ok_and(|peer| require_peer_owner(peer.uid(), socket.uid).is_ok()) {
                    continue;
                }
                let Ok(permit) = permits.clone().try_acquire_owned() else { continue; };
                let engine = engine.clone();
                clients.spawn(async move {
                    let _ = handle_connection(stream, engine, permit).await;
                });
            }
        }
    };
    let primary_instances = engine.unfinished_primary_instances().await?;
    let _ = stop.send(true);
    clients.abort_all();
    while clients.join_next().await.is_some() {}
    while let Some(worker) = workers.join_next().await {
        worker.map_err(|_| {
            EngineError::new(
                EngineErrorCode::Internal,
                "engine worker stopped unexpectedly",
            )
        })??;
    }
    engine.close_primary_instances(primary_instances).await?;
    result
}

async fn read_request(stream: &mut UnixStream) -> EngineResult<EngineRequest> {
    let bytes = read_frame_with_timeout(stream, CONTROL_TIMEOUT)
        .await
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::InvalidData {
                EngineError::new(
                    EngineErrorCode::FrameTooLarge,
                    "control frame exceeds negotiated limit",
                )
            } else {
                error.into()
            }
        })?;
    let request =
        tokio::task::spawn_blocking(move || serde_json::from_slice::<EngineRequest>(&bytes))
            .await
            .map_err(|_| EngineError::new(EngineErrorCode::Internal, "request worker failed"))??;
    request.validate()?;
    Ok(request)
}

pub(super) async fn write_reply(
    stream: &mut UnixStream,
    result: EngineResult<EngineReplyValue>,
) -> EngineResult<()> {
    let bytes = serde_json::to_vec(&EngineReply::from(result))?;
    tokio::time::timeout(CONTROL_TIMEOUT, write_frame(stream, &bytes))
        .await
        .map_err(|_| {
            EngineError::new(EngineErrorCode::Transport, "control response timed out")
        })??;
    Ok(())
}

async fn handle_connection(
    mut stream: UnixStream,
    engine: Arc<LocalEngine>,
    permit: OwnedSemaphorePermit,
) -> EngineResult<()> {
    let request = match read_request(&mut stream).await {
        Ok(request) => request,
        Err(error) => return write_reply(&mut stream, Err(error)).await,
    };
    match request.command {
        EngineCommand::OpenPrimaryProcess { process } => {
            let pending =
                match tokio::time::timeout(CONTROL_TIMEOUT, engine.reserve_primary(*process)).await
                {
                    Ok(Ok(pending)) => pending,
                    Ok(Err(error)) => return write_reply(&mut stream, Err(error)).await,
                    Err(_) => {
                        return write_reply(
                            &mut stream,
                            Err(EngineError::new(
                                EngineErrorCode::Unknown,
                                "process admission timed out; inspect the original request",
                            )),
                        )
                        .await
                    }
                };
            write_reply(
                &mut stream,
                Ok(EngineReplyValue::PrimaryTicket(pending.ticket.clone())),
            )
            .await?;
            drop(permit);
            super::process_transport::serve_primary(stream, engine, pending).await
        }
        EngineCommand::AttachPrimaryProcess { token } => {
            let sender = match engine.take_primary_attachment(&token) {
                Ok(sender) => sender,
                Err(error) => return write_reply(&mut stream, Err(error)).await,
            };
            drop(permit);
            if let Err(mut stream) = sender.send(stream) {
                write_reply(
                    &mut stream,
                    Err(EngineError::new(
                        EngineErrorCode::StreamLost,
                        "primary process owner disconnected",
                    )),
                )
                .await?;
            }
            Ok(())
        }
        command => {
            let result = tokio::time::timeout(
                CONTROL_TIMEOUT,
                engine.dispatch(EngineRequest {
                    schema: request.schema,
                    command,
                }),
            )
            .await
            .unwrap_or_else(|_| {
                Err(EngineError::new(
                    EngineErrorCode::Unknown,
                    "control operation timed out; inspect the original request",
                ))
            });
            write_reply(&mut stream, result).await
        }
    }
}

fn require_peer_owner(actual: u32, expected: u32) -> EngineResult<()> {
    if actual != expected {
        return Err(EngineError::new(
            EngineErrorCode::Forbidden,
            "control peer must own the engine",
        ));
    }
    Ok(())
}

struct SocketOwner {
    listener: std::os::unix::net::UnixListener,
    path: PathBuf,
    uid: u32,
    device: u64,
    inode: u64,
    _lock: FileLock,
}

impl SocketOwner {
    fn bind(path: PathBuf) -> EngineResult<Self> {
        let parent = path.parent().ok_or_else(|| {
            EngineError::new(
                EngineErrorCode::InvalidRequest,
                "socket needs a private parent",
            )
        })?;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;
        let uid = current_unix_uid();
        let metadata = std::fs::symlink_metadata(parent)?;
        if !metadata.is_dir() || metadata.uid() != uid || metadata.mode() & 0o077 != 0 {
            return Err(EngineError::new(
                EngineErrorCode::Forbidden,
                "socket parent must be owner-only",
            ));
        }
        let lock = FileLock::try_acquire(&path)?.ok_or_else(|| {
            EngineError::new(
                EngineErrorCode::RequestConflict,
                "engine socket has a live owner",
            )
        })?;
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if !metadata.file_type().is_socket() || metadata.uid() != uid {
                    return Err(EngineError::new(
                        EngineErrorCode::Forbidden,
                        "refusing to replace socket path",
                    ));
                }
                match std::os::unix::net::UnixStream::connect(&path) {
                    Ok(_) => {
                        return Err(EngineError::new(
                            EngineErrorCode::RequestConflict,
                            "engine socket is in use",
                        ))
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
                        std::fs::remove_file(&path)?
                    }
                    Err(error) => return Err(error.into()),
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let listener = std::os::unix::net::UnixListener::bind(&path)?;
        listener.set_nonblocking(true)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        let metadata = std::fs::symlink_metadata(&path)?;
        Ok(Self {
            listener,
            path,
            uid,
            device: metadata.dev(),
            inode: metadata.ino(),
            _lock: lock,
        })
    }
}

impl Drop for SocketOwner {
    fn drop(&mut self) {
        if std::fs::symlink_metadata(&self.path).is_ok_and(|metadata| {
            metadata.file_type().is_socket()
                && metadata.dev() == self.device
                && metadata.ino() == self.inode
        }) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_refuses_other_peer_or_non_private_parent() {
        assert_eq!(
            require_peer_owner(12, 13).unwrap_err().code,
            EngineErrorCode::Forbidden
        );
        require_peer_owner(12, 12).unwrap();
        let directory = tempfile::tempdir().unwrap();
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(
            SocketOwner::bind(directory.path().join("s")),
            Err(EngineError {
                code: EngineErrorCode::Forbidden,
                ..
            })
        ));
    }

    #[test]
    fn socket_preserves_existing_file_and_exclusively_recovers_stale_socket() {
        let directory = tempfile::tempdir_in("/tmp").unwrap();
        let private = directory.path().join("private");
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&private)
            .unwrap();
        let path = private.join("s");
        std::fs::write(&path, b"keep").unwrap();
        assert!(SocketOwner::bind(path.clone()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"keep");
        std::fs::remove_file(&path).unwrap();
        drop(std::os::unix::net::UnixListener::bind(&path).unwrap());
        let owner = SocketOwner::bind(path.clone()).unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().mode() & 0o777, 0o600);
        assert!(SocketOwner::bind(path.clone()).is_err());
        drop(owner);
        assert!(!path.exists());
        SocketOwner::bind(path).unwrap();
    }
}
