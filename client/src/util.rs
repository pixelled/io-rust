use either::Either;
use futures::Stream;
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};

pub fn with_latest<S: Stream, A: Stream>(src: S, acc: A) -> WithLatest<S, A> {
	WithLatest { src, acc, val: None }
}

#[pin_project]
pub struct WithLatest<S: Stream, A: Stream> {
	#[pin]
	src: S,
	#[pin]
	acc: A,
	val: Option<<A as Stream>::Item>,
}

impl<S: Stream, A: Stream> Stream for WithLatest<S, A> {
	type Item = (<S as Stream>::Item, Option<<A as Stream>::Item>);

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.project();
		if let Poll::Ready(x) = this.acc.poll_next(cx) {
			*this.val = x;
		}
		match this.src.poll_next(cx) {
			Poll::Ready(Some(x)) => {
				let val = std::mem::take(this.val);
				Poll::Ready(Some((x, val)))
			}
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}

pub fn merge<S1: Stream, S2: Stream>(s1: S1, s2: S2) -> Merge<S1, S2> {
	Merge { s1, s2, flag: false }
}

#[pin_project]
pub struct Merge<S1: Stream, S2: Stream> {
	#[pin]
	s1: S1,
	#[pin]
	s2: S2,
	flag: bool,
}

impl<S1: Stream, S2: Stream> Stream for Merge<S1, S2> {
	type Item = Either<S1::Item, S2::Item>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.project();
		if !*this.flag {
			let first_done = match this.s1.poll_next(cx) {
				Poll::Ready(Some(item)) => {
					*this.flag = !*this.flag;
					return Poll::Ready(Some(Either::Left(item)));
				}
				Poll::Ready(None) => true,
				Poll::Pending => false,
			};

			match this.s2.poll_next(cx) {
				Poll::Ready(Some(item)) => Poll::Ready(Some(Either::Right(item))),
				Poll::Ready(None) if first_done => Poll::Ready(None),
				Poll::Ready(None) | Poll::Pending => Poll::Pending,
			}
		} else {
			let first_done = match this.s2.poll_next(cx) {
				Poll::Ready(Some(item)) => {
					*this.flag = !*this.flag;
					return Poll::Ready(Some(Either::Right(item)));
				}
				Poll::Ready(None) => true,
				Poll::Pending => false,
			};

			match this.s1.poll_next(cx) {
				Poll::Ready(Some(item)) => Poll::Ready(Some(Either::Left(item))),
				Poll::Ready(None) if first_done => Poll::Ready(None),
				Poll::Ready(None) | Poll::Pending => Poll::Pending,
			}
		}
	}
}
