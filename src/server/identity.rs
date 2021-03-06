//! bare-bones Identity service
//!
//! Not middleware :)
//!
//! The service is kept in the global state (Data in the router)
//!
//! Only manually verified/tested, use at own risk.
//! Currently has several `unwrap` which may panic.
//!
//! Cookies only, using jwt tokens. No custom claims.
//!
//! It's a bit manual, but you'll have to:
//!
//! - set jwt token on Response `identity.set_authorization(res)`
//! - check authentication on Request `identity.authorized_user(req)`
//! - forget (clear jwt token, basically sets a cookie with no name and no duration)
//! `identity.forget(res)`

use cookie::Cookie;
use futures_util::io::{AsyncRead, AsyncWrite};
use http::header;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::fmt;
use std::time::Duration;

use crate::{server::ResponseWriter, Request};

/// Identity "middlware", for handling authorized sessions.
#[derive(Clone)]
pub struct Identity {
    /// The key for signing jwts.  Should be kept private, but needs
    /// to be the same on multiple servers sharing a jwt domain.
    server_key: String,
    /// Value for the iss (issuer) jwt claim.
    issuer: Option<String>,
    /// How long a token should be valid after creation, in seconds
    expiration_time: Duration,
    /// Cookie name (Currently only cookies supported, no Auth header).
    /// Default "jwt"
    cookie_name: String,
    /// Cookie path
    /// Default "/"
    /// TODO offer more granular path setting?
    cookie_path: String,
    /// Cookie secure
    /// Default true
    cookie_secure: bool,
    /// Cookie Http Only
    /// Default true
    cookie_http_only: bool,
}

impl Identity {
    /// Create a new instance.
    ///
    /// The `server_key` is used for signing and validating the jwt token.
    pub fn build(server_key: &str) -> IdentityBuilder {
        IdentityBuilder::new(server_key)
    }

    /// Checked for an authorized user for the incoming request
    pub fn authorized_user(&self, req: &Request) -> Option<String> {
        // Get Cookie and token
        let jwtstr = get_cookie(&req, &self.cookie_name);

        // Decode token
        if let Some(jwtstr) = jwtstr {
            let token = decode::<Claims>(
                &jwtstr,
                &DecodingKey::from_secret(self.server_key.as_bytes()),
                &Validation::default(),
            )
            .ok()?;

            //println!("{:?}", token);
            Some(token.claims.sub)
        } else {
            None
        }
    }

    /// Set a token on the `ResponseWriter`, which gets set in a cookie, which authorizes the user.
    pub fn set_auth_token<W>(&self, user: &str, resp_wtr: &mut ResponseWriter<W>)
    where
        W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    {
        // in header set_cookie and provide token
        //
        // This should never fail
        let token = self.make_token(Some(user), None).unwrap();
        let cookie = Cookie::build(&self.cookie_name, token)
            .path(&self.cookie_path)
            .max_age(self.expiration_time.try_into().unwrap()) // this uses time crate :(
            .http_only(self.cookie_http_only)
            .secure(self.cookie_secure)
            .finish();
        resp_wtr.append_header(header::SET_COOKIE, cookie.to_string().parse().unwrap());
    }

    /// Set an expired token on the `ResponseWriter`, which gets set in a cookie, which will
    /// effectively "log out" the user.
    pub fn forget<W>(&self, resp_wtr: &mut ResponseWriter<W>)
    where
        W: AsyncRead + AsyncWrite + Clone + Send + Sync + Unpin + 'static,
    {
        // in header set_cookie and provide "blank" token
        //
        // This should never fail
        let token = self.make_token(None, Some(0)).unwrap();
        let cookie = Cookie::build(&self.cookie_name, token)
            .path(&self.cookie_path)
            .max_age(time::Duration::seconds(0)) // this uses time crate :(
            .http_only(self.cookie_http_only)
            .secure(self.cookie_secure)
            .finish();
        resp_wtr.append_header(header::SET_COOKIE, cookie.to_string().parse().unwrap());
    }

    fn make_token(
        &self,
        user: Option<&str>,
        expiration: Option<u64>,
    ) -> Result<String, IdentityFail> {
        let claims = Claims {
            exp: expiration
                .unwrap_or_else(|| self.expiration_time.as_secs() + current_numeric_date()),
            iss: self
                .issuer
                .as_ref()
                .cloned()
                .unwrap_or_else(|| "".to_owned()),
            sub: user.map(|s| s.to_owned()).unwrap_or_else(|| "".to_owned()),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.server_key.as_bytes()),
        )
        .map_err(IdentityFail::Encode)
    }
}

