extern crate gtk;

use gtk::prelude::*;

fn main() -> Result<(), String> {
    gtk::init().map_err(|_| "Failed to initialize Gtk+".to_string())?;

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("Привет, мир!");

    let list_store = gtk::ListStore::new(&[gtk::Type::String]);

    let tree_view = gtk::TreeView::new();

    tree_view.set_model(&list_store);

    let cell_renderer = gtk::CellRendererText::new();
    let column = gtk::TreeViewColumn::new();
    column.set_title("Commit message");
    column.pack_start(&cell_renderer, true);
    column.add_attribute(&cell_renderer, "text", 0);

    tree_view.append_column(&column);

    list_store.insert_with_values(
        None,
        &[0],
        &[&"Initial commit"]
    );

    window.add(&tree_view);

    window.show_all();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    gtk::main();

    Ok(())
}
