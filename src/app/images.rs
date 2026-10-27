use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
use image::{DynamicImage, GenericImageView, RgbaImage, imageops::FilterType};
use resvg::{
    tiny_skia::{Pixmap, Transform},
    usvg,
};

use crate::{
    app::limits::{MAX_DECODED_PIXELS, MAX_IMAGE_FILE_BYTES, MAX_SVG_FILE_BYTES},
    config::data_dir,
    error::{AppError, AppResult, ResultExt},
};

const SUPPORTED_EXTENSIONS: &[&str] = &["avif", "webp", "png", "jpg", "jpeg", "svg"];

const MAX_SVG_RASTER_DIMENSION: u32 = 4096;

#[derive(Clone)]
struct OutputPaths {
    images_root: PathBuf,
    icons_root: PathBuf,
}

impl OutputPaths {
    fn new(root: &Path, scope: &str) -> Self {
        Self {
            images_root: root.join("assets").join("images").join(scope),
            icons_root: root.join("assets").join("icons").join(scope),
        }
    }

    fn image(&self, segments: &[&str], filename: &str) -> PathBuf {
        path_with_filename(&self.images_root, segments, filename)
    }

    fn icon(&self, segments: &[&str], filename: &str) -> PathBuf {
        path_with_filename(&self.icons_root, segments, filename)
    }

    fn logos_dir(&self) -> PathBuf {
        path_with_segments(&self.icons_root, &["logos"])
    }
}

