pub mod promise;
#[macro_use]
pub mod capture;

pub fn gtk_spawn_local(task: impl std::future::Future<Output=()> + 'static) {
    let mc = glib::MainContext::default();
    
    mc.spawn_local(task);
}