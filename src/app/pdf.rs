use std::{
    collections::HashMap,
    io::Cursor,
    sync::{Arc, Mutex, OnceLock},
};

use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use printpdf::{
    Actions, BorderArray, Color, ColorArray, Line, LinePoint, LinkAnnotation, Mm, Op, ParsedFont,
    PdfDocument, PdfFontHandle, PdfPage, PdfSaveOptions, Point, Pt, RawImage, Rect, Rgb, TextItem,
    XObjectId, XObjectTransform,
};
use resvg::{
    tiny_skia::{Pixmap, Transform},
    usvg,
};
use skrifa::{FontRef, MetadataProvider, instance::LocationRef, instance::Size};

use crate::error::{AppError, AppResult};

const PAGE_W: f32 = 595.28;
const PAGE_H: f32 = 841.89;
const MARGIN_X: f32 = 22.0;
const MARGIN_Y: f32 = 40.0;
const FOOTER: f32 = 32.0;
const PHOTO_PX: u32 = 320;
const PHOTO_PT: f32 = 90.0;
const SKILL_ICON_PX: u32 = 128;
const SKILL_ICON_PT: f32 = 28.0;
const BLOCK_INDENT: f32 = 15.0;

const TEXT: u8 = 0x44;
const MUTED: u8 = 0x80;
const RULE: u8 = 0xBB;

const DISPLAY_BOLD_WOFF2: &[u8] =
    include_bytes!("../../assets/fonts/red-hat-display/red-hat-display-700.woff2");
const DISPLAY_REGULAR_WOFF2: &[u8] =
    include_bytes!("../../assets/fonts/red-hat-display/red-hat-display-400.woff2");
const TEXT_BOLD_WOFF2: &[u8] =
    include_bytes!("../../assets/fonts/red-hat-text/RedHatText-Bold.woff2");
const TEXT_REGULAR_WOFF2: &[u8] =
    include_bytes!("../../assets/fonts/red-hat-text/RedHatText-Regular.woff2");
const TEXT_LIGHT_WOFF2: &[u8] =
    include_bytes!("../../assets/fonts/red-hat-text/RedHatText-Light.woff2");

static DISPLAY_BOLD: OnceLock<Vec<u8>> = OnceLock::new();
static DISPLAY_REGULAR: OnceLock<Vec<u8>> = OnceLock::new();
static TEXT_BOLD: OnceLock<Vec<u8>> = OnceLock::new();
static TEXT_REGULAR: OnceLock<Vec<u8>> = OnceLock::new();
static TEXT_LIGHT: OnceLock<Vec<u8>> = OnceLock::new();

#[derive(Clone, Copy)]
enum FontRole {
    DisplayBold,
    DisplayRegular,
    TextBold,
    TextRegular,
    TextLight,
}

pub(crate) struct CvPdfContent {
    pub title: String,
    pub first_name: String,
    pub last_name: String,
    pub role: String,
    pub photo: Option<Vec<u8>>,
    pub facts: Vec<PdfFact>,
    pub sections: Vec<PdfSection>,
    pub legal_notice: String,
}

pub(crate) struct PdfFact {
    pub label: String,
    pub value: String,
    pub href: Option<String>,
}

pub(crate) struct PdfSection {
    pub heading: String,
    pub blocks: Vec<PdfBlock>,
}

pub(crate) enum PdfBlock {
    Paragraph(String),
    Group(PdfGroup),
    Entry(PdfEntry),
    Skills(Vec<PdfSkill>),
}

pub(crate) struct PdfSkill {
    pub icon: String,
}

pub(crate) struct PdfGroup {
    pub title: String,
    pub subtitle: String,
    pub entries: Vec<PdfEntry>,
}

pub(crate) struct PdfEntry {
    pub title: String,
    pub subtitle: String,
    pub meta: String,
    pub body: String,
    pub notes: Vec<String>,
}

struct FontBytes {
    display_bold: &'static [u8],
    display_regular: &'static [u8],
    text_bold: &'static [u8],
    text_regular: &'static [u8],
    text_light: &'static [u8],
}

struct Faces {
    display_bold: FontRef<'static>,
    display_regular: FontRef<'static>,
    text_bold: FontRef<'static>,
    text_regular: FontRef<'static>,
    text_light: FontRef<'static>,
}

struct FontIds {
    display_bold: printpdf::FontId,
    display_regular: printpdf::FontId,
    text_bold: printpdf::FontId,
    text_regular: printpdf::FontId,
    text_light: printpdf::FontId,
}

#[derive(Clone, Copy)]
struct TextStyle<'a> {
    fonts: &'a FontIds,
    faces: &'a Faces,
    role: FontRole,
    size: f32,
    color: u8,
}

struct Column {
    pages: Vec<Vec<Op>>,
    page: usize,
    x: f32,
    width: f32,
    y: f32,
    column_top: f32,
    indent: f32,
    skill_icons: Vec<Option<XObjectId>>,
    skill_icon_at: usize,
}

