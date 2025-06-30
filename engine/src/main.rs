// Engine main entry point
// use engine::config::settings::EngineSettings; // No longer needed directly
use engine::services::trading_service::MyTradingEngine;
use engine::services::TradingEngineServer; // Import the generated server type
use engine::data::market_data::MarketDataStore;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Server;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber for logging
    // Use a simple subscriber for now, can be configured further (e.g., with json output, filtering)
    tracing_subscriber::fmt::init();

    info!("Starting Home Trader Engine...");

    // Load configuration using the new utility function
    let settings = engine::config::settings::get_engine_settings(); // Use the new function
    let addr = format!("{}:{}", settings.host, settings.port).parse()?;
    info!("Engine will listen on {} (Host: {}, Port: {})", addr, settings.host, settings.port);

    // Initialize shared data stores or services
    let market_data_store = Arc::new(RwLock::new(MarketDataStore::new()));

    // Create an instance of the trading service
    let trading_engine_service = MyTradingEngine::new(market_data_store.clone());

    // Build and start the gRPC server
    Server::builder()
        .add_service(TradingEngineServer::new(trading_engine_service))
        .serve(addr)
        .await?;

    Ok(())
}
