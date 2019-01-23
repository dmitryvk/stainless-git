#[macro_use]
mod async_ui;
mod screens;

use futures::prelude::*;
use futures::future;
use futures_cpupool::CpuPool;

use async_ui::gtk_futures_executor::GtkEventLoopAsyncExecutor;

fn main() -> Result<(), String> {
    use std::rc::Rc;
    use std::cell::Cell;

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
            
            let main_screen = MainScreen::new(gtk_executor, cpu_pool, repo_path);

            main_screen.show()
            .map_err(|_| "Error showing main screen".to_string())
            .and_then(|_| {
                println!("Main screen closed");
                future::ok(())
            })
        }));

    Box::new(result)

}
