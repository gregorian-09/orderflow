use super::*;

/// Creates a provider adapter from configuration.
pub fn create_adapter(cfg: &AdapterConfig) -> AdapterResult<Box<dyn MarketDataAdapter>> {
    match &cfg.provider {
        ProviderKind::Mock => Ok(Box::new(MockAdapter::default())),
        ProviderKind::Rithmic => create_rithmic_adapter(cfg),
        ProviderKind::Cqg => create_cqg_adapter(cfg),
        ProviderKind::Binance => create_binance_adapter(cfg),
    }
}

fn create_rithmic_adapter(cfg: &AdapterConfig) -> AdapterResult<Box<dyn MarketDataAdapter>> {
    #[cfg(feature = "rithmic")]
    {
        let adapter = rithmic::RithmicAdapter::from_config(cfg)?;
        Ok(Box::new(adapter))
    }

    #[cfg(not(feature = "rithmic"))]
    {
        let _ = cfg;
        Err(AdapterError::FeatureDisabled(
            "compile with --features rithmic to enable",
        ))
    }
}

fn create_cqg_adapter(cfg: &AdapterConfig) -> AdapterResult<Box<dyn MarketDataAdapter>> {
    #[cfg(feature = "cqg")]
    {
        let adapter = cqg::CqgAdapter::from_config(cfg)?;
        Ok(Box::new(adapter))
    }

    #[cfg(not(feature = "cqg"))]
    {
        let _ = cfg;
        Err(AdapterError::FeatureDisabled(
            "compile with --features cqg to enable",
        ))
    }
}

fn create_binance_adapter(cfg: &AdapterConfig) -> AdapterResult<Box<dyn MarketDataAdapter>> {
    #[cfg(feature = "binance")]
    {
        let adapter = binance::BinanceAdapter::from_config(cfg)?;
        Ok(Box::new(adapter))
    }

    #[cfg(not(feature = "binance"))]
    {
        let _ = cfg;
        Err(AdapterError::FeatureDisabled(
            "compile with --features binance to enable",
        ))
    }
}
