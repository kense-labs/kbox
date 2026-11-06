//! Live stdio attachment to an application-prewarmed pool VM.

use kbox_core::error::{BoxError, Result};
use serde::{Deserialize, Serialize};

use super::client::{read_frame, write_frame};

const ATTACH_STDIN: u8 = 0;
const ATTACH_STDIN_CLOSE: u8 = 1;
const ATTACH_STDOUT: u8 = 0;
const ATTACH_STDERR: u8 = 1;
const ATTACH_EXIT: u8 = 2;

/// Incremental frame decoder for `select!` loops.
///
/// Each await performs at most one cancellation-safe socket read. If another
/// branch wins the selection, the partially-read prefix and payload remain in
/// this decoder instead of being lost with a cancelled `read_exact` future.
#[cfg(not(windows))]
pub struct AttachFrameReader<R> {
    reader: R,
    header: [u8; 4],
    header_read: usize,
    payload: Vec<u8>,
    payload_read: usize,
}

#[cfg(not(windows))]
impl<R: tokio::io::AsyncRead + Unpin> AttachFrameReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            header: [0; 4],
            header_read: 0,
            payload: Vec::new(),
            payload_read: 0,
        }
    }

    pub async fn read_frame(&mut self) -> std::io::Result<Vec<u8>> {
        use tokio::io::AsyncReadExt;

        loop {
            if self.header_read < self.header.len() {
                let read = self
                    .reader
                    .read(&mut self.header[self.header_read..])
                    .await?;
                if read == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "pool attach connection closed while reading a frame header",
                    ));
                }
                self.header_read += read;
                continue;
            }

            if self.payload.is_empty() && self.payload_read == 0 {
                let frame_len = u32::from_le_bytes(self.header) as usize;
                if frame_len > super::client::MAX_FRAME_SIZE {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "pool frame is {frame_len} bytes; maximum is {} bytes",
                            super::client::MAX_FRAME_SIZE
                        ),
                    ));
                }
                self.payload = vec![0; frame_len];
                if frame_len == 0 {
                    self.header_read = 0;
                    return Ok(std::mem::take(&mut self.payload));
                }
            }

            let read = self
                .reader
                .read(&mut self.payload[self.payload_read..])
                .await?;
            if read == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "pool attach connection closed while reading a frame payload",
                ));
            }
            self.payload_read += read;
            if self.payload_read == self.payload.len() {
                self.header_read = 0;
                self.payload_read = 0;
                return Ok(std::mem::take(&mut self.payload));
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PoolAttachRequest {
    /// Image whose prewarmed entrypoint should be attached.
    #[serde(default)]
    pub image: Option<String>,
    /// Boot-time volume key; empty selects the daemon's default prewarm mounts.
    #[serde(default)]
    pub volumes: Vec<String>,
    #[serde(default)]
    pub vcpus: Option<u32>,
    #[serde(default)]
    pub memory_mb: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PoolAttachResponse {
    pub error: Option<String>,
}

pub struct PoolClientAttach {
    pub socket: String,
    pub image: Option<String>,
    pub volumes: Vec<String>,
    pub vcpus: u32,
    pub memory_mb: u32,
}

/// Client-to-daemon input event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachInput {
    Data(Vec<u8>),
    Close,
}

impl AttachInput {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Data(data) => {
                let mut payload = Vec::with_capacity(data.len() + 1);
                payload.push(ATTACH_STDIN);
                payload.extend_from_slice(data);
                payload
            }
            Self::Close => vec![ATTACH_STDIN_CLOSE],
        }
    }

    pub fn parse(payload: &[u8]) -> Result<Self> {
        match payload.split_first() {
            Some((&ATTACH_STDIN, data)) => Ok(Self::Data(data.to_vec())),
            Some((&ATTACH_STDIN_CLOSE, [])) => Ok(Self::Close),
            _ => Err(BoxError::PoolError(
                "invalid pool-attach input frame".to_string(),
            )),
        }
    }
}

/// Daemon-to-client output event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachOutput {
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    Exit(i32),
}

impl AttachOutput {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Stdout(data) | Self::Stderr(data) => {
                let kind = if matches!(self, Self::Stdout(_)) {
                    ATTACH_STDOUT
                } else {
                    ATTACH_STDERR
                };
                let mut payload = Vec::with_capacity(data.len() + 1);
                payload.push(kind);
                payload.extend_from_slice(data);
                payload
            }
            Self::Exit(code) => {
                let mut payload = Vec::with_capacity(5);
                payload.push(ATTACH_EXIT);
                payload.extend_from_slice(&code.to_le_bytes());
                payload
            }
        }
    }

    pub fn parse(payload: &[u8]) -> Result<Self> {
        match payload.split_first() {
            Some((&ATTACH_STDOUT, data)) => Ok(Self::Stdout(data.to_vec())),
            Some((&ATTACH_STDERR, data)) => Ok(Self::Stderr(data.to_vec())),
            Some((&ATTACH_EXIT, code)) if code.len() == 4 => {
                Ok(Self::Exit(i32::from_le_bytes(code.try_into().map_err(
                    |_| BoxError::PoolError("invalid pool-attach exit frame".to_string()),
                )?)))
            }
            _ => Err(BoxError::PoolError(
                "invalid pool-attach output frame".to_string(),
            )),
        }
    }
}

