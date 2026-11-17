//! Minimal Docker Engine HTTP client over a Unix socket.
//!
//! Spec050 adapter surface: create / start / inspect / exec / kill / remove.
//! Socket path is always caller-supplied so ExecutionManager homes stay
//! isolated. Extracted from the binary `client` request framing.

#![cfg(unix)]

use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};

/// HTTP response from the engine daemon.
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Response {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }

    pub fn error_message(&self) -> String {
        self.json()["message"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| String::from_utf8_lossy(&self.body).into_owned())
    }

    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// Client bound to one absolute daemon socket path.
#[derive(Debug, Clone)]
pub struct DockerHttpClient {
    socket: std::path::PathBuf,
}

impl DockerHttpClient {
    pub fn new(socket: impl Into<std::path::PathBuf>) -> Self {
        Self {
            socket: socket.into(),
        }
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket
    }

    /// Fail closed when the socket file is missing.
    ///
    /// Does not open a probe connection: a connect-only probe would race with
    /// single-accept fake daemons in hermetic tests and with real HTTP calls.
    pub fn ensure_reachable(&self) -> Result<(), String> {
        if !self.socket.exists() {
            return Err(format!(
                "sandbox engine daemon unreachable at {}",
                self.socket.display()
            ));
        }
        Ok(())
    }

    pub fn ping(&self) -> io::Result<()> {
        let response = self.request("GET", "/_ping", None)?;
        if response.ok() {
            Ok(())
        } else {
            Err(io::Error::other(response.error_message()))
        }
    }

    /// Ensure `image` (name or `name:tag`) is present in the daemon image store.
    ///
    /// Uses `POST /images/create`. Idempotent when the image is already local.
    pub fn image_pull(&self, image: &str) -> io::Result<()> {
        let (from_image, tag) = split_image_ref(image);
        let path = format!(
            "/images/create?fromImage={}&tag={}",
            urlencoding_minimal(from_image),
            urlencoding_minimal(tag)
        );
        // Pulls stream progress frames; allow a generous wall clock.
        let response = self.request_with_timeout("POST", &path, None, Duration::from_secs(300))?;
        if response.ok() {
            Ok(())
        } else {
            Err(io::Error::other(response.error_message()))
        }
    }

