use gtk::prelude::*;
use futures::prelude::*;
use futures::future;

use futures_cpupool::CpuPool;

use crate::async_ui::promise::PromiseFuture;
use crate::async_ui::gtk_futures_executor::GtkEventLoopAsyncExecutor;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::rc::Rc;

pub struct MainScreenUi {
    executor: GtkEventLoopAsyncExecutor,
    window: gtk::Window,

    commits_list_store: gtk::ListStore,
    commits_tree_view: gtk::TreeView,

    diff_items_list_store: gtk::ListStore,
    diff_items_tree_view: gtk::TreeView,

    commit_info_view: gtk::TextView,
}

pub struct MainScreenBackend {
    cpu_pool: CpuPool,
    repo: Mutex<git2::Repository>,
    requested_commit: Mutex<Option<git2::Oid>>,
}

pub struct MainScreen {
    backend: Arc<MainScreenBackend>,
    ui: MainScreenUi,
}

impl MainScreen {
    pub fn new(executor: GtkEventLoopAsyncExecutor, cpu_pool: CpuPool, repo_path: PathBuf) -> impl Future<Item=Rc<MainScreen>, Error=String> {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_title(&repo_path.to_string_lossy());

        let commits_list_store = gtk::ListStore::new(&[
            gtk::Type::String, // Commit ID as string
            gtk::Type::String, // Commit message
            gtk::Type::String, // Commit date as string
            gtk::Type::String // Commit author email
        ]);

        let diff_items_list_store = gtk::ListStore::new(&[
            gtk::Type::String, // Parent OID; empty if this a "grouping" item
            gtk::Type::String, // File path
            gtk::Type::U8, // Change type, 0: none, 1: delete, 2: add, 3: rename, 0: update
            gtk::Type::String, // Renamed from path; empty if not renamed
        ]);

        let vpane = gtk::Paned::new(gtk::Orientation::Vertical);

        window.add(&vpane);

        let commits_tree_view = gtk::TreeView::new();

        let scrolled_window = gtk::ScrolledWindow::new(None, None);

        scrolled_window.add(&commits_tree_view);

        commits_tree_view.set_model(&commits_list_store);

        {
            let cell_renderer = gtk::CellRendererText::new();
            cell_renderer.set_property_ellipsize(pango::EllipsizeMode::End);
            let column = gtk::TreeViewColumn::new();
            column.set_title("Summary");
            column.set_resizable(true);
            column.set_expand(true);
            column.pack_start(&cell_renderer, true);
            column.add_attribute(&cell_renderer, "text", 1);

            commits_tree_view.append_column(&column);
        }

        {
            let cell_renderer = gtk::CellRendererText::new();
            let column = gtk::TreeViewColumn::new();
            column.set_title("Time");
            column.set_resizable(true);
            column.set_expand(false);
            column.pack_start(&cell_renderer, true);
            column.add_attribute(&cell_renderer, "text", 2);

            commits_tree_view.append_column(&column);
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

            commits_tree_view.append_column(&column);
        }

        vpane.pack1(&scrolled_window, true, false);

        let commit_info_view = gtk::TextView::new();
        commit_info_view.set_editable(false);

        let scrolled_window_2 = gtk::ScrolledWindow::new(None, None);

        vpane.pack2(&scrolled_window_2, true, false);

        let commit_info_viewport = gtk::Viewport::new(
            None,
            None
        );

        scrolled_window_2.add(&commit_info_viewport);

        let commit_info_vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
        commit_info_vbox.pack_start(&commit_info_view, false, false, 0);
        commit_info_viewport.add(&commit_info_vbox);

        let diff_items_tree_view = gtk::TreeView::new();
        commit_info_vbox.add(&diff_items_tree_view);

        diff_items_tree_view.set_model(&diff_items_list_store);

        {
            let column = gtk::TreeViewColumn::new();
            column.set_title("Change");
            column.set_resizable(true);
            column.set_expand(true);

            let cell_renderer_change_type = gtk::CellRendererText::new();
            cell_renderer_change_type.set_property_ellipsize(pango::EllipsizeMode::End);
            column.pack_start(&cell_renderer_change_type, false);
            column.add_attribute(&cell_renderer_change_type, "text", 2); /* 2: change type */
            
            let cell_renderer_path = gtk::CellRendererText::new();
            cell_renderer_path.set_property_ellipsize(pango::EllipsizeMode::End);
            column.pack_start(&cell_renderer_path, true);
            column.add_attribute(&cell_renderer_path, "text", 1); /* 1: file path */

            diff_items_tree_view.append_column(&column);
        }

        cpu_pool.spawn_fn(capture!(repo_path; move || {
            use git2::Repository;
            let repo = match Repository::discover(repo_path) {
                Ok(repo) => repo,
                Err(error) => return Box::new(future::err(format!("Error opening repository: {}", error))),
            };

            Box::new(future::ok(repo))
        }))
        .and_then(capture!(commits_list_store, commit_info_view, cpu_pool, executor; move |repo| {

            let main_screen = MainScreen {
                backend: Arc::new(MainScreenBackend {
                    cpu_pool,
                    repo: Mutex::new(repo),
                    requested_commit: Mutex::new(None),
                }),
                ui: MainScreenUi {
                    executor,
                    window,
                    commits_list_store,
                    commits_tree_view,
                    commit_info_view,
                    diff_items_list_store: diff_items_list_store,
                    diff_items_tree_view,
                },
            };

            let main_screen = Rc::new(main_screen);

            Self::subscribe(main_screen.clone());
            Box::new(future::ok(main_screen))
        }))
    }

