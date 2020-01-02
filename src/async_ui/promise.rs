use std::task::{Context, Poll, Waker};
use std::sync::{Arc, Mutex};
use std::pin::Pin;

#[derive(Debug)]
struct PromiseData<T> {
    done: bool,
    val: Option<T>,
    waiter: Option<Waker>,
}

#[derive(Debug, Clone)]
pub struct PromiseResolver<T> {
    data: Arc<Mutex<PromiseData<T>>>,
}

#[derive(Debug)]
pub struct Promise<T> {
    data: Arc<Mutex<PromiseData<T>>>,
}

impl<T> std::future::Future for Promise<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let mut promise_data = self.data.lock().expect("Promise mutex was poisoned");
        
        if promise_data.done {
            panic!("Promise::poll called after completion");
        }

        match promise_data.val.take() {
            Some(result) => Poll::Ready(result),
            _ => {
                promise_data.waiter = Some(cx.waker().clone());
                Poll::Pending
            },
        }
    }
}

impl<T> Promise<T> {
    pub fn new() -> (Self, PromiseResolver<T>) {
        let data_1 = Arc::new(
            Mutex::new(
                PromiseData {
                    done: false,
                    val: None,
                    waiter: None,
                }
            )
        );
        let data_2 = data_1.clone();

        (
            Promise {
                data: data_1
            },
            PromiseResolver {
                data: data_2
            }
        )
    }
}

impl<T> PromiseResolver<T> {
    pub fn resolve(&self, value: T) {
        let waker;

        {
            let mut promise_data = self.data.lock().expect("Promise mutex was poisoned");
            promise_data.val = Some(value);
            waker = promise_data.waiter.take();
        }

        if let Some(waker) = waker {
            waker.wake();
        }
    }
}
