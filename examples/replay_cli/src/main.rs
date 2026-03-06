use of_adapters::{AdapterConfig, ProviderKind};
use of_core::SymbolId;
use of_runtime::{build_default_engine, EngineConfig};

fn main() {
    let symbol = SymbolId {
        venue: "CME".to_string(),
        symbol: "ESM6".to_string(),
    };

    let mut engine = build_default_engine(EngineConfig {
        instance_id: "replay-cli".to_string(),
        enable_persistence: false,
        data_root: "data".to_string(),
        audit_log_path: "audit/replay_cli.log".to_string(),
        audit_max_bytes: 5 * 1024 * 1024,
        audit_max_files: 3,
        audit_redact_tokens: vec![
            "secret".to_string(),
            "password".to_string(),
            "token".to_string(),
            "api_key".to_string(),
        ],
        data_retention_max_bytes: 10 * 1024 * 1024,
        data_retention_max_age_secs: 24 * 60 * 60,
        adapter: AdapterConfig {
            provider: ProviderKind::Mock,
            ..AdapterConfig::default()
        },
        signal_threshold: 100,
    })
    .expect("engine creation failed");

    engine.start().expect("engine start failed");
    engine
        .subscribe(symbol.clone(), 10)
        .expect("subscribe failed");
    println!("{}", engine.metrics_json());
}