    /// Inspect a network by name or id.
    pub fn network_inspect(&self, name: &str) -> io::Result<Value> {
        let path = format!("/networks/{}", path_segment(name));
        let response = self.request("GET", &path, None)?;
        if response.status == 404 {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                response.error_message(),
            ));
        }
        if !response.ok() {
            return Err(io::Error::other(response.error_message()));
        }
        Ok(response.json())
    }

    /// Create a user-defined bridge network (idempotent on name conflict).
    pub fn network_create(&self, name: &str) -> io::Result<()> {
        let body = json!({
            "Name": name,
            "Driver": "bridge",
            "CheckDuplicate": true,
        });
        let response = self.request("POST", "/networks/create", Some(&body))?;
        if response.ok() || response.status == 201 {
            return Ok(());
        }
        // Already exists is success for compose project networks.
        let message = response.error_message().to_ascii_lowercase();
        if response.status == 409 || message.contains("already") {
            return Ok(());
        }
        Err(io::Error::other(response.error_message()))
    }

    /// Inspect-or-create a user-defined bridge network by name.
    pub fn network_ensure(&self, name: &str) -> io::Result<()> {
        match self.network_inspect(name) {
            Ok(_) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => self.network_create(name),
            Err(error) => Err(error),
        }
    }

    /// Connect a container to a network, optionally registering DNS aliases.
    pub fn network_connect(
        &self,
        network: &str,
        container: &str,
        aliases: &[String],
    ) -> io::Result<()> {
        let mut endpoint = json!({});
        if !aliases.is_empty() {
            endpoint["Aliases"] = json!(aliases);
        }
        let body = json!({
            "Container": container,
            "EndpointConfig": endpoint,
        });
        let path = format!("/networks/{}/connect", path_segment(network));
        let response = self.request("POST", &path, Some(&body))?;
        if response.ok() {
            Ok(())
        } else {
            Err(io::Error::other(response.error_message()))
        }
    }

    /// Remove a user-defined network (404 = already gone).
    pub fn network_remove(&self, name: &str) -> io::Result<()> {
        let path = format!("/networks/{}", path_segment(name));
        let response = self.request("DELETE", &path, None)?;
        if response.ok() || response.status == 404 {
            Ok(())
        } else {
            Err(io::Error::other(response.error_message()))
        }
    }

    /// Create a container; returns the daemon container id.
    pub fn container_create(
        &self,
        image: &str,
        cmd: &[String],
        env: &[(String, String)],
        workdir: Option<&str>,
        name: Option<&str>,
        binds: &[String],
        port_map: &[String],
    ) -> io::Result<String> {
        self.container_create_with_network(
            image,
            cmd,
            env,
            workdir,
            name,
            binds,
            port_map,
            None,
            &[],
        )
    }

    /// Create a container optionally attached to a named bridge network.
    #[allow(clippy::too_many_arguments)]
    pub fn container_create_with_network(
        &self,
        image: &str,
        cmd: &[String],
        env: &[(String, String)],
        workdir: Option<&str>,
        name: Option<&str>,
        binds: &[String],
        port_map: &[String],
        network: Option<&str>,
        aliases: &[String],
    ) -> io::Result<String> {
        let (exposed, bindings) = docker_port_spec(port_map);
        let mut host_config = json!({
            "AutoRemove": false,
            "Binds": binds,
            "PortBindings": bindings,
        });
        let network = network.filter(|n| !n.is_empty());
        if let Some(network) = network {
            host_config["NetworkMode"] = Value::String(network.to_string());
        }
        let mut body = json!({
            "Image": image,
            "Cmd": cmd,
            "Env": env.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>(),
            "ExposedPorts": exposed,
            "HostConfig": host_config,
        });
        if let Some(network) = network {
            // User-defined networks resolve aliases (and container names) for
            // Compose service DNS; builtin bridge does not.
            let mut endpoint = json!({});
            if !aliases.is_empty() {
                endpoint["Aliases"] = json!(aliases);
            }
            body["NetworkingConfig"] = json!({
                "EndpointsConfig": {
                    network: endpoint,
                }
            });
        }
        if let Some(workdir) = workdir {
            body["WorkingDir"] = Value::String(workdir.to_string());
        }
        let path = match name {
            Some(name) => format!("/containers/create?name={}", urlencoding_minimal(name)),
            None => "/containers/create".to_string(),
        };
        let response = self.request("POST", &path, Some(&body))?;
        if !response.ok() {
            return Err(io::Error::other(response.error_message()));
        }
        response.json()["Id"]
            .as_str()
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| io::Error::other("container create response missing Id"))
    }

    pub fn container_start(&self, id: &str) -> io::Result<()> {
        let path = format!("/containers/{}/start", path_segment(id));
        let response = self.request("POST", &path, None)?;
        if response.ok() || response.status == 304 {
            Ok(())
        } else {
            Err(io::Error::other(response.error_message()))
        }
    }

    /// Attach process-only Env that reaches start/exec but is omitted from
    /// durable `containers.json` and inspect Config.Env.
    pub fn container_set_ephemeral_env(
        &self,
        id: &str,
        env: &[(String, String)],
    ) -> io::Result<()> {
        let path = format!("/kbox/containers/{}/ephemeral-env", path_segment(id));
        let body = json!({
            "env": env
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>(),
        });
        let response = self.request("POST", &path, Some(&body))?;
        if response.ok() || response.status == 204 {
            Ok(())
        } else {
            Err(io::Error::other(response.error_message()))
        }
    }

    pub fn container_pause(&self, id: &str) -> io::Result<()> {
        let path = format!("/containers/{}/pause", path_segment(id));
        let response = self.request("POST", &path, None)?;
        if response.ok() {
            Ok(())
        } else {
            Err(io::Error::other(response.error_message()))
        }
    }

    pub fn container_unpause(&self, id: &str) -> io::Result<()> {
        let path = format!("/containers/{}/unpause", path_segment(id));
        let response = self.request("POST", &path, None)?;
        if response.ok() {
            Ok(())
        } else {
            Err(io::Error::other(response.error_message()))
        }
    }

    pub fn container_inspect(&self, id: &str) -> io::Result<Value> {
        let path = format!("/containers/{}/json", path_segment(id));
        let response = self.request("GET", &path, None)?;
        if response.status == 404 {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                response.error_message(),
            ));
        }
        if !response.ok() {
            return Err(io::Error::other(response.error_message()));
        }
        Ok(response.json())
    }

    pub fn container_kill(&self, id: &str, signal: &str) -> io::Result<()> {
        let path = format!(
            "/containers/{}/kill?signal={}",
            path_segment(id),
            urlencoding_minimal(signal)
        );
        let response = self.request("POST", &path, None)?;
        if response.ok() {
            return Ok(());
        }
        if response.status == 404 || response.status == 409 {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                response.error_message(),
            ));
        }
        Err(io::Error::other(response.error_message()))
    }

    pub fn container_remove(&self, id: &str, force: bool) -> io::Result<()> {
        let path = format!("/containers/{}?force={force}", path_segment(id));
        let response = self.request("DELETE", &path, None)?;
        if response.ok() || response.status == 404 {
            Ok(())
        } else {
            Err(io::Error::other(response.error_message()))
        }
    }

    /// Fetch current stdout/stderr logs (non-follow) as demultiplexed bytes.
    pub fn container_logs(&self, id: &str) -> io::Result<Vec<u8>> {
        let path = format!(
            "/containers/{}/logs?stdout=1&stderr=1&timestamps=0",
            path_segment(id)
        );
        let response = self.request("GET", &path, None)?;
        if !response.ok() {
            return Err(io::Error::other(response.error_message()));
        }
        Ok(demux_docker_logs(&response.body))
    }

    /// Upper bound for one-shot captured stdin (matches OCI SDK write bound).
    pub const MAX_CAPTURED_EXEC_STDIN_BYTES: usize = 16 * 1024 * 1024;

    /// Run `cmd` inside a running container via create → start → inspect.
    ///
    /// Uses non-TTY multiplexed framing so stdout and stderr stay separate.
    /// When `stdin` is empty, does not request a TCP upgrade: the daemon
    /// returns HTTP 200 with the stream as the body. Non-empty `stdin` uses
    /// AttachStdin + upgrade hijack, writes the buffer, half-closes write,
    /// then drains mux frames until EOF.
    pub fn container_exec(
        &self,
        id: &str,
        cmd: &[String],
        env: Option<&[String]>,
        user: Option<&str>,
        workdir: Option<&str>,
        stdin: Option<&[u8]>,
    ) -> io::Result<(i32, Vec<u8>, Vec<u8>)> {
        if cmd.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "container exec requires a non-empty command",
            ));
        }
        let stdin = stdin.unwrap_or(&[]);
        if stdin.len() > Self::MAX_CAPTURED_EXEC_STDIN_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "captured exec stdin is {} bytes; max is {}",
                    stdin.len(),
                    Self::MAX_CAPTURED_EXEC_STDIN_BYTES
                ),
            ));
        }
        if stdin.is_empty() {
            let exec_id = self.exec_create(id, cmd, env, user, workdir, false, false, None)?;
            let start_path = format!("/exec/{}/start", path_segment(&exec_id));
            let start_body = json!({ "Detach": false, "Tty": false });
            // Wall clock covers process lifetime + stream drain (mirrors CLI client).
            let started = self.request_with_timeout(
                "POST",
                &start_path,
                Some(&start_body),
                Duration::from_secs(120),
            )?;
            if !(started.ok() || started.status == 101) {
                return Err(io::Error::other(started.error_message()));
            }
            let (stdout, stderr) = demux_docker_streams(&started.body);
            let exit_code = self.exec_inspect_exit(&exec_id)?;
            return Ok((exit_code, stdout, stderr));
        }

        // Captured stdin: AttachStdin + hijack so we can write then EOF.
        let mut hijack = self.exec_hijack(
            id,
            cmd,
            env,
            user,
            workdir,
            false,
            None,
            json!({ "Detach": false, "Tty": false }),
        )?;
        {
            let mut writer = hijack.try_clone_writer()?;
            writer.write_all(stdin)?;
            writer.flush()?;
        }
        hijack.shutdown_write()?;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        while let Some(chunk) = hijack.next_output()? {
            match chunk {
                HijackOutput::Stdout(bytes) => stdout.extend_from_slice(&bytes),
                HijackOutput::Stderr(bytes) => stderr.extend_from_slice(&bytes),
            }
        }
        let exit_code = self.exec_inspect_exit(&hijack.exec_id)?;
        Ok((exit_code, stdout, stderr))
    }

    /// Create + start a TTY exec and return the hijacked bidirectional stream.
    ///
    /// Uses Docker `Upgrade: tcp` so stdin stays open for interactive sessions
    /// (`kbox exec -it`). Output is raw (not multiplexed) while `Tty` is true.
    pub fn container_exec_pty(
        &self,
        id: &str,
        cmd: &[String],
        env: Option<&[String]>,
        user: Option<&str>,
        workdir: Option<&str>,
        cols: u16,
        rows: u16,
    ) -> io::Result<ExecHijack> {
        if cols == 0 || rows == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "PTY width and height must both be positive",
            ));
        }
        self.exec_hijack(
            id,
            cmd,
            env,
            user,
            workdir,
            true,
            Some((rows, cols)),
            json!({
                "Detach": false,
                "Tty": true,
                "ConsoleSize": [rows, cols],
            }),
        )
    }

    /// Create + start a non-TTY exec attach and return the hijacked stream.
    ///
    /// Output uses Docker multiplexed framing (stdout=1, stderr=2). Stdin stays
    /// open for streaming sessions (`kbox exec -i` without `-t`).
    pub fn container_exec_attach(
        &self,
        id: &str,
        cmd: &[String],
        env: Option<&[String]>,
        user: Option<&str>,
        workdir: Option<&str>,
    ) -> io::Result<ExecHijack> {
        self.exec_hijack(
            id,
            cmd,
            env,
            user,
            workdir,
            false,
            None,
            json!({ "Detach": false, "Tty": false }),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn exec_hijack(
        &self,
        id: &str,
        cmd: &[String],
        env: Option<&[String]>,
        user: Option<&str>,
        workdir: Option<&str>,
        tty: bool,
        console_size: Option<(u16, u16)>,
        start_body: Value,
    ) -> io::Result<ExecHijack> {
        if cmd.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "container exec requires a non-empty command",
            ));
        }
        let exec_id = self.exec_create(id, cmd, env, user, workdir, tty, true, console_size)?;
        let start_path = format!("/exec/{}/start", path_segment(&exec_id));
        let mut stream = UnixStream::connect(&self.socket)?;
        // Interactive sessions outlive short request timeouts; keep write
        // bounded and let reads block until peer EOF / process exit.
        stream.set_write_timeout(Some(Duration::from_secs(30)))?;
        stream.set_read_timeout(None)?;
        stream.write_all(&encode_upgrade_request(
            "POST",
            &start_path,
            Some(&start_body),
        ))?;
        let mut reader = BufReader::new(stream);
        let (status, _len, _chunked) = read_head(&mut reader)?;
        if !(status == 101 || (200..300).contains(&status)) {
            let mut err_body = Vec::new();
            let _ = reader.read_to_end(&mut err_body);
            return Err(io::Error::other(
                String::from_utf8_lossy(&err_body).into_owned(),
            ));
        }
        // Bytes already buffered behind the response headers must be visible
        // to the first read so the caller sees a contiguous attach stream.
        let pending = reader.buffer().to_vec();
        let stream = reader.into_inner();
        Ok(ExecHijack {
            exec_id,
            socket: self.socket.clone(),
            stream,
            pending,
            tty,
        })
    }

    /// Inspect a finished (or running) exec for its exit code.
    pub fn exec_inspect_exit(&self, exec_id: &str) -> io::Result<i32> {
        let inspect_path = format!("/exec/{}/json", path_segment(exec_id));
        let inspected = self.request("GET", &inspect_path, None)?;
        if !inspected.ok() {
            return Err(io::Error::other(inspected.error_message()));
        }
        Ok(inspected.json()["ExitCode"].as_i64().unwrap_or(-1) as i32)
    }

    /// Resize a running TTY exec (`POST /exec/{id}/resize?h=&w=`).
    pub fn exec_resize(&self, exec_id: &str, cols: u16, rows: u16) -> io::Result<()> {
        if cols == 0 || rows == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "PTY width and height must both be positive",
            ));
        }
        let path = format!("/exec/{}/resize?h={rows}&w={cols}", path_segment(exec_id));
        let response = self.request("POST", &path, None)?;
        if response.ok() || response.status == 204 {
            Ok(())
        } else {
            Err(io::Error::other(response.error_message()))
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn exec_create(
        &self,
        id: &str,
        cmd: &[String],
        env: Option<&[String]>,
        user: Option<&str>,
        workdir: Option<&str>,
        tty: bool,
        attach_stdin: bool,
        console_size: Option<(u16, u16)>,
    ) -> io::Result<String> {
        let mut body = json!({
            "Cmd": cmd,
            "AttachStdout": true,
            "AttachStderr": true,
            "AttachStdin": attach_stdin,
            "Tty": tty,
        });
        if let Some(env) = env.filter(|entries| !entries.is_empty()) {
            body["Env"] = json!(env);
        }
        if let Some(user) = user.filter(|value| !value.is_empty()) {
            body["User"] = Value::String(user.to_string());
        }
        if let Some(workdir) = workdir.filter(|value| !value.is_empty()) {
            body["WorkingDir"] = Value::String(workdir.to_string());
        }
        if let Some((rows, cols)) = console_size {
            body["ConsoleSize"] = json!([rows, cols]);
        }
        let create_path = format!("/containers/{}/exec", path_segment(id));
        let created = self.request("POST", &create_path, Some(&body))?;
        if !created.ok() {
            return Err(io::Error::other(created.error_message()));
        }
        created.json()["Id"]
            .as_str()
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| io::Error::other("exec create response missing Id"))
    }

    /// Follow logs until the HTTP stream ends; append demultiplexed text to `path`.
    pub fn follow_logs_to_file(&self, id: &str, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let http_path = format!(
            "/containers/{}/logs?stdout=1&stderr=1&follow=1&timestamps=0",
            path_segment(id)
        );
        let mut stream = UnixStream::connect(&self.socket)?;
        stream.set_read_timeout(Some(Duration::from_secs(3600)))?;
        stream.set_write_timeout(Some(Duration::from_secs(30)))?;
        stream.write_all(&encode_request("GET", &http_path, None))?;
        let mut reader = BufReader::new(stream);
        let (status, len, chunked) = read_head(&mut reader)?;
        if !(200..300).contains(&status) {
            let mut err_body = Vec::new();
            let _ = reader.read_to_end(&mut err_body);
            return Err(io::Error::other(
                String::from_utf8_lossy(&err_body).into_owned(),
            ));
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        let mut pending = Vec::new();
        let mut write_payload = |chunk: &[u8]| -> io::Result<()> {
            pending.extend_from_slice(chunk);
            let (demuxed, rest) = take_demuxed_frames(&pending);
            pending = rest;
            if !demuxed.is_empty() {
                file.write_all(&demuxed)?;
                file.flush()?;
            }
            Ok(())
        };
        if chunked {
            while let Some(chunk) = read_chunk(&mut reader)? {
                write_payload(&chunk)?;
            }
        } else if let Some(n) = len {
            let mut body = vec![0_u8; n];
            reader.read_exact(&mut body)?;
            write_payload(&body)?;
        } else {
            let mut buf = [0_u8; 8192];
            loop {
                let n = match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => continue,
                    Err(error) if error.kind() == io::ErrorKind::TimedOut => continue,
                    Err(error) => return Err(error),
                };
                write_payload(&buf[..n])?;
            }
        }
        if !pending.is_empty() {
            file.write_all(&demux_docker_logs(&pending))?;
            file.flush()?;
        }
        Ok(())
    }

    /// Encode and send one request; returns the raw HTTP response.
    pub fn request(&self, method: &str, path: &str, body: Option<&Value>) -> io::Result<Response> {
        self.request_with_timeout(method, path, body, Duration::from_secs(30))
    }

    fn request_with_timeout(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        timeout: Duration,
    ) -> io::Result<Response> {
        let mut stream = UnixStream::connect(&self.socket)?;
        stream.set_read_timeout(Some(timeout))?;
        stream.set_write_timeout(Some(timeout))?;
        let bytes = encode_request(method, path, body);
        stream.write_all(&bytes)?;
        let mut reader = BufReader::new(stream);
        let (status, len, chunked) = read_head(&mut reader)?;
        let mut body = Vec::new();
        if status == 204 || status == 304 {
            return Ok(Response { status, body });
        }
        if chunked {
            while let Some(chunk) = read_chunk(&mut reader)? {
                body.extend_from_slice(&chunk);
            }
        } else if let Some(n) = len {
            body.resize(n, 0);
            reader.read_exact(&mut body)?;
        } else {
            reader.read_to_end(&mut body)?;
        }
        Ok(Response { status, body })
    }
}

