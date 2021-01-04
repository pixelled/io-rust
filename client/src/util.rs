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

impl<S, A> Stream for WithLatest<S, A>
where
	S: Stream + Unpin,
	A: Stream + Unpin,
{
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

impl<S1, S2> Stream for Merge<S1, S2>
where
	S1: Stream,
	S2: Stream,
{
	type Item = Either<S1::Item, S2::Item>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.project();
		// if !self.flag {
		// 	let a_done = match this.s1.poll_next(cx) {
		// 		Poll::Ready(Some(item)) => {
		// 			// give the other stream a chance to go first next time
		// 			self.flag = !self.flag;
		// 			return Poll::Ready(Some(Either::Left(item)));
		// 		}
		// 		Poll::Ready(None) => true,
		// 		Poll::Pending => false,
		// 	};
		//
		// 	match this.s2.poll_next(cx) {
		// 		Poll::Ready(Some(item)) => Poll::Ready(Some(Either::Right(item))),
		// 		Poll::Ready(None) if a_done => Poll::Ready(None),
		// 		Poll::Ready(None) | Poll::Pending => Poll::Pending,
		// 	}
		// }
		if *this.flag {
			poll_inner2(this.flag, this.s2, this.s1, cx)
		} else {
			poll_inner1(this.flag, this.s1, this.s2, cx)
		}
	}
}

fn poll_inner1<S1, S2>(
	flag: &mut bool,
	a: Pin<&mut S1>,
	b: Pin<&mut S2>,
	cx: &mut Context<'_>,
) -> Poll<Option<Either<S1::Item, S2::Item>>>
where
	S1: Stream,
	S2: Stream,
{
	let a_done = match a.poll_next(cx) {
		Poll::Ready(Some(item)) => {
			// give the other stream a chance to go first next time
			*flag = !*flag;
			return Poll::Ready(Some(Either::Left(item)));
		}
		Poll::Ready(None) => true,
		Poll::Pending => false,
	};

	match b.poll_next(cx) {
		Poll::Ready(Some(item)) => Poll::Ready(Some(Either::Right(item))),
		Poll::Ready(None) if a_done => Poll::Ready(None),
		Poll::Ready(None) | Poll::Pending => Poll::Pending,
	}
}

fn poll_inner2<S1, S2>(
	flag: &mut bool,
	a: Pin<&mut S1>,
	b: Pin<&mut S2>,
	cx: &mut Context<'_>,
) -> Poll<Option<Either<S2::Item, S1::Item>>>
where
	S1: Stream,
	S2: Stream,
{
	let a_done = match a.poll_next(cx) {
		Poll::Ready(Some(item)) => {
			// give the other stream a chance to go first next time
			*flag = !*flag;
			return Poll::Ready(Some(Either::Right(item)));
		}
		Poll::Ready(None) => true,
		Poll::Pending => false,
	};

	match b.poll_next(cx) {
		Poll::Ready(Some(item)) => Poll::Ready(Some(Either::Left(item))),
		Poll::Ready(None) if a_done => Poll::Ready(None),
		Poll::Ready(None) | Poll::Pending => Poll::Pending,
	}
}
