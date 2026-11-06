use std::process::ExitCode;

fn main() -> ExitCode {
    let result = match koci_agent::run_internal_container_init() {
        Some(result) => result,
        None => koci_agent::mount_runtime_share_if_requested()
            .and_then(|runtime_parent| {
                koci_agent::take_vm_attachment_manifest(runtime_parent.as_deref())
                    .map(|attachments| (runtime_parent, attachments))
            })
            .and_then(|(runtime_parent, attachments)| {
                koci_agent::take_session_token_with_security()
                    .map(|(token, security)| (runtime_parent, attachments, token, security))
            })
            .and_then(|(runtime_parent, attachments, token, security)| {
                koci_agent::take_transport_qualification_request().and_then(|qualification| match (
                    qualification,
                    attachments,
                ) {
                    (Some(_), Some(_)) => Err(koci_sdk::Error::new(
                        koci_sdk::ErrorCode::FailedPrecondition,
                        "transport qualification cannot consume production VM attachments",
                    )
                    .for_operation("bootstrap-guest-agent")),
                    (Some(request), None) => koci_agent::run_transport_qualification_with_security(
                        token,
                        request,
                        runtime_parent,
                        security,
                    ),
                    (None, attachments) => koci_agent::run_with_vm_attachments_and_security(
                        token,
                        runtime_parent,
                        attachments,
                        security,
                    ),
                })
            }),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("koci-agent: {error}");
            ExitCode::FAILURE
        }
    }
}