pub(crate) fn render(content: &CvPdfContent) -> AppResult<Vec<u8>> {
    let bytes = font_bytes()?;
    let faces = Faces {
        display_bold: parse_face(bytes.display_bold, "Red Hat Display Bold")?,
        display_regular: parse_face(bytes.display_regular, "Red Hat Display")?,
        text_bold: parse_face(bytes.text_bold, "Red Hat Text Bold")?,
        text_regular: parse_face(bytes.text_regular, "Red Hat Text")?,
        text_light: parse_face(bytes.text_light, "Red Hat Text Light")?,
    };

    let mut warnings = Vec::new();
    let mut document = PdfDocument::new(&content.title);
    let fonts = FontIds {
        display_bold: add_font(&mut document, bytes.display_bold, "Red Hat Display Bold")?,
        display_regular: add_font(&mut document, bytes.display_regular, "Red Hat Display")?,
        text_bold: add_font(&mut document, bytes.text_bold, "Red Hat Text Bold")?,
        text_regular: add_font(&mut document, bytes.text_regular, "Red Hat Text")?,
        text_light: add_font(&mut document, bytes.text_light, "Red Hat Text Light")?,
    };

    let photo = content.photo.as_deref().and_then(circular_portrait);
    let photo_id = photo.as_ref().map(|image| document.add_image(image));
    let skill_icons = skill_icon_ids(&mut document, &content.sections);

    let mut pages = vec![Vec::new()];
    let header_bottom = draw_header(&mut pages[0], content, &fonts, &faces, photo_id);

    let mut column = Column {
        pages,
        page: 0,
        x: MARGIN_X,
        width: PAGE_W - (MARGIN_X * 2.0),
        y: header_bottom,
        column_top: header_bottom,
        indent: 0.0,
        skill_icons,
        skill_icon_at: 0,
    };

    for section in &content.sections {
        column.draw_section(section, &fonts, &faces);
    }
    column.draw_legal(&content.legal_notice, &fonts, &faces);
    column.draw_page_numbers(&fonts, &faces);

    let pdf_pages =
        column.pages.into_iter().map(|ops| PdfPage::new(Mm(210.0), Mm(297.0), ops)).collect();

    Ok(document.with_pages(pdf_pages).save(&PdfSaveOptions::default(), &mut warnings))
}

pub(crate) fn content_disposition(first_name: &str, last_name: &str, language: &str) -> String {
    let person =
        format!("{first_name} {last_name}").split_whitespace().collect::<Vec<_>>().join(" ");
    let language = language.trim();
    let utf8_name = if person.is_empty() {
        if language.is_empty() {
            "CV.pdf".to_string()
        } else {
            format!("CV - {language}.pdf")
        }
    } else if language.is_empty() {
        format!("{person} - CV.pdf")
    } else {
        format!("{person} - CV - {language}.pdf")
    };
    let ascii = ascii_filename(&utf8_name);
    format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{}", percent_encode(&utf8_name))
}

fn draw_header(
    ops: &mut Vec<Op>,
    content: &CvPdfContent,
    fonts: &FontIds,
    faces: &Faces,
    photo_id: Option<XObjectId>,
) -> f32 {
    let top = MARGIN_Y;
    let column_width = (PAGE_W - (MARGIN_X * 2.0)) / 2.0;
    let right_x = MARGIN_X + column_width;
    let name_style =
        TextStyle { fonts, faces, role: FontRole::DisplayBold, size: 16.0, color: TEXT };
    let role_style =
        TextStyle { fonts, faces, role: FontRole::DisplayRegular, size: 12.0, color: TEXT };

    let mut left_y = top;
    if let Some(photo_id) = photo_id {
        place_image(
            ops,
            photo_id,
            MARGIN_X + (column_width - PHOTO_PT) / 2.0,
            left_y,
            PHOTO_PT,
            PHOTO_PX,
        );
        left_y += PHOTO_PT + 8.0;
    }
    left_y = draw_centered_block(
        ops,
        &display_name(content),
        name_style,
        MARGIN_X,
        column_width,
        left_y,
    );
    left_y += 2.0;
    if !content.role.trim().is_empty() {
        left_y =
            draw_centered_block(ops, &content.role, role_style, MARGIN_X, column_width, left_y);
    }

    let label_style = TextStyle { fonts, faces, role: FontRole::TextBold, size: 8.0, color: TEXT };
    let value_style =
        TextStyle { fonts, faces, role: FontRole::TextRegular, size: 9.0, color: TEXT };
    let facts_height = facts_block_height(&content.facts, label_style, value_style, column_width);
    let mut right_y = top + ((left_y - top - facts_height) / 2.0).max(0.0);
    for (index, fact) in content.facts.iter().enumerate() {
        if index > 0 {
            right_y += 5.0;
        }
        right_y =
            draw_centered_block(ops, &fact.label, label_style, right_x, column_width, right_y);
        right_y = draw_centered_block_with_link(
            ops,
            &fact.value,
            value_style,
            right_x,
            column_width,
            right_y,
            fact.href.as_deref(),
        );
    }

    let mut bottom = left_y.max(right_y);
    bottom += 8.0;
    let rule_y = PAGE_H - bottom;
    stroke_line(ops, 0.0, rule_y, PAGE_W, rule_y, 0.6, RULE);
    bottom + 14.0
}

