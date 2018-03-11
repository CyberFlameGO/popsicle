use super::{hash, App, FlashTask, OpenDialog};

use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use gtk;
use gtk::*;
use muff::{self, DiskError, Image};

pub struct Connected(App);

impl Connected {
    /// Display the window, and execute the gtk main event loop.
    pub fn then_execute(self) {
        self.0.window.show_all();
        gtk::main();
    }
}

pub trait Connect {
    /// Creates external state, and maps all of the UI functionality to the UI.
    fn connect_events(self) -> Connected;

    /// Programs the button for selecting an image.
    fn connect_image_chooser(&self);

    /// Programs the combo box which generates the hash sum for initial image selection view.
    fn connect_hash_generator(&self);

    /// Programs the back button, whose behavior changes based on the currently active view.
    fn connect_back_button(&self);

    /// Programs the next button, whose behavior changes based on the currently active view.
    fn connect_next_button(&self);

    /// Programs the action that will be performed when the check all button is clicked.
    fn connect_check_all(&self);

    /// Adds a function for GTK to execute when the application is idle, to monitor and
    /// update the progress bars for devices that are being flashed, and to generate
    /// the summary view after all devices have been flashed.
    fn watch_flashing_devices(&self);
}

impl Connect for App {
    fn connect_events(self) -> Connected {
        self.connect_image_chooser();
        self.connect_hash_generator();
        self.connect_back_button();
        self.connect_next_button();
        self.connect_check_all();
        self.watch_flashing_devices();

        Connected(self)
    }

    fn connect_image_chooser(&self) {
        let path_label = self.content.image_view.image_path.clone();
        let next = self.header.next.clone();
        let image_data = self.state.image_data.clone();
        self.content.image_view.chooser.connect_clicked(move |_| {
            if let Some(path) = OpenDialog::new(None).run() {
                let mut new_image = match Image::new(&path) {
                    Ok(image) => image,
                    Err(why) => {
                        eprintln!("muff: unable to open image: {}", why);
                        return;
                    }
                };

                let new_data = match new_image.read(|_| ()) {
                    Ok(data) => data,
                    Err(why) => {
                        eprintln!("muff: unable to read image: {}", why);
                        return;
                    }
                };

                path_label.set_label(&path.file_name().unwrap().to_string_lossy());
                next.set_sensitive(true);
                *image_data.borrow_mut() = Some(Arc::new(new_data));
            }
        });
    }

    fn connect_hash_generator(&self) {
        let image_data = self.state.image_data.clone();
        let hash_label = self.content.image_view.hash_label.clone();
        self.content.image_view.hash.connect_changed(move |hash| {
            if let Some(ref data) = *image_data.borrow() {
                hash_label.set_icon_from_icon_name(EntryIconPosition::Primary, "gnome-spinner");
                hash_label.set_icon_sensitive(EntryIconPosition::Primary, true);
                hash::set(&hash_label, hash.get_active_text().unwrap().as_str(), data);
                hash_label.set_icon_sensitive(EntryIconPosition::Primary, false);
            }
        });
    }

    fn connect_back_button(&self) {
        let stack = self.content.container.clone();
        let back = self.header.back.clone();
        let next = self.header.next.clone();
        let view = self.state.view.clone();
        back.connect_clicked(move |back| {
            match *view.borrow() {
                0 => gtk::main_quit(),
                1 => {
                    stack.set_transition_type(StackTransitionType::SlideRight);
                    stack.set_visible_child_name("image");
                    back.set_label("Cancel");
                    next.set_label("Next");
                    next.set_sensitive(true);
                    next.get_style_context().map(|c| {
                        c.remove_class("destructive-action");
                        c.add_class("suggested-action");
                    });
                }
                _ => unreachable!(),
            }
            *view.borrow_mut() -= 1;
        });
    }

