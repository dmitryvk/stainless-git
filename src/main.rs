#[macro_use]
mod async_ui;
mod screens;
mod dag_layout;
mod dump_git_layout;

use futures::prelude::*;
use futures::future;
use futures_cpupool::CpuPool;

use async_ui::gtk_futures_executor::GtkEventLoopAsyncExecutor;
use gtk::prelude::*;

fn main() -> Result<(), String> {
    use std::rc::Rc;
    use std::cell::Cell;

    {
        let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
        if let Some(cmd) = args.get(0) {
            if cmd == "dump-git-layout" {
                let path = args.get(1).unwrap();
                dump_git_layout::dump_git_layout(&path);

                return Ok(());
            }
        }
    }

    gtk::init().map_err(|_| "Failed to initialize Gtk+".to_string())?;

    let gtk_executor = GtkEventLoopAsyncExecutor::new();

    let cpu_pool = CpuPool::new_num_cpus();

    let process_status_code  = Rc::new(Cell::<i32>::new(0));

    gtk_executor.spawn(
        main_flow(cpu_pool.clone(), gtk_executor.clone())
        .then(capture!(process_status_code; move |result| {
            if let Err(e) = result {
                eprintln!("{}", e);
                process_status_code.set(1);
            }

            gtk::main_quit();

            future::ok(())
        }))
    );

    gtk::main();

    std::process::exit(process_status_code.get());
}

fn main_flow(cpu_pool: CpuPool, gtk_executor: GtkEventLoopAsyncExecutor) -> Box<Future<Item=(), Error=String>> {

    use std::path::PathBuf;
    use screens::intro_screen::IntroScreen;
    use screens::main_screen::MainScreen;

    let args = std::env::args_os().collect::<Vec<_>>();

    let repo_path_fut: Box<Future<Item=PathBuf, Error=String>> = match args.get(1) {
        Some(x) => Box::new(futures::future::ok(PathBuf::from(x))),
        None => {
            let intro_screen = IntroScreen::new(gtk_executor.clone(), cpu_pool.clone());
            Box::new(intro_screen.show_and_pick_repo())
        },
    };

    let result = repo_path_fut
        .and_then(capture!(gtk_executor, cpu_pool; |repo_path| {

            println!("Loading git repository {}", repo_path.to_string_lossy());
            println!("Intro screen closed");
            
            MainScreen::create(gtk_executor, cpu_pool, repo_path)
            .and_then(|main_screen| {
                main_screen.show()
            })
            .or_else(|e| {
                use async_ui::promise::PromiseFuture;
                let promise = PromiseFuture::new();
                let dialog = gtk::MessageDialog::new::<gtk::Window>(
                    Option::None,
                    gtk::DialogFlags::MODAL,
                    gtk::MessageType::Error,
                    gtk::ButtonsType::Close,
                    &format!("Error showing main screen: {}", e)
                );
                dialog.connect_response(capture!(promise, dialog; move |_, _| {
                    dialog.destroy();
                    promise.reject(e.clone());
                }));

                dialog.show();

                promise
            })
            .and_then(|_| {
                println!("Main screen closed");
                future::ok(())
            })
        }));

    Box::new(result)

}
