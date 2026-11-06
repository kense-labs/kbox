use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, Method, Request, Response, StatusCode};
use axum::routing::any;
use axum::Router;
use base64::Engine;
use sha2::{Digest, Sha256};

#[derive(Clone)]
struct StoredManifest {
    bytes: Vec<u8>,
    media_type: String,
    digest: String,
}

#[derive(Clone)]
struct RegistryState {
    base_url: String,
    manifests: Arc<Mutex<HashMap<String, StoredManifest>>>,
    blobs: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    uploads: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    next_upload: Arc<Mutex<u64>>,
    expected_authorization: Arc<Mutex<Option<String>>>,
}

pub struct TestRegistry {
    address: String,
    state: RegistryState,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl TestRegistry {
    pub fn start() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind OCI registry");
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let state = RegistryState {
            base_url: format!("http://{address}"),
            manifests: Arc::new(Mutex::new(HashMap::new())),
            blobs: Arc::new(Mutex::new(HashMap::new())),
            uploads: Arc::new(Mutex::new(HashMap::new())),
            next_upload: Arc::new(Mutex::new(1)),
            expected_authorization: Arc::new(Mutex::new(None)),
        };
        let app = Router::new()
            .route("/{*path}", any(registry_handler))
            .with_state(state.clone());
        let (shutdown, receiver) = tokio::sync::oneshot::channel();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("registry runtime");
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                axum::serve(listener, app.into_make_service())
                    .with_graceful_shutdown(async {
                        let _ = receiver.await;
                    })
                    .await
                    .expect("serve OCI registry");
            });
        });
        Self {
            address,
            state,
            shutdown: Some(shutdown),
            thread: Some(thread),
        }
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn reference(&self, repository: &str, tag: &str) -> String {
        format!("{}/{repository}:{tag}", self.address)
    }

    pub fn require_basic_auth(&self, username: &str, password: &str) {
        let credentials =
            base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
        *self.state.expected_authorization.lock().unwrap() = Some(format!("Basic {credentials}"));
    }

    pub fn seed_oci_tar(&self, repository: &str, tag: &str, archive: &Path) -> String {
        let layout = tempfile::tempdir().expect("extract OCI seed");
        tar::Archive::new(std::fs::File::open(archive).expect("open OCI seed"))
            .unpack(layout.path())
            .expect("unpack OCI seed");
        let index = read_json(&layout.path().join("index.json"));
        let descriptor = &index["manifests"][0];
        let digest = descriptor["digest"].as_str().unwrap().to_string();
        let manifest_bytes = read_blob(layout.path(), &digest);
        let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes).unwrap();
        let media_type = manifest["mediaType"]
            .as_str()
            .or(descriptor["mediaType"].as_str())
            .unwrap_or("application/vnd.oci.image.manifest.v1+json")
            .to_string();

        let mut blob_digests = vec![manifest["config"]["digest"].as_str().unwrap().to_string()];
        blob_digests.extend(
            manifest["layers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|layer| layer["digest"].as_str().unwrap().to_string()),
        );
        let mut blobs = self.state.blobs.lock().unwrap();
        for blob_digest in blob_digests {
            let bytes = read_blob(layout.path(), &blob_digest);
            blobs.insert(format!("/v2/{repository}/blobs/{blob_digest}"), bytes);
        }
        drop(blobs);

        let stored = StoredManifest {
            bytes: manifest_bytes,
            media_type,
            digest: digest.clone(),
        };
        let mut manifests = self.state.manifests.lock().unwrap();
        manifests.insert(format!("/v2/{repository}/manifests/{tag}"), stored.clone());
        manifests.insert(format!("/v2/{repository}/manifests/{digest}"), stored);
        digest
    }

    pub fn manifest(&self, repository: &str, reference: &str) -> Option<Vec<u8>> {
        self.state
            .manifests
            .lock()
            .unwrap()
            .get(&format!("/v2/{repository}/manifests/{reference}"))
            .map(|manifest| manifest.bytes.clone())
    }
}

impl Drop for TestRegistry {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

async fn registry_handler(
    State(state): State<RegistryState>,
    request: Request<Body>,
) -> Response<Body> {
    let (parts, body) = request.into_parts();
    let method = parts.method;
    let path = parts.uri.path().to_string();
    let query = parts.uri.query().unwrap_or_default().to_string();

    if method == Method::GET && path == "/v2/" {
        return response(StatusCode::OK, Body::empty());
    }
    if let Some(expected) = state.expected_authorization.lock().unwrap().clone() {
        let actual = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok());
        if actual != Some(expected.as_str()) {
            return Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(header::WWW_AUTHENTICATE, "Basic realm=\"kbox-test\"")
                .body(Body::empty())
                .unwrap();
        }
    }