// Separate builder, because there's two sets of apis, one for building and one for using.
//
// If it was just build and then finish, might not need a builder.
/// Builder for Identity
pub struct IdentityBuilder {
    server_key: String,
    issuer: Option<String>,
    expiration_time: Duration,
    cookie_name: Option<String>, // default "jwt"
    cookie_path: Option<String>, // default "/"
    cookie_secure: bool,         // default true
    cookie_http_only: bool,      // default true
}

impl IdentityBuilder {
    /// Create a new instance.
    ///
    /// The `server_key` is used for signing and validating the jwt token.
    pub fn new(server_key: &str) -> IdentityBuilder {
        IdentityBuilder {
            server_key: server_key.to_owned(),
            issuer: None,
            expiration_time: Duration::from_secs(60 * 60 * 24),
            cookie_name: None,
            cookie_path: None,
            cookie_secure: true,
            cookie_http_only: true,
        }
    }

    /// Set a value for the iss (issuer) jwt claim.
    ///
    /// The default is to not set an issuer.
    pub fn cookie_name(mut self, name: &str) -> Self {
        self.cookie_name = Some(name.to_owned());
        self
    }
    /// Set cookie path
    ///
    /// The default is "/".
    pub fn cookie_path(mut self, path: &str) -> Self {
        self.cookie_path = Some(path.to_owned());
        self
    }

    /// Set cookie Secure (https only)
    ///
    /// The default is true.
    pub fn cookie_secure(mut self, secure: bool) -> Self {
        self.cookie_secure = secure;
        self
    }

    /// Set cookie http only
    ///
    /// The default is true.
    pub fn cookie_http_only(mut self, http_only: bool) -> Self {
        self.cookie_http_only = http_only;
        self
    }

    /// Set a value for the iss (issuer) jwt claim.
    ///
    /// The default is to not set an issuer.
    pub fn issuer(mut self, issuer: &str) -> Self {
        self.issuer = Some(issuer.to_owned());
        self
    }

    /// Set how long a token should be valid after creation (in seconds).
    ///
    /// The default is 24 hours.
    pub fn expiration_time(mut self, expiration_time: Duration) -> Self {
        self.expiration_time = expiration_time;
        self
    }

    /// Finish building an Identity
    pub fn finish(self) -> Identity {
        Identity {
            server_key: self.server_key,
            issuer: self.issuer,
            expiration_time: self.expiration_time,
            cookie_name: self.cookie_name.unwrap_or_else(|| "jwt".to_owned()),
            cookie_path: self.cookie_path.unwrap_or_else(|| "/".to_owned()),
            cookie_secure: self.cookie_secure,
            cookie_http_only: self.cookie_http_only,
        }
    }
}

/// Gets the first cookie with the name
fn get_cookie(req: &Request, name: &str) -> Option<String> {
    for cookie in req.headers().get_all(header::COOKIE) {
        let cookie = Cookie::parse(cookie.to_str().ok()?).ok()?;
        if cookie.name() == name {
            return Some(cookie.value().to_string());
        }
    }
    None
}

/// Get the current value for jwt NumericDate.
///
/// Defined in RFC 7519 section 2 to be equivalent to POSIX.1 "Seconds
/// Since the Epoch".  The RFC allows a NumericDate to be non-integer
/// (for sub-second resolution), but the jwt crate uses u64.
fn current_numeric_date() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .unwrap()
        .as_secs()
}

// Claims to token
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: u64,
    iss: String,
    // user
    sub: String,
}

/// Error for Identity. Bascially, the errors are for encoding or decoding the jwt token.
#[derive(Debug)]
pub enum IdentityFail {
    /// Encode error for jwt token
    Encode(jsonwebtoken::errors::Error),
    /// Decode error for jwt token
    Decode(jsonwebtoken::errors::Error),
}

impl std::error::Error for IdentityFail {}

impl fmt::Display for IdentityFail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use IdentityFail::*;
        match self {
            Encode(err) => write!(f, "jwt encoding error: {}", err),
            Decode(err) => write!(f, "jwt decoding error: {}", err),
        }
    }
}
