use crate::{logging::LoggingConfig, trigger::Triggers};

#[allow(non_snake_case)]
pub struct HypetriggerConfig {
    pub inputPath: String,
    pub samplesPerSecond: f64,
    pub triggers: Triggers,
    pub logging: LoggingConfig,
}