/// Build the exact HTTP/1.1 request bytes the client would write (test aid).
pub fn encode_request(method: &str, path: &str, body: Option<&Value>) -> Vec<u8> {
    let payload = body.map(Value::to_string).unwrap_or_default();
    format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{payload}",
        payload.len()
    )
    .into_bytes()
}

/// HTTP/1.1 request that asks the daemon to upgrade into a raw TCP stream.
pub fn encode_upgrade_request(method: &str, path: &str, body: Option<&Value>) -> Vec<u8> {
    let payload = body.map(Value::to_string).unwrap_or_default();
    format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: Upgrade\r\nUpgrade: tcp\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{payload}",
        payload.len()
    )
    .into_bytes()
}

/// One demultiplexed output fragment from a hijacked exec attach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HijackOutput {
    /// Docker stream type 1 (or raw TTY bytes).
    Stdout(Vec<u8>),
    /// Docker stream type 2.
    Stderr(Vec<u8>),
}

/// Hijacked Docker exec attach stream (stdin + stdout/stderr on one socket).
#[derive(Debug)]
pub struct ExecHijack {
    /// Exec id returned by create (needed for inspect / resize).
    pub exec_id: String,
    /// Daemon socket used for follow-up HTTP calls on other connections.
    pub socket: std::path::PathBuf,
    stream: UnixStream,
    pending: Vec<u8>,
    /// When true, output is raw TTY bytes (not multiplexed frames).
    pub tty: bool,
}

