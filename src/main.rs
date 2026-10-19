// Ukrywa okno konsoli w wersji --release na Windowsie
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

#[derive(Clone)]
struct LogEntry {
    timestamp: String,
    date_only: String,
    host: String,
    url: String,
    filename: String,
    release_prefix: String,
}

impl LogEntry {
    fn extract_release_name(filename: &str) -> String {
        let lower = filename.to_lowercase();
        if let Some(idx) = lower.find(".part") {
            filename[..idx].to_string()
        } else if let Some(idx) = lower.rfind('.') {
            filename[..idx].to_string()
        } else {
            filename.to_string()
        }
    }
}

struct LogFilterApp {
    file_path: Option<PathBuf>,
    entries: Vec<LogEntry>,
    available_dates: Vec<String>,
    available_releases: Vec<String>,
    available_hosts: Vec<String>,
    selected_date: String,
    selected_release: String,
    selected_host: String,
    search_query: String,
    wrap_code: bool,
    wrap_hide: bool,
    include_host_header: bool,
    strip_archive: bool,
    status_msg: String,
}

impl Default for LogFilterApp {
    fn default() -> Self {
        let mut app = Self {
            file_path: None,
            entries: Vec::new(),
            available_dates: Vec::new(),
            available_releases: Vec::new(),
            available_hosts: Vec::new(),
            selected_date: "Wszystkie".to_string(),
            selected_release: "Wszystkie".to_string(),
            selected_host: "Wszystkie".to_string(),
            search_query: String::new(),
            wrap_code: true,
            wrap_hide: false,
            include_host_header: true,
            strip_archive: true,
            status_msg: "Wybierz plik FileUploader.log lub umieść go w folderze z programem."
                .to_string(),
        };

        let default_path = PathBuf::from("FileUploader.log");
        if default_path.exists() {
            app.load_file(default_path);
        }

        app
    }
}

impl LogFilterApp {
    // Zoptymalizowane, szybkie wczytywanie pliku logów
    fn load_file(&mut self, path: PathBuf) {
        match fs::read_to_string(&path) {
            Ok(content) => {
                let estimated_lines = content.bytes().filter(|&b| b == b'\n').count();
                let mut entries = Vec::with_capacity(estimated_lines);

                for line in content.lines() {
                    let line_trimmed = line.trim();
                    if line_trimmed.is_empty() {
                        continue;
                    }

                    let mut parts = line_trimmed.split('|');
                    let timestamp_opt = parts.next();
                    let host_opt = parts.next();
                    let url_opt = parts.next();
                    let _empty_opt = parts.next();
                    let filename_opt = parts.next();

                    if let (Some(ts), Some(h), Some(u), Some(f)) = (timestamp_opt, host_opt, url_opt, filename_opt) {
                        let timestamp = ts.trim().to_string();
                        let date_only = timestamp
                            .split_whitespace()
                            .next()
                            .unwrap_or("")
                            .to_string();
                        let filename = f.trim().to_string();
                        let release_prefix = LogEntry::extract_release_name(&filename);

                        entries.push(LogEntry {
                            timestamp,
                            date_only,
                            host: h.trim().to_string(),
                            url: u.trim().to_string(),
                            filename,
                            release_prefix,
                        });
                    }
                }

                self.entries = entries;

                let dates_set: BTreeSet<String> = self
                    .entries
                    .iter()
                    .map(|e| e.date_only.clone())
                    .filter(|d| !d.is_empty())
                    .collect();
                self.available_dates = dates_set.into_iter().rev().collect();

                let releases_set: BTreeSet<String> = self
                    .entries
                    .iter()
                    .map(|e| e.release_prefix.clone())
                    .filter(|r| !r.is_empty())
                    .collect();
                self.available_releases = releases_set.into_iter().collect();

                let hosts_set: BTreeSet<String> = self
                    .entries
                    .iter()
                    .map(|e| e.host.clone())
                    .filter(|h| !h.is_empty())
                    .collect();
                self.available_hosts = hosts_set.into_iter().collect();

                self.status_msg = format!(
                    "Wczytano błyskawicznie {} linków ({} dni, {} wstawek, {} serwisów)",
                    self.entries.len(),
                    self.available_dates.len(),
                    self.available_releases.len(),
                    self.available_hosts.len()
                );
                self.file_path = Some(path);
            }
            Err(e) => {
                self.status_msg = format!("Błąd odczytu pliku: {}", e);
            }
        }
    }