/// Attach the current process's stdio to a prewarmed application and return its
/// real exit code. No command is spawned on this path.
#[cfg(not(windows))]
pub async fn attach_client(req: PoolClientAttach) -> Result<i32> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    let mut stream = UnixStream::connect(&req.socket).await.map_err(|error| {
        BoxError::PoolError(format!(
            "Failed to connect to pool daemon at {} ({error}). Is `kbox pool start` running?",
            req.socket
        ))
    })?;
    write_frame(
        &mut stream,
        &serde_json::to_vec(&super::client::PoolRequest::Attach(PoolAttachRequest {
            image: req.image,
            volumes: req.volumes,
            vcpus: Some(req.vcpus),
            memory_mb: Some(req.memory_mb),
        }))?,
    )
    .await?;
    let response: PoolAttachResponse = serde_json::from_slice(&read_frame(&mut stream).await?)?;
    if let Some(error) = response.error {
        return Err(BoxError::PoolError(error));
    }

    let (reader, mut writer) = stream.into_split();
    let mut frames = AttachFrameReader::new(reader);
    let mut stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut stderr = tokio::io::stderr();
    let mut input_buffer = [0_u8; 16 * 1024];
    let mut stdin_open = true;

    loop {
        tokio::select! {
            input = stdin.read(&mut input_buffer), if stdin_open => {
                let read = input.map_err(|error| BoxError::PoolError(format!("stdin read failed: {error}")))?;
                let event = if read == 0 {
                    stdin_open = false;
                    AttachInput::Close
                } else {
                    AttachInput::Data(input_buffer[..read].to_vec())
                };
                write_frame(&mut writer, &event.encode()).await?;
            }
            frame = frames.read_frame() => {
                let frame = frame?;
                match AttachOutput::parse(&frame)? {
                    AttachOutput::Stdout(data) => {
                        stdout.write_all(&data).await?;
                        stdout.flush().await?;
                    }
                    AttachOutput::Stderr(data) => {
                        stderr.write_all(&data).await?;
                        stderr.flush().await?;
                    }
                    AttachOutput::Exit(code) => {
                        stdout.flush().await?;
                        stderr.flush().await?;
                        return Ok(code);
                    }
                }
            }
        }
    }
}

#[cfg(windows)]
pub async fn attach_client(_req: PoolClientAttach) -> Result<i32> {
    Err(BoxError::PoolError(
        "`pool attach` is not supported on Windows".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attach_frames_round_trip() {
        for event in [
            AttachInput::Data(b"hello\0world".to_vec()),
            AttachInput::Close,
        ] {
            assert_eq!(AttachInput::parse(&event.encode()).unwrap(), event);
        }
        for event in [
            AttachOutput::Stdout(b"out".to_vec()),
            AttachOutput::Stderr(b"err".to_vec()),
            AttachOutput::Exit(-1),
        ] {
            assert_eq!(AttachOutput::parse(&event.encode()).unwrap(), event);
        }
    }

    #[test]
    fn malformed_attach_frames_are_rejected() {
        assert!(AttachInput::parse(&[]).is_err());
        assert!(AttachInput::parse(&[ATTACH_STDIN_CLOSE, 0]).is_err());
        assert!(AttachOutput::parse(&[ATTACH_EXIT, 0]).is_err());
        assert!(AttachOutput::parse(&[255]).is_err());
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn cancelled_read_preserves_partial_header_and_payload() {
        use tokio::io::AsyncWriteExt;

        let (mut writer, reader) = tokio::io::duplex(64);
        let mut frames = AttachFrameReader::new(reader);
        writer.write_all(&5_u32.to_le_bytes()[..2]).await.unwrap();

        tokio::select! {
            biased;
            result = frames.read_frame() => panic!("partial header unexpectedly completed: {result:?}"),
            _ = tokio::task::yield_now() => {}
        }
        assert_eq!(frames.header_read, 2, "the partial prefix must be retained");

        writer.write_all(&5_u32.to_le_bytes()[2..]).await.unwrap();
        writer.write_all(b"he").await.unwrap();
        tokio::select! {
            biased;
            result = frames.read_frame() => panic!("partial payload unexpectedly completed: {result:?}"),
            _ = tokio::task::yield_now() => {}
        }
        assert_eq!(
            frames.payload_read, 2,
            "the partial payload must be retained"
        );

        writer.write_all(b"llo").await.unwrap();
        writer.write_all(&3_u32.to_le_bytes()).await.unwrap();
        writer.write_all(b"two").await.unwrap();
        assert_eq!(frames.read_frame().await.unwrap(), b"hello");
        assert_eq!(frames.read_frame().await.unwrap(), b"two");
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn incremental_reader_rejects_oversized_and_truncated_frames() {
        use tokio::io::AsyncWriteExt;

        let (mut writer, reader) = tokio::io::duplex(64);
        let mut frames = AttachFrameReader::new(reader);
        writer
            .write_all(&((super::super::client::MAX_FRAME_SIZE as u32) + 1).to_le_bytes())
            .await
            .unwrap();
        assert_eq!(
            frames.read_frame().await.unwrap_err().kind(),
            std::io::ErrorKind::InvalidData
        );

        let (mut writer, reader) = tokio::io::duplex(64);
        let mut frames = AttachFrameReader::new(reader);
        writer.write_all(&4_u32.to_le_bytes()).await.unwrap();
        writer.write_all(b"ab").await.unwrap();
        drop(writer);
        assert_eq!(
            frames.read_frame().await.unwrap_err().kind(),
            std::io::ErrorKind::UnexpectedEof
        );
    }
}
