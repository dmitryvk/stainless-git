extern crate glib;
extern crate gtk;
extern crate futures;
extern crate futures_cpupool;

#[macro_use]
mod async_ui;
mod screens;

use futures::prelude::*;
use futures_cpupool::CpuPool;

use crate::async_ui::gtk_futures_executor::GtkEventLoopAsyncExecutor;
use crate::screens::intro_screen::IntroScreen;
use crate::screens::main_screen::MainScreen;

fn main() -> Result<(), String> {
    let args = std::env::args_os().collect::<Vec<_>>();

    let repo_path = args.get(1).ok_or("Not passed path to git repository")?.clone();

    println!("Loading git repository {}", repo_path.to_string_lossy());

    gtk::init().map_err(|_| "Failed to initialize Gtk+".to_string())?;

    let gtk_executor = GtkEventLoopAsyncExecutor::new();

    let cpu_pool = CpuPool::new_num_cpus();

    gtk_executor.spawn(capture!(gtk_executor, cpu_pool; futures::future::lazy(move || {

        println!("Showing intro");
        let screen = IntroScreen::new(gtk_executor.clone(), cpu_pool.clone());

        let result = screen.show()
        .and_then(capture!(gtk_executor, cpu_pool; |_| {
            println!("Intro screen closed");
            
            let main_screen = MainScreen::new(gtk_executor, cpu_pool, repo_path);

            main_screen.show()
            .and_then(|_| {
                println!("Main screen closed");
                gtk::main_quit();
                futures::future::ok(())
            })
        }));

        result
    })));

    gtk::main();

    Ok(())
}
