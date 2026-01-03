use win32_notif::{
    NotificationBuilder, ToastsNotifier,
    notification::visual::{Text, text::HintStyle},
};

pub fn send_toast() {
    let notifier = ToastsNotifier::new("Cloudreve.Sync").unwrap();

    let notif = NotificationBuilder::new()
        .visual(
            Text::create(0, "Welcome to \"win32_notif\"!! 👋")
                .with_align_center(true)
                .with_wrap(true)
                .with_style(HintStyle::Title),
        )
        .visual(
            Text::create_binded(1, "desc")
                .with_align_center(true)
                .with_wrap(true)
                .with_style(HintStyle::Body),
        )
        .value("desc", "Data binding works as well {WOW}!")
        .build(0, &notifier, "01", "readme")
        .unwrap();

    notif.show().unwrap();
}