fn find_supported_image(
    base_dir: &Path,
    scope: &[&str],
    base_filename: &str,
) -> AppResult<PathBuf> {
    let scoped_dir = path_with_segments(base_dir, scope);

    for extension in SUPPORTED_EXTENSIONS {
        let candidate = scoped_dir.join(format!("{base_filename}.{extension}"));
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(AppError::msg(format!(
        "no source image found for {}/{base_filename} with extensions {:?}",
        scoped_dir.display(),
        SUPPORTED_EXTENSIONS
    )))
}

pub(crate) fn generate_all_images() -> AppResult<()> {
    let root = project_root()?;

    generate_cv_assets(&root)?;
    generate_main_assets(&root)?;

    Ok(())
}

fn generate_cv_assets(root: &Path) -> AppResult<()> {
    let outputs = OutputPaths::new(root, "cv");
    let images_root = data_dir().join("images");

    let favicon_path = find_supported_image(&images_root, &["cv"], "favicon")?;

    let home_background = find_supported_image(&images_root, &["cv"], "home-background")?;
    let home_background_mobile =
        find_supported_image(&images_root, &["cv"], "home-background-mobile")?;
    let cv_profile_picture = find_supported_image(&images_root, &["cv"], "profile-picture")?;
    let cv_social_card = find_supported_image(&images_root, &["cv"], "social-card")?;

    let home_avif = outputs.image(&["home-background"], "home-background.avif");
    let home_jpg = outputs.image(&["home-background"], "home-background.jpg");
    export_resized(&home_background, &[(&home_avif, 1440, 1920), (&home_jpg, 1440, 1920)])?;

    let mobile_avif = outputs.image(&["home-background"], "home-background-mobile.avif");
    let mobile_jpg = outputs.image(&["home-background"], "home-background-mobile.jpg");
    export_resized(
        &home_background_mobile,
        &[(&mobile_avif, 1440, 810), (&mobile_jpg, 1440, 810)],
    )?;

    let chrome_192 = outputs.icon(&["logos"], "android-chrome-192x192.png");
    let chrome_512 = outputs.icon(&["logos"], "android-chrome-512x512.png");
    export_resized(&favicon_path, &[(&chrome_192, 192, 192), (&chrome_512, 512, 512)])?;

    let profile_avif = outputs.image(&["profile-pictures"], "profile-picture.avif");
    let profile_png = outputs.image(&["profile-pictures"], "profile-picture.png");
    export_resized(&cv_profile_picture, &[(&profile_avif, 512, 512), (&profile_png, 512, 512)])?;
    remove_stale_file(&outputs.image(&["profile-pictures"], "profile-picture.jpg"))?;

    copy_image(&cv_social_card, &outputs.image(&["social-card"], "social-card.png"))?;

    generate_favicon_set(&favicon_path, &outputs.logos_dir())?;

    Ok(())
}

fn generate_main_assets(root: &Path) -> AppResult<()> {
    let outputs = OutputPaths::new(root, "main");

    let images_root = data_dir().join("images");

    let favicon_path = find_supported_image(&images_root, &["main"], "favicon")?;

    let main_icon = find_supported_image(&images_root, &["main"], "icon")?;
    let main_profile_picture = find_supported_image(&images_root, &["main"], "profile-picture")?;
    let main_social_card = find_supported_image(&images_root, &["main"], "social-card")?;

    let chrome_192 = outputs.icon(&["logos"], "android-chrome-192x192.png");
    let chrome_512 = outputs.icon(&["logos"], "android-chrome-512x512.png");
    export_resized(&main_icon, &[(&chrome_192, 192, 192), (&chrome_512, 512, 512)])?;

    let profile_avif = outputs.image(&["profile-pictures"], "profile-picture.avif");
    let profile_png = outputs.image(&["profile-pictures"], "profile-picture.png");
    export_resized(&main_profile_picture, &[(&profile_avif, 512, 512), (&profile_png, 512, 512)])?;
    remove_stale_file(&outputs.image(&["profile-pictures"], "profile-picture.jpg"))?;

    copy_image(&main_social_card, &outputs.image(&["social-card"], "social-card.png"))?;

    generate_favicon_set(&favicon_path, &outputs.logos_dir())?;

    Ok(())
}

fn path_with_filename(base: &Path, segments: &[&str], filename: &str) -> PathBuf {
    let mut path = path_with_segments(base, segments);
    path.push(filename);
    path
}

fn path_with_segments(base: &Path, segments: &[&str]) -> PathBuf {
    segments.iter().fold(base.to_path_buf(), |mut acc, segment| {
        acc.push(segment);
        acc
    })
}

fn copy_image(input: &Path, output: &Path) -> AppResult<()> {
    let image = load_dynamic_image(input)?;
    ensure_parent_directory(output)?;
    image.save(output).with_context(|| format!("writing {output:?}"))?;
    Ok(())
}

fn export_resized(input: &Path, outputs: &[(&Path, u32, u32)]) -> AppResult<()> {
    let image = load_rgba_image(input)?;

    for &(output, width, height) in outputs {
        let resized = resize_cover(&image, width, height);
        ensure_parent_directory(output)?;
        resized.save(output).with_context(|| format!("writing {output:?}"))?;
    }

    Ok(())
}

fn load_dynamic_image(input: &Path) -> AppResult<DynamicImage> {
    ensure_file_within_limit(input, MAX_IMAGE_FILE_BYTES)?;

    let image = if input
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
    {
        rasterize_svg(input)?
    } else {
        image::open(input).with_context(|| format!("opening {input:?}"))?
    };

    ensure_decoded_pixel_limit(&image, input)?;
    Ok(image)
}

fn load_rgba_image(input: &Path) -> AppResult<DynamicImage> {
    Ok(DynamicImage::ImageRgba8(load_dynamic_image(input)?.into_rgba8()))
}

fn ensure_file_within_limit(path: &Path, max_bytes: u64) -> AppResult<()> {
    let metadata = fs::metadata(path).with_context(|| format!("reading metadata for {path:?}"))?;
    if metadata.len() > max_bytes {
        return Err(AppError::msg(format!(
            "file {path:?} is {} bytes; maximum allowed is {max_bytes} bytes",
            metadata.len()
        )));
    }
    Ok(())
}

fn ensure_decoded_pixel_limit(image: &DynamicImage, path: &Path) -> AppResult<()> {
    let (width, height) = image.dimensions();
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if pixels > MAX_DECODED_PIXELS {
        return Err(AppError::msg(format!(
            "decoded image {path:?} is {width}x{height} ({pixels} pixels); maximum allowed is {MAX_DECODED_PIXELS} pixels"
        )));
    }
    Ok(())
}

/// Center-crop the source to the destination aspect ratio, then scale uniformly.
fn resize_cover(image: &DynamicImage, width: u32, height: u32) -> DynamicImage {
    let (src_width, src_height) = image.dimensions();
    if src_width == 0 || src_height == 0 || width == 0 || height == 0 {
        return image.resize_exact(width.max(1), height.max(1), FilterType::Triangle);
    }

    let src_ratio = f64::from(src_width) / f64::from(src_height);
    let dst_ratio = f64::from(width) / f64::from(height);

    let (crop_width, crop_height) = if src_ratio > dst_ratio {
        let crop_width = ((f64::from(src_height) * dst_ratio).round() as u32).clamp(1, src_width);
        (crop_width, src_height)
    } else {
        let crop_height = ((f64::from(src_width) / dst_ratio).round() as u32).clamp(1, src_height);
        (src_width, crop_height)
    };

    let left = (src_width - crop_width) / 2;
    let top = (src_height - crop_height) / 2;

    image.crop_imm(left, top, crop_width, crop_height).resize_exact(
        width,
        height,
        FilterType::Triangle,
    )
}

fn generate_favicon_set(source: &Path, output_dir: &Path) -> AppResult<()> {
    let apple_touch = output_dir.join("apple-touch-icon.png");
    let favicon_16 = output_dir.join("favicon-16x16.png");
    let favicon_32 = output_dir.join("favicon-32x32.png");
    let ico_output = output_dir.join("favicon.ico");

    if source
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
    {
        let svg_output = output_dir.join("favicon.svg");
        copy_svg(source, &svg_output)?;
    }

    let source_image = load_rgba_image(source)?;

    for (output, width, height) in
        [(&apple_touch, 180, 180), (&favicon_16, 16, 16), (&favicon_32, 32, 32)]
    {
        let resized = resize_cover(&source_image, width, height);
        ensure_parent_directory(output)?;
        resized.save(output).with_context(|| format!("writing {output:?}"))?;
    }

    save_ico(&[(&favicon_16, 16, 16), (&favicon_32, 32, 32)], &ico_output)?;

    Ok(())
}

fn rasterize_svg(svg_source: &Path) -> AppResult<DynamicImage> {
    ensure_file_within_limit(svg_source, MAX_SVG_FILE_BYTES)?;
    let data = fs::read(svg_source).with_context(|| format!("reading {svg_source:?}"))?;

    // usvg/simplecss cannot apply @media and logs noisy warnings; PNG/ICO also cannot
    // honor prefers-color-scheme, so drop those rules only for rasterization. The
    // browser-facing favicon.svg copy keeps the original media queries.
    let svg = String::from_utf8_lossy(&data);
    let svg_for_raster = strip_css_at_media_rules(&svg);

    let resources_dir = svg_source.parent().map(|parent| parent.to_path_buf());
    let options = confined_svg_options(resources_dir);

    let tree = usvg::Tree::from_data(svg_for_raster.as_bytes(), &options)
        .with_context(|| format!("parsing {svg_source:?} as SVG"))?;

    let size = tree.size().to_int_size();
    let (mut width, mut height) = (size.width(), size.height());

    if width == 0 || height == 0 {
        return Err(AppError::msg(format!(
            "invalid SVG dimensions ({width}x{height}) for {svg_source:?}",
        )));
    }

    let scale = if width > MAX_SVG_RASTER_DIMENSION || height > MAX_SVG_RASTER_DIMENSION {
        let max_dimension = MAX_SVG_RASTER_DIMENSION as f64;
        let scale_factor = (max_dimension / width as f64).min(max_dimension / height as f64);
        width = ((width as f64 * scale_factor).max(1.0)).round() as u32;
        height = ((height as f64 * scale_factor).max(1.0)).round() as u32;
        scale_factor as f32
    } else {
        1.0
    };

    let mut pixmap = Pixmap::new(width, height)
        .ok_or_else(|| AppError::msg(format!("creating pixmap for {svg_source:?}")))?;

    let mut pixmap_mut = pixmap.as_mut();
    resvg::render(&tree, Transform::from_scale(scale, scale), &mut pixmap_mut);

    let rgba: RgbaImage = RgbaImage::from_raw(width, height, pixmap.data().to_vec())
        .ok_or_else(|| AppError::msg(format!("creating RGBA image for {svg_source:?}")))?;

    Ok(DynamicImage::ImageRgba8(rgba))
}

fn strip_css_at_media_rules(svg: &str) -> String {
    let mut output = String::with_capacity(svg.len());
    let mut rest = svg;

    while let Some(media_at) = rest.find("@media") {
        output.push_str(&rest[..media_at]);
        let after_media = &rest[media_at..];
        let Some(brace_rel) = after_media.find('{') else {
            output.push_str(after_media);
            return output;
        };

        let mut depth = 0usize;
        let mut end = brace_rel;
        for (offset, ch) in after_media[brace_rel..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        end = brace_rel + offset + ch.len_utf8();
                        break;
                    }
                },
                _ => {},
            }
        }

        rest = &after_media[end..];
    }

    output.push_str(rest);
    output
}