impl ExecHijack {
    /// Read the next chunk of process output (raw when `tty`, else mux frames).
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.pending.is_empty() {
            let n = buf.len().min(self.pending.len());
            buf[..n].copy_from_slice(&self.pending[..n]);
            self.pending.drain(..n);
            return Ok(n);
        }
        match self.stream.read(buf) {
            Ok(n) => Ok(n),
            Err(error)
                if error.kind() == io::ErrorKind::WouldBlock
                    || error.kind() == io::ErrorKind::TimedOut =>
            {
                Ok(0)
            }
            Err(error) => Err(error),
        }
    }

    /// Read the next logical stdout/stderr chunk. `Ok(None)` is clean EOF.
    pub fn next_output(&mut self) -> io::Result<Option<HijackOutput>> {
        if self.tty {
            let mut buf = [0_u8; 8192];
            return match self.read(&mut buf)? {
                0 => Ok(None),
                n => Ok(Some(HijackOutput::Stdout(buf[..n].to_vec()))),
            };
        }
        loop {
            let Some(header) = self.read_exact(8)? else {
                return Ok(None);
            };
            let stream_type = header[0];
            let size = u32::from_be_bytes([header[4], header[5], header[6], header[7]]) as usize;
            let payload = match self.read_exact(size)? {
                Some(bytes) => bytes,
                None if size == 0 => Vec::new(),
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "truncated multiplexed exec frame",
                    ));
                }
            };
            match stream_type {
                1 => return Ok(Some(HijackOutput::Stdout(payload))),
                2 => return Ok(Some(HijackOutput::Stderr(payload))),
                // 0 = stdin (should not appear on the read side); skip unknowns.
                _ => continue,
            }
        }
    }

    fn read_exact(&mut self, need: usize) -> io::Result<Option<Vec<u8>>> {
        let mut out = Vec::with_capacity(need);
        while out.len() < need {
            let mut buf = vec![0_u8; need - out.len()];
            match self.read(&mut buf)? {
                0 if out.is_empty() => return Ok(None),
                0 => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "truncated hijack read",
                    ));
                }
                n => out.extend_from_slice(&buf[..n]),
            }
        }
        Ok(Some(out))
    }

    /// Clone the underlying socket for concurrent stdin writes.
    pub fn try_clone_writer(&self) -> io::Result<UnixStream> {
        self.stream.try_clone()
    }

    /// Half-close the write side so the guest sees EOF on stdin.
    pub fn shutdown_write(&self) -> io::Result<()> {
        self.stream.shutdown(std::net::Shutdown::Write)
    }

    /// Shut down both directions (best-effort cancel).
    pub fn shutdown_both(&self) -> io::Result<()> {
        self.stream.shutdown(std::net::Shutdown::Both)
    }
}

