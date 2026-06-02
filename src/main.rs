#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, Color32, TextureHandle, TextureOptions};
use pdfium_render::prelude::*;
use rfd::FileDialog;
use std::path::PathBuf;

// ─── Colours ──────────────────────────────────────────────────────────────────

const BG: Color32          = Color32::from_rgb(18, 18, 28);
const TOOLBAR_BG: Color32  = Color32::from_rgb(26, 26, 40);
const ACCENT: Color32      = Color32::from_rgb(66, 135, 245);
const TEXT_DIM: Color32    = Color32::from_rgb(140, 140, 180);
const TEXT_FAINT: Color32  = Color32::from_rgb(70, 70, 100);

// ─── Entry point ──────────────────────────────────────────────────────────────

fn main() -> eframe::Result<()> {
    // Try to locate pdfium.dll next to the exe, then fall back to system path.
    let binding = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
        .or_else(|_| Pdfium::bind_to_system_library());

    let pdfium = match binding {
        Ok(b) => Pdfium::new(b),
        Err(_) => {
            rfd::MessageDialog::new()
                .set_title("PDF Viewer — missing library")
                .set_description(
                    "pdfium.dll was not found.\n\n\
                     Run  get_pdfium.ps1  (included) to download it,\n\
                     then place it next to pdf-viewer.exe.",
                )
                .set_level(rfd::MessageLevel::Error)
                .show();
            return Ok(());
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PDF Viewer")
            .with_inner_size([1100.0, 800.0])
            .with_min_inner_size([550.0, 400.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "PDF Viewer",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(dark_visuals());
            Box::new(PdfViewerApp::new(pdfium))
        }),
    )
}

fn dark_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();
    v.panel_fill        = BG;
    v.window_fill       = BG;
    v.extreme_bg_color  = Color32::from_rgb(12, 12, 20);
    v.override_text_color = Some(Color32::from_rgb(210, 210, 230));
    v.selection.bg_fill = Color32::from_rgb(60, 100, 200);
    v.window_rounding   = egui::Rounding::same(8.0);
    v
}

// ─── App state ────────────────────────────────────────────────────────────────

struct PdfViewerApp {
    pdfium: Pdfium,

    pdf_bytes:  Option<Vec<u8>>,
    pdf_path:   Option<PathBuf>,
    page_count: usize,
    current_page: usize,
    zoom: f32,

    // Cached rendered page
    page_texture:  Option<TextureHandle>,
    rendered_page: usize,
    rendered_zoom: f32,
    needs_render:  bool,

    error: Option<String>,
}

impl PdfViewerApp {
    fn new(pdfium: Pdfium) -> Self {
        let mut app = Self {
            pdfium,
            pdf_bytes:    None,
            pdf_path:     None,
            page_count:   0,
            current_page: 0,
            zoom:         1.0,
            page_texture:  None,
            rendered_page: usize::MAX,
            rendered_zoom: -1.0,
            needs_render:  false,
            error:         None,
        };

        // Support "Open with…" / dropping a path on the exe
        if let Some(arg) = std::env::args().nth(1) {
            let p = PathBuf::from(arg);
            if p.extension().map_or(false, |e| e.eq_ignore_ascii_case("pdf")) {
                app.load_pdf(p);
            }
        }

        app
    }

    // ── File loading ────────────────────────────────────────────────────────

    fn open_dialog(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("PDF files", &["pdf", "PDF"])
            .pick_file()
        {
            self.load_pdf(path);
        }
    }

    fn load_pdf(&mut self, path: PathBuf) {
        self.error = None;

        let bytes = match std::fs::read(&path) {
            Ok(b)  => b,
            Err(e) => { self.error = Some(format!("Cannot read file: {e}")); return; }
        };

        // Validate and count pages in its own block so the borrow on `bytes` ends
        // before we move it into self.pdf_bytes.
        let page_count = {
            let doc = match self.pdfium.load_pdf_from_byte_slice(&bytes, None) {
                Ok(d)  => d,
                Err(e) => { self.error = Some(format!("Invalid PDF: {e}")); return; }
            };
            doc.pages().len() as usize
        };

        self.page_count   = page_count;
        self.pdf_bytes    = Some(bytes);
        self.pdf_path     = Some(path);
        self.current_page = 0;
        self.zoom         = 1.0;
        self.page_texture = None;
        self.needs_render = true;
    }

    // ── Zoom / navigation ───────────────────────────────────────────────────

    fn set_zoom(&mut self, z: f32) {
        let z = z.clamp(0.1, 8.0);
        if (z - self.zoom).abs() > 0.001 {
            self.zoom         = z;
            self.needs_render = true;
        }
    }

    fn go_to_page(&mut self, p: usize) {
        let p = p.min(self.page_count.saturating_sub(1));
        if p != self.current_page {
            self.current_page = p;
            self.needs_render = true;
        }
    }

    // ── Rendering ───────────────────────────────────────────────────────────

