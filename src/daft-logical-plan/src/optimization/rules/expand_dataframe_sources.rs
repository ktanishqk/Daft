//! Replace a DataSource leaf with the LogicalPlan from `get_dataframe`.

use std::sync::Arc;

#[cfg(feature = "python")]
use common_error::DaftError;
use common_error::DaftResult;
use common_treenode::Transformed;
#[cfg(feature = "python")]
use common_treenode::TreeNode;
#[cfg(feature = "python")]
use daft_scan::ScanState;

use super::OptimizerRule;
use crate::LogicalPlan;
#[cfg(feature = "python")]
use crate::{ops::Filter, ops::Project, SourceInfo};

#[derive(Default, Debug)]
pub struct ExpandDataFrameSources {}

impl ExpandDataFrameSources {
    pub fn new() -> Self {
        Self {}
    }
}

impl OptimizerRule for ExpandDataFrameSources {
    fn try_optimize(&self, plan: Arc<LogicalPlan>) -> DaftResult<Transformed<Arc<LogicalPlan>>> {
        #[cfg(feature = "python")]
        {
            return plan.transform_down(|node| self.try_optimize_node(node));
        }
        #[cfg(not(feature = "python"))]
        {
            Ok(Transformed::no(plan))
        }
    }
}

#[cfg(feature = "python")]
impl ExpandDataFrameSources {
    fn try_optimize_node(
        &self,
        plan: Arc<LogicalPlan>,
    ) -> DaftResult<Transformed<Arc<LogicalPlan>>> {
        let LogicalPlan::Source(source) = plan.as_ref() else {
            return Ok(Transformed::no(plan));
        };
        let SourceInfo::Physical(physical) = source.source_info.as_ref() else {
            return Ok(Transformed::no(plan));
        };
        let ScanState::Operator(scan_op) = &physical.scan_state else {
            return Ok(Transformed::no(plan));
        };
        let Some(expander) = scan_op.0.as_dataframe_expander() else {
            return Ok(Transformed::no(plan));
        };

        let Some(py_df) = expander.expand_dataframe(&physical.pushdowns)? else {
            return Ok(Transformed::no(plan));
        };

        let source_name = scan_op.0.name().to_string();
        let mut inner = logical_plan_from_py_dataframe(&py_df, &source_name)?;

        if let Some(predicate) = physical.pushdowns.filters.clone() {
            inner = Filter::try_new(inner, predicate)
                .map_err(|e| {
                    DaftError::ValueError(format!(
                        "DataSource '{source_name}' get_dataframe() plan cannot apply a pushed filter: {e}"
                    ))
                })?
                .into();
        }

        if inner.schema() != source.output_schema {
            let inner_names = inner.schema().names();
            for name in source.output_schema.names() {
                if inner.schema().get_field(&name).is_err() {
                    return Err(DaftError::ValueError(format!(
                        "DataSource '{source_name}' get_dataframe() plan is missing column '{name}' \
                         (inner columns: {})",
                        inner_names.join(", ")
                    )));
                }
            }
            inner = Project::new_from_schema(inner, source.output_schema.clone())
                .map_err(|e| {
                    DaftError::ValueError(format!(
                        "DataSource '{source_name}' get_dataframe() plan cannot project to the source schema: {e}"
                    ))
                })?
                .into();
        }

        Ok(Transformed::yes(inner))
    }
}

#[cfg(feature = "python")]
fn logical_plan_from_py_dataframe(
    df: &pyo3::Py<pyo3::PyAny>,
    source_name: &str,
) -> DaftResult<Arc<LogicalPlan>> {
    use pyo3::{intern, types::PyAnyMethods, Python};

    use crate::PyLogicalPlanBuilder;

    Python::attach(|py| {
        let bound = df.bind(py);
        let current = bound
            .call_method0(intern!(py, "_get_current_builder"))
            .map_err(|e| {
                DaftError::ValueError(format!(
                    "DataSource '{source_name}' get_dataframe() must return a DataFrame: {e}"
                ))
            })?;
        let inner = current.getattr(intern!(py, "_builder")).map_err(|e| {
            DaftError::ValueError(format!(
                "DataSource '{source_name}' get_dataframe() returned a DataFrame without a logical plan: {e}"
            ))
        })?;
        let py_lpb = inner.extract::<PyLogicalPlanBuilder>().map_err(|e| {
            DaftError::ValueError(format!(
                "DataSource '{source_name}' get_dataframe() returned a DataFrame with an unexpected builder: {e}"
            ))
        })?;
        Ok(py_lpb.builder.plan.clone())
    })
}