fn read_head(reader: &mut impl BufRead) -> io::Result<(u16, Option<usize>, bool)> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let status = line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "bad status line"))?;
    let (mut len, mut chunked) = (None, false);
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            match key.to_ascii_lowercase().as_str() {
                "content-length" => len = value.trim().parse().ok(),
                "transfer-encoding" => {
                    chunked = value.trim().eq_ignore_ascii_case("chunked");
                }
                _ => {}
            }
        }
    }
    Ok((status, len, chunked))
}

fn read_chunk(reader: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(None);
    }
    let n = usize::from_str_radix(line.trim().split(';').next().unwrap_or(""), 16)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad chunk"))?;
    if n == 0 {
        return Ok(None);
    }
    let mut buf = vec![0; n + 2];
    reader.read_exact(&mut buf)?;
    buf.truncate(n);
    Ok(Some(buf))
}

/// Docker multiplexed log frames → plain bytes (stdout+stderr payloads).
fn demux_docker_logs(input: &[u8]) -> Vec<u8> {
    take_demuxed_frames(input).0
}

/// Split multiplexed frames into stdout (stream 1) and stderr (stream 2).
fn demux_docker_streams(input: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut i = 0;
    while i + 8 <= input.len() {
        let stream_type = input[i];
        if stream_type > 2 {
            // Non-multiplexed fallback (TTY): treat the whole buffer as stdout.
            stdout.extend_from_slice(input);
            return (stdout, stderr);
        }
        let size =
            u32::from_be_bytes([input[i + 4], input[i + 5], input[i + 6], input[i + 7]]) as usize;
        let start = i + 8;
        let end = start + size;
        if end > input.len() {
            break;
        }
        match stream_type {
            1 => stdout.extend_from_slice(&input[start..end]),
            2 => stderr.extend_from_slice(&input[start..end]),
            _ => {}
        }
        i = end;
    }
    (stdout, stderr)
}

fn take_demuxed_frames(input: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 8 <= input.len() {
        let stream_type = input[i];
        if stream_type > 2 {
            // Non-multiplexed fallback (TTY): return the whole buffer.
            return (input.to_vec(), Vec::new());
        }
        let size =
            u32::from_be_bytes([input[i + 4], input[i + 5], input[i + 6], input[i + 7]]) as usize;
        let start = i + 8;
        let end = start + size;
        if end > input.len() {
            break;
        }
        out.extend_from_slice(&input[start..end]);
        i = end;
    }
    (out, input[i..].to_vec())
}

