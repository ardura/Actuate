// Actuate GUI
// Builds the EGUI editor outside of the main file because it is huge
// Ardura

use std::{collections::HashMap, ffi::OsStr, ops::RangeInclusive, path::{Path, PathBuf}, sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex, RwLock}};
use egui_file::{FileDialog, State};
use nih_plug::{context::gui::AsyncExecutor, editor::Editor, nih_log};
use nih_plug_egui::{create_egui_editor, egui::{self, Color32, Pos2, Rect, RichText, Rounding, ScrollArea, Vec2}, widgets::ParamSlider};
use walkdir::WalkDir;

use crate::{actuate_enums::PresetBrowserEntry, CustomWidgets::ComboBoxParam};
#[allow(unused_imports)]
use crate::{
    actuate_enums::{
        AMFilterRouting, FilterAlgorithms, LFOSelect, ModulationDestination, ModulationSource, PresetType, UIBottomSelection}, actuate_structs::ActuatePresetV131, audio_module::{AudioModule, AudioModuleType}, Actuate, ActuateParams, CustomWidgets::{
            slim_checkbox, toggle_switch, ui_knob::{self, KnobLayout}, BeizerButton::{self, ButtonLayout}, BoolButton, CustomParamSlider, CustomVerticalSlider::ParamSlider as VerticalParamSlider}, A_BACKGROUND_COLOR_TOP, DARKER_GREY_UI_COLOR, DARKEST_BOTTOM_UI_COLOR, DARK_GREY_UI_COLOR, FONT, FONT_COLOR, HEIGHT, LIGHTER_GREY_UI_COLOR, MEDIUM_GREY_UI_COLOR, SMALLER_FONT, TEAL_GREEN, WIDTH, YELLOW_MUSTARD};