    fn render_if_needed(&mut self, ctx: &egui::Context) {
        if !self.needs_render
            && self.rendered_page == self.current_page
            && (self.rendered_zoom - self.zoom).abs() < 0.001
        {
            return;
        }

        let Some(bytes) = self.pdf_bytes.as_deref() else { return };

        match render_page(&self.pdfium, bytes, self.current_page, self.zoom) {
            Ok((pixels, w, h)) => {
                let texture = ctx.load_texture(
                    "pdf_page",
                    egui::ColorImage { size: [w, h], pixels },
                    TextureOptions::LINEAR,
                );
                self.page_texture  = Some(texture);
                self.rendered_page = self.current_page;
                self.rendered_zoom = self.zoom;
                self.needs_render  = false;
            }
            Err(e) => self.error = Some(e),
        }
    }
}

/// Render a single PDF page to an egui-compatible pixel buffer.
fn render_page(
    pdfium: &Pdfium,
    bytes: &[u8],
    page_idx: usize,
    zoom: f32,
) -> Result<(Vec<Color32>, usize, usize), String> {
    let doc  = pdfium.load_pdf_from_byte_slice(bytes, None).map_err(|e| e.to_string())?;
    let page = doc.pages().get(page_idx as u16).map_err(|e| e.to_string())?;

    // PDF points are 72 dpi; screen is 96 dpi.
    let dpi = 96.0 * zoom;
    let w   = ((page.width().value  * dpi / 72.0) as i32).max(1);
    let h   = ((page.height().value * dpi / 72.0) as i32).max(1);

    let bitmap = page
        .render_with_config(&PdfRenderConfig::new().set_target_width(w).set_target_height(h))
        .map_err(|e| e.to_string())?;

    let rgba  = bitmap.as_image().into_rgba8();
    let width = rgba.width() as usize;
    let height= rgba.height() as usize;
    let pixels: Vec<Color32> = rgba
        .pixels()
        .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();

    Ok((pixels, width, height))
}

// ─── eframe::App ──────────────────────────────────────────────────────────────

impl eframe::App for PdfViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render_if_needed(ctx);
        self.handle_input(ctx);
        self.draw_toolbar(ctx);
        self.draw_content(ctx);
    }
}

