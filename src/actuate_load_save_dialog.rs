use std::fs;
use std::path::{Path, PathBuf};

use nih_plug::nih_log;
use nih_plug_egui::egui::{self, Id, Ui, Vec2};

pub struct FileDialog {
    current_dir: PathBuf,
    selected: Option<PathBuf>,
    input_filename: String,
    pub open: bool,
    dialog_mode: DialogMode,
    result: Option<PathBuf>,
    id: Id,
}

#[derive(PartialEq)]
pub enum DialogMode {
    OpenPreset,
    ExportPreset,
    OpenSample,
    ExportSample,
}

impl FileDialog {
    pub fn new(start_path: PathBuf, dialog_mode: DialogMode) -> Self {
        let dir = if start_path.is_dir() {
            start_path
        } else {
            start_path.parent().unwrap_or(Path::new(".")).to_path_buf()
        };

        FileDialog {
            current_dir: dir,
            selected: None,
            input_filename: String::new(),
            open: false,
            dialog_mode,
            result: None,
            id: egui::Id::new(format!("file_dialog_{}", rand::random::<u64>()))
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<PathBuf> {
        let mut result = None;

        if self.open {
            egui::Window::new(match self.dialog_mode {
                DialogMode::ExportPreset => { "Export Preset" },
                DialogMode::OpenPreset => { "Open Preset" },
                DialogMode::ExportSample => { "Export Sample" },
                DialogMode::OpenSample => { "Open Sample" },
            })
                .collapsible(false)
                .resizable(true)
                 .id(self.id)
                .max_size(Vec2::new(800.0,480.0))
                .show(ctx, |ui| {
                    self.ui_contents(ui);

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.open = false;
                            self.selected = None;
                            // Kind of jank, but it works with the structure outside this
                            Some(PathBuf::new());
                        }

                        if self.dialog_mode == DialogMode::ExportPreset || self.dialog_mode == DialogMode::ExportSample {
                            if ui.button("Save").clicked() {
                                if !self.input_filename.is_empty() {
                                    let path = self.current_dir.join(&self.input_filename);
                                    nih_log!("Path: {:?}", path.clone());
                                    self.result = Some(path.clone());
                                    result = Some(path);
                                    self.selected = None;
                                    nih_log!("Filename: {:?}", self.input_filename);
                                }
                            }
                        } else {
                            if ui.button("Open").clicked() {
                                if let Some(sel) = &self.selected {
                                    self.result = Some(sel.clone());
                                    result = Some(sel.clone());
                                    self.selected = None;
                                }
                            }
                        }
                    });
                });
        }

        if self.result.is_some() {
            let r = self.result.clone();
            self.result = None;
            return r;
        }

        result
    }

    fn ui_contents(&mut self, ui: &mut Ui) {
        ui.label(format!("Current directory: {}", self.current_dir.display()));

        // List directory contents
        let mut entries: Vec<PathBuf> = Vec::new();
        let mut cloned_back_dir = self.current_dir.clone();
        cloned_back_dir.pop();
        entries.push(cloned_back_dir);

        let main_entries = fs::read_dir(&self.current_dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        for entry in main_entries {
            entries.push(entry);
        }

        egui::ScrollArea::vertical()
            .max_height(480.0)
            .show(ui, |ui| {
            for entry in entries {
                let is_dir = entry.is_dir();
                let name = entry.file_name().unwrap_or_default().to_string_lossy();

                if is_dir {
                    if ui.button(format!("[{}]", name)).clicked() {
                        self.current_dir = entry;
                        self.selected = None;
                    }
                } else {
                    let selected = self.selected.as_ref() == Some(&entry);
                    if ui.selectable_label(selected, name).clicked() {
                        self.selected = Some(entry.clone());
                        self.input_filename =
                            entry.file_name().unwrap_or_default().to_string_lossy().into();
                    }
                }
            }
        });

        if self.dialog_mode == DialogMode::ExportPreset || self.dialog_mode == DialogMode::ExportSample {
            ui.separator();
            ui.label("File name:");
            ui.scope(|ui| {
                ui.push_id("file_name_input", |ui| {
                    ui.text_edit_singleline(&mut self.input_filename);
                });
            });
        }
    }
}
