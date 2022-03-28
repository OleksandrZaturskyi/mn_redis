use future::Delay;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() {
    let when = Instant::now() + Duration::from_millis(1 * 1000);
    let delay = Delay::new(when);

    let mut mini_tokio = mn_tokio::MiniTokio::new();

    mini_tokio.spawn(delay);

    mini_tokio.run();
}

mod mn_tokio {
    use crossbeam::channel::{self, Receiver, Sender};
    use futures::task::{self, ArcWake};
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use std::task::Context;

    pub struct MiniTokio {
        scheduled: Receiver<Arc<Task>>,
        sender: Sender<Arc<Task>>,
    }

    impl MiniTokio {
        pub fn new() -> Self {
            let (tx, rx) = channel::unbounded();

            Self {
                scheduled: rx,
                sender: tx,
            }
        }

        pub fn spawn<F>(&mut self, future: F)
        where
            F: Future<Output = ()> + Send + 'static,
        {
            Task::spawn(future, &self.sender)
        }

        pub fn run(&mut self) {
            while let Ok(task) = self.scheduled.recv() {
                task.poll();
            }
        }
    }

    struct Task {
        future: Mutex<Pin<Box<dyn Future<Output = ()> + Send>>>,
        executor: Sender<Arc<Task>>,
    }

    impl Task {
        fn spawn<F>(future: F, sender: &Sender<Arc<Self>>)
        where
            F: Future<Output = ()> + Send + 'static,
        {
            let task = Arc::new(Self {
                future: Mutex::new(Box::pin(future)),
                executor: sender.clone(),
            });

            let _ = sender.send(task);
        }

        fn schedule(self: &Arc<Self>) {
            self.executor.send(self.clone()).unwrap();
        }

        fn poll(self: Arc<Self>) {
            let waker = task::waker(self.clone());
            let mut ctx = Context::from_waker(&waker);

            let mut future = self.future.try_lock().unwrap();

            let _ = future.as_mut().poll(&mut ctx);
        }
    }

    impl ArcWake for Task {
        fn wake_by_ref(arc_self: &Arc<Self>) {
            arc_self.schedule();
        }
    }
}

mod future {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Waker};
    use std::thread;
    use std::time::Instant;

    pub struct Delay {
        when: Instant,
        waker: Option<Arc<Mutex<Waker>>>,
    }

    impl Delay {
        pub fn new(when: Instant) -> Self {
            Self { when, waker: None }
        }
    }

    impl Future for Delay {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
            if let Some(waker) = &self.waker {
                let mut waker = waker.lock().unwrap();

                if !waker.will_wake(cx.waker()) {
                    *waker = cx.waker().clone();
                }
            } else {
                let when = self.when;
                let waker = Arc::new(Mutex::new(cx.waker().clone()));
                self.waker = Some(waker.clone());

                thread::spawn(move || {
                    let now = Instant::now();

                    if now < when {
                        thread::sleep(when - now);
                    }

                    let waker = waker.lock().unwrap();
                    waker.wake_by_ref()
                });
            }

            if Instant::now() >= self.when {
                println!("hello world");
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }
    }
}