    if let Some(manifest) = state.manifests.lock().unwrap().get(&path).cloned() {
        if method == Method::GET || method == Method::HEAD {
            let body = if method == Method::HEAD {
                Body::empty()
            } else {
                Body::from(manifest.bytes)
            };
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, manifest.media_type)
                .header("docker-content-digest", manifest.digest)
                .body(body)
                .unwrap();
        }
    }

    if let Some(bytes) = state.blobs.lock().unwrap().get(&path).cloned() {
        if method == Method::GET || method == Method::HEAD {
            let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
            let body = if method == Method::HEAD {
                Body::empty()
            } else {
                Body::from(bytes)
            };
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .header("docker-content-digest", digest)
                .body(body)
                .unwrap();
        }
    }

    if method == Method::POST && path.ends_with("/blobs/uploads/") {
        let mut next = state.next_upload.lock().unwrap();
        let id = next.to_string();
        *next += 1;
        drop(next);
        let upload_path = format!("{path}{id}");
        state.uploads.lock().unwrap().insert(id, Vec::new());
        return Response::builder()
            .status(StatusCode::ACCEPTED)
            .header(header::LOCATION, format!("{}{upload_path}", state.base_url))
            .body(Body::empty())
            .unwrap();
    }

    let body_bytes = axum::body::to_bytes(body, 64 * 1024 * 1024)
        .await
        .unwrap_or_default()
        .to_vec();
    if let Some(upload_path) = path
        .strip_prefix("/v2/")
        .and_then(|path| path.split_once("/blobs/uploads/"))
        .map(|_| path.clone())
    {
        let id = upload_path
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_string();
        if state.uploads.lock().unwrap().contains_key(&id) {
            let bytes = body_bytes.clone();
            state
                .uploads
                .lock()
                .unwrap()
                .get_mut(&id)
                .unwrap()
                .extend_from_slice(&bytes);
            if method == Method::PATCH {
                return Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .header(header::LOCATION, format!("{}{}", state.base_url, path))
                    .body(Body::empty())
                    .unwrap();
            }
            if method == Method::PUT {
                let digest = query
                    .split('&')
                    .find_map(|part| part.strip_prefix("digest="))
                    .expect("upload digest")
                    .replace("%3A", ":");
                let bytes = state.uploads.lock().unwrap().remove(&id).unwrap();
                assert_eq!(digest, format!("sha256:{:x}", Sha256::digest(&bytes)));
                let repository = path
                    .strip_prefix("/v2/")
                    .and_then(|value| value.split_once("/blobs/uploads/"))
                    .map(|(repository, _)| repository)
                    .unwrap();
                let blob_path = format!("/v2/{repository}/blobs/{digest}");
                state.blobs.lock().unwrap().insert(blob_path, bytes);
                return Response::builder()
                    .status(StatusCode::CREATED)
                    .header(
                        header::LOCATION,
                        format!("{}/v2/{repository}/blobs/{digest}", state.base_url),
                    )
                    .header("docker-content-digest", &digest)
                    .body(Body::empty())
                    .unwrap();
            }
        }
    }

    if method == Method::PUT && path.contains("/manifests/") {
        let bytes = body_bytes;
        let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
        let media_type = parts
            .headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("application/vnd.oci.image.manifest.v1+json")
            .to_string();
        let stored = StoredManifest {
            bytes,
            media_type,
            digest: digest.clone(),
        };
        let repository = path
            .strip_prefix("/v2/")
            .and_then(|value| value.split_once("/manifests/"))
            .map(|(repository, _)| repository)
            .unwrap();
        let mut manifests = state.manifests.lock().unwrap();
        manifests.insert(path.clone(), stored.clone());
        manifests.insert(format!("/v2/{repository}/manifests/{digest}"), stored);
        return Response::builder()
            .status(StatusCode::CREATED)
            .header(
                header::LOCATION,
                format!("{}/v2/{repository}/manifests/{digest}", state.base_url),
            )
            .header("docker-content-digest", digest)
            .body(Body::empty())
            .unwrap();
    }

    response(StatusCode::NOT_FOUND, Body::empty())
}

fn response(status: StatusCode, body: Body) -> Response<Body> {
    Response::builder().status(status).body(body).unwrap()
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(path).expect("read OCI JSON")).expect("parse OCI JSON")
}

fn read_blob(root: &Path, digest: &str) -> Vec<u8> {
    let (algorithm, hex) = digest.split_once(':').unwrap();
    let bytes = std::fs::read(root.join("blobs").join(algorithm).join(hex)).expect("read OCI blob");
    assert_eq!(digest, format!("{algorithm}:{:x}", Sha256::digest(&bytes)));
    bytes
}
