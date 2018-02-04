use diesel;
use diesel::r2d2;
use diesel_migrations::RunMigrationsError;
use hyper;
use native_tls;
use reqwest;
// use rss;
use url;

use std::io;

#[allow(dead_code)]
#[derive(Fail, Debug)]
#[fail(display = "IO Error: {}", _0)]
struct IOError(io::Error);

// fadsadfs NOT SYNC
// #[derive(Fail, Debug)]
// #[fail(display = "RSS Error: {}", _0)]
// struct RSSError(rss::Error);

#[derive(Fail, Debug)]
pub enum DatabaseError {
    #[fail(display = "SQL Query failed: {}", _0)]
    DieselResultError(#[cause] diesel::result::Error),
    #[fail(display = "Database Migration error: {}", _0)]
    DieselMigrationError(#[cause] RunMigrationsError),
    #[fail(display = "R2D2 error: {}", _0)]
    R2D2Error(#[cause] r2d2::Error),
    #[fail(display = "R2D2 Pool error: {}", _0)]
    R2D2PoolError(#[cause] r2d2::PoolError),
}

impl From<RunMigrationsError> for DatabaseError {
    fn from(err: RunMigrationsError) -> Self {
        DatabaseError::DieselMigrationError(err)
    }
}

impl From<diesel::result::Error> for DatabaseError {
    fn from(err: diesel::result::Error) -> Self {
        DatabaseError::DieselResultError(err)
    }
}

impl From<r2d2::Error> for DatabaseError {
    fn from(err: r2d2::Error) -> Self {
        DatabaseError::R2D2Error(err)
    }
}

impl From<r2d2::PoolError> for DatabaseError {
    fn from(err: r2d2::PoolError) -> Self {
        DatabaseError::R2D2PoolError(err)
    }
}

#[derive(Fail, Debug)]
pub enum HttpError {
    #[fail(display = "Reqwest Error: {}", _0)]
    ReqError(#[cause] reqwest::Error),
    #[fail(display = "Hyper Error: {}", _0)]
    HyperError(#[cause] hyper::Error),
    #[fail(display = "Url Error: {}", _0)]
    UrlError(#[cause] url::ParseError),
    #[fail(display = "TLS Error: {}", _0)]
    TLSError(#[cause] native_tls::Error),
}