#[cfg(test)]
fn mux_frame(stream: u8, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(8 + payload.len());
    frame.push(stream);
    frame.extend_from_slice(&[0, 0, 0]);
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

/// Parse Docker-style `host:container[/proto]` publish specs into Engine API maps.
fn docker_port_spec(port_map: &[String]) -> (Value, Value) {
    let mut exposed = serde_json::Map::new();
    let mut bindings = serde_json::Map::new();
    for entry in port_map {
        let (host, rest) = match entry.rsplit_once(':') {
            Some(pair) => pair,
            None => continue,
        };
        let (container, proto) = match rest.split_once('/') {
            Some((c, p)) => (c, p),
            None => (rest, "tcp"),
        };
        if host.is_empty() || container.is_empty() {
            continue;
        }
        let key = format!("{container}/{proto}");
        exposed.insert(key.clone(), json!({}));
        bindings.insert(key, json!([{ "HostIp": "", "HostPort": host }]));
    }
    (Value::Object(exposed), Value::Object(bindings))
}

/// Split `repo`, `repo:tag`, or `host/repo:tag` into (fromImage, tag).
fn split_image_ref(image: &str) -> (&str, &str) {
    match image.rsplit_once(':') {
        Some((name, tag)) if !name.is_empty() && !tag.is_empty() && !tag.contains('/') => {
            (name, tag)
        }
        _ => (image, "latest"),
    }
}

fn urlencoding_minimal(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn path_segment(id: &str) -> String {
    urlencoding_minimal(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::os::unix::net::UnixListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    #[test]
    fn encodes_name_query_safely() {
        assert_eq!(urlencoding_minimal("abc-1"), "abc-1");
        assert_eq!(urlencoding_minimal("a b"), "a%20b");
    }

    #[test]
    fn encode_create_request_includes_image_cmd_env() {
        let body = json!({
            "Image": "alpine:latest",
            "Cmd": ["echo", "hi"],
            "Env": ["A=1"],
        });
        let bytes = encode_request("POST", "/containers/create?name=demo", Some(&body));
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.starts_with("POST /containers/create?name=demo HTTP/1.1\r\n"));
        assert!(text.contains("Content-Type: application/json\r\n"));
        assert!(text.contains("\"Image\":\"alpine:latest\""));
        assert!(text.contains("\"Cmd\":[\"echo\",\"hi\"]"));
        assert!(text.contains("\"Env\":[\"A=1\"]"));
    }

    #[test]
    fn ensure_reachable_fails_when_socket_absent() {
        let path = std::path::PathBuf::from(format!(
            "/tmp/kbox-no-dir-{}/missing.sock",
            std::process::id()
        ));
        assert!(!path.exists());
        let error = DockerHttpClient::new(&path).ensure_reachable().unwrap_err();
        assert!(
            error.contains("not present") || error.contains("unreachable"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn demux_docker_streams_splits_stdout_and_stderr() {
        let mut body = mux_frame(1, b"out-");
        body.extend_from_slice(&mux_frame(2, b"err"));
        body.extend_from_slice(&mux_frame(1, b"ok"));
        let (stdout, stderr) = demux_docker_streams(&body);
        assert_eq!(stdout, b"out-ok");
        assert_eq!(stderr, b"err");
    }

    #[test]
    fn encode_upgrade_request_asks_for_tcp_upgrade() {
        let bytes = encode_upgrade_request(
            "POST",
            "/exec/x/start",
            Some(&json!({"Detach": false, "Tty": true})),
        );
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("Connection: Upgrade\r\n"));
        assert!(text.contains("Upgrade: tcp\r\n"));
        assert!(!text.contains("Connection: close"));
    }

    #[test]
    fn fake_unix_http_container_exec_attach_mux() {
        let dir = std::env::temp_dir().join(format!(
            "ke{}att-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let socket = dir.join("d.sock");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0_u8; 16384];
            let n = stream.read(&mut buf).unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            assert!(request.starts_with("POST /containers/cidattach/exec"));
            assert!(request.contains("\"Tty\":false"));
            assert!(request.contains("\"AttachStdin\":true"));
            let body = br#"{"Id":"execattach01"}"#;
            let header = format!(
                "HTTP/1.1 201 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(header.as_bytes()).unwrap();
            stream.write_all(body).unwrap();

            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0_u8; 16384];
            let n = stream.read(&mut buf).unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            assert!(request.starts_with("POST /exec/execattach01/start"));
            assert!(request.contains("Upgrade: tcp"));
            assert!(request.contains("\"Tty\":false"));
            let mut payload = mux_frame(1, b"STREAM_");
            payload.extend_from_slice(&mux_frame(2, b"warn\n"));
            payload.extend_from_slice(&mux_frame(1, b"OK\n"));
            let mut response =
                b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\n"
                    .to_vec();
            response.extend_from_slice(&payload);
            stream.write_all(&response).unwrap();
            let mut got = Vec::new();
            let _ = stream.read_to_end(&mut got);
            assert!(got.windows(5).any(|w| w == b"stdin"));
        });

        let client = DockerHttpClient::new(&socket);
        let mut hijack = client
            .container_exec_attach("cidattach", &["sh".into()], None, None, None)
            .unwrap();
        assert_eq!(hijack.exec_id, "execattach01");
        assert!(!hijack.tty);
        assert_eq!(
            hijack.next_output().unwrap(),
            Some(HijackOutput::Stdout(b"STREAM_".to_vec()))
        );
        assert_eq!(
            hijack.next_output().unwrap(),
            Some(HijackOutput::Stderr(b"warn\n".to_vec()))
        );
        assert_eq!(
            hijack.next_output().unwrap(),
            Some(HijackOutput::Stdout(b"OK\n".to_vec()))
        );
        let mut writer = hijack.try_clone_writer().unwrap();
        writer.write_all(b"stdin").unwrap();
        hijack.shutdown_write().unwrap();
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fake_unix_http_container_exec_pty_hijack() {
        let dir = std::env::temp_dir().join(format!("ke{}p", std::process::id() % 100_000));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let socket = dir.join("d.sock");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        let server = thread::spawn(move || {
            // create
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0_u8; 16384];
            let n = stream.read(&mut buf).unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            assert!(request.starts_with("POST /containers/cidpty/exec"));
            assert!(request.contains("\"Tty\":true"));
            assert!(request.contains("\"AttachStdin\":true"));
            assert!(request.contains("\"ConsoleSize\":[24,80]"));
            let body = br#"{"Id":"execpty01"}"#;
            let header = format!(
                "HTTP/1.1 201 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(header.as_bytes()).unwrap();
            stream.write_all(body).unwrap();

            // start (upgrade) — leave connection open for hijack I/O
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0_u8; 16384];
            let n = stream.read(&mut buf).unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            assert!(request.starts_with("POST /exec/execpty01/start"));
            assert!(request.contains("Upgrade: tcp"));
            stream
                .write_all(
                    b"HTTP/1.1 101 Switching Protocols\r\nContent-Type: application/vnd.docker.raw-stream\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\nPTY_HIJACK_OK\n",
                )
                .unwrap();
            // Read stdin from client, then close.
            let mut got = Vec::new();
            let _ = stream.read_to_end(&mut got);
            assert!(got.windows(4).any(|w| w == b"ping"));
        });

        let client = DockerHttpClient::new(&socket);
        let mut hijack = client
            .container_exec_pty("cidpty", &["sh".into()], None, None, None, 80, 24)
            .unwrap();
        assert_eq!(hijack.exec_id, "execpty01");
        assert!(hijack.tty);
        let mut out = vec![0_u8; 64];
        let n = hijack.read(&mut out).unwrap();
        assert_eq!(&out[..n], b"PTY_HIJACK_OK\n");
        let mut writer = hijack.try_clone_writer().unwrap();
        writer.write_all(b"ping").unwrap();
        hijack.shutdown_write().unwrap();
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fake_unix_http_container_exec() {
        let dir = std::env::temp_dir().join(format!("ke{}e", std::process::id() % 100_000));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let socket = dir.join("d.sock");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let seen_thread = Arc::clone(&seen);
        let server = thread::spawn(move || {
            let mut answered = 0;
            while answered < 3 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buf = vec![0_u8; 8192];
                let n = match stream.read(&mut buf) {
                    Ok(0) | Err(_) => continue,
                    Ok(n) => n,
                };
                let request = String::from_utf8_lossy(&buf[..n]).into_owned();
                if !request.contains("HTTP/1.1") {
                    continue;
                }
                seen_thread.lock().unwrap().push(request.clone());
                let (status, body_bytes): (u16, Vec<u8>) =
                    if request.starts_with("POST /containers/cidexec/exec") {
                        assert!(request.contains("\"Cmd\":[\"echo\",\"hi\"]"));
                        assert!(request.contains("\"Env\":[\"A=1\"]"));
                        assert!(request.contains("\"User\":\"root\""));
                        assert!(request.contains("\"WorkingDir\":\"/tmp\""));
                        (201, br#"{"Id":"execdeadbeef"}"#.to_vec())
                    } else if request.starts_with("POST /exec/execdeadbeef/start") {
                        let mut payload = mux_frame(1, b"hello\n");
                        payload.extend_from_slice(&mux_frame(2, b"warn\n"));
                        (200, payload)
                    } else if request.starts_with("GET /exec/execdeadbeef/json") {
                        (
                            200,
                            br#"{"ID":"execdeadbeef","Running":false,"ExitCode":7}"#.to_vec(),
                        )
                    } else {
                        (500, br#"{"message":"unexpected"}"#.to_vec())
                    };
                let header = format!(
                    "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body_bytes.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(&body_bytes);
                answered += 1;
            }
        });

        let client = DockerHttpClient::new(&socket);
        let (code, stdout, stderr) = client
            .container_exec(
                "cidexec",
                &["echo".into(), "hi".into()],
                Some(&["A=1".into()]),
                Some("root"),
                Some("/tmp"),
                None,
            )
            .unwrap();
        assert_eq!(code, 7);
        assert_eq!(stdout, b"hello\n");
        assert_eq!(stderr, b"warn\n");

        server.join().unwrap();
        let requests = seen.lock().unwrap();
        assert!(requests[0].starts_with("POST /containers/cidexec/exec"));
        assert!(requests[0].contains("\"AttachStdin\":false"));
        assert!(requests[1].starts_with("POST /exec/execdeadbeef/start"));
        assert!(requests[2].starts_with("GET /exec/execdeadbeef/json"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fake_unix_http_container_exec_with_captured_stdin() {
        let dir = std::env::temp_dir().join(format!("ke{}i", std::process::id() % 100_000));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let socket = dir.join("d.sock");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0_u8; 8192];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            assert!(request.contains("/containers/cidstdin/exec"));
            assert!(request.contains("\"AttachStdin\":true"));
            assert!(request.contains("\"Cmd\":[\"sh\"]"));
            let body = br#"{"Id":"execstdin01"}"#;
            let header = format!(
                "HTTP/1.1 201 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(body);

            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0_u8; 8192];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            assert!(request.contains("/exec/execstdin01/start"));
            assert!(request.contains("Upgrade: tcp"));
            let mut payload = mux_frame(1, b"CAP_STDIN_OK\n");
            payload.extend_from_slice(&mux_frame(2, b""));
            let mut response =
                b"HTTP/1.1 101 Switching Protocols\r\nConnection: Upgrade\r\nUpgrade: tcp\r\n\r\n"
                    .to_vec();
            response.extend_from_slice(&payload);
            let _ = stream.write_all(&response);
            let mut got = Vec::new();
            let _ = stream.read_to_end(&mut got);
            assert!(
                got.windows(18).any(|w| w == b"echo CAP_STDIN_OK\n"),
                "expected captured stdin payload, got {got:?}"
            );
            let _ = stream.shutdown(std::net::Shutdown::Write);

            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0_u8; 8192];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            assert!(request.contains("/exec/execstdin01/json"));
            let body = br#"{"ID":"execstdin01","Running":false,"ExitCode":0}"#;
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(body);
        });

        let client = DockerHttpClient::new(&socket);
        let (code, stdout, stderr) = client
            .container_exec(
                "cidstdin",
                &["sh".into()],
                None,
                None,
                None,
                Some(b"echo CAP_STDIN_OK\n"),
            )
            .unwrap();
        assert_eq!(code, 0);
        assert_eq!(stdout, b"CAP_STDIN_OK\n");
        assert!(stderr.is_empty());
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn container_exec_rejects_oversized_stdin() {
        let client = DockerHttpClient::new("/tmp/missing-sandbox-exec.sock");
        let oversized = vec![0_u8; DockerHttpClient::MAX_CAPTURED_EXEC_STDIN_BYTES + 1];
        let error = client
            .container_exec("cid", &["true".into()], None, None, None, Some(&oversized))
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains("max is"));
    }

    #[test]
    fn fake_unix_http_network_ensure_connect_remove() {
        let dir = std::env::temp_dir().join(format!("ke{}n", std::process::id() % 100_000));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let socket = dir.join("d.sock");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let seen_thread = Arc::clone(&seen);
        let server = thread::spawn(move || {
            let mut answered = 0;
            while answered < 4 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buf = vec![0_u8; 8192];
                let n = match stream.read(&mut buf) {
                    Ok(0) | Err(_) => continue,
                    Ok(n) => n,
                };
                let request = String::from_utf8_lossy(&buf[..n]).into_owned();
                if !request.contains("HTTP/1.1") {
                    continue;
                }
                seen_thread.lock().unwrap().push(request.clone());
                let (status, body) = if request.starts_with("GET /networks/proj_default") {
                    (404_u16, r#"{"message":"network not found"}"#)
                } else if request.starts_with("POST /networks/create") {
                    assert!(request.contains("\"Name\":\"proj_default\""));
                    (201_u16, r#"{"Id":"netid","Warning":""}"#)
                } else if request.starts_with("POST /networks/proj_default/connect") {
                    assert!(request.contains("\"Container\":\"cid1\""));
                    assert!(request.contains("\"Aliases\":[\"web\"]"));
                    (200_u16, "null")
                } else if request.starts_with("DELETE /networks/proj_default") {
                    (204_u16, "")
                } else {
                    (500_u16, r#"{"message":"unexpected"}"#)
                };
                let response = format!(
                    "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                answered += 1;
            }
        });

        let client = DockerHttpClient::new(&socket);
        client.network_ensure("proj_default").unwrap();
        client
            .network_connect("proj_default", "cid1", &["web".into()])
            .unwrap();
        client.network_remove("proj_default").unwrap();
        server.join().unwrap();
        let requests = seen.lock().unwrap();
        assert!(requests[0].starts_with("GET /networks/proj_default"));
        assert!(requests[1].starts_with("POST /networks/create"));
        assert!(requests[2].starts_with("POST /networks/proj_default/connect"));
        assert!(requests[3].starts_with("DELETE /networks/proj_default"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn container_create_with_network_sets_host_config_and_aliases() {
        let dir = std::env::temp_dir().join(format!("ke{}a", std::process::id() % 100_000));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let socket = dir.join("d.sock");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let seen_thread = Arc::clone(&seen);
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0_u8; 8192];
            let n = stream.read(&mut buf).unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).into_owned();
            seen_thread.lock().unwrap().push(request);
            let body = r#"{"Id":"cidnet"}"#;
            let response = format!(
                "HTTP/1.1 201 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        });
        let client = DockerHttpClient::new(&socket);
        let id = client
            .container_create_with_network(
                "alpine:latest",
                &["sleep".into(), "inf".into()],
                &[],
                None,
                Some("box-1"),
                &[],
                &[],
                Some("proj_default"),
                &["web".into(), "proj-web".into()],
            )
            .unwrap();
        assert_eq!(id, "cidnet");
        server.join().unwrap();
        let request = &seen.lock().unwrap()[0];
        assert!(request.contains("\"NetworkMode\":\"proj_default\""));
        assert!(request.contains("\"Aliases\":[\"web\",\"proj-web\"]"));
        assert!(request.contains("\"NetworkingConfig\""));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fake_unix_http_create_start_inspect() {
        // Keep the path short: AF_UNIX sun_path is small on macOS.
        let dir = std::env::temp_dir().join(format!("ke{}c", std::process::id() % 100_000));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let socket = dir.join("d.sock");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let seen_thread = Arc::clone(&seen);
        let server = thread::spawn(move || {
            // Skip connect-only probes (ensure_reachable); answer three HTTP calls.
            let mut answered = 0;
            while answered < 3 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buf = vec![0_u8; 8192];
                let n = match stream.read(&mut buf) {
                    Ok(0) | Err(_) => continue,
                    Ok(n) => n,
                };
                let request = String::from_utf8_lossy(&buf[..n]).into_owned();
                if !request.contains("HTTP/1.1") {
                    continue;
                }
                seen_thread.lock().unwrap().push(request.clone());
                let (status, body) = if request.starts_with("POST /containers/create") {
                    (201_u16, r#"{"Id":"ciddeadbeef"}"#)
                } else if request.starts_with("POST /containers/ciddeadbeef/start") {
                    (204_u16, "")
                } else if request.starts_with("GET /containers/ciddeadbeef/json") {
                    (
                        200_u16,
                        r#"{"Id":"ciddeadbeef","State":{"Running":true,"Paused":false,"ExitCode":0,"Pid":4242}}"#,
                    )
                } else {
                    (500_u16, r#"{"message":"unexpected"}"#)
                };
                let response = format!(
                    "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                answered += 1;
            }
        });

        let client = DockerHttpClient::new(&socket);
        client.ensure_reachable().unwrap();
        let id = client
            .container_create(
                "alpine:latest",
                &["true".into()],
                &[("K".into(), "V".into())],
                None,
                Some("box-demo"),
                &[],
                &[],
            )
            .unwrap();
        assert_eq!(id, "ciddeadbeef");
        client.container_start(&id).unwrap();
        let inspect = client.container_inspect(&id).unwrap();
        assert_eq!(inspect["State"]["Running"], true);

        server.join().unwrap();
        let requests = seen.lock().unwrap();
        assert!(requests[0].contains("POST /containers/create?name=box-demo"));
        assert!(requests[0].contains("\"Image\":\"alpine:latest\""));
        assert!(requests[1].starts_with("POST /containers/ciddeadbeef/start"));
        assert!(requests[2].starts_with("GET /containers/ciddeadbeef/json"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn container_set_ephemeral_env_posts_kbox_path() {
        let dir = std::env::temp_dir().join(format!("ke{}eph", std::process::id() % 100_000));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let socket = dir.join("e.sock");
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        let seen = Arc::new(Mutex::new(String::new()));
        let seen_thread = Arc::clone(&seen);
        let server = thread::spawn(move || {
            loop {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buf = vec![0_u8; 8192];
                let n = match stream.read(&mut buf) {
                    Ok(0) | Err(_) => continue,
                    Ok(n) => n,
                };
                let request = String::from_utf8_lossy(&buf[..n]).into_owned();
                if !request.contains("HTTP/1.1") {
                    continue;
                }
                *seen_thread.lock().unwrap() = request;
                let response =
                    "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                let _ = stream.write_all(response.as_bytes());
                break;
            }
        });

        let client = DockerHttpClient::new(&socket);
        client
            .container_set_ephemeral_env(
                "cidsecret01",
                &[("API_TOKEN".into(), "fixture-only".into())],
            )
            .unwrap();
        server.join().unwrap();
        let request = seen.lock().unwrap();
        assert!(request.starts_with("POST /kbox/containers/cidsecret01/ephemeral-env"));
        assert!(request.contains("\"env\":[\"API_TOKEN=fixture-only\"]"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