    fn subscribe(main_screen: Rc<Self>) {
        main_screen.ui.commits_tree_view.get_selection().connect_changed(capture!(main_screen; move |selection| {
            let msg = match selection.get_selected() {
                None => "".to_owned(),
                Some((model, iter)) => {
                    let oid_str = model.get_value(&iter, 0).get::<String>().unwrap();
                    let oid = git2::Oid::from_str(&oid_str).unwrap();
                    main_screen.backend.requested_commit.lock().unwrap().replace(oid.clone());
                    main_screen.ui.executor.spawn(
                        main_screen.backend.cpu_pool
                        .spawn_fn(capture!(backend = main_screen.backend; move || -> Result<Option<_>, String> {
                            if Some(oid) != *backend.requested_commit.lock().unwrap() {
                                return Ok(None);
                            }

                            let repo = backend.repo.lock().unwrap();
                            let commit = repo.find_commit(oid).or_else(|e| Err(format!("{}", e)))?;

                            let message = String::from_utf8_lossy(
                                commit.message_bytes()
                            ).to_string();

                            let parents_count = commit.parent_count();

                            let mut commit_summary = String::new();

                            use std::fmt::Write;
                            use chrono::TimeZone;

                            write!(&mut commit_summary, "Commit {}\n", commit.id()).unwrap();
                            for parent in commit.parents() {
                                write!(&mut commit_summary, "Parent {}\n", parent.id()).unwrap();
                            }
                            let author_timestamp =
                                chrono::Utc.timestamp(commit.author().when().seconds(), 0)
                                .with_timezone(&chrono::FixedOffset::east(commit.author().when().offset_minutes() * 60));
                            let author_name = String::from_utf8_lossy(commit.author().name_bytes()).to_string();
                            let author_email = String::from_utf8_lossy(commit.author().email_bytes()).to_string();
                            let committer_timestamp =
                                chrono::Utc.timestamp(commit.committer().when().seconds(), 0)
                                .with_timezone(&chrono::FixedOffset::east(commit.committer().when().offset_minutes() * 60));
                            let committer_name = String::from_utf8_lossy(commit.committer().name_bytes()).to_string();
                            let committer_email = String::from_utf8_lossy(commit.committer().email_bytes()).to_string();

                            write!(
                                &mut commit_summary,
                                "Timestamp {}\nAuthor {} <{}>\n",
                                author_timestamp.format("%Y-%m-%d %H:%M:%S %:z"),
                                author_name,
                                author_email
                            ).unwrap();

                            if author_timestamp != committer_timestamp || author_name != committer_name || author_email != committer_email {
                                write!(
                                    &mut commit_summary,
                                    "Commit timestamp {}\nCommitter {} <{}>\n",
                                    committer_timestamp.format("%Y-%m-%d %H:%M:%S %:z"),
                                    committer_name,
                                    committer_email
                                ).unwrap();
                            }

                            write!(&mut commit_summary, "\n{}", message).unwrap();

                            let mut changes = Vec::new();

                            if parents_count > 0 {
                                for parent in commit.parents() {

                                    if parents_count > 1 {
                                        changes.push((
                                            "".to_string(),
                                            format!("Changes from {}", parent.id()),
                                            0,
                                            "".to_string()
                                        ));
                                    }

                                    let diff = repo.diff_tree_to_tree(
                                        Some(&parent.tree().or_else(|e| Err(format!("{}", e)))?),
                                        Some(&commit.tree().or_else(|e| Err(format!("{}", e)))?),
                                        None
                                    ).or_else(|e| Err(format!("{}", e)))?;

                                    for delta in diff.deltas() {
                                        let path = String::from_utf8_lossy(
                                                match (delta.new_file().path_bytes(), delta.old_file().path_bytes()) {
                                                    (Some(bytes), _) => bytes,
                                                    (None, Some(bytes)) => bytes,
                                                    (None, None) => "(none)".as_bytes(),
                                                }
                                            ).to_string();
                                        changes.push((
                                            "".to_string(), // TODO
                                            path,
                                            0, // TODO
                                            "".to_string() // TODO
                                        ));
                                    }
                                }
                            } else {
                                let diff = repo.diff_tree_to_tree(
                                    None,
                                    Some(&commit.tree().or_else(|e| Err(format!("{}", e)))?),
                                    None
                                ).or_else(|e| Err(format!("{}", e)))?;

                                for delta in diff.deltas() {
                                    let path = String::from_utf8_lossy(
                                            match (delta.new_file().path_bytes(), delta.old_file().path_bytes()) {
                                                (Some(bytes), _) => bytes,
                                                (None, Some(bytes)) => bytes,
                                                (None, None) => "(none)".as_bytes(),
                                            }
                                        ).to_string();
                                    changes.push((
                                        "".to_string(), // TODO
                                        path,
                                        0, // TODO
                                        "".to_string() // TODO
                                    ));
                                }
                            }

                            Ok(Some((commit_summary, changes)))
                        }))
                        .then(capture!(main_screen; move |result| {
                            match result {
                                Ok(Some((summary_text, changes))) => {
                                    main_screen.ui.commit_info_view.get_buffer().unwrap().set_text(&summary_text);
                                    main_screen.ui.diff_items_list_store.clear();
                                    for (commit_id, path, change_type, old_path) in changes {
                                        main_screen.ui.diff_items_list_store.insert_with_values(
                                            None,
                                            &[0, 1, 2, 3],
                                            &[
                                                &commit_id,
                                                &path,
                                                &change_type,
                                                &old_path
                                            ]
                                        );
                                    }
                                },
                                Ok(None) => {
                                    // Do nothing
                                },
                                Err(e) => {
                                    main_screen.ui.commit_info_view.get_buffer().unwrap().set_text(&e);
                                }
                            }

                            Ok(())
                        }))
                    );
                    
                    "Loading".to_owned()
                },
            };

            main_screen.ui.commit_info_view.get_buffer().unwrap().set_text(&msg);
        }));
    }

