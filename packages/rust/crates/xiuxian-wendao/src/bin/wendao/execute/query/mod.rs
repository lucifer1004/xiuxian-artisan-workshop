mod graphql;
mod rest;
mod sql;

use anyhow::Result;

use crate::types::{Cli, Command, QueryCommand};

pub(super) async fn handle(cli: &Cli) -> Result<()> {
    let Command::Query { command } = &cli.command else {
        unreachable!("query handler called with non-query command");
    };

    match command {
        QueryCommand::Graphql(args) => graphql::handle(cli, args).await,
        QueryCommand::Rest(args) => rest::handle(cli, args).await,
        QueryCommand::Sql(args) => sql::handle(cli, args).await,
    }
}
