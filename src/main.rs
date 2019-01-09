extern crate glib;
extern crate gtk;
extern crate futures;
extern crate futures_cpupool;

mod screens;
mod async_ui;

use gtk::prelude::*;
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

    let cpupool = CpuPool::new_num_cpus();

    // let result = screen.show();

    let gtk_executor_2 = gtk_executor.clone();
    let cpu_pool_2 = cpupool.clone();
    let gtk_executor_3 = gtk_executor.clone();
    let cpu_pool_3 = cpupool.clone();
    gtk_executor.spawn(futures::future::lazy(|| {

        println!("Showing intro");
        let screen = IntroScreen::new(gtk_executor_2, cpu_pool_2);

        let result = screen.show()
        .and_then(|_| {
            println!("Intro screen closed");
            
            let main_screen = MainScreen::new(gtk_executor_3, cpu_pool_3, repo_path);

            main_screen.show()
            .and_then(|_| {
                println!("Main screen closed");
                gtk::main_quit();
                futures::future::ok(())
            })
        });

        result
    }));

    gtk::main();

    Ok(())
}