    pub fn show(&self) -> impl Future<Item=(), Error=String> {
        use std::result::Result::{Err, Ok};
        println!("Showing main screen");
        let result = PromiseFuture::new();

        self.ui.window.set_default_size(600, 800);
        self.ui.window.set_position(gtk::WindowPosition::Center);

        self.ui.window.show_all();

        self.ui.window.maximize();

        self.ui.window.connect_delete_event(capture!(result, window = self.ui.window; move |_, _| {
            result.resolve(());
            window.destroy();
            Inhibit(false)
        }));

        {
            self.ui.commits_list_store.insert_with_values(
                None,
                &[0, 1, 2, 3],
                &[&"", &"Loading...", &"", &""]
            );
        }

        self.ui.executor.spawn(future::lazy(capture!(
            backend = self.backend, commits_list_store = self.ui.commits_list_store, window = self.ui.window;
            move || {
            backend.cpu_pool.spawn_fn(capture!(backend; move || -> Box<Future<Item=_, Error=String>+Send> {
                let repo = backend.repo.lock().unwrap();

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
            })).then(capture!(commits_list_store, window; move |commits_result| {
                match commits_result {
                    Ok(commits) => {
                        commits_list_store.clear();
                        for (commit_id, summary, timestamp, author, email) in commits {
                            commits_list_store.insert_with_values(
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
                        commits_list_store.clear();
                        commits_list_store.insert_with_values(
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