fn facts_block_height(
    facts: &[PdfFact],
    label: TextStyle<'_>,
    value: TextStyle<'_>,
    width: f32,
) -> f32 {
    let mut height = 0.0;
    for (index, fact) in facts.iter().enumerate() {
        if index > 0 {
            height += 5.0;
        }
        height += text_block_height(&fact.label, label, width);
        height += text_block_height(&fact.value, value, width);
    }
    height
}

impl Column {
    fn draw_section(&mut self, section: &PdfSection, fonts: &FontIds, faces: &Faces) {
        let heading_height =
            self.wrapped_height(&section.heading, FontRole::DisplayBold, 13.0, faces, self.indent)
                + 8.0;

        for (index, block) in section.blocks.iter().enumerate() {
            let mut needed = self.block_height(block, faces);
            if index == 0 {
                if self.y > self.column_top + 1.0 {
                    needed += 16.0;
                }
                needed += heading_height;
            }
            self.keep_together(needed);

            if index == 0 {
                if self.y > self.column_top + 1.0 {
                    self.gap(16.0);
                }
                self.write(&section.heading, FontRole::DisplayBold, 13.0, TEXT, fonts, faces);
                self.gap(8.0);
            }

            match block {
                PdfBlock::Paragraph(text) => {
                    self.write(text, FontRole::TextRegular, 10.0, TEXT, fonts, faces);
                },
                PdfBlock::Skills(skills) => self.draw_skills(skills),
                PdfBlock::Group(group) => self.draw_group(group, fonts, faces),
                PdfBlock::Entry(entry) => self.draw_indented(BLOCK_INDENT, |column| {
                    column.draw_entry(entry, fonts, faces);
                }),
            }
            self.gap(10.0);
        }
    }

    fn draw_group(&mut self, group: &PdfGroup, fonts: &FontIds, faces: &Faces) {
        self.draw_indented(BLOCK_INDENT, |column| {
            column.write(&group.title, FontRole::DisplayBold, 12.0, TEXT, fonts, faces);
            if !group.subtitle.trim().is_empty() {
                column.write(&group.subtitle, FontRole::TextRegular, 9.5, TEXT, fonts, faces);
            }
            for entry in &group.entries {
                let entry_height = column.entry_height(entry, faces, column.indent + BLOCK_INDENT);
                column.keep_together(entry_height + 6.0);
                if column.y > column.column_top + 1.0 {
                    column.gap(6.0);
                }
                column.draw_indented(BLOCK_INDENT, |column| {
                    column.draw_entry(entry, fonts, faces);
                });
            }
        });
    }

    fn draw_entry(&mut self, entry: &PdfEntry, fonts: &FontIds, faces: &Faces) {
        if !entry.title.trim().is_empty() {
            self.write(&entry.title, FontRole::DisplayBold, 10.5, TEXT, fonts, faces);
        }
        if !entry.subtitle.trim().is_empty() {
            self.write(&entry.subtitle, FontRole::TextRegular, 9.5, TEXT, fonts, faces);
        }
        if !entry.meta.trim().is_empty() {
            self.write(&entry.meta, FontRole::TextLight, 8.0, MUTED, fonts, faces);
        }
        if !entry.body.trim().is_empty() {
            self.gap(2.0);
            self.write(&entry.body, FontRole::TextRegular, 9.5, TEXT, fonts, faces);
        }
        for note in &entry.notes {
            if !note.trim().is_empty() {
                self.write(note, FontRole::TextRegular, 8.5, MUTED, fonts, faces);
            }
        }
    }

    fn draw_skills(&mut self, skills: &[PdfSkill]) {
        if skills.is_empty() {
            return;
        }

        let gap = 14.0;
        let min_item = SKILL_ICON_PT + 8.0;
        let per_row = ((self.width + gap) / (min_item + gap)).floor().max(1.0) as usize;

        for row in skills.chunks(per_row) {
            let count = row.len() as f32;
            let item_width = (self.width - gap * (count - 1.0)) / count;
            self.keep_together(SKILL_ICON_PT);
            let row_top = self.y;

            for (index, _) in row.iter().enumerate() {
                let item_x = self.x + (index as f32 * (item_width + gap));
                if let Some(icon) = self.take_skill_icon() {
                    place_image(
                        &mut self.pages[self.page],
                        icon,
                        item_x + (item_width - SKILL_ICON_PT) / 2.0,
                        row_top,
                        SKILL_ICON_PT,
                        SKILL_ICON_PX,
                    );
                }
            }

            self.y = row_top + SKILL_ICON_PT;
        }
    }

    fn take_skill_icon(&mut self) -> Option<XObjectId> {
        let icon = self.skill_icons.get(self.skill_icon_at).and_then(Clone::clone);
        self.skill_icon_at += 1;
        icon
    }

