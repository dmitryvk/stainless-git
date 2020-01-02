#[macro_use]
mod async_ui;
mod screens;
mod dag_layout;
mod dump_git_layout;

use gtk::prelude::*;

use async_ui::gtk_spawn_local;

fn main() -> Result<(), String> {
    use std::sync::Arc;
    use std::sync::Mutex;

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

    let process_status_code = Arc::new(Mutex::<i32>::new(0));

    gtk_spawn_local(capture!(process_status_code; async move {
        let result = main_flow().await;
        if let Err(e) = result {
            eprintln!("{}", e);
            *process_status_code.lock().expect("Mutex poisoned") = 1;
        }

        gtk::main_quit();
    }));

    gtk::main();

    std::process::exit(*process_status_code.lock().expect("Mutex poisoned"));
}

async fn main_flow() -> std::result::Result<(), String> {

    use std::path::PathBuf;
    use screens::intro_screen::IntroScreen;
    use screens::main_screen::MainScreen;

    let args = std::env::args_os().collect::<Vec<_>>();

    let repo_path = match args.get(1) {
        Some(x) => PathBuf::from(x),
        None => {
            let intro_screen = IntroScreen::new();
            intro_screen.show_and_pick_repo().await?
        },
    };

    println!("Loading git repository {}", repo_path.to_string_lossy());
    println!("Intro screen closed");
    
    match MainScreen::create(repo_path).await {
        Err(e) => {
            use async_ui::promise::Promise;
            let (promise, resolver) = Promise::new();
            let dialog = gtk::MessageDialog::new::<gtk::Window>(
                Option::None,
                gtk::DialogFlags::MODAL,
                gtk::MessageType::Error,
                gtk::ButtonsType::Close,
                &format!("Error showing main screen: {}", e)
            );
            dialog.connect_response(capture!(resolver, dialog; move |_, _| {
                dialog.destroy();
                resolver.resolve(());
            }));
    
            dialog.show();

            promise.await;
        },
        Ok(main_screen) => {
            MainScreen::show(main_screen.clone()).await;
        }
    };

    println!("Main screen closed");

    Ok(())
}
