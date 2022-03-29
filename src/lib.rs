use std::time::Duration;
use bytes::Bytes;
use mini_redis::client::{self, Client, Message, Subscriber};
use mini_redis::Result;
use tokio::net::ToSocketAddrs;
use tokio::runtime::{Builder, Runtime};

pub struct BlockingClient {
    inner: Client,
    rt: Runtime,
}

impl BlockingClient {
    pub fn connect<T: ToSocketAddrs>(addr: T) -> Result<BlockingClient> {
        let rt = Builder::new_current_thread().enable_all().build()?;

        let inner = rt.block_on(client::connect(addr))?;

        Ok(BlockingClient { inner, rt })
    }

    pub fn get(&mut self, key: &str) -> Result<Option<Bytes>> {
        self.rt.block_on(self.inner.get(key))
    }

    pub fn set(&mut self, key: &str, value: Bytes) -> Result<()> {
        self.rt.block_on(self.inner.set(key, value))
    }

    pub fn set_expires(&mut self, key: &str, value: Bytes, expiration: Duration) -> Result<()> {
        self.rt
            .block_on(self.inner.set_expires(key, value, expiration))
    }

    pub fn publish(&mut self, channel: &str, message: Bytes) -> Result<u64> {
        self.rt.block_on(self.inner.publish(channel, message))
    }

    pub fn subscribe(self, channels: Vec<String>) -> Result<BlockingSubscriber> {
        let inner = self.rt.block_on(self.inner.subscribe(channels))?;
        Ok(BlockingSubscriber { inner, rt: self.rt })
    }
}

pub struct BlockingSubscriber {
    inner: Subscriber,
    rt: Runtime,
}

impl BlockingSubscriber {
    pub fn get_subscribed(&self) -> &[String] {
        self.inner.get_subscribed()
    }

    pub fn next_message(&mut self) -> Result<Option<Message>> {
        self.rt.block_on(self.inner.next_message())
    }

    pub fn subscribe(&mut self, channels: &[String]) -> Result<()> {
        self.rt.block_on(self.inner.subscribe(channels))
    }

    pub fn unsubscribe(&mut self, channels: &[String]) -> Result<()> {
        self.rt.block_on(self.inner.unsubscribe(channels))
    }
}
