// This code is taken from https://github.com/olback/tray-item-rs/blob/master/src/api/linux/mod.rs
// and was gently adapted for my needs
use crate::sync::remote::RemoteDaemon;
use anyhow::Result;
use gtk::prelude::*;
use libappindicator::{AppIndicator, AppIndicatorStatus};

pub struct Tray {
    tray: AppIndicator,
    menu: gtk::Menu,
}

impl Tray {
    pub fn setup(icon: &str, remote: RemoteDaemon) -> Result<Self> {
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

        t.add_menu_item("Stop Ocean", || -> Result<()> {
            gtk::main_quit();
            println!("Info: Stopped from tray. Exitting.");
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