    fn draw_legal(&mut self, notice: &str, fonts: &FontIds, faces: &Faces) {
        if notice.trim().is_empty() {
            return;
        }
        self.gap(8.0);
        self.ensure(36.0);
        self.write(notice, FontRole::TextLight, 8.0, MUTED, fonts, faces);
    }

    fn draw_page_numbers(&mut self, fonts: &FontIds, faces: &Faces) {
        let total = self.pages.len();
        for (index, ops) in self.pages.iter_mut().enumerate() {
            let label = format!("{} / {total}", index + 1);
            let width = text_width(face(faces, FontRole::TextLight), &label, 8.0);
            let x = (PAGE_W - width) / 2.0;
            paint_text(
                ops,
                &label,
                TextStyle { fonts, faces, role: FontRole::TextLight, size: 8.0, color: MUTED },
                x,
                18.0,
            );
        }
    }

    fn write(
        &mut self,
        text: &str,
        role: FontRole,
        size: f32,
        color: u8,
        fonts: &FontIds,
        faces: &Faces,
    ) {
        let face = face(faces, role);
        let x = self.x + self.indent;
        let width = (self.width - self.indent).max(1.0);
        for line in wrap_text(face, text, size, width) {
            if line.is_empty() {
                self.gap(size * 0.45);
                continue;
            }
            let line_height = size * 1.3;
            self.ensure(line_height);
            let baseline = PAGE_H - (self.y + size);
            paint_text(
                &mut self.pages[self.page],
                &line,
                TextStyle { fonts, faces, role, size, color },
                x,
                baseline,
            );
            self.y += line_height;
        }
    }

    fn draw_indented(&mut self, amount: f32, draw: impl FnOnce(&mut Self)) {
        self.indent += amount;
        draw(self);
        self.indent -= amount;
    }

    fn block_height(&self, block: &PdfBlock, faces: &Faces) -> f32 {
        match block {
            PdfBlock::Paragraph(text) => {
                self.wrapped_height(text, FontRole::TextRegular, 10.0, faces, self.indent)
            },
            PdfBlock::Skills(skills) => self.skills_height(skills),
            PdfBlock::Group(group) => self.group_height(group, faces, self.indent + BLOCK_INDENT),
            PdfBlock::Entry(entry) => self.entry_height(entry, faces, self.indent + BLOCK_INDENT),
        }
    }

    fn group_height(&self, group: &PdfGroup, faces: &Faces, indent: f32) -> f32 {
        let mut height =
            self.wrapped_height(&group.title, FontRole::DisplayBold, 12.0, faces, indent);
        if !group.subtitle.trim().is_empty() {
            height +=
                self.wrapped_height(&group.subtitle, FontRole::TextRegular, 9.5, faces, indent);
        }
        for entry in &group.entries {
            height += 6.0;
            height += self.entry_height(entry, faces, indent + BLOCK_INDENT);
        }
        height
    }

    fn entry_height(&self, entry: &PdfEntry, faces: &Faces, indent: f32) -> f32 {
        let mut height = 0.0;
        if !entry.title.trim().is_empty() {
            height += self.wrapped_height(&entry.title, FontRole::DisplayBold, 10.5, faces, indent);
        }
        if !entry.subtitle.trim().is_empty() {
            height +=
                self.wrapped_height(&entry.subtitle, FontRole::TextRegular, 9.5, faces, indent);
        }
        if !entry.meta.trim().is_empty() {
            height += self.wrapped_height(&entry.meta, FontRole::TextLight, 8.0, faces, indent);
        }
        if !entry.body.trim().is_empty() {
            height += 2.0;
            height += self.wrapped_height(&entry.body, FontRole::TextRegular, 9.5, faces, indent);
        }
        for note in &entry.notes {
            if !note.trim().is_empty() {
                height += self.wrapped_height(note, FontRole::TextRegular, 8.5, faces, indent);
            }
        }
        height
    }

    fn skills_height(&self, skills: &[PdfSkill]) -> f32 {
        if skills.is_empty() {
            return 0.0;
        }
        let gap = 14.0;
        let min_item = SKILL_ICON_PT + 8.0;
        let per_row = ((self.width + gap) / (min_item + gap)).floor().max(1.0) as usize;
        skills.chunks(per_row).count() as f32 * SKILL_ICON_PT
    }

    fn wrapped_height(
        &self,
        text: &str,
        role: FontRole,
        size: f32,
        faces: &Faces,
        indent: f32,
    ) -> f32 {
        let width = (self.width - indent).max(1.0);
        wrap_text(face(faces, role), text, size, width)
            .into_iter()
            .map(|line| if line.is_empty() { size * 0.45 } else { size * 1.3 })
            .sum()
    }

    fn keep_together(&mut self, needed: f32) {
        let available = (PAGE_H - FOOTER) - self.y;
        if needed <= available || self.y <= self.column_top + 1.0 {
            return;
        }
        self.new_page();
    }

    fn gap(&mut self, amount: f32) {
        if self.y + amount > PAGE_H - FOOTER {
            self.new_page();
            return;
        }
        self.y += amount;
    }

    fn ensure(&mut self, needed: f32) {
        if self.y + needed > PAGE_H - FOOTER {
            self.new_page();
        }
    }

