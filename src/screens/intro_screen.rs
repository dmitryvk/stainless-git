
use gtk::prelude::*;
use futures::prelude::*;

use futures_cpupool::CpuPool;

use crate::async_ui::event_future::EventFuture;
use crate::async_ui::gtk_futures_executor::GtkEventLoopAsyncExecutor;

pub struct IntroScreen {
    #[allow(dead_code)]
    executor: GtkEventLoopAsyncExecutor,
    #[allow(dead_code)]
    cpu_pool: CpuPool,
    window: gtk::Window,
}

impl IntroScreen {
    pub fn new(executor: GtkEventLoopAsyncExecutor, cpu_pool: CpuPool) -> IntroScreen {

        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title("Привет, мир!");
        window.set_default_size(300, 300);
        window.set_position(gtk::WindowPosition::Center);

        IntroScreen {
            executor: executor,
            cpu_pool: cpu_pool,
            window: window
        }
    }

    pub fn show(&self) -> impl Future<Item=(), Error=()> {
        println!("Showing screen");
        let result = EventFuture::new();

        self.window.show_all();

        self.window.connect_delete_event(capture!(result, window = self.window; move |_, _| {
            result.notify();
            window.destroy();

            Inhibit(false)
        }));

        result
    }
}