pub(crate) fn make_actuate_gui(instance: &mut Actuate, _async_executor: AsyncExecutor<Actuate>) -> Option<Box<dyn Editor>> {
        let params: Arc<ActuateParams> = instance.params.clone();
        let arc_preset: Arc<Mutex<ActuatePresetV131>> = Arc::clone(&instance.current_loaded_params);
        let clear_voices: Arc<AtomicBool> = Arc::clone(&instance.clear_voices);
        let reload_entire_preset: Arc<AtomicBool> = Arc::clone(&instance.reload_entire_preset);
        let browse_preset_active: Arc<AtomicBool> = Arc::clone(&instance.browsing_presets);
        let import_preset_active: Arc<AtomicBool> = Arc::clone(&instance.importing_presets);
        let export_preset_active: Arc<AtomicBool> = Arc::clone(&instance.exporting_presets);
        let safety_clip_output: Arc<Mutex<bool>> = Arc::clone(&instance.safety_clip_output);
        let AM1: Arc<Mutex<AudioModule>> = Arc::clone(&instance.audio_module_1);
        let AM2: Arc<Mutex<AudioModule>> = Arc::clone(&instance.audio_module_2);
        let AM3: Arc<Mutex<AudioModule>> = Arc::clone(&instance.audio_module_3);

        let update_current_preset: Arc<AtomicBool> = Arc::clone(&instance.update_current_preset);
        let filter_select_outside: Arc<Mutex<UIBottomSelection>> =
            Arc::new(Mutex::new(UIBottomSelection::Filter1));
        let lfo_select_outside: Arc<Mutex<LFOSelect>> = Arc::new(Mutex::new(LFOSelect::INFO));

        let filter_acid = instance.filter_acid.clone();
        let filter_analog = instance.filter_analog.clone();
        let filter_bright = instance.filter_bright.clone();
        let filter_chord = instance.filter_chord.clone();
        let filter_crisp = instance.filter_crisp.clone();
        let filter_deep = instance.filter_deep.clone();
        let filter_delicate = instance.filter_delicate.clone();
        let filter_hard = instance.filter_hard.clone();
        let filter_harsh = instance.filter_harsh.clone();
        let filter_lush = instance.filter_lush.clone();
        let filter_mellow = instance.filter_mellow.clone();
        let filter_resonant = instance.filter_resonant.clone();
        let filter_rich = instance.filter_rich.clone();
        let filter_sharp = instance.filter_sharp.clone();
        let filter_silky = instance.filter_silky.clone();
        let filter_smooth = instance.filter_smooth.clone();
        let filter_soft = instance.filter_soft.clone();
        let filter_stab = instance.filter_stab.clone();
        let filter_warm = instance.filter_warm.clone();
        let dir_files_map = instance.dir_files_map.clone();
        let str_files_map = instance.str_files_map.clone();
        let lite_db = instance.preset_browser_lite_db.clone();

        let mut home_dir = PathBuf::new();
        let home_location = dirs::home_dir().expect("Unable to determine home directory");
        home_dir.push(home_location);

        let default_dir_temp = dirs::document_dir();
        let mut default_dir: PathBuf = home_dir.clone();
        if default_dir_temp.is_some() {
            default_dir = default_dir_temp.unwrap().as_path().join("ActuateDB").join("Default");
        }


        let bank_current_value: RwLock<String> = RwLock::new(String::new());
        let base_dir: PathBuf;
        let binding: Option<PathBuf> = dirs::document_dir();
        if binding.is_some() && instance.dir_files_map.lock().unwrap().is_empty() {
            // Attempt to create dir if it doesn't exist
            base_dir = binding.unwrap().as_path().join("ActuateDB");
            if !base_dir.exists() {
                //default_dir = base_dir.as_path().join("Default");
                let creation_attempt = std::fs::create_dir_all(default_dir.clone());
                if creation_attempt.is_ok() {
                    let stringpath = base_dir.as_path().to_str().unwrap();
                    nih_log!("Created DB at {}", stringpath);
                }
            }
            let root = base_dir;
                
            // Traverse directories and files up to two levels deep
            for entry in WalkDir::new(root)
                .min_depth(1) // Skip the root directory itself
                .max_depth(2) // Limit traversal to two levels deep
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
            
                // If it's a directory (at level 1), initialize its file vector
                if path.is_dir() && entry.depth() == 1 {
                    instance.dir_files_map.lock().unwrap().insert(path.to_path_buf(), Vec::new());
                    instance.str_files_map.lock().unwrap().insert(path.file_name().unwrap().to_str().unwrap().to_string(), Vec::new());
                } 
                // If it's a file inside a level 1 directory, add it to the corresponding directory
                else if path.is_file() {
                    if let Some(parent_dir) = path.parent() {
                        if let Some(files) = instance.dir_files_map.lock().unwrap().get_mut(parent_dir) {
                            files.push(path.to_path_buf());
                        }
                        if let Some(files) = instance.str_files_map.lock().unwrap().get_mut(parent_dir.file_name().unwrap().to_str().unwrap()) {
                            files.push(path.to_path_buf());
                        }
                        // Load info into our DB
                        let unserialized: Option<ActuatePresetV131>;
                        (_, unserialized) = Actuate::import_preset(Some(path.to_path_buf()));
                        if unserialized.is_some() {
                            let current_import = unserialized.unwrap();
                            let mut lite_db_write = lite_db.write().unwrap();
                            lite_db_write.entry(parent_dir.file_name().unwrap().to_str().unwrap().to_string()).or_insert_with(HashMap::new)
                                .insert(
                                    path.file_name().unwrap().to_str().unwrap().to_string().replace(".actuate", ""),
                                    PresetBrowserEntry {
                                        PresetCategory: current_import.preset_category,
                                        tag_acid: current_import.tag_acid,
                                        tag_analog: current_import.tag_analog,
                                        tag_bright: current_import.tag_bright,
                                        tag_chord: current_import.tag_chord,
                                        tag_crisp: current_import.tag_crisp,
                                        tag_deep: current_import.tag_deep,
                                        tag_delicate: current_import.tag_delicate,
                                        tag_hard: current_import.tag_hard,
                                        tag_harsh: current_import.tag_harsh,
                                        tag_lush: current_import.tag_lush,
                                        tag_mellow: current_import.tag_mellow,
                                        tag_resonant: current_import.tag_resonant,
                                        tag_rich: current_import.tag_rich,
                                        tag_sharp: current_import.tag_sharp,
                                        tag_silky: current_import.tag_silky,
                                        tag_smooth: current_import.tag_smooth,
                                        tag_soft: current_import.tag_soft,
                                        tag_stab: current_import.tag_stab,
                                        tag_warm: current_import.tag_warm,
                                        _file: path.to_path_buf(),
                                    });
                        }
                    }
                }
            }

            // Print the directory-file structure
            for (dir, files) in instance.dir_files_map.lock().unwrap().iter() {
                nih_log!("Directory: {:?}", dir);
                for file in files {
                    nih_log!("  File: {:?}", file);
                }
            }
        }

        // Set default
        *bank_current_value.write().unwrap() = "Default".to_string();












        // Show only files with our extensions
        let preset_filter = Box::new({
            let ext = Some(OsStr::new("actuate"));
            move |path: &Path| -> bool { path.extension() == ext }
        });
        let save_preset_filter = Box::new({
            let ext = Some(OsStr::new("actuate"));
            move |path: &Path| -> bool { path.extension() == ext }
        });
        let sample_filter = Box::new({
            let ext = Some(OsStr::new("wav"));
            move |path: &Path| -> bool { path.extension() == ext }
        });

        let dialog_main: Arc<Mutex<FileDialog>> = Arc::new(
            Mutex::new(
                    if default_dir.clone().exists() {
                        FileDialog::open_file(Some(default_dir.clone()))
                            //.current_pos([(WIDTH/4) as f32, 10.0])
                            .show_files_filter(preset_filter)
                            .keep_on_top(true)
                            .show_new_folder(false)
                            .show_rename(false)
                    } else {
                        FileDialog::open_file(Some(home_dir.clone()))
                            //.current_pos([(WIDTH/4) as f32, 10.0])
                            .show_files_filter(preset_filter)
                            .keep_on_top(true)
                            .show_new_folder(false)
                            .show_rename(false)
                    }
                )
            );
        let save_dialog_main: Arc<Mutex<FileDialog>> = Arc::new(
            Mutex::new(
                    if default_dir.clone().exists() {
                        FileDialog::save_file(Some(default_dir.clone()))
                            //.current_pos([(WIDTH/4) as f32, 10.0])
                            .show_files_filter(save_preset_filter)
                            .keep_on_top(true)
                            .show_new_folder(false)
                            .show_rename(false)
                    } else {
                        FileDialog::save_file(Some(home_dir.clone()))
                            //.current_pos([(WIDTH/4) as f32, 10.0])
                            .show_files_filter(save_preset_filter)
                            .keep_on_top(true)
                            .show_new_folder(false)
                            .show_rename(false)
                    }
                )
            );

        let load_sample_dialog: Arc<Mutex<FileDialog>> = Arc::new(
            Mutex::new(
                FileDialog::open_file(Some(home_dir.clone()))
                    .current_pos([(WIDTH/4) as f32, 10.0])
                    .show_files_filter(sample_filter)
                    .keep_on_top(true)
                    .show_new_folder(false)
                    .show_rename(false)
                )
        );

        // Do our GUI stuff. Store this to later get parent window handle from it
        create_egui_editor(
            instance.params.editor_state.clone(),
            (),
            |_, _| {},
            move |egui_ctx, setter, _state| {
                egui::CentralPanel::default()
                    .show(egui_ctx, |ui| {
                        //let current_preset_index = current_preset.load(Ordering::SeqCst);
                        let filter_select = filter_select_outside.clone();
                        let lfo_select = lfo_select_outside.clone();

                        // This lets the internal param track the current samples for when the plugin gets reopened/reloaded
                        // It runs if there is peristent sample data but not sample data in the audio module
                        // This is not very pretty looking but I couldn't allocate separately locked Audio Modules since somewhere
                        // This would cause a deadlock and break Actuate :|
                        // Maybe in future this will become nicer
                        if params.am1_sample.lock().unwrap()[0].len() > 1 && 
                           AM1.lock().unwrap().loaded_sample[0].len() <= 2 &&
                           AM1.lock().unwrap().sample_lib[0][0][0] == 0.0 &&
                           (AM1.lock().unwrap().audio_module_type == AudioModuleType::Sampler ||
                            AM1.lock().unwrap().audio_module_type == AudioModuleType::Granulizer)
                           {
                            let mut AM1_Lock = AM1.lock().unwrap();

                            AM1_Lock.loaded_sample = params.am1_sample.lock().unwrap().to_vec();

                            AM1_Lock.regenerate_samples();
                        }
                        if params.am2_sample.lock().unwrap()[0].len() > 1 && 
                           AM2.lock().unwrap().loaded_sample[0].len() <= 2 &&
                           AM2.lock().unwrap().sample_lib[0][0][0] == 0.0 &&
                           (AM2.lock().unwrap().audio_module_type == AudioModuleType::Sampler ||
                            AM2.lock().unwrap().audio_module_type == AudioModuleType::Granulizer)
                           {
                            let mut AM2_Lock = AM2.lock().unwrap();

                            AM2_Lock.loaded_sample = params.am2_sample.lock().unwrap().to_vec();

                            AM2_Lock.regenerate_samples();
                        }
                        if params.am3_sample.lock().unwrap()[0].len() > 1 && 
                           AM3.lock().unwrap().loaded_sample[0].len() <= 2 &&
                           AM3.lock().unwrap().sample_lib[0][0][0] == 0.0 &&
                           (AM3.lock().unwrap().audio_module_type == AudioModuleType::Sampler ||
                            AM3.lock().unwrap().audio_module_type == AudioModuleType::Granulizer)
                           {
                            let mut AM3_Lock = AM3.lock().unwrap();

                            AM3_Lock.loaded_sample = params.am3_sample.lock().unwrap().to_vec();

                            AM3_Lock.regenerate_samples();
                        }

                        if update_current_preset.load(Ordering::SeqCst) || params.param_update_current_preset.value() {
                            setter.set_parameter(&params.param_update_current_preset, false);
                            update_current_preset.store(false, Ordering::SeqCst);
                        }
                        if params.filter_cutoff_link.value() {
                            setter.set_parameter(&params.filter_cutoff_2, params.filter_cutoff.value());
                        }

                        // Assign default colors
                        ui.style_mut().visuals.widgets.inactive.bg_stroke.color = TEAL_GREEN;
                        ui.style_mut().visuals.widgets.inactive.bg_fill = DARK_GREY_UI_COLOR;
                        ui.style_mut().visuals.widgets.active.fg_stroke.color = TEAL_GREEN;
                        ui.style_mut().visuals.widgets.active.bg_stroke.color = TEAL_GREEN;
                        ui.style_mut().visuals.widgets.open.fg_stroke.color = TEAL_GREEN;
                        ui.style_mut().visuals.widgets.open.bg_fill = DARK_GREY_UI_COLOR;
                        // Lettering on param sliders
                        ui.style_mut().visuals.widgets.inactive.fg_stroke.color = TEAL_GREEN;
                        // Background of the bar in param sliders
                        ui.style_mut().visuals.selection.bg_fill = TEAL_GREEN;
                        ui.style_mut().visuals.selection.stroke.color = TEAL_GREEN;
                        // Unfilled background of the bar
                        ui.style_mut().visuals.widgets.noninteractive.bg_fill = DARK_GREY_UI_COLOR;
                        // egui 0.20 to 0.22 changed this styling then I later decided proportional looks nice
                        //ui.style_mut().drag_value_text_style = egui::TextStyle::Monospace;

                        // Trying to draw background box as rect
                        ui.painter().rect_filled(
                            Rect::from_x_y_ranges(
                                RangeInclusive::new(0.0, WIDTH as f32),
                                RangeInclusive::new(0.0, (HEIGHT as f32)*0.65)),
                            Rounding::ZERO,
                            DARK_GREY_UI_COLOR);

                        // Draw top bar background
                        ui.painter().rect_filled(
                            Rect::from_x_y_ranges(
                                RangeInclusive::new(0.0, WIDTH as f32),
                                RangeInclusive::new(0.0, HEIGHT as f32 * 0.05)),
                            Rounding::ZERO,
                            DARKER_GREY_UI_COLOR
                        );

                        // Background boxes for Generators
                        ui.painter().rect_filled(
                            Rect::from_x_y_ranges(
                                RangeInclusive::new(WIDTH as f32 * 0.005, WIDTH as f32 * 0.20),
                                RangeInclusive::new(HEIGHT as f32 * 0.05, HEIGHT as f32 * 0.25)),
                            Rounding::from(4.0),
                            LIGHTER_GREY_UI_COLOR
                        );
                        ui.painter().rect_filled(
                            Rect::from_x_y_ranges(
                                RangeInclusive::new(WIDTH as f32 * 0.005, WIDTH as f32 * 0.20),
                                RangeInclusive::new(HEIGHT as f32 * 0.26, HEIGHT as f32 * 0.45)),
                            Rounding::from(4.0),
                            LIGHTER_GREY_UI_COLOR
                        );
                        ui.painter().rect_filled(
                            Rect::from_x_y_ranges(
                                RangeInclusive::new(WIDTH as f32 * 0.005, WIDTH as f32 * 0.20),
                                RangeInclusive::new(HEIGHT as f32 * 0.46, HEIGHT as f32 * 0.65)),
                            Rounding::from(4.0),
                            LIGHTER_GREY_UI_COLOR
                        );

                        // Background boxes for Audio Modules
                        ui.painter().rect_filled(
                            Rect::from_x_y_ranges(
                                RangeInclusive::new(WIDTH as f32 * 0.21, WIDTH as f32 * 0.99),
                                RangeInclusive::new(HEIGHT as f32 * 0.05, HEIGHT as f32 * 0.25)),
                            Rounding::from(4.0),
                            LIGHTER_GREY_UI_COLOR
                        );
                        ui.painter().rect_filled(
                            Rect::from_x_y_ranges(
                                RangeInclusive::new(WIDTH as f32 * 0.21, WIDTH as f32 * 0.99),
                                RangeInclusive::new(HEIGHT as f32 * 0.26, HEIGHT as f32 * 0.45)),
                            Rounding::from(4.0),
                            LIGHTER_GREY_UI_COLOR
                        );
                        ui.painter().rect_filled(
                            Rect::from_x_y_ranges(
                                RangeInclusive::new(WIDTH as f32 * 0.21, WIDTH as f32 * 0.99),
                                RangeInclusive::new(HEIGHT as f32 * 0.46, HEIGHT as f32 * 0.65)),
                            Rounding::from(4.0),
                            LIGHTER_GREY_UI_COLOR
                        );

                            // GUI Structure
                            ui.vertical(|ui| {
                                ui.horizontal(|ui|{
                                    ui.add_space(2.0);
                                    ui.label(RichText::new("Actuate")
                                        .font(FONT)
                                        .color(FONT_COLOR))
                                        .on_hover_text("v1.3.8 by Ardura!");
                                    ui.add_space(2.0);
                                    ui.separator();

                                    let master_knob = ui_knob::ArcKnob::for_param(
                                        &params.master_level,
                                        setter,
                                        11.0,
                                        KnobLayout::HorizontalInline)
                                        .preset_style(ui_knob::KnobStyle::Preset1)
                                        .set_fill_color(DARK_GREY_UI_COLOR)
                                        .set_line_color(YELLOW_MUSTARD)
                                        .set_text_size(TEXT_SIZE)
                                        .set_hover_text("Master volume level for Actuate".to_string());
                                    ui.add(master_knob);

                                    ui.separator();
                                    let browse = ui.button(RichText::new("Browse Presets")
                                        .font(FONT)
                                        .background_color(YELLOW_MUSTARD.linear_multiply(1.1))
                                        .color(DARKEST_BOTTOM_UI_COLOR)
                                    );
                                    ui.separator();
                                    ui.selectable_value(&mut *lfo_select.lock().unwrap(), LFOSelect::INFO, RichText::new("Preset Info").background_color(DARKEST_BOTTOM_UI_COLOR).font(SMALLER_FONT));
                                    if browse.clicked() {
                                        browse_preset_active.store(true, Ordering::SeqCst);
                                    }
                                    if browse_preset_active.load(Ordering::SeqCst) {
                                        let window = egui::Window::new("Preset Browser")
                                            .id(egui::Id::new("browse_presets_window"))
                                            .resizable(false)
                                            .constrain(true)
                                            .collapsible(false)
                                            .title_bar(true)
                                            .fixed_pos(Pos2::new(
                                                (WIDTH as f32/ 2.0) - 330.0,
                                                (HEIGHT as f32/ 2.0) - 250.0))
                                            .fixed_size(Vec2::new(660.0, 500.0))
                                            .scroll([true, true])
                                            .fade_in(false)
                                            .fade_out(false)
                                            .enabled(true);
                                        window.show(egui_ctx, |ui| {
                                            ui.visuals_mut().extreme_bg_color = Color32::DARK_GRAY;
                                            //let max_rows = PRESET_BANK_SIZE;

                                            ui.vertical_centered(|ui| {
                                                let close_button = ui.button(RichText::new("Close Browser")
                                                    .font(FONT)
                                                    .background_color(A_BACKGROUND_COLOR_TOP)
                                                    .color(FONT_COLOR)
                                                ).on_hover_text("Close this window without doing anything");
                                                if close_button.clicked() {
                                                    browse_preset_active.store(false, Ordering::SeqCst);
                                                }
                                                ui.horizontal(|ui|{
                                                    ui.label(RichText::new("Tags:")
                                                        .font(FONT)
                                                        .background_color(A_BACKGROUND_COLOR_TOP)
                                                        .color(FONT_COLOR));
                                                    let acid = slim_checkbox::AtomicSlimCheckbox::new(&filter_acid, "Acid");
                                                    ui.add(acid);
                                                    let analog = slim_checkbox::AtomicSlimCheckbox::new(&filter_analog, "Analog");
                                                    ui.add(analog);
                                                    let bright = slim_checkbox::AtomicSlimCheckbox::new(&filter_bright, "Bright");
                                                    ui.add(bright);
                                                    let chord = slim_checkbox::AtomicSlimCheckbox::new(&filter_chord, "Chord");
                                                    ui.add(chord);
                                                    let crisp = slim_checkbox::AtomicSlimCheckbox::new(&filter_crisp, "Crisp");
                                                    ui.add(crisp);
                                                    let deep = slim_checkbox::AtomicSlimCheckbox::new(&filter_deep, "Deep");
                                                    ui.add(deep);
                                                    let delicate = slim_checkbox::AtomicSlimCheckbox::new(&filter_delicate, "Delicate");
                                                    ui.add(delicate);
                                                    let hard = slim_checkbox::AtomicSlimCheckbox::new(&filter_hard, "Hard");
                                                    ui.add(hard);
                                                    let harsh = slim_checkbox::AtomicSlimCheckbox::new(&filter_harsh, "Harsh");
                                                    ui.add(harsh);
                                                    let lush = slim_checkbox::AtomicSlimCheckbox::new(&filter_lush, "Lush");
                                                    ui.add(lush);
                                                });
                                                ui.horizontal(|ui|{
                                                    ui.add_space(34.0);
                                                    let mellow = slim_checkbox::AtomicSlimCheckbox::new(&filter_mellow, "Mellow");
                                                    ui.add(mellow);
                                                    let resonant = slim_checkbox::AtomicSlimCheckbox::new(&filter_resonant, "Resonant");
                                                    ui.add(resonant);
                                                    let rich = slim_checkbox::AtomicSlimCheckbox::new(&filter_rich, "Rich");
                                                    ui.add(rich);
                                                    let sharp = slim_checkbox::AtomicSlimCheckbox::new(&filter_sharp, "Sharp");
                                                    ui.add(sharp);
                                                    let silky = slim_checkbox::AtomicSlimCheckbox::new(&filter_silky, "Silky");
                                                    ui.add(silky);
                                                    let smooth = slim_checkbox::AtomicSlimCheckbox::new(&filter_smooth, "Smooth");
                                                    ui.add(smooth);
                                                    let soft = slim_checkbox::AtomicSlimCheckbox::new(&filter_soft, "Soft");
                                                    ui.add(soft);
                                                    let stab = slim_checkbox::AtomicSlimCheckbox::new(&filter_stab, "Stab");
                                                    ui.add(stab);
                                                    let warm = slim_checkbox::AtomicSlimCheckbox::new(&filter_warm, "Warm");
                                                    ui.add(warm);
                                                });
                                            });

                                            ui.separator();

                                            ui.horizontal(|ui|{
                                                ui.vertical(|ui|{
                                                    ui.colored_label(YELLOW_MUSTARD, "Preset Banks");
                                                    for (directory, _) in dir_files_map.lock().unwrap().iter() {
                                                        let name = directory.file_name().unwrap().to_str().unwrap().to_string();
                                                        ui.selectable_value(&mut *bank_current_value.write().unwrap(), name.clone(), 
                                                            RichText::new(name)
                                                                .font(FONT)
                                                                .background_color(A_BACKGROUND_COLOR_TOP)
                                                                .color(TEAL_GREEN));
                                                    }
                                                });
                                                ui.separator();
                                                ui.vertical(|ui|{
                                                    egui::Grid::new("preset_table")
                                                        .striped(true)
                                                        .num_columns(5)
                                                        .min_col_width(2.0)
                                                        .max_col_width(200.0)
                                                        .show(ui, |ui| {
                                                            ui.label(RichText::new("Load")
                                                                .font(FONT)
                                                                .background_color(A_BACKGROUND_COLOR_TOP)
                                                                .color(FONT_COLOR));
                                                            ui.label(RichText::new("Preset Name")
                                                                .font(FONT)
                                                                .background_color(A_BACKGROUND_COLOR_TOP)
                                                                .color(FONT_COLOR));
                                                            ui.label(RichText::new("Category")
                                                                .font(FONT)
                                                                .background_color(A_BACKGROUND_COLOR_TOP)
                                                                .color(FONT_COLOR));
                                                            ui.label(RichText::new("Tags")
                                                                .font(FONT)
                                                                .background_color(A_BACKGROUND_COLOR_TOP)
                                                                .color(FONT_COLOR));
                                                            ui.end_row();
                                                            // No filters are checked
                                                            if  !filter_acid.load(Ordering::SeqCst) &&
                                                                !filter_analog.load(Ordering::SeqCst) &&
                                                                !filter_bright.load(Ordering::SeqCst) &&
                                                                !filter_chord.load(Ordering::SeqCst) &&
                                                                !filter_crisp.load(Ordering::SeqCst) &&
                                                                !filter_deep.load(Ordering::SeqCst) &&
                                                                !filter_delicate.load(Ordering::SeqCst) &&
                                                                !filter_hard.load(Ordering::SeqCst) &&
                                                                !filter_harsh.load(Ordering::SeqCst) &&
                                                                !filter_lush.load(Ordering::SeqCst) &&
                                                                !filter_mellow.load(Ordering::SeqCst) &&
                                                                !filter_resonant.load(Ordering::SeqCst) &&
                                                                !filter_rich.load(Ordering::SeqCst) &&
                                                                !filter_sharp.load(Ordering::SeqCst) &&
                                                                !filter_silky.load(Ordering::SeqCst) &&
                                                                !filter_smooth.load(Ordering::SeqCst) &&
                                                                !filter_soft.load(Ordering::SeqCst) &&
                                                                !filter_stab.load(Ordering::SeqCst) &&
                                                                !filter_warm.load(Ordering::SeqCst)
                                                                {
                                                                    let tmp_val = bank_current_value.read().unwrap();
                                                                    if let Some(row) = str_files_map.lock().unwrap().get(&*tmp_val) {
                                                                        //ui.vertical(|ui|{
                                                                            for (pno, presetfile) in row.iter().enumerate() {
                                                                                //ui.horizontal(|ui|{
                                                                                    let unserialized: Option<ActuatePresetV131>;
                                                                                    let preset_name = presetfile.file_name().unwrap_or(OsStr::new("ERROR")).to_str().unwrap().replace(".actuate", "");
                                                                                    if ui.button(format!("Load Preset {pno}")).clicked() {

                                                                                        (_, unserialized) = Actuate::import_preset(Some(presetfile.to_path_buf()));
                                                                                        
                                                                                        // Stop our current voices
                                                                                        clear_voices.store(true, Ordering::SeqCst);
                                                                                    
                                                                                        // Move to info tab on preset change
                                                                                        *lfo_select.lock().unwrap() = LFOSelect::INFO;

                                                                                        if unserialized.is_some() {
                                                                                            let mut locked_lib = arc_preset.lock().unwrap();
                                                                                            *locked_lib = unserialized.unwrap();
                                                                                            //let temp_preset = &locked_lib;
                                                                                            *params.preset_name_p.lock().unwrap() =  locked_lib.preset_name.clone();
                                                                                            *params.preset_info_p.lock().unwrap() = locked_lib.preset_info.clone();
                                                                                            setter.set_parameter(&params.preset_category, locked_lib.preset_category);
                                                                                        
                                                                                            import_preset_active.store(false, Ordering::SeqCst);
                                                                                        
                                                                                            drop(locked_lib);
                                                                                        
                                                                                            // GUI thread misses this without this call here for some reason
                                                                                            Actuate::reload_entire_preset(
                                                                                                setter,
                                                                                                params.clone(),
                                                                                                arc_preset.lock().unwrap().clone(),
                                                                                                &mut AM1.lock().unwrap(),
                                                                                                &mut AM2.lock().unwrap(),
                                                                                                &mut AM3.lock().unwrap(),);
                                                                                            // This is set for the process thread
                                                                                            reload_entire_preset.store(true, Ordering::SeqCst);
                                                                                        }
                                                                                    }
                                                                                    // Tags
                                                                                    if !preset_name.contains("ERROR") {
                                                                                        let bank_current = bank_current_value.read().unwrap(); // clone the value
                                                                                        let preset_db_read = lite_db.read().unwrap();
                                                                                        if let Some(inner_map) = preset_db_read.get(&*bank_current) {
                                                                                            if let Some(tag_unwrap) = inner_map.get(&preset_name) {
                                                                                                ui.label(preset_name.trim());
                                                                                                ui.label(format!("{:?}",tag_unwrap.PresetCategory.clone()).trim());
                                                                                                ui.horizontal(|ui|{
                                                                                                    if tag_unwrap.tag_acid {
                                                                                                        ui.label("Acid");
                                                                                                    }
                                                                                                    if tag_unwrap.tag_analog {
                                                                                                        ui.label("Analog");
                                                                                                    }
                                                                                                    if tag_unwrap.tag_bright {
                                                                                                        ui.label("Bright");
                                                                                                    }
                                                                                                    if tag_unwrap.tag_chord {
                                                                                                        ui.label("Chord");
                                                                                                    }
                                                                                                    if tag_unwrap.tag_crisp {
                                                                                                        ui.label("Crisp");
                                                                                                    }
                                                                                                    if tag_unwrap.tag_deep {
                                                                                                        ui.label("Deep");
                                                                                                    }
                                                                                                    if tag_unwrap.tag_delicate {
                                                                                                        ui.label("Delicate");
                                                                                                    }
                                                                                                    if tag_unwrap.tag_hard {
                                                                                                        ui.label("Hard");
                                                                                                    }
                                                                                                    if tag_unwrap.tag_harsh {
                                                                                                        ui.label("Harsh");
                                                                                                    }
                                                                                                    if tag_unwrap.tag_lush {
                                                                                                        ui.label("Lush");
                                                                                                    }
                                                                                                    if tag_unwrap.tag_mellow {
                                                                                                        ui.label("Mellow");
                                                                                                    }
                                                                                                    if tag_unwrap.tag_resonant {
                                                                                                        ui.label("Resonant");
                                                                                                    }
                                                                                                    if tag_unwrap.tag_rich {
                                                                                                        ui.label("Rich");
                                                                                                    }
                                                                                                    if tag_unwrap.tag_sharp {
                                                                                                        ui.label("Sharp");
                                                                                                    }
                                                                                                    if tag_unwrap.tag_silky {
                                                                                                        ui.label("Silky");
                                                                                                    }
                                                                                                    if tag_unwrap.tag_smooth {
                                                                                                        ui.label("Smooth");
                                                                                                    }
                                                                                                    if tag_unwrap.tag_soft {
                                                                                                        ui.label("Soft");
                                                                                                    }
                                                                                                    if tag_unwrap.tag_stab {
                                                                                                        ui.label("Stab");
                                                                                                    }
                                                                                                    if tag_unwrap.tag_warm {
                                                                                                        ui.label("Warm");
                                                                                                    }
                                                                                                });
                                                                                            } else {
                                                                                                ui.label(preset_name.trim());
                                                                                                ui.label("Error Loading");
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                ui.end_row();
                                                                            }
                                                                    }
                                                                } else {
                                                                    // Filter results
                                                                    let tmp_val = bank_current_value.read().unwrap();
                                                                    if let Some(row) = str_files_map.lock().unwrap().get(&*tmp_val) {
                                                                        //ui.vertical(|ui|{
                                                                            for (pno, presetfile) in row.iter().enumerate() {
                                                                                //ui.horizontal(|ui|{
                                                                                    let unserialized: Option<ActuatePresetV131>;
                                                                                    let preset_name = presetfile.file_name().unwrap_or(OsStr::new("ERROR")).to_str().unwrap().replace(".actuate", "");

                                                                                    if !preset_name.contains("ERROR") {
                                                                                        let bank_current = bank_current_value.read().unwrap(); // clone the value
                                                                                        let preset_db_read = lite_db.read().unwrap();
                                                                                        if let Some(inner_map) = preset_db_read.get(&*bank_current) {
                                                                                            if let Some(preset) = inner_map.get(&preset_name) {
                                                                                                if (filter_acid.load(Ordering::SeqCst) && preset.tag_acid == true) ||
                                                                                                    (filter_analog.load(Ordering::SeqCst) && preset.tag_analog == true) ||
                                                                                                    (filter_bright.load(Ordering::SeqCst) && preset.tag_bright == true) ||
                                                                                                    (filter_chord.load(Ordering::SeqCst) && preset.tag_chord == true) ||
                                                                                                    (filter_crisp.load(Ordering::SeqCst) && preset.tag_crisp == true) ||
                                                                                                    (filter_deep.load(Ordering::SeqCst) && preset.tag_deep == true) ||
                                                                                                    (filter_delicate.load(Ordering::SeqCst) && preset.tag_delicate == true) ||
                                                                                                    (filter_hard.load(Ordering::SeqCst) && preset.tag_hard == true) ||
                                                                                                    (filter_harsh.load(Ordering::SeqCst) && preset.tag_harsh == true) ||
                                                                                                    (filter_lush.load(Ordering::SeqCst) && preset.tag_lush == true) ||
                                                                                                    (filter_mellow.load(Ordering::SeqCst) && preset.tag_mellow == true) ||
                                                                                                    (filter_resonant.load(Ordering::SeqCst) && preset.tag_resonant == true) ||
                                                                                                    (filter_rich.load(Ordering::SeqCst) && preset.tag_rich == true) ||
                                                                                                    (filter_sharp.load(Ordering::SeqCst) && preset.tag_sharp == true) ||
                                                                                                    (filter_silky.load(Ordering::SeqCst) && preset.tag_silky == true) ||
                                                                                                    (filter_smooth.load(Ordering::SeqCst) && preset.tag_smooth == true) ||
                                                                                                    (filter_soft.load(Ordering::SeqCst) && preset.tag_soft == true) ||
                                                                                                    (filter_stab.load(Ordering::SeqCst) && preset.tag_stab == true) ||
                                                                                                    (filter_warm.load(Ordering::SeqCst) && preset.tag_warm == true) {
                                                                                                    
                                                                                                        if ui.button(format!("Load Preset {pno}")).clicked() {

                                                                                                            (_, unserialized) = Actuate::import_preset(Some(presetfile.to_path_buf()));
                                                                                                            
                                                                                                            // Stop our current voices
                                                                                                            clear_voices.store(true, Ordering::SeqCst);
                                                                                                        
                                                                                                            // Move to info tab on preset change
                                                                                                            *lfo_select.lock().unwrap() = LFOSelect::INFO;
                    
                                                                                                            if unserialized.is_some() {
                                                                                                                let mut locked_lib = arc_preset.lock().unwrap();
                                                                                                                *locked_lib = unserialized.unwrap();
                                                                                                                //let temp_preset = &locked_lib;
                                                                                                                *params.preset_name_p.lock().unwrap() = locked_lib.preset_name.clone();
                                                                                                                *params.preset_info_p.lock().unwrap() = locked_lib.preset_info.clone();
                                                                                                                setter.set_parameter(&params.preset_category, locked_lib.preset_category);
                                                                                                            
                                                                                                                import_preset_active.store(false, Ordering::SeqCst);
                                                                                                            
                                                                                                                drop(locked_lib);
                                                                                                            
                                                                                                                // GUI thread misses this without this call here for some reason
                                                                                                                Actuate::reload_entire_preset(
                                                                                                                    setter,
                                                                                                                    params.clone(),
                                                                                                                    arc_preset.lock().unwrap().clone(),
                                                                                                                    &mut AM1.lock().unwrap(),
                                                                                                                    &mut AM2.lock().unwrap(),
                                                                                                                    &mut AM3.lock().unwrap(),);
                                                                                                                // This is set for the process thread
                                                                                                                reload_entire_preset.store(true, Ordering::SeqCst);
                                                                                                            }
                                                                                                        }
                                                                                                        // Tags
                                                                                                        if !preset_name.contains("ERROR") {
                                                                                                            let bank_current = bank_current_value.read().unwrap(); // clone the value
                                                                                                            let preset_db_read = lite_db.read().unwrap();
                                                                                                            if let Some(inner_map) = preset_db_read.get(&*bank_current) {
                                                                                                                if let Some(tag_unwrap) = inner_map.get(&preset_name) {
                                                                                                                    ui.label(preset_name.trim());
                                                                                                                    ui.label(format!("{:?}",tag_unwrap.PresetCategory.clone()).trim());
                                                                                                                    ui.horizontal(|ui|{
                                                                                                                        if tag_unwrap.tag_acid {
                                                                                                                            ui.label("Acid");
                                                                                                                        }
                                                                                                                        if tag_unwrap.tag_analog {
                                                                                                                            ui.label("Analog");
                                                                                                                        }
                                                                                                                        if tag_unwrap.tag_bright {
                                                                                                                            ui.label("Bright");
                                                                                                                        }
                                                                                                                        if tag_unwrap.tag_chord {
                                                                                                                            ui.label("Chord");
                                                                                                                        }
                                                                                                                        if tag_unwrap.tag_crisp {
                                                                                                                            ui.label("Crisp");
                                                                                                                        }
                                                                                                                        if tag_unwrap.tag_deep {
                                                                                                                            ui.label("Deep");
                                                                                                                        }
                                                                                                                        if tag_unwrap.tag_delicate {
                                                                                                                            ui.label("Delicate");
                                                                                                                        }
                                                                                                                        if tag_unwrap.tag_hard {
                                                                                                                            ui.label("Hard");
                                                                                                                        }
                                                                                                                        if tag_unwrap.tag_harsh {
                                                                                                                            ui.label("Harsh");
                                                                                                                        }
                                                                                                                        if tag_unwrap.tag_lush {
                                                                                                                            ui.label("Lush");
                                                                                                                        }
                                                                                                                        if tag_unwrap.tag_mellow {
                                                                                                                            ui.label("Mellow");
                                                                                                                        }
                                                                                                                        if tag_unwrap.tag_resonant {
                                                                                                                            ui.label("Resonant");
                                                                                                                        }
                                                                                                                        if tag_unwrap.tag_rich {
                                                                                                                            ui.label("Rich");
                                                                                                                        }
                                                                                                                        if tag_unwrap.tag_sharp {
                                                                                                                            ui.label("Sharp");
                                                                                                                        }
                                                                                                                        if tag_unwrap.tag_silky {
                                                                                                                            ui.label("Silky");
                                                                                                                        }
                                                                                                                        if tag_unwrap.tag_smooth {
                                                                                                                            ui.label("Smooth");
                                                                                                                        }
                                                                                                                        if tag_unwrap.tag_soft {
                                                                                                                            ui.label("Soft");
                                                                                                                        }
                                                                                                                        if tag_unwrap.tag_stab {
                                                                                                                            ui.label("Stab");
                                                                                                                        }
                                                                                                                        if tag_unwrap.tag_warm {
                                                                                                                            ui.label("Warm");
                                                                                                                        }
                                                                                                                    });
                                                                                                                } else {
                                                                                                                    ui.label(preset_name.trim());
                                                                                                                    ui.label("Error Loading");
                                                                                                                }
                                                                                                            }
                                                                                                        }
                                                                                                    ui.end_row();
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                    //ui.end_row();
                                                                            }
                                                                        }
                                                                    }
                                                                });
                                                    
                                                    ui.vertical_centered(|ui| {
                                                        let close_button = ui.button(RichText::new("Close Browser")
                                                            .font(FONT)
                                                            .background_color(A_BACKGROUND_COLOR_TOP)
                                                            .color(FONT_COLOR)
                                                        ).on_hover_text("Close this window without doing anything");
                                                        if close_button.clicked() {
                                                            browse_preset_active.store(false, Ordering::SeqCst);
                                                        }
                                                    });
                                                });
                                            });
                                        });
                                    }

                                    ui.separator();

                                    let use_fx_toggle = BoolButton::BoolButton::for_param(&params.use_fx, setter, 2.5, 1.0, SMALLER_FONT);
                                    ui.add(use_fx_toggle).on_hover_text("Enable or disable FX processing");

                                    // Studio One changes (compatible for all DAWs)
                                    let import_preset_button = ui.button(RichText::new("Import Preset")
                                        .font(SMALLER_FONT)
                                        .background_color(DARK_GREY_UI_COLOR)
                                        .color(TEAL_GREEN)
                                    );
                                    if import_preset_button.clicked() {
                                        import_preset_active.store(true, Ordering::SeqCst);
                                    }
                                    if import_preset_active.load(Ordering::SeqCst) {
                                        let dialock = dialog_main.clone();
                                        let mut dialog = dialock.lock().unwrap();
                                        dialog.open();
                                        let mut dvar = Some(dialog);

                                        if let Some(dialog) = &mut dvar {
                                            if dialog.show(egui_ctx).selected() {
                                              if let Some(file) = dialog.path() {
                                                let opened_file = Some(file.to_path_buf());
                                                let unserialized: Option<ActuatePresetV131>;
                                                (_, unserialized) = Actuate::import_preset(opened_file);

                                                if unserialized.is_some() {
                                                    let mut locked_lib = arc_preset.lock().unwrap();
                                                    *locked_lib = unserialized.unwrap();
                                                    let temp_preset = &locked_lib;
                                                    *params.preset_name_p.lock().unwrap() =  temp_preset.preset_name.clone();
                                                    *params.preset_info_p.lock().unwrap() = temp_preset.preset_info.clone();
                                                    setter.set_parameter(&params.preset_category, temp_preset.preset_category);

                                                    import_preset_active.store(false, Ordering::SeqCst);

                                                    drop(locked_lib);
                                                
                                                    // GUI thread misses this without this call here for some reason
                                                    Actuate::reload_entire_preset(
                                                        setter,
                                                        params.clone(),
                                                        arc_preset.lock().unwrap().clone(),
                                                        &mut AM1.lock().unwrap(),
                                                        &mut AM2.lock().unwrap(),
                                                        &mut AM3.lock().unwrap(),);
                                                    // This is set for the process thread
                                                    reload_entire_preset.store(true, Ordering::SeqCst);
                                                }
                                              }
                                            }
                                            match dialog.state() {
                                                State::Cancelled | State::Closed => {
                                                    import_preset_active.store(false, Ordering::SeqCst);
                                                },
                                                _ => {}
                                            }
                                        }

                                    }
                                    // Studio One changes (compatible for all DAWs)
                                    let export_preset_button = ui.button(RichText::new("Export Preset")
                                        .font(SMALLER_FONT)
                                        .background_color(DARK_GREY_UI_COLOR)
                                        .color(TEAL_GREEN)
                                    );
                                    if export_preset_button.clicked() {
                                        export_preset_active.store(true, Ordering::SeqCst);
                                    }
                                    if export_preset_active.load(Ordering::SeqCst) {
                                        let save_dialock = save_dialog_main.clone();
                                        let mut save_dialog = save_dialock.lock().unwrap();
                                        save_dialog.open();
                                        let mut dvar = Some(save_dialog);
                                        if let Some(s_dialog) = &mut dvar {
                                            if s_dialog.show(egui_ctx).selected() {
                                              if let Some(file) = s_dialog.path() {
                                                let saved_file = Some(file.to_path_buf());
                                                let locked_lib = arc_preset.lock().unwrap();
                                                Actuate::export_preset(saved_file, locked_lib.clone());
                                                drop(locked_lib);
                                                export_preset_active.store(false, Ordering::SeqCst);
                                              }
                                            }

                                            match s_dialog.state() {
                                                State::Cancelled | State::Closed => {
                                                    export_preset_active.store(false, Ordering::SeqCst);
                                                },
                                                _ => {}
                                            }
                                        }
                                    }
                                    ui.checkbox(&mut safety_clip_output.lock().unwrap(), "Safety Clip").on_hover_text("Clip the output at 0dB to save your ears/speakers");
                                });
                                const KNOB_SIZE: f32 = 28.0;
                                const TEXT_SIZE: f32 = 11.0;
                                ui.horizontal(|ui|{
                                    ui.vertical(|ui|{
                                        ui.label(RichText::new("Generators")
                                            .font(FONT))
                                            .on_hover_text("These are the audio modules that create sound on midi events");
                                        ui.horizontal(|ui|{
                                            ui.add_space(8.0);
                                            ui.vertical(|ui|{
                                                ui.add_space(12.0);
                                                ui.colored_label(TEAL_GREEN, "Type");
                                                let cb1 = ComboBoxParam::ParamComboBox::for_param(&params.audio_module_1_type, setter, vec![
                                                    String::from("Off"),
                                                    String::from("Sine"),
                                                    String::from("Tri"),
                                                    String::from("Saw"),
                                                    String::from("RSaw"),
                                                    String::from("WSaw"),
                                                    String::from("SSaw"),
                                                    String::from("RASaw"),
                                                    String::from("Ramp"),
                                                    String::from("Square"),
                                                    String::from("RSquare"),
                                                    String::from("Pulse"),
                                                    String::from("Noise"),
                                                    String::from("Sampler"),
                                                    String::from("Granulizer"),
                                                    String::from("Additive"),
                                                ],
                                                "cb1".to_string());
                                                ui.add(cb1);
                                                
                                                ui.colored_label(TEAL_GREEN, "Filter Assign");
                                                let fr1 = ComboBoxParam::ParamComboBox::for_param(&params.audio_module_1_routing, setter, vec![
                                                    String::from("Bypass"),
                                                    String::from("Filter1"),
                                                    String::from("Filter2"),
                                                    String::from("Both"),
                                                ],
                                                "fr1".to_string());
                                                ui.add(fr1);
                                            });

                                            let audio_module_1_level_knob = ui_knob::ArcKnob::for_param(
                                                &params.audio_module_1_level,
                                                setter,
                                                KNOB_SIZE,
                                                KnobLayout::Vertical)
                                                .preset_style(ui_knob::KnobStyle::Preset1)
                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                .set_line_color(TEAL_GREEN)
                                                .set_text_size(TEXT_SIZE).set_hover_text("The output gain of the generator".to_string())
                                                .use_outline(true);
                                            ui.add(audio_module_1_level_knob);
                                        });
                                        ui.add_space(48.0);

                                        ui.horizontal(|ui|{
                                            ui.add_space(8.0);
                                            ui.vertical(|ui|{
                                                ui.add_space(12.0);
                                                ui.colored_label(TEAL_GREEN, "Type");
                                                let cb2 = ComboBoxParam::ParamComboBox::for_param(&params.audio_module_2_type, setter, vec![
                                                    String::from("Off"),
                                                    String::from("Sine"),
                                                    String::from("Tri"),
                                                    String::from("Saw"),
                                                    String::from("RSaw"),
                                                    String::from("WSaw"),
                                                    String::from("SSaw"),
                                                    String::from("RASaw"),
                                                    String::from("Ramp"),
                                                    String::from("Square"),
                                                    String::from("RSquare"),
                                                    String::from("Pulse"),
                                                    String::from("Noise"),
                                                    String::from("Sampler"),
                                                    String::from("Granulizer"),
                                                    String::from("Additive"),
                                                ],
                                                "cb2".to_string());
                                                ui.add(cb2);
                                                
                                                ui.colored_label(TEAL_GREEN, "Filter Assign");
                                                let fr2 = ComboBoxParam::ParamComboBox::for_param(&params.audio_module_2_routing, setter, vec![
                                                    String::from("Bypass"),
                                                    String::from("Filter1"),
                                                    String::from("Filter2"),
                                                    String::from("Both"),
                                                ],
                                                "fr2".to_string());
                                                ui.add(fr2);
                                            });

                                            let audio_module_2_level_knob = ui_knob::ArcKnob::for_param(
                                                &params.audio_module_2_level,
                                                setter,
                                                KNOB_SIZE,
                                                KnobLayout::Vertical)
                                                .preset_style(ui_knob::KnobStyle::Preset1)
                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                .set_line_color(TEAL_GREEN)
                                                .set_text_size(TEXT_SIZE).set_hover_text("The output gain of the generator".to_string());
                                            ui.add(audio_module_2_level_knob);
                                        });
                                        ui.add_space(46.0);

                                        ui.horizontal(|ui| {
                                            ui.add_space(8.0);
                                            ui.vertical(|ui|{
                                                ui.add_space(12.0);
                                                ui.colored_label(TEAL_GREEN, "Type");
                                                let cb3 = ComboBoxParam::ParamComboBox::for_param(&params.audio_module_3_type, setter, vec![
                                                    String::from("Off"),
                                                    String::from("Sine"),
                                                    String::from("Tri"),
                                                    String::from("Saw"),
                                                    String::from("RSaw"),
                                                    String::from("WSaw"),
                                                    String::from("SSaw"),
                                                    String::from("RASaw"),
                                                    String::from("Ramp"),
                                                    String::from("Square"),
                                                    String::from("RSquare"),
                                                    String::from("Pulse"),
                                                    String::from("Noise"),
                                                    String::from("Sampler"),
                                                    String::from("Granulizer"),
                                                    String::from("Additive"),
                                                ],
                                                "cb3".to_string());
                                                ui.add(cb3);
                                                
                                                ui.colored_label(TEAL_GREEN, "Filter Assign");
                                                let fr3 = ComboBoxParam::ParamComboBox::for_param(&params.audio_module_3_routing, setter, vec![
                                                    String::from("Bypass"),
                                                    String::from("Filter1"),
                                                    String::from("Filter2"),
                                                    String::from("Both"),
                                                ],
                                                "fr3".to_string());
                                                ui.add(fr3);
                                            });
                                            let audio_module_3_level_knob = ui_knob::ArcKnob::for_param(
                                                &params.audio_module_3_level,
                                                setter,
                                                KNOB_SIZE,
                                                KnobLayout::Vertical)
                                                .preset_style(ui_knob::KnobStyle::Preset1)
                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                .set_line_color(TEAL_GREEN)
                                                .set_text_size(TEXT_SIZE).set_hover_text("The output gain of the generator".to_string());
                                            ui.add(audio_module_3_level_knob);
                                        });
                                        ui.add_space(32.0);
                                    });

                                    ui.add_space(20.0);
                                    ui.vertical(|ui|{
                                        let mut sample_dialog_lock = load_sample_dialog.lock().unwrap();
                                        ui.add_space(12.0);
                                        AudioModule::draw_module(ui, egui_ctx, setter, params.clone(), &mut sample_dialog_lock, 1, &AM1, &AM2, &AM3);
                                        ui.add_space(10.0);
                                        AudioModule::draw_module(ui, egui_ctx, setter, params.clone(), &mut sample_dialog_lock, 2, &AM1, &AM2, &AM3);
                                        ui.add_space(10.0);
                                        AudioModule::draw_module(ui, egui_ctx, setter, params.clone(), &mut sample_dialog_lock, 3, &AM1, &AM2, &AM3);
                                        ui.add_space(4.0);
                                    });
                                });
                                ui.horizontal(|ui|{
                                    ui.selectable_value(&mut *filter_select.lock().unwrap(), UIBottomSelection::Filter1, RichText::new("Filter 1").background_color(DARKEST_BOTTOM_UI_COLOR));
                                    ui.selectable_value(&mut *filter_select.lock().unwrap(), UIBottomSelection::Filter2, RichText::new("Filter 2").background_color(DARKEST_BOTTOM_UI_COLOR));
                                    ui.selectable_value(&mut *filter_select.lock().unwrap(), UIBottomSelection::Pitch1, RichText::new("Pitch 1").background_color(DARKEST_BOTTOM_UI_COLOR));
                                    ui.selectable_value(&mut *filter_select.lock().unwrap(), UIBottomSelection::Pitch2, RichText::new("Pitch 2").background_color(DARKEST_BOTTOM_UI_COLOR));
                                    // Jank spacing stuff :)
                                    ui.add_space(304.0);
                                    ui.selectable_value(&mut *lfo_select.lock().unwrap(), LFOSelect::Modulation, RichText::new("Modulation").background_color(DARKEST_BOTTOM_UI_COLOR).font(SMALLER_FONT));
                                    ui.selectable_value(&mut *lfo_select.lock().unwrap(), LFOSelect::LFO1, RichText::new("LFO 1").background_color(DARKEST_BOTTOM_UI_COLOR).font(SMALLER_FONT));
                                    ui.selectable_value(&mut *lfo_select.lock().unwrap(), LFOSelect::LFO2, RichText::new("LFO 2").background_color(DARKEST_BOTTOM_UI_COLOR).font(SMALLER_FONT));
                                    ui.selectable_value(&mut *lfo_select.lock().unwrap(), LFOSelect::LFO3, RichText::new("LFO 3").background_color(DARKEST_BOTTOM_UI_COLOR).font(SMALLER_FONT));
                                    ui.selectable_value(&mut *lfo_select.lock().unwrap(), LFOSelect::FM, RichText::new("FM").background_color(DARKEST_BOTTOM_UI_COLOR).font(SMALLER_FONT));
                                    ui.selectable_value(&mut *lfo_select.lock().unwrap(), LFOSelect::FX, RichText::new("FX").background_color(DARKEST_BOTTOM_UI_COLOR).font(SMALLER_FONT));
                                    ui.selectable_value(&mut *lfo_select.lock().unwrap(), LFOSelect::Misc, RichText::new("Misc").background_color(DARKEST_BOTTOM_UI_COLOR).font(SMALLER_FONT));
                                });

                                ////////////////////////////////////////////////////////////
                                // ADSR FOR FILTER
                                const VERT_BAR_HEIGHT: f32 = 110.0;
                                const VERT_BAR_WIDTH: f32 = 14.0;
                                ui.horizontal(|ui|{
                                    ui.horizontal(|ui|{
                                        // Filter ADSR+Curves + Routing
                                        ui.vertical(|ui|{
                                            ui.horizontal(|ui|{
                                                match *filter_select.lock().unwrap() {
                                                    UIBottomSelection::Filter1 => {
                                                        // ADSR
                                                        ui.add(
                                                            VerticalParamSlider::for_param(&params.filter_env_attack, setter)
                                                                .with_width(VERT_BAR_WIDTH)
                                                                .with_height(VERT_BAR_HEIGHT)
                                                                .set_reversed(true)
                                                                .override_colors(
                                                                    LIGHTER_GREY_UI_COLOR,
                                                                    YELLOW_MUSTARD,
                                                                ),
                                                        );
                                                        ui.add(
                                                            VerticalParamSlider::for_param(&params.filter_env_decay, setter)
                                                                .with_width(VERT_BAR_WIDTH)
                                                                .with_height(VERT_BAR_HEIGHT)
                                                                .set_reversed(true)
                                                                .override_colors(
                                                                    LIGHTER_GREY_UI_COLOR,
                                                                    YELLOW_MUSTARD,
                                                                ),
                                                        );
                                                        ui.add(
                                                            VerticalParamSlider::for_param(&params.filter_env_sustain, setter)
                                                                .with_width(VERT_BAR_WIDTH)
                                                                .with_height(VERT_BAR_HEIGHT)
                                                                .set_reversed(true)
                                                                .override_colors(
                                                                    LIGHTER_GREY_UI_COLOR,
                                                                    YELLOW_MUSTARD,
                                                                ),
                                                        );
                                                        ui.add(
                                                            VerticalParamSlider::for_param(&params.filter_env_release, setter)
                                                                .with_width(VERT_BAR_WIDTH)
                                                                .with_height(VERT_BAR_HEIGHT)
                                                                .set_reversed(true)
                                                                .override_colors(
                                                                    LIGHTER_GREY_UI_COLOR,
                                                                    YELLOW_MUSTARD,
                                                                ),
                                                        );
                                                    },
                                                    UIBottomSelection::Filter2 => {
                                                        // ADSR
                                                        ui.add(
                                                            VerticalParamSlider::for_param(&params.filter_env_attack_2, setter)
                                                                .with_width(VERT_BAR_WIDTH)
                                                                .with_height(VERT_BAR_HEIGHT)
                                                                .set_reversed(true)
                                                                .override_colors(
                                                                    LIGHTER_GREY_UI_COLOR,
                                                                    TEAL_GREEN,
                                                                ),
                                                        );
                                                        ui.add(
                                                            VerticalParamSlider::for_param(&params.filter_env_decay_2, setter)
                                                                .with_width(VERT_BAR_WIDTH)
                                                                .with_height(VERT_BAR_HEIGHT)
                                                                .set_reversed(true)
                                                                .override_colors(
                                                                    LIGHTER_GREY_UI_COLOR,
                                                                    TEAL_GREEN,
                                                                ),
                                                        );
                                                        ui.add(
                                                            VerticalParamSlider::for_param(&params.filter_env_sustain_2, setter)
                                                                .with_width(VERT_BAR_WIDTH)
                                                                .with_height(VERT_BAR_HEIGHT)
                                                                .set_reversed(true)
                                                                .override_colors(
                                                                    LIGHTER_GREY_UI_COLOR,
                                                                    TEAL_GREEN,
                                                                ),
                                                        );
                                                        ui.add(
                                                            VerticalParamSlider::for_param(&params.filter_env_release_2, setter)
                                                                .with_width(VERT_BAR_WIDTH)
                                                                .with_height(VERT_BAR_HEIGHT)
                                                                .set_reversed(true)
                                                                .override_colors(
                                                                    LIGHTER_GREY_UI_COLOR,
                                                                    TEAL_GREEN,
                                                                ),
                                                        );
                                                    },
                                                    UIBottomSelection::Pitch1 => {
                                                        // ADSR
                                                        ui.add(
                                                            VerticalParamSlider::for_param(&params.pitch_env_attack, setter)
                                                                .with_width(VERT_BAR_WIDTH)
                                                                .with_height(VERT_BAR_HEIGHT)
                                                                .set_reversed(true)
                                                                .override_colors(
                                                                    LIGHTER_GREY_UI_COLOR,
                                                                    YELLOW_MUSTARD,
                                                                ),
                                                        );
                                                        ui.add(
                                                            VerticalParamSlider::for_param(&params.pitch_env_decay, setter)
                                                                .with_width(VERT_BAR_WIDTH)
                                                                .with_height(VERT_BAR_HEIGHT)
                                                                .set_reversed(true)
                                                                .override_colors(
                                                                    LIGHTER_GREY_UI_COLOR,
                                                                    YELLOW_MUSTARD,
                                                                ),
                                                        );
                                                        ui.add(
                                                            VerticalParamSlider::for_param(&params.pitch_env_sustain, setter)
                                                                .with_width(VERT_BAR_WIDTH)
                                                                .with_height(VERT_BAR_HEIGHT)
                                                                .set_reversed(true)
                                                                .override_colors(
                                                                    LIGHTER_GREY_UI_COLOR,
                                                                    YELLOW_MUSTARD,
                                                                ),
                                                        );
                                                        ui.add(
                                                            VerticalParamSlider::for_param(&params.pitch_env_release, setter)
                                                                .with_width(VERT_BAR_WIDTH)
                                                                .with_height(VERT_BAR_HEIGHT)
                                                                .set_reversed(true)
                                                                .override_colors(
                                                                    LIGHTER_GREY_UI_COLOR,
                                                                    YELLOW_MUSTARD,
                                                                ),
                                                        );
                                                    },
                                                    UIBottomSelection::Pitch2 => {
                                                        // ADSR
                                                        ui.add(
                                                            VerticalParamSlider::for_param(&params.pitch_env_attack_2, setter)
                                                                .with_width(VERT_BAR_WIDTH)
                                                                .with_height(VERT_BAR_HEIGHT)
                                                                .set_reversed(true)
                                                                .override_colors(
                                                                    LIGHTER_GREY_UI_COLOR,
                                                                    TEAL_GREEN,
                                                                ),
                                                        );
                                                        ui.add(
                                                            VerticalParamSlider::for_param(&params.pitch_env_decay_2, setter)
                                                                .with_width(VERT_BAR_WIDTH)
                                                                .with_height(VERT_BAR_HEIGHT)
                                                                .set_reversed(true)
                                                                .override_colors(
                                                                    LIGHTER_GREY_UI_COLOR,
                                                                    TEAL_GREEN,
                                                                ),
                                                        );
                                                        ui.add(
                                                            VerticalParamSlider::for_param(&params.pitch_env_sustain_2, setter)
                                                                .with_width(VERT_BAR_WIDTH)
                                                                .with_height(VERT_BAR_HEIGHT)
                                                                .set_reversed(true)
                                                                .override_colors(
                                                                    LIGHTER_GREY_UI_COLOR,
                                                                    TEAL_GREEN,
                                                                ),
                                                        );
                                                        ui.add(
                                                            VerticalParamSlider::for_param(&params.pitch_env_release_2, setter)
                                                                .with_width(VERT_BAR_WIDTH)
                                                                .with_height(VERT_BAR_HEIGHT)
                                                                .set_reversed(true)
                                                                .override_colors(
                                                                    LIGHTER_GREY_UI_COLOR,
                                                                    TEAL_GREEN,
                                                                ),
                                                        );
                                                    }
                                                }
                                                // Curve sliders
                                                ui.vertical(|ui| {
                                                    match *filter_select.lock().unwrap() {
                                                        UIBottomSelection::Filter1 => {
                                                            ui.add(
                                                                BeizerButton::BeizerButton::for_param(
                                                                    &params.filter_env_atk_curve,
                                                                    setter,
                                                                    5.1,
                                                                    2.0,
                                                                    ButtonLayout::HorizontalInline,
                                                                    true,
                                                                )
                                                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                                                .with_line_color(YELLOW_MUSTARD),
                                                            ).on_hover_text_at_pointer("The behavior of Attack movement in the envelope".to_string());
                                                            ui.add(
                                                                BeizerButton::BeizerButton::for_param(
                                                                    &params.filter_env_dec_curve,
                                                                    setter,
                                                                    5.1,
                                                                    2.0,
                                                                    ButtonLayout::HorizontalInline,
                                                                    false
                                                                )
                                                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                                                .with_line_color(YELLOW_MUSTARD),
                                                            ).on_hover_text_at_pointer("The behavior of Decay movement in the envelope".to_string());
                                                            ui.add(
                                                                BeizerButton::BeizerButton::for_param(
                                                                    &params.filter_env_rel_curve,
                                                                    setter,
                                                                    5.1,
                                                                    2.0,
                                                                    ButtonLayout::HorizontalInline,
                                                                    false
                                                                )
                                                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                                                .with_line_color(YELLOW_MUSTARD),
                                                            ).on_hover_text_at_pointer("The behavior of Release movement in the envelope".to_string());
                                                        },
                                                        UIBottomSelection::Filter2 => {
                                                            ui.add(
                                                                BeizerButton::BeizerButton::for_param(
                                                                    &params.filter_env_atk_curve_2,
                                                                    setter,
                                                                    5.1,
                                                                    2.0,
                                                                    ButtonLayout::HorizontalInline,
                                                                    true,
                                                                )
                                                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                                                .with_line_color(YELLOW_MUSTARD),
                                                            ).on_hover_text_at_pointer("The behavior of Attack movement in the envelope".to_string());
                                                            ui.add(
                                                                BeizerButton::BeizerButton::for_param(
                                                                    &params.filter_env_dec_curve_2,
                                                                    setter,
                                                                    5.1,
                                                                    2.0,
                                                                    ButtonLayout::HorizontalInline,
                                                                    false,
                                                                )
                                                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                                                .with_line_color(YELLOW_MUSTARD),
                                                            ).on_hover_text_at_pointer("The behavior of Decay movement in the envelope".to_string());
                                                            ui.add(
                                                                BeizerButton::BeizerButton::for_param(
                                                                    &params.filter_env_rel_curve_2,
                                                                    setter,
                                                                    5.1,
                                                                    2.0,
                                                                    ButtonLayout::HorizontalInline,
                                                                    false,
                                                                )
                                                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                                                .with_line_color(YELLOW_MUSTARD),
                                                            ).on_hover_text_at_pointer("The behavior of Release movement in the envelope".to_string());
                                                        },
                                                        UIBottomSelection::Pitch1 => {
                                                            ui.add(
                                                                BeizerButton::BeizerButton::for_param(
                                                                    &params.pitch_env_atk_curve,
                                                                    setter,
                                                                    5.1,
                                                                    2.0,
                                                                    ButtonLayout::HorizontalInline,
                                                                    true,
                                                                )
                                                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                                                .with_line_color(YELLOW_MUSTARD),
                                                            ).on_hover_text_at_pointer("The behavior of Attack movement in the envelope".to_string());
                                                            ui.add(
                                                                BeizerButton::BeizerButton::for_param(
                                                                    &params.pitch_env_dec_curve,
                                                                    setter,
                                                                    5.1,
                                                                    2.0,
                                                                    ButtonLayout::HorizontalInline,
                                                                    false,
                                                                )
                                                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                                                .with_line_color(YELLOW_MUSTARD),
                                                            ).on_hover_text_at_pointer("The behavior of Decay movement in the envelope".to_string());
                                                            ui.add(
                                                                BeizerButton::BeizerButton::for_param(
                                                                    &params.pitch_env_rel_curve,
                                                                    setter,
                                                                    5.1,
                                                                    2.0,
                                                                    ButtonLayout::HorizontalInline,
                                                                    false,
                                                                )
                                                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                                                .with_line_color(YELLOW_MUSTARD),
                                                            ).on_hover_text_at_pointer("The behavior of Release movement in the envelope".to_string());
                                                        },
                                                        UIBottomSelection::Pitch2 => {
                                                            ui.add(
                                                                BeizerButton::BeizerButton::for_param(
                                                                    &params.pitch_env_atk_curve_2,
                                                                    setter,
                                                                    5.1,
                                                                    2.0,
                                                                    ButtonLayout::HorizontalInline,
                                                                    true,
                                                                )
                                                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                                                .with_line_color(YELLOW_MUSTARD),
                                                            ).on_hover_text_at_pointer("The behavior of Attack movement in the envelope".to_string());
                                                            ui.add(
                                                                BeizerButton::BeizerButton::for_param(
                                                                    &params.pitch_env_dec_curve_2,
                                                                    setter,
                                                                    5.1,
                                                                    2.0,
                                                                    ButtonLayout::HorizontalInline,
                                                                    false,
                                                                )
                                                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                                                .with_line_color(YELLOW_MUSTARD),
                                                            ).on_hover_text_at_pointer("The behavior of Decay movement in the envelope".to_string());
                                                            ui.add(
                                                                BeizerButton::BeizerButton::for_param(
                                                                    &params.pitch_env_rel_curve_2,
                                                                    setter,
                                                                    5.1,
                                                                    2.0,
                                                                    ButtonLayout::HorizontalInline,
                                                                    false,
                                                                )
                                                                .with_background_color(MEDIUM_GREY_UI_COLOR)
                                                                .with_line_color(YELLOW_MUSTARD),
                                                            ).on_hover_text_at_pointer("The behavior of Release movement in the envelope".to_string());
                                                        }
                                                    }
                                                });
                                            });
                                            // Filter routing goes here
                                            let filter_routing_hknob = ui_knob::ArcKnob::for_param(
                                                &params.filter_routing,
                                                setter,
                                                26.0,
                                                KnobLayout::Horizonal)
                                                .preset_style(ui_knob::KnobStyle::Preset1)
                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                .set_line_color(YELLOW_MUSTARD)
                                                .set_text_size(TEXT_SIZE)
                                                .set_hover_text("This controls filter ordering or isolation".to_string());
                                            ui.add(filter_routing_hknob);
                                        });
                                    });
                                //});
                                const BKNOB_SIZE: f32 = 26.0;
                                const BTEXT_SIZE: f32 = 11.0;
                                // Filter section
                                //ui.horizontal(|ui| {
                                    ui.vertical(|ui|{
                                        ui.horizontal(|ui|{
                                            match *filter_select.lock().unwrap() {
                                                UIBottomSelection::Filter1 => {
                                                    match params.filter_alg_type.value() {
                                                        FilterAlgorithms::SVF => {
                                                            ui.vertical(|ui|{
                                                                let filter_alg_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_alg_type,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(TEAL_GREEN)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text(
"The filter algorithm to use.
SVF: State Variable Filter model
Tilt: A linear filter that cuts one side and boosts another
VCF: Voltage Controlled Filter model
V4: Analog Inspired Filter Idea
A4I: Averaged 4 Pole Integrator
A4II: Averaged 4 Pole Integrator II".to_string());
                                                                ui.add(filter_alg_knob);
                                                                let filter_lp_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_lp_amount,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD.gamma_multiply(2.0))
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Low passed signal output".to_string());
                                                                ui.add(filter_lp_knob);
                                                                let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_resonance,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter resonance/emphasis".to_string());
                                                                ui.add(filter_resonance_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_wet,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("How much signal to process in the filter".to_string());
                                                                ui.add(filter_wet_knob);
                                                                let filter_bp_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_bp_amount,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD.gamma_multiply(2.0))
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Band passed signal output".to_string());
                                                                ui.add(filter_bp_knob);
                                                                let filter_res_type_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_res_type,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Which resonance algorithm to use".to_string());
                                                                ui.add(filter_res_type_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_cutoff,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter cutoff/center frequency".to_string());
                                                                ui.add(filter_cutoff_knob);
                                                                let filter_hp_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_hp_amount,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD.gamma_multiply(2.0))
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("High passed signal output".to_string());
                                                                ui.add(filter_hp_knob);
                                                                let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_env_peak,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_readable_box(false)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("The relative cutoff level to reach in the ADSR envelope".to_string());
                                                                ui.add(filter_env_peak);
                                                            });
                                                        },
                                                        FilterAlgorithms::TILT => {
                                                            ui.vertical(|ui|{
                                                                let filter_alg_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_alg_type,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(TEAL_GREEN)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text(
"The filter algorithm to use.
SVF: State Variable Filter model
Tilt: A linear filter that cuts one side and boosts another
VCF: Voltage Controlled Filter model
V4: Analog Inspired Filter Idea
A4I: Averaged 4 Pole Integrator
A4II: Averaged 4 Pole Integrator II".to_string());
                                                                ui.add(filter_alg_knob);
                                                                let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_wet,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("How much signal to process in the filter".to_string());
                                                                ui.add(filter_wet_knob);
                                                                let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_resonance,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter resonance/emphasis".to_string());
                                                                ui.add(filter_resonance_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_cutoff,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter cutoff/center frequency".to_string());
                                                                ui.add(filter_cutoff_knob);
                                                                let filter_tilt_type_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.tilt_filter_type,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD.gamma_multiply(2.0))
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Tilt filter algorithm type".to_string());
                                                                ui.add(filter_tilt_type_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_env_peak,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_readable_box(false)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("The relative cutoff level to reach in the ADSR envelope".to_string());
                                                                ui.add(filter_env_peak);
                                                            });
                                                        },
                                                        FilterAlgorithms::VCF => {
                                                            ui.vertical(|ui|{
                                                                let filter_alg_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_alg_type,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(TEAL_GREEN.gamma_multiply(2.0))
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text(
"The filter algorithm to use.
SVF: State Variable Filter model
Tilt: A linear filter that cuts one side and boosts another
VCF: Voltage Controlled Filter model
V4: Analog Inspired Filter Idea
A4I: Averaged 4 Pole Integrator
A4II: Averaged 4 Pole Integrator II".to_string());
                                                                ui.add(filter_alg_knob);
                                                                let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_wet,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("How much signal to process in the filter".to_string());
                                                                ui.add(filter_wet_knob);
                                                                let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_resonance,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter resonance/emphasis".to_string());
                                                                ui.add(filter_resonance_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_cutoff,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter cutoff/center frequency".to_string());
                                                                ui.add(filter_cutoff_knob);
                                                                let vcf_filter_type_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.vcf_filter_type,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE);
                                                                ui.add(vcf_filter_type_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_env_peak,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_readable_box(false)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("The relative cutoff level to reach in the ADSR envelope".to_string());
                                                                ui.add(filter_env_peak);
                                                            });
                                                        },
                                                        FilterAlgorithms::V4 => {
                                                            ui.vertical(|ui|{
                                                                let filter_alg_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_alg_type,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(TEAL_GREEN.gamma_multiply(2.0))
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text(
"The filter algorithm to use.
SVF: State Variable Filter model
Tilt: A linear filter that cuts one side and boosts another
VCF: Voltage Controlled Filter model
V4: Analog Inspired Filter Idea
A4I: Averaged 4 Pole Integrator
A4II: Averaged 4 Pole Integrator II".to_string());
                                                                ui.add(filter_alg_knob);
                                                                let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_wet,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("How much signal to process in the filter".to_string());
                                                                ui.add(filter_wet_knob);
                                                                let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_resonance,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter resonance/emphasis".to_string());
                                                                ui.add(filter_resonance_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_cutoff,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter cutoff/center frequency".to_string());
                                                                ui.add(filter_cutoff_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_env_peak,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_readable_box(false)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("The relative cutoff level to reach in the ADSR envelope".to_string());
                                                                ui.add(filter_env_peak);
                                                            });
                                                        },
                                                        FilterAlgorithms::A4I => {
                                                            ui.vertical(|ui|{
                                                                let filter_alg_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_alg_type,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(TEAL_GREEN.gamma_multiply(2.0))
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text(
"The filter algorithm to use.
SVF: State Variable Filter model
Tilt: A linear filter that cuts one side and boosts another
VCF: Voltage Controlled Filter model
V4: Analog Inspired Filter Idea
A4I: Averaged 4 Pole Integrator
A4II: Averaged 4 Pole Integrator II".to_string());
                                                                ui.add(filter_alg_knob);
                                                                let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_wet,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("How much signal to process in the filter".to_string());
                                                                ui.add(filter_wet_knob);
                                                                let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_resonance,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter resonance/emphasis".to_string());
                                                                ui.add(filter_resonance_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_cutoff,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter cutoff/center frequency".to_string());
                                                                ui.add(filter_cutoff_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_env_peak,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_readable_box(false)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("The relative cutoff level to reach in the ADSR envelope".to_string());
                                                                ui.add(filter_env_peak);
                                                            });
                                                        },
                                                        FilterAlgorithms::A4II => {
                                                            ui.vertical(|ui|{
                                                                let filter_alg_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_alg_type,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(TEAL_GREEN.gamma_multiply(2.0))
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text(
"The filter algorithm to use.
SVF: State Variable Filter model
Tilt: A linear filter that cuts one side and boosts another
VCF: Voltage Controlled Filter model
V4: Analog Inspired Filter Idea
A4I: Averaged 4 Pole Integrator
A4II: Averaged 4 Pole Integrator II".to_string());
                                                                ui.add(filter_alg_knob);
                                                                let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_wet,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("How much signal to process in the filter".to_string());
                                                                ui.add(filter_wet_knob);
                                                                let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_resonance,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter resonance/emphasis".to_string());
                                                                ui.add(filter_resonance_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_cutoff,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter cutoff/center frequency".to_string());
                                                                ui.add(filter_cutoff_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_env_peak,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_readable_box(false)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("The relative cutoff level to reach in the ADSR envelope".to_string());
                                                                ui.add(filter_env_peak);
                                                            });
                                                        },
                                                    }
                                                },
                                                UIBottomSelection::Filter2 => {
                                                    match params.filter_alg_type_2.value() {
                                                        FilterAlgorithms::SVF => {
                                                            ui.vertical(|ui|{
                                                                let filter_alg_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_alg_type_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(TEAL_GREEN)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text(
"The filter algorithm to use.
SVF: State Variable Filter model
Tilt: A linear filter that cuts one side and boosts another
VCF: Voltage Controlled Filter model
V4: Analog Inspired Filter Idea
A4I: Averaged 4 Pole Integrator
A4II: Averaged 4 Pole Integrator II".to_string());
                                                                ui.add(filter_alg_knob);
                                                                let filter_lp_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_lp_amount_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD.gamma_multiply(2.0))
                                                                    .set_text_size(BTEXT_SIZE);
                                                                ui.add(filter_lp_knob);
                                                                let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_resonance_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter resonance/emphasis".to_string());
                                                                ui.add(filter_resonance_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_wet_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("How much signal to process in the filter".to_string());
                                                                ui.add(filter_wet_knob);
                                                                let filter_bp_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_bp_amount_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD.gamma_multiply(2.0))
                                                                    .set_text_size(BTEXT_SIZE);
                                                                ui.add(filter_bp_knob);
                                                                let filter_res_type_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_res_type_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE);
                                                                ui.add(filter_res_type_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_cutoff_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter cutoff/center frequency".to_string());
                                                                ui.add(filter_cutoff_knob);
                                                                let filter_hp_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_hp_amount_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD.gamma_multiply(2.0))
                                                                    .set_text_size(BTEXT_SIZE);
                                                                ui.add(filter_hp_knob);
                                                                let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_env_peak_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_readable_box(false)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("The relative cutoff level to reach in the ADSR envelope".to_string());
                                                                ui.add(filter_env_peak);
                                                            });
                                                        },
                                                        FilterAlgorithms::TILT => {
                                                            ui.vertical(|ui|{
                                                                let filter_alg_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_alg_type_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(TEAL_GREEN)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text(
"The filter algorithm to use.
SVF: State Variable Filter model
Tilt: A linear filter that cuts one side and boosts another
VCF: Voltage Controlled Filter model
V4: Analog Inspired Filter Idea
A4I: Averaged 4 Pole Integrator
A4II: Averaged 4 Pole Integrator II".to_string());
                                                                ui.add(filter_alg_knob);
                                                                let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_wet_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("How much signal to process in the filter".to_string());
                                                                ui.add(filter_wet_knob);
                                                                let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_resonance_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter resonance/emphasis".to_string());
                                                                ui.add(filter_resonance_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_cutoff_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter cutoff/center frequency".to_string());
                                                                ui.add(filter_cutoff_knob);
                                                                let filter_tilt_type_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.tilt_filter_type_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD.gamma_multiply(2.0))
                                                                    .set_text_size(BTEXT_SIZE);
                                                                ui.add(filter_tilt_type_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_env_peak_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_readable_box(false)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("The relative cutoff level to reach in the ADSR envelope".to_string());
                                                                ui.add(filter_env_peak);
                                                            });
                                                        },
                                                        FilterAlgorithms::VCF => {
                                                            ui.vertical(|ui|{
                                                                let filter_alg_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_alg_type_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(TEAL_GREEN)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text(
"The filter algorithm to use.
SVF: State Variable Filter model
Tilt: A linear filter that cuts one side and boosts another
VCF: Voltage Controlled Filter model
V4: Analog Inspired Filter Idea
A4I: Averaged 4 Pole Integrator
A4II: Averaged 4 Pole Integrator II".to_string());
                                                                ui.add(filter_alg_knob);
                                                                let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_wet_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("How much signal to process in the filter".to_string());
                                                                ui.add(filter_wet_knob);
                                                                let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_resonance_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter resonance/emphasis".to_string());
                                                                ui.add(filter_resonance_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_cutoff_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter cutoff/center frequency".to_string());
                                                                ui.add(filter_cutoff_knob);
                                                                let vcf_filter_type_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.vcf_filter_type_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD.gamma_multiply(2.0))
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("VCF filter algorithm to use".to_string());
                                                                ui.add(vcf_filter_type_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_env_peak_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_readable_box(false)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("The relative cutoff level to reach in the ADSR envelope".to_string());
                                                                ui.add(filter_env_peak);
                                                            });
                                                        },
                                                        FilterAlgorithms::V4 => {
                                                            ui.vertical(|ui|{
                                                                let filter_alg_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_alg_type_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(TEAL_GREEN)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text(
"The filter algorithm to use.
SVF: State Variable Filter model
Tilt: A linear filter that cuts one side and boosts another
VCF: Voltage Controlled Filter model
V4: Analog Inspired Filter Idea
A4I: Averaged 4 Pole Integrator
A4II: Averaged 4 Pole Integrator II".to_string());
                                                                ui.add(filter_alg_knob);
                                                                let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_wet_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("How much signal to process in the filter".to_string());
                                                                ui.add(filter_wet_knob);
                                                                let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_resonance_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter resonance/emphasis".to_string());
                                                                ui.add(filter_resonance_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_cutoff_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter cutoff/center frequency".to_string());
                                                                ui.add(filter_cutoff_knob);
                                                                let vcf_filter_type_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.vcf_filter_type_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD.gamma_multiply(2.0))
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("VCF filter algorithm to use".to_string());
                                                                ui.add(vcf_filter_type_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_env_peak_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_readable_box(false)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("The relative cutoff level to reach in the ADSR envelope".to_string());
                                                                ui.add(filter_env_peak);
                                                            });
                                                        },
                                                        FilterAlgorithms::A4I => {
                                                            ui.vertical(|ui|{
                                                                let filter_alg_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_alg_type_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(TEAL_GREEN)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text(
"The filter algorithm to use.
SVF: State Variable Filter model
Tilt: A linear filter that cuts one side and boosts another
VCF: Voltage Controlled Filter model
V4: Analog Inspired Filter Idea
A4I: Averaged 4 Pole Integrator
A4II: Averaged 4 Pole Integrator II".to_string());
                                                                ui.add(filter_alg_knob);
                                                                let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_wet_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("How much signal to process in the filter".to_string());
                                                                ui.add(filter_wet_knob);
                                                                let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_resonance_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter resonance/emphasis".to_string());
                                                                ui.add(filter_resonance_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_cutoff_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter cutoff/center frequency".to_string());
                                                                ui.add(filter_cutoff_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_env_peak_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_readable_box(false)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("The relative cutoff level to reach in the ADSR envelope".to_string());
                                                                ui.add(filter_env_peak);
                                                            });
                                                        },
                                                        FilterAlgorithms::A4II => {
                                                            ui.vertical(|ui|{
                                                                let filter_alg_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_alg_type_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(TEAL_GREEN)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text(
"The filter algorithm to use.
SVF: State Variable Filter model
Tilt: A linear filter that cuts one side and boosts another
VCF: Voltage Controlled Filter model
V4: Analog Inspired Filter Idea
A4I: Averaged 4 Pole Integrator
A4II: Averaged 4 Pole Integrator II".to_string());
                                                                ui.add(filter_alg_knob);
                                                                let filter_wet_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_wet_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("How much signal to process in the filter".to_string());
                                                                ui.add(filter_wet_knob);
                                                                let filter_resonance_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_resonance_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter resonance/emphasis".to_string());
                                                                ui.add(filter_resonance_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_cutoff_knob = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_cutoff_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("Filter cutoff/center frequency".to_string());
                                                                ui.add(filter_cutoff_knob);
                                                            });
                                                            ui.vertical(|ui|{
                                                                let filter_env_peak = ui_knob::ArcKnob::for_param(
                                                                    &params.filter_env_peak_2,
                                                                    setter,
                                                                    BKNOB_SIZE,
                                                                    KnobLayout::Horizonal)
                                                                    .preset_style(ui_knob::KnobStyle::Preset1)
                                                                    .set_fill_color(DARK_GREY_UI_COLOR)
                                                                    .set_line_color(YELLOW_MUSTARD)
                                                                    .set_readable_box(false)
                                                                    .set_text_size(BTEXT_SIZE)
                                                                    .set_hover_text("The relative cutoff level to reach in the ADSR envelope".to_string());
                                                                ui.add(filter_env_peak);
                                                            });
                                                        },
                                                    }
                                                },
                                                UIBottomSelection::Pitch1 => {
                                                    ui.vertical(|ui|{
                                                        ui.horizontal(|ui|{
                                                            let pitch_toggle = toggle_switch::ToggleSwitch::for_param(&params.pitch_enable, setter);
                                                            ui.add(pitch_toggle);
                                                            ui.label(RichText::new("Enable Pitch Envelope")
                                                                .font(FONT)
                                                                .color(FONT_COLOR)
                                                            );
                                                        });

                                                        ui.horizontal(|ui|{
                                                            let pitch_env_peak_knob = ui_knob::ArcKnob::for_param(
                                                                &params.pitch_env_peak,
                                                                setter,
                                                                BKNOB_SIZE,
                                                                KnobLayout::Horizonal)
                                                                .preset_style(ui_knob::KnobStyle::Preset1)
                                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                                .set_line_color(TEAL_GREEN)
                                                                .set_readable_box(false)
                                                                .set_text_size(BTEXT_SIZE)
                                                                .set_hover_text("The relative pitch level to reach in the ADSR envelope".to_string());
                                                            ui.add(pitch_env_peak_knob);

                                                            let pitch_routing_knob = ui_knob::ArcKnob::for_param(
                                                                &params.pitch_routing,
                                                                setter,
                                                                BKNOB_SIZE,
                                                                KnobLayout::Horizonal)
                                                                .preset_style(ui_knob::KnobStyle::Preset1)
                                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                                .set_line_color(TEAL_GREEN)
                                                                .set_readable_box(false)
                                                                .set_text_size(BTEXT_SIZE)
                                                                .set_hover_text("Where the pitch envelope should be applied".to_string());
                                                            ui.add(pitch_routing_knob);
                                                        });
                                                    });
                                                    ui.add_space(BKNOB_SIZE*3.39);
                                                },
                                                UIBottomSelection::Pitch2 => {
                                                    ui.vertical(|ui|{
                                                        ui.horizontal(|ui|{
                                                            let pitch_toggle_2 = toggle_switch::ToggleSwitch::for_param(&params.pitch_enable_2, setter);
                                                            ui.add(pitch_toggle_2);
                                                            ui.label(RichText::new("Enable Pitch Envelope")
                                                                .font(FONT)
                                                                .color(FONT_COLOR)
                                                            );
                                                        });

                                                        ui.horizontal(|ui|{
                                                            let pitch_env_peak_knob_2 = ui_knob::ArcKnob::for_param(
                                                                &params.pitch_env_peak_2,
                                                                setter,
                                                                BKNOB_SIZE,
                                                                KnobLayout::Horizonal)
                                                                .preset_style(ui_knob::KnobStyle::Preset1)
                                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                                .set_line_color(TEAL_GREEN)
                                                                .set_readable_box(false)
                                                                .set_text_size(BTEXT_SIZE)
                                                                .set_hover_text("The relative pitch level to reach in the ADSR envelope".to_string());
                                                            ui.add(pitch_env_peak_knob_2);

                                                            let pitch_routing_knob_2 = ui_knob::ArcKnob::for_param(
                                                                &params.pitch_routing_2,
                                                                setter,
                                                                BKNOB_SIZE,
                                                                KnobLayout::Horizonal)
                                                                .preset_style(ui_knob::KnobStyle::Preset1)
                                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                                .set_line_color(TEAL_GREEN)
                                                                .set_readable_box(false)
                                                                .set_text_size(BTEXT_SIZE)
                                                                .set_hover_text("Where the pitch envelope should be applied".to_string());
                                                            ui.add(pitch_routing_knob_2);
                                                        });
                                                    });
                                                    ui.add_space(BKNOB_SIZE*3.39);
                                                }
                                            }
                                        });
                                    });

                                    // LFO Box
                                    ui.vertical(|ui|{
                                        // Got stuck in a doube nested deadlock so added this clone
                                        let current_select = lfo_select.lock().unwrap().clone();
                                        //ui.separator();
                                        match current_select {
                                            LFOSelect::LFO1 => {
                                                ui.vertical(|ui|{
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("LFO Enabled")
                                                            .font(FONT)
                                                        );
                                                        let lfo1_toggle = toggle_switch::ToggleSwitch::for_param(&params.lfo1_enable, setter);
                                                        ui.add(lfo1_toggle);
                                                    });
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Sync")
                                                            .font(FONT)
                                                        )
                                                            .on_hover_text("Sync LFO values to your DAW");
                                                        let lfosync1 = toggle_switch::ToggleSwitch::for_param(&params.lfo1_sync, setter);
                                                        ui.add(lfosync1);
                                                        ui.separator();
                                                        ui.label(RichText::new("Retrig")
                                                            .font(FONT)
                                                        )
                                                            .on_hover_text("When to reset the LFO".to_string());
                                                        ui.add(ParamSlider::for_param(&params.lfo1_retrigger, setter).with_width(80.0));
                                                    });
                                                    ui.separator();
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Rate ")
                                                            .font(FONT)
                                                        );
                                                        if params.lfo1_sync.value() {
                                                            ui.add(ParamSlider::for_param(&params.lfo1_snap, setter).with_width(180.0));
                                                        } else {
                                                            ui.add(ParamSlider::for_param(&params.lfo1_freq, setter).with_width(180.0));
                                                        }
                                                    });
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Shape")
                                                            .font(FONT)
                                                        );
                                                        ui.add(ParamSlider::for_param(&params.lfo1_waveform, setter).with_width(180.0));
                                                    });
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Phase")
                                                            .font(FONT)
                                                        );
                                                        ui.add(ParamSlider::for_param(&params.lfo1_phase, setter).with_width(180.0));
                                                    });
                                                });
                                            },
                                            LFOSelect::LFO2 => {
                                                ui.vertical(|ui|{
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("LFO Enabled")
                                                            .font(FONT)
                                                        );
                                                        let lfo2_toggle = toggle_switch::ToggleSwitch::for_param(&params.lfo2_enable, setter);
                                                        ui.add(lfo2_toggle);
                                                    });
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Sync")
                                                            .font(FONT)
                                                        )
                                                            .on_hover_text("Sync LFO values to your DAW");
                                                        let lfosync2 = toggle_switch::ToggleSwitch::for_param(&params.lfo2_sync, setter);
                                                        ui.add(lfosync2);
                                                        ui.separator();
                                                        ui.label(RichText::new("Retrig")
                                                            .font(FONT)
                                                        ).on_hover_text("When to reset the LFO".to_string());
                                                        ui.add(ParamSlider::for_param(&params.lfo2_retrigger, setter).with_width(80.0));
                                                    });
                                                    ui.separator();
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Rate ")
                                                            .font(FONT)
                                                        );
                                                        if params.lfo2_sync.value() {
                                                            ui.add(ParamSlider::for_param(&params.lfo2_snap, setter).with_width(180.0));
                                                        } else {
                                                            ui.add(ParamSlider::for_param(&params.lfo2_freq, setter).with_width(180.0));
                                                        }
                                                    });
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Shape")
                                                            .font(FONT)
                                                        );
                                                        ui.add(ParamSlider::for_param(&params.lfo2_waveform, setter).with_width(180.0));
                                                    });
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Phase")
                                                            .font(FONT)
                                                        );
                                                        ui.add(ParamSlider::for_param(&params.lfo2_phase, setter).with_width(180.0));
                                                    });
                                                });
                                            },
                                            LFOSelect::LFO3 => {
                                                ui.vertical(|ui|{
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("LFO Enabled")
                                                            .font(FONT)
                                                        );
                                                        let lfo3_toggle = toggle_switch::ToggleSwitch::for_param(&params.lfo3_enable, setter);
                                                        ui.add(lfo3_toggle);
                                                    });
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Sync")
                                                            .font(FONT)
                                                        )
                                                            .on_hover_text("Sync LFO values to your DAW");
                                                        let lfosync3 = toggle_switch::ToggleSwitch::for_param(&params.lfo3_sync, setter);
                                                        ui.add(lfosync3);
                                                        ui.separator();
                                                        ui.label(RichText::new("Retrig")
                                                            .font(FONT)
                                                        ).on_hover_text("When to reset the LFO".to_string());
                                                        ui.add(ParamSlider::for_param(&params.lfo3_retrigger, setter).with_width(80.0));
                                                    });
                                                    ui.separator();
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Rate ")
                                                            .font(FONT)
                                                        );
                                                        if params.lfo3_sync.value() {
                                                            ui.add(ParamSlider::for_param(&params.lfo3_snap, setter).with_width(180.0));
                                                        } else {
                                                            ui.add(ParamSlider::for_param(&params.lfo3_freq, setter).with_width(180.0));
                                                        }
                                                    });
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Shape")
                                                            .font(FONT)
                                                        );
                                                        ui.add(ParamSlider::for_param(&params.lfo3_waveform, setter).with_width(180.0));
                                                    });
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Phase")
                                                            .font(FONT)
                                                        );
                                                        ui.add(ParamSlider::for_param(&params.lfo3_phase, setter).with_width(180.0));
                                                    });
                                                });
                                            },
                                            LFOSelect::Misc => {
                                                ui.vertical(|ui|{
                                                    let max_voice_knob = ui_knob::ArcKnob::for_param(
                                                        &params.voice_limit,
                                                        setter,
                                                        11.0,
                                                        KnobLayout::HorizontalInline)
                                                        .preset_style(ui_knob::KnobStyle::Preset1)
                                                        .set_fill_color(DARK_GREY_UI_COLOR)
                                                        .set_line_color(YELLOW_MUSTARD)
                                                        .set_text_size(TEXT_SIZE)
                                                        .set_hover_text("The maximum number of voices that can be playing at once".to_string());
                                                    ui.add(max_voice_knob);
                                                    ui.separator();
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Link Cutoff 2 to Cutoff 1")
                                                            .font(FONT)
                                                        )
                                                            .on_hover_text("Filter 1 will control both filter cutoff values");
                                                        let filter_cutoff_link = toggle_switch::ToggleSwitch::for_param(&params.filter_cutoff_link, setter);
                                                        ui.add(filter_cutoff_link);
                                                    });
                                                    ui.separator();
                                                    ui.horizontal(|ui|{
                                                        ui.label(RichText::new("Stereo Behavior")
                                                            .font(FONT)
                                                        )
                                                            .on_hover_text("The stereo algorithm to use for voice spreads");
                                                        ui.add(ParamSlider::for_param(&params.stereo_algorithm, setter).with_width(180.0));
                                                    }); 
                                                });
                                            },
                                            LFOSelect::FM => {
                                                ui.horizontal(|ui|{
                                                    ui.vertical(|ui|{
                                                        let fm_one_to_two = ui_knob::ArcKnob::for_param(
                                                            &params.fm_one_to_two,
                                                            setter,
                                                            28.0,
                                                            KnobLayout::Horizonal)
                                                                .preset_style(ui_knob::KnobStyle::Preset1)
                                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                                .set_line_color(TEAL_GREEN)
                                                                .set_show_label(true)
                                                                .set_text_size(10.0)
                                                                .set_hover_text("The amount Generator 1 modulates generator 2".to_string());
                                                        ui.add(fm_one_to_two);
                                                        let fm_one_to_three = ui_knob::ArcKnob::for_param(
                                                            &params.fm_one_to_three,
                                                            setter,
                                                            28.0,
                                                            KnobLayout::Horizonal)
                                                                .preset_style(ui_knob::KnobStyle::Preset1)
                                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                                .set_line_color(TEAL_GREEN)
                                                                .set_show_label(true)
                                                                .set_text_size(10.0)
                                                                .set_hover_text("The amount Generator 1 modulates generator 3".to_string());
                                                        ui.add(fm_one_to_three);
                                                        let fm_two_to_three = ui_knob::ArcKnob::for_param(
                                                            &params.fm_two_to_three,
                                                            setter,
                                                            28.0,
                                                            KnobLayout::Horizonal)
                                                                .preset_style(ui_knob::KnobStyle::Preset1)
                                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                                .set_line_color(TEAL_GREEN)
                                                                .set_show_label(true)
                                                                .set_text_size(10.0)
                                                                .set_hover_text("The amount Generator 2 modulates generator 3".to_string());
                                                        ui.add(fm_two_to_three);
                                                    });
                                                    // ADSR for FM Signal
                                                    ui.add(
                                                        VerticalParamSlider::for_param(&params.fm_attack, setter)
                                                            .with_width(VERT_BAR_WIDTH)
                                                            .with_height(VERT_BAR_HEIGHT)
                                                            .set_reversed(true)
                                                            .override_colors(
                                                                LIGHTER_GREY_UI_COLOR,
                                                                TEAL_GREEN,
                                                            ),
                                                    );
                                                    ui.add(
                                                        VerticalParamSlider::for_param(&params.fm_decay, setter)
                                                            .with_width(VERT_BAR_WIDTH)
                                                            .with_height(VERT_BAR_HEIGHT)
                                                            .set_reversed(true)
                                                            .override_colors(
                                                                LIGHTER_GREY_UI_COLOR,
                                                                TEAL_GREEN,
                                                            ),
                                                    );
                                                    ui.add(
                                                        VerticalParamSlider::for_param(&params.fm_sustain, setter)
                                                            .with_width(VERT_BAR_WIDTH)
                                                            .with_height(VERT_BAR_HEIGHT)
                                                            .set_reversed(true)
                                                            .override_colors(
                                                                LIGHTER_GREY_UI_COLOR,
                                                                TEAL_GREEN,
                                                            ),
                                                    );
                                                    ui.add(
                                                        VerticalParamSlider::for_param(&params.fm_release, setter)
                                                            .with_width(VERT_BAR_WIDTH)
                                                            .with_height(VERT_BAR_HEIGHT)
                                                            .set_reversed(true)
                                                            .override_colors(
                                                                LIGHTER_GREY_UI_COLOR,
                                                                TEAL_GREEN,
                                                            ),
                                                    );
                                                    ui.vertical(|ui|{
                                                        ui.add(
                                                            BeizerButton::BeizerButton::for_param(
                                                                &params.fm_attack_curve,
                                                                setter,
                                                                5.1,
                                                                2.0,
                                                                ButtonLayout::HorizontalInline,
                                                                true,
                                                            )
                                                            .with_background_color(MEDIUM_GREY_UI_COLOR)
                                                            .with_line_color(YELLOW_MUSTARD),
                                                        ).on_hover_text_at_pointer("The behavior of Attack movement in the envelope".to_string());
                                                        ui.add(
                                                            BeizerButton::BeizerButton::for_param(
                                                                &params.fm_decay_curve,
                                                                setter,
                                                                5.1,
                                                                2.0,
                                                                ButtonLayout::HorizontalInline,
                                                                false,
                                                            )
                                                            .with_background_color(MEDIUM_GREY_UI_COLOR)
                                                            .with_line_color(YELLOW_MUSTARD),
                                                        ).on_hover_text_at_pointer("The behavior of Decay movement in the envelope".to_string());
                                                        ui.add(
                                                            BeizerButton::BeizerButton::for_param(
                                                                &params.fm_release_curve,
                                                                setter,
                                                                5.1,
                                                                2.0,
                                                                ButtonLayout::HorizontalInline,
                                                                false,
                                                            )
                                                            .with_background_color(MEDIUM_GREY_UI_COLOR)
                                                            .with_line_color(YELLOW_MUSTARD),
                                                        ).on_hover_text_at_pointer("The behavior of Release movement in the envelope".to_string());
                                                        let fm_cycle_knob = ui_knob::ArcKnob::for_param(
                                                            &params.fm_cycles,
                                                            setter,
                                                            26.0,
                                                            KnobLayout::Horizonal)
                                                                .preset_style(ui_knob::KnobStyle::Preset1)
                                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                                .set_line_color(TEAL_GREEN)
                                                                .set_show_label(true)
                                                                .set_text_size(10.0)
                                                                .set_hover_text("The amount of FM iterations".to_string());
                                                        ui.add(fm_cycle_knob);
                                                        ui.label("Hover for help")
                                                            .on_hover_text_at_pointer("HIGHLY Recommend putting a limiter after Actuate for FM!

The FM knobs let a signal modulate another signal.
Turning any FM knob enables FM Processing, then the knob alters the phase of the FM as you turn it further.
The ADSR envelope here controls the behavior of the FM amount(knobs) sent at a time.
For constant FM, turn Sustain to 100% and A,D,R to 0%".to_string());
                                                    });
                                                });
                                            },
                                            LFOSelect::Modulation => {
                                                ui.vertical(|ui|{
                                                    // Modulator section 1
                                                    //////////////////////////////////////////////////////////////////////////////////
                                                    ui.horizontal(|ui|{
                                                        let mod_1_knob = ui_knob::ArcKnob::for_param(
                                                            &params.mod_amount_knob_1,
                                                            setter,
                                                            12.0,
                                                            KnobLayout::SquareNoLabel)
                                                                .preset_style(ui_knob::KnobStyle::Preset2)
                                                                .set_fill_color(DARK_GREY_UI_COLOR)
                                                                .set_line_color(TEAL_GREEN)
                                                                .set_show_label(false);
                                                        ui.add(mod_1_knob);
                                                        ui.separator();
                                                        let ms1 = ComboBoxParam::ParamComboBox::for_param(&params.mod_source_1, setter, vec![
                                                            String::from("None"),
                                                            String::from("Velocity"),
                                                            String::from("LFO1"),
                                                            String::from("LFO2"),
                                                            String::from("LFO3"),
                                                        ],
                                                        "ms1".to_string());
                                                        ui.add(ms1);
                                                        ui.label(RichText::new("Mods")
                                                            .font(FONT));
                                                        let md1 = ComboBoxParam::ParamComboBox::for_param(&params.mod_destination_1, setter, vec![
                                                            String::from("None"),
                                                            String::from("Cutoff_1"),
                                                            String::from("Cutoff_2"),
                                                            String::from("Resonance_1"),
                                                            String::from("Resonance_2"),
                                                            String::from("All_Gain"),
                                                            String::from("Osc1_Gain"),
                                                            String::from("Osc2_Gain"),
                                                            String::from("Osc3_Gain"),
                                                            String::from("All_Detune"),
                                                            String::from("Osc1Detune"),
                                                            String::from("Osc2Detune"),
                                                            String::from("Osc3Detune"),
                                                            String::from("All_UniDetune"),
                                                            String::from("Osc1UniDetune"),
                                                            String::from("Osc2UniDetune"),
                                                            String::from("Osc3UniDetune"),
                                                        ],
                                                        "md1".to_string());
                                                        ui.add(md1);
                                                    });
                                                    ui.separator();

                                                    // Modulator section 2
                                                    //////////////////////////////////////////////////////////////////////////////////
                                                    ui.horizontal(|ui|{
                                                        let mod_2_knob = ui_knob::ArcKnob::for_param(
                                                            &params.mod_amount_knob_2,
                                                            setter,
                                                            12.0,
                                                            KnobLayout::SquareNoLabel)
                                                            .preset_style(ui_knob::KnobStyle::Preset2)
                                                            .set_fill_color(DARK_GREY_UI_COLOR)
                                                            .set_line_color(TEAL_GREEN)
                                                            .set_show_label(false);
                                                        ui.add(mod_2_knob);
                                                        ui.separator();
                                                        let ms2 = ComboBoxParam::ParamComboBox::for_param(&params.mod_source_2, setter, vec![
                                                            String::from("None"),
                                                            String::from("Velocity"),
                                                            String::from("LFO1"),
                                                            String::from("LFO2"),
                                                            String::from("LFO3"),
                                                        ],
                                                        "ms2".to_string());
                                                        ui.add(ms2);
                                                        ui.label(RichText::new("Mods")
                                                            .font(FONT));
                                                        let md2 = ComboBoxParam::ParamComboBox::for_param(&params.mod_destination_2, setter, vec![
                                                            String::from("None"),
                                                            String::from("Cutoff_1"),
                                                            String::from("Cutoff_2"),
                                                            String::from("Resonance_1"),
                                                            String::from("Resonance_2"),
                                                            String::from("All_Gain"),
                                                            String::from("Osc1_Gain"),
                                                            String::from("Osc2_Gain"),
                                                            String::from("Osc3_Gain"),
                                                            String::from("All_Detune"),
                                                            String::from("Osc1Detune"),
                                                            String::from("Osc2Detune"),
                                                            String::from("Osc3Detune"),
                                                            String::from("All_UniDetune"),
                                                            String::from("Osc1UniDetune"),
                                                            String::from("Osc2UniDetune"),
                                                            String::from("Osc3UniDetune"),
                                                        ],
                                                        "md2".to_string());
                                                        ui.add(md2);
                                                    });
                                                    ui.separator();

                                                    // Modulator section 3
                                                    //////////////////////////////////////////////////////////////////////////////////
                                                    ui.horizontal(|ui|{
                                                        let mod_3_knob = ui_knob::ArcKnob::for_param(
                                                            &params.mod_amount_knob_3,
                                                            setter,
                                                            12.0,
                                                            KnobLayout::SquareNoLabel)
                                                            .preset_style(ui_knob::KnobStyle::Preset2)
                                                            .set_fill_color(DARK_GREY_UI_COLOR)
                                                            .set_line_color(TEAL_GREEN)
                                                            .set_show_label(false);
                                                        ui.add(mod_3_knob);
                                                        ui.separator();
                                                        let ms3 = ComboBoxParam::ParamComboBox::for_param(&params.mod_source_3, setter, vec![
                                                            String::from("None"),
                                                            String::from("Velocity"),
                                                            String::from("LFO1"),
                                                            String::from("LFO2"),
                                                            String::from("LFO3"),
                                                        ],
                                                        "ms3".to_string());
                                                        ui.add(ms3);
                                                        ui.label(RichText::new("Mods")
                                                            .font(FONT));
                                                        let md3 = ComboBoxParam::ParamComboBox::for_param(&params.mod_destination_3, setter, vec![
                                                            String::from("None"),
                                                            String::from("Cutoff_1"),
                                                            String::from("Cutoff_2"),
                                                            String::from("Resonance_1"),
                                                            String::from("Resonance_2"),
                                                            String::from("All_Gain"),
                                                            String::from("Osc1_Gain"),
                                                            String::from("Osc2_Gain"),
                                                            String::from("Osc3_Gain"),
                                                            String::from("All_Detune"),
                                                            String::from("Osc1Detune"),
                                                            String::from("Osc2Detune"),
                                                            String::from("Osc3Detune"),
                                                            String::from("All_UniDetune"),
                                                            String::from("Osc1UniDetune"),
                                                            String::from("Osc2UniDetune"),
                                                            String::from("Osc3UniDetune"),
                                                        ],
                                                        "md3".to_string());
                                                        ui.add(md3);
                                                    });
                                                    ui.separator();

                                                    // Modulator section 4
                                                    //////////////////////////////////////////////////////////////////////////////////
                                                    ui.horizontal(|ui|{
                                                        let mod_4_knob = ui_knob::ArcKnob::for_param(
                                                            &params.mod_amount_knob_4,
                                                            setter,
                                                            12.0,
                                                            KnobLayout::SquareNoLabel)
                                                            .preset_style(ui_knob::KnobStyle::Preset2)
                                                            .set_fill_color(DARK_GREY_UI_COLOR)
                                                            .set_line_color(TEAL_GREEN)
                                                            .set_show_label(false);
                                                        ui.add(mod_4_knob);
                                                        ui.separator();
                                                        let ms4 = ComboBoxParam::ParamComboBox::for_param(&params.mod_source_4, setter, vec![
                                                            String::from("None"),
                                                            String::from("Velocity"),
                                                            String::from("LFO1"),
                                                            String::from("LFO2"),
                                                            String::from("LFO3"),
                                                        ],
                                                        "ms4".to_string());
                                                        ui.add(ms4);
                                                        ui.label(RichText::new("Mods")
                                                            .font(FONT));
                                                        let md4 = ComboBoxParam::ParamComboBox::for_param(&params.mod_destination_4, setter, vec![
                                                            String::from("None"),
                                                            String::from("Cutoff_1"),
                                                            String::from("Cutoff_2"),
                                                            String::from("Resonance_1"),
                                                            String::from("Resonance_2"),
                                                            String::from("All_Gain"),
                                                            String::from("Osc1_Gain"),
                                                            String::from("Osc2_Gain"),
                                                            String::from("Osc3_Gain"),
                                                            String::from("All_Detune"),
                                                            String::from("Osc1Detune"),
                                                            String::from("Osc2Detune"),
                                                            String::from("Osc3Detune"),
                                                            String::from("All_UniDetune"),
                                                            String::from("Osc1UniDetune"),
                                                            String::from("Osc2UniDetune"),
                                                            String::from("Osc3UniDetune"),
                                                        ],
                                                        "md4".to_string());
                                                        ui.add(md4);
                                                    });
                                                    ui.separator();
                                                });
                                            },
                                            LFOSelect::INFO => {
                                                ui.horizontal(|ui| {
                                                    ui.add(
                                                        nih_plug_egui::egui::TextEdit::singleline(&mut *params.preset_name_p.lock().unwrap())
                                                            .interactive(true)
                                                            .hint_text("Preset Name")
                                                            .desired_width(150.0));

                                                    ui.label(RichText::new("Category:")
                                                            .font(FONT)
                                                            .size(12.0));

                                                        let preset_category_box = ComboBoxParam::ParamComboBox::for_param(&params.preset_category, setter, vec![
                                                            String::from("Select"),
                                                            String::from("Atmosphere"),
                                                            String::from("Bass"),
                                                            String::from("Keys"),
                                                            String::from("Lead"),
                                                            String::from("Pad"),
                                                            String::from("Percussion"),
                                                            String::from("Pluck"),
                                                            String::from("Synth"),
                                                            String::from("Other"),
                                                        ],
                                                        "preset_category_box".to_string());
                                                        ui.add(preset_category_box);
                                                });

                                                ui.horizontal(|ui|{
                                                    ui.add(
                                                        egui::TextEdit::multiline(&mut *params.preset_info_p.lock().unwrap())
                                                            .interactive(true)
                                                            .hint_text("Preset Info")
                                                            .desired_width(150.0)
                                                            .desired_rows(6)
                                                            .lock_focus(true));

                                                    ui.vertical(|ui|{
                                                        ui.horizontal(|ui|{
                                                            let tag_acid = BoolButton::BoolButton::for_param(&params.tag_acid, setter, 2.0, 0.9, SMALLER_FONT);
                                                            ui.add(tag_acid);
                                                            let tag_analog = BoolButton::BoolButton::for_param(&params.tag_analog, setter, 2.0, 0.9, SMALLER_FONT);
                                                            ui.add(tag_analog);
                                                            let tag_bright = BoolButton::BoolButton::for_param(&params.tag_bright, setter, 2.0, 0.9, SMALLER_FONT);
                                                            ui.add(tag_bright);
                                                            let tag_chord = BoolButton::BoolButton::for_param(&params.tag_chord, setter, 2.0, 0.9, SMALLER_FONT);
                                                            ui.add(tag_chord);
                                                            let tag_crisp = BoolButton::BoolButton::for_param(&params.tag_crisp, setter, 2.0, 0.9, SMALLER_FONT);
                                                            ui.add(tag_crisp);
                                                        });
                                                        ui.horizontal(|ui|{
                                                            
                                                            let tag_deep = BoolButton::BoolButton::for_param(&params.tag_deep, setter, 2.0, 0.9, SMALLER_FONT);
                                                            ui.add(tag_deep);
                                                            let tag_delicate = BoolButton::BoolButton::for_param(&params.tag_delicate, setter, 2.7, 0.9, SMALLER_FONT);
                                                            ui.add(tag_delicate);
                                                            let tag_hard = BoolButton::BoolButton::for_param(&params.tag_hard, setter, 2.0, 0.9, SMALLER_FONT);
                                                            ui.add(tag_hard);
                                                            let tag_harsh = BoolButton::BoolButton::for_param(&params.tag_harsh, setter, 2.0, 0.9, SMALLER_FONT);
                                                            ui.add(tag_harsh);
                                                        });
                                                        ui.horizontal(|ui|{
                                                            let tag_lush = BoolButton::BoolButton::for_param(&params.tag_lush, setter, 1.8, 0.9, SMALLER_FONT);
                                                            ui.add(tag_lush);
                                                            let tag_mellow = BoolButton::BoolButton::for_param(&params.tag_mellow, setter, 2.0, 0.9, SMALLER_FONT);
                                                            ui.add(tag_mellow);
                                                            let tag_resonant = BoolButton::BoolButton::for_param(&params.tag_resonant, setter, 2.7, 0.9, SMALLER_FONT);
                                                            ui.add(tag_resonant);
                                                            let tag_rich = BoolButton::BoolButton::for_param(&params.tag_rich, setter, 2.0, 0.9, SMALLER_FONT);
                                                            ui.add(tag_rich);
                                                            let tag_sharp = BoolButton::BoolButton::for_param(&params.tag_sharp, setter, 1.8, 0.9, SMALLER_FONT);
                                                            ui.add(tag_sharp);
                                                        });
                                                        ui.horizontal(|ui|{
                                                            let tag_silky = BoolButton::BoolButton::for_param(&params.tag_silky, setter, 2.0, 0.9, SMALLER_FONT);
                                                            ui.add(tag_silky);
                                                            let tag_smooth = BoolButton::BoolButton::for_param(&params.tag_smooth, setter, 2.0, 0.9, SMALLER_FONT);
                                                            ui.add(tag_smooth);
                                                            let tag_soft = BoolButton::BoolButton::for_param(&params.tag_soft, setter, 2.0, 0.9, SMALLER_FONT);
                                                            ui.add(tag_soft);
                                                            let tag_stab = BoolButton::BoolButton::for_param(&params.tag_stab, setter, 2.0, 0.9, SMALLER_FONT);
                                                            ui.add(tag_stab);
                                                            let tag_warm = BoolButton::BoolButton::for_param(&params.tag_warm, setter, 2.0, 0.9, SMALLER_FONT);
                                                            ui.add(tag_warm);
                                                        });
                                                    });
                                                });
                                                ui.separator();
                                                ui.horizontal(|ui| {
                                                    let update_current_preset = BoolButton::BoolButton::for_param(&params.param_update_current_preset, setter, 5.2, 1.5, FONT)
                                                        .with_background_color(DARK_GREY_UI_COLOR);
                                                    ui.add(update_current_preset);
                                                    ui.separator();
                                                    let use_fx_toggle = BoolButton::BoolButton::for_param(&params.use_fx, setter, 2.8, 1.2, SMALLER_FONT);
                                                    ui.add(use_fx_toggle);
                                                });
                                            },
                                            LFOSelect::FX => {
                                                ScrollArea::vertical()
                                                    .auto_shrink([false; 2])
                                                    .max_height(200.0)
                                                    .max_width(400.0)
                                                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                                                    .show(ui, |ui|{
                                                        ui.set_min_width(400.0);
                                                        ui.vertical(|ui|{
                                                            // Equalizer
                                                            ui.horizontal(|ui|{
                                                                ui.vertical(|ui|{
                                                                    ui.label(RichText::new("EQ")
                                                                        .font(FONT)
                                                                    )
                                                                        .on_hover_text("An standard Biquad Equalizer implementation");
                                                                    let use_eq_toggle = toggle_switch::ToggleSwitch::for_param(&params.pre_use_eq, setter);
                                                                    ui.add(use_eq_toggle);
                                                                });
                                                                ui.vertical(|ui|{
                                                                    ui.add(
                                                                        VerticalParamSlider::for_param(&params.pre_low_gain, setter)
                                                                            .with_width(VERT_BAR_WIDTH * 2.5)
                                                                            .with_height(VERT_BAR_HEIGHT * 0.8)
                                                                            .set_reversed(true)
                                                                            .override_colors(
                                                                                DARK_GREY_UI_COLOR,
                                                                                TEAL_GREEN,
                                                                            ),
                                                                    );
                                                                    let low_freq_knob = ui_knob::ArcKnob::for_param(
                                                                        &params.pre_low_freq,
                                                                        setter,
                                                                        BKNOB_SIZE,
                                                                        KnobLayout::Vertical)
                                                                        .preset_style(ui_knob::KnobStyle::Preset1)
                                                                        .set_fill_color(DARK_GREY_UI_COLOR)
                                                                        .set_line_color(TEAL_GREEN)
                                                                        .set_text_size(BTEXT_SIZE)
                                                                        .override_text_color(Color32::DARK_GRAY);
                                                                    ui.add(low_freq_knob);
                                                                });
                                                                ui.vertical(|ui|{
                                                                    ui.add(
                                                                        VerticalParamSlider::for_param(&params.pre_mid_gain, setter)
                                                                            .with_width(VERT_BAR_WIDTH * 2.5)
                                                                            .with_height(VERT_BAR_HEIGHT * 0.8)
                                                                            .set_reversed(true)
                                                                            .override_colors(
                                                                                DARK_GREY_UI_COLOR,
                                                                                TEAL_GREEN,
                                                                            ),
                                                                    );
                                                                    let mid_freq_knob = ui_knob::ArcKnob::for_param(
                                                                        &params.pre_mid_freq,
                                                                        setter,
                                                                        BKNOB_SIZE,
                                                                        KnobLayout::Vertical)
                                                                        .preset_style(ui_knob::KnobStyle::Preset1)
                                                                        .set_fill_color(DARK_GREY_UI_COLOR)
                                                                        .set_line_color(TEAL_GREEN)
                                                                        .set_text_size(BTEXT_SIZE)
                                                                        .override_text_color(Color32::DARK_GRAY);
                                                                    ui.add(mid_freq_knob);
                                                                });
                                                                ui.vertical(|ui|{
                                                                    ui.add(
                                                                        VerticalParamSlider::for_param(&params.pre_high_gain, setter)
                                                                            .with_width(VERT_BAR_WIDTH * 2.5)
                                                                            .with_height(VERT_BAR_HEIGHT * 0.8)
                                                                            .set_reversed(true)
                                                                            .override_colors(
                                                                                DARK_GREY_UI_COLOR,
                                                                                TEAL_GREEN,
                                                                            ),
                                                                    );
                                                                    let high_freq_knob = ui_knob::ArcKnob::for_param(
                                                                        &params.pre_high_freq,
                                                                        setter,
                                                                        BKNOB_SIZE,
                                                                        KnobLayout::Vertical)
                                                                        .preset_style(ui_knob::KnobStyle::Preset1)
                                                                        .set_fill_color(DARK_GREY_UI_COLOR)
                                                                        .set_line_color(TEAL_GREEN)
                                                                        .set_text_size(BTEXT_SIZE)
                                                                        .override_text_color(Color32::DARK_GRAY);
                                                                    ui.add(high_freq_knob);
                                                                });
                                                                ui.colored_label(TEAL_GREEN, "This AREA is scrollable!");
                                                                ui.separator();
                                                            });
                                                            ui.separator();
                                                            // Compressor
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("Compressor")
                                                                    .font(FONT));
                                                                let use_comp_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_compressor, setter);
                                                                ui.add(use_comp_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.comp_amt, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.comp_atk, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.comp_rel, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.comp_drive, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                            });
                                                            ui.separator();
                                                            // ABass
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("ABass Algorithm")
                                                                    .font(FONT)).on_hover_text("Bass enhancement inspired by a plugin of renaissance that made waves");
                                                                let use_abass_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_abass, setter);
                                                                ui.add(use_abass_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.abass_amount, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                            });
                                                            ui.separator();
                                                            // Saturation
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("Saturation")
                                                                    .font(FONT));
                                                                let use_sat_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_saturation, setter);
                                                                ui.add(use_sat_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.sat_type, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.sat_amt, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                            });
                                                            ui.separator();
                                                            // Chorus
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("Chorus")
                                                                    .font(FONT));
                                                                let use_chorus_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_chorus, setter);
                                                                ui.add(use_chorus_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.chorus_amount, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.chorus_range, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.chorus_speed, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                            });
                                                            ui.separator();
                                                            // Phaser
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("Phaser")
                                                                    .font(FONT));
                                                                let use_phaser_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_phaser, setter);
                                                                ui.add(use_phaser_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.phaser_amount, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.phaser_depth, setter)
                                                                    .slimmer(0.7)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.phaser_rate, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                            });
                                                            ui.separator();
                                                            // Flanger
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("Flanger")
                                                                    .font(FONT));
                                                                let use_flanger_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_flanger, setter);
                                                                ui.add(use_flanger_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.flanger_amount, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.flanger_depth, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.flanger_rate, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.flanger_feedback, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                            });
                                                            ui.separator();
                                                            // Buffer Modulator
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("Buffer Modulator")
                                                                    .font(FONT)).on_hover_text("Weird buffer modulation based off a reverb that didn't work right");
                                                                let use_buffermod_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_buffermod, setter);
                                                                ui.add(use_buffermod_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.buffermod_amount, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.buffermod_depth, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.buffermod_rate, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.buffermod_spread, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.buffermod_timing, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                            });
                                                            ui.separator();
                                                            // Delay
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("Delay")
                                                                    .font(FONT));
                                                                let use_delay_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_delay, setter);
                                                                ui.add(use_delay_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.delay_amount, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.delay_time, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.delay_decay, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.delay_type, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                            });
                                                            ui.separator();
                                                            // Reverb
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("Reverb")
                                                                    .font(FONT)).on_hover_text("A tapped delay line reverb implementation");
                                                                let use_reverb_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_reverb, setter);
                                                                ui.add(use_reverb_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.reverb_model, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.reverb_amount, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.reverb_size, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.reverb_feedback, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                            });
                                                            ui.separator();
                                                            // Limiter
                                                            ui.horizontal(|ui|{
                                                                ui.label(RichText::new("Limiter")
                                                                    .font(FONT)).on_hover_text("A basic limiter with knee adjustment");
                                                                let use_limiter_toggle = toggle_switch::ToggleSwitch::for_param(&params.use_limiter, setter);
                                                                ui.add(use_limiter_toggle);
                                                            });
                                                            ui.vertical(|ui|{
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.limiter_threshold, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                                ui.add(CustomParamSlider::ParamSlider::for_param(&params.limiter_knee, setter)
                                                                    .set_left_sided_label(true)
                                                                    .set_label_width(84.0)
                                                                    .with_width(268.0));
                                                            });
                                                        });
                                                    }).inner;
                                            }
                                        }
                                    });
                                });
                            });
                    });
            },
            // This is the end of create_egui_editor()
        )
}