    fn new_page(&mut self) {
        self.pages.push(Vec::new());
        self.page += 1;
        self.x = MARGIN_X;
        self.width = PAGE_W - (MARGIN_X * 2.0);
        self.y = MARGIN_Y;
        self.column_top = MARGIN_Y;
    }
}

fn place_image(ops: &mut Vec<Op>, id: XObjectId, left: f32, top: f32, size: f32, pixels: u32) {
    ops.push(Op::UseXobject {
        id,
        transform: XObjectTransform {
            translate_x: Some(Pt(left)),
            translate_y: Some(Pt(PAGE_H - top - size)),
            dpi: Some(pixels as f32 * 72.0 / size),
            ..XObjectTransform::default()
        },
    });
}

fn skill_icon_ids(document: &mut PdfDocument, sections: &[PdfSection]) -> Vec<Option<XObjectId>> {
    let mut icons = Vec::new();
    let mut decode_warnings = Vec::new();
    for section in sections {
        for block in &section.blocks {
            if let PdfBlock::Skills(skills) = block {
                for skill in skills {
                    decode_warnings.clear();
                    let image = rasterize_skill_icon(&skill.icon).and_then(|png| {
                        RawImage::decode_from_bytes(png.as_slice(), &mut decode_warnings).ok()
                    });
                    icons.push(image.map(|image| document.add_image(&image)));
                }
            }
        }
    }
    icons
}

fn rasterize_skill_icon(slug: &str) -> Option<Arc<Vec<u8>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<Vec<u8>>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    if let Ok(guard) = cache.lock()
        && let Some(hit) = guard.get(slug)
    {
        return Some(Arc::clone(hit));
    }

    let png = rasterize_skill_icon_uncached(slug)?;
    let png = Arc::new(png);

    if let Ok(mut guard) = cache.lock() {
        return Some(Arc::clone(guard.entry(slug.to_owned()).or_insert(png)));
    }

    Some(png)
}

