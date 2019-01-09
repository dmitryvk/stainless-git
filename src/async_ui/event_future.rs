use futures::prelude::*;

#[derive(Debug)]
struct EventFutureBackend {
    ready: bool,
    waiting_tasks: Vec<futures::task::Task>,
}

#[derive(Debug, Clone)]
pub struct EventFuture {
    backend: std::sync::Arc<std::sync::Mutex<EventFutureBackend>>,
}

impl EventFuture {
    pub fn new() -> EventFuture {
        EventFuture {
            backend: std::sync::Arc::new(
                std::sync::Mutex::new(
                    EventFutureBackend {
                        ready: false,
                        waiting_tasks: vec![],
                    }
                )
            )
        }
    }

    pub fn notify(&self) {
        let mut backend = self.backend.lock().unwrap();

        backend.ready = true;
        for task in &backend.waiting_tasks {
            task.notify();
        }

        backend.waiting_tasks.clear();
    }
}

impl Future for EventFuture {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut guard = self.backend.lock().unwrap();
        let backend = &mut *guard;

        if backend.ready {
            std::result::Result::Ok(Async::Ready(()))
        } else {
            backend.waiting_tasks.push(futures::task::current());
            std::result::Result::Ok(Async::NotReady)
        }
    }
}
