extern crate gtk;

use gtk::prelude::*;

fn main() -> Result<(), String> {
    let args = std::env::args_os().collect::<Vec<_>>();

    let repo_path = args.get(1).ok_or("Not passed path to git repository")?;

    println!("Loading git repository {}", repo_path.to_string_lossy());

    gtk::init().map_err(|_| "Failed to initialize Gtk+".to_string())?;

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title(&repo_path.to_string_lossy());

    let list_store = gtk::ListStore::new(&[gtk::Type::String]);

    let tree_view = gtk::TreeView::new();

    let scrolled_window = gtk::ScrolledWindow::new(
        None,
        &tree_view.get_vadjustment()
    );

    scrolled_window.add(&tree_view);

    tree_view.set_model(&list_store);

    let cell_renderer = gtk::CellRendererText::new();
    let column = gtk::TreeViewColumn::new();
    column.set_title("Commit message");
    column.pack_start(&cell_renderer, true);
    column.add_attribute(&cell_renderer, "text", 0);

    tree_view.append_column(&column);

    {
        use std::process::Command;

        let mut cmd = Command::new("git");
        cmd
            .current_dir(&repo_path)
            .args(&["log", "--oneline"]);

        println!("Running {:?} at {:?}", cmd, repo_path);

        let cmd_output = cmd
            .output()
            .map_err(|e| e.to_string())?;

        if !cmd_output.status.success() {
            return Err(format!("Error executing {:?}:\n{}", cmd, String::from_utf8_lossy(&cmd_output.stderr)));
        }

        let output_str = String::from_utf8_lossy(&cmd_output.stdout);
        for line in output_str.lines() {
            list_store.insert_with_values(
                None,
                &[0],
                &[&line]
            );
        }
    }

    window.add(&scrolled_window);

    window.show_all();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    gtk::main();

    Ok(())
}
