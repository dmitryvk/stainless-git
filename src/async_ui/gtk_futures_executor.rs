type BoxedFuture = Box<futures::future::Future<Item=(), Error=()>>;

struct GtkEventLoopAsyncExecutorBackend {
    main_thread_id: std::thread::ThreadId,
    next_id: std::sync::atomic::AtomicUsize,
    spawns: std::cell::RefCell<std::collections::HashMap<usize, futures::executor::Spawn<BoxedFuture>>>,
}

#[derive(Clone)]
pub struct GtkEventLoopAsyncExecutor {
    backend: std::sync::Arc<GtkEventLoopAsyncExecutorBackend>,
}

// BoxedFuture is only invoked from glib::source::idle_add callback which is sequentially called from single thread
unsafe impl Send for GtkEventLoopAsyncExecutor{}
unsafe impl Sync for GtkEventLoopAsyncExecutor{}

impl GtkEventLoopAsyncExecutor {
    pub fn new() -> Self {
        GtkEventLoopAsyncExecutor {
            backend: std::sync::Arc::new(
                GtkEventLoopAsyncExecutorBackend {
                    main_thread_id: std::thread::current().id(),
                    next_id: std::sync::atomic::AtomicUsize::new(0),
                    spawns: std::cell::RefCell::new(std::collections::HashMap::new())
                }
            )
        }
    }

    pub fn spawn<F: futures::future::Future<Item=(), Error=()> + Sized + 'static>(&self, f: F) {
        let id = self.backend.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        {
            let mut spawns = self.backend.spawns.borrow_mut();
            let spawn = futures::executor::spawn(Box::new(f) as Box<futures::future::Future<Item=(), Error=()>>);
            spawns.insert(id, spawn);
        }

        use futures::executor::Notify;
        self.notify(id);
    }

    fn invoke(&self, id: usize) {
        let opt_spawn = self.backend.spawns.borrow_mut().remove(&id);
        match opt_spawn {
            None => {
                eprintln!("Attempted to invoke non-existing spawn {}", id);
            },
            Some(mut spawn) => {
                let result = spawn.poll_future_notify(
                    &futures::executor::NotifyHandle::from(
                        std::sync::Arc::new(self.clone())
                    ),
                    id
                );
                
                match result {
                    Ok(futures::Async::Ready(_)) => {
                        // Do nothing
                    },
                    Ok(futures::Async::NotReady) => {
                        self.backend.spawns.borrow_mut().insert(id, spawn);
                    },
                    Err(_) => {
                        eprintln!("Spawned future {} returned error", id);
                    }
                }
            }
        }
    }
}

impl futures::executor::Notify for GtkEventLoopAsyncExecutor {
    fn notify(&self, id: usize) {
        let executor = self.clone();
        glib::source::idle_add(move || {
            executor.invoke(id);
            glib::source::Continue(false)
        });
    }
}