//! Shared test helpers for integration and E2E tests.
//!
//! # Overview
//!
//! This module provides container-startup helpers built on top of [`testcontainers`].
//! Each helper returns a [`ContainerAsync`] handle that the *caller* owns.  The
//! container is automatically stopped and removed when the handle is dropped, so
//! tests are fully self-contained.
//!
//! # Usage
//!
//! ```rust,ignore
//! mod common;
//!
//! #[tokio::test]
//! async fn my_test() {
//!     let _container = common::start_generic_container().await;
//!     // … assertions …
//! }   // container drops here → Docker removes it automatically
//! ```
//!
//! # Extending
//!
//! Add new helpers in this file for each service type your tests need
//! (e.g. `start_postgres()`, `start_redis()`).  Keep helpers focused: one
//! function per service image, returning the typed container handle.

use testcontainers::{ContainerAsync, GenericImage, core::WaitFor, runners::AsyncRunner};

/// Start a lightweight `hello-world` container and return the handle.
///
/// The container is ready when Docker's "Hello from Docker!" message appears on
/// stdout.  Ownership of the returned handle gives the caller full control over
/// the container lifecycle: the container is removed as soon as the handle is
/// dropped (at the end of the test function).
///
/// # Panics
///
/// Panics if Docker is unavailable or the image cannot be pulled.  The panic
/// message from testcontainers identifies the root cause clearly.
pub async fn start_generic_container() -> ContainerAsync<GenericImage> {
    GenericImage::new("hello-world", "latest")
        .with_wait_for(WaitFor::message_on_stdout("Hello from Docker!"))
        .start()
        .await
        .expect("failed to start hello-world container — is Docker running?")
}
