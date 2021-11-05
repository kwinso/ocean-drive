// This code is taken from https://github.com/olback/tray-item-rs/blob/master/src/api/linux/mod.rs
// and was gently adapted for my needs
use crate::sync::remote::RemoteDaemon;
use anyhow::Result;
use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};
use std::process::Command;
use webbrowser;

pub struct Tray {
    tray: AppIndicator,
    menu: gtk::Menu,
}

impl Tray {
    pub fn setup(
        icon: &str,
        remote: RemoteDaemon,
        remote_dir_id: String,
        local_path: String,
    ) -> Result<Self> {
        gtk::init()?;

        let mut t = Self {
            tray: AppIndicator::new("Ocean Drive", icon),
            menu: gtk::Menu::new(),
        };

        let mut version = String::from("Ocean Drive v");
        version.push_str(env!("CARGO_PKG_VERSION"));

        t.add_label(&version).unwrap();

        t.add_menu_item("Sync now", move || -> Result<()> {
            remote.sync()?;
            Ok(())
        })?;

        t.add_menu_item("Open in browser", move || -> Result<()> {

            match webbrowser::open(&format!(
                "https://drive.google.com/drive/folders/{}",
                remote_dir_id
            )) {
                Err(e) => {
                    eprintln!("Tray: Unable to open root directory in browser: {}", e) 
                }
                _ => {}

            }
            Ok(())
        })?;

        t.add_menu_item("Open local folder", move || -> Result<()> {
            match Command::new("xdg-open")
                .args([&local_path])
                .output() {
                    Err(e) => {
                        eprintln!("Tray: Failed to open local folder in default file explorer: {}", e);
                    },
                    _ => {}
                }

            Ok(())
        })?;

        t.add_menu_item("Stop Ocean", || -> Result<()> {
            gtk::main_quit();
            println!("Tray: Received stop command. Exitting.");
            std::process::exit(0);
        })
        .unwrap();

        t.set_icon(icon)?;

        Ok(t)
    }

    pub fn start(&self) {
        gtk::main();
    }

    fn set_icon(&mut self, icon: &str) -> Result<()> {
        self.tray.set_icon(icon);
        self.tray.set_status(AppIndicatorStatus::Active);

        Ok(())
    }

    fn add_label(&mut self, label: &str) -> Result<()> {
        let item = gtk::MenuItem::with_label(label.as_ref());
        item.set_sensitive(false);
        self.menu.append(&item);
        self.menu.show_all();
        self.tray.set_menu(&mut self.menu);

        Ok(())
    }

    fn add_menu_item<F>(&mut self, label: &str, cb: F) -> Result<()>
    where
        F: Fn() -> Result<()> + Send + Sync + 'static,
    {
        let item = gtk::MenuItem::with_label(label.as_ref());
        item.connect_activate(move |_| {
            cb().unwrap();
        });
        self.menu.append(&item);
        self.menu.show_all();
        self.tray.set_menu(&mut self.menu);

        Ok(())
    }
}
