use super::analyzer::AnalysisOperationRepr;
use super::Result;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::Arc;

use crate::proto::ExperimentRunBase;
use crate::simulation::packages::output::packages::analysis::analyzer::AnalysisSourceRepr;
use crate::simulation::packages::output::packages::analysis::get_analysis_source;
use crate::ExperimentConfig;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct AnalysisOutputConfig {
    pub outputs: HashMap<Arc<String>, Vec<AnalysisOperationRepr>>,
    pub manifest: String,
}

impl AnalysisOutputConfig {
    pub fn new(config: &ExperimentConfig<ExperimentRunBase>) -> Result<AnalysisOutputConfig> {
        let manifest = get_analysis_source(&config.run.project_base.packages)?;
        let analysis_src_repr = AnalysisSourceRepr::try_from(&manifest as &str)?;
        Ok(AnalysisOutputConfig {
            outputs: analysis_src_repr.outputs,
            manifest,
        })
    }
}