fn confined_svg_options(resources_dir: Option<PathBuf>) -> usvg::Options<'static> {
    let base_dir = resources_dir.clone();
    let mut options = usvg::Options { resources_dir, ..usvg::Options::default() };

    options.image_href_resolver = usvg::ImageHrefResolver {
        resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
        resolve_string: Box::new(move |href: &str, opts: &usvg::Options<'_>| {
            if !is_confined_svg_href(href) {
                return None;
            }

            let path = opts.get_abs_path(Path::new(href));
            if let Some(base) = base_dir.as_ref()
                && !path_is_within_base(&path, base)
            {
                return None;
            }

            if fs::metadata(&path).ok().is_none_or(|meta| meta.len() > MAX_IMAGE_FILE_BYTES) {
                return None;
            }

            usvg::ImageHrefResolver::default_string_resolver()(href, opts)
        }),
    };

    options
}

fn is_confined_svg_href(href: &str) -> bool {
    if href.is_empty() || href.contains("://") || href.starts_with('/') {
        return false;
    }

    let path = Path::new(href);
    if path.is_absolute() {
        return false;
    }

    !path.components().any(|component| {
        matches!(component, Component::RootDir | Component::ParentDir | Component::Prefix(_))
    })
}

fn path_is_within_base(path: &Path, base: &Path) -> bool {
    let Ok(base) = fs::canonicalize(base) else {
        return false;
    };
    let Ok(path) = fs::canonicalize(path) else {
        return false;
    };
    path.starts_with(&base)
}

