use gtk::prelude::*;
use futures::prelude::*;
use futures::future;

use futures_cpupool::CpuPool;

use crate::async_ui::promise::PromiseFuture;
use crate::async_ui::gtk_futures_executor::GtkEventLoopAsyncExecutor;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

pub struct MainScreen {
    executor: GtkEventLoopAsyncExecutor,
    cpu_pool: CpuPool,
    window: gtk::Window,
    repo: Arc<Mutex<git2::Repository>>,

    list_store: gtk::ListStore,

    commit_info_view: gtk::TextView,
    
}

impl MainScreen {
    pub fn new(executor: GtkEventLoopAsyncExecutor, cpu_pool: CpuPool, repo_path: PathBuf) -> impl Future<Item=MainScreen, Error=String> {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title(&repo_path.to_string_lossy());

        let list_store = gtk::ListStore::new(&[
            gtk::Type::String, // Commit ID as string
            gtk::Type::String, // Commit message
            gtk::Type::String, // Commit date as string
            gtk::Type::String // Commit author email
        ]);

        let vpane = gtk::Paned::new(gtk::Orientation::Vertical);

        window.add(&vpane);

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
            column.add_attribute(&cell_renderer, "text", 1);

            tree_view.append_column(&column);
        }

        {
            let cell_renderer = gtk::CellRendererText::new();
            let column = gtk::TreeViewColumn::new();
            column.set_title("Time");
            column.set_resizable(true);
            column.set_expand(false);
            column.pack_start(&cell_renderer, true);
            column.add_attribute(&cell_renderer, "text", 2);

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
            column.add_attribute(&cell_renderer, "text", 3);

            tree_view.append_column(&column);
        }

        vpane.pack1(&scrolled_window, true, false);

        let commit_info_view = gtk::TextView::new();

        let scrolled_window_2 = gtk::ScrolledWindow::new(None, None);

        vpane.pack2(&scrolled_window_2, true, false);
        scrolled_window_2.add(&commit_info_view);

        cpu_pool.spawn_fn(capture!(repo_path; move || -> Box<Future<Item=_, Error=String>+Send> {
            use git2::Repository;
            let repo = match Repository::discover(repo_path) {
                Ok(repo) => repo,
                Err(error) => return Box::new(future::err(format!("Error opening repository: {}", error))),
            };

            Box::new(future::ok(Arc::new(Mutex::new(repo))))
        }))
        .and_then(capture!(commit_info_view, cpu_pool, executor; move |repo| {
            tree_view.get_selection().connect_changed(capture!(commit_info_view, repo; move |selection| {
                let msg = match selection.get_selected() {
                    None => "".to_owned(),
                    Some((model, iter)) => {
                        let oid_str = model.get_value(&iter, 0).get::<String>().unwrap();
                        executor.spawn(
                            cpu_pool
                            .spawn_fn(capture!(repo; move || -> Result<_, String> {
                                let oid = git2::Oid::from_str(&oid_str).or_else(|e| Err(format!("{}", e)))?;
                                let repo = repo.lock().unwrap();
                                let commit = repo.find_commit(oid).or_else(|e| Err(format!("{}", e)))?;

                                let message = String::from_utf8_lossy(
                                    commit.message_bytes()
                                ).to_string();

                                let parents_count = commit.parent_count();

                                let result = format!("{}\n\nParents count: {}", message, parents_count);

                                Ok(result)
                            }))
                            .then(capture!(commit_info_view; move |result| {
                                match result {
                                    Ok(message) => {
                                        commit_info_view.get_buffer().unwrap().set_text(&message);
                                    },
                                    Err(e) => {
                                        commit_info_view.get_buffer().unwrap().set_text(&e);
                                    }
                                }

                                Ok(())
                            }))
                        );
                        
                        "Loading".to_owned()
                    },
                };

                commit_info_view.get_buffer().unwrap().set_text(&msg);
            }));

            Box::new(future::ok(repo))
        }))
        .and_then(move |repo| {
            future::ok(
                MainScreen {
                    executor,
                    cpu_pool,
                    window,
                    list_store,
                    commit_info_view,
                    repo: repo,
                }
            )
        })
    }

    pub fn show(&self) -> impl Future<Item=(), Error=String> {
        use std::result::Result::{Err, Ok};
        println!("Showing main screen");
        let result = PromiseFuture::new();

        self.window.set_default_size(600, 800);
        self.window.set_position(gtk::WindowPosition::Center);

        self.window.show_all();

        self.window.maximize();

        self.window.connect_delete_event(capture!(result, window = self.window; move |_, _| {
            result.resolve(());
            window.destroy();
            Inhibit(false)
        }));

        {
            self.list_store.insert_with_values(
                None,
                &[0, 1, 2, 3],
                &[&"", &"Loading...", &"", &""]
            );
        }

        self.executor.spawn(future::lazy(capture!(
            cpu_pool = self.cpu_pool, repo = self.repo, list_store = self.list_store, window = self.window;
            move || {
            cpu_pool.spawn_fn(move || -> Box<Future<Item=_, Error=String>+Send> {
                let repo = repo.lock().unwrap();

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
                    let commit_id = format!("{}", commit.id());
                    let summary = String::from_utf8_lossy(commit.summary_bytes().unwrap_or(&[])).to_string();
                    use chrono::TimeZone;
                    let timestamp =
                        chrono::Utc.timestamp(commit.author().when().seconds(), 0)
                        .with_timezone(&chrono::FixedOffset::east(commit.author().when().offset_minutes() * 60));
                    let author = String::from_utf8_lossy(commit.author().name_bytes()).to_string();
                    let email = String::from_utf8_lossy(commit.author().email_bytes()).to_string();

                    (commit_id, summary, timestamp, author, email)
                }).collect::<Vec<_>>();

                Box::new(future::ok(commit_infos))
            }).then(capture!(list_store, window; move |commits_result| {
                match commits_result {
                    Ok(commits) => {
                        list_store.clear();
                        for (commit_id, summary, timestamp, author, email) in commits {
                            list_store.insert_with_values(
                                None,
                                &[0, 1, 2, 3],
                                &[
                                    &commit_id,
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
                            &[&msg.to_string()]
                        );
                    }
                }

                future::ok::<_, _>(())
            }))
        })));

        result
    }
}