//! # BCS Shield — Islamic Fintech Cryptographic Security Platform
//!
//! A production-grade REST API server that exposes BCS-521 Fortress
//! cryptography as a service for Islamic fintech applications.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │           Islamic Fintech Apps              │
//! │  (Banking, Zakat, Waqf, Takaful, Sukuk)    │
//! └──────────────────┬──────────────────────────┘
//!                    │ HTTPS / REST API
//! ┌──────────────────▼──────────────────────────┐
//! │              BCS Shield API                 │
//! │  ┌─────────┐ ┌──────────┐ ┌──────────────┐ │
//! │  │ Key Mgmt│ │ Crypto   │ │ Shariah Audit│ │
//! │  │ Service │ │ Service  │ │   Service    │ │
//! │  └─────────┘ └──────────┘ └──────────────┘ │
//! │  ┌─────────────────────────────────────────┐ │
//! │  │         Fortress Security Layer         │ │
//! │  │  DPA Mask │ Fault Resist │ Agg Zeroize  │ │
//! │  │  PQ Hybrid │ CT Ladder │ Exec Proofs   │ │
//! │  └─────────────────────────────────────────┘ │
//! │  ┌─────────────────────────────────────────┐ │
//! │  │         BCS-521 Core (Rust)             │ │
//! │  └─────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────┘
//! ```

mod api;
mod crypto_service;
mod key_store;
mod shariah_audit;
mod models;

use actix_web::{web, App, HttpServer, middleware};
use actix_cors::Cors;
use tracing_subscriber::EnvFilter;

use api::*;

const SHIELD_VERSION: &str = "0.1.0-fortress";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("bcs_shield=info".parse().unwrap()))
        .init();

    let bind_addr = "0.0.0.0:8443";
    tracing::info!("🕌 BCS Shield v{} starting on {}", SHIELD_VERSION, bind_addr);
    tracing::info!("🛡️  Fortress: DPA + Fault + PQ Hybrid + Aggressive Zeroize");
    tracing::info!("📖 API docs: http://{}/swagger-ui/", bind_addr);

    // Shared state
    let key_store = web::Data::new(key_store::KeyStore::new());
    let audit_log = web::Data::new(shariah_audit::AuditLog::new());

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .app_data(key_store.clone())
            .app_data(audit_log.clone())
            // Health & info
            .route("/api/v1/health", web::get().to(health))
            .route("/api/v1/info", web::get().to(shield_info))
            // Key management
            .route("/api/v1/keys/generate", web::post().to(generate_key))
            .route("/api/v1/keys/{id}", web::get().to(get_key_info))
            .route("/api/v1/keys", web::get().to(list_keys))
            .route("/api/v1/keys/{id}", web::delete().to(revoke_key))
            // Cryptographic operations
            .route("/api/v1/crypto/sign", web::post().to(sign_message))
            .route("/api/v1/crypto/verify", web::post().to(verify_signature))
            .route("/api/v1/crypto/ecdh", web::post().to(ecdh_key_agreement))
            .route("/api/v1/crypto/hybrid-encaps", web::post().to(hybrid_encaps))
            .route("/api/v1/crypto/hybrid-decaps", web::post().to(hybrid_decaps))
            // Shariah audit
            .route("/api/v1/audit/log", web::get().to(get_audit_log))
            .route("/api/v1/audit/proof/{id}", web::get().to(get_execution_proof))
            .route("/api/v1/audit/compliance", web::get().to(compliance_report))
            // Swagger UI
            .service(
                utoipa_swagger_ui::SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", api::openapi_spec())
            )
    })
    .bind(bind_addr)?
    .run()
    .await
}
