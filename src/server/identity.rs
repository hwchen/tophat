//! bare-bones Identity service
//!
//! Not middleware :)
//!
//! Only manually verified/tested, use at own risk.
//!
//! The service is kept in the global state (Data in the router)
//!
//! Cookies only
//!
//! It uses jwt tokens
//!
//! It's a bit manual, but you'll have to:
//!
//! - set jwt token on Response `identity.set_authorization(res)`
//! - check authentication on Request `identity.authorized_user(req)`
//! - forget (clear jwt token, basically sets a cookie with no name and no duration)
//! `identity.forget(res)`

use cookie::Cookie;
use http::header;
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Serialize, Deserialize};
use std::convert::TryInto;
use std::time::Duration;
use thiserror::Error as ThisError;

use crate::{Request, Response};
use crate::server::reply;

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
}

impl Identity {
    /// Create a new instance.
    ///
    /// The `server_key` is used for signing and validating the jwt token.
    pub fn new(server_key: &str) -> IdentityBuilder {
        IdentityBuilder::new(server_key)
    }

    pub fn authorized_user(&self, req: &Request) -> Option<String> {
        // Get Cookie and token
        let jwtstr = get_cookie(&req, &self.cookie_name);

        // Decode token
        if let Some(jwtstr) = jwtstr {
            let token = decode::<Claims>(
                &jwtstr,
                &DecodingKey::from_secret(self.server_key.as_bytes()),
                &Validation::default()
            ).ok()?;

            //println!("{:?}", token);
            Some(token.claims.sub)
        } else {
            None
        }
    }

    pub fn set_auth_token(&self, user: &str, resp: &mut Response) {
        // in header set_cookie and provide token
        //
        // This should never fail
        let token = self.make_token(Some(user), None).unwrap();
        let cookie = Cookie::build(&self.cookie_name, token)
            .path(&self.cookie_path)
            .max_age(self.expiration_time.try_into().unwrap()) // this uses time crate :(
            .http_only(true)
            // .secure(true) make this an option
            .finish();
        resp.headers_mut().append(
            header::SET_COOKIE,
            cookie.to_string().parse().unwrap(),
        );
    }

    pub fn forget(&self, resp: &mut Response) {
        // in header set_cookie and provide "blank" token
        //
        // This should never fail
        let token = self.make_token(None, Some(0)).unwrap();
        let cookie = Cookie::build(&self.cookie_name, token)
            .path(&self.cookie_path)
            .max_age(time::Duration::seconds(0)) // this uses time crate :(
            .http_only(true)
            // .secure(true) make this an option
            .finish();
        resp.headers_mut().append(
            header::SET_COOKIE,
            cookie.to_string().parse().unwrap(),
        );
    }

    fn make_token(
        &self,
        user: Option<&str>,
        expiration: Option<u64>,
    ) -> Result<String, IdentityFail> {
        let claims = Claims {
            exp: expiration.unwrap_or_else(|| self.expiration_time.as_secs() + current_numeric_date()),
            iss: self.issuer.as_ref().cloned().unwrap_or_else(||"".to_owned()),
            sub: user.map(|s| s.to_owned()).unwrap_or_else(||"".to_owned()),
        };

        encode(&Header::default(), &claims, &EncodingKey::from_secret(self.server_key.as_bytes()))
            .map_err(|err| IdentityFail::Encode(err))
    }
}

// Separate builder, because there's two sets of apis, one for building and one for using.
//
// If it was just build and then finish, might not need a builder.
pub struct IdentityBuilder {
    server_key: String,
    issuer: Option<String>,
    expiration_time: Duration,
    cookie_name: Option<String>, // default "jwt"
    cookie_path: Option<String>, // default "/"
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

    pub fn finish(self) -> Identity {
        Identity {
            server_key: self.server_key,
            issuer: self.issuer,
            expiration_time: self.expiration_time,
            cookie_name: self.cookie_name.unwrap_or_else(|| "jwt".to_owned()),
            cookie_path: self.cookie_path.unwrap_or_else(|| "/".to_owned()),
        }
    }
}

/// Gets the first cookie with the name
fn get_cookie(
    req: &Request,
    name: &str,
) -> Option<String> {
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
    SystemTime::now().duration_since(UNIX_EPOCH).ok().unwrap().as_secs()
}

// Claims to token
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: u64,
    iss: String,
    // user
    sub: String,
}

#[derive(ThisError, Debug)]
pub enum IdentityFail {
    #[error("jwt encoding error: {0}")]
    Encode(jsonwebtoken::errors::Error),
    #[error("jwt decoding error: {0}")]
    Decode(jsonwebtoken::errors::Error),
}

impl IdentityFail {
    pub fn to_response(&self) -> crate::Response {
        reply::code(400).unwrap()
    }
}
