use eframe::egui;
use image::GenericImageView;

use crate::types::media_library::{MediaItem, MediaLibrary};

pub fn medialib_panel(
    ui: &mut egui::Ui,
    medialib: &mut MediaLibrary,
    _on_import: impl Fn(&mut MediaLibrary),
    on_remove: impl Fn(&mut MediaLibrary, usize),
) {
    ui.vertical(|ui| {
        ui.heading("Media Library");
        ui.separator();

        if ui.button("Import Media").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Media", &["mp4", "mov", "mkv", "mp3", "wav", "ogg", "flac"])
                .pick_file()
            {
                medialib.add_file(&path);
            }
        }

        if medialib.all_items().is_empty() {
            ui.label("No media found");
        } else {
            // Compact grid card layout
            let card_width = 56.0;
            let thumb_size = egui::vec2(48.0, 27.0);
            let items_per_row = (ui.available_width() / card_width).floor() as usize;
            let items = medialib.all_items();
            let mut to_remove = Vec::new();

            for row in items.chunks(items_per_row.max(1)) {
                ui.horizontal(|ui| {
                    for (i, item) in row.iter().enumerate() {
                        let item_id = egui::Id::new(("media_drag", i));
                        let drag_payload = item.clone();
                        ui.dnd_drag_source(item_id, drag_payload, |ui| {
                            ui.vertical(|ui| {
                                // Icon only (no thumbnail)
                                match item {
                                    MediaItem::VideoItem(_) => {
                                        ui.label("🎬");
                                    }
                                    MediaItem::AudioItem(_) => {
                                        ui.label("🎵");
                                    }
                                }
                                // Filename below, small font, ellipsized
                                let name = match item {
                                    MediaItem::AudioItem(audio) => &audio.file_descriptor.file_name,
                                    MediaItem::VideoItem(video) => &video.file_descriptor.file_name,
                                };
                                ui.label(
                                    egui::RichText::new(name)
                                        .size(9.0)
                                        .color(egui::Color32::GRAY),
                                );
                                // Compact remove button
                                if ui.button("✖").clicked() {
                                    let idx = items
                                        .iter()
                                        .position(|x| std::ptr::eq(x, item))
                                        .unwrap_or(i);
                                    to_remove.push(idx);
                                }
                            });
                            ui.add_space(thumb_size.y + 20.0);
                        });
                    }
                });
            }
            // Remove items after iteration to avoid borrow conflict
            to_remove.sort_unstable();
            to_remove.dedup();
            for i in to_remove.into_iter().rev() {
                on_remove(medialib, i);
            }
        }
    });
}
