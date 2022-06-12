// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{future::Future, marker::PhantomData, pin::Pin, task::Poll};

use super::{serde::Serializeable, Event, Result};

pub struct Subscriber<K, V> {
    inner: sled::Subscriber,
    phantom_k: PhantomData<K>,
    phantom_v: PhantomData<V>,
}

impl<K, V> From<sled::Subscriber> for Subscriber<K, V> {
    fn from(inner: sled::Subscriber) -> Self {
        Subscriber {
            inner,
            phantom_k: PhantomData,
            phantom_v: PhantomData,
        }
    }
}

impl<K, V> Iterator for Subscriber<K, V>
where
    K: Serializeable,
    V: Serializeable,
{
    type Item = Result<Event<K, V>>;

    fn next(&mut self) -> Option<Self::Item> {
        let ev = self.inner.next()?;
        Some(Event::from_sled(ev))
    }
}

impl<K, V> Future for Subscriber<K, V>
where
    K: Serializeable + Unpin,
    V: Serializeable + Unpin,
{
    type Output = Option<Result<Event<K, V>>>;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let s = Pin::get_mut(self);
        let future = Pin::new(&mut s.inner);

        let ev = match future.poll(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(ev) => ev,
        };

        let ev = match ev {
            None => return Poll::Ready(None),
            Some(ev) => ev,
        };

        let ev = Event::from_sled(ev);
        Poll::Ready(Some(ev))
    }
}
