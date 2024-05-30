use http::{header::HeaderValue, HeaderMap};
use pingora::{Error, ErrorType::HTTPStatus};
#[derive(Debug)]
pub struct BearerToken(String);
impl BearerToken {
    pub fn get_user_id(&self) -> Result<String, Error> {
        Ok("Migo".to_string())
    }
}

impl TryFrom<&HeaderValue> for BearerToken {
    fn try_from(value: &HeaderValue) -> Result<Self, Self::Error> {
        Ok(Self(
            value
                .to_str()
                .map_err(|_| Error::explain(HTTPStatus(401), "Bearer Not Found"))?
                .chars()
                .skip(7)
                .collect::<String>(),
        ))
    }

    type Error = std::boxed::Box<pingora::Error>;
}

impl TryFrom<&HeaderMap<HeaderValue>> for BearerToken {
    fn try_from(value: &HeaderMap<HeaderValue>) -> Result<Self, Self::Error> {
        value
            .get("Authorization")
            .map(TryInto::try_into)
            .ok_or(Error::explain(
                HTTPStatus(401),
                "Authorization token required",
            ))?
    }
    type Error = std::boxed::Box<pingora::Error>;
}
