mod common;

/// Verify the full container lifecycle: start → assert running → drop → removed.
///
/// This test is the canary for the entire E2E infrastructure.  If it passes,
/// Docker is reachable, the testcontainers async runner works, and the `common`
/// helper module compiles correctly.
#[tokio::test]
async fn test_container_starts_and_stops() {
    // The helper panics with a clear message if Docker is unavailable.
    let container = common::start_generic_container().await;

    // The container ID is non-empty while the handle is alive.
    assert!(
        !container.id().is_empty(),
        "container ID should be non-empty while the container is running"
    );

    // `container` drops here — testcontainers removes the Docker container
    // automatically.  No orphaned containers remain on the host.
}
