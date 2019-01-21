use futures::prelude::*;

#[derive(Debug)]
struct PromiseFutureBackend<T, E> {
    result: Option<Result<T, E>>,
    waiting_tasks: Vec<futures::task::Task>,
}

#[derive(Debug, Clone)]
pub struct PromiseFuture<T, E> {
    backend: std::sync::Arc<std::sync::Mutex<PromiseFutureBackend<T, E>>>,
}

impl<T, E> PromiseFuture<T, E> {
    pub fn new() -> PromiseFuture<T, E> {
        PromiseFuture {
            backend: std::sync::Arc::new(
                std::sync::Mutex::new(
                    PromiseFutureBackend {
                        result: None,
                        waiting_tasks: vec![],
                    }
                )
            )
        }
    }

    pub fn resolve(&self, result: T) {
        let mut backend = self.backend.lock().unwrap();

        backend.result = Some(Ok(result));
        for task in &backend.waiting_tasks {
            task.notify();
        }

        backend.waiting_tasks.clear();
    }

    pub fn reject(&self, error: E) {
        let mut backend = self.backend.lock().unwrap();

        backend.result = Some(Err(error));
        for task in &backend.waiting_tasks {
            task.notify();
        }

        backend.waiting_tasks.clear();
    }
}

impl <T, E> Future for PromiseFuture<T, E> {
    type Item = T;
    type Error = E;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut guard = self.backend.lock().unwrap();
        let backend = &mut *guard;

        match backend.result {
            Some(_) => match backend.result.take().unwrap() {
                Ok(result) => std::result::Result::Ok(Async::Ready(result)),
                Err(error) => std::result::Result::Err(error),
            },
            None => {
                backend.waiting_tasks.push(futures::task::current());
                std::result::Result::Ok(Async::NotReady)
            }
        }
    }
}
