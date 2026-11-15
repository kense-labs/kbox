//! One-shot reconciliation of an exact sentinel-owned failed native candidate.
use koci_sdk::{
    ContainerId, ContainerTarget, DeleteMode, DeleteRequest, DriverKind, Generation,
    IsolationClass, ListRequest, LocalIpcEndpoint, OperationContext, OperationId, RuntimeClient,
    StateRequest,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 4);
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let target = ContainerTarget::exact(ContainerId::new(&args[1])?, Generation(1));
        let client = RuntimeClient::connect(&LocalIpcEndpoint::unix_socket(&args[0])?).await?;
        let before = client.list(ListRequest::default()).await?;
        assert_eq!(before.len(), 1);
        let current = client.state(StateRequest { target: target.clone() }).await?;
        assert_eq!(current.state.id(), target.id.as_str());
        assert_eq!(current.generation, target.generation.unwrap());
        assert_eq!(current.driver, DriverKind::LibkHvf);
        assert_eq!(current.isolation, IsolationClass::DedicatedVm);
        assert_eq!(current.config_digest.as_str(), args[2]);
        assert!(matches!(current.state.status(),
            koci_sdk::oci_spec::runtime::ContainerState::Created |
            koci_sdk::oci_spec::runtime::ContainerState::Stopped));
        client.delete(DeleteRequest {
            context: OperationContext::new(OperationId::new("owned-failed-candidate-force-delete")?),
            target: target.clone(), mode: DeleteMode::Force,
        }).await?;
        assert!(client.list(ListRequest::default()).await?.is_empty());
        std::fs::write(&args[3], serde_json::to_vec_pretty(&serde_json::json!({
            "status":"PASS", "exactTarget":target, "before":before,
            "observed":current, "after":[], "startInvoked":false
        }))?)?;
        Ok::<_, Box<dyn std::error::Error>>(())
    })
}