    fn clean_url<'a>(&self, url: &'a str) -> &'a str {
        if self.strip_archive {
            url.strip_suffix("(archive)").unwrap_or(url)
        } else {
            url
        }
    }

    fn get_grouped_links(&self) -> BTreeMap<String, Vec<LogEntry>> {
        let query = self.search_query.to_lowercase();
        let mut grouped: BTreeMap<String, Vec<LogEntry>> = BTreeMap::new();

        for entry in &self.entries {
            if self.selected_date != "Wszystkie" && entry.date_only != self.selected_date {
                continue;
            }

            if self.selected_release != "Wszystkie" && entry.release_prefix != self.selected_release
            {
                continue;
            }

            if self.selected_host != "Wszystkie" && entry.host != self.selected_host {
                continue;
            }

            if !query.is_empty() {
                let match_file = entry.filename.to_lowercase().contains(&query);
                let match_date = entry.timestamp.to_lowercase().contains(&query);
                let match_host = entry.host.to_lowercase().contains(&query);

                if !match_file && !match_date && !match_host {
                    continue;
                }
            }

            grouped
                .entry(entry.host.clone())
                .or_default()
                .push(entry.clone());
        }

        for list in grouped.values_mut() {
            list.sort_by(|a, b| a.filename.cmp(&b.filename));
            list.dedup_by(|a, b| a.url == b.url);
        }

        grouped
    }

    fn format_host_block(&self, host: &str, list: &[LogEntry], force_header: bool) -> String {
        let urls: Vec<&str> = list.iter().map(|e| self.clean_url(&e.url)).collect();
        let mut block = urls.join("\n");

        if self.wrap_code {
            block = format!("[code]\n{}\n[/code]", block);
        }
        if self.wrap_hide {
            block = format!("[hide]\n{}\n[/hide]", block);
        }
        if force_header {
            block = format!("[b]{}[/b]\n{}", host.to_uppercase(), block);
        }

        block
    }
}

