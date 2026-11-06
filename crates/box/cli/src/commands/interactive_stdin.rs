//! Host-side forwarding into the shim's interactive console socket.

#[cfg(unix)]
pub fn spawn_forwarder(
    console_log: &std::path::Path,
) -> Result<std::thread::JoinHandle<std::io::Result<u64>>, Box<dyn std::error::Error>> {
    let path = console_log.with_file_name("stdin.sock");
    let mut writer = std::os::unix::net::UnixStream::connect(&path).map_err(|error| {
        format!(
            "Failed to connect interactive stdin socket {}: {error}",
            path.display()
        )
    })?;
    let mut input = std::io::stdin();
    Ok(std::thread::spawn(move || {
        std::io::copy(&mut input, &mut writer)
    }))
}
