//! Studio API endpoint handlers.

pub(crate) mod flight;
pub(crate) mod service;

pub(crate) use flight::{
    StudioCodeAstAnalysisFlightRouteProvider, StudioMarkdownAnalysisFlightRouteProvider,
};