    fn connect_next_button(&self) {
        let stack = self.content.container.clone();
        let back = self.header.back.clone();
        let next = self.header.next.clone();
        let list = self.content.devices_view.list.clone();
        let tasks = self.state.tasks.clone();
        let task_handles = self.state.task_handles.clone();
        let summary_grid = self.content.flash_view.progress_list.clone();
        let bars = self.state.bars.clone();
        let view = self.state.view.clone();
        let device_list = self.state.devices.clone();
        let image_data = self.state.image_data.clone();
        next.connect_clicked(move |next| {
            stack.set_transition_type(StackTransitionType::SlideLeft);
            match *view.borrow() {
                // Move to device selection screen
                0 => {
                    back.set_label("Back");
                    next.set_label("Flash");
                    next.get_style_context().map(|c| {
                        c.remove_class("suggested-action");
                        c.add_class("destructive-action");
                    });
                    stack.set_visible_child_name("devices");

                    // Remove all but the first row
                    list.get_children()
                        .into_iter()
                        .skip(1)
                        .for_each(|widget| widget.destroy());

                    let mut devices = vec![];
                    if let Err(why) = muff::get_disk_args(&mut devices) {
                        eprintln!("muff: unable to get devices: {}", why);
                    }

                    let mut device_list = device_list.lock().unwrap();
                    for device in &devices {
                        let name = Path::new(&device).canonicalize().unwrap();
                        let button = CheckButton::new_with_label(&name.to_string_lossy());
                        list.insert(&button, -1);
                        device_list.push((device.clone(), button));
                    }

                    list.show_all();
                }
                // Begin the device flashing process
                1 => {
                    let device_list = device_list.lock().unwrap();
                    let devs = device_list.iter().map(|x| x.0.clone());
                    // TODO: Handle Error
                    let mounts = muff::Mount::all().unwrap();
                    // TODO: Handle Error
                    let disks = muff::disks_from_args(devs, &mounts, true).unwrap();

                    back.set_visible(false);
                    next.set_visible(false);
                    stack.set_visible_child_name("flash");

                    // Clear the progress bar summaries.
                    let mut bars = bars.borrow_mut();
                    bars.clear();
                    summary_grid.get_children().iter().for_each(|c| c.destroy());

                    let mut tasks = tasks.lock().unwrap();
                    let mut task_handles = task_handles.lock().unwrap();
                    for (id, (disk_path, mut disk)) in disks.into_iter().enumerate() {
                        let id = id as i32;
                        let image_data = image_data.borrow();
                        let image_data = image_data.as_ref().unwrap().clone();
                        let progress = Arc::new(AtomicUsize::new(0));
                        let finished = Arc::new(AtomicUsize::new(0));
                        let bar = ProgressBar::new();
                        bar.set_hexpand(true);
                        let label = Label::new(
                            Path::new(&disk_path)
                                .canonicalize()
                                .unwrap()
                                .to_str()
                                .unwrap(),
                        );
                        label.set_justify(Justification::Right);
                        label
                            .get_style_context()
                            .map(|c| c.add_class("progress-label"));
                        summary_grid.attach(&label, 0, id, 1, 1);
                        summary_grid.attach(&bar, 1, id, 1, 1);
                        bars.push(bar);

                        // Spawn a thread that will update the progress value over time.
                        //
                        // The value will be stored within an intermediary atomic integer,
                        // because it is unsafe to send GTK widgets across threads.
                        task_handles.push({
                            let progress = progress.clone();
                            let finished = finished.clone();
                            thread::spawn(move || -> Result<(), DiskError> {
                                let result = muff::write_to_disk(
                                    |_msg| (),
                                    || (),
                                    |value| progress.store(value as usize, Ordering::SeqCst),
                                    disk,
                                    disk_path,
                                    image_data.len() as u64,
                                    &image_data,
                                    false,
                                );

                                finished.store(1, Ordering::SeqCst);
                                result
                            })
                        });

                        tasks.push(FlashTask { progress, finished });
                    }

                    summary_grid.show_all();
                }
                2 => gtk::main_quit(),
                _ => unreachable!(),
            }

            *view.borrow_mut() += 1;
        });
    }

    fn connect_check_all(&self) {
        let all = self.content.devices_view.select_all.clone();
        let devices = self.state.devices.clone();
        all.connect_clicked(move |all| {
            if all.get_active() {
                devices
                    .lock()
                    .unwrap()
                    .iter()
                    .for_each(|&(_, ref device)| device.set_active(true));
            }
        });
    }

    fn watch_flashing_devices(&self) {
        let tasks = self.state.tasks.clone();
        let bars = self.state.bars.clone();
        let image_data = self.state.image_data.clone();
        let stack = self.content.container.clone();
        let next = self.header.next.clone();

        gtk::timeout_add(1000, move || {
            let image_length = match *image_data.borrow() {
                Some(ref data) => data.len() as f64,
                None => {
                    return Continue(true);
                }
            };

            let tasks = tasks.lock().unwrap();
            if tasks.is_empty() {
                return Continue(true);
            }

            let mut finished = true;
            for (task, bar) in tasks.deref().iter().zip(bars.borrow().iter()) {
                let value = if task.finished.load(Ordering::SeqCst) == 1 {
                    1.0f64
                } else {
                    finished = false;
                    task.progress.load(Ordering::SeqCst) as f64 / image_length
                };

                bar.set_fraction(value);
            }

            if finished {
                stack.set_visible_child_name("summary");
                next.set_label("Finished");
                next.get_style_context()
                    .map(|c| c.remove_class("destructive-action"));
                next.set_visible(true);

                // TODO: Generate summary here

                Continue(false)
            } else {
                Continue(true)
            }
        });
    }
}
