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

        let list_store = gtk::ListStore::new(&[
            gtk::Type::String,
            gtk::Type::String,
            gtk::Type::String
        ]);

        let tree_view = gtk::TreeView::new();

        let scrolled_window = gtk::ScrolledWindow::new(None, None);

        scrolled_window.add(&tree_view);

        tree_view.set_model(&list_store);

        {
            let cell_renderer = gtk::CellRendererText::new();
            cell_renderer.set_property_ellipsize(pango::EllipsizeMode::End);
            let column = gtk::TreeViewColumn::new();
            column.set_title("Summary");
            column.set_resizable(true);
            column.set_expand(true);
            column.pack_start(&cell_renderer, true);
            column.add_attribute(&cell_renderer, "text", 0);

            tree_view.append_column(&column);
        }

        {
            let cell_renderer = gtk::CellRendererText::new();
            let column = gtk::TreeViewColumn::new();
            column.set_title("Time");
            column.set_resizable(true);
            column.set_expand(false);
            column.pack_start(&cell_renderer, true);
            column.add_attribute(&cell_renderer, "text", 1);

            tree_view.append_column(&column);
        }

        {
            let cell_renderer = gtk::CellRendererText::new();
            cell_renderer.set_property_ellipsize(pango::EllipsizeMode::End);
            let column = gtk::TreeViewColumn::new();
            column.set_title("Author");
            column.set_resizable(true);
            column.set_expand(true);
            column.pack_start(&cell_renderer, true);
            column.add_attribute(&cell_renderer, "text", 2);

            tree_view.append_column(&column);
        }

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
        use std::result::Result::{Err, Ok};
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
                &[0, 1, 2],
                &[&"Loading...", &"", &""]
            );
        }

        self.executor.spawn(future::lazy(capture!(
            cpu_pool = self.cpu_pool, repo_path = self.repo_path, list_store = self.list_store, window = self.window;
            move || {
            cpu_pool.spawn_fn(move || -> Box<Future<Item=_, Error=String>+Send> {
                use git2::Repository;
                let repo = match Repository::discover(repo_path) {
                    Ok(repo) => repo,
                    Err(error) => return Box::new(future::err(format!("Error opening repository: {}", error))),
                };

                let mut revwalk = match repo.revwalk() {
                    Ok(revwalk) => revwalk,
                    Err(error) => return Box::new(future::err(format!("Error loading commit graph: {}", error))),
                };

                revwalk.set_sorting(git2::Sort::TIME | git2::Sort::TOPOLOGICAL/* | git2::Sort::REVERSE*/);

                if let Err(e) = revwalk.push_head() {
                    return Box::new(future::err(format!("Error initializing commit graph walk: {}", e)));
                }

                let commit_ids = match revwalk.collect::<Result<Vec<git2::Oid>, _>>() {
                    Ok(commits) => commits,
                    Err(error) => return Box::new(future::err(format!("Error iterating over commits: {}", error))),
                };

                let commits = commit_ids.into_iter().map(|commit_id| repo.find_commit(commit_id));

                let commits = match commits.collect::<Result<Vec<git2::Commit>, _>>() {
                    Ok(commits) => commits,
                    Err(e) => return Box::new(future::err(format!("Error reading commit: {}", e))),
                };

                let commit_infos = commits.into_iter().map(|commit| {
                    let summary = String::from_utf8_lossy(commit.summary_bytes().unwrap_or(&[])).to_string();
                    use chrono::TimeZone;
                    let timestamp =
                        chrono::Utc.timestamp(commit.author().when().seconds(), 0)
                        .with_timezone(&chrono::FixedOffset::east(commit.author().when().offset_minutes() * 60));
                    let author = String::from_utf8_lossy(commit.author().name_bytes()).to_string();
                    let email = String::from_utf8_lossy(commit.author().email_bytes()).to_string();

                    (summary, timestamp, author, email)
                }).collect::<Vec<_>>();

                Box::new(future::ok(commit_infos))
            }).then(capture!(list_store, window; move |commits_result| {
                match commits_result {
                    Ok(commits) => {
                        list_store.clear();
                        for (summary, timestamp, author, email) in commits {
                            list_store.insert_with_values(
                                None,
                                &[0, 1, 2],
                                &[
                                    &summary,
                                    &timestamp.format("%Y-%m-%d %H:%M:%S %:z").to_string(),
                                    &format!("{} <{}>", author, email)
                                ]
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
                            &[&format!("{}", msg)]
                        );
                    }
                }

                future::ok::<_, ()>(())
            }))
        })));

        result
    }
}