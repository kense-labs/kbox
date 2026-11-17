//! `kbox shell` command — Open an interactive shell in a running box.
//!
//! Convenience wrapper around `exec -it <box> -- <shell>`.
//! Equivalent to: `kbox exec -it <box> -- /bin/sh`
//!
//! Spec050 Sandbox boxes have no guest PTY socket; routing through `exec`
//! uses the managed `start_pty` path (fused-engine Docker exec hijack).

use clap::Args;

use super::exec::{self, ExecArgs};

#[derive(Args)]
pub struct ShellArgs {
    /// Box name or ID
    pub r#box: String,

    /// Shell to launch (default: /bin/sh)
    #[arg(long, default_value = "/bin/sh")]
    pub shell: String,

    /// Run as a specific user (supported: root, UID, UID:GID)
    #[arg(short = 'u', long)]
    pub user: Option<String>,

    /// Working directory inside the box
    #[arg(short = 'w', long)]
    pub workdir: Option<String>,
}

pub async fn execute(args: ShellArgs) -> Result<(), Box<dyn std::error::Error>> {
    exec::execute(ExecArgs {
        r#box: args.r#box,
        cmd: vec![args.shell],
        interactive: true,
        tty: true,
        envs: vec![],
        user: args.user,
        workdir: args.workdir,
        timeout: 5,
        request_id: None,
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_args_defaults() {
        let args = ShellArgs {
            r#box: "mybox".to_string(),
            shell: "/bin/sh".to_string(),
            user: None,
            workdir: None,
        };
        assert_eq!(args.shell, "/bin/sh");
        assert!(args.user.is_none());
        assert!(args.workdir.is_none());
    }

    #[test]
    fn test_shell_args_custom() {
        let args = ShellArgs {
            r#box: "mybox".to_string(),
            shell: "/bin/bash".to_string(),
            user: Some("root".to_string()),
            workdir: Some("/workspace".to_string()),
        };
        assert_eq!(args.shell, "/bin/bash");
        assert_eq!(args.user.as_deref(), Some("root"));
        assert_eq!(args.workdir.as_deref(), Some("/workspace"));
    }
}
