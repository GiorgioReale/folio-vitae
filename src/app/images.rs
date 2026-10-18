use std::{
    fs,
    path::{Path, PathBuf},
};

use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
use image::{DynamicImage, GenericImageView, RgbaImage, imageops::FilterType};
use resvg::{
    tiny_skia::{Pixmap, Transform},
    usvg,
};

use crate::{
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

pub(crate) fn generated_outputs_missing() -> AppResult<bool> {
    let root = project_root()?;

    for path in expected_cv_outputs(&root)? {
        if !path.exists() {
            return Ok(true);
        }
    }

    for path in expected_main_outputs(&root)? {
        if !path.exists() {
            return Ok(true);
        }
    }

    Ok(false)
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

fn expected_cv_outputs(root: &Path) -> AppResult<Vec<PathBuf>> {
    let outputs = OutputPaths::new(root, "cv");
    let images_root = data_dir().join("images");

    let favicon_path = find_supported_image(&images_root, &["cv"], "favicon")?;
    let favicon_is_svg = favicon_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("svg"))
        .unwrap_or(false);

    let mut paths = vec![
        outputs.image(&["home-background"], "home-background.avif"),
        outputs.image(&["home-background"], "home-background.jpg"),
        outputs.image(&["home-background"], "home-background-mobile.avif"),
        outputs.image(&["home-background"], "home-background-mobile.jpg"),
        outputs.icon(&["logos"], "android-chrome-192x192.png"),
        outputs.icon(&["logos"], "android-chrome-512x512.png"),
        outputs.image(&["profile-pictures"], "profile-picture.avif"),
        outputs.image(&["profile-pictures"], "profile-picture.png"),
        outputs.image(&["social-card"], "social-card.png"),
        outputs.logos_dir().join("apple-touch-icon.png"),
        outputs.logos_dir().join("favicon-16x16.png"),
        outputs.logos_dir().join("favicon-32x32.png"),
        outputs.logos_dir().join("favicon.ico"),
    ];

    if favicon_is_svg {
        paths.push(outputs.logos_dir().join("favicon.svg"));
    }

    Ok(paths)
}

fn expected_main_outputs(root: &Path) -> AppResult<Vec<PathBuf>> {
    let outputs = OutputPaths::new(root, "main");
    let images_root = data_dir().join("images");

    let favicon_path = find_supported_image(&images_root, &["main"], "favicon")?;
    let favicon_is_svg = favicon_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("svg"))
        .unwrap_or(false);

    let mut paths = vec![
        outputs.icon(&["logos"], "android-chrome-192x192.png"),
        outputs.icon(&["logos"], "android-chrome-512x512.png"),
        outputs.image(&["profile-pictures"], "profile-picture.avif"),
        outputs.image(&["profile-pictures"], "profile-picture.png"),
        outputs.image(&["social-card"], "social-card.png"),
        outputs.logos_dir().join("apple-touch-icon.png"),
        outputs.logos_dir().join("favicon-16x16.png"),
        outputs.logos_dir().join("favicon-32x32.png"),
        outputs.logos_dir().join("favicon.ico"),
    ];

    if favicon_is_svg {
        paths.push(outputs.logos_dir().join("favicon.svg"));
    }

    Ok(paths)
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
    if is_up_to_date(input, output)? {
        return Ok(());
    }

    let image = load_dynamic_image(input)?;
    ensure_parent_directory(output)?;
    image.save(output).with_context(|| format!("writing {output:?}"))?;
    Ok(())
}

fn export_resized(input: &Path, outputs: &[(&Path, u32, u32)]) -> AppResult<()> {
    let mut pending = Vec::new();

    for &(output, width, height) in outputs {
        if !is_up_to_date(input, output)? {
            pending.push((output, width, height));
        }
    }

    if pending.is_empty() {
        return Ok(());
    }

    let image = load_rgba_image(input)?;

    for (output, width, height) in pending {
        let resized = resize_cover(&image, width, height);
        ensure_parent_directory(output)?;
        resized.save(output).with_context(|| format!("writing {output:?}"))?;
    }

    Ok(())
}

fn load_dynamic_image(input: &Path) -> AppResult<DynamicImage> {
    if input
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
    {
        rasterize_svg(input)
    } else {
        image::open(input).with_context(|| format!("opening {input:?}"))
    }
}

fn load_rgba_image(input: &Path) -> AppResult<DynamicImage> {
    Ok(DynamicImage::ImageRgba8(load_dynamic_image(input)?.into_rgba8()))
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

    let mut source_image: Option<DynamicImage> = None;

    if source
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
    {
        let svg_output = output_dir.join("favicon.svg");
        copy_svg(source, &svg_output)?;
    }

    for (output, width, height) in
        [(&apple_touch, 180, 180), (&favicon_16, 16, 16), (&favicon_32, 32, 32)]
    {
        if is_up_to_date(source, output)? {
            continue;
        }

        if source_image.is_none() {
            source_image = Some(load_rgba_image(source)?);
        }

        let Some(image) = source_image.as_ref() else {
            continue;
        };
        let resized = resize_cover(image, width, height);
        ensure_parent_directory(output)?;
        resized.save(output).with_context(|| format!("writing {output:?}"))?;
    }

    if !is_up_to_date(&favicon_16, &ico_output)? || !is_up_to_date(&favicon_32, &ico_output)? {
        save_ico(&[(&favicon_16, 16, 16), (&favicon_32, 32, 32)], &ico_output)?;
    }

    Ok(())
}

fn rasterize_svg(svg_source: &Path) -> AppResult<DynamicImage> {
    let data = fs::read(svg_source).with_context(|| format!("reading {svg_source:?}"))?;

    let options = usvg::Options {
        resources_dir: svg_source.parent().map(|parent| parent.to_path_buf()),
        ..usvg::Options::default()
    };

    let tree = usvg::Tree::from_data(&data, &options)
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

fn remove_stale_file(path: &Path) -> AppResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("removing {path:?}")),
    }
}

fn copy_svg(input: &Path, output: &Path) -> AppResult<()> {
    if is_up_to_date(input, output)? {
        return Ok(());
    }

    ensure_parent_directory(output)?;
    fs::copy(input, output).with_context(|| format!("copying {input:?} to {output:?}"))?;
    Ok(())
}

fn save_ico(images: &[(&Path, u32, u32)], output: &Path) -> AppResult<()> {
    let mut should_write = false;
    for (path, _, _) in images {
        if !is_up_to_date(path, output)? {
            should_write = true;
            break;
        }
    }

    if !should_write {
        return Ok(());
    }

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

fn is_up_to_date(input: &Path, output: &Path) -> AppResult<bool> {
    let output_metadata = match fs::metadata(output) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(false),
    };

    let input_metadata =
        fs::metadata(input).with_context(|| format!("reading metadata for {input:?}"))?;

    let input_modified = input_metadata
        .modified()
        .with_context(|| format!("getting modification time for {input:?}"))?;
    let output_modified = output_metadata
        .modified()
        .with_context(|| format!("getting modification time for {output:?}"))?;

    Ok(output_modified >= input_modified)
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
