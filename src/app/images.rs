use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use fast_image_resize as fir;
use fir::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer, images::Image};
use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
use image::{DynamicImage, RgbaImage};
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
    let cv_icon = favicon_path.clone();
    let cv_profile_picture = find_supported_image(&images_root, &["cv"], "profile-picture")?;
    let cv_social_card = find_supported_image(&images_root, &["cv"], "social-card")?;
    let cv_favicon = cv_icon.clone();

    resize_image(
        &home_background,
        &outputs.image(&["home-background"], "home-background.avif"),
        1440,
        1920,
    )?;

    resize_image(
        &home_background,
        &outputs.image(&["home-background"], "home-background.jpg"),
        1440,
        1920,
    )?;

    resize_image(
        &home_background_mobile,
        &outputs.image(&["home-background"], "home-background-mobile.avif"),
        1440,
        810,
    )?;

    resize_image(
        &home_background_mobile,
        &outputs.image(&["home-background"], "home-background-mobile.jpg"),
        1440,
        810,
    )?;

    resize_image(&cv_icon, &outputs.icon(&["logos"], "android-chrome-192x192.png"), 192, 192)?;

    resize_image(&cv_icon, &outputs.icon(&["logos"], "android-chrome-512x512.png"), 512, 512)?;

    resize_image(
        &cv_profile_picture,
        &outputs.image(&["profile-pictures"], "profile-picture.avif"),
        512,
        512,
    )?;

    resize_image(
        &cv_profile_picture,
        &outputs.image(&["profile-pictures"], "profile-picture.jpg"),
        512,
        512,
    )?;

    copy_image(&cv_social_card, &outputs.image(&["social-card"], "social-card.png"))?;

    generate_favicon_set(&cv_favicon, &outputs.logos_dir())?;

    Ok(())
}

fn generate_main_assets(root: &Path) -> AppResult<()> {
    let outputs = OutputPaths::new(root, "main");

    let images_root = data_dir().join("images");

    let favicon_path = find_supported_image(&images_root, &["main"], "favicon")?;

    let main_icon = find_supported_image(&images_root, &["main"], "icon")?;
    let main_profile_picture = find_supported_image(&images_root, &["main"], "profile-picture")?;
    let main_social_card = find_supported_image(&images_root, &["main"], "social-card")?;
    let main_favicon = favicon_path.clone();

    resize_image(&main_icon, &outputs.icon(&["logos"], "android-chrome-192x192.png"), 192, 192)?;

    resize_image(&main_icon, &outputs.icon(&["logos"], "android-chrome-512x512.png"), 512, 512)?;

    resize_image(
        &main_profile_picture,
        &outputs.image(&["profile-pictures"], "profile-picture.avif"),
        512,
        512,
    )?;

    resize_image(
        &main_profile_picture,
        &outputs.image(&["profile-pictures"], "profile-picture.jpg"),
        512,
        512,
    )?;

    copy_image(&main_social_card, &outputs.image(&["social-card"], "social-card.png"))?;

    generate_favicon_set(&main_favicon, &outputs.logos_dir())?;

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
        outputs.image(&["profile-pictures"], "profile-picture.jpg"),
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
        outputs.image(&["profile-pictures"], "profile-picture.jpg"),
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

fn resize_image(input: &Path, output: &Path, width: u32, height: u32) -> AppResult<()> {
    if is_up_to_date(input, output)? {
        return Ok(());
    }

    let image = load_dynamic_image(input)?;
    save_resized_image(&image, output, width, height)
}

fn save_resized_image(
    image: &DynamicImage,
    output: &Path,
    width: u32,
    height: u32,
) -> AppResult<()> {
    let resized = resize_with_fast_image(image, width, height)?;
    ensure_parent_directory(output)?;
    resized.save(output).with_context(|| format!("writing {output:?}"))?;
    Ok(())
}

fn load_dynamic_image(input: &Path) -> AppResult<DynamicImage> {
    match input.extension().and_then(|ext| ext.to_str()).map(|ext| ext.to_lowercase()).as_deref() {
        Some("svg") => rasterize_svg(input),
        _ => image::open(input).with_context(|| format!("opening {input:?}")),
    }
}

fn resize_with_fast_image(
    image: &DynamicImage,
    width: u32,
    height: u32,
) -> AppResult<DynamicImage> {
    let rgba = image.to_rgba8();

    let src_image =
        Image::from_vec_u8(rgba.width(), rgba.height(), rgba.into_raw(), PixelType::U8x4)
            .map_err(|err| AppError::msg(format!("creating source image for resize: {err}")))?;

    let mut dst_image = Image::new(width, height, PixelType::U8x4);

    let mut resizer = Resizer::new();
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Bilinear));

    resizer
        .resize(&src_image, &mut dst_image, &options)
        .map_err(|err| AppError::msg(format!("resizing image to {width}x{height}: {err}")))?;

    let resized_image = RgbaImage::from_raw(width, height, dst_image.into_vec())
        .ok_or_else(|| AppError::msg("creating resized RGBA image"))?;

    Ok(DynamicImage::ImageRgba8(resized_image))
}

fn generate_favicon_set(source: &Path, output_dir: &Path) -> AppResult<()> {
    let apple_touch = output_dir.join("apple-touch-icon.png");
    let favicon_16 = output_dir.join("favicon-16x16.png");
    let favicon_32 = output_dir.join("favicon-32x32.png");
    let ico_output = output_dir.join("favicon.ico");

    let mut base_image: Option<DynamicImage> = None;

    if source
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("svg"))
        .unwrap_or(false)
    {
        let svg_output = output_dir.join("favicon.svg");
        copy_svg(source, &svg_output)?;
    }

    for (output, width, height) in
        [(&apple_touch, 180, 180), (&favicon_16, 16, 16), (&favicon_32, 32, 32)]
    {
        if !is_up_to_date(source, output)? {
            if base_image.is_none() {
                base_image = Some(load_dynamic_image(source)?);
            }

            if let Some(image) = base_image.as_ref() {
                save_resized_image(image, output, width, height)?;
            }
        }
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

fn copy_svg(input: &Path, output: &Path) -> AppResult<()> {
    if is_up_to_date(input, output)? {
        return Ok(());
    }

    ensure_parent_directory(output)?;
    let contents = fs::read(input).with_context(|| format!("reading {input:?}"))?;
    let mut file = fs::File::create(output).with_context(|| format!("creating {output:?}"))?;
    file.write_all(&contents).with_context(|| format!("writing {output:?}"))?;
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
