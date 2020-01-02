use gtk::prelude::*;
use std::path::PathBuf;

pub struct IntroScreen {
    window: gtk::Window,
    file_chooser_button: gtk::FileChooserButton,
    ok_button: gtk::Button,
}

impl IntroScreen {
    pub fn new() -> IntroScreen {

        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        let label = gtk::Label::new(Some("Please pick Git repository to browse"));
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
            window,
            file_chooser_button,
            ok_button,
        }
    }

    pub async fn show_and_pick_repo(&self) -> Result<PathBuf, String> {
        println!("Showing intro (repository picker) screen");
        use super::super::async_ui::promise::Promise;
        let (promise, resolver) = Promise::new();

        self.window.show_all();

        self.window.connect_delete_event(capture!(resolver, window = self.window; move |_, _| {
            resolver.resolve(Err("Not implemented".to_string()));
            window.destroy();

            Inhibit(false)
        }));

        self.ok_button.connect_clicked(capture!(resolver, file_chooser_button = self.file_chooser_button, window = self.window; move |_| {
            match file_chooser_button.get_filename() {
                Some(path) => {
                    resolver.resolve(Ok(path));
                    window.destroy();
                },
                None => {
                    println!("Wait! Please pick a directory!");
                }
            }
        }));

        promise.await
    }
}