use crate::engine::compiler::mechanism_dispatch::resolver_chain;
use crate::engine::compiler::{task_mechanisms, task_type};

pub(super) fn build(
    context: resolver_chain::DispatchContext<'_>,
) -> Option<resolver_chain::ResolveOutcome> {
    let resolver_chain::DispatchContext {
        task_type,
        node_def,
        ..
    } = context;
    match task_type {
        task_type::TaskType::WendaoSqlDiscover => {
            Some(Ok(task_mechanisms::wendao_sql_discover(node_def)))
        }
        task_type::TaskType::WendaoSqlValidate => {
            Some(Ok(task_mechanisms::wendao_sql_validate(node_def)))
        }
        task_type::TaskType::WendaoSqlExecute => {
            Some(Ok(task_mechanisms::wendao_sql_execute(node_def)))
        }
        _ => None,
    }
}