impl PdfViewerApp {
    fn handle_input(&mut self, ctx: &egui::Context) {
        // Collect what we need from the input closure first (borrows resolved).
        let (want_open, go_prev, go_next, zoom_in, zoom_out, zoom_reset, dropped) =
            ctx.input(|i| {
                let want_open  = i.modifiers.ctrl && i.key_pressed(egui::Key::O);
                let go_prev    = i.key_pressed(egui::Key::ArrowLeft)
                              || i.key_pressed(egui::Key::PageUp);
                let go_next    = i.key_pressed(egui::Key::ArrowRight)
                              || i.key_pressed(egui::Key::PageDown);
                let zoom_in    = i.modifiers.ctrl
                              && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals));
                let zoom_out   = i.modifiers.ctrl && i.key_pressed(egui::Key::Minus);
                let zoom_reset = i.modifiers.ctrl && i.key_pressed(egui::Key::Num0);

                let dropped: Vec<PathBuf> = i
                    .raw
                    .dropped_files
                    .iter()
                    .filter_map(|f| f.path.clone())
                    .filter(|p| p.extension().map_or(false, |e| e.eq_ignore_ascii_case("pdf")))
                    .collect();

                (want_open, go_prev, go_next, zoom_in, zoom_out, zoom_reset, dropped)
            });

        if want_open  { self.open_dialog(); }
        if go_prev && self.current_page > 0 { self.go_to_page(self.current_page - 1); }
        if go_next    { self.go_to_page(self.current_page + 1); }
        if zoom_in    { self.set_zoom(self.zoom * 1.25); }
        if zoom_out   { self.set_zoom(self.zoom / 1.25); }
        if zoom_reset { self.set_zoom(1.0); }
        if let Some(path) = dropped.into_iter().next() { self.load_pdf(path); }
    }

    // ── Toolbar ─────────────────────────────────────────────────────────────

    fn draw_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar")
            .frame(
                egui::Frame::none()
                    .fill(TOOLBAR_BG)
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().button_padding = egui::vec2(10.0, 6.0);
                    ui.spacing_mut().item_spacing.x = 6.0;

                    // ── Open ──────────────────────────────────────────────
                    let open = egui::Button::new(
                        egui::RichText::new("⊕  Open PDF").color(Color32::WHITE),
                    )
                    .fill(ACCENT)
                    .rounding(egui::Rounding::same(6.0));
                    if ui.add(open).on_hover_text("Ctrl+O").clicked() {
                        self.open_dialog();
                    }

                    if self.pdf_bytes.is_some() {
                        ui.separator();

                        // ── Page navigation ───────────────────────────────
                        ui.add_enabled_ui(self.current_page > 0, |ui| {
                            if ui
                                .button(egui::RichText::new("◀").size(13.0))
                                .on_hover_text("Previous page  (←)")
                                .clicked()
                            {
                                self.go_to_page(self.current_page - 1);
                            }
                        });

                        ui.label(
                            egui::RichText::new(format!(
                                "{}  /  {}",
                                self.current_page + 1,
                                self.page_count
                            ))
                            .color(Color32::from_rgb(200, 200, 225))
                            .size(13.0),
                        );

                        ui.add_enabled_ui(self.current_page + 1 < self.page_count, |ui| {
                            if ui
                                .button(egui::RichText::new("▶").size(13.0))
                                .on_hover_text("Next page  (→)")
                                .clicked()
                            {
                                self.go_to_page(self.current_page + 1);
                            }
                        });

                        ui.separator();

                        // ── Zoom ──────────────────────────────────────────
                        if ui
                            .button(egui::RichText::new("−").size(15.0))
                            .on_hover_text("Zoom out  (Ctrl+−)")
                            .clicked()
                        {
                            self.set_zoom(self.zoom / 1.25);
                        }

                        if ui
                            .button(
                                egui::RichText::new(format!("{:.0}%", self.zoom * 100.0))
                                    .monospace()
                                    .size(12.0),
                            )
                            .on_hover_text("Reset zoom  (Ctrl+0)")
                            .clicked()
                        {
                            self.set_zoom(1.0);
                        }

                        if ui
                            .button(egui::RichText::new("+").size(15.0))
                            .on_hover_text("Zoom in  (Ctrl++)")
                            .clicked()
                        {
                            self.set_zoom(self.zoom * 1.25);
                        }

                        // ── Filename (right-aligned) ───────────────────────
                        if let Some(path) = &self.pdf_path {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if let Some(name) = path.file_name() {
                                        ui.label(
                                            egui::RichText::new(
                                                name.to_string_lossy().as_ref(),
                                            )
                                            .color(TEXT_DIM)
                                            .italics()
                                            .size(12.0),
                                        );
                                    }
                                },
                            );
                        }
                    }
                });
            });
    }

    // ── Main content area ───────────────────────────────────────────────────

    fn draw_content(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG))
            .show(ctx, |ui| {
                if let Some(err) = self.error.clone() {
                    self.show_error(ui, &err);
                } else if self.pdf_bytes.is_none() {
                    self.show_empty_state(ui);
                } else if let Some(texture) = self.page_texture.clone() {
                    self.show_page(ui, &texture);
                }
            });
    }

    fn show_error(&self, ui: &mut egui::Ui, message: &str) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.28);
            ui.label(egui::RichText::new("⚠").size(44.0).color(Color32::from_rgb(220, 80, 80)));
            ui.add_space(12.0);
            ui.label(
                egui::RichText::new(message)
                    .size(14.0)
                    .color(Color32::from_rgb(200, 100, 100)),
            );
        });
    }

    fn show_empty_state(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.28);
            ui.label(egui::RichText::new("📄").size(72.0).color(Color32::from_rgb(50, 50, 80)));
            ui.add_space(18.0);
            ui.label(
                egui::RichText::new("Open a PDF to get started")
                    .size(18.0)
                    .color(TEXT_DIM),
            );
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Click  ⊕ Open PDF  ·  Ctrl+O  ·  or drag and drop")
                    .size(12.0)
                    .color(TEXT_FAINT),
            );
        });
    }

    fn show_page(&self, ui: &mut egui::Ui, texture: &TextureHandle) {
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let tex_size   = texture.size_vec2();
                let avail_w    = ui.available_width();
                let h_pad      = ((avail_w - tex_size.x) / 2.0).max(24.0);
                let v_pad      = 24.0;
                let total_w    = (h_pad * 2.0 + tex_size.x).max(avail_w);
                let total_h    = v_pad * 2.0 + tex_size.y;

                let (area_rect, _) = ui.allocate_exact_size(
                    egui::vec2(total_w, total_h),
                    egui::Sense::hover(),
                );

                let page_origin = egui::pos2(
                    area_rect.left() + ((area_rect.width() - tex_size.x) / 2.0).max(24.0),
                    area_rect.top() + v_pad,
                );
                let page_rect = egui::Rect::from_min_size(page_origin, tex_size);

                // Soft drop-shadow
                for i in 0..6u8 {
                    let spread = i as f32 * 2.5;
                    let alpha  = 55u8.saturating_sub(i * 9);
                    ui.painter().rect_filled(
                        page_rect
                            .translate(egui::vec2(4.0 + spread * 0.2, 6.0 + spread * 0.2))
                            .expand(spread),
                        4.0,
                        Color32::from_rgba_unmultiplied(0, 0, 0, alpha),
                    );
                }

                // PDF page
                ui.painter().image(
                    texture.id(),
                    page_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            });
    }
}
