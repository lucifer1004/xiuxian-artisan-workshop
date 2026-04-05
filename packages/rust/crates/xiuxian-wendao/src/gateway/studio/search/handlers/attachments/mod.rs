mod batch;
mod provider;
mod response;
#[cfg(test)]
mod tests;

pub(crate) use provider::StudioAttachmentSearchFlightRouteProvider;
#[cfg(test)]
pub(crate) use response::load_attachment_search_response_from_studio;
