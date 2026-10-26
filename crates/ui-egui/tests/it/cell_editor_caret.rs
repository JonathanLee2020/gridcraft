//! The caret of the in-cell editor stays visible on the cell's fill, whatever the theme.
use egui::{Color32, Context, RawInput, Rect, Shape, pos2, vec2};
use gridcraft_engine::Session;
use gridcraft_ui_egui::{SheetApp, theme};
use serde_json::json;

fn frame(ctx: &Context, app: &mut SheetApp) -> egui::FullOutput {
    let mut output =
        ctx.run_ui(RawInput { screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1200.0, 800.0))), ..Default::default() }, |ui| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        });
    output.textures_delta.clear();
    output
}

/// The caret painted while editing A1 over `fill` (`None`: no fill), with the app in `dark` mode.
fn caret(fill: Option<&str>, dark: bool) -> Color32 {
    let ctx = Context::default();
    theme::apply(&ctx, dark);
    let mut session = Session::new();
    session.new_workbook();
    let mut app = crate::common::app(session);
    app.ui.dark = dark;
    if let Some(color) = fill {
        app.run("home.fillColor", json!({"range": "A1", "color": color})).unwrap();
    }
    for _ in 0..4 {
        frame(&ctx, &mut app);
    }
    app.begin_edit(Some("abc".into()), false);
    for _ in 0..4 {
        frame(&ctx, &mut app);
    }
    let output = frame(&ctx, &mut app);
    // The editor paints its backdrop first; the caret is the vertical segment after it.
    let paper = fill.map_or(if dark { Color32::BLACK } else { Color32::WHITE }, |c| Color32::from_hex(c).unwrap());
    let backdrop = output
        .shapes
        .iter()
        .rposition(|s| matches!(&s.shape, Shape::Rect(r) if r.fill == paper && r.rect.width() > 20.0))
        .expect("editor backdrop painted");
    let text = output.shapes[backdrop..]
        .iter()
        .find_map(|s| match &s.shape {
            Shape::Text(t) if t.galley.job.text == "abc" => Some(t),
            _ => None,
        })
        .expect("editor text painted");
    let ink = if paper == Color32::BLACK || fill == Some("#1F3864") { Color32::WHITE } else { Color32::BLACK };
    assert!(text.galley.job.sections.iter().all(|s| s.format.color == ink), "editor text contrasts with fill");
    output.shapes[backdrop..]
        .iter()
        .find_map(|s| match &s.shape {
            Shape::LineSegment { points, stroke } if points[0].x == points[1].x => Some(stroke.color),
            _ => None,
        })
        .expect("caret painted")
}

#[test]
fn caret_contrasts_with_the_cell_fill() {
    assert_eq!(caret(Some("#1F3864"), false), Color32::WHITE, "dark fill, light theme");
    assert_eq!(caret(Some("#1F3864"), true), Color32::WHITE, "dark fill, dark theme");
    assert_eq!(caret(Some("#FFFF00"), true), Color32::BLACK, "light fill, dark theme");
    assert_eq!(caret(None, true), Color32::WHITE, "no fill, dark theme");
    assert_eq!(caret(None, false), Color32::BLACK, "no fill, light theme");
}

#[test]
fn ordinary_and_merged_cells_repaint_when_the_theme_changes() {
    let ctx = Context::default();
    let mut session = Session::new();
    session.new_workbook();
    session.execute("cell.set", json!({"cell": "B2", "input": "Ordinary"})).unwrap();
    session.execute("cell.set", json!({"cell": "D2", "input": "Merged"})).unwrap();
    session.execute("home.mergeCells", json!({"range": "D2:E2"})).unwrap();
    let mut app = crate::common::app(session);
    for dark in [false, true, false] {
        theme::apply(&ctx, dark);
        app.ui.dark = dark;
        for _ in 0..4 {
            frame(&ctx, &mut app);
        }
        let output = frame(&ctx, &mut app);
        let (paper, ink) = if dark { (Color32::BLACK, Color32::WHITE) } else { (Color32::WHITE, Color32::BLACK) };
        for label in ["Ordinary", "Merged"] {
            let text = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    Shape::Text(t) if t.galley.job.text == label => Some(t),
                    _ => None,
                })
                .expect("cell text painted");
            assert!(text.galley.job.sections.iter().all(|s| s.format.color == ink), "{label}, dark={dark}");
            if label == "Merged" {
                assert!(
                    output.shapes.iter().any(|s| matches!(&s.shape, Shape::Rect(r)
                    if r.fill == paper && r.rect.contains(text.pos) && r.rect.width() < 400.0 && r.rect.height() < 50.0)),
                    "merged cell backdrop, dark={dark}"
                );
            }
        }
    }
}
