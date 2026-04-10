pub(crate) mod metadata;
#[cfg(feature = "runtime-transport")]
mod route;

#[cfg(feature = "runtime-transport")]
pub(crate) use self::route::StudioSqlFlightRouteProvider;
