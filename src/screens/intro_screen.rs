use gtk::prelude::*;
use futures::prelude::*;

use futures_cpupool::CpuPool;

use crate::async_ui::event_future::EventFuture;
use crate::async_ui::gtk_futures_executor::GtkEventLoopAsyncExecutor;

pub struct IntroScreen {
    executor: GtkEventLoopAsyncExecutor,
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

        let result2 = result.clone();
        let window2 = self.window.clone();

        self.window.connect_delete_event(move |_, _| {
            result2.notify();
            window2.destroy();

            Inhibit(false)
        });

        result
    }
}