use bytes::Bytes;
use std::pin::Pin;
use std::task::{Context, Poll};
use http::HeaderMap;
use http_body::{Body as HttpBody, SizeHint};

pub struct Body {
    kind: Kind,
}

enum Kind {
    Once(Option<Bytes>),
}

impl Body {
    pub fn empty() -> Body {
        Body { kind: Kind::Once(None) }
    }

    fn poll_inner(&mut self, _cx: &mut Context<'_>) -> Poll<Option<http::Result<Bytes>>> {
        match self.kind {
            Kind::Once(ref mut val) => Poll::Ready(val.take().map(Ok)),
        }
    }
}

impl HttpBody for Body {
    type Data = Bytes;
    type Error = http::Error;

    fn poll_data(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        self.poll_inner(cx)
    }

    fn poll_trailers(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<Option<HeaderMap>, Self::Error>> {
        match self.kind {
            _ => Poll::Ready(Ok(None)),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self.kind {
            Kind::Once(ref val) => val.is_none(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self.kind {
            Kind::Once(Some(ref val)) => SizeHint::with_exact(val.len() as u64),
            Kind::Once(None) => SizeHint::with_exact(0),
        }
    }
}
