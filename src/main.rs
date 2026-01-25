mod colors;
mod date_parser;
mod keybindings;
mod todo;
mod ui;

use gtk4::prelude::*;
use gtk4::Application;
use ui::ZapWindow;

fn main() {
    let app = Application::builder()
        .application_id("com.zap.todo")
        .build();

    app.connect_activate(|app| {
        let zap = ZapWindow::new(app);
        zap.window.present();
    });

    app.run();
}
