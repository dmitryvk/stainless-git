use gtk::prelude::*;
use futures::prelude::*;

use futures_cpupool::CpuPool;

use crate::async_ui::promise::PromiseFuture;
use crate::async_ui::gtk_futures_executor::GtkEventLoopAsyncExecutor;

use std::path::PathBuf;

pub struct IntroScreen {
    #[allow(dead_code)]
    executor: GtkEventLoopAsyncExecutor,
    #[allow(dead_code)]
    cpu_pool: CpuPool,

    window: gtk::Window,
    file_chooser_button: gtk::FileChooserButton,
    ok_button: gtk::Button,
}

impl IntroScreen {
    pub fn new(executor: GtkEventLoopAsyncExecutor, cpu_pool: CpuPool) -> IntroScreen {

        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        let label = gtk::Label::new("Please pick Git repository to browse");
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let file_chooser_button = gtk::FileChooserButton::new(
            "Please pick Git repository to browse",
            gtk::FileChooserAction::SelectFolder
        );
        let ok_button = gtk::Button::new_with_label("OK");

        window.set_title("Stainless Git");
        window.set_default_size(300, 300);
        window.set_position(gtk::WindowPosition::Center);

        window.add(&vbox);

        vbox.add(&label);
        vbox.add(&file_chooser_button);
        vbox.add(&ok_button);

        IntroScreen {
            executor: executor,
            cpu_pool: cpu_pool,
            window: window,
            file_chooser_button: file_chooser_button,
            ok_button: ok_button,
        }
    }

    pub fn show_and_pick_repo(&self) -> impl Future<Item=PathBuf, Error=String> {
        println!("Showing intro (repository picker) screen");
        let result = PromiseFuture::new();

        self.window.show_all();

        self.window.connect_delete_event(capture!(result, window = self.window; move |_, _| {
            result.reject("Not implemented".to_string());
            window.destroy();

            Inhibit(false)
        }));

        self.ok_button.connect_clicked(capture!(result, file_chooser_button = self.file_chooser_button, window = self.window; move |_| {
            match file_chooser_button.get_current_folder() {
                Some(path) => {
                    result.resolve(path);
                    window.destroy();
                },
                None => {
                    println!("Wait! Please pick a directory!");
                }
            }
        }));

        result
    }
}