impl eframe::App for LogFilterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- ZWIĘKSZENIE OSTROŚCI I SKALI TEKSTU ---
        // Jeśli tekst wydaje się rozmyty, możemy wymusić nieco większy współczynnik (np. 1.1 lub 1.2)
        // lub zdać się na automatyczne DPI systemowe:
        ctx.set_pixels_per_point(ctx.pixels_per_point().max(1.1));
        // -------------------------------------------

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Filtr logów - Zoom FileUploader");
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                if ui.button("📂 Wybierz plik...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Log files", &["log", "txt"])
                        .pick_file()
                    {
                        self.load_file(path);
                    }
                }

                if let Some(ref path) = self.file_path {
                    if ui.button("🔄 Odśwież").clicked() {
                        let p = path.clone();
                        self.load_file(p);
                    }
                }

                ui.label(&self.status_msg);
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("📅 Data:");
                egui::ComboBox::from_id_salt("date_combo")
                    .selected_text(&self.selected_date)
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.selected_date,
                            "Wszystkie".to_string(),
                            "Wszystkie",
                        );
                        for date in &self.available_dates {
                            ui.selectable_value(&mut self.selected_date, date.clone(), date);
                        }
                    });

                ui.add_space(6.0);

                ui.label("🌐 Serwis:");
                egui::ComboBox::from_id_salt("host_combo")
                    .selected_text(&self.selected_host)
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.selected_host,
                            "Wszystkie".to_string(),
                            "Wszystkie",
                        );
                        for host in &self.available_hosts {
                            ui.selectable_value(&mut self.selected_host, host.clone(), host);
                        }
                    });

                ui.add_space(6.0);

                ui.label("📦 Wstawka:");
                egui::ComboBox::from_id_salt("release_combo")
                    .selected_text(&self.selected_release)
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.selected_release,
                            "Wszystkie".to_string(),
                            "Wszystkie",
                        );
                        for rel in &self.available_releases {
                            ui.selectable_value(&mut self.selected_release, rel.clone(), rel);
                        }
                    });

                ui.add_space(6.0);

                ui.label("🔍 Szukaj:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("tekst...")
                        .desired_width(110.0),
                );

                if (self.selected_date != "Wszystkie"
                    || self.selected_release != "Wszystkie"
                    || self.selected_host != "Wszystkie"
                    || !self.search_query.is_empty())
                    && ui.button("✖ Reset").clicked()
                {
                    self.selected_date = "Wszystkie".to_string();
                    self.selected_release = "Wszystkie".to_string();
                    self.selected_host = "Wszystkie".to_string();
                    self.search_query.clear();
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.strong("Opcje formatowania:");
                ui.checkbox(&mut self.strip_archive, "Usuń \"(archive)\"");
                ui.checkbox(&mut self.wrap_code, "Dodaj [code]");
                ui.checkbox(&mut self.wrap_hide, "Dodaj [hide]");
                ui.checkbox(&mut self.include_host_header, "Nagłówek [b]SERWIS[/b]");
            });

            ui.separator();

            let grouped = self.get_grouped_links();

            if grouped.is_empty() {
                ui.label("Brak linków spełniających wybrane kryteria.");
                return;
            }

            ui.horizontal(|ui| {
                if ui
                    .button("📋 Kopiuj WSZYSTKIE widoczne grupy do schowka")
                    .clicked()
                {
                    let mut blocks = Vec::new();
                    for (host, list) in &grouped {
                        blocks.push(self.format_host_block(host, list, self.include_host_header));
                    }
                    ctx.copy_text(blocks.join("\n\n"));
                    self.status_msg = "Skopiowano wszystkie widoczne grupy!".to_string();
                }
            });

            ui.add_space(5.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (host, list) in grouped {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.strong(format!(
                                "{} (Liczba linków: {})",
                                host.to_uppercase(),
                                list.len()
                            ));

                            if ui.button("📋 Kopiuj linki").clicked() {
                                let formatted = self.format_host_block(&host, &list, false);
                                ctx.copy_text(formatted);
                                self.status_msg = format!("Skopiowano z {}!", host.to_uppercase());
                            }

                            if ui.button("📋 Kopiuj z nagłówkiem").clicked() {
                                let formatted = self.format_host_block(&host, &list, true);
                                ctx.copy_text(formatted);
                                self.status_msg = format!(
                                    "Skopiowano z {} (razem z nagłówkiem)!",
                                    host.to_uppercase()
                                );
                            }
                        });

                        let mut urls_text: String = list
                            .iter()
                            .map(|e| self.clean_url(&e.url))
                            .collect::<Vec<&str>>()
                            .join("\n");

                        ui.add(
                            egui::TextEdit::multiline(&mut urls_text)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .desired_rows(list.len().clamp(2, 8)),
                        );
                    });
                    ui.add_space(5.0);
                }
            });
        });
    }
}

// Wkompilowuje ikonę bezpośrednio do pliku .exe, eliminując problem z brakującym plikiem w release
fn load_icon() -> egui::IconData {
    let icon_bytes = include_bytes!("../icon.png");
    
    match image::load_from_memory(icon_bytes) {
        Ok(image) => {
            let image = image.into_rgba8();
            let width = image.width();
            let height = image.height();
            let rgba = image.into_raw();
            egui::IconData {
                rgba,
                width,
                height,
            }
        }
        Err(_) => {
            egui::IconData {
                rgba: vec![0, 0, 0, 0],
                width: 1,
                height: 1,
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([980.0, 700.0])
            .with_title("Zoom FileUploader - Filtr Linków")
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Zoom FileUploader - Filtr Linków",
        options,
        Box::new(|_cc| Ok(Box::new(LogFilterApp::default()))),
    )
}