fn remove_stale_file(path: &Path) -> AppResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("removing {path:?}")),
    }
}

fn copy_svg(input: &Path, output: &Path) -> AppResult<()> {
    ensure_parent_directory(output)?;
    fs::copy(input, output).with_context(|| format!("copying {input:?} to {output:?}"))?;
    Ok(())
}

fn save_ico(images: &[(&Path, u32, u32)], output: &Path) -> AppResult<()> {
    let mut icon_dir = IconDir::new(ResourceType::Icon);

    for (path, width, height) in images {
        let dynamic = image::open(path).with_context(|| format!("opening {path:?}"))?;
        let rgba = dynamic.to_rgba8();
        let icon_image = IconImage::from_rgba_data(*width, *height, rgba.into_raw());
        let entry = IconDirEntry::encode(&icon_image)
            .with_context(|| format!("encoding {path:?} for ICO"))?;
        icon_dir.add_entry(entry);
    }

    ensure_parent_directory(output)?;
    let mut file = fs::File::create(output).with_context(|| format!("creating {output:?}"))?;
    icon_dir.write(&mut file).with_context(|| format!("writing {output:?}"))?;

    Ok(())
}

fn ensure_parent_directory(path: &Path) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating directory {parent:?}"))?;
    }
    Ok(())
}

fn project_root() -> AppResult<PathBuf> {
    std::env::current_dir().with_context(|| "reading current project directory")
}

#[cfg(test)]
mod tests {
    use super::strip_css_at_media_rules;

    #[test]
    fn strip_css_at_media_rules_removes_prefers_color_scheme_blocks() {
        let svg = r#"<svg><style>
    svg { color: #444; }
    @media (prefers-color-scheme: dark) {
      svg { color: #bbb; }
    }
    path { fill: currentColor; }
  </style></svg>"#;

        let stripped = strip_css_at_media_rules(svg);
        assert!(stripped.contains("color: #444"));
        assert!(stripped.contains("fill: currentColor"));
        assert!(!stripped.contains("@media"));
        assert!(!stripped.contains("color: #bbb"));
    }

    #[test]
    fn strip_css_at_media_rules_leaves_plain_svg_unchanged() {
        let svg = r#"<svg><path d="M0 0h1v1z"/></svg>"#;
        assert_eq!(strip_css_at_media_rules(svg), svg);
    }
}
