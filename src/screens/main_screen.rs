use gtk::prelude::*;
use futures::prelude::*;
use futures::future;

use futures_cpupool::CpuPool;

use crate::async_ui::event_future::EventFuture;
use crate::async_ui::gtk_futures_executor::GtkEventLoopAsyncExecutor;

use std::path::PathBuf;

pub struct MainScreen {
    executor: GtkEventLoopAsyncExecutor,
    cpu_pool: CpuPool,
    window: gtk::Window,

    repo_path: std::path::PathBuf,

    list_store: gtk::ListStore,
}

impl MainScreen {
    pub fn new(executor: GtkEventLoopAsyncExecutor, cpu_pool: CpuPool, repo_path: PathBuf) -> MainScreen {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title(&repo_path.to_string_lossy());

        let list_store = gtk::ListStore::new(&[gtk::Type::String]);

        let tree_view = gtk::TreeView::new();

        let scrolled_window = gtk::ScrolledWindow::new(None, None);

        scrolled_window.add(&tree_view);

        tree_view.set_model(&list_store);

        let cell_renderer = gtk::CellRendererText::new();
        let column = gtk::TreeViewColumn::new();
        column.set_title("Commit message");
        column.pack_start(&cell_renderer, true);
        column.add_attribute(&cell_renderer, "text", 0);

        tree_view.append_column(&column);

        window.add(&scrolled_window);

        window.set_default_size(300, 300);
        window.set_position(gtk::WindowPosition::Center);

        MainScreen {
            executor: executor,
            cpu_pool: cpu_pool,
            window: window,
            repo_path: repo_path,
            list_store: list_store,
        }
    }

    pub fn show(&self) -> impl Future<Item=(), Error=()> {
        println!("Showing main screen");
        let result = EventFuture::new();

        self.window.show_all();

        self.window.connect_delete_event(capture!(result, window = self.window; move |_, _| {
            result.notify();
            window.destroy();
            Inhibit(false)
        }));

        {
            self.list_store.insert_with_values(
                None,
                &[0],
                &[&"Loading..."]
            );
        }

        self.executor.spawn(future::lazy(capture!(
            cpu_pool = self.cpu_pool, repo_path = self.repo_path, list_store = self.list_store, window = self.window;
            move || {
            cpu_pool.spawn_fn(move || {
                use std::process::Command;

                std::thread::sleep(std::time::Duration::from_millis(1000));

                let mut cmd = Command::new("git");
                cmd
                    .current_dir(&repo_path)
                    .args(&["log", "--oneline"]);

                let msg = format!("Running {:?} at {:?}", cmd, repo_path);

                future::ok::<_, ()>((cmd, msg))
            }).and_then(capture!(list_store, cpu_pool; move |(cmd, msg)| {
                list_store.clear();
                list_store.insert_with_values(
                    None,
                    &[0],
                    &[&msg]
                );

                cpu_pool.spawn_fn(move || {
                    let mut cmd = cmd;
                    let cmd_output = cmd
                        .output()
                        .map_err(|e| e.to_string());

                    let cmd_output = match cmd_output {
                        Ok(ref x) if !x.status.success() => Err(String::from_utf8_lossy(&x.stderr).into_owned()),
                        _ => cmd_output
                    };

                    future::ok::<_, ()>((cmd, cmd_output))
                })
            })).and_then(capture!(list_store, window; move |(cmd, cmd_output)| {
                match cmd_output {
                    Ok(cmd_output) => {
                        let output_str = String::from_utf8_lossy(&cmd_output.stdout).into_owned();
                        list_store.clear();
                        for line in output_str.lines() {
                            list_store.insert_with_values(
                                None,
                                &[0],
                                &[&line]
                            );
                        }
                    },
                    Err(msg) => {
                        use gtk::{ButtonsType, DialogFlags, MessageType, MessageDialog};
                        let dialog = MessageDialog::new(Some(&window),
                            DialogFlags::empty() | DialogFlags::MODAL,
                            MessageType::Error,
                            ButtonsType::Ok,
                            &msg
                        );
                        dialog.run();
                        dialog.destroy();
                        list_store.clear();
                        list_store.insert_with_values(
                            None,
                            &[0],
                            &[&format!("Error executing {:?}:\n{}", cmd, msg)]
                        );
                    }
                }

                future::ok::<_, ()>(())
            }))
        })));

        result
    }
}