fn rasterize_skill_icon_uncached(slug: &str) -> Option<Vec<u8>> {
    let icon = simpleicons_rs::slug(slug)?;
    let svg = icon.svg.replacen("<svg", "<svg fill=\"#444444\"", 1);
    let tree = usvg::Tree::from_data(svg.as_bytes(), &usvg::Options::default()).ok()?;
    let source = tree.size().to_int_size();
    let source_side = source.width().max(source.height()).max(1);
    let scale = SKILL_ICON_PX as f32 / source_side as f32;
    let width = ((source.width() as f32) * scale).round().max(1.0) as u32;
    let height = ((source.height() as f32) * scale).round().max(1.0) as u32;
    let mut pixmap = Pixmap::new(width, height)?;
    resvg::render(&tree, Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    let rgba = RgbaImage::from_raw(width, height, pixmap.data().to_vec())?;

    let mut png = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(rgba).write_to(&mut png, image::ImageFormat::Png).ok()?;
    Some(png.into_inner())
}

fn paint_text(ops: &mut Vec<Op>, text: &str, style: TextStyle<'_>, x: f32, baseline: f32) {
    let text = visible_text(face(style.faces, style.role), text);
    if text.is_empty() {
        return;
    }

    ops.extend([
        Op::StartTextSection,
        Op::SetTextCursor { pos: Point { x: Pt(x), y: Pt(baseline) } },
        Op::SetFont {
            font: PdfFontHandle::External(font_id(style.fonts, style.role).clone()),
            size: Pt(style.size),
        },
        Op::SetLineHeight { lh: Pt(style.size) },
        Op::SetFillColor { col: gray(style.color) },
        Op::ShowText { items: vec![TextItem::Text(text)] },
        Op::EndTextSection,
    ]);
}

fn draw_centered_block(
    ops: &mut Vec<Op>,
    text: &str,
    style: TextStyle<'_>,
    x: f32,
    width: f32,
    y: f32,
) -> f32 {
    draw_centered_block_with_link(ops, text, style, x, width, y, None)
}

fn draw_centered_block_with_link(
    ops: &mut Vec<Op>,
    text: &str,
    style: TextStyle<'_>,
    x: f32,
    width: f32,
    mut y: f32,
    href: Option<&str>,
) -> f32 {
    let face = face(style.faces, style.role);
    for line in wrap_text(face, text, style.size, width) {
        if line.is_empty() {
            y += style.size * 0.4;
            continue;
        }
        let line_width = text_width(face, &line, style.size);
        let text_x = x + ((width - line_width) / 2.0);
        let baseline = PAGE_H - (y + style.size);
        paint_text(ops, &line, style, text_x, baseline);
        if let Some(href) = href.filter(|value| !value.trim().is_empty()) {
            ops.push(Op::LinkAnnotation {
                link: LinkAnnotation::new(
                    Rect::from_xywh(
                        Pt(text_x),
                        Pt(baseline - (style.size * 0.2)),
                        Pt(line_width),
                        Pt(style.size * 1.2),
                    ),
                    Actions::uri(href.to_string()),
                    Some(BorderArray::Solid([0.0, 0.0, 0.0])),
                    Some(ColorArray::Transparent),
                    None,
                ),
            });
        }
        y += style.size * 1.25;
    }
    y
}

fn text_block_height(text: &str, style: TextStyle<'_>, width: f32) -> f32 {
    let face = face(style.faces, style.role);
    wrap_text(face, text, style.size, width)
        .into_iter()
        .map(|line| if line.is_empty() { style.size * 0.4 } else { style.size * 1.25 })
        .sum()
}

fn stroke_line(ops: &mut Vec<Op>, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: u8) {
    ops.extend([
        Op::SetOutlineColor { col: gray(color) },
        Op::SetOutlineThickness { pt: Pt(thickness) },
        Op::DrawLine {
            line: Line {
                points: vec![
                    LinePoint { p: Point { x: Pt(x1), y: Pt(y1) }, bezier: false },
                    LinePoint { p: Point { x: Pt(x2), y: Pt(y2) }, bezier: false },
                ],
                is_closed: false,
            },
        },
    ]);
}

fn circular_portrait(bytes: &[u8]) -> Option<RawImage> {
    let image = image::load_from_memory(bytes).ok()?;
    let mut square = cover_square(image, PHOTO_PX);
    composite_onto_text_color(&mut square);
    mask_circle(&mut square);

    let mut png = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(square).write_to(&mut png, image::ImageFormat::Png).ok()?;
    RawImage::decode_from_bytes(&png.into_inner(), &mut Vec::new()).ok()
}

fn cover_square(image: DynamicImage, size: u32) -> RgbaImage {
    let (width, height) = image.dimensions();
    let side = width.min(height);
    let x = (width - side) / 2;
    let y = (height - side) / 2;
    image
        .crop_imm(x, y, side, side)
        .resize_exact(size, size, image::imageops::FilterType::Lanczos3)
        .to_rgba8()
}

fn composite_onto_text_color(image: &mut RgbaImage) {
    for pixel in image.pixels_mut() {
        let alpha = u32::from(pixel[3]);
        let blend = |source: u8| -> u8 {
            let blended = u32::from(source) * alpha + u32::from(TEXT) * (255 - alpha);
            ((blended + 127) / 255) as u8
        };
        *pixel = Rgba([blend(pixel[0]), blend(pixel[1]), blend(pixel[2]), 255]);
    }
}

fn mask_circle(image: &mut RgbaImage) {
    let size = image.width() as f32;
    let center = (size - 1.0) / 2.0;
    let radius = center;
    for y in 0..image.height() {
        for x in 0..image.width() {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            if dx.mul_add(dx, dy * dy) > radius * radius {
                image.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
    }
}

fn wrap_text(face: &FontRef<'_>, text: &str, size: f32, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for (index, paragraph) in text.replace("\r\n", "\n").split('\n').enumerate() {
        if index > 0 && paragraph.trim().is_empty() && lines.last().is_some_and(String::is_empty) {
            continue;
        }
        if paragraph.trim().is_empty() {
            lines.push(String::new());
            continue;
        }
        lines.extend(wrap_paragraph(face, paragraph.trim(), size, max_width));
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

fn wrap_paragraph(face: &FontRef<'_>, text: &str, size: f32, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        let parts = split_overflow(face, word, size, max_width);
        for part in parts {
            if current.is_empty() {
                current = part;
                continue;
            }
            let current_width = text_width(face, &current, size);
            let part_width = text_width(face, &part, size);
            let space_width = text_width(face, " ", size);
            if current_width + space_width + part_width <= max_width {
                current.push(' ');
                current.push_str(&part);
            } else {
                lines.push(std::mem::take(&mut current));
                current = part;
            }
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn split_overflow(face: &FontRef<'_>, word: &str, size: f32, max_width: f32) -> Vec<String> {
    if text_width(face, word, size) <= max_width {
        return vec![word.to_string()];
    }

    let mut parts = Vec::new();
    let mut current = String::new();
    for ch in visible_text(face, word).chars() {
        current.push(ch);
        let wider_than_line = text_width(face, &current, size) > max_width;
        let has_previous_char = current.chars().nth(1).is_some();
        if wider_than_line && has_previous_char {
            current.pop();
            parts.push(std::mem::take(&mut current));
            current.push(ch);
        }
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

fn text_width(face: &FontRef<'_>, text: &str, size: f32) -> f32 {
    let metrics = face.glyph_metrics(Size::new(size), LocationRef::default());
    let charmap = face.charmap();
    let mut width = 0.0;
    for ch in text.chars() {
        let Some(glyph) = charmap.map(ch) else {
            continue;
        };
        width += metrics.advance_width(glyph).unwrap_or(0.0);
    }
    width
}

fn visible_text(face: &FontRef<'_>, text: &str) -> String {
    let charmap = face.charmap();
    text.chars().filter(|ch| ch.is_whitespace() || charmap.map(*ch).is_some()).collect()
}

fn face(faces: &Faces, role: FontRole) -> &FontRef<'static> {
    match role {
        FontRole::DisplayBold => &faces.display_bold,
        FontRole::DisplayRegular => &faces.display_regular,
        FontRole::TextBold => &faces.text_bold,
        FontRole::TextRegular => &faces.text_regular,
        FontRole::TextLight => &faces.text_light,
    }
}

fn font_id(fonts: &FontIds, role: FontRole) -> &printpdf::FontId {
    match role {
        FontRole::DisplayBold => &fonts.display_bold,
        FontRole::DisplayRegular => &fonts.display_regular,
        FontRole::TextBold => &fonts.text_bold,
        FontRole::TextRegular => &fonts.text_regular,
        FontRole::TextLight => &fonts.text_light,
    }
}

fn gray(value: u8) -> Color {
    let channel = f32::from(value) / 255.0;
    Color::Rgb(Rgb { r: channel, g: channel, b: channel, icc_profile: None })
}

fn display_name(content: &CvPdfContent) -> String {
    format!("{} {}", content.first_name.trim(), content.last_name.trim())
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn font_bytes() -> AppResult<FontBytes> {
    Ok(FontBytes {
        display_bold: cached_font(&DISPLAY_BOLD, DISPLAY_BOLD_WOFF2, "Red Hat Display Bold")?,
        display_regular: cached_font(&DISPLAY_REGULAR, DISPLAY_REGULAR_WOFF2, "Red Hat Display")?,
        text_bold: cached_font(&TEXT_BOLD, TEXT_BOLD_WOFF2, "Red Hat Text Bold")?,
        text_regular: cached_font(&TEXT_REGULAR, TEXT_REGULAR_WOFF2, "Red Hat Text")?,
        text_light: cached_font(&TEXT_LIGHT, TEXT_LIGHT_WOFF2, "Red Hat Text Light")?,
    })
}

fn cached_font(
    slot: &'static OnceLock<Vec<u8>>,
    woff2: &[u8],
    name: &str,
) -> AppResult<&'static [u8]> {
    if let Some(bytes) = slot.get() {
        return Ok(bytes.as_slice());
    }

    let decoded = wuff::decompress_woff2(woff2).map_err(|error| {
        AppError::msg(error.to_string()).with_context(format!("decoding the {name} font"))
    })?;
    Ok(slot.get_or_init(|| decoded).as_slice())
}

fn parse_face(bytes: &'static [u8], name: &str) -> AppResult<FontRef<'static>> {
    FontRef::new(bytes).map_err(|error| {
        AppError::msg(error.to_string()).with_context(format!("reading the {name} font"))
    })
}

fn add_font(document: &mut PdfDocument, bytes: &[u8], name: &str) -> AppResult<printpdf::FontId> {
    let mut font_warnings = Vec::new();
    let font = ParsedFont::from_bytes(bytes, 0, &mut font_warnings)
        .ok_or_else(|| AppError::msg(format!("embedding the {name} font")))?;
    Ok(document.add_font(&font))
}

fn ascii_filename(utf8_name: &str) -> String {
    let stem = utf8_name.strip_suffix(".pdf").unwrap_or(utf8_name);
    let mut name = String::new();
    for ch in stem.chars() {
        if let Some(folded) = fold_ascii(ch) {
            name.push(folded);
        } else if (ch.is_ascii_whitespace() || ch == '-') && !name.ends_with('-') {
            name.push('-');
        }
    }
    let name = name.trim_matches('-');
    if name.is_empty() { "cv.pdf".to_string() } else { format!("{name}.pdf") }
}

fn fold_ascii(ch: char) -> Option<char> {
    let folded = match ch {
        'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' | 'ā' => 'a',
        'è' | 'é' | 'ê' | 'ë' | 'ē' => 'e',
        'ì' | 'í' | 'î' | 'ï' | 'ī' => 'i',
        'ò' | 'ó' | 'ô' | 'ö' | 'õ' | 'ō' => 'o',
        'ù' | 'ú' | 'û' | 'ü' | 'ū' => 'u',
        'ý' | 'ÿ' => 'y',
        'ñ' => 'n',
        'ç' => 'c',
        'À' | 'Á' | 'Â' | 'Ä' | 'Ã' | 'Å' | 'Ā' => 'A',
        'È' | 'É' | 'Ê' | 'Ë' | 'Ē' => 'E',
        'Ì' | 'Í' | 'Î' | 'Ï' | 'Ī' => 'I',
        'Ò' | 'Ó' | 'Ô' | 'Ö' | 'Õ' | 'Ō' => 'O',
        'Ù' | 'Ú' | 'Û' | 'Ü' | 'Ū' => 'U',
        'Ý' => 'Y',
        'Ñ' => 'N',
        'Ç' => 'C',
        _ if ch.is_ascii_alphanumeric() => ch,
        _ => return None,
    };
    Some(folded)
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use printpdf::PdfParseOptions;

    #[test]
    fn curriculum_pdf_contains_the_visible_sections() {
        let content = CvPdfContent {
            title: "Alex Carter".to_string(),
            first_name: "Alex".to_string(),
            last_name: "Carter".to_string(),
            role: "Full Stack Developer".to_string(),
            photo: None,
            facts: vec![PdfFact {
                label: "City:".to_string(),
                value: "Austin".to_string(),
                href: None,
            }],
            sections: vec![
                PdfSection {
                    heading: "Experience".to_string(),
                    blocks: vec![PdfBlock::Group(PdfGroup {
                        title: "BrightForge".to_string(),
                        subtitle: "Remote".to_string(),
                        entries: vec![PdfEntry {
                            title: "Senior Developer".to_string(),
                            subtitle: String::new(),
                            meta: "January 2022 - Present".to_string(),
                            body: "Built internal tools and kept the product simple. ".repeat(220),
                            notes: vec!["https://example.com".to_string()],
                        }],
                    })],
                },
                PdfSection {
                    heading: "Skills".to_string(),
                    blocks: vec![PdfBlock::Skills(vec![PdfSkill { icon: "rust".to_string() }])],
                },
            ],
            legal_notice: "Personal data notice.".to_string(),
        };

        let bytes = render(&content).expect("pdf");
        assert!(bytes.starts_with(b"%PDF"));
        assert!(
            bytes.windows(9).any(|window| window == b"FontFile2"),
            "the PDF should embed the Red Hat font program"
        );

        let parsed = PdfDocument::parse(&bytes, &PdfParseOptions::default(), &mut Vec::new())
            .expect("parse pdf");
        let text = parsed.extract_text().into_iter().flatten().collect::<Vec<_>>().join("\n");
        assert!(text.contains("Alex Carter"), "{text}");
        assert!(text.contains("BrightForge"), "{text}");
        assert!(text.contains("Experience"), "{text}");
        assert!(text.contains("Skills"), "{text}");
        assert!(!text.contains("Rust"), "{text}");
        assert!(!text.contains("GitHub"), "{text}");
        assert!(
            bytes.windows(6).any(|window| window == b"/Image"),
            "skill icons should be embedded images"
        );
        assert!(parsed.pages.len() >= 2, "long descriptions should continue on another page");
    }

    #[test]
    fn blocks_that_fit_on_one_page_are_not_split() {
        let content = CvPdfContent {
            title: "Alex Carter".to_string(),
            first_name: "Alex".to_string(),
            last_name: "Carter".to_string(),
            role: "Developer".to_string(),
            photo: None,
            facts: Vec::new(),
            sections: vec![
                PdfSection {
                    heading: "Experience".to_string(),
                    blocks: vec![PdfBlock::Group(PdfGroup {
                        title: "Filler".to_string(),
                        subtitle: "Remote".to_string(),
                        entries: vec![PdfEntry {
                            title: "Long role".to_string(),
                            subtitle: String::new(),
                            meta: "2020 - 2024".to_string(),
                            body: "Filled the page with a long description. ".repeat(180),
                            notes: Vec::new(),
                        }],
                    })],
                },
                PdfSection {
                    heading: "Other Projects".to_string(),
                    blocks: vec![PdfBlock::Entry(PdfEntry {
                        title: "Whole Project".to_string(),
                        subtitle: String::new(),
                        meta: "2024".to_string(),
                        body: "This project stays together.".to_string(),
                        notes: vec!["https://example.com/project".to_string()],
                    })],
                },
            ],
            legal_notice: String::new(),
        };

        let bytes = render(&content).expect("pdf");
        let parsed = PdfDocument::parse(&bytes, &PdfParseOptions::default(), &mut Vec::new())
            .expect("parse pdf");
        let pages =
            parsed.extract_text().into_iter().map(|page| page.join("\n")).collect::<Vec<_>>();

        let project_pages = pages
            .iter()
            .filter(|page| {
                page.contains("Whole Project")
                    || page.contains("This project stays together.")
                    || page.contains("https://example.com/project")
            })
            .count();
        assert_eq!(project_pages, 1, "a project was split across pages: {pages:?}");
    }

    #[test]
    fn transparent_portrait_background_uses_the_text_color() {
        let mut image = RgbaImage::from_pixel(4, 4, Rgba([0, 0, 0, 0]));
        image.put_pixel(1, 1, Rgba([10, 20, 30, 255]));
        image.put_pixel(1, 2, Rgba([255, 0, 0, 128]));

        composite_onto_text_color(&mut image);

        assert_eq!(image.get_pixel(0, 0).0, [TEXT, TEXT, TEXT, 255]);
        assert_eq!(image.get_pixel(1, 1).0, [10, 20, 30, 255]);
        assert_eq!(image.get_pixel(1, 2).0, [162, 34, 34, 255]);
    }

    #[test]
    fn content_disposition_keeps_unicode_out_of_the_ascii_filename() {
        let header = content_disposition("Giòrgio", "Reale", "it");
        assert!(
            header.contains("filename=\"Giorgio-Reale-CV-it.pdf\""),
            "header was {header}"
        );
        assert!(header.contains("filename*=UTF-8''"));
        assert!(
            header.contains("Gi%C3%B2rgio%20Reale%20-%20CV%20-%20it.pdf"),
            "header was {header}"
        );
    }
}
