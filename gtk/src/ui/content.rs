use gtk::*;

pub struct Content {
    pub container:  Stack,
    pub image_view: ImageView,
}

impl Content {
    pub fn new() -> Content {
        let container = Stack::new();

        let image_view = ImageView::new();
        let devices_view = DevicesView::new();

        container.add_named(&image_view.container, "image");
        container.add_named(&devices_view.container, "devices");
        container.set_visible_child_name("image");

        Content {
            container,
            image_view,
        }
    }
}

pub struct DevicesView {
    pub container: Box,
}

impl DevicesView {
    pub fn new() -> DevicesView {
        let image = Image::new_from_icon_name("drive-removable-media-usb", 6);
        image.set_valign(Align::Start);

        let topic = Label::new("Select drives");
        topic.set_halign(Align::Start);
        topic.get_style_context().map(|c| c.add_class("h2"));

        let description = Label::new("Flashing will erase all data on the selected drives.");
        description.set_line_wrap(true);
        description.set_halign(Align::Start);

        let inner_container = Box::new(Orientation::Vertical, 0);
        inner_container.pack_start(&topic, false, false, 0);
        inner_container.pack_start(&description, false, false, 0);

        let container = Box::new(Orientation::Horizontal, 0);
        container.pack_start(&image, false, false, 0);
        container.pack_start(&inner_container, true, true, 0);

        DevicesView { container }
    }
}

pub struct ImageView {
    pub container:  Box,
    pub chooser:    Button,
    pub hash:       ComboBoxText,
    pub hash_label: Label,
}

impl ImageView {
    pub fn new() -> ImageView {
        let image = Image::new_from_icon_name("application-x-cd-image", 6);
        image.set_valign(Align::Start);

        let topic = Label::new("Choose an image");
        topic.set_halign(Align::Start);
        topic.get_style_context().map(|c| c.add_class("h2"));

        let description = Label::new(
            "Select the .iso or .img that you want to flash. You can also plug your USB drives in \
             now.",
        );
        description.set_line_wrap(true);
        description.set_halign(Align::Start);

        let chooser = Button::new_with_label("Choose Image");
        chooser.set_halign(Align::Center);
        chooser.set_halign(Align::Center);

        let hash = ComboBoxText::new();
        hash.append_text("SHA256");
        hash.append_text("SHA1");
        hash.append_text("MD5");
        hash.set_active(0);

        let hash_label = Label::new("");

        let hash_container = Box::new(Orientation::Horizontal, 0);
        hash_container.pack_start(&hash, false, false, 0);
        hash_container.pack_start(&hash_label, true, true, 0);

        let inner_container = Box::new(Orientation::Vertical, 0);
        inner_container.pack_start(&topic, false, false, 0);
        inner_container.pack_start(&description, false, false, 0);
        inner_container.pack_start(&chooser, true, false, 0);
        inner_container.pack_start(&hash_container, false, false, 0);

        let container = Box::new(Orientation::Horizontal, 0);
        container.pack_start(&image, false, false, 0);
        container.pack_start(&inner_container, true, true, 0);

        ImageView {
            container,
            chooser,
            hash,
            hash_label,
        }
    }
}
