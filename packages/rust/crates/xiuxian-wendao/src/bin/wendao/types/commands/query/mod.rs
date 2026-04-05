mod graphql;
mod rest;
mod sql;

use clap::Subcommand;

pub(crate) use self::graphql::GraphqlQueryArgs;
pub(crate) use self::rest::RestQueryArgs;
pub(crate) use self::sql::SqlQueryArgs;

#[derive(Debug, Subcommand, Clone)]
pub(crate) enum QueryCommand {
    /// Execute a GraphQL query against the shared query system.
    Graphql(GraphqlQueryArgs),
    /// Execute a REST-style query against the shared query system.
    Rest(RestQueryArgs),
    /// Execute a SQL query against the shared query system.
    Sql(SqlQueryArgs),
}

#[cfg(test)]
pub(crate) fn query(command: QueryCommand) -> super::Command {
    super::Command::Query { command }
}

#[cfg(test)]
#[path = "../../../../../../tests/unit/bin/wendao/types/commands/query.rs"]
mod tests;
