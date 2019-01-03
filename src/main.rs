extern crate glib;
extern crate gtk;
extern crate futures;
extern crate futures_cpupool;

mod gtk_futures_executor;

use gtk::prelude::*;
use futures::prelude::*;
use futures_cpupool::CpuPool;

use crate::gtk_futures_executor::GtkEventLoopAsyncExecutor;

fn main() -> Result<(), String> {
    let args = std::env::args_os().collect::<Vec<_>>();

    let repo_path = args.get(1).ok_or("Not passed path to git repository")?.clone();

    println!("Loading git repository {}", repo_path.to_string_lossy());

    gtk::init().map_err(|_| "Failed to initialize Gtk+".to_string())?;

    let gtk_executor = GtkEventLoopAsyncExecutor::new();
    let cpupool = CpuPool::new_num_cpus();

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

    window.show_all();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    {
        list_store.insert_with_values(
            None,
            &[0],
            &[&"Loading..."]
        );
    }

    let list_store_1 = list_store.clone();
    let list_store_2 = list_store.clone();
    let cpupool_1 = cpupool.clone();
    let cpupool_2 = cpupool.clone();
    let window_1 = window.clone();
    gtk_executor.spawn(futures::future::lazy(move || {
        cpupool_1.spawn_fn(move || {
            use std::process::Command;

            std::thread::sleep_ms(1000);

            let mut cmd = Command::new("git");
            cmd
                .current_dir(&repo_path)
                .args(&["log", "--oneline"]);

            let msg = format!("Running {:?} at {:?}", cmd, repo_path);

            futures::future::ok::<_, ()>((cmd, msg))
        }).and_then(move |(cmd, msg)| {
            let list_store = list_store_1;
            list_store.clear();
            list_store.insert_with_values(
                None,
                &[0],
                &[&msg]
            );

            cpupool_2.spawn_fn(move || {
                let mut cmd = cmd;
                let cmd_output = cmd
                    .output()
                    .map_err(|e| e.to_string());

                let cmd_output = match cmd_output {
                    Ok(ref x) if !x.status.success() => Err(String::from_utf8_lossy(&x.stderr).into_owned()),
                    _ => cmd_output
                };

                futures::future::ok::<_, ()>((cmd, cmd_output))
            })
        }).and_then(move |(cmd, cmd_output)| {
            match cmd_output {
                Ok(cmd_output) => {
                    let output_str = String::from_utf8_lossy(&cmd_output.stdout).into_owned();
                    let list_store = list_store_2;
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
                    let dialog = MessageDialog::new(Some(&window_1),
                        DialogFlags::empty() | DialogFlags::MODAL,
                        MessageType::Error,
                        ButtonsType::Ok,
                        &msg
                    );
                    dialog.run();
                    dialog.destroy();
                    let list_store = list_store_2;
                    list_store.clear();
                    list_store.insert_with_values(
                        None,
                        &[0],
                        &[&format!("Error executing {:?}:\n{}", cmd, msg)]
                    );
                }
            }

            futures::future::ok::<_, ()>(())
        })
    }));


    gtk::main();

    Ok(())
}
