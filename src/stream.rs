use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::Stream;

use crate::Error;

pub struct Mp4Frame {}

pub struct Mp4Stream {}
impl Stream for Mp4Stream {
    type Item = Result<Mp4Frame, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        todo!()
    }
}
